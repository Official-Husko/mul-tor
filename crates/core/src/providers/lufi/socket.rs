//! Sending a file to a Lufi instance over its WebSocket: slice by slice, each
//! slice encrypted, each acknowledged before the next goes out.

use std::time::Duration;

use futures::{SinkExt, TryStreamExt};
use reqwest_websocket::{Message, Upgrade, WebSocket};
use serde_json::{Value, json};
use tokio::io::AsyncReadExt;

use super::sjcl;
use crate::error::UploadError;
use crate::upload::UploadContext;

/// Bytes of the file in each slice; what the web client uses.
const SLICE: u64 = 750_000;

/// Between a slice's metadata and its encrypted body.
const SEPARATOR: &str = "XXMOJOXX";

/// How long to wait for the acknowledgement of one slice.
const REPLY_TIMEOUT: Duration = Duration::from_secs(300);

/// Send the whole file; the id the instance gave it, once the last slice is in.
pub(super) async fn send_file(
    cx: &UploadContext<'_>,
    endpoint: &str,
    key: &str,
    delay: u32,
) -> Result<String, UploadError> {
    let mut socket = open(cx, endpoint).await?;
    let mut file = tokio::fs::File::open(&cx.file.path).await?;
    let parts = cx.file.size.div_ceil(SLICE).max(1);

    let mut short: Option<String> = None;
    for part in 0..parts {
        cx.progress
            .set_message(format!("Part {}/{}", part + 1, parts));
        let len = SLICE.min(cx.file.size - part * SLICE) as usize;
        let mut slice = vec![0u8; len];
        file.read_exact(&mut slice).await?;

        let meta = json!({
            "total": parts,
            "part": part,
            "size": cx.file.size,
            "name": cx.file.name,
            "type": "application/octet-stream",
            "delay": delay,
            "del_at_first_view": false,
            "zipped": false,
            "id": short,
            "i": 0,
        });
        let message = seal(meta, key.to_string(), slice).await?;
        socket
            .send(Message::Text(message))
            .await
            .map_err(socket_error)?;

        let reply = next_text(&mut socket).await?;
        cx.progress.add(len as u64);
        short = Some(acknowledged(&reply)?);
    }
    short.ok_or(UploadError::EmptyFile)
}

async fn open(cx: &UploadContext<'_>, endpoint: &str) -> Result<WebSocket, UploadError> {
    cx.http
        .get_for_upgrade(endpoint)
        .upgrade()
        .send()
        .await
        .map_err(socket_error)?
        .into_websocket()
        .await
        .map_err(socket_error)
}

/// Metadata, a separator and the encrypted slice, as the web client builds them.
/// The encryption is the slow part, so it runs off the async threads.
async fn seal(meta: Value, key: String, slice: Vec<u8>) -> Result<String, UploadError> {
    tokio::task::spawn_blocking(move || {
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &slice);
        let encrypted = sjcl::encrypt(&key, encoded.as_bytes());
        // The web client stringifies the encrypted text once more.
        let body = serde_json::to_string(&encrypted).expect("a string is valid JSON");
        format!("{meta}{SEPARATOR}{body}")
    })
    .await
    .map_err(|_| UploadError::unexpected("encrypting a slice failed"))
}

/// The next text message; pings and the like are skipped.
async fn next_text(socket: &mut WebSocket) -> Result<String, UploadError> {
    loop {
        let next = tokio::time::timeout(REPLY_TIMEOUT, socket.try_next())
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

/// The id from a successful acknowledgement. It goes into the link and into the
/// next message, so it must be plain.
fn acknowledged(reply: &str) -> Result<String, UploadError> {
    let json: Value = serde_json::from_str(reply)
        .map_err(|_| UploadError::protocol("reply is not valid JSON", reply))?;
    if json.get("success").and_then(Value::as_bool) != Some(true) {
        let why = json
            .get("msg")
            .and_then(Value::as_str)
            .unwrap_or("no reason given");
        return Err(UploadError::protocol(
            format!("server refused the slice: {why}"),
            reply,
        ));
    }
    json.get("short")
        .and_then(Value::as_str)
        .filter(|short| {
            !short.is_empty()
                && short
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
        .map(str::to_string)
        .ok_or_else(|| UploadError::protocol("reply has no usable id", reply))
}

/// A socket failure, without the address (which may carry a query).
fn socket_error(error: reqwest_websocket::Error) -> UploadError {
    UploadError::unexpected(format!("websocket: {error}"))
}
