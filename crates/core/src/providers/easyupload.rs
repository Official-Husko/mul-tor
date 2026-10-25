//! EasyUpload — find the upload server, validate the upload with a UUID, then
//! send Dropzone-style chunks.

use std::sync::LazyLock;

use async_trait::async_trait;
use regex::Regex;
use uuid::Uuid;

use super::dropzone::{Dropzone, Headers};
use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::{SendReply, capture, embedded_json};
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile};

const INFO: ProviderInfo = ProviderInfo::new(
    "EasyUpload",
    "https://easyupload.io/",
    "https://easyupload.io/",
)
.max_size(SizeLimit::gb(10));

static SERVER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"https://upload\d+\.easyupload\.io/action\.php").unwrap());

pub struct EasyUpload {
    chunk_size: u64,
}

impl Default for EasyUpload {
    fn default() -> Self {
        Self {
            chunk_size: 100_000_000,
        }
    }
}

impl EasyUpload {
    #[doc(hidden)]
    pub fn with_chunk_size(chunk_size: u64) -> Self {
        Self { chunk_size }
    }
}

#[async_trait]
impl Provider for EasyUpload {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        cx.progress.set_message("Finding upload server");
        let home = cx
            .http
            .get("https://easyupload.io/")
            .send_reply()
            .await?
            .ok()?;
        let upload_url = capture(&SERVER, &home.body).ok_or_else(|| {
            UploadError::unexpected("could not find the upload server in the page")
        })?;

        let uuid = Uuid::new_v4().to_string();
        let headers: Headers = vec![
            ("Accept", "application/json".into()),
            ("Referer", "https://easyupload.io/".into()),
        ];

        cx.progress.set_message("Validating upload");
        let mut validate = cx.http.post("https://easyupload.io/action.php");
        for (name, value) in &headers {
            validate = validate.header(*name, value);
        }
        let validated = validate
            .form(&[("type", "validate_tuf"), ("files[]", uuid.as_str())])
            .send_reply()
            .await?
            .ok()?;
        // The host prints PHP notices around its JSON, hence the lenient parse.
        let accepted = embedded_json(&validated.body)
            .and_then(|json| json.get("status").and_then(|s| s.as_bool()));
        if accepted != Some(true) {
            return Err(UploadError::protocol(
                "the server refused to start the upload",
                &validated.body,
            ));
        }

        let chunks = Dropzone {
            url: &upload_url,
            uuid: &uuid,
            chunk_size: self.chunk_size,
            file_field: "file",
            fields: vec![
                ("type", "dropzone-multi-upload".into()),
                // Keep files for 30 days.
                ("expiration", "30".into()),
            ],
        };
        let mut headers = headers;
        let last = chunks.upload(cx, &mut headers, |_, _| {}).await?;

        let link = embedded_json(&last.body)
            .and_then(|json| json.get("download_link")?.as_str().map(str::to_string))
            .filter(|link| !link.is_empty())
            .ok_or_else(|| UploadError::protocol("reply has no download_link", &last.body))?;
        Ok(UploadedFile::new(link))
    }
}
