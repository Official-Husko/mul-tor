//! What an uploader receives, and helpers that stream file data with progress.

use std::io::SeekFrom;
use std::path::Path;

use futures::TryStreamExt;
use reqwest::Body;
use reqwest::multipart::{Form, Part};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

use super::file::LocalFile;
use crate::error::UploadError;
use crate::http::Http;
use crate::progress::Progress;
use crate::provider::Credentials;

/// Read buffer for streamed bodies. Large enough to keep throughput high,
/// small enough that progress updates stay smooth.
const STREAM_CHUNK: usize = 256 * 1024;

/// A successful upload.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct UploadedFile {
    pub url: String,
}

impl UploadedFile {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

/// Everything an uploader needs for one file on one provider.
pub struct UploadContext<'a> {
    pub file: &'a LocalFile,
    pub http: &'a Http,
    pub credentials: &'a Credentials,
    pub progress: &'a Progress,
}

impl UploadContext<'_> {
    /// A required credential, or [`UploadError::MissingCredential`].
    pub fn credential(&self, key: &'static str) -> Result<&str, UploadError> {
        self.credentials
            .get(key)
            .ok_or(UploadError::MissingCredential(key))
    }

    /// The file part of a multipart form covering `offset..offset + len`,
    /// streamed from disk. `Content-Length` of the whole form stays known, so
    /// servers never see chunked transfer encoding.
    pub async fn file_part(&self, offset: u64, len: u64) -> Result<Part, UploadError> {
        let body = self.range_body(offset, len).await?;
        Ok(Part::stream_with_length(body, len)
            .file_name(self.file.name.clone())
            .mime_str("application/octet-stream")?)
    }

    /// The whole file as one multipart part.
    pub async fn whole_file_part(&self) -> Result<Part, UploadError> {
        self.file_part(0, self.file.size).await
    }

    /// The whole file as a raw request body. Callers must also set
    /// `Content-Length` (see [`Self::raw_upload`]) or the body goes out chunked.
    pub async fn file_body(&self) -> Result<Body, UploadError> {
        self.range_body(0, self.file.size).await
    }

    /// `builder` with the whole file as its body and an explicit `Content-Length`.
    pub async fn raw_upload(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder, UploadError> {
        Ok(builder
            .header(reqwest::header::CONTENT_LENGTH, self.file.size)
            .body(self.file_body().await?))
    }

    /// Stream `len` bytes starting at `offset`, counting them as progress.
    pub async fn range_body(&self, offset: u64, len: u64) -> Result<Body, UploadError> {
        open_range(&self.file.path, offset, len, self.progress.clone()).await
    }
}

async fn open_range(
    path: &Path,
    offset: u64,
    len: u64,
    progress: Progress,
) -> Result<Body, UploadError> {
    let mut file = tokio::fs::File::open(path).await?;
    if offset > 0 {
        file.seek(SeekFrom::Start(offset)).await?;
    }
    let stream =
        ReaderStream::with_capacity(file.take(len), STREAM_CHUNK).inspect_ok(move |chunk| {
            progress.add(chunk.len() as u64);
        });
    Ok(Body::wrap_stream(stream))
}

/// A multipart form that leaves field names untouched.
///
/// reqwest percent-encodes field names by default, turning `files[]` into
/// `name*=utf-8''files%5B%5D`, which the PHP-style hosts this tool targets do
/// not understand. Always build forms through this function.
pub fn new_form() -> Form {
    Form::new().percent_encode_noop()
}
