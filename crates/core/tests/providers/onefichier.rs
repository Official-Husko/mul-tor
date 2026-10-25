//! 1Fichier uploads to the server it assigns.

use mul_tor_core::UploadError;
use serde_json::json;

use crate::common::*;

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
