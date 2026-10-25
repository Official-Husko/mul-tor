//! Storage.to: presigned URLs from an API, bytes straight to storage, then a confirmation.

use mul_tor_core::UploadError;
use serde_json::{Value, json};

use crate::common::*;
use crate::support::*;

const STORAGE: &str = "acct.r2.cloudflarestorage.com";

fn body_json(r: &Recorded) -> Value {
    serde_json::from_slice(&r.body).unwrap()
}

fn confirmed() -> Reply {
    Reply::json(json!({
        "success": true,
        "file": {"id": "FzmPr0Jlz", "url": "https://storage.to/FzmPr0Jlz"},
        "owner_token": "owner_v1_secret",
    }))
}

fn single(r: &Recorded) -> Reply {
    if r.is("POST", "storage.to", "/api/upload/init") {
        Reply::json(json!({
            "success": true,
            "type": "single",
            "upload_url": format!("https://{STORAGE}/storage-to/KEY1?X-Amz-Signature=abc"),
            "headers": {"Content-Type": ["application/octet-stream"]},
            "r2_key": "KEY1",
        }))
    } else if r.method == "PUT" && r.host == STORAGE {
        Reply::text("").header("etag", "\"single\"")
    } else if r.is("POST", "storage.to", "/api/upload/confirm") {
        confirmed()
    } else {
        Reply::not_found()
    }
}

#[tokio::test]
async fn storage_to_puts_a_small_file_to_the_signed_url_and_confirms() {
    let mock = Mock::start(single).await;
    let run = run_default(&*builtin("storage.to"), &mock).await;

    assert_eq!(run.url(), "https://storage.to/FzmPr0Jlz");
    let requests = mock.requests();
    assert_eq!(requests.len(), 3);

    let init = body_json(&requests[0]);
    assert_eq!(init["filename"], "sample.bin");
    assert_eq!(init["size"], 16);
    assert_eq!(init["content_type"], "application/octet-stream");
    // The longest an anonymous upload may be kept.
    assert_eq!(init["expiry_days"], 7);

    let put = &requests[1];
    assert_eq!(put.path, "/storage-to/KEY1");
    assert_eq!(put.query, "X-Amz-Signature=abc");
    assert_eq!(put.body, b"0123456789abcdef");
    assert_eq!(put.header("content-type"), Some("application/octet-stream"));
    put.assert_not_chunked();
    assert_eq!(run.progress.sent(), 16);

    let confirm = &requests[2];
    let sent = body_json(confirm);
    assert_eq!(sent["r2_key"], "KEY1");
    assert_eq!(sent["size"], 16);
    assert_eq!(sent["expiry_days"], 7);
    assert!(confirm.header("x-owner-token").is_none());
}

fn multipart(r: &Recorded) -> Reply {
    if r.is("POST", "storage.to", "/api/upload/init") {
        // Only the first two parts are listed; the third must be asked for.
        Reply::json(json!({
            "success": true,
            "type": "multipart",
            "upload_id": "UP1",
            "r2_key": "KEY2",
            "part_size": 10,
            "total_parts": 3,
            "initial_urls": {
                "1": format!("https://{STORAGE}/storage-to/KEY2?partNumber=1"),
                "2": format!("https://{STORAGE}/storage-to/KEY2?partNumber=2"),
            },
            "owner_token": "owner_v1_secret",
        }))
    } else if r.is("POST", "storage.to", "/api/upload/parts") {
        Reply::json(json!({"success": true, "urls": {
            "3": format!("https://{STORAGE}/storage-to/KEY2?partNumber=3")}}))
    } else if r.method == "PUT" && r.host == STORAGE {
        let number = r.query.trim_start_matches("partNumber=").to_string();
        Reply::text("").header("etag", &format!("\"etag-{number}\""))
    } else if r.is("POST", "storage.to", "/api/upload/complete-multipart") {
        Reply::json(json!({"success": true, "r2_key": "FINAL"}))
    } else if r.is("POST", "storage.to", "/api/upload/confirm") {
        confirmed()
    } else {
        Reply::not_found()
    }
}

#[tokio::test]
async fn storage_to_sends_parts_asks_for_missing_urls_and_assembles_them() {
    let mock = Mock::start(multipart).await;
    let run = run(&*builtin("storage.to"), &mock, config_25()).await;
    assert_eq!(run.url(), "https://storage.to/FzmPr0Jlz");

    let requests = mock.requests();
    let puts: Vec<_> = requests.iter().filter(|r| r.method == "PUT").collect();
    assert_eq!(puts.len(), 3);
    let sent: Vec<u8> = puts.iter().flat_map(|p| p.body.clone()).collect();
    assert_eq!(sent, CONTENT_25);
    for put in &puts {
        put.assert_not_chunked();
    }
    assert_eq!(run.progress.sent(), 25);

    let more = requests
        .iter()
        .find(|r| r.path == "/api/upload/parts")
        .unwrap();
    assert_eq!(more.header("x-owner-token"), Some("owner_v1_secret"));
    assert_eq!(
        body_json(more),
        json!({"upload_id": "UP1", "part_numbers": [3]})
    );

    let complete = requests
        .iter()
        .find(|r| r.path == "/api/upload/complete-multipart")
        .unwrap();
    assert_eq!(complete.header("x-owner-token"), Some("owner_v1_secret"));
    assert_eq!(
        body_json(complete),
        json!({"upload_id": "UP1", "parts": [
            {"partNumber": 1, "etag": "\"etag-1\""},
            {"partNumber": 2, "etag": "\"etag-2\""},
            {"partNumber": 3, "etag": "\"etag-3\""},
        ]})
    );

    let confirm = requests.last().unwrap();
    assert_eq!(confirm.path, "/api/upload/confirm");
    assert_eq!(confirm.header("x-owner-token"), Some("owner_v1_secret"));
    assert_eq!(body_json(confirm)["r2_key"], "KEY2");
}

#[tokio::test]
async fn storage_to_never_sends_the_file_to_a_url_off_the_storage_domain() {
    for url in [
        "https://evil.example/storage-to/KEY1",
        "https://r2.cloudflarestorage.com.evil.example/KEY1",
        "http://acct.r2.cloudflarestorage.com/KEY1",
    ] {
        let mock = Mock::start(move |r| {
            if r.path == "/api/upload/init" {
                Reply::json(json!({
                    "success": true, "type": "single", "r2_key": "K",
                    "upload_url": url, "headers": {},
                }))
            } else {
                Reply::not_found()
            }
        })
        .await;
        let run = run_default(&*builtin("storage.to"), &mock).await;
        assert!(matches!(run.error(), UploadError::Protocol { .. }), "{url}");
        assert_eq!(mock.requests().len(), 1, "{url}");
    }
}

#[tokio::test]
async fn storage_to_reports_why_a_confirmation_was_refused_without_the_token() {
    let mock = Mock::start(|r| {
        if r.path == "/api/upload/confirm" {
            Reply::json(json!({"success": false, "error": "Upload incomplete"}))
        } else {
            single(r)
        }
    })
    .await;
    let run = run_default(&*builtin("storage.to"), &mock).await;
    let message = format!("{:?}", run.error());
    assert!(message.contains("Upload incomplete"), "{message}");
}

#[tokio::test]
async fn storage_to_stops_when_the_parts_cannot_be_assembled() {
    let mock = Mock::start(|r| {
        if r.path == "/api/upload/complete-multipart" {
            Reply::json(json!({"success": false, "error_code": "MalformedXML"}))
        } else {
            multipart(r)
        }
    })
    .await;
    let run = run(&*builtin("storage.to"), &mock, config_25()).await;
    assert!(run.error().to_string().contains("assembled"));
    assert!(
        mock.requests()
            .iter()
            .all(|r| r.path != "/api/upload/confirm")
    );
}
