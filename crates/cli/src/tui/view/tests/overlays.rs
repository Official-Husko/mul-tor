//! The pop-ups.

use crossterm::event::KeyCode;

use super::{assert_shows, render};
use crate::tui::model::Screen;
use crate::tui::msg::Msg;
use crate::tui::testing::*;

fn on_providers() -> Fixture {
    let mut f = fixture();
    f.model
        .update(Msg::Paste(f.file.to_string_lossy().into_owned()));
    press(&mut f.model, KeyCode::Char('n'));
    f
}

#[test]
fn help_lists_the_keys_of_the_current_screen() {
    let mut f = fixture();
    press(&mut f.model, KeyCode::Char('?'));
    assert_shows(
        &render(&mut f.model, 110, 36),
        &["Keys", "open a folder", "this help"],
    );

    let mut f = on_providers();
    press(&mut f.model, KeyCode::Char('?'));
    assert_shows(
        &render(&mut f.model, 110, 36),
        &["tick or untick", "use a preset"],
    );
}

#[test]
fn the_summary_names_what_will_be_sent_and_what_will_fail() {
    let mut f = on_providers();
    std::fs::write(f._dir.path().join("empty.bin"), "").unwrap();
    f.model.files.add_paths(&[f._dir.path().join("empty.bin")]);
    press(&mut f.model, KeyCode::Char('a'));
    press(&mut f.model, KeyCode::Enter);
    let screen = render(&mut f.model, 110, 40);
    assert_shows(
        &screen,
        &[
            "Ready to upload?",
            "2 files",
            "will fail before anything is sent",
            "empty.bin",
            "start",
        ],
    );
}

#[test]
fn the_preset_list_offers_none_and_the_bundled_presets() {
    let mut f = on_providers();
    press(&mut f.model, KeyCode::Char('p'));
    assert_shows(
        &render(&mut f.model, 110, 36),
        &["Presets", "None", "F95", "CS.Rin.Ru"],
    );
}

#[test]
fn quitting_says_what_would_be_lost() {
    let mut f = fixture();
    press(&mut f.model, KeyCode::Esc);
    assert_shows(
        &render(&mut f.model, 110, 36),
        &["Quit Mul-Tor?", "Nothing is uploading"],
    );

    f.model.overlay = None;
    f.model.files.add_paths(&[f.file.clone()]);
    press(&mut f.model, KeyCode::Esc);
    assert_shows(
        &render(&mut f.model, 110, 36),
        &["list of files will be forgotten"],
    );

    f.model.overlay = None;
    f.model.screen = Screen::Upload;
    f.model.overlay = Some(crate::tui::model::Overlay::Quit);
    assert_shows(
        &render(&mut f.model, 110, 36),
        &["Uploads are still running"],
    );
}
