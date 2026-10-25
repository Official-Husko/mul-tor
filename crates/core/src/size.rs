//! File-size limits and byte formatting.
//!
//! Limits use binary multiples (1 GB = 1024³ bytes). That is what the Python
//! version enforced, so the numbers carried over from its provider table keep their
//! meaning.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SizeUnit {
    Mb,
    Gb,
    Tb,
}

impl SizeUnit {
    const fn multiplier(self) -> u64 {
        match self {
            SizeUnit::Mb => 1024 * 1024,
            SizeUnit::Gb => 1024 * 1024 * 1024,
            SizeUnit::Tb => 1024 * 1024 * 1024 * 1024,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            SizeUnit::Mb => "MB",
            SizeUnit::Gb => "GB",
            SizeUnit::Tb => "TB",
        }
    }
}

/// The largest file a provider accepts, kept in the unit the provider advertises so it
/// can be shown to the user unchanged ("20 GB").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SizeLimit {
    value: u64,
    unit: SizeUnit,
}

impl SizeLimit {
    pub const fn mb(value: u64) -> Self {
        Self {
            value,
            unit: SizeUnit::Mb,
        }
    }

    pub const fn gb(value: u64) -> Self {
        Self {
            value,
            unit: SizeUnit::Gb,
        }
    }

    pub const fn tb(value: u64) -> Self {
        Self {
            value,
            unit: SizeUnit::Tb,
        }
    }

    pub const fn bytes(self) -> u64 {
        self.value * self.unit.multiplier()
    }
}

impl fmt::Display for SizeLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.value, self.unit.label())
    }
}

/// Human-readable byte count using IEC units ("1.50 GiB").
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_use_binary_multiples_like_the_python_tool() {
        assert_eq!(SizeLimit::mb(200).bytes(), 200 * 1024 * 1024);
        assert_eq!(SizeLimit::gb(20).bytes(), 20 * 1024 * 1024 * 1024);
        assert_eq!(SizeLimit::tb(1).bytes(), 1024u64.pow(4));
    }

    #[test]
    fn limits_display_in_their_original_unit() {
        assert_eq!(SizeLimit::gb(20).to_string(), "20 GB");
        assert_eq!(SizeLimit::mb(500).to_string(), "500 MB");
    }

    #[test]
    fn byte_formatting() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.00 KiB");
        assert_eq!(format_bytes(1536 * 1024 * 1024), "1.50 GiB");
    }
}
