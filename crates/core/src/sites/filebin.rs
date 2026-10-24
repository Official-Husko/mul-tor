//! FileBin — raw `POST` to a randomly named bin.

use async_trait::async_trait;
use reqwest::header::{ACCEPT, CONTENT_TYPE};

use crate::error::UploadError;
use crate::reply::SendReply;
use crate::site::{Site, SiteInfo};
use crate::upload::{LOWER_ALNUM, UploadContext, UploadedFile, random_string, url_with_segments};

pub struct FileBin;

const INFO: SiteInfo = SiteInfo::new("FileBin", "https://filebin.net/", "https://filebin.net/");

#[async_trait]
impl Site for FileBin {
    fn info(&self) -> &SiteInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        // Bins are created on first use, so a random name is the whole "auth".
        let bin = random_string(16, LOWER_ALNUM);
        let url = url_with_segments("https://filebin.net/", &[&bin, &cx.file.name])?;

        let request = cx
            .http
            .post(&url)
            .header(CONTENT_TYPE, "application/octet-stream")
            .header(ACCEPT, "application/json");
        cx.raw_upload(request)
            .await?
            .send_reply()
            .await?
            .expect_status(201)?;

        Ok(UploadedFile::new(url))
    }
}
