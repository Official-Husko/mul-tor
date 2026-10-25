//! The log window's state.

use tracing::Level;

/// How much detail the log window shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LogDetail {
    Errors,
    Warnings,
    #[default]
    Info,
    Debug,
}

impl LogDetail {
    pub const ALL: [LogDetail; 4] = [
        LogDetail::Errors,
        LogDetail::Warnings,
        LogDetail::Info,
        LogDetail::Debug,
    ];

    pub fn label(self) -> &'static str {
        match self {
            LogDetail::Errors => "Errors",
            LogDetail::Warnings => "Warnings",
            LogDetail::Info => "Info",
            LogDetail::Debug => "Everything",
        }
    }

    /// The most verbose level that is still shown.
    pub fn level(self) -> Level {
        match self {
            LogDetail::Errors => Level::ERROR,
            LogDetail::Warnings => Level::WARN,
            LogDetail::Info => Level::INFO,
            LogDetail::Debug => Level::TRACE,
        }
    }

    pub fn shows(self, level: Level) -> bool {
        // In `tracing`, a more verbose level compares as greater.
        level <= self.level()
    }
}

#[derive(Default)]
pub struct LogWindow {
    pub open: bool,
    pub detail: LogDetail,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_detail_shows_its_level_and_everything_more_severe() {
        assert!(LogDetail::Errors.shows(Level::ERROR));
        assert!(!LogDetail::Errors.shows(Level::WARN));
        assert!(LogDetail::Warnings.shows(Level::ERROR) && LogDetail::Warnings.shows(Level::WARN));
        assert!(!LogDetail::Warnings.shows(Level::INFO));
        assert!(LogDetail::Info.shows(Level::INFO) && !LogDetail::Info.shows(Level::DEBUG));
        assert!(LogDetail::Debug.shows(Level::TRACE));
    }

    #[test]
    fn the_default_is_info() {
        assert_eq!(LogDetail::default(), LogDetail::Info);
        assert_eq!(LogDetail::ALL.len(), 4);
    }
}
