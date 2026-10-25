//! The application: state, background-task plumbing, and what each user
//! action does. Drawing lives in `view`.

mod action;
mod inbox;
mod logs;
mod notice;
mod presets;
mod rows;
mod run;
mod settings;

use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use anyhow::{Context as _, Result};
use mul_tor_core::logging::LogBuffer;
use mul_tor_core::update::Release;
use mul_tor_core::{AppDirs, Config, ConfigStore, PresetLibrary, Registry};
use tokio_util::sync::CancellationToken;

pub use action::Action;
pub use logs::{LogDetail, LogWindow};
pub use notice::Notice;
pub use rows::ProviderRow;
pub use settings::{Settings, SettingsTab};

use crate::model::Model;
use crate::tasks::{self, Bus, Level, Msg};

pub struct MulTorApp {
    pub rt: tokio::runtime::Runtime,
    pub bus: Bus,
    inbox: Receiver<Msg>,
    pub dirs: AppDirs,
    store: ConfigStore,
    pub config: Config,
    pub registry: Registry,
    pub presets: PresetLibrary,
    pub rows: Vec<ProviderRow>,
    pub model: Model,
    pub settings: Settings,
    pub notice: Option<Notice>,
    pub update: Option<Release>,
    /// Recent log records, for the log window.
    pub logs: LogBuffer,
    pub log_window: LogWindow,
    cancel: Option<CancellationToken>,
    #[cfg(test)]
    pub url_rewrite: Option<mul_tor_core::http::UrlRewrite>,
}

impl MulTorApp {
    pub fn new(ctx: egui::Context, home: Option<PathBuf>, logs: LogBuffer) -> Result<Self> {
        let dirs = AppDirs::discover(home.as_deref())?;
        dirs.create_all()?;
        let store = ConfigStore::new(&dirs);
        // The GUI is started from launchers whose working directory means
        // nothing, so there is no Python-era config.json to look for here.
        let (config, _origin) = store
            .load_or_create(None)
            .context("could not load the config")?;
        let presets = PresetLibrary::load(&dirs.presets_dir());

        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .context("could not start the background runtime")?;
        let (bus, inbox) = Bus::new(ctx);

        let mut app = Self {
            rt,
            bus,
            inbox,
            dirs,
            store,
            config,
            registry: Registry::builtin(),
            presets,
            rows: Vec::new(),
            model: Model::default(),
            settings: Settings::default(),
            notice: None,
            update: None,
            logs,
            log_window: LogWindow::default(),
            cancel: None,
            #[cfg(test)]
            url_rewrite: None,
        };
        app.rebuild_rows();
        for (file, reason) in app.presets.problems.clone() {
            app.show_notice(Level::Error, format!("Ignoring preset {file}: {reason}"));
        }
        if app.config.check_availability {
            app.check_availability();
        }
        if app.config.check_for_updates {
            tasks::check_for_update(app.rt.handle(), &app.bus);
        }
        Ok(app)
    }
}

impl eframe::App for MulTorApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.pump();
        self.take_dropped_files(ui.ctx());
        self.draw(ui);

        // Let a status message disappear on its own even if nothing else happens.
        if self.current_notice().is_some() {
            ui.ctx().request_repaint_after(Duration::from_secs(1));
        }
    }
}

impl MulTorApp {
    /// Files dropped onto the window.
    fn take_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .map(|file| file.path().to_path_buf())
                .collect()
        });
        if !dropped.is_empty() {
            self.add_paths(&dropped);
        }
    }
}
