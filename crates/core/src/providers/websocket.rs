//! The WebSocket plumbing providers share: opening one through the HTTP client
//! (so the proxy and user agent apply) and reading its answers.

use std::time::Duration;

use futures::TryStreamExt;
use reqwest_websocket::{Message, Upgrade, WebSocket};

use crate::error::UploadError;
use crate::upload::UploadContext;

/// Open the socket at `https_address` (the address written `wss://` becomes `https://`).
pub(super) async fn open(
    cx: &UploadContext<'_>,
    https_address: &str,
) -> Result<WebSocket, UploadError> {
    cx.http
        .get_for_upgrade(https_address)
        .upgrade()
        .send()
        .await
        .map_err(socket_error)?
        .into_websocket()
        .await
        .map_err(socket_error)
}

/// The next text message; pings and the like are skipped. Gives up after `wait`.
pub(super) async fn next_text(
    socket: &mut WebSocket,
    wait: Duration,
) -> Result<String, UploadError> {
    loop {
        let next = tokio::time::timeout(wait, socket.try_next())
            .await
            .map_err(|_| UploadError::unexpected("the server stopped answering"))?
            .map_err(socket_error)?;
        match next {
            Some(Message::Text(text)) => return Ok(text),
            Some(Message::Close { .. }) | None => {
                return Err(UploadError::unexpected("the server closed the connection"));
            }
            Some(_) => {}
        }
    }
}

/// A socket failure, without the address (which may carry a query).
pub(super) fn socket_error(error: reqwest_websocket::Error) -> UploadError {
    UploadError::unexpected(format!("websocket: {error}"))
}
