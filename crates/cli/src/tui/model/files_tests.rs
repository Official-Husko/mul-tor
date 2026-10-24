//! Choosing files: the basket and the browser.

use super::files::{AddReport, FilesState};
use crate::tui::msg::Level;

fn folder() -> (tempfile::TempDir, FilesState) {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("a.txt"), "aaa").unwrap();
    std::fs::write(tmp.path().join("b.txt"), "bb").unwrap();
    std::fs::create_dir(tmp.path().join("sub")).unwrap();
    std::fs::write(tmp.path().join("sub").join("c.txt"), "c").unwrap();
    let state = FilesState::new(tmp.path().to_path_buf());
    (tmp, state)
}

#[test]
fn the_cursor_starts_on_the_first_real_entry() {
    let (_tmp, state) = folder();
    assert_eq!(state.current_item().unwrap().name, "sub");
}

#[test]
fn adding_a_folder_adds_the_files_directly_in_it() {
    let (tmp, mut state) = folder();
    let report = state.add_paths(&[tmp.path().to_path_buf()]);
    assert_eq!((report.added, report.duplicates), (2, 0));
    assert_eq!(state.total_size(), 5);
    state.recursive = true;
    let report = state.add_paths(&[tmp.path().to_path_buf()]);
    assert_eq!(
        (report.added, report.duplicates),
        (1, 2),
        "only sub/c.txt is new"
    );
}

#[test]
fn the_same_file_is_never_listed_twice() {
    let (tmp, mut state) = folder();
    let a = tmp.path().join("a.txt");
    state.add_paths(std::slice::from_ref(&a));
    let dotted = tmp.path().join("sub").join("..").join("a.txt");
    let report = state.add_paths(&[dotted]);
    assert_eq!((report.added, report.duplicates), (0, 1));
    assert!(state.contains(&a));
}

#[test]
fn removing_and_clearing_forget_the_files() {
    let (tmp, mut state) = folder();
    state.add_paths(&[tmp.path().to_path_buf()]);
    state.remove_current();
    assert_eq!(state.basket.len(), 1);
    assert!(!state.contains(&tmp.path().join("a.txt")));
    state.clear();
    assert!(state.basket.is_empty() && state.basket_cursor.selected().is_none());
    let report = state.add_paths(&[tmp.path().join("a.txt")]);
    assert_eq!(report.added, 1, "a cleared file can be added again");
}

#[test]
fn going_up_puts_the_cursor_back_on_the_folder() {
    let (tmp, mut state) = folder();
    state.open_dir(tmp.path().join("sub"));
    state.go_up();
    assert_eq!(state.dir, tmp.path());
    assert_eq!(state.current_item().unwrap().name, "sub");
}

#[test]
fn reports_read_naturally() {
    let report = |added, duplicates| AddReport {
        added,
        duplicates,
        problems: vec![],
    };
    assert_eq!(report(0, 0).notice(), None);
    assert_eq!(report(1, 0).notice().unwrap().1, "Added 1 file");
    assert_eq!(
        report(3, 2).notice().unwrap().1,
        "Added 3 files (2 already in the list)"
    );
    let bad = AddReport {
        problems: vec![("/x".into(), "gone".into())],
        ..Default::default()
    };
    assert_eq!(bad.notice().unwrap().0, Level::Warn);
}
