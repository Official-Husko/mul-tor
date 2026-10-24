//! Hosts that take one multipart `POST` with the file in a single field and
//! answer with the link: 0x0.st, EasySend, Qu.ax, Temp.sh, TempFile.org,
//! tfLink, Uguu and X0.at.
//!
//! They differ only in the address, the name of the file field, a few fixed
//! form fields and where the link sits in the reply, so each is one entry of
//! data in [`providers`].

use async_trait::async_trait;
use reqwest::Url;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::{Reply, SendReply};
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

/// Where the download link is in the reply.
#[derive(Clone, Copy)]
enum LinkIn {
    /// The whole body, trimmed.
    Body,
    /// The string at this JSON pointer.
    Json(&'static str),
}

pub struct FormPostProvider {
    info: ProviderInfo,
    url: &'static str,
    file_field: &'static str,
    /// Extra form fields sent before the file.
    fields: &'static [(&'static str, &'static str)],
    link: LinkIn,
}

impl FormPostProvider {
    const fn new(
        info: ProviderInfo,
        url: &'static str,
        file_field: &'static str,
        fields: &'static [(&'static str, &'static str)],
        link: LinkIn,
    ) -> Self {
        Self {
            info,
            url,
            file_field,
            fields,
            link,
        }
    }

    /// The link in `reply`, checked: it ends up in front of the user as a
    /// download address, so an error message or a half-read page must not pass.
    fn link(&self, reply: &Reply) -> Result<String, UploadError> {
        let link = match self.link {
            LinkIn::Body => reply.body.trim().to_string(),
            LinkIn::Json(pointer) => reply.json_string(pointer)?,
        };
        let usable = !link.chars().any(char::is_whitespace)
            && Url::parse(&link)
                .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.has_host());
        if usable {
            Ok(link)
        } else {
            Err(UploadError::protocol(
                "reply is not a download link",
                &reply.body,
            ))
        }
    }
}

#[async_trait]
impl Provider for FormPostProvider {
    fn info(&self) -> &ProviderInfo {
        &self.info
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let mut form = new_form();
        for (name, value) in self.fields {
            form = form.text(*name, *value);
        }
        let form = form.part(self.file_field, cx.whole_file_part().await?);

        let reply = cx
            .http
            .post(self.url)
            .multipart(form)
            .send_reply()
            .await?
            .ok()?;
        Ok(UploadedFile::new(self.link(&reply)?))
    }
}

/// The day the entries marked verified were uploaded to for real.
const CHECKED: &str = "2026-10-24";

/// Extensions Uguu answers with "415 Unsupported Media Type" (found by trying).
const UGUU_REFUSES: &[&str] = &[".zip", ".rar", ".pdf", ".exe", ".html"];

/// Extensions qu.ax takes; it answers anything else with "file type is not allowed".
const QUAX_TYPES: &[&str] = &[
    ".jpg", ".jpeg", ".png", ".gif", ".webp", ".avif", ".svg", ".mp4", ".mov", ".wmv", ".mpeg",
    ".mpg", ".webm", ".zip", ".rar", ".7z", ".tar", ".gz", ".pdf", ".txt",
];

pub(super) fn providers() -> Vec<FormPostProvider> {
    vec![
        FormPostProvider::new(
            ProviderInfo::new("0x0.st", "https://0x0.st/", "https://0x0.st/")
                .aliases(&["NullPointer"])
                .max_size(SizeLimit::mb(512))
                .api_docs("https://0x0.st/"),
            "https://0x0.st/",
            "file",
            &[],
            LinkIn::Body,
        ),
        FormPostProvider::new(
            ProviderInfo::new("EasySend", "https://easysend.co/", "https://easysend.co/")
                .max_size(SizeLimit::gb(1))
                .api_docs("https://easysend.co/developer-api")
                .verified(CHECKED),
            "https://easysend.co/api/v1/upload",
            "files[]",
            &[],
            LinkIn::Json("/share_link"),
        ),
        FormPostProvider::new(
            ProviderInfo::new("Qu.ax", "https://qu.ax/", "https://qu.ax/")
                .max_size(SizeLimit::mb(256))
                .allowed_extensions(QUAX_TYPES)
                .verified(CHECKED),
            "https://qu.ax/upload",
            "files[]",
            &[],
            LinkIn::Json("/files/0/url"),
        ),
        FormPostProvider::new(
            ProviderInfo::new("Temp.sh", "https://temp.sh/", "https://temp.sh/")
                .max_size(SizeLimit::gb(4))
                .api_docs("https://temp.sh/")
                .verified(CHECKED),
            "https://temp.sh/upload",
            "file",
            &[],
            LinkIn::Body,
        ),
        FormPostProvider::new(
            ProviderInfo::new(
                "TempFile.org",
                "https://tempfile.org/",
                "https://tempfile.org/",
            )
            .max_size(SizeLimit::mb(100))
            .api_docs("https://tempfile.org/api")
            .verified(CHECKED),
            "https://tempfile.org/api/upload/local",
            "files",
            // The longest it offers (hours): 1, 6, 24 or 48.
            &[("expiryHours", "48")],
            LinkIn::Json("/files/0/url"),
        ),
        FormPostProvider::new(
            ProviderInfo::new("tfLink", "https://tmpfile.link/", "https://tmpfile.link/")
                .aliases(&["TmpFileLink"])
                .max_size(SizeLimit::mb(100))
                .api_docs("https://tmpfile.link/")
                .verified(CHECKED),
            "https://tmpfile.link/api/upload",
            "file",
            &[],
            LinkIn::Json("/downloadLink"),
        ),
        FormPostProvider::new(
            ProviderInfo::new("Uguu", "https://uguu.se/", "https://uguu.se/")
                .max_size(SizeLimit::mb(128))
                .api_docs("https://uguu.se/api")
                .blocked_extensions(UGUU_REFUSES)
                .verified(CHECKED),
            "https://uguu.se/upload",
            "files[]",
            &[],
            LinkIn::Json("/files/0/url"),
        ),
        FormPostProvider::new(
            ProviderInfo::new("X0.at", "https://x0.at/", "https://x0.at/")
                .max_size(SizeLimit::gb(1))
                .api_docs("https://x0.at/")
                .verified(CHECKED),
            "https://x0.at/",
            "file",
            &[],
            LinkIn::Body,
        ),
    ]
}
