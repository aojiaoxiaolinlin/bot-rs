use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use super::client_error::ClientError;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Client error: {0}")]
    ClientError(#[from] ClientError),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Internal server error: {0}")]
    InternalServerError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::ClientError(e) => match e {
                ClientError::AuthFailed { .. } => (StatusCode::UNAUTHORIZED, e.to_string()),
                ClientError::ParseError(_) => (StatusCode::BAD_REQUEST, e.to_string()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            },
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::SerializationError(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = serde_json::json!({
            "error": error_message,
        });

        (status, axum::Json(body)).into_response()
    }
}
