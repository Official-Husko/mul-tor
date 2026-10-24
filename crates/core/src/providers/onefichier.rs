//! 1Fichier — anonymous upload API: get a server + id, upload, then ask for the links.

use async_trait::async_trait;
use reqwest::header::CONTENT_TYPE;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::SendReply;
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

pub struct OneFichier;

const INFO: ProviderInfo = ProviderInfo::new(
    "1Fichier",
    "https://1fichier.com/",
    "https://api.1fichier.com/v1/upload/",
)
.aliases(&["OneFichier"])
.max_size(SizeLimit::gb(300))
.api_docs("https://1fichier.com/api.html");

#[async_trait]
impl Provider for OneFichier {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        cx.progress.set_message("Requesting upload server");
        let server = cx
            .http
            .get("https://api.1fichier.com/v1/upload/get_upload_server.cgi")
            .header(CONTENT_TYPE, "application/json")
            .send_reply()
            .await?
            .ok()?;
        let host = server.json_string("/url")?;
        let id = server.json_string("/id")?;
        // `host` ends up in a URL; it must be a bare hostname.
        if !host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
        {
            return Err(UploadError::protocol("unusable server name", &server.body));
        }

        cx.progress.set_message("Uploading");
        let form = new_form().part("file[]", cx.whole_file_part().await?);
        cx.http
            .post(format!("https://{host}/upload.cgi"))
            .query(&[("id", id.as_str())])
            .multipart(form)
            .send_reply()
            .await?
            .ok()?;

        cx.progress.set_message("Fetching link");
        let end = cx
            .http
            .get(format!("https://{host}/end.pl"))
            .query(&[("xid", id.as_str())])
            .header("JSON", "1")
            .send_reply()
            .await?
            .ok()?;
        Ok(UploadedFile::new(end.json_string("/links/0/download")?))
    }
}
