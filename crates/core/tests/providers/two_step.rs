//! Providers that look something up before uploading.

use serde_json::json;

use crate::common::*;

#[tokio::test]
async fn krakenfiles_finds_its_upload_server_in_the_home_page() {
    let mock = Mock::start(|r| {
        if r.is("GET", "krakenfiles.com", "/") {
            Reply::html(
                r#"<script>var u = "https://uploads12.krakenfiles.com/_uploader/gallery/upload";</script>"#,
            )
        } else if r.is("POST", "uploads12.krakenfiles.com", "/_uploader/gallery/upload") {
            Reply::json(json!({"files": [{"url": "/view/abc/file.html"}]}))
        } else {
            Reply::not_found()
        }
    })
    .await;
    let run = run_default(&*builtin("krakenfiles"), &mock).await;
    assert_eq!(run.url(), "https://krakenfiles.com/view/abc/file.html");
    let upload = mock.requests().pop().unwrap();
    field(&upload.multipart(), "files[]");
}

#[tokio::test]
async fn krakenfiles_surfaces_the_server_side_error() {
    let mock = Mock::start(|r| {
        if r.method == "GET" {
            Reply::html("https://uploads1.krakenfiles.com/_uploader/gallery/upload")
        } else {
            Reply::json(json!({"files": [{"error": "File type not allowed"}]}))
        }
    })
    .await;
    let run = run_default(&*builtin("krakenfiles"), &mock).await;
    assert!(run.error().to_string().contains("File type not allowed"));
}

#[tokio::test]
async fn bowfile_authorises_then_uploads_with_the_token() {
    let mock = Mock::start(|r| {
        if r.is("POST", "bowfile.com", "/api/v2/authorize") {
            Reply::json(
                json!({"_status": "success", "data": {"access_token": "TOK", "account_id": 77}}),
            )
        } else if r.is("POST", "bowfile.com", "/api/v2/file/upload") {
            Reply::json(json!({"data": [{"url": "https://bowfile.com/abc"}]}))
        } else {
            Reply::not_found()
        }
    })
    .await;
    let run = run(
        &*builtin("bowfile"),
        &mock,
        RunConfig {
            credentials: creds(&[("apiKey1", "K1"), ("apiKey2", "K2")]),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(run.url(), "https://bowfile.com/abc");

    let requests = mock.requests();
    let auth = requests[0].form();
    assert!(auth.contains(&("key1".into(), "K1".into())));
    assert!(auth.contains(&("key2".into(), "K2".into())));
    let parts = requests[1].multipart();
    assert_eq!(text_field(&parts, "access_token"), "TOK");
    assert_eq!(text_field(&parts, "account_id"), "77");
    field(&parts, "upload_file");
}

#[tokio::test]
async fn bowfile_bad_keys_stop_before_uploading() {
    let mock = Mock::start(|_| Reply::json(json!({"_status": "error"}))).await;
    let run = run(
        &*builtin("bowfile"),
        &mock,
        RunConfig {
            credentials: creds(&[("apiKey1", "x"), ("apiKey2", "y")]),
            ..Default::default()
        },
    )
    .await;
    assert!(run.error().to_string().contains("authorisation"));
    assert_eq!(mock.requests().len(), 1);
}
