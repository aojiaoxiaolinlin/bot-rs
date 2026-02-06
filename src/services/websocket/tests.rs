use super::connection::WebSocketManager;
use crate::config::Config;
use crate::models::event::{OpCode, QQBotEvent};
use crate::services::client::QQClient;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

async fn start_mock_server(heartbeat_interval: u64) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("ws://{}/", addr);

    let handle = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let heartbeat_interval = heartbeat_interval;
            tokio::spawn(async move {
                let mut ws_stream = match accept_async(stream).await {
                    Ok(s) => s,
                    Err(_) => return,
                };

                // 1. Send Hello
                let hello = json!({
                    "op": 10,
                    "d": {
                        "heartbeat_interval": heartbeat_interval
                    }
                });
                if ws_stream
                    .send(Message::Text(hello.to_string().into()))
                    .await
                    .is_err()
                {
                    return;
                }

                while let Some(msg) = ws_stream.next().await {
                    if let Ok(Message::Text(text)) = msg {
                        let event: QQBotEvent = serde_json::from_str(&text).unwrap();
                        let op = OpCode::try_from(event.op).unwrap_or(OpCode::Dispatch);
                        match op {
                            OpCode::Identify => {
                                // Send Ready (Dispatch)
                                let ready = json!({
                                    "op": 0,
                                    "t": "READY",
                                    "d": {
                                        "session_id": "test_session_id",
                                        "user": {
                                            "id": "123",
                                            "username": "MockBot"
                                        }
                                    },
                                    "s": 1
                                });
                                let _ = ws_stream
                                    .send(Message::Text(ready.to_string().into()))
                                    .await;
                            }
                            OpCode::Heartbeat => {
                                // Send ACK
                                let ack = json!({
                                    "op": 11
                                });
                                let _ = ws_stream.send(Message::Text(ack.to_string().into())).await;
                            }
                            OpCode::Resume => {
                                // Send Resume ACK (just Dispatch with seq)
                                let dispatch = json!({
                                    "op": 0,
                                    "t": "RESUMED_EVENT",
                                    "d": {},
                                    "s": 100
                                });
                                let _ = ws_stream
                                    .send(Message::Text(dispatch.to_string().into()))
                                    .await;
                            }
                            _ => {}
                        }
                    } else if let Ok(Message::Close(_)) = msg {
                        break;
                    }
                }
            });
        }
    });

    (url, handle)
}

#[tokio::test]
async fn test_websocket_connect_and_identify() {
    let (url, _server_handle) = start_mock_server(1000).await;
    let token = "test_token".to_string();

    let config = Config {
        app_id: "test_app_id".to_string(),
        client_secret: "test_secret".to_string(),
    };
    let client = QQClient::new(config);
    client.set_access_token(token.clone());

    let mut manager = WebSocketManager::new(url.clone(), client).await;

    // Run start in a separate task so we can assert on connection status or wait for completion
    // But start() loops forever unless connection closed or error.
    // Our mock server closes connection after identify if we want, or we can just let it run a bit.

    // For this test, we just want to see if it sends Identify.
    // The manager.start() loops. We can spawn it.

    let _handle = tokio::spawn(async move {
        manager.start().await;
    });

    // Let it run for a bit
    tokio::time::sleep(Duration::from_millis(200)).await;
}

#[tokio::test]
async fn test_heartbeat_timeout() {
    // Create a server that sends Hello but NO HeartbeatACK
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("ws://{}/", addr);

    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut ws_stream = accept_async(stream).await.unwrap();
                // Send Hello with short interval
                let hello = json!({
                    "op": 10,
                    "d": {
                        "heartbeat_interval": 100
                    }
                });
                let _ = ws_stream
                    .send(Message::Text(hello.to_string().into()))
                    .await;

                // Read messages but do NOT send ACK
                while let Some(_) = ws_stream.next().await {}
            });
        }
    });

    let config = Config {
        app_id: "test_app_id".to_string(),
        client_secret: "test_secret".to_string(),
    };
    let client = QQClient::new(config);
    client.set_access_token("token".into());

    let mut manager = WebSocketManager::new(url, client).await;

    // We expect it to connect, send heartbeat, then timeout (after HEARTBEAT_TIMEOUT_SECONDS which is 2s in test), then reconnect
    // We can't easily verify the internal error, but we can verify it doesn't crash

    tokio::select! {
        _ = manager.start() => {},
        _ = tokio::time::sleep(Duration::from_secs(4)) => {}
    }
}
