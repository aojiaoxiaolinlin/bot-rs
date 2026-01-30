use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct QQBotEvent {
    /// 事件id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// 操作类型
    pub op: u8,
    /// 事件内容
    pub d: serde_json::Value,
    /// 事件序列号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s: Option<u64>,
    /// 事件类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t: Option<String>,
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum OpCode {
    Dispatch = 0,
    Heartbeat = 1,
    Identify = 2,

    Hello = 10,

    CallbackACK = 12,
    WebhookValidate = 13,
}
