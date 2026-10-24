//! Building the HTTP layer from the user's settings. Shared by every front-end.

use crate::config::Config;
use crate::http::{HttpFactory, ProxyMode};
use crate::paths::AppDirs;
use crate::{proxies, useragents};

/// A configured [`HttpFactory`] and things worth telling the user about.
pub struct HttpSetup {
    pub factory: HttpFactory,
    pub notes: Vec<String>,
}

/// Apply `config` (random user agents, fixed proxy or scraped proxy pool).
///
/// `proxy_override` (for example a command-line flag) beats the configured
/// proxy. This downloads the user-agent list and proxy pool when they are
/// enabled, so it can take a few seconds.
pub async fn http_factory(
    config: &Config,
    dirs: &AppDirs,
    proxy_override: Option<&str>,
) -> HttpSetup {
    let mut factory = HttpFactory::new();
    let mut notes = Vec::new();

    let fixed = proxy_override
        .map(str::to_string)
        .or_else(|| config.proxy.clone())
        .filter(|proxy| !proxy.trim().is_empty());

    if config.random_user_agent || (fixed.is_none() && config.use_proxies) {
        // A plain client for these bootstrap downloads.
        match HttpFactory::new().client() {
            Ok(bootstrap) => {
                if config.random_user_agent {
                    factory = factory.with_user_agents(
                        useragents::load(&dirs.user_agents_file(), &bootstrap).await,
                    );
                }
                if fixed.is_none() && config.use_proxies {
                    let pool = proxies::fetch_pool(&bootstrap).await;
                    if pool.is_empty() {
                        notes.push("No proxies could be fetched; connecting directly.".to_string());
                    } else {
                        notes.push(format!("Fetched {} proxies.", pool.len()));
                        factory = factory.with_proxy(ProxyMode::Pool(pool));
                    }
                }
            }
            Err(error) => notes.push(format!("Could not set up the HTTP client: {error}")),
        }
    }

    if let Some(url) = fixed {
        factory = factory.with_proxy(ProxyMode::Fixed(url));
    }
    HttpSetup { factory, notes }
}
