use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::services::websocket::error::WebSocketError;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionData {
    pub session_id: Option<String>,
    pub last_seq: Option<u64>,
}

/// 会话状态管理器，负责内存中存储 session_id 和 last_seq
#[derive(Default)]
pub struct SessionState {
    data: RwLock<SessionData>,
}

impl SessionState {
    /// 创建新的会话状态管理器
    pub fn new() -> Self {
        Default::default()
    }

    pub async fn update(
        &self,
        session_id: Option<String>,
        last_seq: Option<u64>,
    ) -> Result<(), WebSocketError> {
        let mut data = self.data.write().await;

        if session_id.is_some() && data.session_id != session_id {
            data.session_id = session_id;
        }

        if last_seq.is_some() && data.last_seq != last_seq {
            data.last_seq = last_seq;
        }

        Ok(())
    }

    pub async fn get_session_id(&self) -> Option<String> {
        self.data.read().await.session_id.clone()
    }

    pub async fn get_last_seq(&self) -> Option<u64> {
        self.data.read().await.last_seq
    }
}
