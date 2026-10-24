//! What the engine reports while it works.

use std::sync::Arc;

use crate::error::UploadError;
use crate::progress::ProgressSnapshot;
use crate::upload::UploadedFile;

/// Position of a job in the run: files in the order given, providers in the order given.
pub type JobId = usize;

#[derive(Debug)]
pub struct JobOutcome {
    pub job: JobId,
    pub file_name: String,
    pub provider: &'static str,
    pub result: Result<UploadedFile, UploadError>,
}

impl JobOutcome {
    pub fn is_ok(&self) -> bool {
        self.result.is_ok()
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    /// A job exists. Sent for every job before any of them starts.
    Queued {
        job: JobId,
        file_name: String,
        provider: &'static str,
        size: u64,
    },
    Started {
        job: JobId,
    },
    Progress {
        job: JobId,
        progress: ProgressSnapshot,
    },
    /// Last event of a job, whether it worked or not.
    Finished(Arc<JobOutcome>),
}

/// Receives events from any thread; it must not block.
pub type EventSink = Arc<dyn Fn(Event) + Send + Sync>;
