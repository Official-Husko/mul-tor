//! Overlays, presets, filters, notices and other interface behaviour.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use mul_tor_core::availability::Health;

use crate::tui::model::{Overlay, Probe};
use crate::tui::msg::{Cmd, Level, Msg};
use crate::tui::testing::*;

#[test]
fn a_preset_ticks_its_providers_and_supplies_the_link_format() {
    let mut f = fixture();
    f.model
        .update(Msg::Paste(f.file.to_string_lossy().into_owned()));
    press(&mut f.model, KeyCode::Char('n'));
    press(&mut f.model, KeyCode::Char('p'));
    assert!(matches!(f.model.overlay, Some(Overlay::Presets(_))));

    let f95 = f
        .model
        .providers
        .presets
        .iter()
        .position(|p| p.stem == "f95")
        .unwrap();
    for _ in 0..=f95 {
        press(&mut f.model, KeyCode::Down);
    }
    press(&mut f.model, KeyCode::Enter);

    assert!(f.model.overlay.is_none());
    assert_eq!(f.model.providers.preset, Some(f95));
    assert!(f.model.providers.selected.contains("gofile"));
    assert!(f.model.providers.link_format().is_some());
}

#[test]
fn quitting_asks_first() {
    let mut f = fixture();
    press(&mut f.model, KeyCode::Esc);
    assert!(matches!(f.model.overlay, Some(Overlay::Quit)));
    press(&mut f.model, KeyCode::Char('n'));
    assert!(f.model.overlay.is_none() && !f.model.quit);

    ctrl_c(&mut f.model);
    assert!(matches!(
        press(&mut f.model, KeyCode::Char('y')).as_slice(),
        [Cmd::Quit]
    ));
    assert!(f.model.quit);
}

#[test]
fn help_opens_with_a_question_mark_except_while_typing() {
    let mut f = fixture();
    press(&mut f.model, KeyCode::Char('?'));
    assert!(matches!(f.model.overlay, Some(Overlay::Help)));
    press(&mut f.model, KeyCode::Char('x'));
    assert!(f.model.overlay.is_none());

    press(&mut f.model, KeyCode::Char('/'));
    assert!(f.model.typing());
    text(&mut f.model, "what?");
    assert!(f.model.overlay.is_none());
    assert_eq!(f.model.files.path_input.text(), "what?");
}

#[test]
fn the_log_pane_toggles_with_a_capital_l() {
    let mut f = fixture();
    press(&mut f.model, KeyCode::Char('L'));
    assert!(f.model.show_logs);
    press(&mut f.model, KeyCode::F(3));
    assert!(!f.model.show_logs);
}

#[test]
fn offline_providers_are_left_out_of_select_all() {
    let mut f = fixture();
    f.model.update(Msg::Availability(vec![
        ("gofile".to_string(), Health::Up),
        ("catbox".to_string(), Health::Down("timed out".into())),
    ]));
    assert!(!f.model.providers.probing);
    let row = |id: &str| f.model.providers.rows.iter().find(|r| r.id == id).unwrap();
    assert_eq!(row("gofile").probe, Probe::Online);
    assert!(matches!(row("catbox").probe, Probe::Offline(_)));

    f.model
        .update(Msg::Paste(f.file.to_string_lossy().into_owned()));
    press(&mut f.model, KeyCode::Char('n'));
    press(&mut f.model, KeyCode::Char('a'));
    assert!(f.model.providers.selected.contains("gofile"));
    assert!(!f.model.providers.selected.contains("catbox"));
}

#[test]
fn a_provider_without_credentials_cannot_be_ticked() {
    let mut f = fixture();
    f.model
        .update(Msg::Paste(f.file.to_string_lossy().into_owned()));
    press(&mut f.model, KeyCode::Char('n'));
    press(&mut f.model, KeyCode::Char('/'));
    text(&mut f.model, "pixeldrain");
    press(&mut f.model, KeyCode::Enter);
    press(&mut f.model, KeyCode::Char(' '));
    assert!(f.model.providers.selected.is_empty());
    assert!(
        f.model
            .notice
            .as_ref()
            .unwrap()
            .text
            .contains("needs an account")
    );
}

#[test]
fn notices_fade_and_key_releases_are_ignored() {
    let mut f = fixture();
    f.model.notify(Level::Info, "hello");
    for _ in 0..70 {
        f.model.update(Msg::Tick);
    }
    assert!(f.model.notice.is_none());

    let mut release = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    release.kind = KeyEventKind::Release;
    f.model.update(Msg::Key(release));
    assert!(f.model.overlay.is_none());
}
