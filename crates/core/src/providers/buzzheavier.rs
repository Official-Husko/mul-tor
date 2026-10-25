//! Buzzheavier — raw `PUT`, JSON reply with the file id.

use async_trait::async_trait;
use reqwest::header::ACCEPT;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::SendReply;
use crate::upload::{UploadContext, UploadedFile, url_with_segments};

pub struct Buzzheavier;

const INFO: ProviderInfo = ProviderInfo::new(
    "Buzzheavier",
    "https://buzzheavier.com/",
    "https://buzzheavier.com/",
)
.api_docs("https://buzzheavier.com/api.html");

#[async_trait]
impl Provider for Buzzheavier {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let url = url_with_segments("https://w.buzzheavier.com/", &[&cx.file.name])?;
        let request = cx.http.put(url).header(ACCEPT, "*/*");
        let reply = cx
            .raw_upload(request)
            .await?
            .send_reply()
            .await?
            .expect_status(201)?;

        // The id sits at the top level in older replies and under `data` in newer ones.
        let id = reply
            .json_string("/id")
            .or_else(|_| reply.json_string("/data/id"))?;
        Ok(UploadedFile::new(format!("https://buzzheavier.com/f/{id}")))
    }
}
