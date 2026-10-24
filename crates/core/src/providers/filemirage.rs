//! FileMirage — ask which storage server to use, then send the file in
//! numbered chunks; the reply to the last one holds the link.
//!
//! No "auto remove" time is sent: the site's options only shorten how long a
//! file is kept, so leaving it out keeps the file the longest.

use async_trait::async_trait;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::SendReply;
use crate::size::SizeLimit;
use crate::upload::{
    LOWER_ALNUM, UploadContext, UploadedFile, chunk_ranges, https_url_within, is_web_link,
    new_form, random_string, url_with_segments,
};

const INFO: ProviderInfo = ProviderInfo::new(
    "FileMirage",
    "https://filemirage.com/",
    "https://filemirage.com/",
)
.max_size(SizeLimit::gb(50))
.verified("2026-10-24");

pub struct FileMirage {
    chunk_size: u64,
}

impl Default for FileMirage {
    fn default() -> Self {
        // What the site's own uploader uses.
        Self {
            chunk_size: 99 * 1024 * 1024,
        }
    }
}

impl FileMirage {
    #[doc(hidden)]
    pub fn with_chunk_size(chunk_size: u64) -> Self {
        Self { chunk_size }
    }
}

#[async_trait]
impl Provider for FileMirage {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        cx.progress.set_message("Requesting upload server");
        let assigned = cx
            .http
            .get("https://filemirage.com/api/servers")
            .send_reply()
            .await?
            .ok()?;
        // The address is requested next, with the file: it must stay on FileMirage.
        let server = assigned.json_string("/data/server")?;
        let Some(server) = https_url_within(&server, "filemirage.com") else {
            return Err(UploadError::protocol(
                "unusable upload server",
                &assigned.body,
            ));
        };
        let endpoint = url_with_segments(server.as_str(), &["upload.php"])?;
        // Names the file for the server while its chunks arrive.
        let upload_id = random_string(12, LOWER_ALNUM);

        let size = cx.file.size;
        let total_chunks = size.div_ceil(self.chunk_size);
        let mut last = None;
        for (index, (offset, len)) in chunk_ranges(size, self.chunk_size).enumerate() {
            cx.progress
                .set_message(format!("Chunk {}/{}", index + 1, total_chunks));
            let form = new_form()
                .part("file", cx.file_part(offset, len).await?)
                .text("filename", cx.file.name.clone())
                .text("upload_id", upload_id.clone())
                // Counted from 0.
                .text("chunk_number", index.to_string())
                .text("total_chunks", total_chunks.to_string());
            last = Some(
                cx.http
                    .post(&endpoint)
                    .multipart(form)
                    .send_reply()
                    .await?
                    .ok()?,
            );
        }

        let reply = last.ok_or(UploadError::EmptyFile)?;
        let link = reply.json_string("/data/url")?;
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
