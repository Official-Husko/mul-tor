//! curl.by — the file is the body of a `POST` to `/<name>`; the reply is a text
//! page that ends in the `wget` command for the new link.

use std::sync::LazyLock;

use async_trait::async_trait;
use regex::Regex;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::{SendReply, capture};
use crate::upload::{UploadContext, UploadedFile, url_with_segments};

pub struct CurlBy;

const INFO: ProviderInfo = ProviderInfo::new("curl.by", "https://curl.by/", "https://curl.by/")
    .api_docs("https://curl.by/")
    .verified("2026-10-24");

/// `wget https://curl.by/tio_C/hello.txt` on a line of its own.
static WGET_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^wget\s+(https://curl\.by/\S+)\s*$").expect("pattern is valid")
});

#[async_trait]
impl Provider for CurlBy {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let url = url_with_segments("https://curl.by/", &[&cx.file.name])?;
        let request = cx
            .http
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/octet-stream");
        let reply = cx.raw_upload(request).await?.send_reply().await?.ok()?;

        match capture(&WGET_LINE, &reply.body) {
            Some(link) => Ok(UploadedFile::new(link)),
            None => Err(UploadError::protocol(
                "reply has no download link",
                &reply.body,
            )),
        }
    }
}
