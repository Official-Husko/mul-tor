//! File-Upload.net — multipart `POST`; the link is assembled from the reply.

use async_trait::async_trait;

use crate::error::UploadError;
use crate::reply::SendReply;
use crate::site::{Site, SiteInfo};
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

pub struct FileUpload;

const INFO: SiteInfo = SiteInfo::new(
    "FileUpload",
    "https://file-upload.net/",
    "https://file-upload.net/",
)
.aliases(&["File-Upload.net"])
.max_size(SizeLimit::gb(5));

#[async_trait]
impl Site for FileUpload {
    fn info(&self) -> &SiteInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let form = new_form()
            .text("description", "Uploaded using Mul-Tor on GitHub!")
            .part("file", cx.whole_file_part().await?);
        let reply = cx
            .http
            .post("https://up.file-upload.net/upload.php")
            .multipart(form)
            .send_reply()
            .await?
            .ok()?;

        let name = reply.json_string("/name")?;
        let id = reply.json_string("/id")?;
        Ok(UploadedFile::new(format!(
            "https://file-upload.net/download-{id}/{name}.html"
        )))
    }
}
