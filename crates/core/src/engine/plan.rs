//! Turning "these files, these providers" into a list of jobs.

use std::path::PathBuf;
use std::sync::Arc;

use super::Engine;
use super::event::{Event, EventSink, JobId, JobOutcome};
use crate::error::UploadError;
use crate::provider::Provider;
use crate::upload::{LocalFile, UploadedFile};

pub(super) struct Job {
    pub id: JobId,
    /// `Err` if the file could not be opened: kind and message.
    pub file: Result<Arc<LocalFile>, (std::io::ErrorKind, String)>,
    pub file_name: String,
    pub provider: Arc<dyn Provider>,
}

impl Engine {
    pub(super) async fn plan(
        &self,
        files: Vec<PathBuf>,
        providers: &[Arc<dyn Provider>],
        sink: &EventSink,
    ) -> Vec<Job> {
        let mut jobs = Vec::with_capacity(files.len() * providers.len());
        for path in files {
            let local = match LocalFile::open(&path).await {
                Ok(file) => Ok(Arc::new(file)),
                Err(UploadError::Io(error)) => Err((error.kind(), error.to_string())),
                Err(other) => Err((std::io::ErrorKind::Other, other.to_string())),
            };
            let (file_name, size) = match &local {
                Ok(file) => (file.name.clone(), file.size),
                Err(_) => (
                    path.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string()),
                    0,
                ),
            };
            for provider in providers {
                let id = jobs.len();
                sink(Event::Queued {
                    job: id,
                    file_name: file_name.clone(),
                    provider: provider.info().name,
                    size,
                });
                jobs.push(Job {
                    id,
                    file: local.clone(),
                    file_name: file_name.clone(),
                    provider: provider.clone(),
                });
            }
        }
        jobs
    }
}

pub(super) fn finish(
    sink: &EventSink,
    job: &Job,
    result: Result<UploadedFile, UploadError>,
) -> Arc<JobOutcome> {
    let outcome = Arc::new(JobOutcome {
        job: job.id,
        file_name: job.file_name.clone(),
        provider: job.provider.info().name,
        result,
    });
    sink(Event::Finished(outcome.clone()));
    outcome
}
