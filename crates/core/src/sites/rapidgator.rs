//! Rapidgator — log in for a token, announce the file (with its MD5), upload
//! it, then poll until the server has processed it.

use std::io::Read;
use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use md5::{Digest, Md5};
use serde_json::Value;

use crate::error::UploadError;
use crate::reply::{Reply, SendReply, as_u64};
use crate::site::{CredentialField, Site, SiteInfo};
use crate::size::format_bytes;
use crate::upload::{UploadContext, UploadedFile, new_form};

const INFO: SiteInfo = SiteInfo::new("Rapidgator", "https://rapidgator.net/", "https://rapidgator.net/")
    .requires(&[
        CredentialField::new("email", "Account e-mail", false),
        CredentialField::new("password", "Password", true),
    ])
    .api_docs("https://rapidgator.net/article/api/")
    .credentials_url("https://rapidgator.net/article/api/")
    .known_issue(
        "The size limit depends on the account, so it is only checked after logging in. \
         The Python version reported intermittent failures on batch uploads.",
    );

const API: &str = "https://rapidgator.net/api/v2";

/// The server reports an upload as finished with this state.
const STATE_DONE: u64 = 2;

pub struct Rapidgator {
    poll_interval: Duration,
    max_polls: usize,
}

impl Default for Rapidgator {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(5),
            // About 15 minutes of waiting for server-side processing.
            max_polls: 180,
        }
    }
}

impl Rapidgator {
    #[doc(hidden)]
    pub fn with_polling(poll_interval: Duration, max_polls: usize) -> Self {
        Self {
            poll_interval,
            max_polls,
        }
    }
}

/// Rapidgator wraps results as `{"status": 200, "details": …, "response": …}`.
fn check_status(reply: &Reply, what: &str) -> Result<Value, UploadError> {
    let json = reply.json()?;
    if json.get("status").and_then(Value::as_u64) == Some(200) {
        return Ok(json);
    }
    let details = json
        .get("details")
        .and_then(Value::as_str)
        .unwrap_or("no details");
    Err(UploadError::protocol(
        format!("{what} failed: {details}"),
        &reply.body,
    ))
}

fn upload_state(json: &Value) -> Option<u64> {
    as_u64(json.pointer("/response/upload/state"))
}

fn file_url(json: &Value, raw: &str) -> Result<String, UploadError> {
    crate::reply::json_string(json, "/response/upload/file/url", raw)
}

#[async_trait]
impl Site for Rapidgator {
    fn info(&self) -> &SiteInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let email = cx.credential("email")?;
        let password = cx.credential("password")?;
        let size = cx.file.size;

        cx.progress.set_message("Logging in");
        let login = cx
            .http
            .get(format!("{API}/user/login"))
            .query(&[("login", email), ("password", password)])
            .send_reply()
            .await?;
        let login_json = check_status(&login, "login")?;
        let token = crate::reply::json_string(&login_json, "/response/token", &login.body)?;

        // Limits depend on the account; only enforce what the server told us.
        if let Some(max) = as_u64(login_json.pointer("/response/user/upload/max_file_size")) {
            if max > 0 && size > max {
                return Err(UploadError::TooLarge {
                    limit: format_bytes(max),
                });
            }
        }
        if let Some(left) = as_u64(login_json.pointer("/response/user/storage/left")) {
            if left <= size {
                return Err(UploadError::Rejected(
                    "not enough storage space left on the account".into(),
                ));
            }
        }

        cx.progress.set_message("Hashing file");
        let hash = md5_hex(cx.file.path.clone()).await?;

        cx.progress.set_message("Announcing upload");
        let announce = cx
            .http
            .get(format!("{API}/file/upload"))
            .query(&[
                ("name", cx.file.name.as_str()),
                ("hash", hash.as_str()),
                ("size", size.to_string().as_str()),
                ("token", token.as_str()),
            ])
            .send_reply()
            .await?;
        let announce_json = check_status(&announce, "announcing the upload")?;

        // The server already has a file with this hash: nothing to send.
        if upload_state(&announce_json) == Some(STATE_DONE) {
            return Ok(UploadedFile::new(file_url(&announce_json, &announce.body)?));
        }
        let upload_id =
            crate::reply::json_string(&announce_json, "/response/upload/upload_id", &announce.body)?;
        let upload_url =
            crate::reply::json_string(&announce_json, "/response/upload/url", &announce.body)?;

        cx.progress.set_message("Uploading");
        let form = new_form().part("file", cx.whole_file_part().await?);
        let sent = cx
            .http
            .post(upload_url)
            .multipart(form)
            .send_reply()
            .await?;
        check_status(&sent, "the upload")?;

        for poll in 1..=self.max_polls {
            cx.progress
                .set_message(format!("Waiting for the server to process the file ({poll})"));
            let info = cx
                .http
                .get(format!("{API}/file/upload_info"))
                .query(&[("upload_id", upload_id.as_str()), ("token", token.as_str())])
                .send_reply()
                .await?;
            let info_json = check_status(&info, "checking the upload")?;
            if upload_state(&info_json) == Some(STATE_DONE) {
                return Ok(UploadedFile::new(file_url(&info_json, &info.body)?));
            }
            tokio::time::sleep(self.poll_interval).await;
        }
        Err(UploadError::unexpected(
            "the server did not finish processing the file in time",
        ))
    }
}

/// MD5 of a file, computed off the async runtime. Rapidgator requires it to
/// announce an upload (and uses it to skip files it already has).
async fn md5_hex(path: std::path::PathBuf) -> Result<String, UploadError> {
    tokio::task::spawn_blocking(move || hash_file(&path))
        .await
        .map_err(|e| UploadError::unexpected(format!("hashing task failed: {e}")))?
        .map_err(UploadError::from)
}

fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Md5::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_matches_the_well_known_vector() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello");
        std::fs::write(&path, b"hello world").unwrap();
        assert_eq!(hash_file(&path).unwrap(), "5eb63bbbe01eeed093cb22bb8f5acdc3");
    }
}
