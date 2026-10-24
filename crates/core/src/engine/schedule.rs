//! Running the planned jobs: bounded parallelism, cancellation, panic isolation.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use super::Engine;
use super::event::{Event, EventSink, JobOutcome};
use super::plan::finish;
use crate::error::UploadError;
use crate::provider::Provider;

impl Engine {
    /// Upload each of `files` to each of `providers`.
    ///
    /// Jobs start in file-major order (all providers for the first file, then the
    /// second file, …), at most `parallelism` at a time. Cancelling `cancel`
    /// aborts running jobs and marks the ones not yet started as cancelled.
    /// Returns every job's outcome in job order.
    pub async fn run(
        &self,
        files: Vec<PathBuf>,
        providers: Vec<Arc<dyn Provider>>,
        sink: EventSink,
        cancel: CancellationToken,
    ) -> Vec<Arc<JobOutcome>> {
        let jobs = self.plan(files, &providers, &sink).await;

        let permits = Arc::new(Semaphore::new(self.parallel));
        let mut running = Vec::with_capacity(jobs.len());
        let mut outcomes = Vec::with_capacity(jobs.len());

        for job in jobs {
            let permit = tokio::select! {
                biased;
                _ = cancel.cancelled() => None,
                permit = permits.clone().acquire_owned() => permit.ok(),
            };
            let Some(permit) = permit else {
                outcomes.push(finish(&sink, &job, Err(UploadError::Cancelled)));
                continue;
            };

            let engine = self.clone();
            let sink = sink.clone();
            let cancel = cancel.clone();
            let identity = (job.id, job.file_name.clone(), job.provider.info().name);
            let handle = tokio::spawn(async move {
                let outcome = engine.run_job(job, &sink, &cancel).await;
                drop(permit);
                outcome
            });
            running.push((identity, handle));
        }

        for ((job, file_name, provider), handle) in running {
            outcomes.push(match handle.await {
                Ok(outcome) => outcome,
                // An uploader panicked. Report it as a failed job instead of
                // taking the whole run down.
                Err(error) => {
                    let outcome = Arc::new(JobOutcome {
                        job,
                        file_name,
                        provider,
                        result: Err(UploadError::unexpected(format!(
                            "internal error: the uploader crashed ({error})"
                        ))),
                    });
                    sink(Event::Finished(outcome.clone()));
                    outcome
                }
            });
        }

        outcomes.sort_by_key(|outcome| outcome.job);
        outcomes
    }
}
