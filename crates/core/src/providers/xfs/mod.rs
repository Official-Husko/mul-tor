//! Hosts running the "XFileSharing" script: ClicknUpload, Datanodes, Dosya.co,
//! Filespace, HexUpload, Send.now, UserCloud and UsersDrive.
//!
//! The upload endpoint is not fixed; it is embedded in the home page (the
//! script load-balances over several servers), so the flow is: fetch the page,
//! find the endpoint, POST the file, read the file code from the reply. Newer
//! versions answer with JSON; older ones redirect to a result page.

use async_trait::async_trait;
use regex::Regex;

mod entries;
mod reply;

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::{SendReply, capture};
use crate::upload::{UploadContext, UploadedFile, new_form};
use reply::ReplyKind;

pub(super) use entries::providers;

pub struct XfsProvider {
    info: ProviderInfo,
    /// Page that contains the upload endpoint.
    page_url: &'static str,
    /// Finds the endpoint in that page (capture group 1 if present, else the whole match).
    endpoint: Regex,
    file_field: &'static str,
    fields: &'static [(&'static str, &'static str)],
    /// Download link with `{code}` standing in for the returned `file_code`.
    link_template: &'static str,
    /// Treat a reply without `file_status` as a failure. (A `file_status` other
    /// than "OK" always is.) Only UsersDrive was strict about this before.
    require_status: bool,
    reply: ReplyKind,
}

impl XfsProvider {
    fn new(
        info: ProviderInfo,
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
            require_status: false,
            reply: ReplyKind::Json,
        }
    }

    fn requiring_status(mut self) -> Self {
        self.require_status = true;
        self
    }

    /// The host answers with a redirect to a result page instead of JSON.
    fn redirecting(mut self) -> Self {
        self.reply = ReplyKind::Redirect;
        self
    }
}

#[async_trait]
impl Provider for XfsProvider {
    fn info(&self) -> &ProviderInfo {
        &self.info
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        cx.progress.set_message("Finding upload server");
        let page = cx.http.get(self.page_url).send_reply().await?.ok()?;
        // The address sits in an HTML attribute, where `&` is written `&amp;`.
        let endpoint = capture(&self.endpoint, &page.body)
            .ok_or_else(|| UploadError::unexpected("could not find the upload server in the page"))?
            .replace("&amp;", "&");

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

        let code = self.reply.file_code(&reply, self.require_status)?;
        Ok(UploadedFile::new(
            self.link_template.replace("{code}", &code),
        ))
    }
}
