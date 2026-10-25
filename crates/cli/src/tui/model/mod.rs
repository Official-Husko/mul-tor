//! What the interface knows, and how it changes. Nothing in here touches the
//! terminal, the network or a thread: [`Model::update`] takes a message and
//! returns the commands the outside world should carry out.

mod browser;
mod files;
mod files_keys;
#[cfg(test)]
mod files_keys_tests;
#[cfg(test)]
mod files_tests;
mod overlay;
#[cfg(test)]
mod overlay_tests;
mod presets;
mod providers;
mod providers_keys;
mod screen_keys;
#[cfg(test)]
mod tests;
mod update;
mod upload;
#[cfg(test)]
mod upload_tests;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use mul_tor_core::Credentials;
use mul_tor_core::logging::LogBuffer;
use mul_tor_core::update::Release;

pub use files::{FilesState, Pane};
pub use overlay::{Overlay, Preflight};
pub use providers::{Probe, ProviderRow, ProvidersState};
pub use upload::{JobState, UploadState};

use super::msg::{Cmd, Level};
use crate::app::App;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Files,
    Providers,
    Upload,
    Results,
}

/// A line of feedback that fades after a few seconds.
#[derive(Clone, Debug)]
pub struct Notice {
    pub level: Level,
    pub text: String,
    pub expires_at: u64,
}

/// Ticks (a tenth of a second each) a notice stays up.
const NOTICE_TICKS: u64 = 60;

/// The parts of the settings the interface needs.
pub struct Settings {
    pub parallel: usize,
    pub save_links: bool,
    pub links_dir: PathBuf,
    pub credentials: BTreeMap<String, Credentials>,
    pub config_path: PathBuf,
    pub log_dir: PathBuf,
}

pub struct Model {
    pub screen: Screen,
    pub overlay: Option<Overlay>,
    pub files: FilesState,
    pub providers: ProvidersState,
    pub upload: UploadState,
    pub notice: Option<Notice>,
    pub update: Option<Release>,
    pub logs: LogBuffer,
    pub show_logs: bool,
    pub settings: Settings,
    /// Advances every tick; drives animation and notice expiry.
    pub tick: u64,
    pub quit: bool,
    pub exit_code: u8,
}

impl Model {
    /// The model at start-up, and the commands to run right away.
    pub fn new(app: &App, logs: LogBuffer, start_dir: PathBuf) -> (Self, Vec<Cmd>) {
        let rows = app
            .eligible_providers()
            .into_iter()
            .map(|provider| {
                let usable = !app.lacks_credentials(provider.info());
                ProviderRow::new(provider, usable)
            })
            .collect();
        let (choices, auto) = presets::build(app);
        let mut providers = ProvidersState::new(rows, choices);
        if auto.is_some() {
            providers.apply_preset(auto);
        }

        let mut commands = Vec::new();
        if app.config.check_availability {
            providers.probing = true;
            commands.push(Cmd::CheckAvailability);
        }
        if app.config.check_for_updates {
            commands.push(Cmd::CheckUpdate);
        }

        let model = Self {
            screen: Screen::Files,
            overlay: None,
            files: FilesState::new(start_dir),
            providers,
            upload: UploadState::default(),
            notice: None,
            update: None,
            logs,
            show_logs: false,
            settings: Settings {
                parallel: app.config.parallel(),
                save_links: app.config.save_links,
                links_dir: app
                    .config
                    .links_dir
                    .clone()
                    .unwrap_or_else(|| PathBuf::from(".")),
                credentials: app.config.credentials_by_id(),
                config_path: app.store.path().to_path_buf(),
                log_dir: app.dirs.log_dir(),
            },
            tick: 0,
            quit: false,
            exit_code: 0,
        };
        (model, commands)
    }

    pub fn notify(&mut self, level: Level, text: impl Into<String>) {
        self.notice = Some(Notice {
            level,
            text: text.into(),
            expires_at: self.tick + NOTICE_TICKS,
        });
    }

    pub fn notify_opt(&mut self, notice: Option<(Level, String)>) {
        if let Some((level, text)) = notice {
            self.notify(level, text);
        }
    }

    /// Typing into a text field: letters are text, not commands.
    pub fn typing(&self) -> bool {
        match self.screen {
            Screen::Files => self.files.typing(),
            Screen::Providers => self.providers.filtering,
            _ => false,
        }
    }

    /// What to print on the normal screen after leaving: the links, so they
    /// stay in the scrollback.
    pub fn farewell(&self) -> String {
        let links = self.upload.links();
        if links.is_empty() {
            return String::new();
        }
        let mut out = String::new();
        for link in links {
            let _ = writeln!(out, "{}  {}  {}", link.provider, link.file_name, link.url);
        }
        out
    }
}
