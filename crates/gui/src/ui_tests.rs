//! Headless tests of the real window: the actual `eframe::App::ui` runs inside
//! a kittest harness, with an in-process server standing in for the providers.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, Response};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

use crate::app::{Action, MulTorApp};
use crate::model::JobState;

/// A home folder whose config keeps the app from touching the network.
fn quiet_home() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("config.toml"),
        "check_for_updates = false\ncheck_availability = false\n",
    )
    .unwrap();
    dir
}

fn harness(home: &Path) -> Harness<'static, MulTorApp> {
    let home = home.to_path_buf();
    Harness::builder()
        .with_size(egui::vec2(1100.0, 720.0))
        .build_eframe(move |cc| MulTorApp::new(cc.egui_ctx.clone(), Some(home)).unwrap())
}

fn sample_file(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, b"hello from the gui test").unwrap();
    path
}

fn is_disabled(node: egui_kittest::Node<'_>) -> bool {
    node.accesskit_node().is_disabled()
}

/// Step the UI until `done` holds, so background work gets time to report.
fn wait_until(
    harness: &mut Harness<'static, MulTorApp>,
    what: &str,
    done: impl Fn(&MulTorApp) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !done(harness.state()) {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
        harness.step();
    }
    harness.run_steps(3);
}

enum Behaviour {
    /// Answer every request with a Catbox-style link.
    Link,
    /// Accept the request and never answer.
    Hang,
}

async fn serve(behaviour: Behaviour) -> (String, tokio::task::JoinHandle<()>) {
    let behaviour = Arc::new(behaviour);
    let app = Router::new().fallback(move |request: Request<Body>| {
        let behaviour = behaviour.clone();
        async move {
            let _ = to_bytes(request.into_body(), usize::MAX).await;
            match *behaviour {
                Behaviour::Link => Response::new(Body::from("https://files.catbox.moe/gui123.bin")),
                Behaviour::Hang => std::future::pending().await,
            }
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (base, task)
}

fn redirect_to(base: String) -> mul_tor_core::http::UrlRewrite {
    Arc::new(move |url: &str| {
        let rest = url.strip_prefix("https://").unwrap_or(url);
        format!("{base}/{rest}")
    })
}

#[test]
fn the_main_sections_are_there() {
    let home = quiet_home();
    let harness = harness(home.path());

    for label in [
        "Mul-Tor",
        "Files",
        "Providers",
        "Add files…",
        "Add folder…",
        "Settings",
        "Catbox",
        "Gofile",
    ] {
        harness.get_by_label(label);
    }
    harness.get_by_label_contains("Upload 0 files to 0 providers");
    // All 37 providers are listed (none blacklisted by default).
    assert_eq!(harness.state().rows.len(), 37);
}

#[test]
fn providers_that_need_an_account_cannot_be_ticked_until_configured() {
    let home = quiet_home();
    let harness = harness(home.path());

    let needing: Vec<_> = harness.query_all_by_label("needs account").collect();
    assert_eq!(
        needing.len(),
        5,
        "BowFile, DooDrive, Mixdrop, Pixeldrain, Rapidgator"
    );
    assert!(is_disabled(harness.get_by_label("Pixeldrain")));
    assert!(!is_disabled(harness.get_by_label("Catbox")));
}

#[test]
fn ticking_a_provider_and_adding_a_file_enables_the_upload_button() {
    let home = quiet_home();
    let mut harness = harness(home.path());
    let file = sample_file(home.path(), "a.txt");

    harness.state_mut().add_paths(&[file]);
    harness.run();
    harness.get_by_label_contains("a.txt");
    assert!(is_disabled(
        harness.get_by_label_contains("Upload 1 file to 0 providers")
    ));

    harness.get_by_label("Catbox").click();
    harness.run();
    assert!(harness.state().model.selected.contains("catbox"));
    assert!(!is_disabled(
        harness.get_by_label_contains("Upload 1 file to 1 provider")
    ));

    harness.get_by_label("Catbox").click(); // untick again
    harness.run();
    assert!(harness.state().model.selected.is_empty());
}

#[test]
fn the_search_box_filters_the_provider_list() {
    let home = quiet_home();
    let mut harness = harness(home.path());
    harness.state_mut().model.provider_filter = "cat".into();
    harness.run();
    assert!(harness.query_by_label("Catbox").is_some());
    assert!(harness.query_by_label("Gofile").is_none());
}

#[test]
fn a_preset_ticks_its_usable_providers_and_explains_what_it_skipped() {
    let home = quiet_home();
    let mut harness = harness(home.path());

    harness.get_by_label("Preset:").click();
    harness.run();
    harness.get_by_label("F95").click();
    harness.run();

    let model = &harness.state().model;
    assert_eq!(model.preset.as_deref(), Some("f95"));
    assert!(model.selected.contains("catbox") && model.selected.contains("gofile"));
    assert!(
        !model.selected.contains("pixeldrain"),
        "no credentials, so not ticked"
    );
    let notice = harness
        .state()
        .current_notice()
        .expect("a note about the skipped providers");
    assert!(notice.text.contains("Pixeldrain"), "{}", notice.text);
}

#[derive(Debug)]
struct Dropped(PathBuf);

impl egui::DroppedFile for Dropped {
    fn path(&self) -> &Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

#[test]
fn files_dropped_on_the_window_are_added() {
    let home = quiet_home();
    let mut harness = harness(home.path());
    let file = sample_file(home.path(), "dropped.txt");

    harness
        .input_mut()
        .dropped_files
        .push(Arc::new(Dropped(file)));
    harness.run();

    assert_eq!(harness.state().model.files.len(), 1);
    harness.get_by_label_contains("dropped.txt");
}

#[test]
fn credentials_entered_in_settings_are_saved_and_unlock_the_provider() {
    let home = quiet_home();
    let mut harness = harness(home.path());

    harness.get_by_label("Settings").click();
    harness.run();
    harness.get_by_label("Accounts").click();
    harness.run();

    // Type into a field through the real widget (BowFile's is the first on screen)...
    let field = harness.get_by_label("API key 1");
    field.focus();
    field.type_text("bow-key-1");
    harness.run();
    assert_eq!(
        harness.state().settings.buffers[&("bowfile".to_string(), "apiKey1".to_string())],
        "bow-key-1"
    );
    // ...and fill Pixeldrain's directly: it is scrolled out of view, so it is
    // not in the accessibility tree.
    harness.state_mut().settings.buffers.insert(
        ("pixeldrain".into(), "apiKey".into()),
        "my-pixeldrain-key".into(),
    );

    harness.get_by_label("Save").click();
    harness.run();

    assert!(!harness.state().settings.open);
    assert!(!is_disabled(harness.get_by_label("Pixeldrain")));
    assert!(
        is_disabled(harness.get_by_label("BowFile")),
        "BowFile needs both of its keys"
    );
    let saved = std::fs::read_to_string(home.path().join("config.toml")).unwrap();
    assert!(
        saved.contains("[credentials.Pixeldrain]") && saved.contains("my-pixeldrain-key"),
        "{saved}"
    );
    assert!(saved.contains("apiKey1 = \"bow-key-1\""), "{saved}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(home.path().join("config.toml"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0);
    }
}

#[test]
fn cancelling_the_settings_window_discards_edits() {
    let home = quiet_home();
    let mut harness = harness(home.path());
    harness.get_by_label("Settings").click();
    harness.run();
    harness.state_mut().settings.draft.parallel_uploads = 9;
    harness.get_by_label("Cancel").click();
    harness.run_steps(3);
    assert!(!harness.state().settings.open);
    assert_eq!(harness.state().config.parallel_uploads, 1);
}

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

#[test]
fn an_invalid_action_target_does_not_crash() {
    let home = quiet_home();
    let mut harness = harness(home.path());
    let ctx = harness.ctx.clone();
    harness.state_mut().perform(Action::RemoveFile(42), &ctx);
    harness
        .state_mut()
        .perform(Action::ApplyPreset(Some("no-such-preset".into())), &ctx);
    harness.state_mut().perform(Action::Cancel, &ctx);
    harness.run();
    assert!(harness.state().current_notice().is_some());
}
