//! Building provider URLs safely.

use reqwest::Url;

use crate::error::UploadError;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_segments_are_escaped() {
        let url = url_with_segments("https://h.example/api/file/", &["a b#c?d.zip"]).unwrap();
        assert_eq!(url, "https://h.example/api/file/a%20b%23c%3Fd.zip");
        let url = url_with_segments("https://h.example", &["bin", "x/y"]).unwrap();
        assert_eq!(url, "https://h.example/bin/x%2Fy");
    }
}
