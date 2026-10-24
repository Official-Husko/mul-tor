//! Cancelling, and survival of a crashing provider.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use mul_tor_core::{Event, UploadError};
use tokio_util::sync::CancellationToken;

use crate::support::*;

#[tokio::test]
async fn cancelling_aborts_running_jobs_and_skips_the_rest() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let providers = vec![
        fake(plain("Hangs"), Behaviour::Hang, &probe),
        fake(plain("Later"), Behaviour::Succeed, &probe),
    ];
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    let probe_for_trigger = probe.clone();
    tokio::spawn(async move {
        while probe_for_trigger.calls.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        trigger.cancel();
    });

    let outcomes = tokio::time::timeout(
        Duration::from_secs(10),
        h.engine()
            .run(vec![h.file("f", b"x")], providers, h.sink.clone(), cancel),
    )
    .await
    .expect("run must not hang after cancellation");

    assert_eq!(outcomes.len(), 2);
    assert!(
        outcomes
            .iter()
            .all(|o| matches!(o.result, Err(UploadError::Cancelled)))
    );
    assert_eq!(
        probe.calls.load(Ordering::SeqCst),
        1,
        "the second provider never started"
    );
    assert_eq!(
        probe.active.load(Ordering::SeqCst),
        0,
        "the hung upload was dropped"
    );

    // Every job still gets exactly one Finished event.
    let finishes = h
        .events()
        .iter()
        .filter(|e| matches!(e, Event::Finished(_)))
        .count();
    assert_eq!(finishes, 2);
}

#[tokio::test]
async fn cancelling_before_the_run_starts_nothing() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let cancel = CancellationToken::new();
    cancel.cancel();
    let outcomes = h
        .engine()
        .run(
            vec![h.file("f", b"x")],
            vec![fake(plain("A"), Behaviour::Succeed, &probe)],
            h.sink.clone(),
            cancel,
        )
        .await;
    assert!(matches!(outcomes[0].result, Err(UploadError::Cancelled)));
    assert_eq!(probe.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_panicking_uploader_becomes_a_failed_job() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let providers = vec![
        fake(plain("Boom"), Behaviour::Panic, &probe),
        fake(plain("Fine"), Behaviour::Succeed, &probe),
    ];
    let outcomes = h
        .engine()
        .run(
            vec![h.file("f", b"x")],
            providers,
            h.sink.clone(),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(
        finished(&outcomes),
        [("f".into(), "Boom", false), ("f".into(), "Fine", true)]
    );
    assert!(
        outcomes[0]
            .result
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("crashed")
    );
    assert_eq!(
        h.events()
            .iter()
            .filter(|e| matches!(e, Event::Finished(_)))
            .count(),
        2
    );
}
