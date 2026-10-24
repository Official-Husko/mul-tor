//! Fastupload — Laravel app with Dropzone-style chunks. The session cookie
//! rotates while uploading, so each reply's cookies feed the next request.

use async_trait::async_trait;
use uuid::Uuid;

use super::dropzone::{Dropzone, Headers};
use crate::error::UploadError;
use crate::reply::{Reply, SendReply, csrf_meta};
use crate::site::{Site, SiteInfo};
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile};

const INFO: SiteInfo = SiteInfo::new(
    "Fastupload",
    "https://fastupload.io/",
    "https://fastupload.io/",
)
.max_size(SizeLimit::gb(10))
.blocked_extensions(&[
    ".exe", ".php", ".lnk", ".html", ".htm", ".phtml", ".shtm", ".js", ".bat", ".dll",
]);

const SESSION_COOKIE: &str = "filebob_user_session";

pub struct Fastupload {
    chunk_size: u64,
}

impl Default for Fastupload {
    fn default() -> Self {
        Self {
            chunk_size: 94_371_840,
        }
    }
}

impl Fastupload {
    #[doc(hidden)]
    pub fn with_chunk_size(chunk_size: u64) -> Self {
        Self { chunk_size }
    }
}

fn cookie_header(xsrf: &str, session: &str) -> String {
    format!("XSRF-TOKEN={xsrf}; {SESSION_COOKIE}={session}")
}

#[async_trait]
impl Site for Fastupload {
    fn info(&self) -> &SiteInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        cx.progress.set_message("Fetching session");
        let home = cx
            .http
            .get("https://fastupload.io/")
            .send_reply()
            .await?
            .ok()?;
        let (Some(csrf), Some(xsrf), Some(session)) = (
            csrf_meta(&home.body),
            home.cookie("XSRF-TOKEN"),
            home.cookie(SESSION_COOKIE),
        ) else {
            return Err(UploadError::protocol(
                "could not get the CSRF token and session cookies",
                &home.body,
            ));
        };

        let mut headers: Headers = vec![
            ("Accept", "application/json".into()),
            ("X-CSRF-TOKEN", csrf),
            ("Cookie", cookie_header(xsrf, session)),
        ];
        // Latest cookie values; a reply that does not set one keeps the old value.
        let mut xsrf = xsrf.to_string();
        let mut session = session.to_string();

        let uuid = Uuid::new_v4().to_string();
        let chunks = Dropzone {
            url: "https://fastupload.io/upload",
            uuid: &uuid,
            chunk_size: self.chunk_size,
            file_field: "file",
            fields: vec![
                ("size", cx.file.size.to_string()),
                ("upload_auto_delete", "0".into()),
            ],
        };
        let last: Reply = chunks
            .upload(cx, &mut headers, |reply, headers| {
                if let Some(value) = reply.cookie("XSRF-TOKEN") {
                    xsrf = value.to_string();
                }
                if let Some(value) = reply.cookie(SESSION_COOKIE) {
                    session = value.to_string();
                }
                if let Some(entry) = headers.iter_mut().find(|(name, _)| *name == "Cookie") {
                    entry.1 = cookie_header(&xsrf, &session);
                }
            })
            .await?;

        Ok(UploadedFile::new(last.json_string("/download_link")?))
    }
}
