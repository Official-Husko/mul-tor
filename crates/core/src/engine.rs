//! Runs uploads: every file to every selected provider, reporting as it goes.
//!
//! The engine is the one piece the CLI and the GUI share. It knows nothing
//! about terminals or windows; it reports through an [`EventSink`] callback.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::Local;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::error::UploadError;
use crate::http::HttpFactory;
use crate::linklog::{LinkEntry, LinkLog};
use crate::progress::{Progress, ProgressSnapshot};
use crate::provider::{Credentials, Provider};
use crate::upload::{LocalFile, UploadContext, UploadedFile};

/// Position of a job in the run: files in the order given, providers in the order given.
pub type JobId = usize;

/// How often progress is sampled and reported.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(150);

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

#[derive(Clone)]
pub struct Engine {
    http: HttpFactory,
    credentials: Arc<BTreeMap<String, Credentials>>,
    parallel: usize,
    link_log: Option<LinkLog>,
}

impl Engine {
    pub fn new(http: HttpFactory) -> Self {
        Self {
            http,
            credentials: Arc::default(),
            parallel: 1,
            link_log: None,
        }
    }

    /// Credentials by provider id (see [`crate::Config::credentials_by_id`]).
    pub fn with_credentials(mut self, credentials: BTreeMap<String, Credentials>) -> Self {
        self.credentials = Arc::new(credentials);
        self
    }

    /// How many uploads may run at once (at least 1).
    pub fn with_parallelism(mut self, parallel: usize) -> Self {
        self.parallel = parallel.max(1);
        self
    }

    /// Record every successful upload.
    pub fn with_link_log(mut self, log: Option<LinkLog>) -> Self {
        self.link_log = log;
        self
    }

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

    async fn plan(
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

    async fn run_job(
        &self,
        job: Job,
        sink: &EventSink,
        cancel: &CancellationToken,
    ) -> Arc<JobOutcome> {
        if cancel.is_cancelled() {
            return finish(sink, &job, Err(UploadError::Cancelled));
        }
        sink(Event::Started { job: job.id });
        let result = self.execute(&job, sink, cancel).await;

        let info = job.provider.info();
        match &result {
            Ok(uploaded) => {
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
            Err(UploadError::Cancelled) => {}
            Err(error) => {
                tracing::warn!(
                    provider = info.name,
                    file = %job.file_name,
                    %error,
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

struct Job {
    id: JobId,
    /// `Err` if the file could not be opened: kind and message.
    file: Result<Arc<LocalFile>, (std::io::ErrorKind, String)>,
    file_name: String,
    provider: Arc<dyn Provider>,
}

fn finish(
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
