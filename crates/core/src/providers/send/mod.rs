//! Firefox Send and its forks: the file is encrypted here with a random secret,
//! record by record, and sent over a WebSocket; the secret is only in the part
//! of the link after `#`, so the server never sees what it stores.
//!
//! An instance publishes its limits at `/config`. The longest time and the most
//! downloads it allows are always asked for.

use async_trait::async_trait;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::SendReply;
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile};

mod ece;
mod keys;
mod socket;

pub struct SendProvider {
    info: ProviderInfo,
    host: &'static str,
}

#[async_trait]
impl Provider for SendProvider {
    fn info(&self) -> &ProviderInfo {
        &self.info
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        cx.progress.set_message("Reading the instance's limits");
        let config = cx
            .http
            .get(format!("https://{}/config", self.host))
            .send_reply()
            .await?
            .ok()?;
        let seconds = config.json_u64("/LIMITS/MAX_EXPIRE_SECONDS")?;
        let downloads = config.json_u64("/LIMITS/MAX_DOWNLOADS")?;

        let keys = keys::Keys::random();
        let address = format!("https://{}/api/ws", self.host);
        let id = socket::send_file(cx, &address, &keys, seconds, downloads).await?;
        Ok(UploadedFile::new(format!(
            "https://{}/download/{id}/#{}",
            self.host,
            keys.secret_text()
        )))
    }
}

/// The instances, as data. Each states its own limits at `/config`.
pub(super) fn providers() -> Vec<SendProvider> {
    vec![
        instance(
            ProviderInfo::new("SendVis", "https://send.vis.ee/", "https://send.vis.ee/")
                .aliases(&["Send.vis.ee"])
                .max_size(SizeLimit::mb(2560))
                .verified(CHECKED),
            "send.vis.ee",
        ),
        instance(
            ProviderInfo::new(
                "Send.cyberjake.xyz",
                "https://send.cyberjake.xyz/",
                "https://send.cyberjake.xyz/",
            )
            .max_size(SizeLimit::gb(10))
            .verified(CHECKED),
            "send.cyberjake.xyz",
        ),
        instance(
            ProviderInfo::new(
                "Send.monks.tools",
                "https://send.monks.tools/",
                "https://send.monks.tools/",
            )
            .max_size(SizeLimit::gb(5))
            .verified(CHECKED),
            "send.monks.tools",
        ),
        instance(
            ProviderInfo::new(
                "Send.adminforge.de",
                "https://send.adminforge.de/",
                "https://send.adminforge.de/",
            )
            .max_size(SizeLimit::gb(8))
            .verified(CHECKED),
            "send.adminforge.de",
        ),
        instance(
            ProviderInfo::new(
                "Send.mni.li",
                "https://send.mni.li/",
                "https://send.mni.li/",
            )
            .max_size(SizeLimit::gb(8))
            .verified(CHECKED),
            "send.mni.li",
        ),
    ]
}

/// The day the instances marked verified were uploaded to for real.
const CHECKED: &str = "2026-10-25";

fn instance(info: ProviderInfo, host: &'static str) -> SendProvider {
    SendProvider { info, host }
}
