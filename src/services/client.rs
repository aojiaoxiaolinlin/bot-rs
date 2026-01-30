use std::sync::{Arc, RwLock};

use axum::http::{HeaderMap, HeaderValue};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use tracing::{debug, error};

use crate::{
    config::Config,
    models::{auth::AuthToken, client_error::ClientError, message::PostMessageBody},
};

// 固定的QQ API地址
const QQ_BASE_URL: &str = "https://api.sgroup.qq.com";
const QQ_AUTH_URL: &str = "https://bots.qq.com/app/getAppAccessToken";

#[derive(Clone)]
pub struct QQClient {
    client: reqwest::Client,
    config: Config,
    token: Arc<RwLock<Option<String>>>,
}

impl QQClient {
    pub fn new(config: Config) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("构建 reqwest 客户端失败");

        Self {
            client,
            config,
            token: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn auth(&self) -> Result<(), ClientError> {
        let body = serde_json::json!({
            "appId": self.config.app_id,
            "clientSecret": self.config.client_secret
        });

        let response = self.client.post(QQ_AUTH_URL).json(&body).send().await?;

        if response.status().is_client_error() || response.status().is_server_error() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(ClientError::AuthFailed {
                status,
                response: text,
            });
        }

        let token = response.json::<AuthToken>().await?;
        debug!("Token: {:?}", token);

        // Update internal token
        if let Ok(mut lock) = self.token.write() {
            *lock = Some(token.access_token.clone());
        }

        Ok(())
    }

    pub fn get_access_token(&self) -> Option<String> {
        self.token.read().ok().and_then(|lock| lock.clone())
    }

    pub async fn post_group_message(
        &self,
        group_openid: &str,
        body: PostMessageBody,
    ) -> Result<(), ClientError> {
        let access_token = self
            .get_access_token()
            .ok_or_else(|| ClientError::Unknown("No access token available".to_string()))?;

        let url = format!("{}/v2/groups/{}/messages", QQ_BASE_URL, group_openid);
        let response = self
            .client
            .post(url)
            .header(AUTHORIZATION, format!("QQBot {access_token}"))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("Failed to post message: {}", text);
            return Err(ClientError::PostMessageFailed(format!(
                "status: {}, response: {}",
                status, text
            )));
        }

        debug!("Message posted successfully");
        Ok(())
    }

    pub async fn get_wss_endpoint(&self) -> Result<String, ClientError> {
        let access_token = self
            .get_access_token()
            .ok_or_else(|| ClientError::Unknown("No access token available".to_string()))?;

        let url = format!("{}/gateway", QQ_BASE_URL);
        let response = self
            .client
            .get(url)
            .header(AUTHORIZATION, format!("QQBot {access_token}"))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(ClientError::GetWssEndpointFailed(format!(
                "status: {}, response: {}",
                status, text
            )));
        }

        #[derive(Deserialize)]
        struct WssEndpoint {
            url: String,
        }
        let endpoint = response.json::<WssEndpoint>().await?;
        debug!("WSS Endpoint: {:?}", endpoint.url);
        Ok(endpoint.url)
    }
}
