//! Krakenfiles — the upload server is embedded in the home page.

use std::sync::LazyLock;

use async_trait::async_trait;
use regex::Regex;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::{SendReply, capture};
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

pub struct Krakenfiles;

const INFO: ProviderInfo = ProviderInfo::new(
    "Krakenfiles",
    "https://krakenfiles.com/",
    "https://krakenfiles.com/",
)
.max_size(SizeLimit::gb(1));

static UPLOAD_URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https://uploads\d+\.krakenfiles\.com/_uploader/gallery/upload").unwrap()
});

#[async_trait]
impl Provider for Krakenfiles {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        cx.progress.set_message("Finding upload server");
        let page = cx
            .http
            .get("https://krakenfiles.com/")
            .send_reply()
            .await?
            .ok()?;
        let upload_url = capture(&UPLOAD_URL, &page.body).ok_or_else(|| {
            UploadError::unexpected("could not find the upload server in the page")
        })?;

        cx.progress.set_message("Uploading");
        let form = new_form().part("files[]", cx.whole_file_part().await?);
        let reply = cx
            .http
            .post(upload_url)
            .multipart(form)
            .send_reply()
            .await?
            .expect_status(200)?;

        let json = reply.json()?;
        if let Some(error) = json
            .pointer("/files/0/error")
            .and_then(|e| e.as_str())
            .filter(|e| !e.is_empty())
        {
            return Err(UploadError::protocol(
                format!("Krakenfiles refused the file: {error}"),
                &reply.body,
            ));
        }
        let path = reply.json_string("/files/0/url")?;
        Ok(UploadedFile::new(format!("https://krakenfiles.com{path}")))
    }
}
