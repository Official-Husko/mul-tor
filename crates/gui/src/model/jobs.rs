//! The state of every upload, built up from engine events.

use mul_tor_core::linklog::{FinishedLink, render_links};
use mul_tor_core::{Event, JobId, ProgressSnapshot, UploadError};

use super::Model;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running(ProgressSnapshot),
    Done { url: String },
    Failed { kind: &'static str, message: String },
    Cancelled,
}

impl JobState {
    pub fn is_finished(&self) -> bool {
        matches!(
            self,
            JobState::Done { .. } | JobState::Failed { .. } | JobState::Cancelled
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobRow {
    pub id: JobId,
    pub file: String,
    pub provider: &'static str,
    pub size: u64,
    pub state: JobState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub queued: usize,
    pub running: usize,
    pub done: usize,
    pub failed: usize,
    pub cancelled: usize,
}

impl Model {
    /// Fold one engine event into the job table.
    pub fn apply(&mut self, event: Event) {
        match event {
            Event::Queued {
                job,
                file_name,
                provider,
                size,
            } => {
                self.jobs.push(JobRow {
                    id: job,
                    file: file_name,
                    provider,
                    size,
                    state: JobState::Queued,
                });
            }
            Event::Started { job } => {
                if let Some(row) = self.job_mut(job) {
                    row.state = JobState::Running(ProgressSnapshot {
                        total: row.size,
                        ..ProgressSnapshot::default()
                    });
                }
            }
            Event::Progress { job, progress } => {
                if let Some(row) = self.job_mut(job) {
                    // A late sample must not resurrect a finished job.
                    if !row.state.is_finished() {
                        row.state = JobState::Running(progress);
                    }
                }
            }
            Event::Finished(outcome) => {
                if let Some(row) = self.job_mut(outcome.job) {
                    row.state = match &outcome.result {
                        Ok(uploaded) => JobState::Done {
                            url: uploaded.url.clone(),
                        },
                        Err(UploadError::Cancelled) => JobState::Cancelled,
                        Err(error) => JobState::Failed {
                            kind: error.kind(),
                            message: error.to_string(),
                        },
                    };
                }
            }
        }
    }

    fn job_mut(&mut self, id: JobId) -> Option<&mut JobRow> {
        // Jobs arrive in id order, so the id is normally also the index.
        let index = if self.jobs.get(id).is_some_and(|row| row.id == id) {
            Some(id)
        } else {
            self.jobs.iter().position(|row| row.id == id)
        };
        index.map(|index| &mut self.jobs[index])
    }

    pub fn counts(&self) -> Counts {
        let mut counts = Counts::default();
        for job in &self.jobs {
            match job.state {
                JobState::Queued => counts.queued += 1,
                JobState::Running(_) => counts.running += 1,
                JobState::Done { .. } => counts.done += 1,
                JobState::Failed { .. } => counts.failed += 1,
                JobState::Cancelled => counts.cancelled += 1,
            }
        }
        counts
    }

    /// Overall progress 0..=1 across all jobs, weighting by file size.
    pub fn overall_fraction(&self) -> f32 {
        let total: u64 = self.jobs.iter().map(|j| j.size.max(1)).sum();
        if total == 0 {
            return 0.0;
        }
        let done: u64 = self
            .jobs
            .iter()
            .map(|job| match &job.state {
                JobState::Done { .. } => job.size.max(1),
                JobState::Running(p) => p.sent.min(job.size.max(1)),
                _ => 0,
            })
            .sum();
        (done as f64 / total as f64) as f32
    }

    /// Links of the finished uploads, one per line: plain URLs, or the preset's
    /// template when one is set.
    pub fn links_text(&self, formatted: bool) -> String {
        let template = formatted.then_some(self.link_format.as_deref()).flatten();
        let links = self.jobs.iter().filter_map(|job| match &job.state {
            JobState::Done { url } => Some(FinishedLink {
                provider: job.provider,
                file_name: &job.file,
                url,
            }),
            _ => None,
        });
        render_links(links, template)
    }

    pub fn has_links(&self) -> bool {
        self.jobs
            .iter()
            .any(|j| matches!(j.state, JobState::Done { .. }))
    }
}
