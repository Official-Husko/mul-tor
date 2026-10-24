//! Files.dp.ua — multipart `POST` with client-chosen ids; the link is built
//! from the id the client picked.

use async_trait::async_trait;

use crate::error::UploadError;
use crate::reply::SendReply;
use crate::site::{Site, SiteInfo};
use crate::size::SizeLimit;
use crate::upload::{MIXED_ALNUM, UploadContext, UploadedFile, new_form, random_string};

pub struct FilesDpUa;

const INFO: SiteInfo = SiteInfo::new(
    "Files.dp.ua",
    "https://files.dp.ua/",
    "https://files2.dp.ua/upload.php",
)
.max_size(SizeLimit::gb(100));

#[async_trait]
impl Site for FilesDpUa {
    fn info(&self) -> &SiteInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let file_id = random_string(10, MIXED_ALNUM);
        let access_code = random_string(7, MIXED_ALNUM);

        let form = new_form()
            .text("idfile", file_id.clone())
            .text("accesscode", access_code)
            .part("files[]", cx.whole_file_part().await?);
        cx.http
            .post("https://files2.dp.ua/upload.php")
            .multipart(form)
            .send_reply()
            .await?
            .expect_status(200)?;

        Ok(UploadedFile::new(format!(
            "https://files.dp.ua/en/{file_id}"
        )))
    }
}
