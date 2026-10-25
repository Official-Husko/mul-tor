//! VikingFile: ask for a server, then upload to it.

use mul_tor_core::UploadError;
use serde_json::json;

use crate::common::*;

fn viking(server: &'static str) -> impl Fn(&Recorded) -> Reply {
    move |r| {
        if r.is("GET", "vikingfile.com", "/api/get-server") {
            Reply::json(json!({ "server": server }))
        } else if r.method == "POST" && r.host == "jp.vikingfile.com" {
            Reply::json(json!({
                "name": "sample.bin",
                "size": "16",
                "hash": "TPRSfLvcIu",
                "url": "https://vikingfile.com/f/TPRSfLvcIu"
            }))
        } else {
            Reply::not_found()
        }
    }
}

#[tokio::test]
async fn vikingfile_posts_the_file_anonymously_to_the_server_it_is_given() {
    let mock = Mock::start(viking("https://jp.vikingfile.com/index2.php")).await;
    let run = run_default(&*builtin("vikingfile"), &mock).await;

    assert_eq!(run.url(), "https://vikingfile.com/f/TPRSfLvcIu");
    let requests = mock.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].path, "/index2.php");
    let form = requests[1].multipart();
    assert_eq!(field(&form, "file").data, b"0123456789abcdef");
    // An empty `user` is what makes the upload anonymous.
    assert_eq!(text_field(&form, "user"), "");
    requests[1].assert_not_chunked();
    assert_eq!(run.progress.sent(), 16);
}

#[tokio::test]
async fn vikingfile_does_not_send_the_file_to_a_server_off_its_domain() {
    for server in [
        "https://evil.example/index.php",
        "https://vikingfile.com.evil.example/index.php",
        "http://jp.vikingfile.com/index.php",
        "https://user@jp.vikingfile.com/index.php",
    ] {
        let mock = Mock::start(viking(server)).await;
        let run = run_default(&*builtin("vikingfile"), &mock).await;

        assert!(
            matches!(run.error(), UploadError::Protocol { .. }),
            "{server}"
        );
        // Only the question about the server was ever asked.
        assert_eq!(mock.requests().len(), 1, "{server}");
    }
}

#[tokio::test]
async fn vikingfile_rejects_a_reply_without_a_usable_link() {
    let mock = Mock::start(|r| {
        if r.host == "vikingfile.com" {
            Reply::json(json!({"server": "https://jp.vikingfile.com/index2.php"}))
        } else {
            Reply::json(json!({"url": "javascript:alert(1)"}))
        }
    })
    .await;
    let run = run_default(&*builtin("vikingfile"), &mock).await;
    assert!(matches!(run.error(), UploadError::Protocol { .. }));
}
