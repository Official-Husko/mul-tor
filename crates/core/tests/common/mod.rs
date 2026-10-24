//! Test harness: an in-process HTTP server that records every request, plus
//! helpers to run an uploader against it.
//!
//! Uploaders talk to their real `https://host/path` URLs. [`Mock::rewrite`]
//! maps those onto `http://127.0.0.1:PORT/host/path`, so a handler can tell
//! providers apart by `host` and nothing ever leaves the machine.

#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Request, Response, StatusCode};
use mul_tor_core::http::UrlRewrite;
use mul_tor_core::{
    Credentials, HttpFactory, LocalFile, Progress, Provider, ProxyMode, UploadContext, UploadError,
    UploadedFile,
};

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

#[derive(Clone, Debug)]
pub struct FormPart {
    pub name: String,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    /// The `Content-Disposition` line exactly as sent.
    pub disposition: String,
    pub data: Vec<u8>,
}

impl FormPart {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.data).into_owned()
    }
}

/// Find a part by field name.
pub fn field<'a>(parts: &'a [FormPart], name: &str) -> &'a FormPart {
    parts.iter().find(|p| p.name == name).unwrap_or_else(|| {
        panic!(
            "no multipart field `{name}`; got {:?}",
            parts.iter().map(|p| &p.name).collect::<Vec<_>>()
        )
    })
}

pub fn text_field(parts: &[FormPart], name: &str) -> String {
    field(parts, name).text()
}

pub fn parse_multipart(content_type: &str, body: &[u8]) -> Vec<FormPart> {
    let boundary = content_type
        .split(';')
        .filter_map(|p| p.trim().strip_prefix("boundary="))
        .next()
        .unwrap_or_else(|| panic!("no boundary in `{content_type}`"))
        .trim_matches('"');
    let delimiter = format!("--{boundary}").into_bytes();

    let mut parts = Vec::new();
    let mut rest = body;
    // Skip the preamble up to the first delimiter.
    let Some(start) = find(rest, &delimiter) else {
        return parts;
    };
    rest = &rest[start + delimiter.len()..];
    loop {
        if rest.starts_with(b"--") {
            break; // closing delimiter
        }
        rest = rest.strip_prefix(b"\r\n").unwrap_or(rest);
        let Some(header_end) = find(rest, b"\r\n\r\n") else {
            break;
        };
        let headers = String::from_utf8_lossy(&rest[..header_end]).into_owned();
        let after_headers = &rest[header_end + 4..];
        let mut next = delimiter.clone();
        next.splice(0..0, *b"\r\n");
        let Some(data_end) = find(after_headers, &next) else {
            break;
        };
        let data = after_headers[..data_end].to_vec();
        rest = &after_headers[data_end + next.len()..];

        let disposition = headers
            .lines()
            .find(|l| l.to_ascii_lowercase().starts_with("content-disposition"))
            .unwrap_or("")
            .to_string();
        let content_type = headers
            .lines()
            .find_map(|l| l.strip_prefix("Content-Type: "))
            .map(str::to_string);
        parts.push(FormPart {
            name: quoted(&disposition, "name").unwrap_or_default(),
            filename: quoted(&disposition, "filename"),
            content_type,
            disposition,
            data,
        });
    }
    parts
}

fn quoted(disposition: &str, key: &str) -> Option<String> {
    // Match ` name="` / `; filename="` without confusing `name` and `filename`.
    let needle = format!("{key}=\"");
    let mut search = disposition;
    while let Some(pos) = search.find(&needle) {
        let preceded_ok = pos == 0 || matches!(search.as_bytes()[pos - 1], b' ' | b';');
        let value = &search[pos + needle.len()..];
        if preceded_ok {
            // Read up to the first unescaped quote, undoing `\"` and `\\`.
            let mut out = String::new();
            let mut chars = value.chars();
            while let Some(c) = chars.next() {
                match c {
                    '"' => return Some(out),
                    '\\' => out.extend(chars.next()),
                    c => out.push(c),
                }
            }
            return None;
        }
        search = &search[pos + needle.len()..];
    }
    None
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// What a handler answers with.
#[derive(Clone, Debug)]
pub struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Reply {
    pub fn text(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            headers: vec![("content-type".into(), "text/plain".into())],
            body: body.into().into_bytes(),
        }
    }

    pub fn html(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            headers: vec![("content-type".into(), "text/html".into())],
            body: body.into().into_bytes(),
        }
    }

    pub fn json(value: serde_json::Value) -> Self {
        Self {
            status: 200,
            headers: vec![("content-type".into(), "application/json".into())],
            body: value.to_string().into_bytes(),
        }
    }

    pub fn status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn not_found() -> Self {
        Reply::text("no such route").status(404)
    }
}

type Handler = dyn Fn(&Recorded) -> Reply + Send + Sync;

pub struct Mock {
    port: u16,
    requests: Arc<Mutex<Vec<Recorded>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Mock {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Mock {
    pub async fn start(handler: impl Fn(&Recorded) -> Reply + Send + Sync + 'static) -> Mock {
        let requests: Arc<Mutex<Vec<Recorded>>> = Arc::default();
        let handler: Arc<Handler> = Arc::new(handler);
        let log = requests.clone();

        let app = Router::new().fallback(move |request: Request<Body>| {
            let handler = handler.clone();
            let log = log.clone();
            async move {
                let (parts, body) = request.into_parts();
                let body = to_bytes(body, usize::MAX).await.unwrap().to_vec();
                // `/host/the/real/path` -> host + path
                let full = parts.uri.path().trim_start_matches('/');
                let (host, path) = match full.split_once('/') {
                    Some((host, rest)) => (host.to_string(), format!("/{rest}")),
                    None => (full.to_string(), "/".to_string()),
                };
                let recorded = Recorded {
                    method: parts.method.to_string(),
                    host,
                    path,
                    query: parts.uri.query().unwrap_or("").to_string(),
                    headers: parts.headers,
                    body,
                };
                let reply = handler(&recorded);
                log.lock().unwrap().push(recorded);

                let mut response =
                    Response::builder().status(StatusCode::from_u16(reply.status).unwrap());
                for (name, value) in &reply.headers {
                    response = response.header(
                        HeaderName::from_bytes(name.as_bytes()).unwrap(),
                        HeaderValue::from_str(value).unwrap(),
                    );
                }
                response.body(Body::from(reply.body)).unwrap()
            }
        });

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Mock {
            port,
            requests,
            task,
        }
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.requests.lock().unwrap().clone()
    }

    /// Requests that were *not* the given host (e.g. to spot leaks).
    pub fn requests_to(&self, host: &str) -> Vec<Recorded> {
        self.requests()
            .into_iter()
            .filter(|r| r.host == host)
            .collect()
    }

    /// Maps `https://host/path?q` onto this server as `/host/path?q`.
    pub fn rewrite(&self) -> UrlRewrite {
        let port = self.port;
        Arc::new(move |url: &str| {
            let rest = url
                .strip_prefix("https://")
                .or_else(|| url.strip_prefix("http://"))
                .unwrap_or(url);
            format!("http://127.0.0.1:{port}/{rest}")
        })
    }
}

/// Outcome of running one uploader against a [`Mock`].
pub struct Run {
    pub result: Result<UploadedFile, UploadError>,
    pub progress: Progress,
    pub file: LocalFile,
}

impl Run {
    pub fn url(&self) -> &str {
        match &self.result {
            Ok(file) => &file.url,
            Err(e) => panic!("upload failed: {e}"),
        }
    }

    pub fn error(&self) -> &UploadError {
        match &self.result {
            Ok(file) => panic!("expected an error, got {}", file.url),
            Err(e) => e,
        }
    }
}

pub struct RunConfig {
    pub name: String,
    pub content: Vec<u8>,
    pub credentials: Credentials,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            name: "sample.bin".into(),
            content: b"0123456789abcdef".to_vec(),
            credentials: Credentials::default(),
        }
    }
}

pub fn creds(pairs: &[(&str, &str)]) -> Credentials {
    pairs.iter().copied().collect()
}

pub async fn run(provider: &dyn Provider, mock: &Mock, config: RunConfig) -> Run {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(&config.name);
    std::fs::write(&path, &config.content).unwrap();
    let file = LocalFile::open(&path).await.unwrap();

    let http = HttpFactory::new()
        .with_proxy(ProxyMode::Direct)
        .with_url_rewrite(mock.rewrite())
        .client()
        .unwrap();
    let progress = Progress::new(file.size);
    let cx = UploadContext {
        file: &file,
        http: &http,
        credentials: &config.credentials,
        progress: &progress,
    };
    let result = provider.upload(&cx).await;
    Run {
        result,
        progress,
        file,
    }
}

/// Shorthand: run with default content and no credentials.
pub async fn run_default(provider: &dyn Provider, mock: &Mock) -> Run {
    run(provider, mock, RunConfig::default()).await
}

pub fn builtin(name: &str) -> std::sync::Arc<dyn Provider> {
    mul_tor_core::Registry::builtin()
        .find(name)
        .unwrap_or_else(|| panic!("provider `{name}` is not registered"))
        .clone()
}
