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

        // Deliberately no `read_timeout`/`timeout` here. In reqwest the read
        // timeout starts when the request starts and covers the wait for the
        // response headers, so it would abort any upload that takes longer
        // than the timeout to send. Stalled connections are caught by TCP
        // keepalive instead; small API calls set their own timeout.
        let mut builder = Client::builder()
            .user_agent(&user_agent)
            .connect_timeout(CONNECT_TIMEOUT)
            .tcp_keepalive(Some(Duration::from_secs(60)));

        match &self.proxy {
            ProxyMode::System => {}
            ProxyMode::Direct => builder = builder.no_proxy(),
            ProxyMode::Fixed(url) => builder = builder.proxy(Proxy::all(url)?),
            ProxyMode::Pool(pool) => {
                if let Some(url) = pool.choose(&mut rng) {
                    builder = builder.proxy(Proxy::all(url)?);
                }
            }
        }

        Ok(Http {
            client: builder.build()?,
            rewrite: self.rewrite.clone(),
            user_agent,
        })
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
mod tests {
    use super::*;

    #[test]
    fn client_builds_for_every_proxy_mode() {
        for mode in [
            ProxyMode::System,
            ProxyMode::Direct,
            ProxyMode::Fixed("socks5h://127.0.0.1:9050".into()),
            ProxyMode::Pool(vec!["http://127.0.0.1:8080".into()]),
            ProxyMode::Pool(vec![]),
        ] {
            HttpFactory::new().with_proxy(mode).client().unwrap();
        }
    }

    #[test]
    fn invalid_proxy_url_is_an_error_not_a_panic() {
        let result = HttpFactory::new()
            .with_proxy(ProxyMode::Fixed("not a url".into()))
            .client();
        assert!(result.is_err());
    }

    #[test]
    fn user_agent_comes_from_the_configured_list() {
        let http = HttpFactory::new()
            .with_user_agents(vec!["UA/1".into()])
            .client()
            .unwrap();
        assert_eq!(http.user_agent(), "UA/1");
    }

    #[test]
    fn empty_user_agent_list_keeps_the_default() {
        let http = HttpFactory::new()
            .with_user_agents(vec![])
            .client()
            .unwrap();
        assert!(http.user_agent().starts_with("mul-tor/"));
    }
}
