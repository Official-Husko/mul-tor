//! AnonTransfer — plain Dropzone-style chunked upload.

use async_trait::async_trait;
use uuid::Uuid;

use super::dropzone::{Dropzone, Headers};
use crate::error::UploadError;
use crate::reply::Reply;
use crate::site::{Site, SiteInfo};
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile};

const INFO: SiteInfo = SiteInfo::new(
    "AnonTransfer",
    "https://anontransfer.com/",
    "https://anontransfer.com/",
)
.max_size(SizeLimit::gb(5));

pub struct AnonTransfer {
    chunk_size: u64,
}

impl Default for AnonTransfer {
    fn default() -> Self {
        Self {
            chunk_size: 52_428_800,
        }
    }
}

impl AnonTransfer {
    #[doc(hidden)]
    pub fn with_chunk_size(chunk_size: u64) -> Self {
        Self { chunk_size }
    }
}

#[async_trait]
impl Site for AnonTransfer {
    fn info(&self) -> &SiteInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let uuid = Uuid::new_v4().to_string();
        let chunks = Dropzone {
            url: "https://anontransfer.com/upload.php",
            uuid: &uuid,
            chunk_size: self.chunk_size,
            file_field: "file",
            fields: vec![],
        };
        let mut headers: Headers = vec![];
        let last: Reply = chunks.upload(cx, &mut headers, |_, _| {}).await?;

        let name = last.json_string("/name")?;
        let dir = last.json_string("/dir")?;
        Ok(UploadedFile::new(format!(
            "https://anontransfer.com/download/{dir}/{name}"
        )))
    }
}
