//! Pop-ups: help, the preset list, the "really upload?" summary, quitting.

use std::collections::BTreeMap;

use mul_tor_core::{Credentials, LocalFile};
use ratatui::widgets::ListState;

use super::files::FileEntry;
use super::providers::{Probe, ProviderRow};

/// How many individual problems the summary spells out.
const MAX_LISTED: usize = 5;

pub enum Overlay {
    Help,
    /// 0 is "no preset"; entry `n` is `presets[n - 1]`.
    Presets(ListState),
    Confirm(Box<Preflight>),
    Quit,
}

/// What is about to happen, and what is already known to go wrong.
#[derive(Clone, Debug, Default)]
pub struct Preflight {
    pub files: usize,
    pub bytes: u64,
    pub providers: Vec<&'static str>,
    /// `files x providers`.
    pub uploads: usize,
    /// Uploads the offline checks already reject (too large, empty, …).
    pub rejected: usize,
    /// The first few rejections, spelled out.
    pub issues: Vec<String>,
    /// Things worth a second thought: an offline or broken provider.
    pub warnings: Vec<String>,
}

impl Preflight {
    pub fn compute(
        files: &[FileEntry],
        rows: &[&ProviderRow],
        credentials: &BTreeMap<String, Credentials>,
    ) -> Self {
        let mut report = Self {
            files: files.len(),
            bytes: files.iter().map(|file| file.size).sum(),
            providers: rows.iter().map(|row| row.provider.info().name).collect(),
            uploads: files.len() * rows.len(),
            ..Self::default()
        };

        for row in rows {
            let info = row.provider.info();
            if let Probe::Offline(reason) = &row.probe {
                report.warnings.push(format!(
                    "{} did not answer the last check ({reason})",
                    info.name
                ));
            }
            if info.reliability.is_broken() {
                report
                    .warnings
                    .push(format!("{} is recorded as {}", info.name, info.reliability));
            }
            let own = credentials.get(&row.id).cloned().unwrap_or_default();
            for entry in files {
                let file = LocalFile {
                    path: entry.path.clone(),
                    name: entry
                        .path
                        .file_name()
                        .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
                    size: entry.size,
                };
                if let Err(error) = info.precheck(&file, &own) {
                    report.rejected += 1;
                    if report.issues.len() < MAX_LISTED {
                        report
                            .issues
                            .push(format!("{}: {} - {error}", info.name, file.name));
                    }
                }
            }
        }
        report
    }

    /// Rejections beyond the ones listed.
    pub fn unlisted(&self) -> usize {
        self.rejected.saturating_sub(self.issues.len())
    }
}
