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

/// Is `link` fit to be shown as a download address: one `http(s)` URL with a
/// host and no whitespace? Anything a host sends back goes through this before
/// it reaches the user, so an error message or a half-read page is not a result.
pub fn is_web_link(link: &str) -> bool {
    !link.chars().any(char::is_whitespace)
        && Url::parse(link)
            .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.has_host())
}

/// `url` as a URL if it is `https`, has no credentials or port, and its host is
/// `domain` or one of its subdomains. A server name taken from a reply may only
/// be requested after this.
pub fn https_url_within(url: &str, domain: &str) -> Option<Url> {
    let url = Url::parse(url).ok()?;
    let host = url.host_str()?;
    let within = host == domain
        || host
            .strip_suffix(domain)
            .is_some_and(|rest| rest.ends_with('.'));
    (url.scheme() == "https"
        && within
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none())
    .then_some(url)
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

    #[test]
    fn only_a_single_web_address_is_a_link() {
        assert!(is_web_link("https://x0.at/LeGh.txt"));
        assert!(is_web_link("http://example.com/a?b=c"));
        for bad in [
            "",
            "Sorry, too big",
            "https://x0.at/a\nmore text",
            "javascript:alert(1)",
            "ftp://example.com/a",
            "https://",
            "/relative/path",
        ] {
            assert!(!is_web_link(bad), "`{bad}` was accepted");
        }
    }

    #[test]
    fn a_server_must_be_the_domain_or_a_subdomain_of_it() {
        let ok = |url| https_url_within(url, "vikingfile.com").is_some();
        assert!(ok("https://jp.vikingfile.com/index2.php"));
        assert!(ok("https://vikingfile.com/index.php"));
        assert!(ok("https://a.b.vikingfile.com/"));
        for bad in [
            "http://jp.vikingfile.com/",
            "https://evilvikingfile.com/",
            "https://vikingfile.com.evil.example/",
            "https://user@jp.vikingfile.com/",
            "https://jp.vikingfile.com:8443/",
            "https://127.0.0.1/",
            "not a url",
        ] {
            assert!(!ok(bad), "`{bad}` was accepted");
        }
    }
}
