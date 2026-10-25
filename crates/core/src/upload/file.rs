//! The local file being uploaded, and how its name is sent.

use std::path::PathBuf;

use crate::error::UploadError;

/// Longest file name (in bytes) we send to a provider. Filesystems allow 255; the
/// margin leaves room for providers that add a prefix or suffix.
const MAX_NAME_BYTES: usize = 240;

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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[tokio::test]
    async fn open_reports_size_and_name() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello.txt");
        std::fs::write(&path, b"hello").unwrap();
        let file = LocalFile::open(&path).await.unwrap();
        assert_eq!(file.size, 5);
        assert_eq!(file.name, "hello.txt");
    }

    #[tokio::test]
    async fn open_rejects_directories_and_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        assert!(LocalFile::open(dir.path()).await.is_err());
        assert!(LocalFile::open(dir.path().join("nope")).await.is_err());
    }
}
