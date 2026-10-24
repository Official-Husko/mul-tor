//! Everything a command needs to run: settings, providers, presets, and how to
//! build the HTTP layer from them.

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use console::style;
use mul_tor_core::{
    AppDirs, Config, ConfigOrigin, ConfigStore, HttpFactory, PresetLibrary, Provider, ProviderInfo,
    Registry, setup,
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

    /// Providers that may be offered at all (not blacklisted).
    pub fn eligible_providers(&self) -> Vec<Arc<dyn Provider>> {
        self.registry
            .iter()
            .filter(|provider| !self.config.is_blacklisted(provider.info()))
            .cloned()
            .collect()
    }

    /// Does `provider` need credentials that are not filled in?
    pub fn lacks_credentials(&self, provider: &ProviderInfo) -> bool {
        self.config.lacks_credentials(provider)
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
        if !quiet
            && self.config.use_proxies
            && self.config.proxy.is_none()
            && proxy_override.is_none()
        {
            eprintln!("{}", style("Fetching fresh proxies...").yellow());
        }
        let setup = setup::http_factory(&self.config, &self.dirs, proxy_override).await;
        if !quiet {
            for note in &setup.notes {
                eprintln!("{} {note}", style("[i]").cyan());
            }
        }
        Ok(with_test_hooks(setup.factory))
    }
}

/// With the `test-hooks` feature, `MUL_TOR_TEST_BASE_URL=http://127.0.0.1:PORT`
/// redirects every provider to a local server (`/host/path`). Compiled out of
/// release builds.
#[cfg(feature = "test-hooks")]
fn with_test_hooks(factory: HttpFactory) -> HttpFactory {
    match std::env::var("MUL_TOR_TEST_BASE_URL") {
        Ok(base) => factory
            .with_proxy(mul_tor_core::ProxyMode::Direct)
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
