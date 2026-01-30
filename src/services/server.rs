use std::str::FromStr;
use std::sync::Arc;

use axum::{
    Router,
    extract::{Json, State},
    response::Result,
    routing::post,
};
use serde::Serialize;
use strum::EnumString;
use tokio::net::ToSocketAddrs;
use tracing::{debug, error, info};

use crate::{
    config::Config,
    event_client::{DefaultEventHandler, QQEvent},
    models::{
        error::AppError,
        event::{OpCode, QQBotEvent},
        message::GroupMessage,
        server_error::ServerError,
    },
    services::{client::QQClient, websocket},
    utils::validation::validate_webhook,
};

#[derive(Clone)]
struct AppState {
    client: QQClient,
    config: Config,
    event_handler: Arc<dyn QQEvent>,
}

pub struct ServerBuilder {
    config: Config,
    event_handler: Option<Arc<dyn QQEvent>>,
}

impl ServerBuilder {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            event_handler: None,
        }
    }

    pub fn with_event_handler(mut self, handler: impl QQEvent + 'static) -> Self {
        self.event_handler = Some(Arc::new(handler));
        self
    }

    pub async fn start<A: ToSocketAddrs>(self, addr: A) -> Result<(), ServerError> {
        info!("启动中...");
        let client = QQClient::new(self.config.clone());
        info!("鉴权中...");
        let _ = client.auth().await?;
        let wss_url = client.get_wss_endpoint().await?;

        let token = client
            .get_access_token()
            .ok_or_else(|| ServerError::AccessTokenMissing)?;

        info!("会话启动中...");
        tokio::spawn(async move {
            websocket::start(wss_url, token).await;
        });

        let event_handler = self
            .event_handler
            .unwrap_or_else(|| Arc::new(DefaultEventHandler));

        let state = AppState {
            client,
            config: self.config,
            event_handler,
        };

        let app = Router::new()
            .route("/", post(qq_bot_event_handler))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind(addr).await?;

        info!("机器人服务已就绪");

        axum::serve(listener, app).await?;

        Ok(())
    }
}

async fn qq_bot_event_handler(
    State(state): State<AppState>,
    Json(payload): Json<QQBotEvent>,
) -> Result<Json<serde_json::Value>, AppError> {
    debug!("Received event: {:?}", payload);

    #[derive(Debug, Serialize)]
    struct CallbackACK {
        op: u8,
    }
    let callback_ack = serde_json::to_value(&CallbackACK {
        op: OpCode::CallbackACK.into(),
    })
    .unwrap();

    match OpCode::try_from(payload.op) {
        Ok(op) => match op {
            OpCode::Dispatch => {
                // 使用 tokio::spawn 异步处理事件，不阻塞 WebHook 响应
                tokio::spawn(async move {
                    if let Err(e) = dispatch_event(payload, state).await {
                        error!("Error handling dispatch event: {:?}", e);
                    }
                });
                Ok(Json(callback_ack))
            }
            OpCode::WebhookValidate => {
                // Handle webhook validation event
                let response = validate_webhook(&payload, &state.config.client_secret);
                Ok(Json(serde_json::to_value(response)?))
            }
            _ => {
                error!("Received unsupported opcode: {}", payload.op);
                Err(AppError::ValidationError(format!(
                    "Unsupported opcode: {}",
                    payload.op
                )))
            }
        },
        Err(err) => {
            error!("Failed to parse opcode: {}", err);
            Err(AppError::ValidationError(format!(
                "Invalid opcode: {}",
                payload.op
            )))
        }
    }
}

async fn dispatch_event(payload: QQBotEvent, state: AppState) -> Result<(), AppError> {
    if let Some(id) = &payload.id {
        debug!(id);
    }
    if let Some(t) = &payload.t {
        debug!(t);

        match EventType::from_str(t) {
            Ok(EventType::GroupAtMessageCreate) => {
                let message: GroupMessage = serde_json::from_value(payload.d)
                    .map_err(|e| AppError::SerializationError(e))?;

                state
                    .event_handler
                    .on_group_at_message_create(message, &state.client)
                    .await
                    .map_err(AppError::ClientError)?;
            }
            Err(err) => {
                error!("Failed to parse event type: {}", err);
                return Err(AppError::ValidationError(format!(
                    "Unknown event type: {}",
                    t
                )));
            }
        }
    }
    Ok(())
}

#[derive(Debug, EnumString)]
pub enum EventType {
    #[strum(serialize = "GROUP_AT_MESSAGE_CREATE")]
    GroupAtMessageCreate,
}
