use super::client_error::ClientError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Client error: {0}")]
    Client(#[from] ClientError),

    #[error("Failed to get access token")]
    AccessTokenMissing,
}
