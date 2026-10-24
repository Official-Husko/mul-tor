//! The settings window.

use egui_kittest::kittest::Queryable;

use super::*;

#[test]
fn credentials_entered_in_settings_are_saved_and_unlock_the_provider() {
    let home = quiet_home();
    let mut harness = harness(home.path());

    harness.get_by_label("Settings").click();
    harness.run();
    harness.get_by_label("Accounts").click();
    harness.run();

    // Type into a field through the real widget (BowFile's is the first on screen)...
    let field = harness.get_by_label("API key 1");
    field.focus();
    field.type_text("bow-key-1");
    harness.run();
    assert_eq!(
        harness.state().settings.buffers[&("bowfile".to_string(), "apiKey1".to_string())],
        "bow-key-1"
    );
    // ...and fill Pixeldrain's directly: it is scrolled out of view, so it is
    // not in the accessibility tree.
    harness.state_mut().settings.buffers.insert(
        ("pixeldrain".into(), "apiKey".into()),
        "my-pixeldrain-key".into(),
    );

    harness.get_by_label("Save").click();
    harness.run();

    assert!(!harness.state().settings.open);
    assert!(!is_disabled(harness.get_by_label("Pixeldrain")));
    assert!(
        is_disabled(harness.get_by_label("BowFile")),
        "BowFile needs both of its keys"
    );
    let saved = std::fs::read_to_string(home.path().join("config.toml")).unwrap();
    assert!(
        saved.contains("[credentials.Pixeldrain]") && saved.contains("my-pixeldrain-key"),
        "{saved}"
    );
    assert!(saved.contains("apiKey1 = \"bow-key-1\""), "{saved}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(home.path().join("config.toml"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0);
    }
}

#[test]
fn cancelling_the_settings_window_discards_edits() {
    let home = quiet_home();
    let mut harness = harness(home.path());
    harness.get_by_label("Settings").click();
    harness.run();
    harness.state_mut().settings.draft.parallel_uploads = 9;
    harness.get_by_label("Cancel").click();
    harness.run_steps(3);
    assert!(!harness.state().settings.open);
    assert_eq!(harness.state().config.parallel_uploads, 1);
}
