//! Mixdrop — multipart `POST` authenticated with e-mail + API key.

use async_trait::async_trait;

use crate::error::UploadError;
use crate::provider::{CredentialField, Provider, ProviderInfo};
use crate::reply::SendReply;
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

pub struct Mixdrop;

const INFO: ProviderInfo = ProviderInfo::new(
    "Mixdrop",
    "https://mixdrop.ag/",
    "https://ul.mixdrop.ag/api",
)
.max_size(SizeLimit::gb(10))
.requires(&[
    CredentialField::new("email", "Account e-mail", false),
    CredentialField::new("apiKey", "API key", true),
])
.api_docs("https://mixdrop.ag/api/")
.credentials_url("https://mixdrop.ag/api/");

#[async_trait]
impl Provider for Mixdrop {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let form = new_form()
            .text("email", cx.credential("email")?.to_string())
            .text("key", cx.credential("apiKey")?.to_string())
            .part("file", cx.whole_file_part().await?);
        let reply = cx
            .http
            .post("https://ul.mixdrop.ag/api")
            .multipart(form)
            .send_reply()
            .await?
            .expect_status(200)?;

        let json = reply.json()?;
        if json.get("success").and_then(|s| s.as_bool()) == Some(false) {
            return Err(UploadError::protocol(
                "Mixdrop refused the upload",
                &reply.body,
            ));
        }
        let file_ref = reply.json_string("/result/fileref")?;
        Ok(UploadedFile::new(format!(
            "https://mixdrop.ag/f/{file_ref}"
        )))
    }
}
