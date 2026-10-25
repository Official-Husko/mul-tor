//! Keeping secrets out of the log.
//!
//! Upload APIs routinely carry keys and tokens in the query string, and
//! sometimes in the user-info part of a URL. A URL is therefore only ever
//! logged through [`redact_url`].

use url::Url;

/// `scheme://host[:port]/path` without user info, query or fragment.
///
/// Anything that does not parse is replaced by a placeholder rather than
/// risking a secret in a half-understood string.
pub fn redact_url(url: &str) -> String {
    match Url::parse(url) {
        Ok(mut url) => {
            url.set_query(None);
            url.set_fragment(None);
            // Both only fail for URLs that cannot carry credentials anyway.
            let _ = url.set_username("");
            let _ = url.set_password(None);
            url.to_string()
        }
        Err(_) => "<unparseable url>".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_fragment_and_credentials_are_removed() {
        assert_eq!(
            redact_url("https://user:pw@api.example.com:8443/v1/upload?key=SECRET&x=1#frag"),
            "https://api.example.com:8443/v1/upload"
        );
    }

    #[test]
    fn plain_urls_are_kept() {
        assert_eq!(
            redact_url("https://example.com/a/b.zip"),
            "https://example.com/a/b.zip"
        );
    }

    #[test]
    fn garbage_is_never_echoed() {
        let shown = redact_url("not a url?token=SECRET");
        assert!(!shown.contains("SECRET"));
    }
}
