use async_trait::async_trait;
use tracing::debug;

use crate::{
    models::{
        client_error::ClientError,
        message::{GroupMessage, PostMessageBody},
    },
    services::client::QQClient,
};

#[async_trait]
pub trait QQEvent: Send + Sync {
    async fn on_group_at_message_create(
        &self,
        message: GroupMessage,
        client: &QQClient,
    ) -> Result<(), ClientError>;
}

pub struct DefaultEventHandler;

#[async_trait]
impl QQEvent for DefaultEventHandler {
    async fn on_group_at_message_create(
        &self,
        message: GroupMessage,
        client: &QQClient,
    ) -> Result<(), ClientError> {
        debug!("Handling GroupAtMessageCreate event");
        let body = PostMessageBody::from_msg_type(0)
            .with_content(format!("收到消息: {}", message.content))
            .with_msg_id(message.id.clone());

        client
            .post_group_message(&message.group_openid, body)
            .await?;

        Ok(())
    }
}
