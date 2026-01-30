use ed25519_dalek::{Signer, SigningKey};
use hex::encode;
use serde::{Deserialize, Serialize};

use crate::models::event::QQBotEvent;

#[derive(Deserialize)]
struct ValidationRequest {
    #[serde(rename = "event_ts")]
    event_ts: String,
    #[serde(rename = "plain_token")]
    plain_token: String,
}

#[derive(Serialize)]
pub struct ValidationResponse {
    #[serde(rename = "plain_token")]
    plain_token: String,
    signature: String,
}

pub fn validate_webhook(payload: &QQBotEvent, secret: &str) -> ValidationResponse {
    let ValidationRequest {
        event_ts,
        plain_token,
    } = serde_json::from_value(payload.d.clone()).unwrap();

    let mut seed = secret.to_owned();
    while seed.len() < ed25519_dalek::SECRET_KEY_LENGTH {
        seed.push_str(secret);
    }
    let seed = &seed.as_bytes()[..ed25519_dalek::SECRET_KEY_LENGTH];

    // 生成私钥
    let signing_key = SigningKey::from_bytes(seed.try_into().unwrap());

    // 构造待签名消息
    let mut msg = event_ts;
    msg.push_str(&plain_token);

    // 签名
    let signature_bytes = signing_key.sign(msg.as_bytes());
    let signature = encode(signature_bytes.to_bytes());

    // 构造响应
    ValidationResponse {
        plain_token,
        signature,
    }
}
