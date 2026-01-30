use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AuthToken {
    pub access_token: String,
    pub expires_in: String,
}
