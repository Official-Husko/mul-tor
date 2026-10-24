//! XFileSharing Pro hosts whose upload page names the node to send the file to.

use serde_json::json;

use crate::common::*;

const SEND_NOW_PAGE: &str = "var pageServer = { first: true, cgi: 'https://u2138.send.now/cgi-bin', upload_url: 'https://u2138.send.now/cgi-bin/upload.cgi?upload_type=file&utype=anon', speed_token: '' };";
const DATANODES_PAGE: &str = r#"<uploader upload-url="https://node42.datanodes.to/cgi-bin/upload.cgi?upload_type=file&amp;utype=anon" session-id="" type="anon">"#;

fn node(
    page_host: &'static str,
    page: &'static str,
    node_host: &'static str,
) -> impl Fn(&Recorded) -> Reply {
    move |r| {
        if r.is("GET", page_host, "/upload") {
            Reply::html(page)
        } else if r.is("POST", node_host, "/cgi-bin/upload.cgi") {
            Reply::json(json!([{"file_code": "s28ma74yrgx2", "file_status": "OK"}]))
        } else {
            Reply::not_found()
        }
    }
}

#[tokio::test]
async fn send_now_posts_to_the_node_its_page_names() {
    let mock = Mock::start(node("send.now", SEND_NOW_PAGE, "u2138.send.now")).await;
    let run = run_default(&*builtin("send.now"), &mock).await;

    assert_eq!(run.url(), "https://send.now/s28ma74yrgx2");
    let upload = mock
        .requests()
        .into_iter()
        .find(|r| r.method == "POST")
        .unwrap();
    assert_eq!(upload.query, "upload_type=file&utype=anon");
    let form = upload.multipart();
    assert_eq!(text_field(&form, "utype"), "anon");
    assert_eq!(text_field(&form, "sess_id"), "");
    assert_eq!(field(&form, "file_0").data, b"0123456789abcdef");
    upload.assert_not_chunked();
}

#[tokio::test]
async fn datanodes_reads_the_node_out_of_an_attribute_without_the_html_escape() {
    let mock = Mock::start(node("datanodes.to", DATANODES_PAGE, "node42.datanodes.to")).await;
    let run = run_default(&*builtin("datanodes"), &mock).await;

    assert_eq!(run.url(), "https://datanodes.to/s28ma74yrgx2");
    let upload = mock
        .requests()
        .into_iter()
        .find(|r| r.method == "POST")
        .unwrap();
    // `&amp;` in the page is `&` on the wire.
    assert_eq!(upload.query, "upload_type=file&utype=anon");
}

#[tokio::test]
async fn a_node_outside_the_host_is_never_sent_the_file() {
    let page = "upload_url: 'https://evil.example/cgi-bin/upload.cgi?upload_type=file&utype=anon'";
    let mock = Mock::start(node("send.now", page, "evil.example")).await;
    let run = run_default(&*builtin("send.now"), &mock).await;

    assert!(run.error().to_string().contains("upload server"));
    assert!(mock.requests().iter().all(|r| r.method == "GET"));
}
