//! Choosing, confirming, uploading and finishing.

use crossterm::event::KeyCode;
use mul_tor_core::{Config, Event, UploadError, UploadedFile};

use super::{finished, start_upload};
use crate::tui::model::{Overlay, Screen};
use crate::tui::msg::{Cmd, Level, Msg};
use crate::tui::testing::*;

#[test]
fn it_starts_on_the_files_screen_and_probes_in_the_background() {
    let f = fixture();
    assert_eq!(f.model.screen, Screen::Files);
    assert!(f.start.iter().any(|c| matches!(c, Cmd::CheckAvailability)));
    assert!(f.start.iter().any(|c| matches!(c, Cmd::CheckUpdate)));
    assert!(f.model.providers.probing);

    let quiet = fixture_with(Config {
        check_availability: false,
        check_for_updates: false,
        ..Config::default()
    });
    assert!(quiet.start.is_empty() && !quiet.model.providers.probing);
}

#[test]
fn a_dropped_file_goes_through_to_an_upload() {
    let mut f = fixture();
    let commands = start_upload(&mut f);

    let Some(Cmd::StartUpload(plan)) = commands.into_iter().next() else {
        panic!("no upload was started");
    };
    assert_eq!(plan.files, [f.file.clone()]);
    assert!(!plan.providers.is_empty());
    assert!(
        plan.providers
            .iter()
            .all(|p| !p.info().reliability.is_broken())
    );
    assert_eq!(f.model.screen, Screen::Upload);
    assert!(f.model.overlay.is_none());
}

#[test]
fn leaving_the_files_screen_needs_a_file_and_the_upload_needs_a_provider() {
    let mut f = fixture();
    press(&mut f.model, KeyCode::Char('n'));
    assert_eq!(f.model.screen, Screen::Files);
    assert_eq!(f.model.notice.as_ref().unwrap().level, Level::Warn);

    f.model
        .update(Msg::Paste(f.file.to_string_lossy().into_owned()));
    press(&mut f.model, KeyCode::Char('n'));
    assert_eq!(f.model.screen, Screen::Providers);
    press(&mut f.model, KeyCode::Enter);
    assert!(f.model.overlay.is_none(), "nothing is ticked yet");
    assert!(
        f.model
            .notice
            .as_ref()
            .unwrap()
            .text
            .contains("Tick at least one")
    );
}

#[test]
fn the_summary_is_shown_before_anything_is_sent() {
    let mut f = fixture();
    f.model
        .update(Msg::Paste(f.file.to_string_lossy().into_owned()));
    press(&mut f.model, KeyCode::Char('n'));
    press(&mut f.model, KeyCode::Char('a'));
    let commands = press(&mut f.model, KeyCode::Enter);
    assert!(commands.is_empty());
    let Some(Overlay::Confirm(report)) = &f.model.overlay else {
        panic!("no summary");
    };
    assert_eq!((report.files, report.bytes), (1, 7));
    assert_eq!(report.uploads, report.providers.len());

    press(&mut f.model, KeyCode::Esc);
    assert!(f.model.overlay.is_none() && f.model.screen == Screen::Providers);
}

#[test]
fn ctrl_c_cancels_a_running_upload_and_twice_quits() {
    let mut f = fixture();
    start_upload(&mut f);

    assert!(matches!(
        ctrl_c(&mut f.model).as_slice(),
        [Cmd::CancelUpload]
    ));
    assert!(f.model.upload.cancelling && !f.model.quit);
    assert!(matches!(ctrl_c(&mut f.model).as_slice(), [Cmd::Quit]));
    assert!(f.model.quit);
    assert_eq!(f.model.exit_code, 130);
}

#[test]
fn a_finished_run_shows_results_and_sets_the_exit_code() {
    let mut f = fixture();
    start_upload(&mut f);
    f.model.update(Msg::Engine(Event::Queued {
        job: 0,
        file_name: "backup.7z".into(),
        provider: "Gofile",
        size: 7,
    }));
    f.model.update(Msg::Engine(Event::Queued {
        job: 1,
        file_name: "backup.7z".into(),
        provider: "Oshi",
        size: 7,
    }));
    f.model
        .update(finished(0, Ok(UploadedFile::new("https://gofile.io/d/1"))));
    f.model.update(finished(1, Err(UploadError::EmptyFile)));
    f.model.update(Msg::UploadFinished);

    assert_eq!(f.model.screen, Screen::Results);
    assert_eq!(f.model.exit_code, 1);
    assert!(f.model.farewell().contains("https://gofile.io/d/1"));
    assert_eq!(f.model.notice.as_ref().unwrap().level, Level::Warn);
}

#[test]
fn links_are_copied_plain_or_in_the_preset_format() {
    let mut f = fixture();
    start_upload(&mut f);
    f.model.update(Msg::Engine(Event::Queued {
        job: 0,
        file_name: "backup.7z".into(),
        provider: "Gofile",
        size: 7,
    }));
    f.model
        .update(finished(0, Ok(UploadedFile::new("https://gofile.io/d/1"))));
    f.model.update(Msg::UploadFinished);

    let Some(Cmd::Copy(plain)) = press(&mut f.model, KeyCode::Char('c')).into_iter().next() else {
        panic!("nothing copied");
    };
    assert_eq!(plain, "https://gofile.io/d/1");

    assert!(press(&mut f.model, KeyCode::Char('f')).is_empty());
    assert!(
        f.model
            .notice
            .as_ref()
            .unwrap()
            .text
            .contains("No preset was used")
    );
}
