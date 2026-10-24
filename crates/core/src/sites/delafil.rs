//! Delafil — single multipart `POST` (the upload handler is a jQuery File
//! Upload backend, but it is sent whole rather than chunked).

use async_trait::async_trait;

use crate::error::UploadError;
use crate::reply::SendReply;
use crate::site::{Site, SiteInfo};
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

pub struct Delafil;

const INFO: SiteInfo = SiteInfo::new("Delafil", "https://delafil.se/", "https://delafil.se/")
    .max_size(SizeLimit::gb(6));

#[async_trait]
impl Site for Delafil {
    fn info(&self) -> &SiteInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let form = new_form()
            .text("maxChunkSize", "2000000000")
            .part("files[]", cx.whole_file_part().await?);
        let reply = cx
            .http
            .post("https://delafil.se/core/page/ajax/file_upload_handler.ajax.php")
            .multipart(form)
            .send_reply()
            .await?
            .ok()?;
        Ok(UploadedFile::new(reply.json_string("/0/url")?))
    }
}
