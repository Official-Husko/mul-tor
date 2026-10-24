//! Work that happens off the UI thread, and how it reports back.
//!
//! Everything is a message on a channel; each send also asks egui to repaint,
//! so the window updates the moment something happens even when idle.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

use mul_tor_core::availability::{self, Health};
use mul_tor_core::update::{self, Release};
use mul_tor_core::{AppDirs, Config, Engine, Event, HttpFactory, LinkLog, Provider, setup};
use tokio::runtime::Handle;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Info,
    Error,
}

pub enum Msg {
    Engine(Event),
    RunFinished,
    FilesPicked(Vec<PathBuf>),
    /// `(provider id, health)` for each probed provider.
    Availability(Vec<(String, Health)>),
    UpdateAvailable(Release),
    Notice(Level, String),
}

#[derive(Clone)]
pub struct Bus {
    tx: Sender<Msg>,
    ctx: egui::Context,
}

impl Bus {
    pub fn new(ctx: egui::Context) -> (Self, Receiver<Msg>) {
        let (tx, rx) = mpsc::channel();
        (Self { tx, ctx }, rx)
    }

    pub fn send(&self, msg: Msg) {
        // The receiver only disappears when the app is closing.
        let _ = self.tx.send(msg);
        self.ctx.request_repaint();
    }

    pub fn notice(&self, level: Level, text: impl Into<String>) {
        self.send(Msg::Notice(level, text.into()));
    }
}

pub struct RunRequest {
    pub files: Vec<PathBuf>,
    pub providers: Vec<Arc<dyn Provider>>,
    pub config: Config,
    pub dirs: AppDirs,
    pub link_format: Option<String>,
    /// Test hook: points every provider at a local server. Not present in real builds.
    #[cfg(test)]
    pub url_rewrite: Option<mul_tor_core::http::UrlRewrite>,
}

/// Start uploading; returns the token that cancels the run.
pub fn start_run(rt: &Handle, bus: &Bus, request: RunRequest) -> CancellationToken {
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let bus = bus.clone();
    rt.spawn(async move {
        #[cfg(test)]
        let url_rewrite = request.url_rewrite.clone();
        let RunRequest {
            files,
            providers,
            config,
            dirs,
            link_format,
            ..
        } = request;

        let setup = setup::http_factory(&config, &dirs, None).await;
        for note in &setup.notes {
            bus.notice(Level::Info, note.clone());
        }
        #[allow(unused_mut)]
        let mut factory = setup.factory;
        // Test hook (compiled out of real builds): talk to a local server.
        #[cfg(test)]
        if let Some(rewrite) = url_rewrite {
            factory = factory
                .with_proxy(mul_tor_core::ProxyMode::Direct)
                .with_url_rewrite(rewrite);
        }

        let links_dir = config.links_dir.clone().unwrap_or_else(|| dirs.links_dir());
        let link_log = config
            .save_links
            .then(|| LinkLog::new(links_dir).with_template(link_format));
        let engine = Engine::new(factory)
            .with_credentials(config.credentials_by_id())
            .with_parallelism(config.parallel())
            .with_link_log(link_log);

        let sink_bus = bus.clone();
        let sink: mul_tor_core::EventSink =
            Arc::new(move |event| sink_bus.send(Msg::Engine(event)));
        engine.run(files, providers, sink, token).await;
        bus.send(Msg::RunFinished);
    });
    cancel
}

/// Probe `providers` and report which answer.
pub fn check_availability(
    rt: &Handle,
    bus: &Bus,
    config: Config,
    dirs: AppDirs,
    providers: Vec<(String, Arc<dyn Provider>)>,
) {
    let bus = bus.clone();
    rt.spawn(async move {
        let setup = setup::http_factory(&config, &dirs, None).await;
        let just_providers: Vec<_> = providers
            .iter()
            .map(|(_, provider)| provider.clone())
            .collect();
        let statuses = availability::check(&setup.factory, &just_providers).await;
        let results = providers
            .iter()
            .zip(statuses)
            .map(|((id, _), status)| (id.clone(), status.health))
            .collect();
        bus.send(Msg::Availability(results));
    });
}

pub fn check_for_update(rt: &Handle, bus: &Bus) {
    let bus = bus.clone();
    rt.spawn(async move {
        let Ok(http) = HttpFactory::new().client() else {
            return;
        };
        match update::latest_release(&http).await {
            Ok(release) if update::is_newer(env!("CARGO_PKG_VERSION"), &release) => {
                bus.send(Msg::UpdateAvailable(release));
            }
            Ok(_) => {}
            // Not worth bothering the user: an offline start is normal.
            Err(error) => tracing::info!(%error, "update check failed"),
        }
    });
}

/// Show the system file (or folder) picker and report what was chosen.
pub fn pick(rt: &Handle, bus: &Bus, folder: bool) {
    let bus = bus.clone();
    rt.spawn(async move {
        let paths: Vec<PathBuf> = if folder {
            rfd::AsyncFileDialog::new()
                .set_title("Add a folder")
                .pick_folder()
                .await
                .into_iter()
                .map(|handle| handle.path().to_path_buf())
                .collect()
        } else {
            rfd::AsyncFileDialog::new()
                .set_title("Add files")
                .pick_files()
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|handle| handle.path().to_path_buf())
                .collect()
        };
        if !paths.is_empty() {
            bus.send(Msg::FilesPicked(paths));
        }
    });
}
