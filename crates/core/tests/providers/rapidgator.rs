//! Rapidgator: login, MD5 announcement, upload and polling.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use mul_tor_core::UploadError;
use mul_tor_core::providers::Rapidgator;
use serde_json::json;

use crate::common::*;

// ---------------------------------------------------------------- Rapidgator

const RG_LOGIN_OK: &str = "login-ok";

fn rapidgator_handler(max_file_size: u64, already_known: bool) -> impl Fn(&Recorded) -> Reply {
    let polls = Arc::new(AtomicUsize::new(0));
    move |r| match (r.method.as_str(), r.host.as_str(), r.path.as_str()) {
        ("GET", "rapidgator.net", "/api/v2/user/login") => Reply::json(json!({
            "status": 200,
            "response": {
                "token": RG_LOGIN_OK,
                "user": {"storage": {"left": "1000000"}, "upload": {"max_file_size": max_file_size}}
            }
        })),
        ("GET", "rapidgator.net", "/api/v2/file/upload") => {
            if already_known {
                Reply::json(json!({"status": 200, "response": {"upload": {
                    "state": 2, "file": {"url": "https://rapidgator.net/file/known"}}}}))
            } else {
                Reply::json(json!({"status": 200, "response": {"upload": {
                    "state": 0, "upload_id": "UP1", "url": "https://u1.rapidgator.net/upload/xyz"}}}))
            }
        }
        ("POST", "u1.rapidgator.net", "/upload/xyz") => Reply::json(json!({"status": 200})),
        ("GET", "rapidgator.net", "/api/v2/file/upload_info") => {
            if polls.fetch_add(1, Ordering::SeqCst) < 2 {
                Reply::json(json!({"status": 200, "response": {"upload": {"state": 1}}}))
            } else {
                Reply::json(json!({"status": 200, "response": {"upload": {
                    "state": 2, "file": {"url": "https://rapidgator.net/file/done"}}}}))
            }
        }
        _ => Reply::not_found(),
    }
}

fn rg_creds() -> RunConfig {
    RunConfig {
        content: b"hello world".to_vec(),
        credentials: creds(&[("email", "me@example.com"), ("password", "pw")]),
        ..Default::default()
    }
}

#[tokio::test]
async fn rapidgator_announces_the_md5_uploads_and_polls_until_done() {
    let mock = Mock::start(rapidgator_handler(1_000_000, false)).await;
    let provider = Rapidgator::with_polling(Duration::from_millis(1), 10);
    let run = run(&provider, &mock, rg_creds()).await;
    assert_eq!(run.url(), "https://rapidgator.net/file/done");

    let requests = mock.requests();
    let login = requests[0].query_pairs();
    assert!(login.contains(&("login".into(), "me@example.com".into())));
    assert!(login.contains(&("password".into(), "pw".into())));

    let announce = requests[1].query_pairs();
    assert!(announce.contains(&("name".into(), "sample.bin".into())));
    assert!(announce.contains(&("hash".into(), "5eb63bbbe01eeed093cb22bb8f5acdc3".into())));
    assert!(announce.contains(&("size".into(), "11".into())));
    assert!(announce.contains(&("token".into(), RG_LOGIN_OK.into())));

    field(&requests[2].multipart(), "file");
    // 3 polls: processing, processing, done.
    assert_eq!(
        mock.requests_to("rapidgator.net")
            .iter()
            .filter(|r| r.path.ends_with("upload_info"))
            .count(),
        3
    );
}

#[tokio::test]
async fn rapidgator_skips_the_upload_when_the_server_already_has_the_file() {
    let mock = Mock::start(rapidgator_handler(1_000_000, true)).await;
    let provider = Rapidgator::with_polling(Duration::from_millis(1), 10);
    let run = run(&provider, &mock, rg_creds()).await;
    assert_eq!(run.url(), "https://rapidgator.net/file/known");
    assert!(mock.requests_to("u1.rapidgator.net").is_empty());
    assert_eq!(run.progress.sent(), 0);
}

#[tokio::test]
async fn rapidgator_enforces_the_account_size_limit_after_login() {
    let mock = Mock::start(rapidgator_handler(5, false)).await;
    let provider = Rapidgator::with_polling(Duration::from_millis(1), 10);
    let run = run(&provider, &mock, rg_creds()).await;
    assert!(matches!(run.error(), UploadError::TooLarge { .. }));
    assert_eq!(
        mock.requests().len(),
        1,
        "only the login should have happened"
    );
}

#[tokio::test]
async fn rapidgator_reports_login_failures_without_leaking_the_password() {
    let mock =
        Mock::start(|_| Reply::json(json!({"status": 401, "details": "Wrong login or password"})))
            .await;
    let provider = Rapidgator::with_polling(Duration::from_millis(1), 10);
    let run = run(&provider, &mock, rg_creds()).await;
    let message = run.error().to_string();
    assert!(message.contains("Wrong login or password"), "{message}");
    assert!(!message.contains("pw"), "{message}");
}

#[tokio::test]
async fn rapidgator_survives_a_few_transient_errors_while_polling() {
    let polls = Arc::new(AtomicUsize::new(0));
    let mock = Mock::start(move |r| match r.path.as_str() {
        "/api/v2/user/login" => Reply::json(json!({"status": 200, "response": {"token": "T"}})),
        "/api/v2/file/upload" => Reply::json(json!({"status": 200, "response": {"upload": {
            "state": 0, "upload_id": "U", "url": "https://u1.rapidgator.net/up"}}})),
        "/up" => Reply::json(json!({"status": 200})),
        "/api/v2/file/upload_info" => {
            if polls.fetch_add(1, Ordering::SeqCst) < 3 {
                Reply::json(json!({"status": 404, "details": "Upload not found yet"}))
            } else {
                Reply::json(json!({"status": 200, "response": {"upload": {
                    "state": 2, "file": {"url": "https://rapidgator.net/file/late"}}}}))
            }
        }
        _ => Reply::not_found(),
    })
    .await;
    let provider = Rapidgator::with_polling(Duration::from_millis(1), 20);
    let run = run(&provider, &mock, rg_creds()).await;
    assert_eq!(run.url(), "https://rapidgator.net/file/late");
}

#[tokio::test]
async fn rapidgator_gives_up_on_persistent_polling_errors_and_says_why() {
    let mock = Mock::start(|r| match r.path.as_str() {
        "/api/v2/user/login" => Reply::json(json!({"status": 200, "response": {"token": "T"}})),
        "/api/v2/file/upload" => Reply::json(json!({"status": 200, "response": {"upload": {
            "state": 0, "upload_id": "U", "url": "https://u1.rapidgator.net/up"}}})),
        "/up" => Reply::json(json!({"status": 200})),
        _ => Reply::json(json!({"status": 401, "details": "Token expired"})),
    })
    .await;
    let provider = Rapidgator::with_polling(Duration::from_millis(1), 500);
    let run = run(&provider, &mock, rg_creds()).await;
    assert!(
        run.error().to_string().contains("Token expired"),
        "{}",
        run.error()
    );
    let polls = mock
        .requests()
        .iter()
        .filter(|r| r.path.ends_with("upload_info"))
        .count();
    assert_eq!(
        polls, 5,
        "should stop after 5 consecutive errors, not wait out all 500 polls"
    );
}

#[tokio::test]
async fn rapidgator_stops_polling_eventually() {
    let mock = Mock::start(|r| match r.path.as_str() {
        "/api/v2/user/login" => Reply::json(json!({"status": 200, "response": {"token": "T"}})),
        "/api/v2/file/upload" => Reply::json(json!({"status": 200, "response": {"upload": {
                "state": 0, "upload_id": "U", "url": "https://u.rapidgator.net/up"}}})),
        "/up" => Reply::json(json!({"status": 200})),
        _ if r.path.ends_with("/up") => Reply::json(json!({"status": 200})),
        _ => Reply::json(json!({"status": 200, "response": {"upload": {"state": 1}}})),
    })
    .await;
    let provider = Rapidgator::with_polling(Duration::from_millis(1), 3);
    let run = run(&provider, &mock, rg_creds()).await;
    assert!(run.error().to_string().contains("in time"));
}
