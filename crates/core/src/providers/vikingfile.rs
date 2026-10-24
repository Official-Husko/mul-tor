//! VikingFile — ask which upload server to use, then POST the file there; an
//! empty `user` makes the upload anonymous.
//!
//! The site also documents a multipart flow with presigned links. Those point
//! at a third-party storage host that cannot be checked against VikingFile's
//! own domain, so the single POST its API page lists as "legacy" is used.

use async_trait::async_trait;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::SendReply;
use crate::upload::{UploadContext, UploadedFile, https_url_within, is_web_link, new_form};

pub struct VikingFile;

const INFO: ProviderInfo = ProviderInfo::new(
    "VikingFile",
    "https://vikingfile.com/",
    "https://vikingfile.com/",
)
.api_docs("https://vikingfile.com/api")
.verified("2026-10-24");

#[async_trait]
impl Provider for VikingFile {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        cx.progress.set_message("Requesting upload server");
        let server = cx
            .http
            .get("https://vikingfile.com/api/get-server")
            .send_reply()
            .await?
            .ok()?;
        // The address is requested next, with the file: it must stay on VikingFile.
        let address = server.json_string("/server")?;
        let Some(address) = https_url_within(&address, "vikingfile.com") else {
            return Err(UploadError::protocol(
                "unusable upload server",
                &server.body,
            ));
        };

        cx.progress.set_message("Uploading");
        let form = new_form()
            .text("user", "")
            .part("file", cx.whole_file_part().await?);
        let reply = cx
            .http
            .post(address)
            .multipart(form)
            .send_reply()
            .await?
            .ok()?;

        let link = reply.json_string("/url")?;
        if is_web_link(&link) {
            Ok(UploadedFile::new(link))
        } else {
            Err(UploadError::protocol(
                "reply is not a download link",
                &reply.body,
            ))
        }
    }
}
