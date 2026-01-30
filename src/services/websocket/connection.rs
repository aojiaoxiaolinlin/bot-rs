use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use tokio::time::{Instant, interval_at, sleep};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use crate::models::event::{OpCode, QQBotEvent};
use crate::services::server::EventType;
use crate::services::websocket::error::WebSocketError;
use crate::services::websocket::state::SessionState;

// 常量定义
const MAX_RESUME_RETRIES: u32 = 3;
#[cfg(not(test))]
const RESUME_WAIT_SECONDS: u64 = 30;
#[cfg(test)]
const RESUME_WAIT_SECONDS: u64 = 1;

#[cfg(not(test))]
const HEARTBEAT_TIMEOUT_SECONDS: u64 = 7;
#[cfg(test)]
const HEARTBEAT_TIMEOUT_SECONDS: u64 = 2; // 测试时稍微长一点以免误判，但比 7s 短

const RECONNECT_BASE_DELAY_MS: u64 = 1000;
const RECONNECT_MAX_DELAY_MS: u64 = 5000;

/// WebSocket 管理器，负责维护连接、心跳和状态恢复
pub struct WebSocketManager {
    /// WebSocket 服务端地址
    wss_url: String,
    /// 鉴权 Token
    token: String,
    /// 会话状态（Session ID, Last Seq）
    state: Arc<SessionState>,
    /// 当前连续 Resume 失败次数
    resume_count: u32,
}

impl WebSocketManager {
    /// 创建新的 WebSocket 管理器
    pub async fn new(wss_url: String, token: String) -> Self {
        let state = Arc::new(SessionState::new());
        Self {
            wss_url,
            token,
            state,
            resume_count: 0,
        }
    }

    pub async fn start(&mut self) {
        loop {
            match self.connect_and_loop().await {
                Ok(_) => {
                    debug!("WebSocket 连接正常关闭");
                    self.resume_count = 0;
                }
                Err(e) => {
                    error!("WebSocket 异常断开: {:?}", e);
                    match e {
                        WebSocketError::HeartbeatTimeout
                        | WebSocketError::ConnectionClosed
                        | WebSocketError::Io(_) => {
                            // 这些错误通常意味着网络问题，尝试 Resume
                            self.handle_reconnect_delay().await;
                        }
                        _ => {
                            // 其他错误可能需要重置会话
                            // 例如 InvalidSession 已经在 connect_and_loop 内部处理并清空了状态
                            self.handle_reconnect_delay().await;
                        }
                    }
                }
            }
        }
    }

    async fn handle_reconnect_delay(&mut self) {
        if self.resume_count >= MAX_RESUME_RETRIES {
            warn!(
                "连续重连失败 {} 次，暂停 {} 秒",
                self.resume_count, RESUME_WAIT_SECONDS
            );
            sleep(Duration::from_secs(RESUME_WAIT_SECONDS)).await;
            self.resume_count = 0;
        } else {
            let delay_ms = {
                let mut rng = rand::rng();
                // 基础延迟 + 随机抖动 (+-20%)
                rng.random_range(
                    (RECONNECT_BASE_DELAY_MS as f64 * 0.8) as u64
                        ..=(RECONNECT_BASE_DELAY_MS as f64 * 1.2) as u64,
                )
            };

            // 简单的指数退避也可以考虑，但这里使用固定范围+抖动
            let final_delay = std::cmp::min(
                delay_ms * (self.resume_count as u64 + 1),
                RECONNECT_MAX_DELAY_MS,
            );

            info!("将在 {}ms 后尝试重连...", final_delay);
            sleep(Duration::from_millis(final_delay)).await;
            self.resume_count += 1;
        }
    }

    async fn connect_and_loop(&mut self) -> Result<(), WebSocketError> {
        debug!("正在连接 WebSocket: {}", self.wss_url);
        let (ws_stream, _) = connect_async(&self.wss_url).await?;
        let (mut write, mut read) = ws_stream.split();

        // 1. 等待 Hello 包以获取心跳间隔
        let heartbeat_interval_ms = loop {
            match read.next().await {
                Some(Ok(Message::Text(text))) => {
                    let event = serde_json::from_str::<QQBotEvent>(&text)?;
                    if OpCode::try_from(event.op).unwrap_or(OpCode::Dispatch) == OpCode::Hello
                        && let Some(serde_json::Value::Object(d)) = event.d
                    {
                        if let Some(interval) = d.get("heartbeat_interval").and_then(|v| v.as_u64())
                        {
                            debug!("收到 Hello，心跳间隔: {}ms", interval);
                            break interval;
                        }
                        return Err(WebSocketError::MissingHeartbeatInterval);
                    } else {
                        debug!("收到非 Hello 消息: {:?}", event);
                    }
                }
                Some(Ok(Message::Close(_))) => return Err(WebSocketError::ConnectionClosed),
                Some(Err(e)) => return Err(WebSocketError::ConnectionFailed(e)),
                None => return Err(WebSocketError::ConnectionClosed),
                _ => {}
            }
        };

        // 2. Identify 或 Resume
        let session_id = self.state.get_session_id().await;
        let last_seq = self.state.get_last_seq().await;

        if let (Some(sid), Some(seq)) = (session_id, last_seq) {
            debug!("尝试 Resume Session: {}, Seq: {}", sid, seq);
            self.send_resume(&mut write, &sid, seq).await?;
        } else {
            debug!("发送 Identify");
            self.send_identify(&mut write).await?;
        }

        // 3. 主循环
        // 使用 interval_at 避免第一次 tick 立即触发
        let mut heartbeat_interval = interval_at(
            Instant::now() + Duration::from_millis(heartbeat_interval_ms),
            Duration::from_millis(heartbeat_interval_ms),
        );
        heartbeat_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let mut awaiting_ack = false;
        // 超时检查器，初始设置为永不触发
        let mut ack_timeout = Box::pin(sleep(Duration::MAX));

        loop {
            tokio::select! {
                // 接收消息
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            // 收到任何文本消息都算一种活跃，但只有 HeartbeatACK 才能清除 awaiting_ack
                            // 这里我们先解析
                            let event = match serde_json::from_str::<QQBotEvent>(&text) {
                                Ok(e) => e,
                                Err(e) => {
                                    error!("解析消息失败: {}", e);
                                    continue;
                                }
                            };

                            // 更新 last_seq (如果有)
                            if let Some(s) = event.s {
                                self.state.update(None, Some(s)).await?;
                            }

                            match OpCode::try_from(event.op).unwrap_or(OpCode::Dispatch) {
                                OpCode::Dispatch => {
                                    self.handle_dispatch(event).await?;
                                    // 收到 Dispatch 也可以视为连接存活，但协议要求必须有 HeartbeatACK
                                }
                                OpCode::HeartbeatACK => {
                                    debug!("收到 HeartbeatACK");
                                    awaiting_ack = false;
                                    // 取消超时计时
                                    ack_timeout = Box::pin(sleep(Duration::MAX));
                                }
                                OpCode::InvalidSession => {
                                    warn!("收到 InvalidSession，会话失效，清理状态");
                                    self.state.update(None, None).await?; // 清空状态
                                    // 这里返回错误，触发重连，重连时会因为没有状态而走 Identify
                                    return Err(WebSocketError::Other("Invalid Session".to_string()));
                                }
                                OpCode::Reconnect => {
                                    debug!("服务端要求重连");
                                    return Ok(());
                                }
                                OpCode::Heartbeat => {
                                    // 服务端请求心跳，立即回复一次
                                    self.send_heartbeat(&mut write).await?;
                                }
                                _ => {
                                    debug!("收到其他 OpCode: {}", event.op);
                                }
                            }
                        }
                        Some(Ok(Message::Close(_))) => {
                            info!("连接被服务端关闭");
                            return Err(WebSocketError::ConnectionClosed);
                        }
                        Some(Err(e)) => return Err(WebSocketError::ConnectionFailed(e)),
                        None => return Err(WebSocketError::ConnectionClosed),
                        _ => {}
                    }
                }

                // 发送心跳
                _ = heartbeat_interval.tick() => {
                    debug!("发送心跳...");
                    self.send_heartbeat(&mut write).await?;
                    awaiting_ack = true;
                    // 启动超时计时
                    ack_timeout = Box::pin(sleep(Duration::from_secs(HEARTBEAT_TIMEOUT_SECONDS)));
                }

                // 心跳超时检测
                _ = &mut ack_timeout => {
                    if awaiting_ack {
                        error!("心跳超时！未在 {} 秒内收到 ACK", HEARTBEAT_TIMEOUT_SECONDS);
                        return Err(WebSocketError::HeartbeatTimeout);
                    }
                }
            }
        }
    }

    async fn send_heartbeat<S>(&self, write: &mut S) -> Result<(), WebSocketError>
    where
        S: SinkExt<Message> + Unpin,
        S::Error: std::error::Error + Send + Sync + 'static,
    {
        let last_seq = self.state.get_last_seq().await;
        let d = match last_seq {
            Some(s) => serde_json::to_value(s)?,
            None => serde_json::Value::Null,
        };

        let event = QQBotEvent {
            op: OpCode::Heartbeat.into(),
            d: Some(d),
            ..Default::default()
        };

        let json = serde_json::to_string(&event)?;
        write
            .send(Message::Text(json.into()))
            .await
            .map_err(|e| WebSocketError::SendFailed(e.to_string()))?;
        Ok(())
    }

    async fn send_identify<S>(&self, write: &mut S) -> Result<(), WebSocketError>
    where
        S: SinkExt<Message> + Unpin,
        S::Error: std::error::Error + Send + Sync + 'static,
    {
        let mut map = serde_json::Map::new();
        map.insert(
            "token".to_owned(),
            serde_json::Value::String(format!("QQBot {}", self.token)),
        );
        map.insert("intents".to_owned(), serde_json::to_value(1 << 30).unwrap());
        map.insert("shard".to_owned(), serde_json::to_value([0, 1]).unwrap());

        let event = QQBotEvent {
            op: OpCode::Identify.into(),
            d: Some(serde_json::Value::Object(map)),
            ..Default::default()
        };

        let json = serde_json::to_string(&event)?;
        write
            .send(Message::Text(json.into()))
            .await
            .map_err(|e| WebSocketError::SendFailed(e.to_string()))?;
        Ok(())
    }

    async fn send_resume<S>(
        &self,
        write: &mut S,
        session_id: &str,
        seq: u64,
    ) -> Result<(), WebSocketError>
    where
        S: SinkExt<Message> + Unpin,
        S::Error: std::error::Error + Send + Sync + 'static,
    {
        let mut map = serde_json::Map::new();
        map.insert(
            "token".to_owned(),
            serde_json::Value::String(format!("QQBot {}", self.token)),
        );
        map.insert(
            "session_id".to_owned(),
            serde_json::Value::String(session_id.to_string()),
        );
        map.insert("seq".to_owned(), serde_json::to_value(seq).unwrap());

        let event = QQBotEvent {
            op: OpCode::Resume.into(),
            d: Some(serde_json::Value::Object(map)),
            ..Default::default()
        };

        let json = serde_json::to_string(&event)?;
        write
            .send(Message::Text(json.into()))
            .await
            .map_err(|e| WebSocketError::SendFailed(e.to_string()))?;
        Ok(())
    }

    async fn handle_dispatch(&self, event: QQBotEvent) -> Result<(), WebSocketError> {
        // 提取 Ready 事件中的 session_id
        // 注意：OpCode 0 (Dispatch) 包含各种事件，Ready 是其中一种，由 event.t 区分
        let Some(t) = event.t else {
            return Ok(());
        };

        if let Ok(t) = EventType::from_str(&t) {
            match t {
                EventType::Ready => {
                    if let Some(serde_json::Value::Object(d)) = &event.d {
                        if let Some(serde_json::Value::String(session_id)) = d.get("session_id") {
                            debug!("Ready 事件，获取到 session_id: {}", session_id);
                            self.state.update(Some(session_id.clone()), None).await?;
                        }
                        if let Some(v) = d.get("user")
                            && let Some(username) = v.get("username").and_then(|u| u.as_str())
                        {
                            info!("机器人: [{}] 启动成功! 就绪！", username);
                        }
                    }
                }
                _ => {
                    // TODO: 分发其他事件到 EventBus 或 Handler
                    // 这里只是打印日志
                    debug!("Dispatch Event: {:?}", t);
                }
            }
        }

        Ok(())
    }
}

pub async fn start(wss_url: String, token: String) {
    let mut manager = WebSocketManager::new(wss_url, token).await;
    manager.start().await;
}
