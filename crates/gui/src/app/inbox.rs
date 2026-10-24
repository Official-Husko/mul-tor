//! What the background tasks report, folded into the app's state.

use std::path::PathBuf;

use super::MulTorApp;
use crate::tasks::{Level, Msg};

impl MulTorApp {
    /// Handle everything the background tasks have reported since last frame.
    pub fn pump(&mut self) {
        while let Ok(msg) = self.inbox.try_recv() {
            match msg {
                Msg::Engine(event) => self.model.apply(event),
                Msg::RunFinished => {
                    self.model.run_finished();
                    self.cancel = None;
                    let counts = self.model.counts();
                    let (level, text) = if counts.failed > 0 {
                        (
                            Level::Error,
                            format!(
                                "Finished: {} uploaded, {} failed",
                                counts.done, counts.failed
                            ),
                        )
                    } else if counts.cancelled > 0 {
                        (
                            Level::Info,
                            format!("Cancelled: {} uploaded before stopping", counts.done),
                        )
                    } else {
                        (Level::Info, format!("Finished: {} uploaded", counts.done))
                    };
                    self.show_notice(level, text);
                }
                Msg::FilesPicked(paths) => self.add_paths(&paths),
                Msg::Availability(results) => {
                    self.model.checking = false;
                    self.model.health = results.into_iter().collect();
                }
                Msg::UpdateAvailable(release) => self.update = Some(release),
                Msg::Notice(level, text) => self.show_notice(level, text),
            }
        }
    }

    pub fn add_paths(&mut self, paths: &[PathBuf]) {
        let problems = self.model.add_paths(paths);
        if let Some((path, reason)) = problems.first() {
            let more = if problems.len() > 1 {
                format!(" (and {} more)", problems.len() - 1)
            } else {
                String::new()
            };
            self.show_notice(Level::Error, format!("{}: {reason}{more}", path.display()));
        }
    }
}
