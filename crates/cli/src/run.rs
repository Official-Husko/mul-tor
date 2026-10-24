//! Running an upload: engine setup, Ctrl-C handling, progress, summary.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Result, bail};
use console::style;
use mul_tor_core::availability;
use mul_tor_core::{Engine, HttpFactory, LinkLog, Site};
use tokio_util::sync::CancellationToken;

use crate::app::App;
use crate::render::Renderer;

pub struct Job {
    pub files: Vec<PathBuf>,
    pub sites: Vec<Arc<dyn Site>>,
    pub link_format: Option<String>,
    pub json: bool,
    pub parallel: usize,
    pub save_links: bool,
    pub links_dir: PathBuf,
}

/// Drop sites that do not answer. `strict` sites (picked by name) are only
/// warned about, since the probe can be wrong about a site the user asked for.
pub async fn filter_reachable(
    factory: &HttpFactory,
    sites: Vec<Arc<dyn Site>>,
    named: &[String],
    quiet: bool,
) -> Vec<Arc<dyn Site>> {
    if !quiet {
        eprint!(
            "{}",
            style(format!("Checking {} sites...", sites.len())).yellow()
        );
    }
    let statuses = availability::check(factory, &sites).await;
    if !quiet {
        eprint!("\r{}\r", " ".repeat(40));
    }

    let mut kept = Vec::new();
    for status in statuses {
        let info = status.site.info();
        match &status.health {
            availability::Health::Up => kept.push(status.site),
            availability::Health::Down(reason) => {
                let asked_for_it = named.iter().any(|name| info.matches(name));
                eprintln!(
                    "{} {} did not answer ({reason}){}",
                    style("[!]").yellow(),
                    info.name,
                    if asked_for_it {
                        "; trying anyway"
                    } else {
                        "; skipping it"
                    }
                );
                if asked_for_it {
                    kept.push(status.site);
                }
            }
        }
    }
    kept
}

pub async fn execute(app: &App, factory: HttpFactory, job: Job) -> Result<ExitCode> {
    if job.files.is_empty() {
        bail!("there is nothing to upload");
    }
    if job.sites.is_empty() {
        bail!("there is no site to upload to");
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
    let watcher = tokio::spawn(watch_for_ctrl_c(cancel.clone()));

    let outcomes = engine
        .run(job.files, job.sites, renderer.sink(), cancel.clone())
        .await;
    watcher.abort();
    renderer.finish();

    let summary = renderer.summary();
    if !job.json {
        println!();
        let mut parts = vec![format!("{} uploaded", style(summary.ok).green().bold())];
        if summary.failed > 0 {
            parts.push(format!("{} failed", style(summary.failed).red().bold()));
        }
        if summary.cancelled > 0 {
            parts.push(format!(
                "{} cancelled",
                style(summary.cancelled).yellow().bold()
            ));
        }
        println!("{}", parts.join(", "));
    }
    debug_assert_eq!(
        outcomes.len(),
        summary.ok + summary.failed + summary.cancelled
    );

    Ok(if cancel.is_cancelled() {
        // Conventional exit status for a run ended with Ctrl-C.
        ExitCode::from(130)
    } else if summary.failed > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

/// First Ctrl-C cancels the run cleanly; a second one exits immediately.
async fn watch_for_ctrl_c(cancel: CancellationToken) {
    if tokio::signal::ctrl_c().await.is_err() {
        return;
    }
    eprintln!(
        "\n{}",
        style("Cancelling... (press Ctrl-C again to quit now)").yellow()
    );
    cancel.cancel();
    if tokio::signal::ctrl_c().await.is_ok() {
        std::process::exit(130);
    }
}
