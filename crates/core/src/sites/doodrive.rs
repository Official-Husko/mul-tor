//! DooDrive — initialise an upload session, send the chunks the server asks
//! for, then complete the session.

use async_trait::async_trait;
use serde_json::Value;

use crate::error::UploadError;
use crate::reply::SendReply;
use crate::site::{CredentialField, Site, SiteInfo};
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, chunk_ranges, new_form};

pub struct DooDrive;

const INFO: SiteInfo = SiteInfo::new("DooDrive", "https://doodrive.com/", "https://doodrive.com/")
    .max_size(SizeLimit::gb(2))
    .requires(&[
        CredentialField::new("apiKey", "API key", true),
        CredentialField::new("apiToken", "API token", true),
    ])
    .api_docs("https://doodrive.com/dashboard/api")
    .credentials_url("https://doodrive.com/dashboard/settings#settings_api");

fn expect_success(json: &Value, raw: &str, what: &str) -> Result<(), UploadError> {
    if json.get("status").and_then(Value::as_str) == Some("success") {
        Ok(())
    } else {
        Err(UploadError::protocol(format!("{what} failed"), raw))
    }
}

#[async_trait]
impl Site for DooDrive {
    fn info(&self) -> &SiteInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let api_key = cx.credential("apiKey")?;
        let api_token = cx.credential("apiToken")?;
        let size = cx.file.size;

        cx.progress.set_message("Starting upload session");
        let init = cx
            .http
            .post("https://doodrive.com/api/v1/upload")
            .form(&[
                ("api_key", api_key),
                ("api_token", api_token),
                ("file_name", cx.file.name.as_str()),
                ("file_size", size.to_string().as_str()),
            ])
            .send_reply()
            .await?
            .ok()?;
        expect_success(&init.json()?, &init.body, "starting the upload session")?;

        let token = init.json_string("/data/token")?;
        let chunk_url = init.json_string("/data/chunk_url")?;
        let complete_url = init.json_string("/data/complete_url")?;
        let chunk_size = init.json_u64("/data/max_chunk_size")?;
        if chunk_size == 0 {
            return Err(UploadError::protocol("server announced a zero chunk size", &init.body));
        }
        let total_chunks = size.div_ceil(chunk_size);

        for (index, (offset, len)) in chunk_ranges(size, chunk_size).enumerate() {
            cx.progress
                .set_message(format!("Chunk {}/{}", index + 1, total_chunks));
            let form = new_form()
                .text("api_key", api_key.to_string())
                .text("api_token", api_token.to_string())
                .text("token", token.clone())
                .text("chunk_id", index.to_string())
                .part("file", cx.file_part(offset, len).await?);
            let reply = cx
                .http
                .post(&chunk_url)
                .multipart(form)
                .send_reply()
                .await?
                .ok()?;
            expect_success(&reply.json()?, &reply.body, "uploading a chunk")?;
        }

        cx.progress.set_message("Completing upload");
        let done = cx
            .http
            .post(&complete_url)
            .form(&[
                ("api_key", api_key),
                ("api_token", api_token),
                ("token", token.as_str()),
                ("chunks", total_chunks.to_string().as_str()),
            ])
            .send_reply()
            .await?
            .ok()?;
        Ok(UploadedFile::new(done.json_string("/data/url")?))
    }
}
