//! Download.gg — multipart `POST`; the plain-text reply is the file id.

use async_trait::async_trait;

use crate::error::UploadError;
use crate::reply::SendReply;
use crate::site::{Site, SiteInfo};
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

pub struct DownloadGg;

const INFO: SiteInfo = SiteInfo::new("Download.gg", "https://download.gg/", "https://download.gg/")
    .max_size(SizeLimit::gb(25))
    .known_issue(
        "The Python version never actually selected this site because of a name mismatch bug, \
         so it has had little real-world testing.",
    );

#[async_trait]
impl Site for DownloadGg {
    fn info(&self) -> &SiteInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let form = new_form().part("file[]", cx.whole_file_part().await?);
        let reply = cx
            .http
            .post("https://download.gg/server/upload.php")
            .multipart(form)
            .send_reply()
            .await?
            .ok()?;

        let id = reply.body.trim();
        if id.is_empty() {
            return Err(UploadError::protocol("reply has no file id", &reply.body));
        }
        // The site uses `_` where its upload script answers with `&`.
        Ok(UploadedFile::new(format!(
            "https://download.gg/file-{}",
            id.replace('&', "_")
        )))
    }
}
