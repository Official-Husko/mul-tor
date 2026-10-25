//! Sending a file to a Send server over its WebSocket: the encrypted metadata
//! first, then the file as encrypted records, then a single zero byte.

use std::time::Duration;

use bytes::Bytes;
use futures::SinkExt;
use reqwest_websocket::Message;
use serde_json::{Value, json};
use tokio::io::AsyncReadExt;

use super::ece::{DATA_PER_RECORD, Encryptor};
use super::keys::Keys;
use crate::error::UploadError;
use crate::providers::websocket::{next_text, open, socket_error};
use crate::upload::UploadContext;

/// How long to wait for the server to answer.
const WAIT: Duration = Duration::from_secs(300);

/// Send the whole file; the id the server gave it once it has taken everything.
pub(super) async fn send_file(
    cx: &UploadContext<'_>,
    address: &str,
    keys: &Keys,
    seconds: u64,
    downloads: u64,
) -> Result<String, UploadError> {
    let mut socket = open(cx, address).await?;
    let first = json!({
        "fileMetadata": keys.encrypted_metadata(&cx.file.name, cx.file.size),
        "authorization": keys.authorization(),
        "timeLimit": seconds,
        "dlimit": downloads,
    });
    socket
        .send(Message::Text(first.to_string()))
        .await
        .map_err(socket_error)?;
    let id = file_id(&next_text(&mut socket, WAIT).await?)?;

    let (mut encryptor, header) = Encryptor::new(keys.secret());
    socket
        .send(Message::Binary(header.into()))
        .await
        .map_err(socket_error)?;

    let mut file = tokio::fs::File::open(&cx.file.path).await?;
    let mut current = read_up_to(&mut file, DATA_PER_RECORD).await?;
    loop {
        // A record is the last one only once nothing follows it.
        let next = read_up_to(&mut file, DATA_PER_RECORD).await?;
        let last = next.is_empty();
        let record = encryptor.record(&current, last);
        socket
            .send(Message::Binary(record.into()))
            .await
            .map_err(socket_error)?;
        cx.progress.add(current.len() as u64);
        if last {
            break;
        }
        current = next;
    }
    socket
        .send(Message::Binary(Bytes::from_static(&[0])))
        .await
        .map_err(socket_error)?;

    acknowledged(&next_text(&mut socket, WAIT).await?)?;
    Ok(id)
}

/// Up to `len` bytes from `file`; fewer only at the end of it.
async fn read_up_to(file: &mut tokio::fs::File, len: usize) -> Result<Vec<u8>, UploadError> {
    let mut buffer = vec![0u8; len];
    let mut filled = 0;
    while filled < len {
        let read = file.read(&mut buffer[filled..]).await?;
        if read == 0 {
            break;
        }
        filled += read;
    }
    buffer.truncate(filled);
    Ok(buffer)
}

/// The id from the server's first answer. It ends up in the link, so it must be plain.
fn file_id(reply: &str) -> Result<String, UploadError> {
    let json: Value = serde_json::from_str(reply)
        .map_err(|_| UploadError::protocol("reply is not valid JSON", reply))?;
    if let Some(error) = json.get("error") {
        return Err(UploadError::protocol(
            format!("server refused: {error}"),
            reply,
        ));
    }
    json.get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric()))
        .map(str::to_string)
        .ok_or_else(|| UploadError::protocol("reply has no usable id", reply))
}

/// The server's last answer: `{"ok": true}` once the file is stored.
fn acknowledged(reply: &str) -> Result<(), UploadError> {
    let json: Value = serde_json::from_str(reply)
        .map_err(|_| UploadError::protocol("reply is not valid JSON", reply))?;
    if json.get("ok").and_then(Value::as_bool) == Some(true) {
        Ok(())
    } else {
        Err(UploadError::protocol(
            "the server did not take the file",
            reply,
        ))
    }
}
