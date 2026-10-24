//! Anonymfile — a Laravel app: fetch the home page for the CSRF token and
//! session cookies, then upload with them.

use async_trait::async_trait;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::{SendReply, csrf_meta};
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

pub struct Anonymfile;

const INFO: ProviderInfo = ProviderInfo::new(
    "Anonymfile",
    "https://anonymfile.com/",
    "https://anonymfile.com/",
)
.max_size(SizeLimit::gb(7))
.api_docs("https://anonymfile.com/docs/api");

#[async_trait]
impl Provider for Anonymfile {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        cx.progress.set_message("Fetching session");
        let home = cx
            .http
            .get("https://anonymfile.com/")
            .send_reply()
            .await?
            .ok()?;
        let csrf = csrf_meta(&home.body);
        let xsrf = home.cookie("XSRF-TOKEN");
        let session = home.cookie("anonymfile_session");
        let (Some(csrf), Some(xsrf), Some(session)) = (csrf, xsrf, session) else {
            return Err(UploadError::protocol(
                "could not get the CSRF token and session cookies",
                &home.body,
            ));
        };

        cx.progress.set_message("Uploading");
        let form = new_form().part("file", cx.whole_file_part().await?);
        let reply = cx
            .http
            .post("https://anonymfile.com/upload")
            .header("Accept", "application/json")
            .header("X-CSRF-TOKEN", csrf)
            .header(
                "Cookie",
                format!("XSRF-TOKEN={xsrf}; anonymfile_session={session}"),
            )
            .multipart(form)
            .send_reply()
            .await?
            .expect_status(200)?;

        let id = reply.body.trim();
        if id.is_empty() {
            return Err(UploadError::protocol("reply has no file id", &reply.body));
        }
        Ok(UploadedFile::new(format!("https://anonymfile.com/{id}")))
    }
}
