//! TempSend — multipart `POST` that redirects to the file's page; that page's
//! URL is the link.

use async_trait::async_trait;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::SendReply;
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

pub struct TempSend;

const INFO: ProviderInfo =
    ProviderInfo::new("TempSend", "https://tempsend.com/", "https://tempsend.com/")
        .max_size(SizeLimit::gb(2));

/// Keep files for a year (seconds).
const EXPIRE: &str = "31536000";

#[async_trait]
impl Provider for TempSend {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let form = new_form()
            .text("expire", EXPIRE)
            .part("file", cx.whole_file_part().await?);
        let reply = cx
            .http
            .post("https://tempsend.com/send")
            .multipart(form)
            .send_reply()
            .await?
            .ok()?;

        // Still on the upload endpoint: no redirect to a file page happened.
        if reply.url.path().trim_end_matches('/').ends_with("/send") {
            return Err(UploadError::protocol(
                "the upload was not redirected to a file page",
                &reply.body,
            ));
        }
        Ok(UploadedFile::new(reply.url.as_str()))
    }
}
