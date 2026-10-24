//! What a supported site *is*: static metadata ([`SiteInfo`]) plus the
//! [`Site::upload`] implementation.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::UploadError;
use crate::size::SizeLimit;
use crate::upload::{LocalFile, UploadContext, UploadedFile};

/// Lower-cases and drops everything that is not a letter or digit, so
/// `Transfer.sh`, `Transfer_sh` and `transfersh` all name the same site.
pub fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// A secret or account detail the site needs (API key, e-mail, …).
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Auth {
    /// Anonymous uploads.
    None,
    /// Works anonymously, but credentials unlock more.
    Optional,
    /// Every field in [`SiteInfo::credentials`] must be filled in.
    Required,
}

/// Static description of a site. All fields are `'static`, so a site's info can
/// live in a `const`.
#[derive(Clone, Copy, Debug)]
pub struct SiteInfo {
    pub name: &'static str,
    /// Other spellings accepted when matching names (presets, CLI, old configs).
    pub aliases: &'static [&'static str],
    pub homepage: &'static str,
    /// Fetched by the availability check.
    pub probe_url: &'static str,
    /// `None` = no known limit.
    pub max_size: Option<SizeLimit>,
    pub auth: Auth,
    pub credentials: &'static [CredentialField],
    pub api_docs: Option<&'static str>,
    /// Where the user gets the credentials from.
    pub credentials_url: Option<&'static str>,
    /// Shown next to the site; for things we know are shaky.
    pub known_issue: Option<&'static str>,
    /// Lower-case extensions with the leading dot (`".exe"`) the site refuses.
    pub blocked_extensions: &'static [&'static str],
}

impl SiteInfo {
    pub const fn new(name: &'static str, homepage: &'static str, probe_url: &'static str) -> Self {
        Self {
            name,
            aliases: &[],
            homepage,
            probe_url,
            max_size: None,
            auth: Auth::None,
            credentials: &[],
            api_docs: None,
            credentials_url: None,
            known_issue: None,
            blocked_extensions: &[],
        }
    }

    pub const fn max_size(mut self, limit: SizeLimit) -> Self {
        self.max_size = Some(limit);
        self
    }

    pub const fn aliases(mut self, aliases: &'static [&'static str]) -> Self {
        self.aliases = aliases;
        self
    }

    pub const fn requires(mut self, credentials: &'static [CredentialField]) -> Self {
        self.auth = Auth::Required;
        self.credentials = credentials;
        self
    }

    pub const fn api_docs(mut self, url: &'static str) -> Self {
        self.api_docs = Some(url);
        self
    }

    pub const fn credentials_url(mut self, url: &'static str) -> Self {
        self.credentials_url = Some(url);
        self
    }

    pub const fn known_issue(mut self, note: &'static str) -> Self {
        self.known_issue = Some(note);
        self
    }

    pub const fn blocked_extensions(mut self, extensions: &'static [&'static str]) -> Self {
        self.blocked_extensions = extensions;
        self
    }

    /// Stable identifier (the normalised name).
    pub fn id(&self) -> String {
        normalize_name(self.name)
    }

    /// Does `query` refer to this site, by name or alias?
    pub fn matches(&self, query: &str) -> bool {
        let query = normalize_name(query);
        !query.is_empty()
            && (normalize_name(self.name) == query
                || self
                    .aliases
                    .iter()
                    .any(|alias| normalize_name(alias) == query))
    }

    /// Cheap, offline checks every upload goes through before any request is made.
    pub fn precheck(&self, file: &LocalFile, credentials: &Credentials) -> Result<(), UploadError> {
        if file.size == 0 {
            return Err(UploadError::EmptyFile);
        }
        if let Some(limit) = self.max_size
            && file.size > limit.bytes()
        {
            return Err(UploadError::TooLarge {
                limit: limit.to_string(),
            });
        }
        let lower = file.name.to_lowercase();
        if let Some(blocked) = self
            .blocked_extensions
            .iter()
            .find(|extension| lower.ends_with(*extension))
        {
            return Err(UploadError::Rejected(format!(
                "`{blocked}` files are not allowed"
            )));
        }
        if self.auth == Auth::Required {
            for field in self.credentials {
                if credentials.get(field.key).is_none() {
                    return Err(UploadError::MissingCredential(field.key));
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
pub trait Site: Send + Sync {
    fn info(&self) -> &SiteInfo;

    /// Upload `cx.file` and return its public link.
    ///
    /// The engine has already run [`SiteInfo::precheck`] and takes care of
    /// cancellation (the future is simply dropped), so implementations only
    /// deal with the protocol.
    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError>;
}

/// Credentials for one site, keyed by [`CredentialField::key`].
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

/// All sites the program knows about.
#[derive(Clone)]
pub struct Registry {
    sites: Vec<Arc<dyn Site>>,
}

impl Registry {
    pub fn new(sites: Vec<Arc<dyn Site>>) -> Self {
        Self { sites }
    }

    /// Every site shipped with Mul-Tor.
    pub fn builtin() -> Self {
        Self::new(crate::sites::all())
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Site>> {
        self.sites.iter()
    }

    pub fn len(&self) -> usize {
        self.sites.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sites.is_empty()
    }

    pub fn find(&self, query: &str) -> Option<&Arc<dyn Site>> {
        self.sites.iter().find(|site| site.info().matches(query))
    }

    /// Looks every name up; returns the sites found (in the order given,
    /// without duplicates) and the names that matched nothing.
    pub fn resolve<'a, I>(&self, names: I) -> (Vec<Arc<dyn Site>>, Vec<String>)
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut found: Vec<Arc<dyn Site>> = Vec::new();
        let mut unknown = Vec::new();
        for name in names {
            match self.find(name) {
                Some(site) => {
                    if !found.iter().any(|f| Arc::ptr_eq(f, site)) {
                        found.push(site.clone());
                    }
                }
                None => unknown.push(name.to_string()),
            }
        }
        (found, unknown)
    }
}

impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.sites.iter().map(|s| s.info().name))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const KEYED: SiteInfo = SiteInfo::new("Example", "https://example.com", "https://example.com")
        .max_size(SizeLimit::mb(10))
        .requires(&[CredentialField::new("apiKey", "API key", true)])
        .blocked_extensions(&[".exe"]);

    fn file(name: &str, size: u64) -> LocalFile {
        LocalFile {
            path: PathBuf::from(name),
            name: name.to_string(),
            size,
        }
    }

    #[test]
    fn names_match_regardless_of_punctuation_and_case() {
        let info = SiteInfo::new("Transfer.sh", "", "").aliases(&["Transfer_sh"]);
        assert!(info.matches("transfer.sh"));
        assert!(info.matches("TRANSFER_SH"));
        assert!(info.matches("transfersh"));
        assert!(!info.matches("transfer"));
        assert!(!info.matches(""));
        assert!(!info.matches("..."));
    }

    #[test]
    fn precheck_rejects_empty_oversized_blocked_and_uncredentialled() {
        let mut creds = Credentials::default();
        creds.set("apiKey", "k");

        assert!(KEYED.precheck(&file("a.zip", 1024), &creds).is_ok());
        assert!(matches!(
            KEYED.precheck(&file("a.zip", 0), &creds),
            Err(UploadError::EmptyFile)
        ));
        assert!(matches!(
            KEYED.precheck(&file("a.zip", 11 * 1024 * 1024), &creds),
            Err(UploadError::TooLarge { .. })
        ));
        assert!(matches!(
            KEYED.precheck(&file("setup.EXE", 1), &creds),
            Err(UploadError::Rejected(_))
        ));
        assert!(matches!(
            KEYED.precheck(&file("a.zip", 1), &Credentials::default()),
            Err(UploadError::MissingCredential("apiKey"))
        ));
    }

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
