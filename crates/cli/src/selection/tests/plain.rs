//! Requests without a preset.

use mul_tor_core::{Config, Credentials};

use super::*;

#[test]
fn nothing_requested_selects_nothing() {
    let app = test_app(Config::default());
    assert!(select(&app, &Request::default()).unwrap().is_none());
}
#[test]
fn named_providers_are_resolved_loosely_in_order_without_duplicates() {
    let app = test_app(Config::default());
    let wanted = strings(&["oshi", "Transfer_sh", "OSHI"]);
    let selection = select(
        &app,
        &Request {
            names: &wanted,
            ..Default::default()
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(names(&selection), ["Oshi", "Transfer.sh"]);
}
#[test]
fn unknown_names_are_an_error_that_lists_them() {
    let app = test_app(Config::default());
    let wanted = strings(&["oshi", "nope", "alsonope"]);
    let err = select(
        &app,
        &Request {
            names: &wanted,
            ..Default::default()
        },
    )
    .err()
    .unwrap()
    .to_string();
    assert!(err.contains("nope, alsonope"), "{err}");
}
#[test]
fn named_providers_keep_missing_credentials_so_the_failure_is_explicit() {
    let app = test_app(Config::default());
    let wanted = strings(&["pixeldrain"]);
    let selection = select(
        &app,
        &Request {
            names: &wanted,
            ..Default::default()
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(names(&selection), ["Pixeldrain"]);
}
#[test]
fn configured_credentials_make_a_keyed_provider_usable() {
    let mut config = Config::default();
    let mut creds = Credentials::default();
    creds.set("apiKey", "k");
    config.credentials.insert("Pixeldrain".into(), creds);
    let app = test_app(config);
    let selection = select(
        &app,
        &Request {
            preset: Some("f95"),
            ..Default::default()
        },
    )
    .unwrap()
    .unwrap();
    assert!(names(&selection).contains(&"Pixeldrain"));
}
#[test]
fn all_excludes_blacklisted_and_credentialless_providers() {
    let config = Config {
        blacklist: strings(&["oshi"]),
        ..Config::default()
    };
    let app = test_app(config);
    let selection = select(
        &app,
        &Request {
            all: true,
            ..Default::default()
        },
    )
    .unwrap()
    .unwrap();
    let picked = names(&selection);
    assert!(!picked.contains(&"Oshi"));
    assert!(!picked.contains(&"BowFile"));
    assert!(picked.contains(&"Gofile"));
    assert_eq!(picked.len(), 37 - 1 - 5);
}
#[test]
fn a_named_blacklisted_provider_is_skipped_with_a_note() {
    let config = Config {
        blacklist: strings(&["oshi"]),
        ..Config::default()
    };
    let app = test_app(config);
    let wanted = strings(&["oshi", "gofile"]);
    let selection = select(
        &app,
        &Request {
            names: &wanted,
            ..Default::default()
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(names(&selection), ["Gofile"]);
    assert!(selection.notes.iter().any(|n| n.contains("blacklist")));
}
