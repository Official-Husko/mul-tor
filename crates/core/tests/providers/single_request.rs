//! Providers that send the file in one raw request.

use mul_tor_core::UploadError;
use serde_json::json;

use crate::common::*;
use crate::support::*;

#[tokio::test]
async fn pixeldrain_puts_the_raw_file_with_the_api_key_cookie() {
    let mock = Mock::start(|r| {
        if r.method == "PUT" && r.host == "pixeldrain.com" {
            Reply::json(json!({"success": true, "id": "AbC123"})).status(201)
        } else {
            Reply::not_found()
        }
    })
    .await;
    let run = run(
        &*builtin("pixeldrain"),
        &mock,
        RunConfig {
            name: "my file#1.bin".into(),
            credentials: creds(&[("apiKey", "sekrit")]),
            ..Default::default()
        },
    )
    .await;

    assert_eq!(run.url(), "https://pixeldrain.com/u/AbC123");
    let request = only_request(&mock);
    assert_eq!(request.path, "/api/file/my%20file%231.bin");
    assert_eq!(request.header("cookie"), Some("pd_auth_key=sekrit"));
    assert_eq!(
        request.header("content-type"),
        Some("application/octet-stream")
    );
    assert_eq!(request.body, b"0123456789abcdef");
    request.assert_not_chunked();
    assert_eq!(run.progress.sent(), 16);
}

#[tokio::test]
async fn pixeldrain_without_a_key_fails_before_any_request() {
    let mock = Mock::start(|_| Reply::not_found()).await;
    let run = run_default(&*builtin("pixeldrain"), &mock).await;
    assert!(matches!(
        run.error(),
        UploadError::MissingCredential("apiKey")
    ));
    assert!(mock.requests().is_empty());
}

#[tokio::test]
async fn pixeldrain_rejects_non_201() {
    let mock = Mock::start(|_| Reply::json(json!({"value": "unauthorized"})).status(401)).await;
    let run = run(
        &*builtin("pixeldrain"),
        &mock,
        RunConfig {
            credentials: creds(&[("apiKey", "k")]),
            ..Default::default()
        },
    )
    .await;
    assert!(run.error().to_string().contains("401"));
}

#[tokio::test]
async fn filebin_posts_to_a_random_bin() {
    let mock = Mock::start(|_| Reply::json(json!({})).status(201)).await;
    let run = run_default(&*builtin("filebin"), &mock).await;

    let request = only_request(&mock);
    assert_eq!(request.method, "POST");
    let segments: Vec<&str> = request.path.trim_start_matches('/').split('/').collect();
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].len(), 16);
    assert!(
        segments[0]
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    );
    assert_eq!(segments[1], "sample.bin");
    assert_eq!(request.header("accept"), Some("application/json"));
    assert_eq!(
        request.header("content-type"),
        Some("application/octet-stream")
    );
    assert_eq!(request.body, b"0123456789abcdef");
    request.assert_not_chunked();
    assert_eq!(
        run.url(),
        format!("https://filebin.net/{}/sample.bin", segments[0])
    );
}

#[tokio::test]
async fn keep_puts_with_a_curl_user_agent() {
    let mock = Mock::start(|_| Reply::text("https://free.keep.sh/AbCd/sample.bin\n")).await;
    let run = run_default(&*builtin("keep"), &mock).await;
    assert_eq!(run.url(), "https://free.keep.sh/AbCd/sample.bin");
    let request = only_request(&mock);
    assert_eq!(request.method, "PUT");
    assert_eq!(request.header("user-agent"), Some("curl/8.4.0"));
    assert_eq!(request.path, "/sample.bin");
    request.assert_not_chunked();
}

#[tokio::test]
async fn keep_rejects_a_reply_that_is_not_a_link() {
    let mock = Mock::start(|_| Reply::text("quota exceeded")).await;
    let run = run_default(&*builtin("keep"), &mock).await;
    assert!(matches!(run.error(), UploadError::Protocol { .. }));
}

#[tokio::test]
async fn transfer_sh_puts_the_plain_file_not_a_multipart_envelope() {
    let mock = Mock::start(|_| Reply::text("https://transfer.sh/xYz/sample.bin\n")).await;
    let run = run_default(&*builtin("transfer.sh"), &mock).await;
    assert_eq!(run.url(), "https://transfer.sh/xYz/sample.bin");
    let request = only_request(&mock);
    assert_eq!(request.method, "PUT");
    assert_eq!(
        request.body, b"0123456789abcdef",
        "body must be the bare file"
    );
}

#[tokio::test]
async fn legacy_provider_spellings_still_resolve() {
    for legacy in [
        "Transfer_sh",
        "Download_gg",
        "Files.fm",
        "Fileio",
        "FileSi",
        "Userscloud",
    ] {
        assert!(
            mul_tor_core::Registry::builtin().find(legacy).is_some(),
            "{legacy}"
        );
    }
}

#[tokio::test]
async fn buzzheavier_reads_the_id_from_either_reply_shape() {
    for reply in [
        json!({"id": "Z1"}),
        json!({"code": 201, "data": {"id": "Z1"}}),
    ] {
        let mock = Mock::start(move |_| Reply::json(reply.clone()).status(201)).await;
        let run = run_default(&*builtin("buzzheavier"), &mock).await;
        assert_eq!(run.url(), "https://buzzheavier.com/f/Z1");
        let request = only_request(&mock);
        assert_eq!(
            (request.method.as_str(), request.host.as_str()),
            ("PUT", "w.buzzheavier.com")
        );
        assert_eq!(request.path, "/sample.bin");
        request.assert_not_chunked();
    }
}
