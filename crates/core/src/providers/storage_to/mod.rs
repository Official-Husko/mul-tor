//! Storage.to — ask the API for presigned storage URLs, send the bytes straight
//! to storage, then confirm to get the link.
//!
//! An anonymous upload may be kept for 7 days at most, which is always asked
//! for. Files above 50 MB go up in parts and need one more call to assemble them.

use async_trait::async_trait;
use reqwest::header::ACCEPT;
use serde_json::json;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::SendReply;
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, is_web_link};

mod parts;

const INFO: ProviderInfo =
    ProviderInfo::new("Storage.to", "https://storage.to/", "https://storage.to/")
        .max_size(SizeLimit::gb(25))
        .api_docs("https://storage.to/docs/api")
        .verified("2026-10-24");

const API: &str = "https://storage.to/api/upload";

/// Header that proves who started a multi-part upload.
const OWNER_HEADER: &str = "X-Owner-Token";

/// Days a file is kept: the most an anonymous upload may ask for.
const EXPIRY_DAYS: u32 = 7;

const CONTENT_TYPE: &str = "application/octet-stream";

pub struct StorageTo;

#[async_trait]
impl Provider for StorageTo {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        cx.progress.set_message("Starting upload");
        // Not kept in an error: the reply holds signed URLs and the owner token.
        let init = cx
            .http
            .post(format!("{API}/init"))
            .header(ACCEPT, "application/json")
            .json(&json!({
                "filename": cx.file.name,
                "content_type": CONTENT_TYPE,
                "size": cx.file.size,
                "expiry_days": EXPIRY_DAYS,
            }))
            .send_reply()
            .await?
            .ok()?;
        let plan = parts::Plan::from_reply(&init)?;

        match &plan {
            parts::Plan::Single { .. } => parts::send_single(cx, &plan).await?,
            parts::Plan::Multipart { .. } => parts::send_multipart(cx, &plan).await?,
        }

        cx.progress.set_message("Confirming");
        let mut request = cx
            .http
            .post(format!("{API}/confirm"))
            .header(ACCEPT, "application/json");
        if let Some(token) = plan.owner_token() {
            request = request.header(OWNER_HEADER, token);
        }
        let done = request
            .json(&json!({
                "filename": cx.file.name,
                "size": cx.file.size,
                "content_type": CONTENT_TYPE,
                "r2_key": plan.key(),
                "expiry_days": EXPIRY_DAYS,
            }))
            .send_reply()
            .await?
            .ok()?;

        // A refusal comes back as `{"success": false, "error": "…"}` with status 200.
        let link = done.json_string("/file/url").map_err(|_| {
            let why = done.json_string("/error").unwrap_or_default();
            UploadError::protocol(format!("upload not confirmed: {why}"), &done.body)
        })?;
        if is_web_link(&link) {
            Ok(UploadedFile::new(link))
        } else {
            Err(UploadError::protocol(
                "reply is not a download link",
                &done.body,
            ))
        }
    }
}
