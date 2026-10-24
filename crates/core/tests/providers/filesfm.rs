//! Files.fm: id and key from the provider, upload must finish.

use mul_tor_core::UploadError;
use serde_json::json;

use crate::common::*;

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
