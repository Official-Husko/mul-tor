//! Turning a [`LogConfig`] into an installed logger.

use std::path::{Path, PathBuf};
use std::sync::Once;

use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::*;

use super::buffer::LogBuffer;
use super::layers::{self, BoxedLayer};
use super::{files, filter, panic};
use crate::{AppDirs, Config};

/// What to record and where to put it.
#[derive(Clone, Debug)]
pub struct LogConfig {
    directory: Option<PathBuf>,
    level: Level,
    console: Option<Level>,
    json: bool,
    buffer: Option<LogBuffer>,
    capture_panics: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            directory: None,
            level: Level::INFO,
            console: None,
            json: false,
            buffer: None,
            capture_panics: true,
        }
    }
}

impl LogConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Files in the standard log folder, with the level and format the user
    /// chose in their settings. An unreadable level means "info".
    pub fn from_settings(config: &Config, dirs: &AppDirs) -> Self {
        Self::new()
            .directory(dirs.log_dir())
            .level(super::parse_level(&config.log_level).unwrap_or(Level::INFO))
            .json(config.log_json)
    }

    /// Write daily files into this folder.
    pub fn directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.directory = Some(directory.into());
        self
    }

    /// Level for the file and the buffer (default: info).
    pub fn level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    /// Also print to stderr at this level. Never use this in a full-screen UI.
    pub fn console(mut self, level: Option<Level>) -> Self {
        self.console = level;
        self
    }

    /// One JSON object per line in the file.
    pub fn json(mut self, json: bool) -> Self {
        self.json = json;
        self
    }

    /// Keep recent records here for a UI to show.
    pub fn buffer(mut self, buffer: LogBuffer) -> Self {
        self.buffer = Some(buffer);
        self
    }

    pub fn capture_panics(mut self, capture: bool) -> Self {
        self.capture_panics = capture;
        self
    }
}

/// Keep this alive until the program ends: dropping it flushes the file.
#[must_use = "dropping the guard stops logging to the file"]
pub struct LogGuard {
    _file: Option<WorkerGuard>,
    directory: Option<PathBuf>,
    problems: Vec<String>,
}

impl LogGuard {
    /// The folder the files go to, if logging to files works.
    pub fn directory(&self) -> Option<&Path> {
        self.directory.as_deref()
    }

    /// Why part of the setup did not work (the program still runs).
    pub fn problems(&self) -> &[String] {
        &self.problems
    }
}

/// Install the logger. Never fails: a part that cannot be set up is skipped
/// and reported in [`LogGuard::problems`].
pub fn init(config: LogConfig) -> LogGuard {
    let mut problems = Vec::new();
    let mut layers: Vec<BoxedLayer> = Vec::new();
    let mut file_guard = None;
    let mut directory = None;

    if let Some(dir) = &config.directory {
        match files::open(dir) {
            Ok((writer, guard)) => {
                layers.push(layers::file(writer, config.json, config.level));
                file_guard = Some(guard);
                directory = Some(dir.clone());
            }
            Err(error) => problems.push(format!(
                "could not write logs to {}: {error}",
                dir.display()
            )),
        }
    }
    if let Some(level) = config.console {
        layers.push(layers::console(level));
    }
    if let Some(buffer) = &config.buffer {
        layers.push(
            buffer
                .layer()
                .with_filter(filter::build(config.level))
                .boxed(),
        );
    }

    if tracing_subscriber::registry()
        .with(layers)
        .try_init()
        .is_err()
    {
        // Tests and embedding programs may have installed their own.
        problems.push("a logger was already installed".to_string());
    } else if config.capture_panics {
        static HOOK: Once = Once::new();
        HOOK.call_once(panic::install);
    }

    LogGuard {
        _file: file_guard,
        directory,
        problems,
    }
}
