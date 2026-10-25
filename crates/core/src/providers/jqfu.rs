//! Hosts running the jQuery File Upload server-side handler with chunking:
//! 1CloudFile, CyberFile, Filesadmin and WDHO.
//!
//! Each chunk is a multipart POST whose `Content-Range` header says which slice
//! of the file it holds; the reply to the last chunk lists the finished file.

use async_trait::async_trait;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::SendReply;
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, chunk_ranges, new_form};

pub struct JqfuProvider {
    info: ProviderInfo,
    url: &'static str,
    chunk_size: u64,
    /// Extra form fields sent with every chunk.
    fields: &'static [(&'static str, &'static str)],
    /// Send `X-Requested-With: XMLHttpRequest` (some hosts require it).
    xhr_header: bool,
}

impl JqfuProvider {
    pub const fn new(
        info: ProviderInfo,
        url: &'static str,
        chunk_size: u64,
        fields: &'static [(&'static str, &'static str)],
        xhr_header: bool,
    ) -> Self {
        Self {
            info,
            url,
            chunk_size,
            fields,
            xhr_header,
        }
    }

    #[doc(hidden)]
    pub fn with_chunk_size(mut self, chunk_size: u64) -> Self {
        self.chunk_size = chunk_size;
        self
    }
}

#[async_trait]
impl Provider for JqfuProvider {
    fn info(&self) -> &ProviderInfo {
        &self.info
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let size = cx.file.size;
        let total_chunks = size.div_ceil(self.chunk_size);

        let mut last = None;
        for (index, (offset, len)) in chunk_ranges(size, self.chunk_size).enumerate() {
            cx.progress
                .set_message(format!("Chunk {}/{}", index + 1, total_chunks));

            let mut form = new_form().text("maxChunkSize", self.chunk_size.to_string());
            for (name, value) in self.fields {
                form = form.text(*name, *value);
            }
            let form = form.part("files[]", cx.file_part(offset, len).await?);

            let mut request = cx
                .http
                .post(self.url)
                .header("Accept", "application/json")
                .header(
                    "Content-Range",
                    format!("bytes {}-{}/{}", offset, offset + len - 1, size),
                );
            if self.xhr_header {
                request = request.header("X-Requested-With", "XMLHttpRequest");
            }
            last = Some(request.multipart(form).send_reply().await?.ok()?);
        }

        let reply = last.ok_or(UploadError::EmptyFile)?;
        Ok(UploadedFile::new(reply.json_string("/0/url")?))
    }
}

const UPLOAD_SOURCE: &[(&str, &str)] = &[("uploadSource", "file_manager")];

pub(super) fn providers() -> Vec<JqfuProvider> {
    vec![
        JqfuProvider::new(
            ProviderInfo::new(
                "1CloudFile",
                "https://1cloudfile.com/",
                "https://1cloudfile.com/",
            )
            .aliases(&["OneCloudFile"])
            .max_size(SizeLimit::gb(5)),
            "https://1cloudfile.com/ajax/file_upload_handler",
            500_000_000,
            &[],
            true,
        ),
        JqfuProvider::new(
            ProviderInfo::new(
                "CyberFile",
                "https://cyberfile.me/",
                "https://cyberfile.me/",
            )
            .max_size(SizeLimit::gb(10)),
            "https://cyberfile.me/ajax/file_upload_handler",
            25_000_000,
            UPLOAD_SOURCE,
            false,
        ),
        JqfuProvider::new(
            ProviderInfo::new(
                "Filesadmin",
                "https://filesadmin.com/",
                "https://filesadmin.com/",
            )
            .max_size(SizeLimit::gb(3)),
            "https://filesadmin.com/ajax/file_upload_handler",
            100_000_000,
            UPLOAD_SOURCE,
            true,
        ),
        JqfuProvider::new(
            ProviderInfo::new("WDHO", "https://wdho.ru/", "https://wdho.ru/")
                .max_size(SizeLimit::gb(5)),
            "https://wdho.ru/core/page/ajax/file_upload_handler.ajax.php",
            100_000_000,
            &[],
            true,
        ),
    ]
}
