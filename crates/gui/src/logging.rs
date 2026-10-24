//! Starting the shared logger for the window.

use std::path::Path;

use mul_tor_core::logging::{self, LogConfig, LogGuard};
use mul_tor_core::{AppDirs, Config, ConfigStore};

/// Daily files with the level the user chose. A broken config is not this
/// module's problem to report: the app itself shows that error.
pub fn init(home: Option<&Path>) -> Option<LogGuard> {
    let dirs = AppDirs::discover(home).ok()?;
    dirs.create_all().ok()?;
    let config = ConfigStore::new(&dirs)
        .load_or_create(None)
        .map(|(config, _)| config)
        .unwrap_or_else(|_| Config::default());
    let guard = logging::init(LogConfig::from_settings(&config, &dirs));
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        logs = %dirs.log_dir().display(),
        "mul-tor-gui started"
    );
    Some(guard)
}
