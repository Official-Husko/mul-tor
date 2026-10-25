//! Starting the shared logger for the command line.

use mul_tor_core::logging::{self, LogBuffer, LogConfig, LogGuard, console_level};

use crate::app::App;

/// Files always; the terminal only when `-v` was given, and never while a
/// full-screen UI owns the terminal (then `buffer` carries the records to it).
pub fn init(app: &App, verbose: u8, buffer: Option<LogBuffer>) -> LogGuard {
    let mut config = LogConfig::from_settings(&app.config, &app.dirs);
    config = match buffer {
        Some(buffer) => config.buffer(buffer),
        None => config.console(console_level(verbose)),
    };
    let guard = logging::init(config);
    for problem in guard.problems() {
        eprintln!("warning: {problem}");
    }
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        config = %app.store.path().display(),
        logs = %app.dirs.log_dir().display(),
        "mul-tor started"
    );
    guard
}
