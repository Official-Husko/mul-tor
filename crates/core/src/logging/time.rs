//! Timestamps: local time with milliseconds and the UTC offset, so a line
//! stays unambiguous when a log is passed to someone in another time zone.

use std::fmt;

use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;

/// `2026-10-24 13:05:41.123 +02:00`
pub(super) const FILE_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f %:z";

/// `13:05:41.123`
const CONSOLE_FORMAT: &str = "%H:%M:%S%.3f";

/// Full local timestamp, used in files.
#[derive(Clone, Copy, Debug)]
pub(super) struct FileTime;

impl FormatTime for FileTime {
    fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
        write!(w, "{}", chrono::Local::now().format(FILE_FORMAT))
    }
}

/// Short local timestamp, used on the terminal.
#[derive(Clone, Copy, Debug)]
pub(super) struct ConsoleTime;

impl FormatTime for ConsoleTime {
    fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
        write!(w, "{}", chrono::Local::now().format(CONSOLE_FORMAT))
    }
}
