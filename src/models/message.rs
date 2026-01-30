use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Default)]
pub struct PostMessageBody {
    msg_type: u8,

    #[serde(skip_serializing_if = "Option::is_none")]
    msg_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    event_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    msg_seq: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    is_wakeup: Option<bool>,
}

impl PostMessageBody {
    pub fn from_msg_type(msg_type: u8) -> Self {
        Self {
            msg_type,
            ..Default::default()
        }
    }

    pub fn with_content(mut self, content: String) -> Self {
        self.content = Some(content);
        self
    }

    pub fn with_msg_id(mut self, msg_id: String) -> Self {
        self.msg_id = Some(msg_id);
        self
    }

    pub fn with_event_id(mut self, event_id: String) -> Self {
        self.event_id = Some(event_id);
        self
    }

    pub fn with_msg_seq(mut self, msg_seq: String) -> Self {
        self.msg_seq = Some(msg_seq);
        self
    }

    pub fn with_is_wakeup(mut self, is_wakeup: bool) -> Self {
        self.is_wakeup = Some(is_wakeup);
        self
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GroupMessage {
    pub author: Author,
    pub content: String,
    pub group_id: String,
    pub group_openid: String,
    pub id: String,
    pub message_scene: MessageScene,
    pub message_type: u8,
    pub timestamp: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Author {
    pub id: String,
    pub member_openid: String,
    pub union_openid: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageScene {
    pub source: String,
}
