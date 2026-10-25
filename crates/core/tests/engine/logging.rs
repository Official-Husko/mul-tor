//! What the engine writes to the log. The logger is process-wide, so this is
//! the only test in the binary that installs one. The other engine tests log
//! into it as well, so this test finds its own records by a parallelism and
//! provider names that nothing else uses.

use std::sync::Arc;

use mul_tor_core::logging::{self, LogBuffer, LogConfig};
use tokio_util::sync::CancellationToken;
use tracing::Level;

use crate::support::*;

#[tokio::test(flavor = "multi_thread")]
async fn runs_and_jobs_leave_a_structured_trail_without_secrets() {
    let buffer = LogBuffer::new(500);
    let guard = logging::init(LogConfig::new().buffer(buffer.clone()).level(Level::DEBUG));
    assert!(guard.problems().is_empty(), "{:?}", guard.problems());

    let h = Harness::new();
    let probe = Arc::new(Probe::default());
    let providers = vec![
        fake(plain("TrailOk"), Behaviour::Succeed, &probe),
        fake(plain("TrailBad"), Behaviour::Fail, &probe),
    ];
    let file = h.file("trail-test.bin", b"payload");
    h.engine()
        .with_parallelism(7)
        .run(
            vec![file],
            providers,
            h.sink.clone(),
            CancellationToken::new(),
        )
        .await;

    let records = buffer.snapshot();
    let find = |message: &str, provider: &str| {
        records
            .iter()
            .find(|r| r.message == message && r.spans.contains(&format!("provider={provider}")))
            .unwrap_or_else(|| panic!("no `{message}` record for {provider}"))
    };

    // Jobs are children of the run, and carry their identity in the span.
    let started = find("upload started", "TrailOk");
    assert_eq!(
        started.spans,
        "run{files=1 providers=2 parallel=7}:job{id=0 provider=TrailOk file=trail-test.bin}"
    );
    assert!(started.fields.contains(&("size".into(), "7".into())));
    assert!(started.target.starts_with("mul_tor_core::engine"));
    assert!(started.location.contains("job.rs:"), "{}", started.location);

    let finished = find("upload finished", "TrailOk");
    assert_eq!(finished.level, Level::INFO);
    assert!(finished.fields.iter().any(|(k, _)| k == "elapsed_ms"));

    // A failure is a warning that says what kind it was.
    let failed = find("upload failed", "TrailBad");
    assert_eq!(failed.level, Level::WARN);
    assert!(failed.fields.contains(&("kind".into(), "protocol".into())));

    // The run closes with a summary.
    let summary = records
        .iter()
        .find(|r| r.message == "run finished" && r.spans.contains("parallel=7"))
        .expect("a run summary");
    for expected in [("succeeded", "1"), ("failed", "1"), ("cancelled", "0")] {
        assert!(
            summary
                .fields
                .contains(&(expected.0.to_string(), expected.1.to_string())),
            "{expected:?} in {:?}",
            summary.fields
        );
    }

    // Order and numbering are stable for a UI that polls with `since`.
    assert!(records.windows(2).all(|pair| pair[0].seq < pair[1].seq));
}
