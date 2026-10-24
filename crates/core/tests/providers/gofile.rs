//! Gofile asks for a server first, then uploads there.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use mul_tor_core::UploadError;
use mul_tor_core::providers::Gofile;
use serde_json::json;

use crate::common::*;

// -------------------------------------------------------------------- Gofile

fn gofile_handler(fail_servers: usize) -> impl Fn(&Recorded) -> Reply {
    let calls = Arc::new(AtomicUsize::new(0));
    move |r| {
        if r.is("GET", "api.gofile.io", "/getServer") {
            if calls.fetch_add(1, Ordering::SeqCst) < fail_servers {
                Reply::json(json!({"status": "error-rateLimit"}))
            } else {
                Reply::json(json!({"status": "ok", "data": {"server": "store9"}}))
            }
        } else if r.is("POST", "store9.gofile.io", "/uploadFile") {
            Reply::json(
                json!({"status": "ok", "data": {"downloadPage": "https://gofile.io/d/Xy12"}}),
            )
        } else {
            Reply::not_found()
        }
    }
}

#[tokio::test]
async fn gofile_asks_for_a_server_then_uploads_there() {
    let mock = Mock::start(gofile_handler(0)).await;
    let run = run_default(&*builtin("gofile"), &mock).await;
    assert_eq!(run.url(), "https://gofile.io/d/Xy12");
    let upload = mock.requests().pop().unwrap();
    assert_eq!(field(&upload.multipart(), "file").data, b"0123456789abcdef");
    upload.assert_not_chunked();
}

#[tokio::test]
async fn gofile_retries_while_rate_limited() {
    let mock = Mock::start(gofile_handler(2)).await;
    let provider = Gofile::with_delays(Duration::from_millis(1), Duration::from_millis(1));
    let run = run_default(&provider, &mock).await;
    assert_eq!(run.url(), "https://gofile.io/d/Xy12");
    assert_eq!(mock.requests_to("api.gofile.io").len(), 3);
}

#[tokio::test]
async fn gofile_gives_up_instead_of_looping_forever() {
    let mock = Mock::start(gofile_handler(usize::MAX)).await;
    let provider = Gofile::with_delays(Duration::from_millis(1), Duration::from_millis(1));
    let run = run_default(&provider, &mock).await;
    assert!(matches!(run.error(), UploadError::Protocol { .. }));
    assert_eq!(mock.requests_to("api.gofile.io").len(), 5);
    assert!(mock.requests_to("store9.gofile.io").is_empty());
}

#[tokio::test]
async fn gofile_refuses_a_server_name_that_is_not_a_hostname_label() {
    let mock = Mock::start(|_| {
        Reply::json(json!({"status": "ok", "data": {"server": "evil.example/x?"}}))
    })
    .await;
    let run = run_default(&*builtin("gofile"), &mock).await;
    assert!(matches!(run.error(), UploadError::Protocol { .. }));
    assert_eq!(mock.requests().len(), 1);
}
