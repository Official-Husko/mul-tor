//! Diagnostics: failures always go to `runtime.log`; `-v` also shows them live.

use std::fs::OpenOptions;
use std::sync::Mutex;

use mul_tor_core::AppDirs;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

/// Past this size the log is moved aside so it cannot grow without bound.
const MAX_LOG_BYTES: u64 = 1024 * 1024;

pub fn init(dirs: &AppDirs, verbose: u8) {
    let path = dirs.log_file();
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_LOG_BYTES) {
        let _ = std::fs::rename(&path, path.with_extension("log.old"));
    }

    let file_layer = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok()
        .map(|file| {
            fmt::layer()
                .with_ansi(false)
                .with_writer(Mutex::new(file))
                .with_filter(EnvFilter::new("warn"))
        });

    let stderr_layer = (verbose > 0).then(|| {
        let ours = if verbose == 1 { "info" } else { "debug" };
        fmt::layer()
            .with_writer(std::io::stderr)
            .with_filter(EnvFilter::new(format!(
                "warn,mul_tor={ours},mul_tor_core={ours}"
            )))
    });

    // Ignore the error: a subscriber may already be installed (tests).
    let _ = tracing_subscriber::registry()
        .with(file_layer)
        .with(stderr_layer)
        .try_init();
}
