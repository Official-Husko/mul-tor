//! One upload from start to finish.

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Local;
use tokio_util::sync::CancellationToken;

use super::Engine;
use super::event::{Event, EventSink, JobOutcome};
use super::plan::{Job, finish};
use crate::error::UploadError;
use crate::linklog::LinkEntry;
use crate::progress::{Progress, ProgressSnapshot};
use crate::upload::{UploadContext, UploadedFile};

/// How often progress is sampled and reported.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(150);

impl Engine {
    pub(super) async fn run_job(
        &self,
        job: Job,
        sink: &EventSink,
        cancel: &CancellationToken,
    ) -> Arc<JobOutcome> {
        if cancel.is_cancelled() {
            return finish(sink, &job, Err(UploadError::Cancelled));
        }
        sink(Event::Started { job: job.id });
        let started = Instant::now();
        let result = self.execute(&job, sink, cancel).await;
        let elapsed_ms = started.elapsed().as_millis() as u64;

        let info = job.provider.info();
        match &result {
            Ok(uploaded) => {
                tracing::info!(elapsed_ms, "upload finished");
                // The link itself can be private; it is in the link file, not here.
                tracing::debug!(link = %uploaded.url, "link received");
                if let Some(log) = &self.link_log {
                    let entry = LinkEntry {
                        when: Local::now(),
                        provider: info.name,
                        file_name: &job.file_name,
                        url: &uploaded.url,
                    };
                    if let Err(error) = log.record(&entry) {
                        tracing::warn!(%error, "could not write the link file");
                    }
                }
            }
            Err(UploadError::Cancelled) => tracing::info!(elapsed_ms, "upload cancelled"),
            Err(error) => {
                tracing::warn!(
                    kind = error.kind(),
                    %error,
                    elapsed_ms,
                    raw = error.raw().unwrap_or(""),
                    "upload failed"
                );
            }
        }
        finish(sink, &job, result)
    }

    async fn execute(
        &self,
        job: &Job,
        sink: &EventSink,
        cancel: &CancellationToken,
    ) -> Result<UploadedFile, UploadError> {
        let file = job.file.as_ref().map_err(|(kind, message)| {
            UploadError::Io(std::io::Error::new(*kind, message.clone()))
        })?;
        let info = job.provider.info();
        let credentials = self
            .credentials
            .get(&info.id())
            .cloned()
            .unwrap_or_default();
        info.precheck(file, &credentials)?;
        tracing::info!(size = file.size, "upload started");

        let http = self.http.client()?;
        let progress = Progress::new(file.size);
        let cx = UploadContext {
            file,
            http: &http,
            credentials: &credentials,
            progress: &progress,
        };

        // Sample progress while the upload runs.
        let report = async {
            let mut ticker = tokio::time::interval(PROGRESS_INTERVAL);
            let mut last = ProgressSnapshot::default();
            loop {
                ticker.tick().await;
                let now = progress.snapshot();
                if now != last {
                    sink(Event::Progress {
                        job: job.id,
                        progress: now.clone(),
                    });
                    last = now;
                }
            }
        };

        let result = tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(UploadError::Cancelled),
            result = job.provider.upload(&cx) => result,
            _ = report => unreachable!("the progress reporter never finishes"),
        };

        // One last sample so the final state is not lost between ticks.
        sink(Event::Progress {
            job: job.id,
            progress: progress.snapshot(),
        });
        result
    }
}
