//! Sendspace: a signed upload address on the home page, the link on the page that answers.

use mul_tor_core::UploadError;

use crate::common::*;

const HOME: &str = r#"<form aria-label="Upload files" method="post" action="https://fs12u.sendspace.com/upload?SPEED_LIMIT=0&amp;MAX_FILE_SIZE=314572800&amp;UPLOAD_IDENTIFIER=1.2.A.18.0&amp;signature=abc123" enctype="multipart/form-data">
<input type="hidden" name="PROGRESS_URL" value="https://fs12u.sendspace.com/progress.php?UPLOAD_IDENTIFIER=1.2.A.18.0" />
<input type="hidden" name="signature" value="abc123" />"#;

const RESULT: &str = r#"<a href="https://www.sendspace.com/file/hizo3w68">download</a>
<a href="https://www.sendspace.com/delete/hizo3w68/fb3da23d26b0519a98e618fa76df65ea">delete</a>"#;

fn site(home: &'static str, result: &'static str) -> impl Fn(&Recorded) -> Reply {
    move |r| {
        if r.is("GET", "www.sendspace.com", "/") {
            Reply::html(home)
        } else if r.is("POST", "fs12u.sendspace.com", "/upload") {
            Reply::html(result)
        } else {
            Reply::not_found()
        }
    }
}

#[tokio::test]
async fn sendspace_posts_the_pages_own_form_and_takes_the_file_link_only() {
    let mock = Mock::start(site(HOME, RESULT)).await;
    let run = run_default(&*builtin("sendspace"), &mock).await;

    // Not the delete link next to it.
    assert_eq!(run.url(), "https://www.sendspace.com/file/hizo3w68");
    let upload = mock
        .requests()
        .into_iter()
        .find(|r| r.method == "POST")
        .unwrap();
    // `&amp;` in the page is `&` on the wire.
    assert_eq!(
        upload.query,
        "SPEED_LIMIT=0&MAX_FILE_SIZE=314572800&UPLOAD_IDENTIFIER=1.2.A.18.0&signature=abc123"
    );
    let form = upload.multipart();
    assert_eq!(text_field(&form, "signature"), "abc123");
    assert_eq!(text_field(&form, "js_enabled"), "0");
    assert_eq!(text_field(&form, "terms"), "1");
    assert_eq!(
        text_field(&form, "PROGRESS_URL"),
        "https://fs12u.sendspace.com/progress.php?UPLOAD_IDENTIFIER=1.2.A.18.0"
    );
    assert_eq!(field(&form, "upload_file[]").data, b"0123456789abcdef");
    upload.assert_not_chunked();
}

#[tokio::test]
async fn sendspace_never_sends_the_file_to_a_server_outside_its_domain() {
    let home = HOME.replace("fs12u.sendspace.com/upload", "evil.example/upload");
    let home: &'static str = Box::leak(home.into_boxed_str());
    let mock = Mock::start(site(home, RESULT)).await;
    let run = run_default(&*builtin("sendspace"), &mock).await;

    assert!(run.error().to_string().contains("upload server"));
    assert!(mock.requests().iter().all(|r| r.method == "GET"));
}

#[tokio::test]
async fn sendspace_fails_clearly_when_the_form_or_the_link_is_missing() {
    let mock = Mock::start(site("<html>maintenance</html>", RESULT)).await;
    let run = run_default(&*builtin("sendspace"), &mock).await;
    assert!(run.error().to_string().contains("upload form"));

    let mock = Mock::start(site(HOME, "<html>Something went wrong</html>")).await;
    let run = run_default(&*builtin("sendspace"), &mock).await;
    assert!(matches!(run.error(), UploadError::Protocol { .. }));
}
