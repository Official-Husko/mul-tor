//! The third and fourth steps: watching uploads, then their results.

use std::time::{Duration, Instant};

use mul_tor_core::linklog::FinishedLink;
use mul_tor_core::{Event, UploadError};
use ratatui::widgets::ListState;

/// How much of each new speed sample is mixed into the running average.
const SPEED_WEIGHT: f64 = 0.3;

#[derive(Clone, Debug)]
pub enum JobState {
    Queued,
    Running {
        sent: u64,
        total: u64,
        message: String,
        /// Bytes per second, smoothed.
        speed: f64,
        last_sample: (Instant, u64),
    },
    Done {
        url: String,
    },
    Failed {
        error: String,
        kind: &'static str,
        /// What the provider replied, trimmed, when that helps.
        raw: Option<String>,
    },
    Cancelled,
}

#[derive(Clone, Debug)]
pub struct JobView {
    pub file: String,
    pub provider: &'static str,
    pub size: u64,
    pub state: JobState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub queued: usize,
    pub running: usize,
    pub ok: usize,
    pub failed: usize,
    pub cancelled: usize,
}

#[derive(Default)]
pub struct UploadState {
    pub jobs: Vec<JobView>,
    pub list: ListState,
    /// Ctrl-C was pressed; running uploads are being stopped.
    pub cancelling: bool,
    pub started: Option<Instant>,
    pub ended: Option<Instant>,
}

impl UploadState {
    pub fn begin(&mut self) {
        *self = Self {
            started: Some(Instant::now()),
            ..Self::default()
        };
    }

    pub fn finish(&mut self) {
        self.ended = Some(Instant::now());
    }

    pub fn is_finished(&self) -> bool {
        self.ended.is_some()
    }

    pub fn apply(&mut self, event: Event) {
        match event {
            Event::Queued {
                job,
                file_name,
                provider,
                size,
            } => {
                debug_assert_eq!(job, self.jobs.len(), "jobs are queued in order");
                self.jobs.push(JobView {
                    file: file_name,
                    provider,
                    size,
                    state: JobState::Queued,
                });
                if self.list.selected().is_none() {
                    self.list.select(Some(0));
                }
            }
            Event::Started { job } => {
                if let Some(view) = self.jobs.get_mut(job) {
                    view.state = JobState::Running {
                        sent: 0,
                        total: view.size,
                        message: String::new(),
                        speed: 0.0,
                        last_sample: (Instant::now(), 0),
                    };
                }
            }
            Event::Progress { job, progress } => {
                let Some(JobState::Running {
                    sent,
                    total,
                    message,
                    speed,
                    last_sample,
                }) = self.jobs.get_mut(job).map(|view| &mut view.state)
                else {
                    return;
                };
                let now = Instant::now();
                let seconds = now.duration_since(last_sample.0).as_secs_f64();
                if seconds > 0.0 {
                    let sample = progress.sent.saturating_sub(last_sample.1) as f64 / seconds;
                    *speed = if *speed == 0.0 {
                        sample
                    } else {
                        *speed * (1.0 - SPEED_WEIGHT) + sample * SPEED_WEIGHT
                    };
                }
                *last_sample = (now, progress.sent);
                *sent = progress.sent;
                *total = progress.total;
                *message = progress.message;
            }
            Event::Finished(outcome) => {
                let Some(view) = self.jobs.get_mut(outcome.job) else {
                    return;
                };
                view.state = match &outcome.result {
                    Ok(uploaded) => JobState::Done {
                        url: uploaded.url.clone(),
                    },
                    Err(UploadError::Cancelled) => JobState::Cancelled,
                    Err(error) => JobState::Failed {
                        error: error.to_string(),
                        kind: error.kind(),
                        raw: error.raw().map(str::to_string),
                    },
                };
            }
        }
    }

    pub fn counts(&self) -> Counts {
        let mut counts = Counts::default();
        for job in &self.jobs {
            match job.state {
                JobState::Queued => counts.queued += 1,
                JobState::Running { .. } => counts.running += 1,
                JobState::Done { .. } => counts.ok += 1,
                JobState::Failed { .. } => counts.failed += 1,
                JobState::Cancelled => counts.cancelled += 1,
            }
        }
        counts
    }

    pub fn finished_jobs(&self) -> usize {
        let counts = self.counts();
        counts.ok + counts.failed + counts.cancelled
    }

    /// `(bytes dealt with, bytes in all)`. A job that failed or was cancelled
    /// will not send more, so it counts as dealt with: the bar can reach the end.
    pub fn bytes(&self) -> (u64, u64) {
        let total = self.jobs.iter().map(|job| job.size).sum();
        let done = self
            .jobs
            .iter()
            .map(|job| match &job.state {
                JobState::Queued => 0,
                JobState::Running { sent, .. } => (*sent).min(job.size),
                _ => job.size,
            })
            .sum();
        (done, total)
    }

    /// Combined speed of everything running now, in bytes per second.
    pub fn speed(&self) -> f64 {
        self.jobs
            .iter()
            .filter_map(|job| match &job.state {
                JobState::Running { speed, .. } => Some(*speed),
                _ => None,
            })
            .sum()
    }

    /// Seconds left at the current speed; `None` while there is no speed to go by.
    pub fn eta_seconds(&self) -> Option<u64> {
        let speed = self.speed();
        if speed < 1.0 {
            return None;
        }
        let (done, total) = self.bytes();
        Some((total.saturating_sub(done) as f64 / speed).ceil() as u64)
    }

    pub fn elapsed(&self) -> Duration {
        match (self.started, self.ended) {
            (Some(start), Some(end)) => end.duration_since(start),
            (Some(start), None) => start.elapsed(),
            _ => Duration::ZERO,
        }
    }

    pub fn links(&self) -> Vec<FinishedLink<'_>> {
        self.jobs
            .iter()
            .filter_map(|job| match &job.state {
                JobState::Done { url } => Some(FinishedLink {
                    provider: job.provider,
                    file_name: &job.file,
                    url,
                }),
                _ => None,
            })
            .collect()
    }
}
