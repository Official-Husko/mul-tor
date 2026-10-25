//! Work that happens off the UI thread, and how it reports back.
//!
//! Everything is a message on a channel; each send also asks egui to repaint,
//! so the window updates the moment something happens even when idle.

mod pick;
mod probes;
mod run;

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

use mul_tor_core::Event;
use mul_tor_core::availability::Health;
use mul_tor_core::update::Release;

pub use pick::pick;
pub use probes::{check_availability, check_for_update};
pub use run::{RunRequest, start_run};

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
