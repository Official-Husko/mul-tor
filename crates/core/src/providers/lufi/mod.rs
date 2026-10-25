//! Lufi — end-to-end encrypted uploads over a WebSocket.
//!
//! The web client picks a random key, encrypts each slice of the file with it
//! (see [`sjcl`]) and sends the slices over a WebSocket; the key never reaches
//! the server and travels only in the part of the link after `#`. This does the
//! same, so the instance's own download page can open what it uploads.
//!
//! Every instance names its retention choices on its page; the longest is
//! always asked for.

use async_trait::async_trait;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use rand::RngExt;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::SendReply;
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile};

mod page;
mod sjcl;
mod socket;

pub struct LufiProvider {
    info: ProviderInfo,
    /// The instance, with a trailing slash.
    base: &'static str,
    host: &'static str,
}

#[async_trait]
impl Provider for LufiProvider {
    fn info(&self) -> &ProviderInfo {
        &self.info
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        cx.progress.set_message("Reading the instance's settings");
        let settings = cx
            .http
            .get(format!("{}partial/index.js", self.base))
            .send_reply()
            .await?
            .ok()?;
        let endpoint = page::socket_address(&settings.body, self.host)
            .ok_or_else(|| UploadError::protocol("no usable upload socket", &settings.body))?;
        let home = cx.http.get(self.base).send_reply().await?.ok()?;
        let delay = page::longest_delay(&home.body)
            .ok_or_else(|| UploadError::unexpected("the page offers no retention choice"))?;

        // 256 random bits, as the web client uses.
        let key = STANDARD.encode(rand::rng().random::<[u8; 32]>());
        let short = socket::send_file(cx, &endpoint, &key, delay).await?;
        Ok(UploadedFile::new(format!("{}r/{short}#{key}", self.base)))
    }
}

pub(super) fn providers() -> Vec<LufiProvider> {
    vec![
        LufiProvider {
            info: ProviderInfo::new(
                "AdminForge",
                "https://upload.adminforge.de/",
                "https://upload.adminforge.de/",
            )
            .max_size(SizeLimit::gb(2))
            .verified(CHECKED),
            base: "https://upload.adminforge.de/",
            host: "upload.adminforge.de",
        },
        LufiProvider {
            info: ProviderInfo::new(
                "Disroot",
                "https://upload.disroot.org/",
                "https://upload.disroot.org/",
            )
            .aliases(&["DisrootUpload"])
            .max_size(SizeLimit::gb(2))
            .verified(CHECKED),
            base: "https://upload.disroot.org/",
            host: "upload.disroot.org",
        },
    ]
}

/// The day the instances marked verified were uploaded to for real.
const CHECKED: &str = "2026-10-24";
