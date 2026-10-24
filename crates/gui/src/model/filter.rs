//! The provider search box.

use mul_tor_core::Provider;

/// Does `provider` match the user's search text (name or alias)?
pub fn matches_filter(provider: &dyn Provider, filter: &str) -> bool {
    let filter = filter.trim().to_lowercase();
    if filter.is_empty() {
        return true;
    }
    let info = provider.info();
    info.name.to_lowercase().contains(&filter)
        || info
            .aliases
            .iter()
            .any(|alias| alias.to_lowercase().contains(&filter))
}
