//! Matching provider names loosely.

/// Lower-cases and drops everything that is not a letter or digit, so
/// `Transfer.sh`, `Transfer_sh` and `transfersh` all name the same provider.
pub fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn punctuation_and_case_do_not_matter() {
        assert_eq!(normalize_name("Transfer.sh"), "transfersh");
        assert_eq!(normalize_name("Transfer_sh"), "transfersh");
        assert_eq!(normalize_name(" Go File! "), "gofile");
        assert_eq!(normalize_name("..."), "");
    }
}
