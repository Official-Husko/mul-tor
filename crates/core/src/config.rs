//! User settings (`config.toml`) and import of the Python version's `config.json`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::paths::AppDirs;
use crate::site::{Auth, Credentials, SiteInfo, normalize_name};

/// Current layout of `config.toml`.
pub const CONFIG_VERSION: u32 = 2;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct PresetSettings {
    /// Pre-select the sites of `name` instead of asking.
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
    /// Ping every site before listing it and hide the ones that do not answer.
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
    /// Sites that are never offered. Matched like site names.
    pub blacklist: Vec<String>,
    pub preset: PresetSettings,
    /// Per-site credentials, keyed by site name.
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
            preset: PresetSettings::default(),
            credentials: BTreeMap::new(),
        }
    }
}

impl Config {
    /// Credentials stored for `site` (empty if none).
    pub fn credentials_for(&self, site: &SiteInfo) -> Credentials {
        let id = site.id();
        self.credentials
            .iter()
            .find(|(name, _)| normalize_name(name) == id)
            .map(|(_, creds)| creds.clone())
            .unwrap_or_default()
    }

    /// Store credentials for `site`, replacing any entry spelled differently.
    pub fn set_credentials(&mut self, site: &SiteInfo, credentials: Credentials) {
        let id = site.id();
        self.credentials
            .retain(|name, _| normalize_name(name) != id);
        if !credentials.is_empty() {
            self.credentials.insert(site.name.to_string(), credentials);
        }
    }

    /// All credentials by site id, ready for the engine.
    pub fn credentials_by_id(&self) -> BTreeMap<String, Credentials> {
        self.credentials
            .iter()
            .map(|(name, creds)| (normalize_name(name), creds.clone()))
            .collect()
    }

    /// Does `site` need credentials that are not filled in?
    pub fn lacks_credentials(&self, site: &SiteInfo) -> bool {
        if site.auth != Auth::Required {
            return false;
        }
        let credentials = self.credentials_for(site);
        site.credentials
            .iter()
            .any(|field| credentials.get(field.key).is_none())
    }

    pub fn is_blacklisted(&self, site: &SiteInfo) -> bool {
        self.blacklist.iter().any(|entry| site.matches(entry))
    }

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

    /// Import the Python version's `config.json`.
    pub fn from_legacy_json(text: &str, origin: &Path) -> Result<Config> {
        #[derive(Deserialize, Default)]
        #[serde(default)]
        struct Legacy {
            #[serde(rename = "checkForUpdates")]
            check_for_updates: Option<bool>,
            #[serde(rename = "useProxies")]
            use_proxies: Option<bool>,
            #[serde(rename = "saveLinksToFile")]
            save_links_to_file: Option<bool>,
            #[serde(rename = "randomUserAgent")]
            random_user_agent: Option<bool>,
            #[serde(rename = "presetSystem")]
            preset_system: LegacyPreset,
            api_keys: BTreeMap<String, BTreeMap<String, String>>,
            blacklist: Vec<String>,
        }
        #[derive(Deserialize, Default)]
        #[serde(default)]
        struct LegacyPreset {
            #[serde(rename = "autoLoadPreset")]
            auto_load: Option<bool>,
            #[serde(rename = "presetName")]
            name: Option<String>,
            #[serde(rename = "enablePresetSelection")]
            enable_selection: Option<bool>,
        }

        /// The placeholder entries the Python tool wrote into every new config.
        const PLACEHOLDER_BLACKLIST: [&str; 3] =
            ["SomeSiteName", "CheapGoFileCopy", "HotSinglesInYourArea"];

        let legacy: Legacy = serde_json::from_str(text).map_err(|e| Error::Config {
            path: origin.to_path_buf(),
            message: e.to_string(),
        })?;

        let defaults = Config::default();
        Ok(Config {
            check_for_updates: legacy
                .check_for_updates
                .unwrap_or(defaults.check_for_updates),
            use_proxies: legacy.use_proxies.unwrap_or(defaults.use_proxies),
            save_links: legacy.save_links_to_file.unwrap_or(defaults.save_links),
            random_user_agent: legacy
                .random_user_agent
                .unwrap_or(defaults.random_user_agent),
            blacklist: legacy
                .blacklist
                .into_iter()
                .filter(|entry| !PLACEHOLDER_BLACKLIST.contains(&entry.as_str()))
                .collect(),
            preset: PresetSettings {
                auto_load: legacy.preset_system.auto_load.unwrap_or(false),
                name: legacy.preset_system.name.unwrap_or_default(),
                enable_selection: legacy.preset_system.enable_selection.unwrap_or(false),
            },
            credentials: legacy
                .api_keys
                .into_iter()
                .map(|(site, fields)| (site, fields.into_iter().collect::<Credentials>()))
                .filter(|(_, creds)| !creds.is_empty())
                .collect(),
            ..defaults
        })
    }

    /// The commented file written on first run. Parses to [`Config::default`].
    pub fn template() -> &'static str {
        include_str!("config_template.toml")
    }
}

/// How the config was obtained at startup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigOrigin {
    /// Read from an existing `config.toml`.
    File,
    /// `config.toml` did not exist; a documented default was written.
    Created,
    /// `config.toml` did not exist; the Python version's `config.json` was imported.
    ImportedLegacy,
}

/// Loads and saves `config.toml`.
#[derive(Clone, Debug)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new(dirs: &AppDirs) -> Self {
        Self {
            path: dirs.config_file(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read the config, creating it if needed.
    ///
    /// If there is no `config.toml` yet, a `config.json` in `legacy_dir` (the
    /// Python version kept it next to the program) is imported; the old file
    /// is left untouched.
    pub fn load_or_create(&self, legacy_dir: Option<&Path>) -> Result<(Config, ConfigOrigin)> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => return Ok((Config::parse(&text, &self.path)?, ConfigOrigin::File)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::io(format!("reading {}", self.path.display()), e)),
        }

        if let Some(dir) = legacy_dir {
            let legacy_path = dir.join("config.json");
            if let Ok(text) = std::fs::read_to_string(&legacy_path) {
                let config = Config::from_legacy_json(&text, &legacy_path)?;
                self.save(&config)?;
                return Ok((config, ConfigOrigin::ImportedLegacy));
            }
        }

        self.write(Config::template())?;
        Ok((Config::default(), ConfigOrigin::Created))
    }

    /// Write `config`. Comments in a hand-edited file are not preserved.
    pub fn save(&self, config: &Config) -> Result<()> {
        let body = toml::to_string_pretty(config)
            .map_err(|e| Error::Other(format!("could not serialise the config: {e}")))?;
        self.write(&format!(
            "# Mul-Tor settings. Saved by the program, so comments are not kept;\n\
             # see the documentation for what each option does.\n\n{body}"
        ))
    }

    fn write(&self, contents: &str) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::io(format!("creating {}", parent.display()), e))?;
        }
        std::fs::write(&self.path, contents)
            .map_err(|e| Error::io(format!("writing {}", self.path.display()), e))?;
        restrict_permissions(&self.path);
        Ok(())
    }
}

/// The file holds API keys and passwords; keep other users out where the
/// platform supports it.
fn restrict_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    let _ = path;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::size::SizeLimit;

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

    const LEGACY: &str = r#"{
        "version": "1.5.0",
        "checkForUpdates": false,
        "useProxies": true,
        "saveLinksToFile": false,
        "randomUserAgent": true,
        "presetSystem": {"autoLoadPreset": true, "presetName": "f95.json", "enablePresetSelection": true},
        "advancedMode": false,
        "api_keys": {
            "BowFile": {"apiKey1": "a", "apiKey2": "b"},
            "DooDrive": {"apiKey": "", "apiToken": ""},
            "Pixeldrain": {"apiKey": "pd"}
        },
        "blacklist": ["SomeSiteName", "CheapGoFileCopy", "HotSinglesInYourArea", "Oshi"]
    }"#;

    #[test]
    fn python_config_json_is_imported() {
        let config = Config::from_legacy_json(LEGACY, Path::new("config.json")).unwrap();
        assert!(!config.check_for_updates);
        assert!(config.use_proxies);
        assert!(!config.save_links);
        assert!(config.random_user_agent);
        assert!(config.preset.auto_load && config.preset.enable_selection);
        assert_eq!(config.preset.name, "f95.json");
        // Placeholder entries from the Python template are dropped, real ones kept.
        assert_eq!(config.blacklist, ["Oshi"]);
        // Empty credential sets are not carried over.
        assert_eq!(config.credentials.len(), 2);
        assert_eq!(config.credentials["BowFile"].get("apiKey2"), Some("b"));
    }

    #[test]
    fn credentials_are_found_regardless_of_spelling() {
        let mut config = Config::default();
        let mut creds = Credentials::default();
        creds.set("apiKey", "k");
        config.credentials.insert("pixeldrain".into(), creds);
        let info = SiteInfo::new("Pixeldrain", "https://p", "https://p").max_size(SizeLimit::gb(1));
        assert_eq!(config.credentials_for(&info).get("apiKey"), Some("k"));
        assert!(config.credentials_by_id().contains_key("pixeldrain"));
    }

    #[test]
    fn set_credentials_replaces_differently_spelled_entries() {
        let info = SiteInfo::new("BowFile", "https://b", "https://b");
        let mut config = Config::default();
        config
            .credentials
            .insert("bowfile".into(), Credentials::default());
        let mut creds = Credentials::default();
        creds.set("apiKey1", "x");
        config.set_credentials(&info, creds);
        assert_eq!(config.credentials.len(), 1);
        assert!(config.credentials.contains_key("BowFile"));

        config.set_credentials(&info, Credentials::default());
        assert!(
            config.credentials.is_empty(),
            "blank credentials are removed"
        );
    }

    #[test]
    fn lacking_credentials_means_a_required_field_is_blank() {
        let pixeldrain = crate::Registry::builtin()
            .find("pixeldrain")
            .unwrap()
            .clone();
        let oshi = crate::Registry::builtin().find("oshi").unwrap().clone();
        let mut config = Config::default();
        assert!(config.lacks_credentials(pixeldrain.info()));
        assert!(
            !config.lacks_credentials(oshi.info()),
            "anonymous sites never lack any"
        );

        let mut creds = Credentials::default();
        creds.set("apiKey", "   ");
        config.set_credentials(pixeldrain.info(), creds.clone());
        assert!(
            config.lacks_credentials(pixeldrain.info()),
            "blank is missing"
        );
        creds.set("apiKey", "k");
        config.set_credentials(pixeldrain.info(), creds);
        assert!(!config.lacks_credentials(pixeldrain.info()));
    }

    #[test]
    fn blacklist_matches_names_loosely() {
        let config = Config {
            blacklist: vec!["transfer_sh".into()],
            ..Config::default()
        };
        assert!(config.is_blacklisted(&SiteInfo::new("Transfer.sh", "https://t", "https://t")));
        assert!(!config.is_blacklisted(&SiteInfo::new("Oshi", "https://o", "https://o")));
    }

    #[test]
    fn store_creates_a_documented_default_on_first_run() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ConfigStore::new(&AppDirs::in_dir(tmp.path()));
        let (config, origin) = store.load_or_create(None).unwrap();
        assert_eq!(origin, ConfigOrigin::Created);
        assert_eq!(config, Config::default());
        assert!(std::fs::read_to_string(store.path()).unwrap().contains('#'));

        let (_, origin) = store.load_or_create(None).unwrap();
        assert_eq!(origin, ConfigOrigin::File);
    }

    #[test]
    fn store_imports_a_legacy_config_once_and_leaves_it_alone() {
        let tmp = tempfile::tempdir().unwrap();
        let legacy_dir = tmp.path().join("old");
        std::fs::create_dir(&legacy_dir).unwrap();
        std::fs::write(legacy_dir.join("config.json"), LEGACY).unwrap();

        let store = ConfigStore::new(&AppDirs::in_dir(tmp.path().join("new")));
        let (config, origin) = store.load_or_create(Some(&legacy_dir)).unwrap();
        assert_eq!(origin, ConfigOrigin::ImportedLegacy);
        assert_eq!(config.credentials["Pixeldrain"].get("apiKey"), Some("pd"));
        assert!(legacy_dir.join("config.json").exists());

        // Second start reads the new file, not the legacy one.
        let (again, origin) = store.load_or_create(Some(&legacy_dir)).unwrap();
        assert_eq!(origin, ConfigOrigin::File);
        assert_eq!(again, config);
    }

    #[test]
    fn a_broken_config_is_an_error_not_silently_replaced() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ConfigStore::new(&AppDirs::in_dir(tmp.path()));
        std::fs::write(store.path(), "this is = not [valid").unwrap();
        assert!(store.load_or_create(None).is_err());
        assert_eq!(
            std::fs::read_to_string(store.path()).unwrap(),
            "this is = not [valid"
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_config_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let store = ConfigStore::new(&AppDirs::in_dir(tmp.path()));
        store.load_or_create(None).unwrap();
        let mode = std::fs::metadata(store.path())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0, "mode is {mode:o}");
    }
}
