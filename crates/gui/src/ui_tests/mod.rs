//! Headless tests of the real window: the actual `eframe::App::ui` runs inside
//! a kittest harness, with an in-process server standing in for the providers.

mod layout;
mod logs;
mod selection;
mod settings;
mod uploads;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, Response};
use egui_kittest::Harness;
use egui_kittest::kittest::NodeT;
use mul_tor_core::logging::LogBuffer;

use crate::app::MulTorApp;

/// A home folder whose config keeps the app from touching the network.
pub fn quiet_home() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("config.toml"),
        "check_for_updates = false\ncheck_availability = false\n",
    )
    .unwrap();
    dir
}

pub fn harness(home: &Path) -> Harness<'static, MulTorApp> {
    let home = home.to_path_buf();
    Harness::builder()
        .with_size(egui::vec2(1100.0, 720.0))
        .build_eframe(move |cc| {
            MulTorApp::new(cc.egui_ctx.clone(), Some(home), LogBuffer::new(200)).unwrap()
        })
}

pub fn sample_file(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, b"hello from the gui test").unwrap();
    path
}

pub fn is_disabled(node: egui_kittest::Node<'_>) -> bool {
    node.accesskit_node().is_disabled()
}

/// Step the UI until `done` holds, so background work gets time to report.
pub fn wait_until(
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

pub enum Behaviour {
    /// Answer every request with a Catbox-style link.
    Link,
    /// Accept the request and never answer.
    Hang,
}

pub async fn serve(behaviour: Behaviour) -> (String, tokio::task::JoinHandle<()>) {
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

pub fn redirect_to(base: String) -> mul_tor_core::http::UrlRewrite {
    Arc::new(move |url: &str| {
        let rest = url.strip_prefix("https://").unwrap_or(url);
        format!("{base}/{rest}")
    })
}
