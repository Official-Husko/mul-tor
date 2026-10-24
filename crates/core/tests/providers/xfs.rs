//! The XFileSharing family.

use serde_json::json;

use crate::common::*;

// -------------------------------------------------------------- XFS family

fn xfs_handler(
    page_host: &'static str,
    page_body: &'static str,
    upload_host: &'static str,
    upload_path: &'static str,
    reply: serde_json::Value,
) -> impl Fn(&Recorded) -> Reply {
    move |r| {
        if r.is("GET", page_host, "/") || (r.method == "GET" && r.host == page_host) {
            Reply::html(page_body)
        } else if r.is("POST", upload_host, upload_path) {
            Reply::json(reply.clone())
        } else {
            Reply::not_found()
        }
    }
}

#[tokio::test]
async fn clicknupload_posts_the_file_as_field_file() {
    let mock = Mock::start(xfs_handler(
        "clicknupload.click",
        r#"<form action="https://srv7.clicknupload.net/cgi-bin/upload.cgi?x=1">"#,
        "srv7.clicknupload.net",
        "/cgi-bin/upload.cgi",
        json!([{"file_code": "abc123", "file_status": "OK"}]),
    ))
    .await;
    let run = run_default(&*builtin("clicknupload"), &mock).await;
    assert_eq!(run.url(), "https://clicknupload.vip/abc123");
    let parts = mock.requests().pop().unwrap().multipart();
    assert_eq!(text_field(&parts, "utype"), "anon");
    assert_eq!(text_field(&parts, "file_public"), "1");
    assert_eq!(text_field(&parts, "keepalive"), "1");
    assert_eq!(
        text_field(&parts, "file_descr"),
        "Uploaded using Mul-Tor on GitHub!"
    );
    assert_eq!(field(&parts, "file").data, b"0123456789abcdef");
}

#[tokio::test]
async fn hexupload_reads_the_form_action_and_uses_file_0() {
    let mock = Mock::start(xfs_handler(
        "hexupload.net",
        r#"<form method="post" action = 'https://s5.hexupload.net/cgi-bin/upload.cgi?upload_type=file'>"#,
        "s5.hexupload.net",
        "/cgi-bin/upload.cgi",
        json!([{"file_code": "hx1", "file_status": "OK"}]),
    ))
    .await;
    let run = run_default(&*builtin("hexupload"), &mock).await;
    assert_eq!(run.url(), "https://hexupload.net/hx1");
    field(&mock.requests().pop().unwrap().multipart(), "file_0");
}

#[tokio::test]
async fn usercloud_uses_its_own_link_base() {
    let mock = Mock::start(xfs_handler(
        "userscloud.com",
        r#"<form action="https://u9.userscloud.com/cgi-bin/upload.cgi">"#,
        "u9.userscloud.com",
        "/cgi-bin/upload.cgi",
        json!([{"file_code": "uc9"}]),
    ))
    .await;
    let run = run_default(&*builtin("usercloud"), &mock).await;
    assert_eq!(run.url(), "https://userscloud.com/uc9");
}

#[tokio::test]
async fn usersdrive_matches_its_server_pattern_and_builds_html_links() {
    let mock = Mock::start(xfs_handler(
        "usersdrive.com",
        r#"var up = "https://dns3.userdrive.org/cgi-bin/upload.cgi?upload_type=file";"#,
        "dns3.userdrive.org",
        "/cgi-bin/upload.cgi",
        json!([{"file_code": "ud7", "file_status": "OK"}]),
    ))
    .await;
    let run = run_default(&*builtin("usersdrive"), &mock).await;
    assert_eq!(run.url(), "https://usersdrive.com/ud7.html");
    let upload = mock.requests().pop().unwrap();
    assert_eq!(upload.query, "upload_type=file");
    let parts = upload.multipart();
    assert_eq!(text_field(&parts, "utype"), "anon");
    assert_eq!(text_field(&parts, "keepalive"), "1");
    assert!(
        !parts.iter().any(|p| p.name == "file_public"),
        "UsersDrive never sent this"
    );
}

#[tokio::test]
async fn only_usersdrive_insists_on_a_file_status() {
    let reply = json!([{"file_code": "ud1"}]); // no file_status
    let mock = Mock::start(xfs_handler(
        "usersdrive.com",
        "https://dns3.userdrive.org/cgi-bin/upload.cgi?upload_type=file",
        "dns3.userdrive.org",
        "/cgi-bin/upload.cgi",
        reply.clone(),
    ))
    .await;
    let run = run_default(&*builtin("usersdrive"), &mock).await;
    assert!(
        run.error().to_string().contains("file_status"),
        "{}",
        run.error()
    );

    // The others never checked it, so a reply without one is fine.
    let mock = Mock::start(xfs_handler(
        "hexupload.net",
        r#"<form action="https://s5.hexupload.net/cgi-bin/upload.cgi">"#,
        "s5.hexupload.net",
        "/cgi-bin/upload.cgi",
        reply,
    ))
    .await;
    assert_eq!(
        run_default(&*builtin("hexupload"), &mock).await.url(),
        "https://hexupload.net/ud1"
    );
}

#[tokio::test]
async fn xfs_providers_report_a_rejected_file() {
    let mock = Mock::start(xfs_handler(
        "usersdrive.com",
        "https://dns3.userdrive.org/cgi-bin/upload.cgi?upload_type=file",
        "dns3.userdrive.org",
        "/cgi-bin/upload.cgi",
        json!([{"file_code": "", "file_status": "unauthorized"}]),
    ))
    .await;
    let run = run_default(&*builtin("usersdrive"), &mock).await;
    assert!(run.error().to_string().contains("unauthorized"));
}

#[tokio::test]
async fn xfs_providers_fail_cleanly_when_the_page_has_no_upload_server() {
    let mock = Mock::start(|_| Reply::html("<html>maintenance</html>")).await;
    let run = run_default(&*builtin("clicknupload"), &mock).await;
    assert!(run.error().to_string().contains("upload server"));
    assert_eq!(mock.requests().len(), 1);
}
