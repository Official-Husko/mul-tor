//! The jQuery File Upload family: `Content-Range` chunks.

use mul_tor_core::ProviderInfo;
use mul_tor_core::providers::JqfuProvider;
use serde_json::json;

use crate::common::*;
use crate::support::*;

// --------------------------------------------------- jQuery File Upload family

fn jqfu_test_provider(xhr: bool, fields: &'static [(&'static str, &'static str)]) -> JqfuProvider {
    JqfuProvider::new(
        ProviderInfo::new("T", "https://t.example/", "https://t.example/"),
        "https://t.example/up",
        10,
        fields,
        xhr,
    )
}

#[tokio::test]
async fn jqfu_sends_content_range_for_each_chunk() {
    let mock = Mock::start(|_| Reply::json(json!([{"url": "https://t.example/f/1"}]))).await;
    let provider = jqfu_test_provider(true, &[("uploadSource", "file_manager")]);
    let run = run(&provider, &mock, config_25()).await;
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
    let run = run_default(&jqfu_test_provider(false, &[]), &mock).await;
    run.url();
    let request = &mock.requests()[0];
    assert!(request.headers.get("x-requested-with").is_none());
    // 16 bytes in chunks of 10: the first request covers bytes 0-9.
    assert_eq!(request.header("content-range"), Some("bytes 0-9/16"));
}

#[tokio::test]
async fn the_four_registered_jqfu_providers_differ_only_in_their_quirks() {
    // (provider, host+path of its handler, sends X-Requested-With, sends uploadSource)
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
