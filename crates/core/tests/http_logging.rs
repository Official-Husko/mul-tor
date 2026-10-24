//! Requests are logged for diagnosis, but never with their query string:
//! Rapidgator, for one, sends the account e-mail and password there.
//!
//! The logger is process-wide, so this file holds a single test.

mod common;

use std::time::Duration;

use common::*;
use mul_tor_core::logging::{self, LogBuffer, LogConfig};
use mul_tor_core::providers::Rapidgator;
use serde_json::json;
use tracing::Level;

#[tokio::test]
async fn secrets_in_query_strings_never_reach_the_log() {
    let buffer = LogBuffer::new(200);
    let _guard = logging::init(LogConfig::new().buffer(buffer.clone()).level(Level::TRACE));

    let mock =
        Mock::start(|_| Reply::json(json!({"status": 401, "details": "Wrong login or password"})))
            .await;
    let provider = Rapidgator::with_polling(Duration::from_millis(1), 3);
    let config = RunConfig {
        credentials: creds(&[("email", "me@example.com"), ("password", "hunter2-secret")]),
        ..Default::default()
    };
    let run = run(&provider, &mock, config).await;
    assert!(run.result.is_err());

    // The request really did carry the secret...
    let sent = &mock.requests()[0];
    assert!(sent.query.contains("hunter2-secret"), "{}", sent.query);

    // ...and the log knows about the request, minus the query string.
    let records = buffer.snapshot();
    let request = records
        .iter()
        .find(|r| r.message == "request sent")
        .expect("requests are logged at debug level");
    let url = request
        .fields
        .iter()
        .find(|(key, _)| key == "url")
        .map(|(_, value)| value.as_str())
        .unwrap();
    assert!(url.ends_with("/api/v2/user/login"), "{url}");
    assert!(!url.contains('?'), "{url}");

    let everything: String = records.iter().map(|r| r.line() + "\n").collect();
    for secret in ["hunter2-secret", "me@example.com", "me%40example.com"] {
        assert!(
            !everything.contains(secret),
            "`{secret}` leaked:\n{everything}"
        );
    }
}
