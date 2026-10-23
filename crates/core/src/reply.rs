//! A fully-read HTTP response plus the small parsing helpers uploaders need.
//!
//! Reading the body eagerly is fine here: everything that comes back from a
//! site is a short JSON/HTML/text reply, never file data.

use std::sync::LazyLock;

use regex::Regex;
use reqwest::header::HeaderMap;
use reqwest::{RequestBuilder, StatusCode, Url};
use serde_json::Value;

use crate::error::UploadError;

#[derive(Debug)]
pub(crate) struct Reply {
    pub status: StatusCode,
    /// Final URL after redirects.
    pub url: Url,
    pub headers: HeaderMap,
    pub cookies: Vec<(String, String)>,
    pub body: String,
}

impl Reply {
    pub(crate) async fn read(response: reqwest::Response) -> Result<Self, UploadError> {
        let status = response.status();
        let url = response.url().clone();
        let headers = response.headers().clone();
        let cookies = response
            .cookies()
            .map(|c| (c.name().to_string(), c.value().to_string()))
            .collect();
        let body = response.text().await?;
        Ok(Self {
            status,
            url,
            headers,
            cookies,
            body,
        })
    }

    /// Fail unless the status is exactly `expected`.
    pub(crate) fn expect_status(self, expected: u16) -> Result<Self, UploadError> {
        if self.status.as_u16() == expected {
            Ok(self)
        } else {
            Err(self.status_error())
        }
    }

    /// Fail unless the status is 2xx.
    pub(crate) fn ok(self) -> Result<Self, UploadError> {
        if self.status.is_success() {
            Ok(self)
        } else {
            Err(self.status_error())
        }
    }

    fn status_error(&self) -> UploadError {
        UploadError::protocol(format!("HTTP {}", self.status), &self.body)
    }

    pub(crate) fn json(&self) -> Result<Value, UploadError> {
        serde_json::from_str(&self.body)
            .map_err(|e| UploadError::protocol(format!("response is not valid JSON ({e})"), &self.body))
    }

    /// A string (or number, rendered as text) at a JSON pointer such as `/data/url`.
    pub(crate) fn json_string(&self, pointer: &str) -> Result<String, UploadError> {
        json_string(&self.json()?, pointer, &self.body)
    }

    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    pub(crate) fn cookie(&self, name: &str) -> Option<&str> {
        self.cookies
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Look up `pointer` in `value`, accepting strings and numbers.
pub(crate) fn json_string(value: &Value, pointer: &str, raw: &str) -> Result<String, UploadError> {
    match value.pointer(pointer) {
        Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        Some(Value::Number(n)) => Ok(n.to_string()),
        _ => Err(UploadError::protocol(
            format!("response has no `{pointer}`"),
            raw,
        )),
    }
}

pub(crate) trait SendReply {
    /// Send the request and read the whole reply.
    async fn send_reply(self) -> Result<Reply, UploadError>;
}

impl SendReply for RequestBuilder {
    async fn send_reply(self) -> Result<Reply, UploadError> {
        Reply::read(self.send().await?).await
    }
}

/// First capture group (or the whole match if the regex has none) of `re` in `text`.
pub(crate) fn capture(re: &Regex, text: &str) -> Option<String> {
    let caps = re.captures(text)?;
    Some(caps.get(1).unwrap_or_else(|| caps.get(0).unwrap()).as_str().to_string())
}

/// The first `{ ... }` object in a text that is meant to be JSON but may be
/// wrapped in other output (some PHP hosts print notices around it).
pub(crate) fn embedded_json(text: &str) -> Option<Value> {
    static OBJECT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)\{.*\}").unwrap());
    serde_json::from_str(OBJECT.find(text)?.as_str()).ok()
}

/// CSRF token from a `<meta name="csrf-token" content="…">` tag (Laravel sites).
pub(crate) fn csrf_meta(html: &str) -> Option<String> {
    static META: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<meta\s+name="csrf-token"\s+content="([^"]*)""#).unwrap()
    });
    capture(&META, html).filter(|t| !t.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_pointer_lookup_accepts_strings_and_numbers() {
        let v: Value = serde_json::json!({"data": {"id": 42, "url": "https://x/y"}, "list": [{"url": "u0"}]});
        assert_eq!(json_string(&v, "/data/url", "").unwrap(), "https://x/y");
        assert_eq!(json_string(&v, "/data/id", "").unwrap(), "42");
        assert_eq!(json_string(&v, "/list/0/url", "").unwrap(), "u0");
        assert!(json_string(&v, "/nope", "raw").is_err());
    }

    #[test]
    fn embedded_json_skips_surrounding_noise() {
        let v = embedded_json("<br>Notice: x\n{\"status\": true}\n<!-- -->").unwrap();
        assert_eq!(v["status"], true);
        assert!(embedded_json("no json here").is_none());
    }

    #[test]
    fn csrf_meta_extracts_token() {
        let html = r#"<head><meta name="csrf-token" content="abc123" /></head>"#;
        assert_eq!(csrf_meta(html).unwrap(), "abc123");
        assert!(csrf_meta("<head></head>").is_none());
    }
}
