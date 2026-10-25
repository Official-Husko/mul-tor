//! GigaFile: the upload server comes from the home page, chunks share a cookie.

use mul_tor_core::UploadError;
use mul_tor_core::providers::GigaFile;
use serde_json::json;

use crate::common::*;
use crate::support::*;

const HOME: &str = r#"<script>var chunk_size = "100mb"; var server = "111.gigafile.nu"; var runtimes = "html5";</script>"#;

/// A server that sets a session cookie on the first chunk and only accepts
/// later chunks that send it back, as the real one does.
fn gigafile(home: &'static str) -> impl Fn(&Recorded) -> Reply {
    move |r| {
        if r.is("GET", "gigafile.nu", "/") {
            Reply::html(home)
        } else if r.is("POST", "111.gigafile.nu", "/upload_chunk.php") {
            let parts = r.multipart();
            let chunk: u32 = text_field(&parts, "chunk").parse().unwrap();
            let chunks: u32 = text_field(&parts, "chunks").parse().unwrap();
            let has_cookie = r.header("cookie").is_some_and(|c| c.contains("gfsid=S1"));
            if chunk > 0 && !has_cookie {
                return Reply::json(json!({"status": 104, "message": "cookie"}));
            }
            if chunk + 1 == chunks {
                Reply::json(json!({
                    "status": 0,
                    "url": "https://111.gigafile.nu/0202-b52dfdbdd0c587452879509871d6cb811",
                    "delkey": "e408",
                }))
            } else {
                Reply::json(json!({"status": 0})).header("set-cookie", "gfsid=S1; path=/")
            }
        } else {
            Reply::not_found()
        }
    }
}

#[tokio::test]
async fn gigafile_sends_numbered_chunks_that_share_the_session_cookie() {
    let mock = Mock::start(gigafile(HOME)).await;
    let run = run(&GigaFile::with_chunk_size(10), &mock, config_25()).await;

    assert_eq!(
        run.url(),
        "https://111.gigafile.nu/0202-b52dfdbdd0c587452879509871d6cb811"
    );
    let chunks = chunks_of(&mock, "111.gigafile.nu", "/upload_chunk.php");
    assert_eq!(chunks.len(), 3);
    let mut ids = std::collections::HashSet::new();
    for (index, chunk) in chunks.iter().enumerate() {
        let form = chunk.multipart();
        assert_eq!(text_field(&form, "chunk"), index.to_string());
        assert_eq!(text_field(&form, "chunks"), "3");
        assert_eq!(text_field(&form, "name"), "data.bin");
        assert_eq!(text_field(&form, "lifetime"), "100");
        ids.insert(text_field(&form, "id"));
        chunk.assert_not_chunked();
    }
    assert_eq!(ids.len(), 1, "every chunk names the same upload");
    assert!(ids.iter().next().unwrap().len() == 32);
    let sent: Vec<u8> = chunks
        .iter()
        .flat_map(|c| field(&c.multipart(), "file").data.clone())
        .collect();
    assert_eq!(sent, CONTENT_25);
    assert_eq!(run.progress.sent(), 25);
}

#[tokio::test]
async fn gigafile_keeps_a_small_file_in_one_chunk() {
    let mock = Mock::start(gigafile(HOME)).await;
    let run = run_default(&GigaFile::default(), &mock).await;
    assert!(run.url().starts_with("https://111.gigafile.nu/"));
    assert_eq!(
        chunks_of(&mock, "111.gigafile.nu", "/upload_chunk.php").len(),
        1
    );
}

#[tokio::test]
async fn gigafile_reports_the_status_the_server_gives() {
    let mock = Mock::start(|r| {
        if r.method == "GET" {
            Reply::html(HOME)
        } else {
            Reply::json(json!({"status": 104, "message": "no directory"}))
        }
    })
    .await;
    let run = run_default(&GigaFile::default(), &mock).await;
    let message = run.error().to_string();
    assert!(message.contains("104"), "{message}");
}

#[tokio::test]
async fn gigafile_does_not_send_the_file_to_a_server_off_its_domain() {
    for page in [
        r#"var server = "evil.example";"#,
        r#"var server = "gigafile.nu.evil.example";"#,
        "no script here",
    ] {
        let mock = Mock::start(gigafile(page)).await;
        let run = run_default(&GigaFile::default(), &mock).await;
        assert!(
            matches!(run.error(), UploadError::Protocol { .. }),
            "{page}"
        );
        assert_eq!(mock.requests().len(), 1, "{page}");
    }
}

#[tokio::test]
async fn gigafile_refuses_a_link_that_leaves_its_domain() {
    let mock = Mock::start(|r| {
        if r.method == "GET" {
            Reply::html(HOME)
        } else {
            Reply::json(json!({"status": 0, "url": "https://evil.example/x"}))
        }
    })
    .await;
    let run = run_default(&GigaFile::default(), &mock).await;
    assert!(matches!(run.error(), UploadError::Protocol { .. }));
}
