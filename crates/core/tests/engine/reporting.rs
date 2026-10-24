//! Progress events, the link file, and empty runs.

use std::sync::Arc;

use mul_tor_core::{Event, LinkLog};
use tokio_util::sync::CancellationToken;

use crate::support::*;

#[tokio::test]
async fn progress_is_reported_while_a_job_runs() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let providers = vec![fake(plain("Slow"), Behaviour::Work(600), &probe)];
    h.engine()
        .run(
            vec![h.file("f", &[0u8; 1000])],
            providers,
            h.sink.clone(),
            CancellationToken::new(),
        )
        .await;

    let sent: Vec<u64> = h
        .events()
        .iter()
        .filter_map(|e| match e {
            Event::Progress { progress, .. } => Some(progress.sent),
            _ => None,
        })
        .collect();
    assert!(sent.len() >= 3, "expected several samples, got {sent:?}");
    assert!(
        sent.windows(2).all(|w| w[0] <= w[1]),
        "progress went backwards: {sent:?}"
    );
    assert_eq!(*sent.last().unwrap(), 1000);
    let totals: Vec<u64> = h
        .events()
        .iter()
        .filter_map(|e| match e {
            Event::Progress { progress, .. } => Some(progress.total),
            _ => None,
        })
        .collect();
    assert!(totals.iter().all(|t| *t == 1000));
}

#[tokio::test]
async fn only_successful_uploads_are_written_to_the_link_file() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let providers = vec![
        fake(plain("Good"), Behaviour::Succeed, &probe),
        fake(plain("Bad"), Behaviour::Fail, &probe),
    ];
    let out = h.dir.path().join("links");
    let log = LinkLog::new(&out).with_template(Some("[{provider_name}]({file_url})".into()));

    h.engine()
        .with_link_log(Some(log))
        .run(
            vec![h.file("a.zip", b"x")],
            providers,
            h.sink.clone(),
            CancellationToken::new(),
        )
        .await;

    let plain_log = std::fs::read_to_string(out.join("file_links.txt")).unwrap();
    assert_eq!(plain_log.lines().count(), 1);
    assert!(plain_log.contains("| Good | a.zip - https://fake.example/a.zip"));
    let formatted = std::fs::read_to_string(out.join("file_links_formatted.txt")).unwrap();
    assert_eq!(formatted.trim(), "[Good](https://fake.example/a.zip)");
}

#[tokio::test]
async fn no_files_or_no_providers_is_a_quiet_no_op() {
    let h = Harness::new();
    let outcomes = h
        .engine()
        .run(vec![], vec![], h.sink.clone(), CancellationToken::new())
        .await;
    assert!(outcomes.is_empty());
    assert!(h.events().is_empty());
}
