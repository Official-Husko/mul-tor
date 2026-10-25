//! Drawing must never panic, whatever the window size or colour support.

use crossterm::event::KeyCode;

use super::{assert_shows, render, render_with};
use crate::tui::model::{Model, Overlay, Screen};
use crate::tui::msg::Msg;
use crate::tui::testing::*;
use crate::tui::theme::{ColorMode, Theme};

fn every_state() -> Vec<Model> {
    let mut states = Vec::new();
    for screen in [
        Screen::Files,
        Screen::Providers,
        Screen::Upload,
        Screen::Results,
    ] {
        for overlay in [None, Some(Overlay::Help), Some(Overlay::Quit)] {
            let mut f = fixture();
            f.model
                .update(Msg::Paste(f.file.to_string_lossy().into_owned()));
            press(&mut f.model, KeyCode::Char('/'));
            text(&mut f.model, "日本語 path");
            press(&mut f.model, KeyCode::Esc);
            f.model.screen = screen;
            f.model.overlay = overlay;
            f.model.show_logs = true;
            states.push(f.model);
        }
    }
    states
}

#[test]
fn no_window_size_makes_drawing_panic() {
    for mut model in every_state() {
        for width in [0, 1, 5, 20, 55, 56, 57, 80, 130, 300] {
            for height in [0, 1, 3, 13, 14, 20, 31, 32, 60] {
                render(&mut model, width, height);
            }
        }
    }
}

#[test]
fn all_three_colour_modes_draw() {
    for mode in [ColorMode::TrueColor, ColorMode::Ansi, ColorMode::Mono] {
        for mut model in every_state() {
            render_with(&mut model, 110, 36, &Theme::new(mode));
        }
    }
}

#[test]
fn a_window_that_is_too_small_says_so() {
    let mut f = fixture();
    let screen = render(&mut f.model, 40, 10);
    assert_shows(&screen, &["Window too small", "needs 56x14, has 40x10"]);
}

#[test]
fn the_banner_gives_way_to_a_one_line_title_on_short_windows() {
    let mut f = fixture();
    let tall = render(&mut f.model, 110, 36);
    let short = render(&mut f.model, 110, 24);
    assert!(tall.contains("d888888b"));
    assert!(
        !short.contains("d888888b") && short.contains("MUL-TOR"),
        "{short}"
    );
}
