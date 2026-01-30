use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Network request failed: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Failed to parse JSON response: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Authentication failed: status={status}, response={response}")]
    AuthFailed {
        status: reqwest::StatusCode,
        response: String,
    },

    #[error("Server error: status={status}, response={response}")]
    ServerError {
        status: reqwest::StatusCode,
        response: String,
    },

    #[error("Failed to post message: {0}")]
    PostMessageFailed(String),

    #[error("Failed to get WSS endpoint: {0}")]
    GetWssEndpointFailed(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}
