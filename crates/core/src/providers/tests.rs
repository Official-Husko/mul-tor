//! Checks on the registry as a whole: names, ordering and metadata.

use std::collections::HashSet;

use super::*;
use crate::provider::{Auth, normalize_name};

#[test]
fn every_legacy_provider_is_registered() {
    // The 37 providers of the Python version (`modules/provider_data.py` at tag 1.5.2).
    let legacy = [
        "Gofile",
        "Pixeldrain",
        "FileBin",
        "Delafil",
        "Files.dp.ua",
        "FilesFm",
        "Krakenfiles",
        "Transfer_sh",
        "TmpFiles",
        "Mixdrop",
        "1Fichier",
        "Fileio",
        "EasyUpload",
        "AnonTransfer",
        "1CloudFile",
        "Anonymfile",
        "FileSi",
        "ClicknUpload",
        "FileUpload",
        "BowFile",
        "HexUpload",
        "UserCloud",
        "DooDrive",
        "uFile",
        "Download_gg",
        "Catbox",
        "LitterBox",
        "Keep",
        "TempSend",
        "UsersDrive",
        "Rapidgator",
        "WDHO",
        "Filesadmin",
        "Fastupload",
        "CyberFile",
        "Buzzheavier",
        "Oshi",
    ];
    assert_eq!(legacy.len(), 37);
    let registry = crate::Registry::builtin();
    for name in legacy {
        assert!(registry.find(name).is_some(), "{name} is not registered");
    }
    assert!(registry.len() >= legacy.len());
}

#[test]
fn names_and_aliases_are_unique() {
    let mut seen = HashSet::new();
    for provider in all() {
        let info = provider.info();
        for name in std::iter::once(info.name).chain(info.aliases.iter().copied()) {
            assert!(
                seen.insert(normalize_name(name)),
                "`{name}` is used by more than one provider"
            );
        }
    }
}

#[test]
fn registry_is_sorted() {
    let names: Vec<String> = all().iter().map(|s| s.info().name.to_lowercase()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
}

#[test]
fn provider_metadata_is_consistent() {
    for provider in all() {
        let info = provider.info();
        assert!(info.homepage.starts_with("https://"), "{}", info.name);
        assert!(info.probe_url.starts_with("https://"), "{}", info.name);
        match info.auth {
            Auth::None => assert!(info.credentials.is_empty(), "{}", info.name),
            _ => assert!(!info.credentials.is_empty(), "{}", info.name),
        }
        for ext in info
            .blocked_extensions
            .iter()
            .chain(info.allowed_extensions)
        {
            assert!(
                ext.starts_with('.') && *ext == ext.to_lowercase(),
                "{}",
                info.name
            );
        }
    }
}

#[test]
fn exactly_the_legacy_credential_providers_need_credentials() {
    let mut keyed: Vec<_> = all()
        .iter()
        .filter(|s| s.info().auth == Auth::Required)
        .map(|s| s.info().name)
        .collect();
    keyed.sort();
    assert_eq!(
        keyed,
        ["BowFile", "DooDrive", "Mixdrop", "Pixeldrain", "Rapidgator"]
    );
}
