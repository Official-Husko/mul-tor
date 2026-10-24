//! Keep.sh — `curl --upload-file` style `PUT`; the reply body is the link.

use async_trait::async_trait;
use reqwest::header::USER_AGENT;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::SendReply;
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, url_with_segments};

pub struct Keep;

const INFO: ProviderInfo = ProviderInfo::new("Keep", "https://keep.sh/", "https://keep.sh/")
    .aliases(&["Keep.sh"])
    .max_size(SizeLimit::mb(500))
    .known_issue("Mul-Tor 1.x flagged this host as having unresolved problems on its side.");

const LINK_PREFIX: &str = "https://free.keep.sh/";

#[async_trait]
impl Provider for Keep {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let url = url_with_segments(LINK_PREFIX, &[&cx.file.name])?;

        // The service only talks to curl-like clients.
        let request = cx.http.put(url).header(USER_AGENT, "curl/8.4.0");
        let reply = cx.raw_upload(request).await?.send_reply().await?.ok()?;

        let link = reply.body.trim();
        if link.starts_with(LINK_PREFIX) {
            Ok(UploadedFile::new(link))
        } else {
            Err(UploadError::protocol(
                "reply is not a download link",
                &reply.body,
            ))
        }
    }
}
