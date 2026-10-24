//! The application: state, background-task plumbing, and what each user
//! action does. Drawing lives in `view`.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use mul_tor_core::update::Release;
use mul_tor_core::{AppDirs, Config, ConfigStore, PresetLibrary, Provider, Registry};
use tokio_util::sync::CancellationToken;

use crate::model::Model;
use crate::tasks::{self, Bus, Level, Msg, RunRequest};

/// How long a status message stays visible.
const NOTICE_LIFETIME: Duration = Duration::from_secs(10);

/// One user intent, produced by the views and applied by [`MulTorApp::perform`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    PickFiles,
    PickFolder,
    ClearFiles,
    RemoveFile(usize),
    ToggleProvider { id: String, on: bool },
    SetFilter(String),
    SelectAllProviders,
    SelectNoProviders,
    ApplyPreset(Option<String>),
    CheckAvailability,
    Start,
    Cancel,
    CopyLinks { formatted: bool },
    CopyText(String),
    OpenUrl(String),
    OpenSettings,
    SaveSettings,
    CloseSettings,
}

/// A provider as displayed: its normalised id plus the uploader.
pub struct ProviderRow {
    pub id: String,
    pub provider: Arc<dyn Provider>,
}

pub struct Notice {
    pub level: Level,
    pub text: String,
    at: Instant,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsTab {
    #[default]
    General,
    Accounts,
}

#[derive(Default)]
pub struct Settings {
    pub open: bool,
    pub tab: SettingsTab,
    /// Edited copy of the config; applied on save.
    pub draft: Config,
    /// Text being edited per `(provider id, credential key)`.
    pub buffers: BTreeMap<(String, String), String>,
    pub reveal_secrets: bool,
}

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
    cancel: Option<CancellationToken>,
    #[cfg(test)]
    pub url_rewrite: Option<mul_tor_core::http::UrlRewrite>,
}

impl MulTorApp {
    pub fn new(ctx: egui::Context, home: Option<PathBuf>) -> Result<Self> {
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

    /// The providers shown in the list: everything not on the blacklist.
    pub fn rebuild_rows(&mut self) {
        self.rows = self
            .registry
            .iter()
            .filter(|provider| !self.config.is_blacklisted(provider.info()))
            .map(|provider| ProviderRow {
                id: provider.info().id(),
                provider: provider.clone(),
            })
            .collect();
        let known: Vec<String> = self.rows.iter().map(|r| r.id.clone()).collect();
        self.model.selected.retain(|id| known.contains(id));
    }

    pub fn lacks_credentials(&self, provider: &Arc<dyn Provider>) -> bool {
        self.config.lacks_credentials(provider.info())
    }

    pub fn show_notice(&mut self, level: Level, text: impl Into<String>) {
        self.notice = Some(Notice {
            level,
            text: text.into(),
            at: Instant::now(),
        });
    }

    /// The current status message, until it times out.
    pub fn current_notice(&self) -> Option<&Notice> {
        self.notice
            .as_ref()
            .filter(|n| n.at.elapsed() < NOTICE_LIFETIME)
    }

    /// Handle everything the background tasks have reported since last frame.
    pub fn pump(&mut self) {
        while let Ok(msg) = self.inbox.try_recv() {
            match msg {
                Msg::Engine(event) => self.model.apply(event),
                Msg::RunFinished => {
                    self.model.run_finished();
                    self.cancel = None;
                    let counts = self.model.counts();
                    let (level, text) = if counts.failed > 0 {
                        (
                            Level::Error,
                            format!(
                                "Finished: {} uploaded, {} failed",
                                counts.done, counts.failed
                            ),
                        )
                    } else if counts.cancelled > 0 {
                        (
                            Level::Info,
                            format!("Cancelled: {} uploaded before stopping", counts.done),
                        )
                    } else {
                        (Level::Info, format!("Finished: {} uploaded", counts.done))
                    };
                    self.show_notice(level, text);
                }
                Msg::FilesPicked(paths) => self.add_paths(&paths),
                Msg::Availability(results) => {
                    self.model.checking = false;
                    self.model.health = results.into_iter().collect();
                }
                Msg::UpdateAvailable(release) => self.update = Some(release),
                Msg::Notice(level, text) => self.show_notice(level, text),
            }
        }
    }

    pub fn add_paths(&mut self, paths: &[PathBuf]) {
        let problems = self.model.add_paths(paths);
        if let Some((path, reason)) = problems.first() {
            let more = if problems.len() > 1 {
                format!(" (and {} more)", problems.len() - 1)
            } else {
                String::new()
            };
            self.show_notice(Level::Error, format!("{}: {reason}{more}", path.display()));
        }
    }

    fn check_availability(&mut self) {
        self.model.checking = true;
        let providers = self
            .rows
            .iter()
            .map(|r| (r.id.clone(), r.provider.clone()))
            .collect();
        tasks::check_availability(
            self.rt.handle(),
            &self.bus,
            self.config.clone(),
            self.dirs.clone(),
            providers,
        );
    }

    fn apply_preset(&mut self, stem: Option<String>) {
        let Some(stem) = stem else {
            self.model.preset = None;
            return;
        };
        let Some(preset) = self.presets.find(&stem) else {
            self.show_notice(Level::Error, format!("No preset called {stem}"));
            return;
        };
        let resolved = preset.resolve(&self.registry);
        let mut skipped = Vec::new();
        let mut broken = Vec::new();
        let mut ids = Vec::new();
        for provider in &resolved.providers {
            let Some(row) = self
                .rows
                .iter()
                .find(|r| Arc::ptr_eq(&r.provider, provider))
            else {
                continue; // blacklisted
            };
            if self.config.lacks_credentials(provider.info()) {
                skipped.push(provider.info().name);
            } else if !provider.info().reliability.included_in_bulk_choices() {
                broken.push(provider.info().name);
            } else {
                ids.push(row.id.clone());
            }
        }
        self.model.select_only(ids.iter().map(String::as_str));
        self.model.preset = Some(stem);

        let mut remarks = Vec::new();
        if !skipped.is_empty() {
            remarks.push(format!("needs credentials: {}", skipped.join(", ")));
        }
        if !broken.is_empty() {
            remarks.push(format!("known not to work: {}", broken.join(", ")));
        }
        if !resolved.unsupported.is_empty() {
            remarks.push(format!(
                "not supported yet: {}",
                resolved.unsupported.join(", ")
            ));
        }
        if !remarks.is_empty() {
            self.show_notice(
                Level::Info,
                format!("Preset applied; {}", remarks.join("; ")),
            );
        }
    }

    fn start(&mut self) {
        if !self.model.can_start() {
            return;
        }
        let providers: Vec<Arc<dyn Provider>> = self
            .rows
            .iter()
            .filter(|row| self.model.selected.contains(&row.id))
            .map(|row| row.provider.clone())
            .collect();
        let files = self.model.files.iter().map(|f| f.path.clone()).collect();
        let link_format = self
            .model
            .preset
            .as_deref()
            .and_then(|stem| self.presets.find(stem))
            .map(|preset| preset.link_format.clone())
            .filter(|format| !format.is_empty());

        self.model.begin_run(link_format.clone());
        self.cancel = Some(tasks::start_run(
            self.rt.handle(),
            &self.bus,
            RunRequest {
                files,
                providers,
                config: self.config.clone(),
                dirs: self.dirs.clone(),
                link_format,
                #[cfg(test)]
                url_rewrite: self.url_rewrite.clone(),
            },
        ));
    }

    pub fn open_settings(&mut self) {
        let mut buffers = BTreeMap::new();
        for row in &self.rows {
            let info = row.provider.info();
            let current = self.config.credentials_for(info);
            for field in info.credentials {
                buffers.insert(
                    (row.id.clone(), field.key.to_string()),
                    current.get(field.key).unwrap_or_default().to_string(),
                );
            }
        }
        self.settings = Settings {
            open: true,
            draft: self.config.clone(),
            buffers,
            ..Settings::default()
        };
    }

    fn save_settings(&mut self) {
        let mut config = self.settings.draft.clone();
        for row in &self.rows {
            let info = row.provider.info();
            if info.credentials.is_empty() {
                continue;
            }
            let mut credentials = mul_tor_core::Credentials::default();
            for field in info.credentials {
                let value = self
                    .settings
                    .buffers
                    .get(&(row.id.clone(), field.key.to_string()))
                    .map(|v| v.trim().to_string())
                    .unwrap_or_default();
                if !value.is_empty() {
                    credentials.set(field.key, value);
                }
            }
            config.set_credentials(info, credentials);
        }
        config.parallel_uploads = config.parallel();
        match self.store.save(&config) {
            Ok(()) => {
                self.config = config;
                self.settings.open = false;
                self.rebuild_rows();
                self.show_notice(Level::Info, "Settings saved");
            }
            Err(error) => self.show_notice(
                Level::Error,
                format!("Could not save the settings: {error}"),
            ),
        }
    }

    /// Carry out what the user asked for.
    pub fn perform(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::PickFiles => tasks::pick(self.rt.handle(), &self.bus, false),
            Action::PickFolder => tasks::pick(self.rt.handle(), &self.bus, true),
            Action::ClearFiles => self.model.files.clear(),
            Action::RemoveFile(index) => self.model.remove_file(index),
            Action::ToggleProvider { id, on } => self.model.toggle_provider(&id, on),
            Action::SetFilter(filter) => self.model.provider_filter = filter,
            Action::SelectAllProviders => {
                let ids: Vec<String> = self
                    .rows
                    .iter()
                    .filter(|r| !self.lacks_credentials(&r.provider))
                    .filter(|r| r.provider.info().reliability.included_in_bulk_choices())
                    .filter(|r| {
                        crate::model::matches_filter(&*r.provider, &self.model.provider_filter)
                    })
                    .map(|r| r.id.clone())
                    .collect();
                for id in ids {
                    self.model.toggle_provider(&id, true);
                }
            }
            Action::SelectNoProviders => self.model.selected.clear(),
            Action::ApplyPreset(stem) => self.apply_preset(stem),
            Action::CheckAvailability => self.check_availability(),
            Action::Start => self.start(),
            Action::Cancel => {
                if let Some(cancel) = &self.cancel {
                    cancel.cancel();
                }
                self.model.request_cancel();
            }
            Action::CopyLinks { formatted } => {
                ctx.copy_text(self.model.links_text(formatted));
                self.show_notice(Level::Info, "Links copied to the clipboard");
            }
            Action::CopyText(text) => ctx.copy_text(text),
            Action::OpenUrl(url) => ctx.open_url(egui::OpenUrl::new_tab(url)),
            Action::OpenSettings => self.open_settings(),
            Action::SaveSettings => self.save_settings(),
            Action::CloseSettings => self.settings.open = false,
        }
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
