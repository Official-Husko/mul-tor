//! uFile — pick a storage server, open a session, send chunks, finalise.

use async_trait::async_trait;

use crate::error::UploadError;
use crate::reply::SendReply;
use crate::site::{Site, SiteInfo};
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, chunk_ranges, new_form};

const INFO: SiteInfo = SiteInfo::new("uFile", "https://ufile.io/", "https://ufile.io/")
    .max_size(SizeLimit::gb(5))
    .api_docs("https://ufile.io/blog/?filter=api");

pub struct UFile {
    chunk_size: u64,
}

impl Default for UFile {
    fn default() -> Self {
        Self {
            chunk_size: 69_512_747,
        }
    }
}

impl UFile {
    #[doc(hidden)]
    pub fn with_chunk_size(chunk_size: u64) -> Self {
        Self { chunk_size }
    }
}

#[async_trait]
impl Site for UFile {
    fn info(&self) -> &SiteInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let size = cx.file.size;
        let total_chunks = size.div_ceil(self.chunk_size);

        cx.progress.set_message("Selecting storage server");
        let storage = cx
            .http
            .post("https://ufile.io/v1/upload/select_storage")
            .header("Accept", "application/json")
            .send_reply()
            .await?
            .ok()?;
        let mut server = storage.json_string("/storageBaseUrl")?;
        if !server.starts_with("https://") {
            return Err(UploadError::protocol(
                "unusable storage server",
                &storage.body,
            ));
        }
        if !server.ends_with('/') {
            server.push('/');
        }

        let session = cx
            .http
            .post(format!("{server}v1/upload/create_session"))
            .header("Accept", "application/json")
            .form(&[("file_size", size.to_string())])
            .send_reply()
            .await?
            .ok()?;
        let fuid = session.json_string("/fuid")?;

        for (index, (offset, len)) in chunk_ranges(size, self.chunk_size).enumerate() {
            cx.progress
                .set_message(format!("Chunk {}/{}", index + 1, total_chunks));
            // The server counts chunks from 1.
            let form = new_form()
                .text("fuid", fuid.clone())
                .text("chunk_index", (index + 1).to_string())
                .part("file", cx.file_part(offset, len).await?);
            cx.http
                .post(format!("{server}v1/upload/chunk"))
                .header("Accept", "application/json")
                .multipart(form)
                .send_reply()
                .await?
                .ok()?;
        }

        cx.progress.set_message("Finalising");
        let done = cx
            .http
            .post(format!("{server}v1/upload/finalise"))
            .header("Accept", "application/json")
            .form(&[
                ("fuid", fuid.as_str()),
                ("file_name", cx.file.name.as_str()),
                ("file_type", "unknown"),
                ("total_chunks", total_chunks.to_string().as_str()),
            ])
            .send_reply()
            .await?
            .ok()?;
        Ok(UploadedFile::new(done.json_string("/url")?))
    }
}
