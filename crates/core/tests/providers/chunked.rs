//! Numbered-chunk providers (uFile, DooDrive).

use mul_tor_core::providers::{DooDrive, UFile};
use serde_json::json;

use crate::common::*;
use crate::support::*;

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
    let provider = UFile::with_chunk_size(10);
    let run = run(&provider, &mock, config_25()).await;
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
