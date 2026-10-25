//! The settings themselves.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::provider::Credentials;

/// Current layout of `config.toml`.
pub const CONFIG_VERSION: u32 = 2;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PresetSettings {
    /// Pre-select the providers of `name` instead of asking.
    pub auto_load: bool,
    /// Preset file name, identifier or title (`f95`, `f95.json`, `husko.f95`).
    pub name: String,
    /// Offer a preset choice in interactive mode.
    pub enable_selection: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub check_for_updates: bool,
    /// Ping every provider before listing it and hide the ones that do not answer.
    pub check_availability: bool,
    /// Route uploads through a random proxy from a freshly scraped public list.
    pub use_proxies: bool,
    /// One fixed proxy (`http://`, `socks5://`, `socks5h://`). Wins over `use_proxies`.
    pub proxy: Option<String>,
    /// Rotate through real browser user agents instead of identifying as Mul-Tor.
    pub random_user_agent: bool,
    /// Append every link to `file_links.txt`.
    pub save_links: bool,
    /// How many uploads may run at once.
    pub parallel_uploads: usize,
    /// Where `file_links.txt` goes. Default: the current directory (CLI).
    pub links_dir: Option<PathBuf>,
    /// Providers that are never offered. Matched like provider names.
    pub blacklist: Vec<String>,
    /// How much detail the log files get: error, warn, info, debug or trace.
    pub log_level: String,
    /// Write the log files as JSON lines instead of plain text.
    pub log_json: bool,
    pub preset: PresetSettings,
    /// Per-provider credentials, keyed by provider name.
    pub credentials: BTreeMap<String, Credentials>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            check_for_updates: true,
            check_availability: true,
            use_proxies: false,
            proxy: None,
            random_user_agent: false,
            save_links: true,
            parallel_uploads: 1,
            links_dir: None,
            blacklist: Vec::new(),
            log_level: "info".to_string(),
            log_json: false,
            preset: PresetSettings::default(),
            credentials: BTreeMap::new(),
        }
    }
}

impl Config {
    /// Parallelism clamped to something sensible.
    pub fn parallel(&self) -> usize {
        self.parallel_uploads.clamp(1, 16)
    }

    pub fn parse(text: &str, origin: &Path) -> Result<Config> {
        toml::from_str(text).map_err(|e| Error::Config {
            path: origin.to_path_buf(),
            message: e.message().to_string(),
        })
    }

    /// The commented file written on first run. Parses to [`Config::default`].
    pub fn template() -> &'static str {
        include_str!("template.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_documented_template_parses_to_the_defaults() {
        let parsed = Config::parse(Config::template(), Path::new("template")).unwrap();
        assert_eq!(parsed, Config::default());
    }

    #[test]
    fn config_round_trips_through_toml() {
        let mut config = Config {
            use_proxies: true,
            proxy: Some("socks5h://127.0.0.1:9050".into()),
            blacklist: vec!["Oshi".into()],
            parallel_uploads: 3,
            ..Config::default()
        };
        config.preset.name = "f95".into();
        let mut creds = Credentials::default();
        creds.set("apiKey", "k");
        config.credentials.insert("Pixeldrain".into(), creds);

        let text = toml::to_string_pretty(&config).unwrap();
        assert_eq!(Config::parse(&text, Path::new("x")).unwrap(), config);
    }

    #[test]
    fn unknown_keys_are_tolerated_and_missing_ones_default() {
        let config =
            Config::parse("use_proxies = true\nfuture_option = 1\n", Path::new("x")).unwrap();
        assert!(config.use_proxies);
        assert!(config.check_for_updates);
    }

    #[test]
    fn type_errors_name_the_file() {
        let err =
            Config::parse("use_proxies = \"yes\"", Path::new("/etc/mt/config.toml")).unwrap_err();
        assert!(err.to_string().contains("/etc/mt/config.toml"), "{err}");
    }
}
