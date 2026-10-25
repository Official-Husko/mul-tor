//! Providers that send one multipart form.

use mul_tor_core::UploadError;
use serde_json::json;

use crate::common::*;
use crate::support::*;

#[tokio::test]
async fn oshi_extracts_the_last_dl_line() {
    let mock =
        Mock::start(|_| Reply::text("MANAGE: https://oshi.at/a/manage\nDL: https://oshi.at/xyz\n"))
            .await;
    let run = run_default(&*builtin("oshi"), &mock).await;
    assert_eq!(run.url(), "https://oshi.at/xyz");
    let request = only_request(&mock);
    assert_eq!(request.query, "shorturl=1");
    assert_eq!(
        field(&request.multipart(), "file").data,
        b"0123456789abcdef"
    );
}

#[tokio::test]
async fn oshi_errors_when_there_is_no_dl_line() {
    let mock = Mock::start(|_| Reply::text("MANAGE: https://oshi.at/a/manage\n")).await;
    let run = run_default(&*builtin("oshi"), &mock).await;
    assert!(matches!(run.error(), UploadError::Protocol { .. }));
}

#[tokio::test]
async fn tmpfiles_reads_data_url() {
    let mock = Mock::start(|_| {
        Reply::json(json!({"status": "success", "data": {"url": "https://tmpfiles.org/1/f.bin"}}))
    })
    .await;
    let run = run_default(&*builtin("tmpfiles"), &mock).await;
    assert_eq!(run.url(), "https://tmpfiles.org/1/f.bin");
    let request = only_request(&mock);
    assert_eq!(request.path, "/api/v1/upload");
    field(&request.multipart(), "file");
}

#[tokio::test]
async fn tempsend_uses_the_url_it_was_redirected_to() {
    let mock = Mock::start(|r| {
        if r.is("POST", "tempsend.com", "/send") {
            Reply::text("")
                .status(302)
                .header("location", "/tempsend.com/AbC123")
        } else if r.is("GET", "tempsend.com", "/AbC123") {
            Reply::html("<html>file page</html>")
        } else {
            Reply::not_found()
        }
    })
    .await;
    let run = run_default(&*builtin("tempsend"), &mock).await;
    assert!(run.url().ends_with("/tempsend.com/AbC123"), "{}", run.url());

    let parts = mock.requests()[0].multipart();
    assert_eq!(text_field(&parts, "expire"), "31536000");
    field(&parts, "file");
}

#[tokio::test]
async fn tempsend_without_a_redirect_is_an_error() {
    let mock = Mock::start(|_| Reply::html("<html>form again</html>")).await;
    let run = run_default(&*builtin("tempsend"), &mock).await;
    assert!(matches!(run.error(), UploadError::Protocol { .. }));
}

#[tokio::test]
async fn file_si_reads_the_short_url() {
    let mock = Mock::start(|_| {
        Reply::json(json!({"data": {"file": {"url": {"short": "https://file.si/abc"}}}}))
    })
    .await;
    let run = run_default(&*builtin("file.si"), &mock).await;
    assert_eq!(run.url(), "https://file.si/abc");
    assert_eq!(only_request(&mock).path, "/api/v1/upload");
}

#[tokio::test]
async fn file_upload_net_builds_the_link_from_name_and_id() {
    let mock = Mock::start(|_| Reply::json(json!({"name": "sample.bin", "id": "9981"}))).await;
    let run = run_default(&*builtin("fileupload"), &mock).await;
    assert_eq!(
        run.url(),
        "https://file-upload.net/download-9981/sample.bin.html"
    );
    let request = only_request(&mock);
    assert!(request.is("POST", "up.file-upload.net", "/upload.php"));
    assert_eq!(
        text_field(&request.multipart(), "description"),
        "Uploaded using Mul-Tor on GitHub!"
    );
}

#[tokio::test]
async fn download_gg_swaps_ampersands_in_the_id() {
    let mock = Mock::start(|_| Reply::text("ab&cd")).await;
    let run = run_default(&*builtin("download.gg"), &mock).await;
    assert_eq!(run.url(), "https://download.gg/file-ab_cd");
    // This host wants the PHP array-style field name, untouched.
    let parts = only_request(&mock).multipart();
    assert_eq!(parts[0].name, "file[]");
}

#[tokio::test]
async fn delafil_sends_max_chunk_size_and_files_array() {
    let mock = Mock::start(|_| Reply::json(json!([{"url": "https://delafil.se/abc"}]))).await;
    let run = run_default(&*builtin("delafil"), &mock).await;
    assert_eq!(run.url(), "https://delafil.se/abc");
    let parts = only_request(&mock).multipart();
    assert_eq!(text_field(&parts, "maxChunkSize"), "2000000000");
    // As the Python version sent it (via `requests`' `files=`): a file-typed
    // part with the field name as its filename and no Content-Type.
    let max_chunk = field(&parts, "maxChunkSize");
    assert_eq!(max_chunk.filename.as_deref(), Some("maxChunkSize"));
    assert_eq!(max_chunk.content_type, None);
    assert_eq!(field(&parts, "files[]").data, b"0123456789abcdef");
}

#[tokio::test]
async fn files_dp_ua_links_to_the_id_the_client_chose() {
    let mock = Mock::start(|_| Reply::text("ok")).await;
    let run = run_default(&*builtin("files.dp.ua"), &mock).await;
    let parts = only_request(&mock).multipart();
    let id = text_field(&parts, "idfile");
    assert_eq!(id.len(), 10);
    assert_eq!(text_field(&parts, "accesscode").len(), 7);
    field(&parts, "files[]");
    assert_eq!(run.url(), format!("https://files.dp.ua/en/{id}"));
}

#[tokio::test]
async fn mixdrop_authenticates_in_the_form() {
    let mock =
        Mock::start(|_| Reply::json(json!({"success": true, "result": {"fileref": "r1abc"}})))
            .await;
    let run = run(
        &*builtin("mixdrop"),
        &mock,
        RunConfig {
            credentials: creds(&[("email", "me@example.com"), ("apiKey", "K")]),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(run.url(), "https://mixdrop.ag/f/r1abc");
    let parts = only_request(&mock).multipart();
    assert_eq!(text_field(&parts, "email"), "me@example.com");
    assert_eq!(text_field(&parts, "key"), "K");
    field(&parts, "file");
}

#[tokio::test]
async fn mixdrop_reports_success_false() {
    let mock =
        Mock::start(|_| Reply::json(json!({"success": false, "error": {"msg": "bad key"}}))).await;
    let run = run(
        &*builtin("mixdrop"),
        &mock,
        RunConfig {
            credentials: creds(&[("email", "e"), ("apiKey", "k")]),
            ..Default::default()
        },
    )
    .await;
    assert!(matches!(run.error(), UploadError::Protocol { .. }));
}

#[tokio::test]
async fn litterbox_requests_72_hours() {
    let mock = Mock::start(|_| Reply::text("https://litter.catbox.moe/abc.bin")).await;
    let run = run_default(&*builtin("litterbox"), &mock).await;
    assert_eq!(run.url(), "https://litter.catbox.moe/abc.bin");
    let request = only_request(&mock);
    assert!(request.is(
        "POST",
        "litterbox.catbox.moe",
        "/resources/internals/api.php"
    ));
    assert_eq!(text_field(&request.multipart(), "time"), "72h");
}
