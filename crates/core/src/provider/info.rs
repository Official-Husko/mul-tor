//! Static description of a provider.

use super::credentials::CredentialField;
use super::name::normalize_name;
use crate::size::SizeLimit;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Auth {
    /// Anonymous uploads.
    None,
    /// Works anonymously, but credentials unlock more.
    Optional,
    /// Every field in [`ProviderInfo::credentials`] must be filled in.
    Required,
}

/// Static description of a provider. All fields are `'static`, so a provider's info can
/// live in a `const`.
#[derive(Clone, Copy, Debug)]
pub struct ProviderInfo {
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
    /// Shown next to the provider; for things we know are shaky.
    pub known_issue: Option<&'static str>,
    /// Lower-case extensions with the leading dot (`".exe"`) the provider refuses.
    pub blocked_extensions: &'static [&'static str],
}

impl ProviderInfo {
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

    /// Does `query` refer to this provider, by name or alias?
    pub fn matches(&self, query: &str) -> bool {
        let query = normalize_name(query);
        !query.is_empty()
            && (normalize_name(self.name) == query
                || self
                    .aliases
                    .iter()
                    .any(|alias| normalize_name(alias) == query))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match_regardless_of_punctuation_and_case() {
        let info = ProviderInfo::new("Transfer.sh", "", "").aliases(&["Transfer_sh"]);
        assert!(info.matches("transfer.sh"));
        assert!(info.matches("TRANSFER_SH"));
        assert!(info.matches("transfersh"));
        assert!(!info.matches("transfer"));
        assert!(!info.matches(""));
        assert!(!info.matches("..."));
    }

    #[test]
    fn the_id_is_the_normalised_name() {
        assert_eq!(ProviderInfo::new("Go File", "", "").id(), "gofile");
    }
}
