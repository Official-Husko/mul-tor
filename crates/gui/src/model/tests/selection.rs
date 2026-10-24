//! Files, providers and the run state.

use mul_tor_core::Registry;

use crate::model::{FileEntry, Model, RunState, matches_filter};

#[test]
fn run_state_transitions() {
    let mut model = Model::default();
    model.add_paths(&[]);
    model.files.push(FileEntry {
        path: "f".into(),
        name: "f".into(),
        size: 1,
    });
    model.toggle_provider("catbox", true);
    assert!(model.can_start());

    model.begin_run(None);
    assert!(model.is_running() && !model.can_start());
    model.request_cancel();
    assert_eq!(model.run, RunState::Cancelling);
    model.run_finished();
    assert_eq!(model.run, RunState::Idle);
    assert!(model.can_start());

    // Cancelling when nothing runs does nothing.
    model.request_cancel();
    assert_eq!(model.run, RunState::Idle);
}

#[test]
fn starting_needs_files_and_providers() {
    let mut model = Model::default();
    assert!(!model.can_start());
    model.toggle_provider("catbox", true);
    assert!(!model.can_start());
    model.files.push(FileEntry {
        path: "f".into(),
        name: "f".into(),
        size: 1,
    });
    assert!(model.can_start());
    model.toggle_provider("catbox", false);
    assert!(!model.can_start());
}

#[test]
fn adding_paths_dedupes_and_expands_folders() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("a.txt"), "aaa").unwrap();
    std::fs::write(tmp.path().join("b.txt"), "bb").unwrap();
    let mut model = Model::default();

    let problems = model.add_paths(&[
        tmp.path().to_path_buf(),
        tmp.path().join("a.txt"),
        tmp.path().join("gone"),
    ]);
    assert_eq!(problems.len(), 1);
    let names: Vec<_> = model.files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["a.txt", "b.txt"]);
    assert_eq!(model.total_size(), 5);

    model.remove_file(0);
    assert_eq!(model.files.len(), 1);
    model.remove_file(99); // out of range is a no-op
}

#[test]
fn selecting_a_preset_replaces_the_selection() {
    let mut model = Model::default();
    model.toggle_provider("oshi", true);
    model.select_only(["catbox", "keep"]);
    assert_eq!(
        model
            .selected
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["catbox", "keep"]
    );
}

#[test]
fn provider_filter_matches_names_and_aliases_case_insensitively() {
    let registry = Registry::builtin();
    let userscloud = registry.find("usercloud").unwrap();
    assert!(matches_filter(&**userscloud, ""));
    assert!(matches_filter(&**userscloud, "CLOUD"));
    assert!(matches_filter(&**userscloud, "userscl"));
    assert!(!matches_filter(&**userscloud, "oshi"));
}
