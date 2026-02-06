use thiserror::Error;

#[derive(Error, Debug)]
pub enum WebSocketError {
    #[error("WebSocket connection failed: {0}")]
    ConnectionFailed(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Heartbeat timeout")]
    HeartbeatTimeout,

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Missing heartbeat interval in Hello payload")]
    MissingHeartbeatInterval,

    #[error("Invalid Session")]
    InvalidSession,

    #[error("Other error: {0}")]
    Other(String),
}
