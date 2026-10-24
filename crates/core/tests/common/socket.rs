//! The WebSocket half of the mock server.

use std::sync::{Arc, Mutex};

use axum::extract::ws::{Message, WebSocket};

/// Gets each text message and returns the messages to send back.
pub type SocketHandler = dyn Fn(&str) -> Vec<String> + Send + Sync;

pub async fn serve_socket(
    mut socket: WebSocket,
    on_text: Arc<SocketHandler>,
    received: Arc<Mutex<Vec<String>>>,
) {
    while let Some(Ok(message)) = socket.recv().await {
        let Message::Text(text) = message else {
            continue;
        };
        let text = text.to_string();
        received.lock().unwrap().push(text.clone());
        for reply in on_text(&text) {
            if socket.send(Message::Text(reply.into())).await.is_err() {
                return;
            }
        }
    }
}
