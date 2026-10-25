//! Presets, including the one the config asks for.

use mul_tor_core::Config;

use super::*;

#[test]
fn presets_skip_providers_that_lack_credentials_and_say_so() {
    let app = test_app(Config::default());
    let selection = select(
        &app,
        &Request {
            preset: Some("f95"),
            ..Default::default()
        },
    )
    .unwrap()
    .unwrap();
    let picked = names(&selection);
    assert!(picked.contains(&"Gofile") && picked.contains(&"Catbox"));
    assert!(!picked.contains(&"Pixeldrain") && !picked.contains(&"Mixdrop"));
    assert!(
        selection
            .notes
            .iter()
            .any(|n| n.contains("Pixeldrain") && n.contains("Mixdrop"))
    );
    assert_eq!(
        selection.link_format.as_deref(),
        Some("[URL='{file_url}']{site_name}[/URL]")
    );
}
#[test]
fn auto_load_preset_applies_only_when_nothing_else_was_requested() {
    let mut config = Config::default();
    config.preset.auto_load = true;
    config.preset.name = "f95.json".into();
    let app = test_app(config);

    let auto = select(&app, &Request::default()).unwrap().unwrap();
    assert!(names(&auto).contains(&"Catbox"));

    let wanted = strings(&["oshi"]);
    let explicit = select(
        &app,
        &Request {
            names: &wanted,
            ..Default::default()
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(names(&explicit), ["Oshi"]);
}
#[test]
fn unknown_presets_are_an_error() {
    let app = test_app(Config::default());
    let err = select(
        &app,
        &Request {
            preset: Some("nope"),
            ..Default::default()
        },
    )
    .err()
    .unwrap()
    .to_string();
    assert!(err.contains("no preset called `nope`"), "{err}");
}
#[test]
fn preset_and_named_providers_combine() {
    let app = test_app(Config::default());
    let wanted = strings(&["keep"]);
    let selection = select(
        &app,
        &Request {
            names: &wanted,
            preset: Some("f95"),
            all: false,
        },
    )
    .unwrap()
    .unwrap();
    assert!(names(&selection).contains(&"Keep") && names(&selection).contains(&"Gofile"));
}
