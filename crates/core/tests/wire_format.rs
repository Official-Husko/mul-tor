//! Checks the HTTP wire format that every uploader depends on, using Catbox as
//! the representative site.

mod common;

use common::*;

fn catbox_handler(r: &Recorded) -> Reply {
    if r.is("POST", "catbox.moe", "/user/api.php") {
        Reply::text("https://files.catbox.moe/abc123.bin\n")
    } else {
        Reply::not_found()
    }
}

#[tokio::test]
async fn catbox_sends_the_expected_form() {
    let mock = Mock::start(catbox_handler).await;
    let run = run_default(&*builtin("catbox"), &mock).await;

    assert_eq!(run.url(), "https://files.catbox.moe/abc123.bin");

    let requests = mock.requests();
    assert_eq!(requests.len(), 1);
    let parts = requests[0].multipart();
    assert_eq!(text_field(&parts, "reqtype"), "fileupload");
    let file = field(&parts, "fileToUpload");
    assert_eq!(file.filename.as_deref(), Some("sample.bin"));
    assert_eq!(file.data, b"0123456789abcdef");
    assert_eq!(file.content_type.as_deref(), Some("application/octet-stream"));
}

#[tokio::test]
async fn uploads_are_not_chunked_and_report_progress() {
    let mock = Mock::start(catbox_handler).await;
    let content = vec![7u8; 3 * 1024 * 1024 + 11];
    let run = run(
        &*builtin("catbox"),
        &mock,
        RunConfig {
            content: content.clone(),
            ..Default::default()
        },
    )
    .await;
    run.url();

    mock.requests()[0].assert_not_chunked();
    assert_eq!(run.progress.sent(), content.len() as u64);
}

#[tokio::test]
async fn bracketed_field_names_are_not_percent_encoded() {
    // reqwest encodes `files[]` as `name*=utf-8''files%5B%5D` unless told not
    // to; PHP-style hosts only understand `name="files[]"`.
    let mock = Mock::start(|_| Reply::json(serde_json::json!([{"url": "https://x/y"}]))).await;
    let http = mul_tor_core::HttpFactory::new()
        .with_proxy(mul_tor_core::ProxyMode::Direct)
        .with_url_rewrite(mock.rewrite())
        .client()
        .unwrap();
    let form = mul_tor_core::upload::new_form().text("files[]", "x");
    http.post("https://h.example/up")
        .multipart(form)
        .send()
        .await
        .unwrap();
    let parts = mock.requests()[0].multipart();
    assert_eq!(parts[0].name, "files[]");
    assert!(parts[0].disposition.contains(r#"name="files[]""#), "{}", parts[0].disposition);
}

#[tokio::test]
async fn non_ascii_file_names_are_sent_as_raw_utf8() {
    let mock = Mock::start(catbox_handler).await;
    let run = run(
        &*builtin("catbox"),
        &mock,
        RunConfig {
            name: "日本語 \"quoted\".txt".into(),
            ..Default::default()
        },
    )
    .await;
    run.url();
    let parts = mock.requests()[0].multipart();
    assert_eq!(
        field(&parts, "fileToUpload").filename.as_deref(),
        Some("日本語 \"quoted\".txt")
    );
    // On the wire the quotes are escaped, the UTF-8 is not percent-encoded.
    let disposition = &parts[1].disposition;
    assert!(disposition.contains("日本語"), "{disposition}");
    assert!(disposition.contains(r#"\"quoted\""#), "{disposition}");
}

#[tokio::test]
async fn prose_replies_are_reported_as_protocol_errors() {
    let mock = Mock::start(|_| Reply::text("No files given")).await;
    let run = run_default(&*builtin("catbox"), &mock).await;
    assert!(matches!(
        run.error(),
        mul_tor_core::UploadError::Protocol { .. }
    ));
    assert_eq!(run.error().raw(), Some("No files given"));
}

#[tokio::test]
async fn http_errors_are_reported_with_the_body() {
    let mock = Mock::start(|_| Reply::text("busy").status(503)).await;
    let run = run_default(&*builtin("catbox"), &mock).await;
    let err = run.error().to_string();
    assert!(err.contains("503"), "{err}");
}
