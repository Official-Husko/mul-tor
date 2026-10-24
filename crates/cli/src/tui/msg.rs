//! What can happen to the interface (`Msg`), and what the interface asks the
//! outside world to do about it (`Cmd`). The model turns the first into the
//! second without touching a terminal, a network or a thread.

use std::path::PathBuf;
use std::sync::Arc;

use crossterm::event::KeyEvent;
use mul_tor_core::availability::Health;
use mul_tor_core::update::Release;
use mul_tor_core::{Event, Provider};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

pub enum Msg {
    Key(KeyEvent),
    /// Text pasted into the terminal. Dropping a file on the window pastes its path.
    Paste(String),
    /// Time passes (animation, notices fading). Also sent on resize.
    Tick,
    Engine(Event),
    UploadFinished,
    /// `(provider id, health)` for every provider that was probed.
    Availability(Vec<(String, Health)>),
    UpdateFound(Release),
    Notice(Level, String),
}

/// Everything the engine needs for one run.
pub struct Plan {
    pub files: Vec<PathBuf>,
    pub providers: Vec<Arc<dyn Provider>>,
    pub link_format: Option<String>,
    pub parallel: usize,
    pub save_links: bool,
    pub links_dir: PathBuf,
}

pub enum Cmd {
    Quit,
    StartUpload(Plan),
    CancelUpload,
    CheckAvailability,
    CheckUpdate,
    /// Put this text on the clipboard.
    Copy(String),
}
