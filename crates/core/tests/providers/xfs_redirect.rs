//! XFileSharing hosts that end the upload on a result page instead of JSON.

use mul_tor_core::UploadError;
use serde_json::json;

use crate::common::*;

const HOME: &str =
    r#"<form action="https://server7.dosya.co/cgi-bin/upload.cgi?upload_id=" method="post">"#;

/// The home page, an upload endpoint that answers with `location`, and the page it points at.
fn dosya(location: &'static str) -> impl Fn(&Recorded) -> Reply {
    move |r| {
        if r.method == "GET" && r.host == "dosya.co" {
            Reply::html(HOME)
        } else if r.is("POST", "server7.dosya.co", "/cgi-bin/upload.cgi") {
            Reply::text("").status(302).header("location", location)
        } else {
            Reply::not_found()
        }
    }
}

#[tokio::test]
async fn dosya_posts_the_anonymous_form_and_reads_the_code_from_the_result_page() {
    let mock = Mock::start(dosya("/dosya.co/?&fn=dw803ifqgmbr&st=OK&op=upload_result")).await;
    let run = run_default(&*builtin("dosya.co"), &mock).await;

    assert_eq!(run.url(), "https://dosya.co/dw803ifqgmbr");
    let upload = mock
        .requests()
        .into_iter()
        .find(|r| r.method == "POST")
        .unwrap();
    assert_eq!(upload.query, "upload_id=");
    let form = upload.multipart();
    assert_eq!(text_field(&form, "upload_type"), "file");
    assert_eq!(text_field(&form, "utype"), "anon");
    assert_eq!(text_field(&form, "sess_id"), "");
    assert_eq!(field(&form, "file_1").data, b"0123456789abcdef");
    upload.assert_not_chunked();
}

#[tokio::test]
async fn filespace_builds_its_own_link() {
    let mock = Mock::start(|r| {
        if r.method == "GET" && r.host == "filespace.com" {
            Reply::html(r#"<form action="https://monica.filespace.com/cgi-bin/upload.cgi?upload_id=" id='fupl1'>"#)
        } else if r.method == "POST" && r.host == "monica.filespace.com" {
            Reply::text("")
                .status(302)
                .header("location", "/filespace.com/?&fn=d3rk949tlktd&st=OK&op=upload_result")
        } else {
            Reply::not_found()
        }
    })
    .await;
    let run = run_default(&*builtin("filespace"), &mock).await;
    assert_eq!(run.url(), "https://filespace.com/d3rk949tlktd");
}

#[tokio::test]
async fn a_status_other_than_ok_is_reported() {
    let mock = Mock::start(dosya(
        "/dosya.co/?&fn=abc&st=file%20too%20big&op=upload_result",
    ))
    .await;
    let run = run_default(&*builtin("dosya.co"), &mock).await;
    let message = run.error().to_string();
    assert!(message.contains("file too big"), "{message}");
}

#[tokio::test]
async fn an_upload_that_does_not_end_on_a_result_page_is_an_error() {
    let mock = Mock::start(|r| {
        if r.method == "GET" {
            Reply::html(HOME)
        } else {
            Reply::html("<html>Upload failed</html>")
        }
    })
    .await;
    let run = run_default(&*builtin("dosya.co"), &mock).await;
    assert!(matches!(run.error(), UploadError::Protocol { .. }));
}

#[tokio::test]
async fn a_file_code_that_is_not_plain_letters_and_digits_is_refused() {
    for code in ["..%2F..%2Fetc", "a%20b", ""] {
        let location: &'static str =
            Box::leak(format!("/dosya.co/?&fn={code}&st=OK&op=upload_result").into_boxed_str());
        let mock = Mock::start(dosya(location)).await;
        let run = run_default(&*builtin("dosya.co"), &mock).await;
        assert!(
            matches!(run.error(), UploadError::Protocol { .. }),
            "`{code}` was accepted"
        );
    }
    // The JSON style gets the same check.
    let mock = Mock::start(|r| {
        if r.method == "GET" {
            Reply::html(r#"<form action="https://srv7.clicknupload.net/cgi-bin/upload.cgi">"#)
        } else {
            Reply::json(json!([{"file_code": "a/b", "file_status": "OK"}]))
        }
    })
    .await;
    let run = run_default(&*builtin("clicknupload"), &mock).await;
    assert!(matches!(run.error(), UploadError::Protocol { .. }));
}
