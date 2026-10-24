//! Download.gg — multipart `POST`; the plain-text reply is the file id.

use async_trait::async_trait;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::SendReply;
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

pub struct DownloadGg;

const INFO: ProviderInfo = ProviderInfo::new(
    "Download.gg",
    "https://download.gg/",
    "https://download.gg/",
)
.max_size(SizeLimit::gb(25));
// Mul-Tor 1.x never offered this provider because of a naming bug, so its
// protocol has not been exercised against the live service. Unverified.

#[async_trait]
impl Provider for DownloadGg {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let form = new_form().part("file[]", cx.whole_file_part().await?);
        let reply = cx
            .http
            .post("https://download.gg/server/upload.php")
            .multipart(form)
            .send_reply()
            .await?
            .ok()?;

        let id = reply.body.trim();
        if id.is_empty() {
            return Err(UploadError::protocol("reply has no file id", &reply.body));
        }
        // The provider uses `_` where its upload script answers with `&`.
        Ok(UploadedFile::new(format!(
            "https://download.gg/file-{}",
            id.replace('&', "_")
        )))
    }
}
