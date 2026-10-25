//! Transfer.sh — `PUT` of the raw file; the reply body is the link.

use async_trait::async_trait;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::SendReply;
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, url_with_segments};

pub struct TransferSh;

const INFO: ProviderInfo = ProviderInfo::new(
    "Transfer.sh",
    "https://transfer.sh/",
    "https://transfer.sh/",
)
.max_size(SizeLimit::gb(10))
.degraded("The public service has been unreliable, and Mul-Tor 1.x never actually offered it because of a naming bug.");

#[async_trait]
impl Provider for TransferSh {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let url = url_with_segments("https://transfer.sh/", &[&cx.file.name])?;

        // The Python version wrapped the file in a multipart form here, which
        // transfer.sh would have stored as-is (boundary lines included). It
        // expects the plain file as the request body.
        let reply = cx
            .raw_upload(cx.http.put(url))
            .await?
            .send_reply()
            .await?
            .expect_status(200)?;

        let link = reply.body.trim();
        if link.starts_with("http") {
            Ok(UploadedFile::new(link))
        } else {
            Err(UploadError::protocol(
                "reply is not a download link",
                &reply.body,
            ))
        }
    }
}
