//! BowFile — authorise with two API keys for a token, then upload with it.

use async_trait::async_trait;

use crate::error::UploadError;
use crate::reply::SendReply;
use crate::site::{CredentialField, Site, SiteInfo};
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

pub struct BowFile;

const INFO: SiteInfo = SiteInfo::new("BowFile", "https://bowfile.com/", "https://bowfile.com/")
    .max_size(SizeLimit::gb(5))
    .requires(&[
        CredentialField::new("apiKey1", "API key 1", true),
        CredentialField::new("apiKey2", "API key 2", true),
    ])
    .api_docs("https://bowfile.com/api")
    .credentials_url("https://bowfile.com/account/edit#api");

#[async_trait]
impl Site for BowFile {
    fn info(&self) -> &SiteInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let key1 = cx.credential("apiKey1")?;
        let key2 = cx.credential("apiKey2")?;

        cx.progress.set_message("Authorising");
        let auth = cx
            .http
            .post("https://bowfile.com/api/v2/authorize")
            .form(&[("key1", key1), ("key2", key2)])
            .send_reply()
            .await?
            .ok()?;
        let json = auth.json()?;
        if json.get("_status").and_then(|s| s.as_str()) != Some("success") {
            return Err(UploadError::protocol(
                "authorisation failed (are the API keys correct?)",
                &auth.body,
            ));
        }
        let access_token = auth.json_string("/data/access_token")?;
        let account_id = auth.json_string("/data/account_id")?;

        cx.progress.set_message("Uploading");
        let form = new_form()
            .text("access_token", access_token)
            .text("account_id", account_id)
            .part("upload_file", cx.whole_file_part().await?);
        let reply = cx
            .http
            .post("https://bowfile.com/api/v2/file/upload")
            .multipart(form)
            .send_reply()
            .await?
            .ok()?;
        Ok(UploadedFile::new(reply.json_string("/data/0/url")?))
    }
}
