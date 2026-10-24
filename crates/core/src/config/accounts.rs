//! Per-provider credentials and the blacklist, looked up by loose name.

use std::collections::BTreeMap;

use super::model::Config;
use crate::provider::{Auth, Credentials, ProviderInfo, normalize_name};

impl Config {
    /// Credentials stored for `provider` (empty if none).
    pub fn credentials_for(&self, provider: &ProviderInfo) -> Credentials {
        let id = provider.id();
        self.credentials
            .iter()
            .find(|(name, _)| normalize_name(name) == id)
            .map(|(_, creds)| creds.clone())
            .unwrap_or_default()
    }

    /// Store credentials for `provider`, replacing any entry spelled differently.
    pub fn set_credentials(&mut self, provider: &ProviderInfo, credentials: Credentials) {
        let id = provider.id();
        self.credentials
            .retain(|name, _| normalize_name(name) != id);
        if !credentials.is_empty() {
            self.credentials
                .insert(provider.name.to_string(), credentials);
        }
    }

    /// All credentials by provider id, ready for the engine.
    pub fn credentials_by_id(&self) -> BTreeMap<String, Credentials> {
        self.credentials
            .iter()
            .map(|(name, creds)| (normalize_name(name), creds.clone()))
            .collect()
    }

    /// Does `provider` need credentials that are not filled in?
    pub fn lacks_credentials(&self, provider: &ProviderInfo) -> bool {
        if provider.auth != Auth::Required {
            return false;
        }
        let credentials = self.credentials_for(provider);
        provider
            .credentials
            .iter()
            .any(|field| credentials.get(field.key).is_none())
    }

    pub fn is_blacklisted(&self, provider: &ProviderInfo) -> bool {
        self.blacklist.iter().any(|entry| provider.matches(entry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::size::SizeLimit;

    #[test]
    fn credentials_are_found_regardless_of_spelling() {
        let mut config = Config::default();
        let mut creds = Credentials::default();
        creds.set("apiKey", "k");
        config.credentials.insert("pixeldrain".into(), creds);
        let info =
            ProviderInfo::new("Pixeldrain", "https://p", "https://p").max_size(SizeLimit::gb(1));
        assert_eq!(config.credentials_for(&info).get("apiKey"), Some("k"));
        assert!(config.credentials_by_id().contains_key("pixeldrain"));
    }

    #[test]
    fn set_credentials_replaces_differently_spelled_entries() {
        let info = ProviderInfo::new("BowFile", "https://b", "https://b");
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
            "anonymous providers never lack any"
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
        assert!(config.is_blacklisted(&ProviderInfo::new("Transfer.sh", "https://t", "https://t")));
        assert!(!config.is_blacklisted(&ProviderInfo::new("Oshi", "https://o", "https://o")));
    }
}
