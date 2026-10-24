//! The other commands, and what happens without a terminal.

use crate::common::*;

#[test]
fn the_python_config_in_the_working_directory_is_imported_with_secrets_masked() {
    let env = Env::new();
    std::fs::remove_file(env.home().join("config.toml")).unwrap();
    std::fs::write(
        env.cwd().join("config.json"),
        r#"{"checkForUpdates": false, "api_keys": {"Pixeldrain": {"apiKey": "super-secret-key"}}}"#,
    )
    .unwrap();

    let shown = env.run(None, &["config", "show"]);

    assert_eq!(code(&shown), 0, "{}", stderr(&shown));
    assert!(
        stderr(&shown).contains("Imported your old config.json"),
        "{}",
        stderr(&shown)
    );
    let out = stdout(&shown);
    assert!(
        out.contains("[credentials.Pixeldrain]") && out.contains("********"),
        "{out}"
    );
    assert!(!out.contains("super-secret-key"));
    assert!(
        std::fs::read_to_string(env.home().join("config.toml"))
            .unwrap()
            .contains("super-secret-key")
    );
    assert!(
        env.cwd().join("config.json").exists(),
        "the old file is left alone"
    );
}

#[test]
fn interactive_mode_refuses_to_run_without_a_terminal() {
    let env = Env::new();
    let output = env.run(None, &[]);
    assert_eq!(code(&output), 2);
    assert!(
        stderr(&output).contains("needs a terminal"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn informational_commands_work_offline() {
    let env = Env::new();
    for command in [
        &["providers"][..],
        &["providers", "--format", "markdown"],
        &["providers", "--format", "json"],
        &["presets"],
        &["config", "path"],
    ] {
        let output = env.run(None, command);
        assert_eq!(code(&output), 0, "{command:?}: {}", stderr(&output));
        assert!(!stdout(&output).is_empty(), "{command:?}");
    }
    let providers = env.run(None, &["providers", "--format", "json"]);
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&providers)).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 37);
}
