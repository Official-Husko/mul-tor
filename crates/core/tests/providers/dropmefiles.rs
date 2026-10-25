//! DropMeFiles: a server and chunk size from the page, a created transfer,
//! raw chunks, a password flag, a save.

use serde_json::{Value, json};

use crate::common::*;
use crate::support::*;

fn page(chunk_mib: u32) -> String {
    format!(
        "<script>var SERVERID = '3';\nvar CHUNKSIZE = '{chunk_mib}m';\nvar MAXSIZE = 50;</script>"
    )
}

/// The site as the page's script sees it; `created` is the reply to `create`.
fn site(chunk_mib: u32, created: Value) -> impl Fn(&Recorded) -> Reply {
    let home = page(chunk_mib);
    move |r| {
        if r.is("GET", "dropmefiles.com", "/") {
            Reply::html(home.clone()).header("Set-Cookie", "PHPSESSID=sess1; path=/")
        } else if r.is("POST", "dropmefiles.com", "/s3/upload/create") {
            Reply::json(created.clone())
        } else if r.is("POST", "dropmefiles.com", "/s3/uploadrmbl")
            || r.is("POST", "dropmefiles.com", "/s3/uploadch")
        {
            Reply::text("")
        } else if r.is("POST", "dropmefiles.com", "/s3/upload/password") {
            Reply::json(json!({"jsonrpc": "2.0", "password": "", "id": "id"}))
        } else if r.is("POST", "dropmefiles.com", "/s3/upload/save") {
            Reply::json(json!({"jsonrpc": "2.0", "result": "Saved", "ziped": "0"}))
        } else {
            Reply::not_found()
        }
    }
}

/// A name the site does not run through its virus check (`.bin` is one it does).
fn text_config() -> RunConfig {
    RunConfig {
        name: "sample.txt".into(),
        ..Default::default()
    }
}

fn created(uid: &str) -> Value {
    json!({"jsonrpc": "2.0", "result": uid, "id": 1209600})
}

fn posts(mock: &Mock, path: &str) -> Vec<Recorded> {
    chunks_of(mock, "dropmefiles.com", path)
}

fn form_value(request: &Recorded, name: &str) -> String {
    request
        .form()
        .into_iter()
        .find(|(key, _)| key == name)
        .unwrap_or_else(|| panic!("no form field {name}"))
        .1
}

#[tokio::test]
async fn dropmefiles_creates_uploads_and_saves_the_way_the_page_does() {
    let mock = Mock::start(site(4, created("vFIo1"))).await;
    let run = run(&*builtin("dropmefiles"), &mock, text_config()).await;

    assert_eq!(run.url(), "https://dropmefiles.com/vFIo1");
    let create = &posts(&mock, "/s3/upload/create")[0];
    assert_eq!(form_value(create, "period"), "3", "the longest it offers");
    assert_eq!(form_value(create, "size"), "16");

    let chunk = &posts(&mock, "/s3/uploadrmbl")[0];
    assert_eq!(
        chunk.query_pairs(),
        [
            ("name".to_string(), "sample.txt".to_string()),
            ("chunk".into(), "0".into()),
            ("chunks".into(), "1".into()),
            ("updir".into(), "vFIo1".into()),
        ]
    );
    assert_eq!(
        chunk.header("content-type"),
        Some("application/octet-stream")
    );
    assert_eq!(chunk.body, b"0123456789abcdef");
    chunk.assert_not_chunked();
    // The resumable-upload headers of the page's uploader.
    let upload_id = chunk.header("session-id").unwrap();
    assert!(upload_id.starts_with("vFIo1_o_"), "{upload_id}");
    assert_eq!(
        chunk.header("content-disposition"),
        Some(format!("attachment; filename=\"{upload_id}\"").as_str())
    );
    assert_eq!(chunk.header("content-range"), Some("bytes 0-15/16"));

    let save = &posts(&mock, "/s3/upload/save")[0];
    assert_eq!(form_value(save, "uid"), "vFIo1");
    assert_eq!(form_value(save, "period"), "3");
    let files: Value = serde_json::from_str(&form_value(save, "files")).unwrap();
    assert_eq!(files[0]["name"], "sample.txt");
    assert_eq!(files[0]["size"], 16);
    assert_eq!(files[0]["dir"], "vFIo1");
    assert_eq!(
        upload_id,
        format!("vFIo1_{}", files[0]["id"].as_str().unwrap())
    );
    assert_eq!(run.progress.sent(), 16);
}

#[tokio::test]
async fn dropmefiles_marks_every_call_as_ajax_and_keeps_the_session_cookie() {
    let mock = Mock::start(site(4, created("vFIo1"))).await;
    run_default(&*builtin("dropmefiles"), &mock).await.url();

    let calls: Vec<_> = mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST")
        .collect();
    assert_eq!(calls.len(), 4);
    for call in calls {
        assert_eq!(
            call.header("x-requested-with"),
            Some("XMLHttpRequest"),
            "{}",
            call.path
        );
        assert_eq!(
            call.header("cookie"),
            Some("PHPSESSID=sess1"),
            "{}",
            call.path
        );
    }
}

#[tokio::test]
async fn dropmefiles_cuts_a_big_file_into_the_chunks_the_page_asks_for() {
    let content: Vec<u8> = (0..2_621_440u32).map(|i| (i % 251) as u8).collect();
    let config = RunConfig {
        name: "big.dat".into(),
        content: content.clone(),
        ..Default::default()
    };
    let mock = Mock::start(site(1, created("vFIo1"))).await;
    let run = run(&*builtin("dropmefiles"), &mock, config).await;

    assert_eq!(run.url(), "https://dropmefiles.com/vFIo1");
    let chunks = posts(&mock, "/s3/uploadrmbl");
    assert_eq!(chunks.len(), 3);
    for (index, chunk) in chunks.iter().enumerate() {
        let query = chunk.query_pairs();
        assert_eq!(query[1], ("chunk".to_string(), index.to_string()));
        assert_eq!(query[2], ("chunks".to_string(), "3".to_string()));
        let start = index * 1_048_576;
        let end = (start + 1_048_576).min(content.len()) - 1;
        let range = format!("bytes {start}-{end}/{}", content.len());
        assert_eq!(chunk.header("content-range"), Some(range.as_str()));
    }
    let sent: Vec<u8> = chunks.iter().flat_map(|c| c.body.clone()).collect();
    assert_eq!(sent, content);
    assert_eq!(run.progress.sent(), content.len() as u64);
}

#[tokio::test]
async fn dropmefiles_sends_archives_to_the_scanning_endpoint() {
    let config = RunConfig {
        name: "backup.zip".into(),
        ..Default::default()
    };
    let mock = Mock::start(site(4, created("vFIo1"))).await;
    run(&*builtin("dropmefiles"), &mock, config).await.url();

    assert_eq!(posts(&mock, "/s3/uploadch").len(), 1);
    assert!(posts(&mock, "/s3/uploadrmbl").is_empty());
}

#[tokio::test]
async fn dropmefiles_reports_what_the_site_refuses_and_stops() {
    let spam = json!({"jsonrpc": "2.0", "error": {"code": 99, "message": "Spam"}, "id": "id"});
    let mock = Mock::start(site(4, spam)).await;
    let run = run_default(&*builtin("dropmefiles"), &mock).await;

    assert!(run.error().to_string().contains("Spam"), "{}", run.error());
    assert!(posts(&mock, "/s3/uploadrmbl").is_empty());
}

#[tokio::test]
async fn dropmefiles_refuses_a_transfer_id_that_is_not_plain() {
    let mock = Mock::start(site(4, created("../evil"))).await;
    let run = run_default(&*builtin("dropmefiles"), &mock).await;

    assert!(run.error().to_string().contains("transfer id"));
    assert!(posts(&mock, "/s3/uploadrmbl").is_empty());
}
