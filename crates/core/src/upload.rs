//! The pieces an uploader works with: the local file, the request context, and
//! helpers that stream file data with progress reporting.

use std::io::SeekFrom;
use std::path::{Path, PathBuf};

use futures::TryStreamExt;
use rand::RngExt;
use reqwest::multipart::{Form, Part};
use reqwest::{Body, Url};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

use crate::error::UploadError;
use crate::http::Http;
use crate::progress::Progress;
use crate::provider::Credentials;

/// Longest file name (in bytes) we send to a provider. Filesystems allow 255; the
/// margin leaves room for providers that add a prefix or suffix.
const MAX_NAME_BYTES: usize = 240;

/// Read buffer for streamed bodies. Large enough to keep throughput high,
/// small enough that progress updates stay smooth.
const STREAM_CHUNK: usize = 256 * 1024;

/// A file on disk that is about to be uploaded.
#[derive(Clone, Debug)]
pub struct LocalFile {
    pub path: PathBuf,
    /// Name as sent to providers (shortened to a safe length, extension kept).
    pub name: String,
    pub size: u64,
}

impl LocalFile {
    pub async fn open(path: impl Into<PathBuf>) -> Result<Self, UploadError> {
        let path = path.into();
        let metadata = tokio::fs::metadata(&path).await?;
        if !metadata.is_file() {
            return Err(UploadError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("{} is not a regular file", path.display()),
            )));
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "file".to_string());
        Ok(Self {
            name: shorten_name(&name, MAX_NAME_BYTES),
            size: metadata.len(),
            path,
        })
    }
}

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

/// `(offset, length)` of each chunk when splitting `total` bytes into pieces of
/// at most `chunk_size`. Yields nothing for `total == 0`.
pub fn chunk_ranges(total: u64, chunk_size: u64) -> impl Iterator<Item = (u64, u64)> {
    assert!(chunk_size > 0, "chunk size must be positive");
    (0..total.div_ceil(chunk_size)).map(move |index| {
        let offset = index * chunk_size;
        (offset, chunk_size.min(total - offset))
    })
}

/// Shorten `name` to at most `max_bytes`, cutting the stem and keeping a
/// reasonably short extension, on a character boundary.
pub fn shorten_name(name: &str, max_bytes: usize) -> String {
    if name.len() <= max_bytes {
        return name.to_string();
    }
    let (stem, ext) = match name.rfind('.') {
        Some(dot) if dot > 0 && name.len() - dot <= 16 => (&name[..dot], &name[dot..]),
        _ => (name, ""),
    };
    let mut end = max_bytes.saturating_sub(ext.len()).min(stem.len());
    while !stem.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{}", &stem[..end], ext)
}

/// `base` with `segments` appended as properly escaped path segments, so file
/// names containing `?`, `#`, `/` or spaces cannot change the URL's meaning.
pub fn url_with_segments(base: &str, segments: &[&str]) -> Result<String, UploadError> {
    let mut url = Url::parse(base)
        .map_err(|e| UploadError::unexpected(format!("invalid URL `{base}`: {e}")))?;
    {
        let mut path = url
            .path_segments_mut()
            .map_err(|_| UploadError::unexpected(format!("URL `{base}` cannot take a path")))?;
        path.pop_if_empty();
        path.extend(segments);
    }
    Ok(url.into())
}

/// Random string of `len` characters drawn from `alphabet`.
pub fn random_string(len: usize, alphabet: &str) -> String {
    let chars: Vec<char> = alphabet.chars().collect();
    let mut rng = rand::rng();
    (0..len)
        .map(|_| chars[rng.random_range(0..chars.len())])
        .collect()
}

pub const LOWER_ALNUM: &str = "abcdefghijklmnopqrstuvwxyz0123456789";
pub const MIXED_ALNUM: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_ranges_cover_the_file_exactly() {
        assert_eq!(
            chunk_ranges(10, 4).collect::<Vec<_>>(),
            [(0, 4), (4, 4), (8, 2)]
        );
        assert_eq!(chunk_ranges(8, 4).collect::<Vec<_>>(), [(0, 4), (4, 4)]);
        assert_eq!(chunk_ranges(3, 100).collect::<Vec<_>>(), [(0, 3)]);
        assert_eq!(chunk_ranges(0, 4).count(), 0);
    }

    #[test]
    fn short_names_are_untouched() {
        assert_eq!(shorten_name("movie.mkv", 240), "movie.mkv");
    }

    #[test]
    fn long_names_keep_their_extension_and_fit() {
        let name = format!("{}.tar.gz", "a".repeat(300));
        let short = shorten_name(&name, 240);
        assert_eq!(short.len(), 240);
        assert!(short.ends_with(".gz"));
    }

    #[test]
    fn shortening_never_splits_a_multibyte_character() {
        let name = format!("{}.zip", "é".repeat(200)); // 400 bytes of stem
        let short = shorten_name(&name, 240);
        assert!(short.len() <= 240);
        assert!(short.ends_with(".zip"));
        assert!(short.chars().all(|c| c == 'é' || ".zip".contains(c)));
    }

    #[test]
    fn extension_less_and_dotfile_names_are_cut_plainly() {
        assert_eq!(shorten_name(&"x".repeat(300), 240).len(), 240);
        assert_eq!(
            shorten_name(&format!(".{}", "x".repeat(300)), 240).len(),
            240
        );
    }

    #[test]
    fn url_segments_are_escaped() {
        let url = url_with_segments("https://h.example/api/file/", &["a b#c?d.zip"]).unwrap();
        assert_eq!(url, "https://h.example/api/file/a%20b%23c%3Fd.zip");
        let url = url_with_segments("https://h.example", &["bin", "x/y"]).unwrap();
        assert_eq!(url, "https://h.example/bin/x%2Fy");
    }

    #[test]
    fn random_strings_use_only_the_alphabet() {
        let s = random_string(64, LOWER_ALNUM);
        assert_eq!(s.len(), 64);
        assert!(s.chars().all(|c| LOWER_ALNUM.contains(c)));
    }

    #[tokio::test]
    async fn local_file_open_reports_size_and_name() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello.txt");
        std::fs::write(&path, b"hello").unwrap();
        let file = LocalFile::open(&path).await.unwrap();
        assert_eq!(file.size, 5);
        assert_eq!(file.name, "hello.txt");
    }

    #[tokio::test]
    async fn local_file_open_rejects_directories_and_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        assert!(LocalFile::open(dir.path()).await.is_err());
        assert!(LocalFile::open(dir.path().join("nope")).await.is_err());
    }
}
