//! Checks that stop a job before the provider is called.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use mul_tor_core::size::SizeLimit;
use mul_tor_core::{Credentials, UploadError};
use tokio_util::sync::CancellationToken;

use crate::support::*;

#[tokio::test]
async fn offline_checks_stop_a_job_before_the_provider_is_called() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let small = plain("Small").max_size(SizeLimit::mb(1));
    let keyed = plain("Keyed").requires(API_KEY);
    let picky = plain("Picky").blocked_extensions(BLOCKED);
    let providers = vec![
        fake(small, Behaviour::Succeed, &probe),
        fake(keyed, Behaviour::Succeed, &probe),
        fake(picky, Behaviour::Succeed, &probe),
    ];
    let big = h.dir.path().join("big.exe");
    std::fs::File::create(&big)
        .unwrap()
        .set_len(2 * 1024 * 1024)
        .unwrap();
    let empty = h.file("empty.bin", b"");

    let outcomes = h
        .engine()
        .run(
            vec![big, empty],
            providers,
            h.sink.clone(),
            CancellationToken::new(),
        )
        .await;

    let errors: Vec<String> = outcomes
        .iter()
        .map(|o| match &o.result {
            Err(UploadError::TooLarge { .. }) => "too large".into(),
            Err(UploadError::MissingCredential(k)) => format!("missing {k}"),
            Err(UploadError::Rejected(_)) => "rejected".into(),
            Err(UploadError::EmptyFile) => "empty".into(),
            other => format!("unexpected {other:?}"),
        })
        .collect();
    assert_eq!(
        errors,
        [
            "too large",
            "missing apiKey",
            "rejected",
            "empty",
            "empty",
            "empty"
        ]
    );
    assert_eq!(probe.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn credentials_reach_the_provider_by_provider_id() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let keyed = plain("Key.d").requires(API_KEY);
    let providers = vec![fake(keyed, Behaviour::EchoKey, &probe)];

    let mut creds = Credentials::default();
    creds.set("apiKey", "SECRET-1");
    let engine = h
        .engine()
        .with_credentials([("keyd".to_string(), creds)].into());

    let outcomes = engine
        .run(
            vec![h.file("f", b"x")],
            providers,
            h.sink.clone(),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(outcomes[0].result.as_ref().unwrap().url, "SECRET-1");
}

#[tokio::test]
async fn an_unreadable_file_fails_its_jobs_but_not_the_rest() {
    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let providers = vec![
        fake(plain("A"), Behaviour::Succeed, &probe),
        fake(plain("B"), Behaviour::Succeed, &probe),
    ];
    let missing = h.dir.path().join("missing.bin");
    let good = h.file("good.bin", b"data");

    let outcomes = h
        .engine()
        .run(
            vec![missing, good],
            providers,
            h.sink.clone(),
            CancellationToken::new(),
        )
        .await;

    assert_eq!(
        finished(&outcomes),
        [
            ("missing.bin".into(), "A", false),
            ("missing.bin".into(), "B", false),
            ("good.bin".into(), "A", true),
            ("good.bin".into(), "B", true),
        ]
    );
    assert!(matches!(outcomes[0].result, Err(UploadError::Io(_))));
}
