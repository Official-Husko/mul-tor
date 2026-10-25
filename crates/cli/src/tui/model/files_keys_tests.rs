//! Keys and pasted text while choosing files.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::files::{FilesState, Pane};
use super::files_keys::{FilesOutcome, FilesResult};

fn press(state: &mut FilesState, code: KeyCode) -> FilesResult {
    state.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn state() -> (tempfile::TempDir, FilesState) {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("a.txt"), "aaa").unwrap();
    std::fs::write(tmp.path().join("b.txt"), "bb").unwrap();
    std::fs::create_dir(tmp.path().join("sub")).unwrap();
    let state = FilesState::new(tmp.path().to_path_buf());
    (tmp, state)
}

#[test]
fn enter_opens_folders_and_adds_files() {
    let (tmp, mut state) = state();
    press(&mut state, KeyCode::Enter); // "sub"
    assert_eq!(state.dir, tmp.path().join("sub"));
    press(&mut state, KeyCode::Left);
    assert_eq!(state.dir, tmp.path());
    press(&mut state, KeyCode::Down);
    let result = press(&mut state, KeyCode::Enter);
    assert_eq!(state.basket.len(), 1);
    assert!(result.notice.unwrap().1.starts_with("Added 1 file"));
}

#[test]
fn space_toggles_a_file_in_and_out() {
    let (_tmp, mut state) = state();
    press(&mut state, KeyCode::Down);
    press(&mut state, KeyCode::Char(' '));
    assert_eq!(state.basket.len(), 1);
    press(&mut state, KeyCode::Char(' '));
    assert!(state.basket.is_empty());
}

#[test]
fn next_needs_a_file() {
    let (_tmp, mut state) = state();
    let result = press(&mut state, KeyCode::Char('n'));
    assert!(matches!(result.outcome, FilesOutcome::Stay));
    assert!(result.notice.unwrap().1.contains("at least one file"));
    press(&mut state, KeyCode::Char('a'));
    assert!(matches!(
        press(&mut state, KeyCode::Char('n')).outcome,
        FilesOutcome::Next
    ));
}

#[test]
fn pasted_paths_are_added_unless_a_path_is_being_typed() {
    let (tmp, mut state) = state();
    let dropped = format!(
        "\"{}\" {}",
        tmp.path().join("a.txt").display(),
        tmp.path().join("b.txt").display()
    );
    assert!(
        state
            .paste(&dropped)
            .unwrap()
            .1
            .starts_with("Added 2 files")
    );

    press(&mut state, KeyCode::Char('/'));
    assert!(state.typing());
    assert!(state.paste("typed text").is_none());
    assert_eq!(state.path_input.text(), "typed text");
}

#[test]
fn typing_a_folder_path_opens_it() {
    let (tmp, mut state) = state();
    press(&mut state, KeyCode::Char('/'));
    state
        .path_input
        .insert_str(&tmp.path().join("sub").to_string_lossy());
    press(&mut state, KeyCode::Enter);
    assert_eq!(state.dir, tmp.path().join("sub"));
    assert!(!state.typing());
}

#[test]
fn the_basket_pane_removes_and_clears() {
    let (_tmp, mut state) = state();
    press(&mut state, KeyCode::Char('a'));
    press(&mut state, KeyCode::Tab);
    assert_eq!(state.pane, Pane::Basket);
    press(&mut state, KeyCode::Char('x'));
    assert_eq!(state.basket.len(), 1);
    press(&mut state, KeyCode::Char('c'));
    assert!(state.basket.is_empty());
    assert_eq!(state.pane, Pane::Browser);
}

#[test]
fn quitting_is_requested_not_done() {
    let (_tmp, mut state) = state();
    assert!(matches!(
        press(&mut state, KeyCode::Esc).outcome,
        FilesOutcome::Quit
    ));
}
