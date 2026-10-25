//! The files finished uploads are appended to.

use std::io::Write;
use std::path::{Path, PathBuf};

use super::entry::LinkEntry;

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
    use chrono::{Local, TimeZone};

    use super::*;

    fn entry() -> LinkEntry<'static> {
        LinkEntry {
            when: Local.with_ymd_and_hms(2024, 5, 1, 12, 30, 45).unwrap(),
            provider: "Pixeldrain",
            file_name: "a.zip",
            url: "https://pixeldrain.com/u/1",
        }
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
