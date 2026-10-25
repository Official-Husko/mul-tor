//! What the window shows, independent of how it is drawn: the file list, the
//! provider selection, and the state of every upload. Everything here is plain
//! data so it can be tested without a window.

mod files;
mod filter;
mod jobs;
#[cfg(test)]
mod tests;

use std::collections::{BTreeSet, HashMap};

use mul_tor_core::availability::Health;

pub use files::FileEntry;
pub use filter::matches_filter;
pub use jobs::{JobRow, JobState};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunState {
    #[default]
    Idle,
    Running,
    Cancelling,
}

#[derive(Default)]
pub struct Model {
    pub files: Vec<FileEntry>,
    /// Ids of the ticked providers.
    pub selected: BTreeSet<String>,
    pub provider_filter: String,
    /// File stem of the chosen preset, if any.
    pub preset: Option<String>,
    pub jobs: Vec<JobRow>,
    pub run: RunState,
    pub health: HashMap<String, Health>,
    pub checking: bool,
    /// Template of the preset used for the current run.
    pub link_format: Option<String>,
}

impl Model {
    pub fn toggle_provider(&mut self, id: &str, on: bool) {
        if on {
            self.selected.insert(id.to_string());
        } else {
            self.selected.remove(id);
        }
    }

    /// Replace the selection with exactly these providers.
    pub fn select_only<'a>(&mut self, ids: impl IntoIterator<Item = &'a str>) {
        self.selected = ids.into_iter().map(str::to_string).collect();
    }

    pub fn is_running(&self) -> bool {
        self.run != RunState::Idle
    }

    pub fn can_start(&self) -> bool {
        !self.is_running() && !self.files.is_empty() && !self.selected.is_empty()
    }

    /// Forget the previous run and prepare for a new one.
    pub fn begin_run(&mut self, link_format: Option<String>) {
        self.jobs.clear();
        self.link_format = link_format;
        self.run = RunState::Running;
    }

    pub fn request_cancel(&mut self) {
        if self.run == RunState::Running {
            self.run = RunState::Cancelling;
        }
    }

    pub fn run_finished(&mut self) {
        self.run = RunState::Idle;
    }
}
