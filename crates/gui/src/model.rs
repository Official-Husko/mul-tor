//! What the window shows, independent of how it is drawn: the file list, the
//! provider selection, and the state of every upload. Everything here is plain
//! data so it can be tested without a window.

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

use mul_tor_core::availability::Health;
use mul_tor_core::files::scan_inputs;
use mul_tor_core::linklog::format_template;
use mul_tor_core::{Event, JobId, ProgressSnapshot, Provider, UploadError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running(ProgressSnapshot),
    Done { url: String },
    Failed { kind: &'static str, message: String },
    Cancelled,
}

impl JobState {
    pub fn is_finished(&self) -> bool {
        matches!(
            self,
            JobState::Done { .. } | JobState::Failed { .. } | JobState::Cancelled
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobRow {
    pub id: JobId,
    pub file: String,
    pub provider: &'static str,
    pub size: u64,
    pub state: JobState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunState {
    #[default]
    Idle,
    Running,
    Cancelling,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub queued: usize,
    pub running: usize,
    pub done: usize,
    pub failed: usize,
    pub cancelled: usize,
}

#[derive(Default)]
pub struct Model {
    pub files: Vec<FileEntry>,
    /// Ids of the ticked providers.
    pub selected: BTreeSet<String>,
    pub provider_filter: String,
    /// File stem of the chosen preset, if any.
    pub preset: Option<String>,
    pub jobs: Vec<JobRow>,
    pub run: RunState,
    pub health: HashMap<String, Health>,
    pub checking: bool,
    /// Template of the preset used for the current run.
    pub link_format: Option<String>,
}

impl Model {
    /// Add files (folders contribute the files directly inside them).
    /// Returns the inputs that could not be used.
    pub fn add_paths(&mut self, paths: &[PathBuf]) -> Vec<(PathBuf, String)> {
        let scan = scan_inputs(paths, false);
        for path in scan.files {
            if self.files.iter().any(|f| f.path == path) {
                continue;
            }
            let Ok(meta) = std::fs::metadata(&path) else {
                continue;
            };
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            self.files.push(FileEntry {
                path,
                name,
                size: meta.len(),
            });
        }
        scan.problems
    }

    pub fn remove_file(&mut self, index: usize) {
        if index < self.files.len() {
            self.files.remove(index);
        }
    }

    pub fn total_size(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }

    pub fn toggle_provider(&mut self, id: &str, on: bool) {
        if on {
            self.selected.insert(id.to_string());
        } else {
            self.selected.remove(id);
        }
    }

    /// Replace the selection with exactly these providers.
    pub fn select_only<'a>(&mut self, ids: impl IntoIterator<Item = &'a str>) {
        self.selected = ids.into_iter().map(str::to_string).collect();
    }

    pub fn is_running(&self) -> bool {
        self.run != RunState::Idle
    }

    pub fn can_start(&self) -> bool {
        !self.is_running() && !self.files.is_empty() && !self.selected.is_empty()
    }

    /// Forget the previous run and prepare for a new one.
    pub fn begin_run(&mut self, link_format: Option<String>) {
        self.jobs.clear();
        self.link_format = link_format;
        self.run = RunState::Running;
    }

    pub fn request_cancel(&mut self) {
        if self.run == RunState::Running {
            self.run = RunState::Cancelling;
        }
    }

    pub fn run_finished(&mut self) {
        self.run = RunState::Idle;
    }

    /// Fold one engine event into the job table.
    pub fn apply(&mut self, event: Event) {
        match event {
            Event::Queued {
                job,
                file_name,
                provider,
                size,
            } => {
                self.jobs.push(JobRow {
                    id: job,
                    file: file_name,
                    provider,
                    size,
                    state: JobState::Queued,
                });
            }
            Event::Started { job } => {
                if let Some(row) = self.job_mut(job) {
                    row.state = JobState::Running(ProgressSnapshot {
                        total: row.size,
                        ..ProgressSnapshot::default()
                    });
                }
            }
            Event::Progress { job, progress } => {
                if let Some(row) = self.job_mut(job) {
                    // A late sample must not resurrect a finished job.
                    if !row.state.is_finished() {
                        row.state = JobState::Running(progress);
                    }
                }
            }
            Event::Finished(outcome) => {
                if let Some(row) = self.job_mut(outcome.job) {
                    row.state = match &outcome.result {
                        Ok(uploaded) => JobState::Done {
                            url: uploaded.url.clone(),
                        },
                        Err(UploadError::Cancelled) => JobState::Cancelled,
                        Err(error) => JobState::Failed {
                            kind: error.kind(),
                            message: error.to_string(),
                        },
                    };
                }
            }
        }
    }

    fn job_mut(&mut self, id: JobId) -> Option<&mut JobRow> {
        // Jobs arrive in id order, so the id is normally also the index.
        let index = if self.jobs.get(id).is_some_and(|row| row.id == id) {
            Some(id)
        } else {
            self.jobs.iter().position(|row| row.id == id)
        };
        index.map(|index| &mut self.jobs[index])
    }

    pub fn counts(&self) -> Counts {
        let mut counts = Counts::default();
        for job in &self.jobs {
            match job.state {
                JobState::Queued => counts.queued += 1,
                JobState::Running(_) => counts.running += 1,
                JobState::Done { .. } => counts.done += 1,
                JobState::Failed { .. } => counts.failed += 1,
                JobState::Cancelled => counts.cancelled += 1,
            }
        }
        counts
    }

    /// Overall progress 0..=1 across all jobs, weighting by file size.
    pub fn overall_fraction(&self) -> f32 {
        let total: u64 = self.jobs.iter().map(|j| j.size.max(1)).sum();
        if total == 0 {
            return 0.0;
        }
        let done: u64 = self
            .jobs
            .iter()
            .map(|job| match &job.state {
                JobState::Done { .. } => job.size.max(1),
                JobState::Running(p) => p.sent.min(job.size.max(1)),
                _ => 0,
            })
            .sum();
        (done as f64 / total as f64) as f32
    }

    /// Links of the finished uploads, one per line: plain URLs, or the preset's
    /// template when one is set.
    pub fn links_text(&self, formatted: bool) -> String {
        let template = formatted.then_some(self.link_format.as_deref()).flatten();
        self.jobs
            .iter()
            .filter_map(|job| match &job.state {
                JobState::Done { url } => Some(match template {
                    Some(template) => format_template(
                        template,
                        &[
                            ("status", "ok"),
                            ("file_name", &job.file),
                            ("file_url", url),
                            ("provider_name", job.provider),
                            ("date_and_time", ""),
                        ],
                    ),
                    None => url.clone(),
                }),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn has_links(&self) -> bool {
        self.jobs
            .iter()
            .any(|j| matches!(j.state, JobState::Done { .. }))
    }
}

/// Does `provider` match the user's search text (name or alias)?
pub fn matches_filter(provider: &dyn Provider, filter: &str) -> bool {
    let filter = filter.trim().to_lowercase();
    if filter.is_empty() {
        return true;
    }
    let info = provider.info();
    info.name.to_lowercase().contains(&filter)
        || info
            .aliases
            .iter()
            .any(|alias| alias.to_lowercase().contains(&filter))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use mul_tor_core::{JobOutcome, Registry, UploadedFile};

    fn queued(job: JobId, file: &str, provider: &'static str, size: u64) -> Event {
        Event::Queued {
            job,
            file_name: file.into(),
            provider,
            size,
        }
    }

    fn finished(job: JobId, result: Result<UploadedFile, UploadError>) -> Event {
        Event::Finished(Arc::new(JobOutcome {
            job,
            file_name: "f".into(),
            provider: "S",
            result,
        }))
    }

    fn progress(job: JobId, sent: u64, total: u64) -> Event {
        Event::Progress {
            job,
            progress: ProgressSnapshot {
                sent,
                total,
                message: String::new(),
            },
        }
    }

    #[test]
    fn jobs_follow_the_event_stream() {
        let mut model = Model::default();
        model.apply(queued(0, "a.zip", "Catbox", 100));
        model.apply(queued(1, "a.zip", "Oshi", 100));
        assert_eq!(
            model.counts(),
            Counts {
                queued: 2,
                ..Counts::default()
            }
        );

        model.apply(Event::Started { job: 0 });
        model.apply(progress(0, 40, 100));
        assert!(matches!(&model.jobs[0].state, JobState::Running(p) if p.sent == 40));
        assert_eq!(model.counts().running, 1);

        model.apply(finished(0, Ok(UploadedFile::new("https://x/1"))));
        model.apply(finished(1, Err(UploadError::EmptyFile)));
        assert_eq!(
            model.jobs[0].state,
            JobState::Done {
                url: "https://x/1".into()
            }
        );
        assert!(matches!(
            &model.jobs[1].state,
            JobState::Failed {
                kind: "empty_file",
                ..
            }
        ));
        assert_eq!(
            model.counts(),
            Counts {
                done: 1,
                failed: 1,
                ..Counts::default()
            }
        );
    }

    #[test]
    fn cancelled_jobs_are_distinct_from_failures() {
        let mut model = Model::default();
        model.apply(queued(0, "a", "S", 1));
        model.apply(finished(0, Err(UploadError::Cancelled)));
        assert_eq!(model.jobs[0].state, JobState::Cancelled);
        assert_eq!(model.counts().cancelled, 1);
    }

    #[test]
    fn a_late_progress_sample_does_not_reopen_a_finished_job() {
        let mut model = Model::default();
        model.apply(queued(0, "a", "S", 10));
        model.apply(finished(0, Ok(UploadedFile::new("u"))));
        model.apply(progress(0, 5, 10));
        assert!(matches!(model.jobs[0].state, JobState::Done { .. }));
    }

    #[test]
    fn events_for_unknown_jobs_are_ignored() {
        let mut model = Model::default();
        model.apply(progress(7, 1, 2));
        model.apply(Event::Started { job: 3 });
        assert!(model.jobs.is_empty());
    }

    #[test]
    fn overall_progress_weights_by_size() {
        let mut model = Model::default();
        model.apply(queued(0, "big", "S", 900));
        model.apply(queued(1, "small", "S", 100));
        model.apply(Event::Started { job: 0 });
        model.apply(progress(0, 450, 900));
        assert!((model.overall_fraction() - 0.45).abs() < 1e-6);
        model.apply(finished(1, Ok(UploadedFile::new("u"))));
        assert!((model.overall_fraction() - 0.55).abs() < 1e-6);
        assert_eq!(Model::default().overall_fraction(), 0.0);
    }

    #[test]
    fn links_can_be_copied_plain_or_in_the_preset_format() {
        let mut model = Model::default();
        model.begin_run(Some("[URL='{file_url}']{provider_name}[/URL]".into()));
        model.apply(queued(0, "a", "Catbox", 1));
        model.apply(queued(1, "a", "Oshi", 1));
        model.apply(queued(2, "a", "Keep", 1));
        model.apply(finished(0, Ok(UploadedFile::new("https://c/1"))));
        model.apply(finished(1, Err(UploadError::EmptyFile)));
        model.apply(finished(2, Ok(UploadedFile::new("https://k/2"))));

        assert_eq!(model.links_text(false), "https://c/1\nhttps://k/2");
        assert_eq!(
            model.links_text(true),
            "[URL='https://c/1']Catbox[/URL]\n[URL='https://k/2']Keep[/URL]"
        );
        assert!(model.has_links());
    }

    #[test]
    fn formatted_links_fall_back_to_plain_without_a_template() {
        let mut model = Model::default();
        model.begin_run(None);
        model.apply(queued(0, "a", "Catbox", 1));
        model.apply(finished(0, Ok(UploadedFile::new("https://c/1"))));
        assert_eq!(model.links_text(true), "https://c/1");
    }

    #[test]
    fn run_state_transitions() {
        let mut model = Model::default();
        model.add_paths(&[]);
        model.files.push(FileEntry {
            path: "f".into(),
            name: "f".into(),
            size: 1,
        });
        model.toggle_provider("catbox", true);
        assert!(model.can_start());

        model.begin_run(None);
        assert!(model.is_running() && !model.can_start());
        model.request_cancel();
        assert_eq!(model.run, RunState::Cancelling);
        model.run_finished();
        assert_eq!(model.run, RunState::Idle);
        assert!(model.can_start());

        // Cancelling when nothing runs does nothing.
        model.request_cancel();
        assert_eq!(model.run, RunState::Idle);
    }

    #[test]
    fn starting_needs_files_and_providers() {
        let mut model = Model::default();
        assert!(!model.can_start());
        model.toggle_provider("catbox", true);
        assert!(!model.can_start());
        model.files.push(FileEntry {
            path: "f".into(),
            name: "f".into(),
            size: 1,
        });
        assert!(model.can_start());
        model.toggle_provider("catbox", false);
        assert!(!model.can_start());
    }

    #[test]
    fn adding_paths_dedupes_and_expands_folders() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "aaa").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "bb").unwrap();
        let mut model = Model::default();

        let problems = model.add_paths(&[
            tmp.path().to_path_buf(),
            tmp.path().join("a.txt"),
            tmp.path().join("gone"),
        ]);
        assert_eq!(problems.len(), 1);
        let names: Vec<_> = model.files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["a.txt", "b.txt"]);
        assert_eq!(model.total_size(), 5);

        model.remove_file(0);
        assert_eq!(model.files.len(), 1);
        model.remove_file(99); // out of range is a no-op
    }

    #[test]
    fn selecting_a_preset_replaces_the_selection() {
        let mut model = Model::default();
        model.toggle_provider("oshi", true);
        model.select_only(["catbox", "keep"]);
        assert_eq!(
            model
                .selected
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["catbox", "keep"]
        );
    }

    #[test]
    fn provider_filter_matches_names_and_aliases_case_insensitively() {
        let registry = Registry::builtin();
        let userscloud = registry.find("usercloud").unwrap();
        assert!(matches_filter(&**userscloud, ""));
        assert!(matches_filter(&**userscloud, "CLOUD"));
        assert!(matches_filter(&**userscloud, "userscl"));
        assert!(!matches_filter(&**userscloud, "oshi"));
    }
}
