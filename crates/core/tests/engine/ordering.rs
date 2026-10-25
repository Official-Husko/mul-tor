//! Order, queueing and the parallelism bound.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use mul_tor_core::Event;
use tokio_util::sync::CancellationToken;

use crate::support::*;

#[tokio::test]
async fn jobs_run_file_major_in_the_order_given() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let providers = vec![
        fake(plain("A"), Behaviour::Succeed, &probe),
        fake(plain("B"), Behaviour::Succeed, &probe),
    ];
    let files = vec![h.file("one", b"1"), h.file("two", b"2")];

    let outcomes = h
        .engine()
        .run(files, providers, h.sink.clone(), CancellationToken::new())
        .await;

    assert_eq!(
        finished(&outcomes),
        [
            ("one".into(), "A", true),
            ("one".into(), "B", true),
            ("two".into(), "A", true),
            ("two".into(), "B", true),
        ]
    );
    assert_eq!(
        *probe.started.lock().unwrap(),
        ["A:one", "B:one", "A:two", "B:two"]
    );
    assert_eq!(
        probe.max_active.load(Ordering::SeqCst),
        1,
        "default is sequential"
    );
}

#[tokio::test]
async fn every_job_is_queued_before_any_starts_and_finishes_last() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let providers = vec![fake(plain("A"), Behaviour::Succeed, &probe)];
    h.engine()
        .run(
            vec![h.file("f1", b"x"), h.file("f2", b"y")],
            providers,
            h.sink.clone(),
            CancellationToken::new(),
        )
        .await;

    let events = h.events();
    let first_started = events
        .iter()
        .position(|e| matches!(e, Event::Started { .. }))
        .unwrap();
    let queued = events
        .iter()
        .filter(|e| matches!(e, Event::Queued { .. }))
        .count();
    assert_eq!(queued, 2);
    assert!(
        events[..first_started]
            .iter()
            .all(|e| matches!(e, Event::Queued { .. }))
    );

    for job in 0..2 {
        let kinds: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                Event::Started { job: j } if *j == job => Some("started"),
                Event::Finished(o) if o.job == job => Some("finished"),
                _ => None,
            })
            .collect();
        assert_eq!(kinds, ["started", "finished"]);
    }
}

#[tokio::test]
async fn parallelism_is_bounded_by_the_setting() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let providers: Vec<_> = ["A", "B", "C", "D"]
        .into_iter()
        .map(|name| fake(plain(name), Behaviour::Work(200), &probe))
        .collect();

    let outcomes = h
        .engine()
        .with_parallelism(2)
        .run(
            vec![h.file("f", b"0123456789")],
            providers,
            h.sink.clone(),
            CancellationToken::new(),
        )
        .await;

    assert!(outcomes.iter().all(|o| o.is_ok()));
    assert_eq!(probe.max_active.load(Ordering::SeqCst), 2);
}
