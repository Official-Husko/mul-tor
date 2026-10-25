//! GigaFile (gigafile.nu) — the home page names the upload server; the file
//! goes there in numbered chunks that share one session cookie, and the reply
//! to the last one holds the link.
//!
//! The longest retention the site offers, 100 days, is always asked for.

use std::sync::LazyLock;

use async_trait::async_trait;
use regex::Regex;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::{SendReply, capture};
use crate::size::SizeLimit;
use crate::upload::{
    CookieJar, HEX, UploadContext, UploadedFile, chunk_ranges, https_url_within, is_web_link,
    new_form, random_string,
};

const INFO: ProviderInfo =
    ProviderInfo::new("GigaFile", "https://gigafile.nu/", "https://gigafile.nu/")
        .aliases(&["GigaFileNu"])
        .max_size(SizeLimit::gb(300))
        .verified("2026-10-24");

/// Days a file is kept: the largest of the seven choices (3 to 100).
const LIFETIME_DAYS: &str = "100";

/// `var server = "111.gigafile.nu";` in the page's script.
static SERVER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"var server = "([A-Za-z0-9.-]+)""#).expect("pattern is valid"));

pub struct GigaFile {
    chunk_size: u64,
}

impl Default for GigaFile {
    fn default() -> Self {
        // The "100mb" the page announces.
        Self {
            chunk_size: 100 * 1024 * 1024,
        }
    }
}

impl GigaFile {
    #[doc(hidden)]
    pub fn with_chunk_size(chunk_size: u64) -> Self {
        Self { chunk_size }
    }
}

#[async_trait]
impl Provider for GigaFile {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        cx.progress.set_message("Finding upload server");
        let home = cx
            .http
            .get("https://gigafile.nu/")
            .send_reply()
            .await?
            .ok()?;
        // The name is requested next, with the file: it must stay on GigaFile.
        let endpoint = capture(&SERVER, &home.body)
            .map(|host| format!("https://{host}/upload_chunk.php"))
            .filter(|url| https_url_within(url, "gigafile.nu").is_some())
            .ok_or_else(|| UploadError::protocol("could not find the upload server", &home.body))?;

        let size = cx.file.size;
        let total_chunks = size.div_ceil(self.chunk_size);
        // Names the file for the server while its chunks arrive.
        let id = random_string(32, HEX);
        let mut jar = CookieJar::default();
        let mut last = None;
        for (index, (offset, len)) in chunk_ranges(size, self.chunk_size).enumerate() {
            cx.progress
                .set_message(format!("Chunk {}/{}", index + 1, total_chunks));
            let form = new_form()
                .text("id", id.clone())
                .text("name", cx.file.name.clone())
                // Counted from 0.
                .text("chunk", index.to_string())
                .text("chunks", total_chunks.to_string())
                .part("file", cx.file_part(offset, len).await?)
                .text("lifetime", LIFETIME_DAYS);
            let mut request = cx.http.post(&endpoint).multipart(form);
            if let Some(cookies) = jar.header() {
                request = request.header("Cookie", cookies);
            }
            let reply = request.send_reply().await?.ok()?;
            jar.absorb(&reply);

            // `status` 0 is success; anything else comes with a message.
            let status = reply.json_u64("/status")?;
            if status != 0 {
                return Err(UploadError::protocol(
                    format!("server refused the chunk (status {status})"),
                    &reply.body,
                ));
            }
            last = Some(reply);
        }

        let reply = last.ok_or(UploadError::EmptyFile)?;
        let link = reply.json_string("/url")?;
        if is_web_link(&link) && https_url_within(&link, "gigafile.nu").is_some() {
            Ok(UploadedFile::new(link))
        } else {
            Err(UploadError::protocol(
                "reply is not a download link",
                &reply.body,
            ))
        }
    }
}
