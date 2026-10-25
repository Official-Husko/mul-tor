//! The page's session: which server to talk to, the cookies it set, and the
//! marker header the API insists on.

use std::sync::LazyLock;

use regex::Regex;
use reqwest::{RequestBuilder, Url};
use serde_json::Value;

use crate::error::UploadError;
use crate::reply::{Reply, SendReply, capture};
use crate::upload::{CookieJar, UploadContext};

/// `var SERVERID = '3';` and `var CHUNKSIZE = '4m';` in the page's script.
static SERVER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"var SERVERID = '(\d{1,3})'").expect("pattern is valid"));
static CHUNK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"var CHUNKSIZE = '(\d{1,3})m'").expect("pattern is valid"));

/// What the page's script puts on every request to the API; without it the
/// API redirects to the home page.
const AJAX: &str = "XMLHttpRequest";
const DEFAULT_CHUNK_MIB: u64 = 4;

pub(super) struct Session<'a> {
    cx: &'a UploadContext<'a>,
    server: String,
    jar: CookieJar,
    pub(super) chunk_size: u64,
}

impl<'a> Session<'a> {
    /// Load the home page: it names the server for this visit and the chunk size.
    pub(super) async fn open(cx: &'a UploadContext<'a>) -> Result<Self, UploadError> {
        let home = cx
            .http
            .get("https://dropmefiles.com/")
            .send_reply()
            .await?
            .ok()?;
        let server = capture(&SERVER, &home.body)
            .ok_or_else(|| UploadError::protocol("no upload server in the page", &home.body))?;
        let mib = capture(&CHUNK, &home.body)
            .and_then(|mib| mib.parse::<u64>().ok())
            .filter(|mib| *mib > 0)
            .unwrap_or(DEFAULT_CHUNK_MIB);
        let mut jar = CookieJar::default();
        jar.absorb(&home);
        Ok(Self {
            cx,
            server,
            jar,
            chunk_size: mib * 1024 * 1024,
        })
    }

    /// The address of an API call on this visit's server.
    pub(super) fn api(&self, path: &str) -> String {
        format!("https://dropmefiles.com/s{}/{path}", self.server)
    }

    /// A `POST` the way the page's script makes it.
    pub(super) fn post(&self, url: impl AsRef<str>) -> RequestBuilder {
        let request = self.cx.http.post(url).header("X-Requested-With", AJAX);
        match self.jar.header() {
            Some(cookies) => request.header("Cookie", cookies),
            None => request,
        }
    }

    /// Send `request`, keep the cookies of the answer, and turn the site's own
    /// error object into an error.
    pub(super) async fn send(&mut self, request: RequestBuilder) -> Result<Reply, UploadError> {
        let reply = request.send_reply().await?.ok()?;
        self.jar.absorb(&reply);
        refused(&reply)?;
        Ok(reply)
    }
}

/// The address of one chunk of the file.
pub(super) fn chunk_url(
    endpoint: &str,
    name: &str,
    chunk: u64,
    chunks: u64,
    uid: &str,
) -> Result<Url, UploadError> {
    let mut url = Url::parse(endpoint)
        .map_err(|e| UploadError::unexpected(format!("invalid URL `{endpoint}`: {e}")))?;
    url.query_pairs_mut()
        .append_pair("name", name)
        .append_pair("chunk", &chunk.to_string())
        .append_pair("chunks", &chunks.to_string())
        .append_pair("updir", uid);
    Ok(url)
}

/// The site answers `{"error": {"code": …, "message": …}}` when it refuses
/// something (a spam filter, the virus check); other replies pass.
fn refused(reply: &Reply) -> Result<(), UploadError> {
    let Ok(json) = serde_json::from_str::<Value>(&reply.body) else {
        return Ok(());
    };
    let Some(error) = json.get("error") else {
        return Ok(());
    };
    let why = error
        .get("message")
        .and_then(Value::as_str)
        .filter(|why| why.len() <= 80 && why.chars().all(|c| c.is_ascii_graphic() || c == ' '))
        .unwrap_or("no reason given");
    Err(UploadError::protocol(
        format!("the site refused the request: {why}"),
        &reply.body,
    ))
}
