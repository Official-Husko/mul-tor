//! Account details a provider needs, and where they are stored.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// A secret or account detail the provider needs (API key, e-mail, …).
///
/// `key` is the name used in the config file, and is kept identical to the
/// Python tool's `config.json` keys so existing credentials import unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CredentialField {
    pub key: &'static str,
    pub label: &'static str,
    /// Mask the value in UIs.
    pub secret: bool,
}

impl CredentialField {
    pub const fn new(key: &'static str, label: &'static str, secret: bool) -> Self {
        Self { key, label, secret }
    }
}

/// Credentials for one provider, keyed by [`CredentialField::key`].
///
/// `Debug` deliberately prints only the key names.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Credentials(BTreeMap<String, String>);

impl Credentials {
    /// The value, or `None` if it is missing or blank.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0
            .get(key)
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.0.insert(key.into(), value.into());
    }

    pub fn is_empty(&self) -> bool {
        self.0.values().all(|value| value.trim().is_empty())
    }

    /// The value for `key`, created empty if absent. For editing in a UI.
    pub fn entry(&mut self, key: impl Into<String>) -> &mut String {
        self.0.entry(key.into()).or_default()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("keys", &self.0.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl<K: Into<String>, V: Into<String>> FromIterator<(K, V)> for Credentials {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self(
            iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_credentials_count_as_missing() {
        let mut creds = Credentials::default();
        creds.set("apiKey", "   ");
        assert!(creds.get("apiKey").is_none());
        assert!(creds.is_empty());
    }

    #[test]
    fn credentials_debug_never_prints_values() {
        let mut creds = Credentials::default();
        creds.set("apiKey", "hunter2");
        let shown = format!("{creds:?}");
        assert!(shown.contains("apiKey"));
        assert!(!shown.contains("hunter2"));
    }
}
