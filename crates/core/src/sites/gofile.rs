//! Gofile — ask the API which server to use, then upload there.

use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;

use crate::error::UploadError;
use crate::reply::SendReply;
use crate::site::{Site, SiteInfo};
use crate::upload::{UploadContext, UploadedFile, new_form};

const INFO: SiteInfo = SiteInfo::new("Gofile", "https://gofile.io/", "https://gofile.io/api/")
    .api_docs("https://gofile.io/api");

/// Gives up after this many failed attempts to get a server. The Python
/// version retried forever.
const SERVER_ATTEMPTS: usize = 5;

pub struct Gofile {
    retry_delay: Duration,
    rate_limit_delay: Duration,
}

impl Default for Gofile {
    fn default() -> Self {
        Self {
            retry_delay: Duration::from_secs(30),
            rate_limit_delay: Duration::from_secs(300),
        }
    }
}

impl Gofile {
    #[doc(hidden)]
    pub fn with_delays(retry_delay: Duration, rate_limit_delay: Duration) -> Self {
        Self {
            retry_delay,
            rate_limit_delay,
        }
    }

    async fn pick_server(&self, cx: &UploadContext<'_>) -> Result<String, UploadError> {
        for attempt in 1..=SERVER_ATTEMPTS {
            cx.progress.set_message("Requesting upload server");
            let reply = cx
                .http
                .get("https://api.gofile.io/getServer")
                .send_reply()
                .await?;
            let json: Option<Value> = reply.json().ok();
            let status = json
                .as_ref()
                .and_then(|j| j.pointer("/status"))
                .and_then(Value::as_str);

            let delay = match status {
                Some("ok") => {
                    let server = json
                        .as_ref()
                        .and_then(|j| j.pointer("/data/server"))
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    // The name becomes part of a hostname; refuse anything else.
                    if !server.is_empty()
                        && server
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '-')
                    {
                        return Ok(server.to_string());
                    }
                    return Err(UploadError::protocol("unusable server name", &reply.body));
                }
                Some("error-rateLimit") => self.rate_limit_delay,
                _ => self.retry_delay,
            };

            if attempt < SERVER_ATTEMPTS {
                cx.progress.set_message(format!(
                    "No upload server yet, retrying ({attempt}/{SERVER_ATTEMPTS})"
                ));
                tokio::time::sleep(delay).await;
            }
        }
        Err(UploadError::unexpected(
            "Gofile did not hand out an upload server",
        ))
    }
}

#[async_trait]
impl Site for Gofile {
    fn info(&self) -> &SiteInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let server = self.pick_server(cx).await?;

        cx.progress.set_message("Uploading");
        let form = new_form().part("file", cx.whole_file_part().await?);
        let reply = cx
            .http
            .post(format!("https://{server}.gofile.io/uploadFile"))
            .multipart(form)
            .send_reply()
            .await?
            .ok()?;

        let json = reply.json()?;
        if json.pointer("/status").and_then(Value::as_str) != Some("ok") {
            return Err(UploadError::protocol(
                "upload was not accepted",
                &reply.body,
            ));
        }
        Ok(UploadedFile::new(reply.json_string("/data/downloadPage")?))
    }
}
