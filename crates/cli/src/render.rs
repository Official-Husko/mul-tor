//! Showing upload progress and results, either for a person (progress bars,
//! coloured lines) or for a script (one JSON object per finished upload).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use mul_tor_core::{Event, EventSink, JobId, JobOutcome, UploadError};
use serde_json::json;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub ok: usize,
    pub failed: usize,
    pub cancelled: usize,
}

pub struct Renderer {
    json: bool,
    bars: MultiProgress,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    overall: Option<ProgressBar>,
    bars: HashMap<JobId, ProgressBar>,
    /// File size per job, from the `Queued` event.
    sizes: HashMap<JobId, u64>,
    summary: Summary,
}

impl Renderer {
    pub fn new(json: bool) -> Arc<Self> {
        let bars = if json {
            MultiProgress::with_draw_target(ProgressDrawTarget::hidden())
        } else {
            MultiProgress::new()
        };
        Arc::new(Self {
            json,
            bars,
            state: Mutex::default(),
        })
    }

    pub fn sink(self: &Arc<Self>) -> EventSink {
        let renderer = self.clone();
        Arc::new(move |event| renderer.handle(event))
    }

    pub fn summary(&self) -> Summary {
        self.lock().summary
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn handle(&self, event: Event) {
        let mut state = self.lock();
        match event {
            Event::Queued { job, size, .. } => {
                state.sizes.insert(job, size);
                if !self.json {
                    let overall = state.overall.get_or_insert_with(|| {
                        let bar = self.bars.add(ProgressBar::new(0));
                        bar.set_style(
                            ProgressStyle::with_template("{prefix:.bold} {pos}/{len} uploads")
                                .expect("valid template"),
                        );
                        bar.set_prefix("Overall");
                        bar
                    });
                    overall.inc_length(1);
                }
            }
            Event::Started { job } => {
                if self.json {
                    return;
                }
                // Created lazily so queued-but-not-running jobs do not clutter the screen.
                let size = state.sizes.get(&job).copied().unwrap_or(0);
                let bar = self.bars.add(ProgressBar::new(size));
                bar.set_style(job_style());
                bar.enable_steady_tick(Duration::from_millis(120));
                state.bars.insert(job, bar);
            }
            Event::Progress { job, progress } => {
                if let Some(bar) = state.bars.get(&job) {
                    bar.set_length(progress.total);
                    bar.set_position(progress.sent);
                    bar.set_message(progress.message);
                }
            }
            Event::Finished(outcome) => {
                if let Some(bar) = state.bars.remove(&outcome.job) {
                    bar.finish_and_clear();
                    self.bars.remove(&bar);
                }
                if let Some(overall) = &state.overall {
                    overall.inc(1);
                }
                match &outcome.result {
                    Ok(_) => state.summary.ok += 1,
                    Err(UploadError::Cancelled) => state.summary.cancelled += 1,
                    Err(_) => state.summary.failed += 1,
                }
                // Cancelled jobs are summarised once at the end instead.
                if !matches!(outcome.result, Err(UploadError::Cancelled)) {
                    let line = self.format_outcome(&outcome);
                    // `suspend` keeps the bars from garbling the line.
                    self.bars.suspend(|| println!("{line}"));
                }
            }
        }
    }

    fn format_outcome(&self, outcome: &JobOutcome) -> String {
        if self.json {
            return json_line(outcome);
        }
        let file = style(&outcome.file_name).cyan();
        let site = style(outcome.site).yellow();
        match &outcome.result {
            Ok(uploaded) => format!(
                "{} {file} {} {site}{} {}",
                style("[+]").green(),
                style("uploaded to").green(),
                style("! URL:").green(),
                style(&uploaded.url).cyan().underlined()
            ),
            Err(error) => format!(
                "{} {file} to {site} {}",
                style("[!]").red(),
                style(format!("failed: {error}")).red()
            ),
        }
    }

    /// Clear the bars. Call once the run is over.
    pub fn finish(&self) {
        let mut state = self.lock();
        for (_, bar) in state.bars.drain() {
            bar.finish_and_clear();
        }
        if let Some(overall) = state.overall.take() {
            overall.finish_and_clear();
        }
    }
}

fn job_style() -> ProgressStyle {
    ProgressStyle::with_template(
        "{spinner:.green} {bar:28.cyan/blue} {bytes:>9}/{total_bytes:<9} {bytes_per_sec:>11}  {msg}",
    )
    .expect("valid template")
    .progress_chars("█▓░")
}

/// One finished upload as a line of JSON.
pub fn json_line(outcome: &JobOutcome) -> String {
    let value = match &outcome.result {
        Ok(uploaded) => json!({
            "file": outcome.file_name,
            "site": outcome.site,
            "status": "ok",
            "url": uploaded.url,
        }),
        Err(error) => json!({
            "file": outcome.file_name,
            "site": outcome.site,
            "status": "error",
            "kind": error.kind(),
            "error": error.to_string(),
        }),
    };
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mul_tor_core::UploadedFile;

    fn outcome(result: Result<UploadedFile, UploadError>) -> JobOutcome {
        JobOutcome {
            job: 0,
            file_name: "a \"b\".zip".into(),
            site: "Pixeldrain",
            result,
        }
    }

    #[test]
    fn json_lines_for_success() {
        let line = json_line(&outcome(Ok(UploadedFile::new("https://x/1"))));
        let value: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["status"], "ok");
        assert_eq!(value["url"], "https://x/1");
        assert_eq!(value["file"], "a \"b\".zip");
        assert!(!line.contains('\n'));
    }

    #[test]
    fn json_lines_for_failure_carry_a_stable_kind() {
        let line = json_line(&outcome(Err(UploadError::TooLarge {
            limit: "20 GB".into(),
        })));
        let value: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["status"], "error");
        assert_eq!(value["kind"], "too_large");
        assert!(value["error"].as_str().unwrap().contains("20 GB"));
    }

    #[test]
    fn the_summary_counts_each_kind_of_result() {
        let renderer = Renderer::new(true);
        let sink = renderer.sink();
        let finish = |result| sink(Event::Finished(Arc::new(outcome(result))));
        finish(Ok(UploadedFile::new("u")));
        finish(Err(UploadError::EmptyFile));
        finish(Err(UploadError::Cancelled));
        finish(Ok(UploadedFile::new("u")));
        assert_eq!(
            renderer.summary(),
            Summary {
                ok: 2,
                failed: 1,
                cancelled: 1
            }
        );
    }
}
