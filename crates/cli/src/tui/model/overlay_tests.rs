//! The summary shown before an upload starts.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use mul_tor_core::{Credentials, Registry};

use super::files::FileEntry;
use super::overlay::Preflight;
use super::providers::{Probe, ProviderRow};

fn rows(names: &[&str]) -> Vec<ProviderRow> {
    let registry = Registry::builtin();
    names
        .iter()
        .map(|name| ProviderRow::new(Arc::clone(registry.find(name).unwrap()), true))
        .collect()
}

fn file(name: &str, size: u64) -> FileEntry {
    FileEntry {
        path: PathBuf::from(name),
        size,
    }
}

#[test]
fn it_counts_files_bytes_and_uploads() {
    let rows = rows(&["gofile", "catbox"]);
    let refs: Vec<_> = rows.iter().collect();
    let report = Preflight::compute(&[file("a", 10), file("b", 20)], &refs, &BTreeMap::new());
    assert_eq!((report.files, report.bytes, report.uploads), (2, 30, 4));
    assert_eq!(report.providers, ["Gofile", "Catbox"], "in the order given");
    assert_eq!(report.rejected, 0);
}

#[test]
fn files_the_offline_checks_reject_are_listed() {
    let rows = rows(&["catbox"]);
    let refs: Vec<_> = rows.iter().collect();
    let files = [
        file("empty.bin", 0),
        file("fine.bin", 5),
        file("setup.exe", 5),
    ];
    let report = Preflight::compute(&files, &refs, &BTreeMap::new());
    assert!(report.rejected >= 1);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.contains("empty.bin")),
        "{:?}",
        report.issues
    );
    assert!(!report.issues.iter().any(|issue| issue.contains("fine.bin")));
}

#[test]
fn only_a_few_rejections_are_spelled_out() {
    let rows = rows(&["catbox"]);
    let refs: Vec<_> = rows.iter().collect();
    let files: Vec<_> = (0..12).map(|n| file(&format!("empty{n}"), 0)).collect();
    let report = Preflight::compute(&files, &refs, &BTreeMap::new());
    assert_eq!(report.rejected, 12);
    assert_eq!(report.issues.len(), 5);
    assert_eq!(report.unlisted(), 7);
}

#[test]
fn offline_providers_and_missing_credentials_are_flagged() {
    let mut rows = rows(&["catbox", "pixeldrain"]);
    rows[0].probe = Probe::Offline("timed out".into());
    let refs: Vec<_> = rows.iter().collect();
    let report = Preflight::compute(&[file("a", 1)], &refs, &BTreeMap::new());
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("did not answer") && w.contains("timed out"))
    );
    assert!(
        report.issues.iter().any(|i| i.contains("credential")),
        "{:?}",
        report.issues
    );

    let mut credentials = BTreeMap::new();
    let mut keyed = Credentials::default();
    keyed.set("apiKey", "k");
    credentials.insert("pixeldrain".to_string(), keyed);
    let report = Preflight::compute(&[file("a", 1)], &refs, &credentials);
    assert!(!report.issues.iter().any(|i| i.contains("credential")));
}
