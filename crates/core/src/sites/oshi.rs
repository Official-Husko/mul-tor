//! Oshi — multipart `POST`; the plain-text reply lists a `DL:` line with the link.

use async_trait::async_trait;

use crate::error::UploadError;
use crate::reply::SendReply;
use crate::site::{Site, SiteInfo};
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

pub struct Oshi;

const INFO: SiteInfo = SiteInfo::new("Oshi", "https://oshi.at/", "https://oshi.at/cmd")
    .max_size(SizeLimit::gb(5))
    .known_issue("Very slow server; the Python version recommended keeping it last.");

#[async_trait]
impl Site for Oshi {
    fn info(&self) -> &SiteInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let form = new_form().part("file", cx.whole_file_part().await?);
        let reply = cx
            .http
            .post("https://oshi.at/?shorturl=1")
            .multipart(form)
            .send_reply()
            .await?
            .expect_status(200)?;

        // Reply looks like "MANAGE: <url>\nDL: <url>\n"; take the last DL line.
        let link = reply
            .body
            .lines()
            .rev()
            .find_map(|line| line.strip_prefix("DL: "))
            .map(str::trim)
            .filter(|link| !link.is_empty())
            .ok_or_else(|| UploadError::protocol("reply has no DL line", &reply.body))?;
        Ok(UploadedFile::new(link))
    }
}
