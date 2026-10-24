//! tmpfiles.org — multipart `POST`, JSON reply.

use async_trait::async_trait;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::SendReply;
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

pub struct TmpFiles;

const INFO: ProviderInfo = ProviderInfo::new(
    "TmpFiles",
    "https://tmpfiles.org/",
    "https://tmpfiles.org/api",
)
.max_size(SizeLimit::mb(100))
.api_docs("https://tmpfiles.org/api");

#[async_trait]
impl Provider for TmpFiles {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let form = new_form().part("file", cx.whole_file_part().await?);
        let reply = cx
            .http
            .post("https://tmpfiles.org/api/v1/upload")
            .multipart(form)
            .send_reply()
            .await?
            .expect_status(200)?;
        Ok(UploadedFile::new(reply.json_string("/data/url")?))
    }
}
