//! Catbox and its temporary-file sibling LitterBox share one API.

use async_trait::async_trait;

use crate::error::UploadError;
use crate::reply::SendReply;
use crate::site::{Site, SiteInfo};
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

pub struct Catbox;

const CATBOX: SiteInfo = SiteInfo::new("Catbox", "https://catbox.moe/", "https://catbox.moe/")
    .max_size(SizeLimit::mb(200))
    .api_docs("https://catbox.moe/tools.php");

#[async_trait]
impl Site for Catbox {
    fn info(&self) -> &SiteInfo {
        &CATBOX
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        upload(
            cx,
            "https://catbox.moe/user/api.php",
            "https://files.catbox.moe/",
            None,
        )
        .await
    }
}

pub struct LitterBox;

const LITTERBOX: SiteInfo = SiteInfo::new(
    "LitterBox",
    "https://litterbox.catbox.moe/",
    "https://litterbox.catbox.moe/",
)
.max_size(SizeLimit::gb(1))
.api_docs("https://litterbox.catbox.moe/tools.php");

#[async_trait]
impl Site for LitterBox {
    fn info(&self) -> &SiteInfo {
        &LITTERBOX
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        upload(
            cx,
            "https://litterbox.catbox.moe/resources/internals/api.php",
            "https://litter.catbox.moe/",
            // Files expire after 72 hours.
            Some("72h"),
        )
        .await
    }
}

async fn upload(
    cx: &UploadContext<'_>,
    endpoint: &str,
    link_prefix: &str,
    expiry: Option<&str>,
) -> Result<UploadedFile, UploadError> {
    let mut form = new_form().text("reqtype", "fileupload");
    if let Some(expiry) = expiry {
        form = form.text("time", expiry.to_string());
    }
    let form = form.part("fileToUpload", cx.whole_file_part().await?);

    let reply = cx
        .http
        .post(endpoint)
        .multipart(form)
        .send_reply()
        .await?
        .ok()?;

    // On success the body is just the link; errors come back as prose.
    let link = reply.body.trim();
    if link.starts_with(link_prefix) {
        Ok(UploadedFile::new(link))
    } else {
        Err(UploadError::protocol(
            "reply is not a download link",
            &reply.body,
        ))
    }
}
