//! A request as the mock server saw it.

use axum::http::HeaderMap;

use super::multipart::{FormPart, parse_multipart};

#[derive(Clone, Debug)]
pub struct Recorded {
    pub method: String,
    pub host: String,
    pub path: String,
    pub query: String,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl Recorded {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    pub fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    pub fn is(&self, method: &str, host: &str, path: &str) -> bool {
        self.method == method && self.host == host && self.path == path
    }

    /// Decoded `application/x-www-form-urlencoded` body.
    pub fn form(&self) -> Vec<(String, String)> {
        url_decode_pairs(&self.body_text())
    }

    pub fn query_pairs(&self) -> Vec<(String, String)> {
        url_decode_pairs(&self.query)
    }

    pub fn multipart(&self) -> Vec<FormPart> {
        parse_multipart(self.header("content-type").unwrap_or(""), &self.body)
    }

    /// Asserts the request was sent with a known length, not chunked.
    pub fn assert_not_chunked(&self) {
        assert!(
            self.headers.get("transfer-encoding").is_none(),
            "{} {} used chunked transfer encoding",
            self.method,
            self.path
        );
        let declared: usize = self
            .header("content-length")
            .unwrap_or_else(|| panic!("{} {} has no Content-Length", self.method, self.path))
            .parse()
            .unwrap();
        assert_eq!(declared, self.body.len(), "Content-Length mismatch");
    }
}

fn url_decode_pairs(input: &str) -> Vec<(String, String)> {
    input
        .split('&')
        .filter(|s| !s.is_empty())
        .map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (percent_decode(k), percent_decode(v))
        })
        .collect()
}

fn percent_decode(s: &str) -> String {
    let bytes = s.replace('+', " ").into_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            if let Ok(v) = u8::from_str_radix(hex, 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
