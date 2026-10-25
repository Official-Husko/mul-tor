//! The WebSocket half of the mock server.

use std::sync::{Arc, Mutex};

use axum::extract::ws::{Message, WebSocket};

/// What arrived over the socket.
pub enum Frame<'a> {
    Text(&'a str),
    Binary(&'a [u8]),
}

/// Gets each frame and returns the text messages to send back.
pub type SocketHandler = dyn Fn(Frame<'_>) -> Vec<String> + Send + Sync;

/// Everything the socket received, in order within each kind.
#[derive(Default)]
pub struct SocketLog {
    pub text: Mutex<Vec<String>>,
    pub binary: Mutex<Vec<Vec<u8>>>,
}

pub async fn serve_socket(mut socket: WebSocket, handler: Arc<SocketHandler>, log: Arc<SocketLog>) {
    while let Some(Ok(message)) = socket.recv().await {
        let replies = match &message {
            Message::Text(text) => {
                log.text.lock().unwrap().push(text.to_string());
                handler(Frame::Text(text.as_str()))
            }
            Message::Binary(bytes) => {
                log.binary.lock().unwrap().push(bytes.to_vec());
                handler(Frame::Binary(bytes))
            }
            _ => continue,
        };
        for reply in replies {
            if socket.send(Message::Text(reply.into())).await.is_err() {
                return;
            }
        }
    }
}
