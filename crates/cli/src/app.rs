//! Everything a command needs to run: settings, sites, presets, and how to
//! build the HTTP layer from them.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use console::style;
use mul_tor_core::{
    AppDirs, Auth, Config, ConfigOrigin, ConfigStore, HttpFactory, PresetLibrary, ProxyMode,
    Registry, Site, SiteInfo, proxies, useragents,
};

pub struct App {
    pub dirs: AppDirs,
    pub store: ConfigStore,
    pub config: Config,
    pub registry: Registry,
    pub presets: PresetLibrary,
}

impl App {
    /// Load (or first-run create) the configuration. `quiet` suppresses the
    /// first-run notices, which go to stderr either way.
    pub fn load(home: Option<&Path>, quiet: bool) -> Result<Self> {
        let dirs = AppDirs::discover(home)?;
        dirs.create_all()?;
        let store = ConfigStore::new(&dirs);
        let cwd = std::env::current_dir().ok();
        let (config, origin) = store.load_or_create(cwd.as_deref())?;

        if !quiet {
            match origin {
                ConfigOrigin::File => {}
                ConfigOrigin::Created => eprintln!(
                    "{} Created a default config at {}",
                    style("[i]").cyan(),
                    store.path().display()
                ),
                ConfigOrigin::ImportedLegacy => eprintln!(
                    "{} Imported your old config.json into {} (the old file was left alone)",
                    style("[i]").cyan(),
                    store.path().display()
                ),
            }
        }

        let presets = PresetLibrary::load(&dirs.presets_dir());
        for (file, reason) in &presets.problems {
            eprintln!("{} Ignoring preset {file}: {reason}", style("[!]").yellow());
        }

        Ok(Self {
            dirs,
            store,
            config,
            registry: Registry::builtin(),
            presets,
        })
    }

    /// Sites that may be offered at all (not blacklisted).
    pub fn eligible_sites(&self) -> Vec<Arc<dyn Site>> {
        self.registry
            .iter()
            .filter(|site| !self.config.is_blacklisted(site.info()))
            .cloned()
            .collect()
    }

    /// Does `site` need credentials that are not filled in?
    pub fn lacks_credentials(&self, site: &SiteInfo) -> bool {
        if site.auth != Auth::Required {
            return false;
        }
        let credentials = self.config.credentials_for(site);
        site.credentials
            .iter()
            .any(|field| credentials.get(field.key).is_none())
    }

    /// The HTTP layer as configured: user agents, proxy.
    ///
    /// `proxy_override` (a command-line flag) beats the config. Fetching the
    /// user-agent list and proxy pool happens here, so this can take a moment.
    pub async fn http_factory(
        &self,
        proxy_override: Option<&str>,
        quiet: bool,
    ) -> Result<HttpFactory> {
        let mut factory = HttpFactory::new();

        if self.config.random_user_agent || self.config.use_proxies {
            // Plain client for these bootstrap downloads.
            let bootstrap = HttpFactory::new()
                .client()
                .context("could not set up the HTTP client")?;

            if self.config.random_user_agent {
                factory = factory.with_user_agents(
                    useragents::load(&self.dirs.user_agents_file(), &bootstrap).await,
                );
            }

            let fixed = proxy_override
                .map(str::to_string)
                .or_else(|| self.config.proxy.clone());
            if let Some(url) = fixed {
                factory = factory.with_proxy(ProxyMode::Fixed(url));
            } else if self.config.use_proxies {
                if !quiet {
                    eprintln!("{}", style("Fetching fresh proxies...").yellow());
                }
                let pool = proxies::fetch_pool(&bootstrap).await;
                if pool.is_empty() {
                    eprintln!(
                        "{} No proxies could be fetched; connecting directly.",
                        style("[!]").yellow()
                    );
                } else {
                    if !quiet {
                        eprintln!("{} Fetched {} proxies.", style("[i]").cyan(), pool.len());
                    }
                    factory = factory.with_proxy(ProxyMode::Pool(pool));
                }
            }
        } else if let Some(url) = proxy_override
            .map(str::to_string)
            .or_else(|| self.config.proxy.clone())
        {
            factory = factory.with_proxy(ProxyMode::Fixed(url));
        }

        Ok(with_test_hooks(factory))
    }
}

/// With the `test-hooks` feature, `MUL_TOR_TEST_BASE_URL=http://127.0.0.1:PORT`
/// redirects every site to a local server (`/host/path`). Compiled out of
/// release builds.
#[cfg(feature = "test-hooks")]
fn with_test_hooks(factory: HttpFactory) -> HttpFactory {
    match std::env::var("MUL_TOR_TEST_BASE_URL") {
        Ok(base) => factory
            .with_proxy(ProxyMode::Direct)
            .with_url_rewrite(Arc::new(move |url: &str| {
                let rest = url
                    .strip_prefix("https://")
                    .or_else(|| url.strip_prefix("http://"))
                    .unwrap_or(url);
                format!("{}/{rest}", base.trim_end_matches('/'))
            })),
        Err(_) => factory,
    }
}

#[cfg(not(feature = "test-hooks"))]
fn with_test_hooks(factory: HttpFactory) -> HttpFactory {
    factory
}

#[cfg(test)]
pub fn test_app(config: Config) -> App {
    let dir = AppDirs::in_dir(std::env::temp_dir().join("mul-tor-test-unused"));
    App {
        store: ConfigStore::new(&dir),
        dirs: dir,
        config,
        registry: Registry::builtin(),
        presets: PresetLibrary::builtin(),
    }
}
