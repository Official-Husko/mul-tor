//! One finished upload, as a line of text.

use chrono::{DateTime, Local};

use super::template::{format_template, template_fields};

/// One finished upload.
#[derive(Clone, Debug)]
pub struct LinkEntry<'a> {
    pub when: DateTime<Local>,
    pub provider: &'a str,
    pub file_name: &'a str,
    pub url: &'a str,
}

impl LinkEntry<'_> {
    /// `2024-05-01 12:00:00.000000 | Pixeldrain | file.zip - https://…`
    pub fn plain_line(&self) -> String {
        format!(
            "{} | {} | {} - {}",
            self.when.format("%Y-%m-%d %H:%M:%S%.6f"),
            self.provider,
            self.file_name,
            self.url
        )
    }

    /// Apply a preset's link template. Available fields: `status`,
    /// `file_name`, `file_url`, `provider_name`, `date_and_time`.
    /// `site_name` is the older spelling of `provider_name` and still works,
    /// because shared preset files use it.
    pub fn formatted(&self, template: &str) -> String {
        let when = self.when.format("%Y-%m-%d %H:%M:%S%.6f").to_string();
        format_template(
            template,
            &template_fields(self.provider, self.file_name, self.url, &when),
        )
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn the_legacy_site_name_placeholder_still_works() {
        let entry = LinkEntry {
            when: Local.with_ymd_and_hms(2024, 1, 2, 3, 4, 5).unwrap(),
            provider: "Pixeldrain",
            file_name: "a.7z",
            url: "https://x/y",
        };
        assert_eq!(
            entry.formatted("{site_name}|{provider_name}"),
            "Pixeldrain|Pixeldrain"
        );
    }

    fn entry() -> LinkEntry<'static> {
        LinkEntry {
            when: Local.with_ymd_and_hms(2024, 5, 1, 12, 30, 45).unwrap(),
            provider: "Pixeldrain",
            file_name: "a.zip",
            url: "https://pixeldrain.com/u/1",
        }
    }

    #[test]
    fn plain_line_matches_the_python_layout() {
        assert_eq!(
            entry().plain_line(),
            "2024-05-01 12:30:45.000000 | Pixeldrain | a.zip - https://pixeldrain.com/u/1"
        );
    }
}
