//! Wire-level tests for the single-request sites.
//!
//! Each test pins what the Python version sent (field names, headers, paths)
//! and what it extracted from the reply, so a refactor can't silently change
//! the protocol.

mod common;

use common::*;
use mul_tor_core::UploadError;
use serde_json::json;

fn only_request(mock: &Mock) -> Recorded {
    let requests = mock.requests();
    assert_eq!(requests.len(), 1, "expected one request, got {requests:#?}");
    requests.into_iter().next().unwrap()
}

// ---------------------------------------------------------------- raw bodies

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
async fn legacy_site_spellings_still_resolve() {
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

// ----------------------------------------------------------------- multipart

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
