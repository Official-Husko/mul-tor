//! FileMirage: a server from the API, then numbered chunks.

use mul_tor_core::UploadError;
use mul_tor_core::providers::FileMirage;
use serde_json::json;

use crate::common::*;
use crate::support::*;

fn filemirage(server: &'static str) -> impl Fn(&Recorded) -> Reply {
    move |r| {
        if r.is("GET", "filemirage.com", "/api/servers") {
            Reply::json(json!({"success": true, "data": {"server": server, "upload_id": "x"}}))
        } else if r.method == "POST" && r.path == "/upload.php" {
            let parts = r.multipart();
            let last = text_field(&parts, "chunk_number").parse::<u32>().unwrap() + 1
                == text_field(&parts, "total_chunks").parse::<u32>().unwrap();
            if last {
                Reply::json(
                    json!({"success": true, "data": {"url": "https://filemirage.com/file/fxj2xfmh"}}),
                )
            } else {
                Reply::json(json!({"status": "success", "message": "Chunk uploaded"}))
            }
        } else {
            Reply::not_found()
        }
    }
}

#[tokio::test]
async fn filemirage_sends_numbered_chunks_to_the_server_it_is_given() {
    let mock = Mock::start(filemirage("https://store1.filemirage.com")).await;
    let run = run(&FileMirage::with_chunk_size(10), &mock, config_25()).await;

    assert_eq!(run.url(), "https://filemirage.com/file/fxj2xfmh");
    let chunks = chunks_of(&mock, "store1.filemirage.com", "/upload.php");
    assert_eq!(chunks.len(), 3);
    let mut ids = std::collections::HashSet::new();
    for (index, chunk) in chunks.iter().enumerate() {
        let form = chunk.multipart();
        assert_eq!(text_field(&form, "chunk_number"), index.to_string());
        assert_eq!(text_field(&form, "total_chunks"), "3");
        assert_eq!(text_field(&form, "filename"), "data.bin");
        ids.insert(text_field(&form, "upload_id"));
        chunk.assert_not_chunked();
    }
    assert_eq!(ids.len(), 1, "every chunk names the same upload");
    let sent: Vec<u8> = chunks
        .iter()
        .flat_map(|c| field(&c.multipart(), "file").data.clone())
        .collect();
    assert_eq!(sent, CONTENT_25);
    assert_eq!(run.progress.sent(), 25);
}

#[tokio::test]
async fn filemirage_keeps_a_small_file_in_one_chunk_with_no_expiry() {
    let mock = Mock::start(filemirage("https://store1.filemirage.com")).await;
    let run = run_default(&FileMirage::default(), &mock).await;

    assert_eq!(run.url(), "https://filemirage.com/file/fxj2xfmh");
    let chunks = chunks_of(&mock, "store1.filemirage.com", "/upload.php");
    assert_eq!(chunks.len(), 1);
    let names: Vec<_> = chunks[0].multipart().into_iter().map(|p| p.name).collect();
    // Nothing that would remove the file early.
    assert_eq!(
        names,
        [
            "file",
            "filename",
            "upload_id",
            "chunk_number",
            "total_chunks"
        ]
    );
}

#[tokio::test]
async fn filemirage_does_not_send_the_file_to_a_server_off_its_domain() {
    for server in [
        "https://evil.example",
        "https://filemirage.com.evil.example",
        "http://store1.filemirage.com",
    ] {
        let mock = Mock::start(filemirage(server)).await;
        let run = run_default(&FileMirage::default(), &mock).await;
        assert!(
            matches!(run.error(), UploadError::Protocol { .. }),
            "{server}"
        );
        assert_eq!(mock.requests().len(), 1, "{server}");
    }
}

#[tokio::test]
async fn filemirage_fails_when_the_last_chunk_brings_no_link() {
    let mock = Mock::start(|r| {
        if r.host == "filemirage.com" {
            Reply::json(json!({"data": {"server": "https://store1.filemirage.com"}}))
        } else {
            Reply::json(json!({"status": "success", "message": "Chunk uploaded"}))
        }
    })
    .await;
    let run = run_default(&FileMirage::default(), &mock).await;
    assert!(run.error().to_string().contains("/data/url"));
}
