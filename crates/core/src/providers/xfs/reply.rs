//! Reading the file code out of a host's answer to an upload.

use crate::error::UploadError;
use crate::reply::Reply;

/// How the host reports the new file once the upload is done.
#[derive(Clone, Copy)]
pub(super) enum ReplyKind {
    /// A JSON array: `[{"file_code": "…", "file_status": "OK"}]`.
    Json,
    /// A redirect to a result page whose query holds `fn=<code>&st=OK`.
    Redirect,
}

impl ReplyKind {
    /// The file code in `reply`. It goes into the download link, so it must be
    /// plain letters and digits whatever the host sent. `require_status` makes a
    /// JSON reply without `file_status` a failure (one with a status other than
    /// "OK" always is).
    pub(super) fn file_code(
        self,
        reply: &Reply,
        require_status: bool,
    ) -> Result<String, UploadError> {
        let code = match self {
            ReplyKind::Json => code_from_json(reply, require_status)?,
            ReplyKind::Redirect => code_from_redirect(reply)?,
        };
        if !code.is_empty() && code.chars().all(|c| c.is_ascii_alphanumeric()) {
            Ok(code)
        } else {
            Err(UploadError::protocol("unusable file code", &reply.body))
        }
    }
}

fn code_from_json(reply: &Reply, require_status: bool) -> Result<String, UploadError> {
    let json = reply.json()?;
    let first = json
        .get(0)
        .ok_or_else(|| UploadError::protocol("reply has no file entry", &reply.body))?;
    match first.get("file_status").and_then(|s| s.as_str()) {
        Some("OK") => {}
        Some(status) => {
            return Err(UploadError::protocol(
                format!("server rejected the file ({status})"),
                &reply.body,
            ));
        }
        None if require_status => {
            return Err(UploadError::protocol(
                "reply has no file_status",
                &reply.body,
            ));
        }
        None => {}
    }
    first
        .get("file_code")
        .and_then(|c| c.as_str())
        .map(str::to_string)
        .ok_or_else(|| UploadError::protocol("reply has no file_code", &reply.body))
}

/// The code from a result page's address, e.g. `/?&fn=abc123&st=OK&op=upload_result`.
fn code_from_redirect(reply: &Reply) -> Result<String, UploadError> {
    let query = |key: &str| {
        reply
            .url
            .query_pairs()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.into_owned())
    };
    match query("st").as_deref() {
        Some("OK") => {}
        Some(status) => {
            return Err(UploadError::protocol(
                format!("server rejected the file ({status})"),
                &reply.body,
            ));
        }
        None => {
            return Err(UploadError::protocol(
                "the upload did not end on a result page",
                &reply.body,
            ));
        }
    }
    query("fn").ok_or_else(|| UploadError::protocol("result page has no file code", &reply.body))
}
