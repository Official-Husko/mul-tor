//! Recording finished uploads, and the `{placeholder}` link templates presets use.

use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};

/// Substitute `{name}` placeholders from `values`, like Python's `str.format`
/// restricted to named fields: `{{` and `}}` are literal braces, and a
/// placeholder that is unknown (or unterminated) is left as written.
pub fn format_template(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(pos) = rest.find(['{', '}']) {
        out.push_str(&rest[..pos]);
        let tail = &rest[pos..];
        if let Some(after) = tail.strip_prefix("{{") {
            out.push('{');
            rest = after;
        } else if let Some(after) = tail.strip_prefix("}}") {
            out.push('}');
            rest = after;
        } else if let Some(inner) = tail.strip_prefix('{') {
            match inner.find('}') {
                Some(end) => {
                    let key = &inner[..end];
                    match values.iter().find(|(name, _)| *name == key) {
                        Some((_, value)) => out.push_str(value),
                        None => out.push_str(&tail[..end + 2]),
                    }
                    rest = &inner[end + 1..];
                }
                None => {
                    out.push('{');
                    rest = inner;
                }
            }
        } else {
            // A lone `}` is kept as written.
            out.push('}');
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    out
}

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
            &[
                ("status", "ok"),
                ("file_name", self.file_name),
                ("file_url", self.url),
                ("provider_name", self.provider),
                ("site_name", self.provider),
                ("date_and_time", &when),
            ],
        )
    }
}

/// Appends finished uploads to `file_links.txt` (and, when a preset supplies
/// a template, `file_links_formatted.txt`) in one directory.
#[derive(Clone, Debug)]
pub struct LinkLog {
    dir: PathBuf,
    template: Option<String>,
}

impl LinkLog {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            template: None,
        }
    }

    pub fn with_template(mut self, template: Option<String>) -> Self {
        self.template = template.filter(|t| !t.is_empty());
        self
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn record(&self, entry: &LinkEntry<'_>) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        append_line(&self.dir.join("file_links.txt"), &entry.plain_line())?;
        if let Some(template) = &self.template {
            append_line(
                &self.dir.join("file_links_formatted.txt"),
                &entry.formatted(template),
            )?;
        }
        Ok(())
    }
}

fn append_line(path: &Path, line: &str) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn placeholders_are_substituted() {
        let out = format_template(
            "[URL='{file_url}']{provider_name}[/URL]",
            &[("file_url", "https://x/y"), ("provider_name", "Pixeldrain")],
        );
        assert_eq!(out, "[URL='https://x/y']Pixeldrain[/URL]");
    }

    #[test]
    fn the_legacy_site_name_placeholder_still_works() {
        let entry = LinkEntry {
            when: chrono::Local.with_ymd_and_hms(2024, 1, 2, 3, 4, 5).unwrap(),
            provider: "Pixeldrain",
            file_name: "a.7z",
            url: "https://x/y",
        };
        assert_eq!(
            entry.formatted("{site_name}|{provider_name}"),
            "Pixeldrain|Pixeldrain"
        );
    }

    #[test]
    fn doubled_braces_are_literal() {
        assert_eq!(format_template("{{{a}}}", &[("a", "1")]), "{1}");
        assert_eq!(format_template("{{a}}", &[("a", "1")]), "{a}");
    }

    #[test]
    fn unknown_and_unterminated_placeholders_are_kept() {
        assert_eq!(format_template("{nope} {a", &[("a", "1")]), "{nope} {a");
        assert_eq!(format_template("}", &[]), "}");
    }

    #[test]
    fn non_ascii_text_survives() {
        assert_eq!(format_template("日本 {a} é", &[("a", "ü")]), "日本 ü é");
        // Multi-byte characters inside an unknown placeholder must not eat what follows.
        assert_eq!(format_template("{é}x{a}", &[("a", "1")]), "{é}x1");
        assert_eq!(format_template("{日本}", &[("日本", "ok")]), "ok");
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

    #[test]
    fn log_appends_plain_and_formatted_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let log = LinkLog::new(tmp.path().join("out"))
            .with_template(Some("[URL='{file_url}']{provider_name}[/URL]".into()));
        log.record(&entry()).unwrap();
        log.record(&entry()).unwrap();

        let plain = std::fs::read_to_string(log.dir().join("file_links.txt")).unwrap();
        assert_eq!(plain.lines().count(), 2);
        let formatted =
            std::fs::read_to_string(log.dir().join("file_links_formatted.txt")).unwrap();
        assert_eq!(
            formatted.lines().next().unwrap(),
            "[URL='https://pixeldrain.com/u/1']Pixeldrain[/URL]"
        );
    }

    #[test]
    fn no_template_means_no_formatted_file() {
        let tmp = tempfile::tempdir().unwrap();
        LinkLog::new(tmp.path()).record(&entry()).unwrap();
        assert!(!tmp.path().join("file_links_formatted.txt").exists());
    }
}
