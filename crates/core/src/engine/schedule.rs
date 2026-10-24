//! Running the planned jobs: bounded parallelism, cancellation, panic isolation.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::Instrument;

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
        let span = tracing::info_span!(
            "run",
            files = files.len(),
            providers = providers.len(),
            parallel = self.parallel
        );
        self.run_jobs(files, providers, sink, cancel)
            .instrument(span)
            .await
    }

    async fn run_jobs(
        &self,
        files: Vec<PathBuf>,
        providers: Vec<Arc<dyn Provider>>,
        sink: EventSink,
        cancel: CancellationToken,
    ) -> Vec<Arc<JobOutcome>> {
        let started = Instant::now();
        let jobs = self.plan(files, &providers, &sink).await;
        tracing::info!(jobs = jobs.len(), "run started");

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
            // A spawned task does not inherit the current span; hand it over.
            let span = tracing::info_span!(
                "job",
                id = job.id,
                provider = identity.2,
                file = %job.file_name
            );
            let handle = tokio::spawn(
                async move {
                    let outcome = engine.run_job(job, &sink, &cancel).await;
                    drop(permit);
                    outcome
                }
                .instrument(span),
            );
            running.push((identity, handle));
        }

        for ((job, file_name, provider), handle) in running {
            outcomes.push(match handle.await {
                Ok(outcome) => outcome,
                // An uploader panicked. Report it as a failed job instead of
                // taking the whole run down.
                Err(error) => {
                    tracing::error!(job, provider, file = %file_name, %error, "the uploader crashed");
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
        let succeeded = outcomes.iter().filter(|o| o.is_ok()).count();
        let cancelled = outcomes
            .iter()
            .filter(|o| matches!(o.result, Err(UploadError::Cancelled)))
            .count();
        tracing::info!(
            succeeded,
            failed = outcomes.len() - succeeded - cancelled,
            cancelled,
            elapsed_ms = started.elapsed().as_millis() as u64,
            "run finished"
        );
        outcomes
    }
}
