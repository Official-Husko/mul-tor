//! Uploading, end to end, against a local server.

use egui_kittest::kittest::Queryable;

use super::*;
use crate::model::JobState;

#[tokio::test(flavor = "multi_thread")]
async fn a_full_upload_shows_the_link_and_writes_the_link_file() {
    let (base, _server) = serve(Behaviour::Link).await;
    let home = quiet_home();
    let file = sample_file(home.path(), "report.txt");

    let home_path = home.path().to_path_buf();
    let outcome = tokio::task::spawn_blocking(move || {
        let mut harness = harness(&home_path);
        harness.state_mut().url_rewrite = Some(redirect_to(base));
        harness.state_mut().add_paths(&[file]);
        harness.run();
        harness.get_by_label("Catbox").click();
        harness.run();

        harness
            .get_by_label_contains("Upload 1 file to 1 provider")
            .click();
        harness.run_steps(3);
        wait_until(&mut harness, "the upload to finish", |app| {
            !app.model.is_running() && app.model.counts().done == 1
        });

        harness.get_by_label_contains("files.catbox.moe/gui123.bin");
        assert!(!is_disabled(harness.get_by_label("Copy links")));
        let jobs = harness.state().model.jobs.clone();
        let text = harness.state().model.links_text(false);
        (jobs, text)
    })
    .await
    .unwrap();

    assert_eq!(outcome.0.len(), 1);
    assert_eq!(
        outcome.0[0].state,
        JobState::Done {
            url: "https://files.catbox.moe/gui123.bin".into()
        }
    );
    assert_eq!(outcome.1, "https://files.catbox.moe/gui123.bin");
    let links = std::fs::read_to_string(home.path().join("links/file_links.txt")).unwrap();
    assert!(
        links.contains("| Catbox | report.txt - https://files.catbox.moe/gui123.bin"),
        "{links}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_cancel_button_stops_an_upload_that_never_finishes() {
    let (base, _server) = serve(Behaviour::Hang).await;
    let home = quiet_home();
    let file = sample_file(home.path(), "stuck.txt");

    let home_path = home.path().to_path_buf();
    let counts = tokio::task::spawn_blocking(move || {
        let mut harness = harness(&home_path);
        harness.state_mut().url_rewrite = Some(redirect_to(base));
        harness.state_mut().add_paths(&[file]);
        harness.state_mut().model.toggle_provider("catbox", true);
        harness.run();

        harness
            .get_by_label_contains("Upload 1 file to 1 provider")
            .click();
        harness.run_steps(3);
        wait_until(&mut harness, "the upload to start", |app| {
            app.model.counts().running == 1
        });

        harness.get_by_label("Cancel").click();
        harness.run_steps(3);
        wait_until(&mut harness, "the cancellation to finish", |app| {
            !app.model.is_running()
        });
        harness.state().model.counts()
    })
    .await
    .unwrap();

    assert_eq!(counts.cancelled, 1);
    assert_eq!(counts.done + counts.failed, 0);
}
