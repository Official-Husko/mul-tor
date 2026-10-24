//! Pixeldrain — raw `PUT` with the API key as the `pd_auth_key` cookie.

use async_trait::async_trait;
use reqwest::header::{CONTENT_TYPE, COOKIE};

use crate::error::UploadError;
use crate::provider::{CredentialField, Provider, ProviderInfo};
use crate::reply::SendReply;
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, url_with_segments};

pub struct Pixeldrain;

const INFO: ProviderInfo = ProviderInfo::new(
    "Pixeldrain",
    "https://pixeldrain.com/",
    "https://pixeldrain.com/api/",
)
.max_size(SizeLimit::gb(20))
.requires(&[CredentialField::new("apiKey", "API key", true)])
.api_docs("https://pixeldrain.com/api")
.credentials_url("https://pixeldrain.com/user/api_keys");

#[async_trait]
impl Provider for Pixeldrain {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let key = cx.credential("apiKey")?;
        let url = url_with_segments("https://pixeldrain.com/api/file/", &[&cx.file.name])?;

        let request = cx
            .http
            .put(url)
            .header(CONTENT_TYPE, "application/octet-stream")
            .header(COOKIE, format!("pd_auth_key={key}"));
        let reply = cx
            .raw_upload(request)
            .await?
            .send_reply()
            .await?
            .expect_status(201)?;

        let id = reply.json_string("/id")?;
        Ok(UploadedFile::new(format!("https://pixeldrain.com/u/{id}")))
    }
}
