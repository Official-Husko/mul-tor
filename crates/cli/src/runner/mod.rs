//! Running an upload from the command line: engine setup, Ctrl-C handling,
//! progress, summary.

mod interrupt;
mod reachable;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Result, bail};
use mul_tor_core::{Engine, HttpFactory, LinkLog, Provider};
use tokio_util::sync::CancellationToken;

use crate::app::App;
use crate::output::Renderer;

pub use reachable::filter_reachable;

pub struct Job {
    pub files: Vec<PathBuf>,
    pub providers: Vec<Arc<dyn Provider>>,
    pub link_format: Option<String>,
    pub json: bool,
    pub parallel: usize,
    pub save_links: bool,
    pub links_dir: PathBuf,
}

pub async fn execute(app: &App, factory: HttpFactory, job: Job) -> Result<ExitCode> {
    if job.files.is_empty() {
        bail!("there is nothing to upload");
    }
    if job.providers.is_empty() {
        bail!("there is no provider to upload to");
    }

    let link_log = job
        .save_links
        .then(|| LinkLog::new(job.links_dir).with_template(job.link_format));
    let engine = Engine::new(factory)
        .with_credentials(app.config.credentials_by_id())
        .with_parallelism(job.parallel)
        .with_link_log(link_log);

    let renderer = Renderer::new(job.json);
    let cancel = CancellationToken::new();
    let watcher = tokio::spawn(interrupt::watch_for_ctrl_c(cancel.clone()));

    let outcomes = engine
        .run(job.files, job.providers, renderer.sink(), cancel.clone())
        .await;
    watcher.abort();
    renderer.finish();

    let summary = renderer.summary();
    if !job.json {
        println!();
        summary.print();
    }
    debug_assert_eq!(
        outcomes.len(),
        summary.ok + summary.failed + summary.cancelled
    );

    Ok(summary.exit_code(cancel.is_cancelled()))
}
