//! What is known about whether a provider works today.
//!
//! Providers are old and file hosts come and go, so each one carries a
//! recorded state. It is static knowledge written down by people who saw the
//! provider work or fail, not a live probe (see [`crate::availability`]).
//! Nothing is marked [`Working`](Reliability::Working) without having been seen
//! working.

use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Reliability {
    /// Nobody has run it against the real service yet. Every provider
    /// starts here.
    #[default]
    Unverified,
    /// Seen working, on this date (`YYYY-MM-DD`).
    Working { checked: &'static str },
    /// Works, with a known problem worth knowing about.
    Degraded(&'static str),
    /// Known not to work. Left out of `--all` and presets; still selectable
    /// by name, in case it was fixed since.
    Broken(&'static str),
}

impl Reliability {
    /// One lower-case word, for tables and machine-readable output.
    pub fn label(&self) -> &'static str {
        match self {
            Reliability::Unverified => "unverified",
            Reliability::Working { .. } => "working",
            Reliability::Degraded(_) => "degraded",
            Reliability::Broken(_) => "broken",
        }
    }

    /// The reason or date that goes with the state.
    pub fn note(&self) -> Option<&'static str> {
        match self {
            Reliability::Unverified => None,
            Reliability::Working { checked } => Some(checked),
            Reliability::Degraded(reason) | Reliability::Broken(reason) => Some(reason),
        }
    }

    pub fn is_broken(&self) -> bool {
        matches!(self, Reliability::Broken(_))
    }

    /// Should a bulk choice (`--all`, a preset, "select all") include it?
    pub fn included_in_bulk_choices(&self) -> bool {
        !self.is_broken()
    }
}

impl fmt::Display for Reliability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reliability::Working { checked } => write!(f, "working (checked {checked})"),
            Reliability::Degraded(reason) => write!(f, "degraded: {reason}"),
            Reliability::Broken(reason) => write!(f, "broken: {reason}"),
            Reliability::Unverified => f.write_str("unverified"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_starts_unverified() {
        assert_eq!(Reliability::default(), Reliability::Unverified);
        assert_eq!(Reliability::default().label(), "unverified");
        assert_eq!(Reliability::default().note(), None);
    }

    #[test]
    fn only_broken_providers_are_left_out_of_bulk_choices() {
        assert!(Reliability::Unverified.included_in_bulk_choices());
        assert!(
            Reliability::Working {
                checked: "2026-10-24"
            }
            .included_in_bulk_choices()
        );
        assert!(Reliability::Degraded("slow").included_in_bulk_choices());
        assert!(!Reliability::Broken("shut down").included_in_bulk_choices());
    }

    #[test]
    fn the_text_form_says_why() {
        assert_eq!(
            Reliability::Broken("shut down").to_string(),
            "broken: shut down"
        );
        assert_eq!(Reliability::Degraded("slow").note(), Some("slow"));
        assert_eq!(
            Reliability::Working {
                checked: "2026-10-24"
            }
            .to_string(),
            "working (checked 2026-10-24)"
        );
    }
}
