//! Importing the Python version's `config.json`.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use super::model::{Config, PresetSettings};
use crate::error::{Error, Result};
use crate::provider::Credentials;

impl Config {
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
                .map(|(provider, fields)| (provider, fields.into_iter().collect::<Credentials>()))
                .filter(|(_, creds)| !creds.is_empty())
                .collect(),
            ..defaults
        })
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub const LEGACY: &str = r#"{
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
}
