//! Uploads: events in, rows and links out.

use std::sync::Arc;

use mul_tor_core::{Event, JobId, JobOutcome, ProgressSnapshot, UploadError, UploadedFile};

use crate::model::jobs::Counts;
use crate::model::{JobState, Model};

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
    model.begin_run(Some("[URL='{file_url}']{site_name}[/URL]".into()));
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
