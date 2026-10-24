//! File.io: upload, then patch metadata with the anonymous token.

use serde_json::json;

use crate::common::*;

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
async fn file_io_needs_only_the_link_when_no_metadata_can_be_set() {
    // No access token and no key: nothing to patch, but the upload worked.
    let mock = Mock::start(|_| Reply::json(json!({"link": "https://file.io/OnlyLink"}))).await;
    let run = run_default(&*builtin("file.io"), &mock).await;
    assert_eq!(run.url(), "https://file.io/OnlyLink");
    assert_eq!(mock.requests().len(), 1, "no PATCH without a token");
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
