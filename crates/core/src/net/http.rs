//! HTTP client construction: user agents, proxies, TLS provider.
//!
//! Every upload gets its own [`Http`] built by [`HttpFactory::client`], which
//! picks a random user agent and (when a proxy pool is configured) a random
//! proxy, mirroring what the Python version did per upload.

use std::sync::{Arc, Once};
use std::time::Duration;

use rand::seq::IndexedRandom;
use reqwest::{Client, Proxy, RequestBuilder};

/// Timeout for small API round-trips (server lookups, init/finalise calls).
/// Never apply a total timeout to a request that carries file data.
pub const API_TIMEOUT: Duration = Duration::from_secs(120);

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

pub fn default_user_agent() -> String {
    format!(
        "mul-tor/{} (by Official Husko on GitHub)",
        env!("CARGO_PKG_VERSION")
    )
}

/// How outgoing connections are routed.
#[derive(Clone, Debug, Default)]
pub enum ProxyMode {
    /// Honour the usual `HTTPS_PROXY`/`NO_PROXY` environment variables.
    #[default]
    System,
    /// Ignore the environment and connect directly.
    Direct,
    /// Route everything through one proxy (`http://`, `socks5://`, `socks5h://`).
    Fixed(String),
    /// Pick a random proxy from the list for every upload.
    Pool(Vec<String>),
}

/// Rewrites request URLs before they are sent. This exists so tests can point
/// the real `https://…` endpoints at a local mock server.
pub type UrlRewrite = Arc<dyn Fn(&str) -> String + Send + Sync>;

/// A configured HTTP client plus the (test-only) URL rewrite hook.
///
/// Uploaders must create every request through this type rather than through
/// the raw [`reqwest::Client`].
#[derive(Clone)]
pub struct Http {
    client: Client,
    /// The same settings, but HTTP/1.1 only: an upgrade to a WebSocket does not
    /// exist in HTTP/2.
    upgradable: Client,
    rewrite: Option<UrlRewrite>,
    user_agent: String,
}

impl Http {
    fn target(&self, url: &str) -> String {
        match &self.rewrite {
            Some(rewrite) => rewrite(url),
            None => url.to_string(),
        }
    }

    /// GET with the API timeout. GETs in this tool fetch small pages or JSON.
    pub fn get(&self, url: impl AsRef<str>) -> RequestBuilder {
        self.client
            .get(self.target(url.as_ref()))
            .timeout(API_TIMEOUT)
    }

    /// GET that may be upgraded to a WebSocket. It goes over HTTP/1.1 and has no
    /// total timeout, which would cut short an upload that keeps the connection open.
    pub fn get_for_upgrade(&self, url: impl AsRef<str>) -> RequestBuilder {
        self.upgradable.get(self.target(url.as_ref()))
    }

    pub fn post(&self, url: impl AsRef<str>) -> RequestBuilder {
        self.client.post(self.target(url.as_ref()))
    }

    pub fn put(&self, url: impl AsRef<str>) -> RequestBuilder {
        self.client.put(self.target(url.as_ref()))
    }

    pub fn patch(&self, url: impl AsRef<str>) -> RequestBuilder {
        self.client.patch(self.target(url.as_ref()))
    }

    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }
}

impl std::fmt::Debug for Http {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Http")
            .field("user_agent", &self.user_agent)
            .finish_non_exhaustive()
    }
}

/// Builds [`Http`] clients with a fresh user agent / proxy each time.
#[derive(Clone)]
pub struct HttpFactory {
    user_agents: Vec<String>,
    proxy: ProxyMode,
    rewrite: Option<UrlRewrite>,
}

impl Default for HttpFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpFactory {
    pub fn new() -> Self {
        install_crypto_provider();
        Self {
            user_agents: vec![default_user_agent()],
            proxy: ProxyMode::System,
            rewrite: None,
        }
    }

    /// Rotate through these user agents. An empty list keeps the current ones.
    pub fn with_user_agents(mut self, user_agents: Vec<String>) -> Self {
        if !user_agents.is_empty() {
            self.user_agents = user_agents;
        }
        self
    }

    pub fn with_proxy(mut self, proxy: ProxyMode) -> Self {
        self.proxy = proxy;
        self
    }

    /// Test hook: see [`UrlRewrite`].
    #[doc(hidden)]
    pub fn with_url_rewrite(mut self, rewrite: UrlRewrite) -> Self {
        self.rewrite = Some(rewrite);
        self
    }

    pub fn client(&self) -> Result<Http, reqwest::Error> {
        let mut rng = rand::rng();
        let user_agent = self
            .user_agents
            .choose(&mut rng)
            .cloned()
            .unwrap_or_else(default_user_agent);
        // One proxy for the whole upload, whichever client a request uses.
        let proxy = match &self.proxy {
            ProxyMode::Fixed(url) => Some(url.clone()),
            ProxyMode::Pool(pool) => pool.choose(&mut rng).cloned(),
            ProxyMode::System | ProxyMode::Direct => None,
        };

        Ok(Http {
            client: self.build(&user_agent, proxy.as_deref(), false)?,
            upgradable: self.build(&user_agent, proxy.as_deref(), true)?,
            rewrite: self.rewrite.clone(),
            user_agent,
        })
    }

    fn build(
        &self,
        user_agent: &str,
        proxy: Option<&str>,
        http1_only: bool,
    ) -> Result<Client, reqwest::Error> {
        // Deliberately no `read_timeout`/`timeout` here. In reqwest the read
        // timeout starts when the request starts and covers the wait for the
        // response headers, so it would abort any upload that takes longer
        // than the timeout to send. Stalled connections are caught by TCP
        // keepalive instead; small API calls set their own timeout.
        let mut builder = Client::builder()
            .user_agent(user_agent)
            .connect_timeout(CONNECT_TIMEOUT)
            .tcp_keepalive(Some(Duration::from_secs(60)))
            // `User-Agent`, not `user-agent`: HTTP/1 header names are case-
            // insensitive, but the Python client sent them title-cased and some
            // of these old hosts have only ever seen that.
            .http1_title_case_headers();
        if http1_only {
            builder = builder.http1_only();
        }
        match (&self.proxy, proxy) {
            (ProxyMode::Direct, _) => builder = builder.no_proxy(),
            (_, Some(url)) => builder = builder.proxy(Proxy::all(url)?),
            _ => {}
        }
        builder.build()
    }
}

fn install_crypto_provider() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        // Err means another provider is already installed, which is fine.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

#[cfg(test)]
mod tests;
