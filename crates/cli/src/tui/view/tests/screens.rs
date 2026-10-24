//! What each step looks like.

use std::sync::Arc;

use crossterm::event::KeyCode;
use mul_tor_core::{Event, JobOutcome, ProgressSnapshot, UploadError, UploadedFile};

use super::{assert_shows, render};
use crate::tui::model::Screen;
use crate::tui::msg::Msg;
use crate::tui::testing::*;

fn queued(job: usize, provider: &'static str) -> Msg {
    Msg::Engine(Event::Queued {
        job,
        file_name: "backup.7z".into(),
        provider,
        size: 1000,
    })
}

fn finished(job: usize, provider: &'static str, result: Result<UploadedFile, UploadError>) -> Msg {
    Msg::Engine(Event::Finished(Arc::new(JobOutcome {
        job,
        file_name: "backup.7z".into(),
        provider,
        result,
    })))
}

#[test]
fn the_first_screen_invites_you_to_choose_files() {
    let mut f = fixture();
    let screen = render(&mut f.model, 110, 36);
    assert_shows(
        &screen,
        &[
            "MUL-TOR".chars().next().map(|_| "d888888b").unwrap(),
            "1 Files",
            "Browse",
            "backup.7z",
            "To upload",
            "Nothing chosen yet",
        ],
    );
}

#[test]
fn chosen_files_are_listed_with_their_total() {
    let mut f = fixture();
    f.model
        .update(Msg::Paste(f.file.to_string_lossy().into_owned()));
    let screen = render(&mut f.model, 110, 36);
    assert_shows(&screen, &["To upload · 1 file", "7 B", "✓"]);
}

#[test]
fn the_path_field_shows_what_is_typed() {
    let mut f = fixture();
    press(&mut f.model, KeyCode::Char('/'));
    text(&mut f.model, "/tmp/some/where");
    assert_shows(
        &render(&mut f.model, 110, 36),
        &["Path (Enter adds it", "/tmp/some/where"],
    );
}

#[test]
fn providers_show_their_state_and_the_details_panel() {
    let mut f = fixture();
    f.model
        .update(Msg::Paste(f.file.to_string_lossy().into_owned()));
    press(&mut f.model, KeyCode::Char('n'));
    press(&mut f.model, KeyCode::Char('a'));
    let screen = render(&mut f.model, 120, 40);
    assert_shows(
        &screen,
        &[
            "2 Providers",
            "Gofile",
            "[x]",
            "needs account",
            "unverified",
            "Details",
            "Limit",
            "Filter:",
        ],
    );
    assert!(
        screen.contains("BowFile") && screen.contains("[-]"),
        "{screen}"
    );
}

#[test]
fn the_filter_narrows_the_list() {
    let mut f = fixture();
    f.model
        .update(Msg::Paste(f.file.to_string_lossy().into_owned()));
    press(&mut f.model, KeyCode::Char('n'));
    press(&mut f.model, KeyCode::Char('/'));
    text(&mut f.model, "gofi");
    let screen = render(&mut f.model, 120, 40);
    assert_shows(&screen, &["Gofile", "Filter: gofi"]);
    assert!(!screen.contains("Pixeldrain"), "{screen}");

    text(&mut f.model, "zzz");
    assert_shows(&render(&mut f.model, 120, 40), &["No provider matches"]);
}

#[test]
fn a_running_upload_shows_bars_percentages_and_queued_jobs() {
    let mut f = fixture();
    f.model.screen = Screen::Upload;
    f.model.upload.begin();
    for (job, provider) in ["Gofile", "Catbox", "Oshi"].into_iter().enumerate() {
        f.model.update(queued(job, provider));
    }
    f.model.update(Msg::Engine(Event::Started { job: 0 }));
    f.model.update(Msg::Engine(Event::Progress {
        job: 0,
        progress: ProgressSnapshot {
            sent: 420,
            total: 1000,
            message: String::new(),
        },
    }));
    f.model
        .update(finished(1, "Catbox", Ok(UploadedFile::new("https://c/1"))));

    let screen = render(&mut f.model, 110, 30);
    assert_shows(
        &screen,
        &[
            "Uploading · 1 of 3 uploads done",
            "Gofile",
            "42%",
            "waiting",
            "✓ uploaded",
            "elapsed",
        ],
    );
}

#[test]
fn a_fully_sent_upload_says_it_is_waiting_for_the_provider() {
    let mut f = fixture();
    f.model.screen = Screen::Upload;
    f.model.upload.begin();
    f.model.update(queued(0, "Gofile"));
    f.model.update(Msg::Engine(Event::Started { job: 0 }));
    f.model.update(Msg::Engine(Event::Progress {
        job: 0,
        progress: ProgressSnapshot {
            sent: 1000,
            total: 1000,
            message: String::new(),
        },
    }));
    assert_shows(
        &render(&mut f.model, 110, 30),
        &["100%", "waiting for reply"],
    );
}

#[test]
fn results_list_links_and_explain_failures() {
    let mut f = fixture();
    f.model.screen = Screen::Upload;
    f.model.upload.begin();
    f.model.update(queued(0, "Gofile"));
    f.model.update(queued(1, "Oshi"));
    f.model.update(finished(
        0,
        "Gofile",
        Ok(UploadedFile::new("https://gofile.io/d/abc")),
    ));
    f.model.update(finished(
        1,
        "Oshi",
        Err(UploadError::protocol(
            "no link in the reply",
            "<html>nope</html>",
        )),
    ));
    f.model.update(Msg::UploadFinished);

    let screen = render(&mut f.model, 110, 30);
    assert_shows(
        &screen,
        &[
            "4 Results",
            "1 uploaded · 1 failed",
            "https://gofile.io/d/abc",
            "✗",
            "no link in the reply",
        ],
    );

    press(&mut f.model, KeyCode::Down);
    assert_shows(
        &render(&mut f.model, 110, 30),
        &["Selected", "kind: protocol", "reply: <html>nope"],
    );
}

#[test]
fn the_log_pane_shows_recent_records() {
    use tracing_subscriber::prelude::*;
    let mut f = fixture();
    let subscriber = tracing_subscriber::registry().with(f.model.logs.layer());
    tracing::subscriber::with_default(subscriber, || {
        tracing::warn!(provider = "Oshi", "gave up");
    });
    press(&mut f.model, KeyCode::Char('L'));
    assert_shows(
        &render(&mut f.model, 110, 36),
        &["Log · L hides", "WARN", "gave up provider=Oshi"],
    );
}
