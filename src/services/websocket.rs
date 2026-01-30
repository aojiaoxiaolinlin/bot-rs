use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info};

use crate::models::event::{OpCode, QQBotEvent};

pub async fn start(wss_url: String, token: String) {
    let connect_result = connect_async(&wss_url).await;

    let (ws_stream, _) = match connect_result {
        Ok(res) => res,
        Err(e) => {
            error!("Failed to connect to WebSocket: {}", e);
            return;
        }
    };

    let (mut sender, mut receiver) = ws_stream.split();
    loop {
        if let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!("QQ Server: {}", &text);
                    // 处理文本消息
                    let message = serde_json::from_str::<QQBotEvent>(text.as_str());

                    match message {
                        Ok(message) => match OpCode::try_from(message.op) {
                            Ok(op_code) => match op_code {
                                OpCode::Dispatch => {
                                    if let serde_json::Value::Object(d) = message.d
                                        && let Some(v) = d.get("user")
                                        && let serde_json::Value::Object(user) = v
                                        && let Some(serde_json::Value::String(username)) =
                                            user.get("username")
                                    {
                                        // 处理用户信息
                                        info!("机器人: [{}] 启动成功! ", username);
                                    }
                                }
                                OpCode::Hello => {
                                    if let serde_json::Value::Object(object) = message.d
                                        && let Some(heartbeat_interval) =
                                            object.get("heartbeat_interval")
                                        && let Some(heartbeat_interval) =
                                            heartbeat_interval.as_u64()
                                    {
                                        // 获取到心跳间隔
                                        let _heartbeat_interval = heartbeat_interval;
                                    }

                                    let mut map = serde_json::Map::new();
                                    map.insert(
                                        "token".to_owned(),
                                        serde_json::Value::String(format!("QQBot {token}")),
                                    );
                                    map.insert(
                                        "intents".to_owned(),
                                        serde_json::to_value(1 << 30).unwrap(),
                                    );
                                    map.insert(
                                        "shard".to_owned(),
                                        serde_json::to_value([0, 1]).unwrap(),
                                    );

                                    let message = QQBotEvent {
                                        op: OpCode::Identify.into(),
                                        d: serde_json::Value::Object(map),
                                        ..Default::default()
                                    };

                                    let _ = sender
                                        .send(Message::Text(
                                            serde_json::to_string(&message).unwrap().into(),
                                        ))
                                        .await;
                                }
                                OpCode::Heartbeat => todo!(),
                                OpCode::CallbackACK => todo!(),
                                OpCode::WebhookValidate => todo!(),
                                _ => {}
                            },
                            Err(err) => {
                                error!("Failed to parse opcode: {}", err);
                            }
                        },
                        Err(e) => {
                            error!("Failed to parse message: {}", e);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket connection closed");
                    break;
                }
                Err(err) => {
                    error!("WebSocket error: {:?}", err);
                    break;
                }
                _ => {}
            }
        }
    }
}
