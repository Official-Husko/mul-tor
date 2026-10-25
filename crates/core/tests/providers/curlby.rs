//! curl.by: the file is the body of a POST; the link is in a `wget` line.

use mul_tor_core::UploadError;

use crate::common::*;
use crate::support::*;

const PAGE: &str = "\n=========================\n\nUploaded 1 file, 16 bytes\n\nwget https://curl.by/tio_C/sample.bin\n\n=========================\n\n";

#[tokio::test]
async fn curl_by_posts_the_raw_file_to_a_path_with_its_name() {
    let mock = Mock::start(|_| Reply::text(PAGE)).await;
    let run = run(
        &*builtin("curl.by"),
        &mock,
        RunConfig {
            name: "my file#1.bin".into(),
            ..Default::default()
        },
    )
    .await;

    assert_eq!(run.url(), "https://curl.by/tio_C/sample.bin");
    let request = only_request(&mock);
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/my%20file%231.bin");
    assert_eq!(
        request.header("content-type"),
        Some("application/octet-stream")
    );
    assert_eq!(request.body, b"0123456789abcdef");
    request.assert_not_chunked();
    assert_eq!(run.progress.sent(), 16);
}

#[tokio::test]
async fn curl_by_only_trusts_a_wget_line_on_its_own_host() {
    for body in [
        "Uploaded 0 files\n",
        "wget https://evil.example/x/y.bin\n",
        "see wget https://curl.by/x/y.bin for it",
        "",
    ] {
        let mock = Mock::start(move |_| Reply::text(body)).await;
        let run = run_default(&*builtin("curl.by"), &mock).await;
        assert!(
            matches!(run.error(), UploadError::Protocol { .. }),
            "`{body}` was accepted"
        );
    }
}

#[tokio::test]
async fn curl_by_reports_a_refusal_with_its_status() {
    let mock = Mock::start(|_| Reply::text("too large").status(413)).await;
    let run = run_default(&*builtin("curl.by"), &mock).await;
    assert!(run.error().to_string().contains("413"));
}
