//! File.io — anonymous multipart `POST`, then a best-effort `PATCH` for metadata.

use async_trait::async_trait;
use serde_json::json;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::SendReply;
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

pub struct FileIo;

const INFO: ProviderInfo =
    ProviderInfo::new("File.io", "https://file.io/", "https://file.io/").max_size(SizeLimit::gb(2));

#[async_trait]
impl Provider for FileIo {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let form = new_form().part("file", cx.whole_file_part().await?);
        let reply = cx
            .http
            .post("https://file.io/")
            .query(&[("title", cx.file.name.as_str())])
            .multipart(form)
            .send_reply()
            .await?
            .ok()?;

        let link = reply.json_string("/link")?;

        // Anonymous uploads hand back a token that allows editing the entry.
        // The upload has already worked at this point, so a failure here only
        // costs the description and expiry, not the link.
        let token = reply.header("x-file-io-anonymous-access-token");
        if let (Some(token), Ok(key)) = (token, reply.json_string("/key")) {
            cx.progress.set_message("Setting expiry");
            let patch = cx
                .http
                .patch(format!("https://file.io/{key}"))
                .bearer_auth(token)
                .json(&json!({
                    "title": cx.file.name,
                    "description": "Uploaded using Mul-Tor on GitHub!",
                    "expires": "2M",
                }))
                .send_reply()
                .await;
            match patch {
                Ok(patch) if patch.status.is_success() => {}
                Ok(patch) => {
                    tracing::warn!(status = %patch.status, "File.io metadata update was refused")
                }
                Err(error) => tracing::warn!(%error, "File.io metadata update failed"),
            }
        }

        Ok(UploadedFile::new(link))
    }
}
