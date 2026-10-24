//! The one-line status message at the bottom of the window.

use std::time::{Duration, Instant};

use super::MulTorApp;
use crate::tasks::Level;

/// How long a status message stays visible.
const NOTICE_LIFETIME: Duration = Duration::from_secs(10);

pub struct Notice {
    pub level: Level,
    pub text: String,
    at: Instant,
}

impl MulTorApp {
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
}
