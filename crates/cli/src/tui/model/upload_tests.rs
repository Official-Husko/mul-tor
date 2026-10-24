//! How job events add up to what the screen shows.

use std::sync::Arc;

use mul_tor_core::{Event, JobOutcome, ProgressSnapshot, UploadError, UploadedFile};

use super::upload::{Counts, JobState, UploadState};

fn queued(job: usize, file: &str, provider: &'static str, size: u64) -> Event {
    Event::Queued {
        job,
        file_name: file.to_string(),
        provider,
        size,
    }
}

fn progress(job: usize, sent: u64, total: u64) -> Event {
    Event::Progress {
        job,
        progress: ProgressSnapshot {
            sent,
            total,
            message: String::new(),
        },
    }
}

fn finished(job: usize, result: Result<UploadedFile, UploadError>) -> Event {
    Event::Finished(Arc::new(JobOutcome {
        job,
        file_name: "f".into(),
        provider: "P",
        result,
    }))
}

fn state() -> UploadState {
    let mut state = UploadState::default();
    state.begin();
    for (job, provider) in ["Alpha", "Beta", "Gamma"].into_iter().enumerate() {
        state.apply(queued(job, "a.zip", provider, 1000));
    }
    state
}

#[test]
fn jobs_move_through_queued_running_and_finished() {
    let mut state = state();
    assert_eq!(
        state.counts(),
        Counts {
            queued: 3,
            ..Counts::default()
        }
    );

    state.apply(Event::Started { job: 0 });
    state.apply(progress(0, 400, 1000));
    assert_eq!(state.counts().running, 1);
    assert_eq!(state.bytes(), (400, 3000));

    state.apply(finished(0, Ok(UploadedFile::new("https://x/1"))));
    state.apply(finished(1, Err(UploadError::EmptyFile)));
    state.apply(finished(2, Err(UploadError::Cancelled)));
    assert_eq!(
        state.counts(),
        Counts {
            ok: 1,
            failed: 1,
            cancelled: 1,
            ..Counts::default()
        }
    );
    assert_eq!(
        state.bytes(),
        (3000, 3000),
        "failed and cancelled jobs are over"
    );
    assert_eq!(state.finished_jobs(), 3);
}

#[test]
fn failures_keep_their_kind_and_message() {
    let mut state = state();
    state.apply(finished(
        1,
        Err(UploadError::protocol("bad reply", "<html>oops</html>")),
    ));
    let JobState::Failed { error, kind, raw } = &state.jobs[1].state else {
        panic!("not failed");
    };
    assert!(error.contains("bad reply"));
    assert_eq!(*kind, "protocol");
    assert_eq!(raw.as_deref(), Some("<html>oops</html>"));
}

#[test]
fn only_successful_uploads_have_links() {
    let mut state = state();
    state.apply(finished(0, Ok(UploadedFile::new("https://x/1"))));
    state.apply(finished(1, Err(UploadError::EmptyFile)));
    let links = state.links();
    assert_eq!(links.len(), 1);
    assert_eq!((links[0].provider, links[0].url), ("Alpha", "https://x/1"));
}

#[test]
fn a_stray_event_for_an_unknown_job_is_ignored() {
    let mut state = state();
    state.apply(progress(9, 1, 1));
    state.apply(Event::Started { job: 9 });
    state.apply(finished(9, Err(UploadError::EmptyFile)));
    assert_eq!(state.jobs.len(), 3);
}

#[test]
fn there_is_no_eta_until_something_moves() {
    let mut state = state();
    assert_eq!(state.eta_seconds(), None);
    state.apply(Event::Started { job: 0 });
    if let JobState::Running { speed, .. } = &mut state.jobs[0].state {
        *speed = 100.0;
    }
    assert_eq!(state.speed(), 100.0);
    assert_eq!(state.eta_seconds(), Some(30), "3000 bytes at 100 B/s");
}

#[test]
fn begin_forgets_the_previous_run() {
    let mut state = state();
    state.cancelling = true;
    state.finish();
    assert!(state.is_finished());
    state.begin();
    assert!(state.jobs.is_empty() && !state.cancelling && !state.is_finished());
}
