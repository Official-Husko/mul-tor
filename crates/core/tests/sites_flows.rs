//! Wire-level tests for chunked and multi-step uploaders.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use common::*;
use mul_tor_core::sites::{
    AnonTransfer, DooDrive, EasyUpload, Fastupload, Gofile, JqfuSite, Rapidgator, UFile,
};
use mul_tor_core::{SiteInfo, UploadError};
use serde_json::json;

/// 25 bytes: three chunks of at most 10.
const CONTENT_25: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXY";

fn config_25() -> RunConfig {
    RunConfig {
        name: "data.bin".into(),
        content: CONTENT_25.to_vec(),
        ..Default::default()
    }
}

fn chunks_of(mock: &Mock, host: &str, path: &str) -> Vec<Recorded> {
    mock.requests()
        .into_iter()
        .filter(|r| r.host == host && r.path == path && r.method == "POST")
        .collect()
}

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
    let site = Gofile::with_delays(Duration::from_millis(1), Duration::from_millis(1));
    let run = run_default(&site, &mock).await;
    assert_eq!(run.url(), "https://gofile.io/d/Xy12");
    assert_eq!(mock.requests_to("api.gofile.io").len(), 3);
}

#[tokio::test]
async fn gofile_gives_up_instead_of_looping_forever() {
    let mock = Mock::start(gofile_handler(usize::MAX)).await;
    let site = Gofile::with_delays(Duration::from_millis(1), Duration::from_millis(1));
    let run = run_default(&site, &mock).await;
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

// ------------------------------------------------------------------ Files.fm

#[tokio::test]
async fn files_fm_uses_the_id_and_key_it_is_given() {
    let mock = Mock::start(|r| {
        if r.is("GET", "files.fm", "/server_scripts/get_upload_id.php") {
            Reply::text("ID123,unused,KEY456,more")
        } else if r.is("POST", "free.files.fm", "/save_file.php") {
            Reply::text("saved")
        } else if r.is("GET", "free.files.fm", "/finish_upload.php") {
            Reply::json(json!({"status": "ok"}))
        } else {
            Reply::not_found()
        }
    })
    .await;
    let run = run_default(&*builtin("filesfm"), &mock).await;
    assert_eq!(run.url(), "https://files.fm/u/ID123");

    let requests = mock.requests();
    assert_eq!(requests[0].query, "show_add_key=1");
    let upload = &requests[1];
    let q = upload.query_pairs();
    assert!(q.contains(&("up_id".into(), "ID123".into())));
    assert!(q.contains(&("key".into(), "KEY456".into())));
    assert!(q.contains(&("ignore_user_abort".into(), "1".into())));
    assert!(q.contains(&("skip_update".into(), "1".into())));
    field(&upload.multipart(), "Filedata");
    assert_eq!(requests[2].query, "upload_hash=ID123");
}

#[tokio::test]
async fn files_fm_fails_when_the_upload_is_not_finished() {
    let mock = Mock::start(|r| {
        if r.path.ends_with("get_upload_id.php") {
            Reply::text("A,b,C")
        } else if r.path.ends_with("finish_upload.php") {
            Reply::json(json!({"status": "error"}))
        } else {
            Reply::text("saved")
        }
    })
    .await;
    let run = run_default(&*builtin("filesfm"), &mock).await;
    assert!(matches!(run.error(), UploadError::Protocol { .. }));
}

// ------------------------------------------------------------------ 1Fichier

#[tokio::test]
async fn one_fichier_uploads_to_the_assigned_server() {
    let mock = Mock::start(|r| {
        if r.path == "/v1/upload/get_upload_server.cgi" {
            Reply::json(json!({"url": "a-1.1fichier.com", "id": "XID42"}))
        } else if r.is("POST", "a-1.1fichier.com", "/upload.cgi") {
            Reply::text("ok")
        } else if r.is("GET", "a-1.1fichier.com", "/end.pl") {
            Reply::json(json!({"links": [{"download": "https://1fichier.com/?abc123"}]}))
        } else {
            Reply::not_found()
        }
    })
    .await;
    let run = run_default(&*builtin("1fichier"), &mock).await;
    assert_eq!(run.url(), "https://1fichier.com/?abc123");

    let requests = mock.requests();
    assert_eq!(requests[1].query, "id=XID42");
    field(&requests[1].multipart(), "file[]");
    assert_eq!(requests[2].query, "xid=XID42");
    assert_eq!(requests[2].header("json"), Some("1"));
}

#[tokio::test]
async fn one_fichier_refuses_a_malformed_server_name() {
    let mock = Mock::start(|_| Reply::json(json!({"url": "evil.example/x", "id": "1"}))).await;
    let run = run_default(&*builtin("1fichier"), &mock).await;
    assert!(matches!(run.error(), UploadError::Protocol { .. }));
    assert_eq!(mock.requests().len(), 1);
}

// ------------------------------------------------------------------- File.io

#[tokio::test]
async fn file_io_patches_metadata_with_the_anonymous_token() {
    let mock = Mock::start(|r| {
        if r.is("POST", "file.io", "/") {
            Reply::json(json!({"link": "https://file.io/AbC", "key": "AbC"}))
                .header("x-file-io-anonymous-access-token", "TOKEN9")
        } else if r.is("PATCH", "file.io", "/AbC") {
            Reply::json(json!({"success": true}))
        } else {
            Reply::not_found()
        }
    })
    .await;
    let run = run(
        &*builtin("file.io"),
        &mock,
        RunConfig {
            name: "my report.pdf".into(),
            ..Default::default()
        },
    )
    .await;
    assert_eq!(run.url(), "https://file.io/AbC");

    let requests = mock.requests();
    assert!(
        requests[0]
            .query_pairs()
            .contains(&("title".into(), "my report.pdf".into()))
    );
    let patch = &requests[1];
    assert_eq!(patch.header("authorization"), Some("Bearer TOKEN9"));
    let body: serde_json::Value = serde_json::from_slice(&patch.body).unwrap();
    assert_eq!(body["title"], "my report.pdf");
    assert_eq!(body["expires"], "2M");
}

#[tokio::test]
async fn file_io_still_succeeds_when_the_metadata_patch_fails() {
    let mock = Mock::start(|r| {
        if r.method == "POST" {
            Reply::json(json!({"link": "https://file.io/AbC", "key": "AbC"}))
                .header("x-file-io-anonymous-access-token", "T")
        } else {
            Reply::text("boom").status(500)
        }
    })
    .await;
    let run = run_default(&*builtin("file.io"), &mock).await;
    assert_eq!(run.url(), "https://file.io/AbC");
}

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
    let site = Rapidgator::with_polling(Duration::from_millis(1), 10);
    let run = run(&site, &mock, rg_creds()).await;
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
    let site = Rapidgator::with_polling(Duration::from_millis(1), 10);
    let run = run(&site, &mock, rg_creds()).await;
    assert_eq!(run.url(), "https://rapidgator.net/file/known");
    assert!(mock.requests_to("u1.rapidgator.net").is_empty());
    assert_eq!(run.progress.sent(), 0);
}

#[tokio::test]
async fn rapidgator_enforces_the_account_size_limit_after_login() {
    let mock = Mock::start(rapidgator_handler(5, false)).await;
    let site = Rapidgator::with_polling(Duration::from_millis(1), 10);
    let run = run(&site, &mock, rg_creds()).await;
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
    let site = Rapidgator::with_polling(Duration::from_millis(1), 10);
    let run = run(&site, &mock, rg_creds()).await;
    let message = run.error().to_string();
    assert!(message.contains("Wrong login or password"), "{message}");
    assert!(!message.contains("pw"), "{message}");
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
    let site = Rapidgator::with_polling(Duration::from_millis(1), 3);
    let run = run(&site, &mock, rg_creds()).await;
    assert!(run.error().to_string().contains("in time"));
}

// ----------------------------------------------------- Laravel + cookie sites

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
    let site = Fastupload::with_chunk_size(10);
    let run = run(&site, &mock, config_25()).await;
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
    let site = EasyUpload::with_chunk_size(10);
    let run = run(&site, &mock, config_25()).await;
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
    let site = AnonTransfer::with_chunk_size(10);
    let run = run(&site, &mock, config_25()).await;
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

// --------------------------------------------------------------------- uFile

#[tokio::test]
async fn ufile_numbers_chunks_from_one_and_finalises_with_the_count() {
    let mock = Mock::start(|r| match r.path.as_str() {
        "/v1/upload/select_storage" => {
            Reply::json(json!({"storageBaseUrl": "https://up3.ufile.io/"}))
        }
        "/v1/upload/create_session" => Reply::json(json!({"fuid": "FU1"})),
        "/v1/upload/chunk" => Reply::json(json!({"status": true})),
        "/v1/upload/finalise" => Reply::json(json!({"url": "https://ufile.io/abc"})),
        _ => Reply::not_found(),
    })
    .await;
    let site = UFile::with_chunk_size(10);
    let run = run(&site, &mock, config_25()).await;
    assert_eq!(run.url(), "https://ufile.io/abc");

    let requests = mock.requests();
    assert!(requests[0].is("POST", "ufile.io", "/v1/upload/select_storage"));
    assert!(
        requests[1]
            .form()
            .contains(&("file_size".into(), "25".into()))
    );

    let chunks = chunks_of(&mock, "up3.ufile.io", "/v1/upload/chunk");
    let indices: Vec<_> = chunks
        .iter()
        .map(|c| text_field(&c.multipart(), "chunk_index"))
        .collect();
    assert_eq!(indices, ["1", "2", "3"]);
    for chunk in &chunks {
        assert_eq!(text_field(&chunk.multipart(), "fuid"), "FU1");
    }

    let finalise = requests.last().unwrap().form();
    assert!(finalise.contains(&("fuid".into(), "FU1".into())));
    assert!(finalise.contains(&("file_name".into(), "data.bin".into())));
    assert!(finalise.contains(&("file_type".into(), "unknown".into())));
    assert!(finalise.contains(&("total_chunks".into(), "3".into())));
}

// ------------------------------------------------------------------ DooDrive

#[tokio::test]
async fn doodrive_uses_the_chunk_size_the_server_announces() {
    let mock = Mock::start(|r| match r.path.as_str() {
        "/api/v1/upload" => Reply::json(json!({"status": "success", "data": {
            "token": "TK", "max_chunk_size": 10,
            "chunk_url": "https://doodrive.com/api/v1/upload/chunk",
            "complete_url": "https://doodrive.com/api/v1/upload/complete"}})),
        "/api/v1/upload/chunk" => Reply::json(json!({"status": "success"})),
        "/api/v1/upload/complete" => {
            Reply::json(json!({"status": "success", "data": {"url": "https://doodrive.com/f/zz"}}))
        }
        _ => Reply::not_found(),
    })
    .await;
    let run = run(
        &DooDrive,
        &mock,
        RunConfig {
            credentials: creds(&[("apiKey", "AK"), ("apiToken", "AT")]),
            ..config_25()
        },
    )
    .await;
    assert_eq!(run.url(), "https://doodrive.com/f/zz");

    let requests = mock.requests();
    let init = requests[0].form();
    assert!(init.contains(&("api_key".into(), "AK".into())));
    assert!(init.contains(&("api_token".into(), "AT".into())));
    assert!(init.contains(&("file_name".into(), "data.bin".into())));
    assert!(init.contains(&("file_size".into(), "25".into())));

    let chunks = chunks_of(&mock, "doodrive.com", "/api/v1/upload/chunk");
    assert_eq!(chunks.len(), 3);
    for (i, chunk) in chunks.iter().enumerate() {
        let parts = chunk.multipart();
        assert_eq!(text_field(&parts, "chunk_id"), i.to_string());
        assert_eq!(text_field(&parts, "token"), "TK");
        assert_eq!(text_field(&parts, "api_key"), "AK");
    }
    assert!(
        requests
            .last()
            .unwrap()
            .form()
            .contains(&("chunks".into(), "3".into()))
    );
}

#[tokio::test]
async fn doodrive_stops_when_a_chunk_is_rejected() {
    let mock = Mock::start(|r| match r.path.as_str() {
        "/api/v1/upload" => Reply::json(json!({"status": "success", "data": {
            "token": "TK", "max_chunk_size": 10,
            "chunk_url": "https://doodrive.com/c", "complete_url": "https://doodrive.com/d"}})),
        _ => Reply::json(json!({"status": "error"})),
    })
    .await;
    let run = run(
        &DooDrive,
        &mock,
        RunConfig {
            credentials: creds(&[("apiKey", "AK"), ("apiToken", "AT")]),
            ..config_25()
        },
    )
    .await;
    assert!(run.error().to_string().contains("chunk"));
    assert_eq!(mock.requests().len(), 2, "init + the one rejected chunk");
}

// --------------------------------------------------- jQuery File Upload family

fn jqfu_test_site(xhr: bool, fields: &'static [(&'static str, &'static str)]) -> JqfuSite {
    JqfuSite::new(
        SiteInfo::new("T", "https://t.example/", "https://t.example/"),
        "https://t.example/up",
        10,
        fields,
        xhr,
    )
}

#[tokio::test]
async fn jqfu_sends_content_range_for_each_chunk() {
    let mock = Mock::start(|_| Reply::json(json!([{"url": "https://t.example/f/1"}]))).await;
    let site = jqfu_test_site(true, &[("uploadSource", "file_manager")]);
    let run = run(&site, &mock, config_25()).await;
    assert_eq!(run.url(), "https://t.example/f/1");

    let requests = mock.requests();
    let ranges: Vec<_> = requests
        .iter()
        .map(|r| r.header("content-range").unwrap().to_string())
        .collect();
    assert_eq!(ranges, ["bytes 0-9/25", "bytes 10-19/25", "bytes 20-24/25"]);

    for request in &requests {
        assert_eq!(request.header("x-requested-with"), Some("XMLHttpRequest"));
        assert_eq!(request.header("accept"), Some("application/json"));
        let parts = request.multipart();
        assert_eq!(text_field(&parts, "maxChunkSize"), "10");
        assert_eq!(text_field(&parts, "uploadSource"), "file_manager");
        assert_eq!(
            field(&parts, "files[]").filename.as_deref(),
            Some("data.bin")
        );
        request.assert_not_chunked();
    }
    let data: Vec<u8> = requests
        .iter()
        .flat_map(|r| field(&r.multipart(), "files[]").data.clone())
        .collect();
    assert_eq!(data, CONTENT_25);
}

#[tokio::test]
async fn jqfu_xhr_header_is_optional() {
    let mock = Mock::start(|_| Reply::json(json!([{"url": "u"}]))).await;
    let run = run_default(&jqfu_test_site(false, &[]), &mock).await;
    run.url();
    let request = &mock.requests()[0];
    assert!(request.headers.get("x-requested-with").is_none());
    // 16 bytes in chunks of 10: the first request covers bytes 0-9.
    assert_eq!(request.header("content-range"), Some("bytes 0-9/16"));
}

#[tokio::test]
async fn the_four_registered_jqfu_sites_differ_only_in_their_quirks() {
    // (site, host+path of its handler, sends X-Requested-With, sends uploadSource)
    let cases = [
        (
            "1cloudfile",
            "1cloudfile.com",
            "/ajax/file_upload_handler",
            true,
            false,
        ),
        (
            "cyberfile",
            "cyberfile.me",
            "/ajax/file_upload_handler",
            false,
            true,
        ),
        (
            "filesadmin",
            "filesadmin.com",
            "/ajax/file_upload_handler",
            true,
            true,
        ),
        (
            "wdho",
            "wdho.ru",
            "/core/page/ajax/file_upload_handler.ajax.php",
            true,
            false,
        ),
    ];
    for (name, host, path, xhr, source) in cases {
        let mock = Mock::start(|_| Reply::json(json!([{"url": "https://x/ok"}]))).await;
        let run = run_default(&*builtin(name), &mock).await;
        assert_eq!(run.url(), "https://x/ok", "{name}");
        let request = &mock.requests()[0];
        assert!(
            request.is("POST", host, path),
            "{name}: {} {}",
            request.host,
            request.path
        );
        assert_eq!(
            request.headers.contains_key("x-requested-with"),
            xhr,
            "{name}"
        );
        assert_eq!(
            request.multipart().iter().any(|p| p.name == "uploadSource"),
            source,
            "{name}"
        );
    }
}

// -------------------------------------------------------------- XFS family

fn xfs_handler(
    page_host: &'static str,
    page_body: &'static str,
    upload_host: &'static str,
    upload_path: &'static str,
    reply: serde_json::Value,
) -> impl Fn(&Recorded) -> Reply {
    move |r| {
        if r.is("GET", page_host, "/") || (r.method == "GET" && r.host == page_host) {
            Reply::html(page_body)
        } else if r.is("POST", upload_host, upload_path) {
            Reply::json(reply.clone())
        } else {
            Reply::not_found()
        }
    }
}

#[tokio::test]
async fn clicknupload_posts_the_file_as_field_file() {
    let mock = Mock::start(xfs_handler(
        "clicknupload.click",
        r#"<form action="https://srv7.clicknupload.net/cgi-bin/upload.cgi?x=1">"#,
        "srv7.clicknupload.net",
        "/cgi-bin/upload.cgi",
        json!([{"file_code": "abc123", "file_status": "OK"}]),
    ))
    .await;
    let run = run_default(&*builtin("clicknupload"), &mock).await;
    assert_eq!(run.url(), "https://clicknupload.vip/abc123");
    let parts = mock.requests().pop().unwrap().multipart();
    assert_eq!(text_field(&parts, "utype"), "anon");
    assert_eq!(text_field(&parts, "file_public"), "1");
    assert_eq!(text_field(&parts, "keepalive"), "1");
    assert_eq!(
        text_field(&parts, "file_descr"),
        "Uploaded using Mul-Tor on GitHub!"
    );
    assert_eq!(field(&parts, "file").data, b"0123456789abcdef");
}

#[tokio::test]
async fn hexupload_reads_the_form_action_and_uses_file_0() {
    let mock = Mock::start(xfs_handler(
        "hexupload.net",
        r#"<form method="post" action = 'https://s5.hexupload.net/cgi-bin/upload.cgi?upload_type=file'>"#,
        "s5.hexupload.net",
        "/cgi-bin/upload.cgi",
        json!([{"file_code": "hx1", "file_status": "OK"}]),
    ))
    .await;
    let run = run_default(&*builtin("hexupload"), &mock).await;
    assert_eq!(run.url(), "https://hexupload.net/hx1");
    field(&mock.requests().pop().unwrap().multipart(), "file_0");
}

#[tokio::test]
async fn usercloud_uses_its_own_link_base() {
    let mock = Mock::start(xfs_handler(
        "userscloud.com",
        r#"<form action="https://u9.userscloud.com/cgi-bin/upload.cgi">"#,
        "u9.userscloud.com",
        "/cgi-bin/upload.cgi",
        json!([{"file_code": "uc9"}]),
    ))
    .await;
    let run = run_default(&*builtin("usercloud"), &mock).await;
    assert_eq!(run.url(), "https://userscloud.com/uc9");
}

#[tokio::test]
async fn usersdrive_matches_its_server_pattern_and_builds_html_links() {
    let mock = Mock::start(xfs_handler(
        "usersdrive.com",
        r#"var up = "https://dns3.userdrive.org/cgi-bin/upload.cgi?upload_type=file";"#,
        "dns3.userdrive.org",
        "/cgi-bin/upload.cgi",
        json!([{"file_code": "ud7", "file_status": "OK"}]),
    ))
    .await;
    let run = run_default(&*builtin("usersdrive"), &mock).await;
    assert_eq!(run.url(), "https://usersdrive.com/ud7.html");
    let upload = mock.requests().pop().unwrap();
    assert_eq!(upload.query, "upload_type=file");
    let parts = upload.multipart();
    assert_eq!(text_field(&parts, "utype"), "anon");
    assert_eq!(text_field(&parts, "keepalive"), "1");
    assert!(
        !parts.iter().any(|p| p.name == "file_public"),
        "UsersDrive never sent this"
    );
}

#[tokio::test]
async fn xfs_sites_report_a_rejected_file() {
    let mock = Mock::start(xfs_handler(
        "usersdrive.com",
        "https://dns3.userdrive.org/cgi-bin/upload.cgi?upload_type=file",
        "dns3.userdrive.org",
        "/cgi-bin/upload.cgi",
        json!([{"file_code": "", "file_status": "unauthorized"}]),
    ))
    .await;
    let run = run_default(&*builtin("usersdrive"), &mock).await;
    assert!(run.error().to_string().contains("unauthorized"));
}

#[tokio::test]
async fn xfs_sites_fail_cleanly_when_the_page_has_no_upload_server() {
    let mock = Mock::start(|_| Reply::html("<html>maintenance</html>")).await;
    let run = run_default(&*builtin("clicknupload"), &mock).await;
    assert!(run.error().to_string().contains("upload server"));
    assert_eq!(mock.requests().len(), 1);
}
