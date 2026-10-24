//! File.si — multipart `POST`, JSON reply.

use async_trait::async_trait;

use crate::error::UploadError;
use crate::reply::SendReply;
use crate::site::{Site, SiteInfo};
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

pub struct FileSi;

const INFO: SiteInfo = SiteInfo::new("File.si", "https://file.si/", "https://file.si/")
    .max_size(SizeLimit::gb(7))
    .api_docs("https://file.si/docs/api");

#[async_trait]
impl Site for FileSi {
    fn info(&self) -> &SiteInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let form = new_form().part("file", cx.whole_file_part().await?);
        let reply = cx
            .http
            .post("https://file.si/api/v1/upload")
            .multipart(form)
            .send_reply()
            .await?
            .ok()?;
        Ok(UploadedFile::new(
            reply.json_string("/data/file/url/short")?,
        ))
    }
}
