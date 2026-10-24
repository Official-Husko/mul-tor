//! Delafil — single multipart `POST` (the upload handler is a jQuery File
//! Upload backend, but it is sent whole rather than chunked).

use async_trait::async_trait;
use reqwest::multipart::Part;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::SendReply;
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

pub struct Delafil;

const INFO: ProviderInfo =
    ProviderInfo::new("Delafil", "https://delafil.se/", "https://delafil.se/")
        .max_size(SizeLimit::gb(6));

#[async_trait]
impl Provider for Delafil {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        // The Python version passed this through `requests`' `files=`, which sends
        // a string there as a file-typed part (a filename, no Content-Type) and
        // not as a plain field. That is the shape known to work, so keep it.
        let max_chunk_size = Part::text("2000000000").file_name("maxChunkSize");
        let form = new_form()
            .part("maxChunkSize", max_chunk_size)
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
