//! Providers behind Laravel-style CSRF tokens and session cookies.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use mul_tor_core::UploadError;
use mul_tor_core::providers::{AnonTransfer, EasyUpload, Fastupload};
use serde_json::json;

use crate::common::*;
use crate::support::*;

// ----------------------------------------------------- Laravel + cookie providers

const HOME: &str = r#"<html><head><meta name="csrf-token" content="CSRF-1" /></head></html>"#;

#[tokio::test]
async fn anonymfile_replays_the_csrf_token_and_cookies() {
    let mock = Mock::start(|r| {
        if r.is("GET", "anonymfile.com", "/") {
            Reply::html(HOME)
                .header("set-cookie", "XSRF-TOKEN=xs%3D1; Path=/")
                .header("set-cookie", "anonymfile_session=sess1; Path=/; HttpOnly")
        } else if r.is("POST", "anonymfile.com", "/upload") {
            Reply::text("QwErTy")
        } else {
            Reply::not_found()
        }
    })
    .await;
    let run = run_default(&*builtin("anonymfile"), &mock).await;
    assert_eq!(run.url(), "https://anonymfile.com/QwErTy");

    let upload = mock.requests().pop().unwrap();
    assert_eq!(upload.header("x-csrf-token"), Some("CSRF-1"));
    assert_eq!(
        upload.header("cookie"),
        Some("XSRF-TOKEN=xs%3D1; anonymfile_session=sess1")
    );
    assert_eq!(upload.header("accept"), Some("application/json"));
    field(&upload.multipart(), "file");
}

#[tokio::test]
async fn anonymfile_needs_all_three_session_values() {
    let mock = Mock::start(|_| Reply::html(HOME)).await; // no cookies
    let run = run_default(&*builtin("anonymfile"), &mock).await;
    assert!(run.error().to_string().contains("CSRF"));
    assert_eq!(mock.requests().len(), 1);
}

fn fastupload_handler() -> impl Fn(&Recorded) -> Reply {
    let chunks = Arc::new(AtomicUsize::new(0));
    move |r| {
        if r.is("GET", "fastupload.io", "/") {
            Reply::html(HOME)
                .header("set-cookie", "XSRF-TOKEN=x0; Path=/")
                .header("set-cookie", "filebob_user_session=s0; Path=/")
        } else if r.is("POST", "fastupload.io", "/upload") {
            let n = chunks.fetch_add(1, Ordering::SeqCst) + 1;
            // Every reply rotates the session; only the last carries the link.
            Reply::json(json!({"download_link": format!("https://fastupload.io/d/{n}")}))
                .header("set-cookie", &format!("filebob_user_session=s{n}; Path=/"))
        } else {
            Reply::not_found()
        }
    }
}

#[tokio::test]
async fn fastupload_sends_dropzone_fields_and_rotates_the_session_cookie() {
    let mock = Mock::start(fastupload_handler()).await;
    let provider = Fastupload::with_chunk_size(10);
    let run = run(&provider, &mock, config_25()).await;
    assert_eq!(run.url(), "https://fastupload.io/d/3");

    let chunks = chunks_of(&mock, "fastupload.io", "/upload");
    assert_eq!(chunks.len(), 3);
    // Cookie of request N is the session handed out by reply N-1 (XSRF unchanged).
    let cookies: Vec<_> = chunks
        .iter()
        .map(|c| c.header("cookie").unwrap().to_string())
        .collect();
    assert_eq!(cookies[0], "XSRF-TOKEN=x0; filebob_user_session=s0");
    assert_eq!(cookies[1], "XSRF-TOKEN=x0; filebob_user_session=s1");
    assert_eq!(cookies[2], "XSRF-TOKEN=x0; filebob_user_session=s2");
    assert_eq!(chunks[0].header("x-csrf-token"), Some("CSRF-1"));

    let uuid = text_field(&chunks[0].multipart(), "dzuuid");
    for (i, chunk) in chunks.iter().enumerate() {
        let parts = chunk.multipart();
        assert_eq!(text_field(&parts, "dzuuid"), uuid);
        assert_eq!(text_field(&parts, "dzchunkindex"), i.to_string());
        assert_eq!(text_field(&parts, "dztotalfilesize"), "25");
        assert_eq!(text_field(&parts, "dzchunksize"), "10");
        assert_eq!(text_field(&parts, "dztotalchunkcount"), "3");
        assert_eq!(
            text_field(&parts, "dzchunkbyteoffset"),
            (i * 10).to_string()
        );
        assert_eq!(text_field(&parts, "size"), "25");
        assert_eq!(text_field(&parts, "upload_auto_delete"), "0");
        chunk.assert_not_chunked();
    }
    let data: Vec<u8> = chunks
        .iter()
        .flat_map(|c| field(&c.multipart(), "file").data.clone())
        .collect();
    assert_eq!(
        data, CONTENT_25,
        "chunks must reassemble to the original file"
    );
    assert_eq!(run.progress.sent(), 25);
}

#[tokio::test]
async fn easyupload_validates_with_the_same_uuid_it_then_uploads_under() {
    let mock = Mock::start(|r| {
        if r.is("GET", "easyupload.io", "/") {
            Reply::html(r#"<form action="https://upload3.easyupload.io/action.php">"#)
        } else if r.is("POST", "easyupload.io", "/action.php") {
            Reply::text("<br />Notice: odd\n{\"status\":true}")
        } else if r.is("POST", "upload3.easyupload.io", "/action.php") {
            Reply::text("{\"download_link\":\"https://easyupload.io/xyz\"}")
        } else {
            Reply::not_found()
        }
    })
    .await;
    let provider = EasyUpload::with_chunk_size(10);
    let run = run(&provider, &mock, config_25()).await;
    assert_eq!(run.url(), "https://easyupload.io/xyz");

    let validate = mock
        .requests_to("easyupload.io")
        .into_iter()
        .find(|r| r.method == "POST")
        .unwrap();
    assert_eq!(validate.header("referer"), Some("https://easyupload.io/"));
    assert_eq!(validate.header("accept"), Some("application/json"));
    let form = validate.form();
    assert!(form.contains(&("type".into(), "validate_tuf".into())));
    let uuid = form.iter().find(|(k, _)| k == "files[]").unwrap().1.clone();

    let chunks = chunks_of(&mock, "upload3.easyupload.io", "/action.php");
    assert_eq!(chunks.len(), 3);
    for (i, chunk) in chunks.iter().enumerate() {
        let parts = chunk.multipart();
        assert_eq!(text_field(&parts, "dzuuid"), uuid);
        assert_eq!(text_field(&parts, "dzchunkindex"), i.to_string());
        assert_eq!(text_field(&parts, "type"), "dropzone-multi-upload");
        assert_eq!(text_field(&parts, "expiration"), "30");
        assert_eq!(chunk.header("referer"), Some("https://easyupload.io/"));
    }
}

#[tokio::test]
async fn easyupload_stops_when_validation_is_refused() {
    let mock = Mock::start(|r| {
        if r.method == "GET" {
            Reply::html("https://upload1.easyupload.io/action.php")
        } else {
            Reply::text("{\"status\":false}")
        }
    })
    .await;
    let run = run_default(&*builtin("easyupload"), &mock).await;
    assert!(matches!(run.error(), UploadError::Protocol { .. }));
    assert_eq!(mock.requests().len(), 2, "no chunk may be sent");
}

#[tokio::test]
async fn anontransfer_chunks_without_extra_fields() {
    let mock = Mock::start(|_| Reply::json(json!({"name": "n.bin", "dir": "d1"}))).await;
    let provider = AnonTransfer::with_chunk_size(10);
    let run = run(&provider, &mock, config_25()).await;
    assert_eq!(run.url(), "https://anontransfer.com/download/d1/n.bin");

    let requests = mock.requests();
    assert_eq!(requests.len(), 3);
    let names: Vec<_> = requests[0]
        .multipart()
        .iter()
        .map(|p| p.name.clone())
        .collect();
    assert_eq!(
        names,
        [
            "dzuuid",
            "dzchunkindex",
            "dztotalfilesize",
            "dzchunksize",
            "dztotalchunkcount",
            "dzchunkbyteoffset",
            "file"
        ]
    );
}
