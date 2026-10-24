//! Hosts running the "XFileSharing" script: ClicknUpload, HexUpload, UserCloud
//! and UsersDrive.
//!
//! The upload endpoint is not fixed; it is embedded in the home page (the
//! script load-balances over several servers), so the flow is: fetch the page,
//! find the endpoint, POST the file, read `file_code` from the JSON reply.

use async_trait::async_trait;
use regex::Regex;

use crate::error::UploadError;
use crate::reply::{SendReply, capture};
use crate::site::{Site, SiteInfo};
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

pub struct XfsSite {
    info: SiteInfo,
    /// Page that contains the upload endpoint.
    page_url: &'static str,
    /// Finds the endpoint in that page (capture group 1 if present, else the whole match).
    endpoint: Regex,
    file_field: &'static str,
    fields: &'static [(&'static str, &'static str)],
    /// Download link with `{code}` standing in for the returned `file_code`.
    link_template: &'static str,
}

impl XfsSite {
    fn new(
        info: SiteInfo,
        page_url: &'static str,
        endpoint: &str,
        file_field: &'static str,
        fields: &'static [(&'static str, &'static str)],
        link_template: &'static str,
    ) -> Self {
        Self {
            info,
            page_url,
            endpoint: Regex::new(endpoint).expect("endpoint pattern is valid"),
            file_field,
            fields,
            link_template,
        }
    }
}

#[async_trait]
impl Site for XfsSite {
    fn info(&self) -> &SiteInfo {
        &self.info
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        cx.progress.set_message("Finding upload server");
        let page = cx.http.get(self.page_url).send_reply().await?.ok()?;
        let endpoint = capture(&self.endpoint, &page.body)
            .ok_or_else(|| UploadError::unexpected("could not find the upload server in the page"))?;

        cx.progress.set_message("Uploading");
        let mut form = new_form();
        for (name, value) in self.fields {
            form = form.text(*name, *value);
        }
        let form = form.part(self.file_field, cx.whole_file_part().await?);

        let reply = cx
            .http
            .post(&endpoint)
            .multipart(form)
            .send_reply()
            .await?
            .ok()?;

        let json = reply.json()?;
        let first = json
            .get(0)
            .ok_or_else(|| UploadError::protocol("reply has no file entry", &reply.body))?;
        if let Some(status) = first.get("file_status").and_then(|s| s.as_str()) {
            if status != "OK" {
                return Err(UploadError::protocol(
                    format!("server rejected the file ({status})"),
                    &reply.body,
                ));
            }
        }
        let code = first
            .get("file_code")
            .and_then(|c| c.as_str())
            .filter(|c| !c.is_empty())
            .ok_or_else(|| UploadError::protocol("reply has no file_code", &reply.body))?;

        Ok(UploadedFile::new(self.link_template.replace("{code}", code)))
    }
}

const ANON_FORM: &[(&str, &str)] = &[
    ("utype", "anon"),
    ("file_descr", "Uploaded using Mul-Tor on GitHub!"),
    ("file_public", "1"),
    ("keepalive", "1"),
];

/// Generic pattern: the form's `action="https://…"` attribute.
const FORM_ACTION: &str = r#"action\s*=\s*["'](https?://[^\s"']+)["']"#;

pub(super) fn sites() -> Vec<XfsSite> {
    vec![
        XfsSite::new(
            SiteInfo::new(
                "ClicknUpload",
                "https://clicknupload.click",
                "https://clicknupload.click",
            )
            .max_size(SizeLimit::gb(2)),
            "https://clicknupload.click",
            r"https://\w+\.clicknupload\.net/cgi-bin/upload\.cgi",
            "file",
            ANON_FORM,
            "https://clicknupload.vip/{code}",
        ),
        XfsSite::new(
            SiteInfo::new("HexUpload", "https://hexupload.net/", "https://hexupload.net/")
                .max_size(SizeLimit::gb(2))
                .blocked_extensions(&[".jpeg", ".jpg", ".png"]),
            "https://hexupload.net/",
            FORM_ACTION,
            "file_0",
            ANON_FORM,
            "https://hexupload.net/{code}",
        ),
        XfsSite::new(
            SiteInfo::new(
                "UserCloud",
                "https://userscloud.com/",
                "https://userscloud.com/",
            )
            .aliases(&["Userscloud"])
            .max_size(SizeLimit::gb(5)),
            "https://userscloud.com/",
            FORM_ACTION,
            "file_0",
            ANON_FORM,
            "https://userscloud.com/{code}",
        ),
        XfsSite::new(
            SiteInfo::new(
                "UsersDrive",
                "https://usersdrive.com/",
                "https://usersdrive.com/",
            )
            .max_size(SizeLimit::gb(2)),
            "https://usersdrive.com/",
            r"https://\S+\.userdrive\.(?:me|org)/cgi-bin/upload\.cgi\?upload_type=file",
            "file_0",
            &[("utype", "anon"), ("keepalive", "1")],
            "https://usersdrive.com/{code}.html",
        ),
    ]
}
