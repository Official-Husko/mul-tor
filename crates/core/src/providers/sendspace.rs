//! Sendspace — the home page carries a signed upload address; the file goes to
//! it as the page's own form would send it, and the page that answers holds the
//! download link.

use std::sync::LazyLock;

use async_trait::async_trait;
use regex::Regex;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::{SendReply, capture};
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, https_url_within, new_form};

const INFO: ProviderInfo = ProviderInfo::new(
    "Sendspace",
    "https://www.sendspace.com/",
    "https://www.sendspace.com/",
)
.max_size(SizeLimit::mb(300))
.verified("2026-10-25");

/// `<form … action="https://fs12u.sendspace.com/upload?…signature=…">`.
static ACTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"action="(https://[a-z0-9.-]+/upload\?[^"]+)""#).expect("pattern is valid")
});
static PROGRESS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"name="PROGRESS_URL" value="([^"]+)""#).expect("pattern is valid")
});
static SIGNATURE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"name="signature" value="([^"]+)""#).expect("pattern is valid"));
/// The first `https://www.sendspace.com/file/<id>` on the result page; the
/// `/delete/` link next to it would delete the file and is never taken.
static FILE_LINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"https://www\.sendspace\.com/file/[A-Za-z0-9]+"#).expect("pattern is valid")
});

pub struct Sendspace;

/// The text of an HTML attribute, with its `&amp;` written as `&`.
fn attribute(captured: String) -> String {
    captured.replace("&amp;", "&")
}

#[async_trait]
impl Provider for Sendspace {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        cx.progress.set_message("Finding upload server");
        let home = cx
            .http
            .get("https://www.sendspace.com/")
            .send_reply()
            .await?
            .ok()?;
        let found = (
            capture(&ACTION, &home.body).map(attribute),
            capture(&PROGRESS, &home.body).map(attribute),
            capture(&SIGNATURE, &home.body).map(attribute),
        );
        let (Some(action), Some(progress), Some(signature)) = found else {
            return Err(UploadError::unexpected("could not find the upload form"));
        };
        // The file is sent to this address: it must be one of Sendspace's own servers.
        if https_url_within(&action, "sendspace.com").is_none() {
            return Err(UploadError::unexpected("unusable upload server"));
        }

        cx.progress.set_message("Uploading");
        let form = new_form()
            .text("PROGRESS_URL", progress)
            .text("js_enabled", "0")
            .text("signature", signature)
            .text("upload_files", "")
            .text("terms", "1")
            .part("upload_file[]", cx.whole_file_part().await?);
        let reply = cx
            .http
            .post(&action)
            .multipart(form)
            .send_reply()
            .await?
            .ok()?;

        match FILE_LINK.find(&reply.body) {
            Some(link) => Ok(UploadedFile::new(link.as_str())),
            None => Err(UploadError::protocol(
                "the answer has no download link",
                &reply.body,
            )),
        }
    }
}
