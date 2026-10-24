//! Files.fm — get an upload id + key, upload with them, then "finish" the upload.

use async_trait::async_trait;
use reqwest::Url;

use crate::error::UploadError;
use crate::reply::SendReply;
use crate::site::{Site, SiteInfo};
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

pub struct FilesFm;

const INFO: SiteInfo = SiteInfo::new(
    "FilesFm",
    "https://files.fm/",
    "https://free.files.fm/save_file.php",
)
.max_size(SizeLimit::gb(5));

#[async_trait]
impl Site for FilesFm {
    fn info(&self) -> &SiteInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        cx.progress.set_message("Requesting upload id");
        let init = cx
            .http
            .get("https://files.fm/server_scripts/get_upload_id.php?show_add_key=1")
            .send_reply()
            .await?
            .ok()?;
        // Reply is comma separated: "<id>,<something>,<key>,…".
        let fields: Vec<&str> = init.body.trim().split(',').collect();
        let (Some(id), Some(key)) = (fields.first(), fields.get(2)) else {
            return Err(UploadError::protocol(
                "unexpected upload id reply",
                &init.body,
            ));
        };
        if id.is_empty() || key.is_empty() {
            return Err(UploadError::protocol(
                "unexpected upload id reply",
                &init.body,
            ));
        }

        cx.progress.set_message("Uploading");
        let upload_url = Url::parse_with_params(
            "https://free.files.fm/save_file.php",
            &[
                ("up_id", *id),
                ("ignore_user_abort", "1"),
                ("skip_update", "1"),
                ("key", *key),
            ],
        )
        .map_err(|e| UploadError::unexpected(format!("could not build the upload URL: {e}")))?;
        let form = new_form().part("Filedata", cx.whole_file_part().await?);
        cx.http
            .post(upload_url)
            .multipart(form)
            .send_reply()
            .await?
            .ok()?;

        cx.progress.set_message("Finishing");
        let finish_url = Url::parse_with_params(
            "https://free.files.fm/finish_upload.php",
            &[("upload_hash", *id)],
        )
        .map_err(|e| UploadError::unexpected(format!("could not build the finish URL: {e}")))?;
        let finished = cx.http.get(finish_url).send_reply().await?.ok()?;
        let json = finished.json()?;
        if json.get("status").and_then(|s| s.as_str()) != Some("ok") {
            return Err(UploadError::protocol(
                "upload was not finished",
                &finished.body,
            ));
        }

        Ok(UploadedFile::new(format!("https://files.fm/u/{id}")))
    }
}
