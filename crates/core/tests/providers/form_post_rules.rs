//! Rules specific to single hosts of the multipart-POST family, and how bad replies are handled.

use mul_tor_core::{Credentials, LocalFile, UploadError};
use serde_json::json;

use crate::common::*;
use crate::form_post::{CASES, serving};
use crate::support::*;

#[tokio::test]
async fn tempfile_org_asks_for_the_longest_retention() {
    let case = CASES[4];
    let mock = Mock::start(serving(case)).await;
    run_default(&*builtin(case.provider), &mock).await.url();

    let form = only_request(&mock).multipart();
    assert_eq!(text_field(&form, "expiryHours"), "48");
}

#[tokio::test]
async fn easysend_returns_the_absolute_link_not_the_relative_one() {
    let mock = Mock::start(|_| {
        Reply::json(json!({
            "share_url": "/gJVf2",
            "share_link": "https://easysend.co/gJVf2",
            "manage_link": "https://easysend.co/u/secret-token",
        }))
    })
    .await;
    let run = run_default(&*builtin("easysend"), &mock).await;
    // The manage link carries the token that deletes the upload: never hand it out.
    assert_eq!(run.url(), "https://easysend.co/gJVf2");
}

#[tokio::test]
async fn qu_ax_refuses_file_types_it_does_not_take_before_asking() {
    let info = *builtin("qu.ax").info();
    let dir = tempfile::tempdir().unwrap();
    let creds = Credentials::default();
    for (name, allowed) in [
        ("a.zip", true),
        ("b.TXT", true),
        ("c.tar.gz", true),
        ("d.bin", false),
    ] {
        let path = dir.path().join(name);
        std::fs::write(&path, b"x").unwrap();
        let file = LocalFile::open(&path).await.unwrap();
        assert_eq!(info.precheck(&file, &creds).is_ok(), allowed, "{name}");
    }
}

#[tokio::test]
async fn uguu_refuses_the_types_its_server_rejects_before_asking() {
    let info = *builtin("uguu").info();
    let dir = tempfile::tempdir().unwrap();
    let creds = Credentials::default();
    for (name, allowed) in [
        ("a.7z", true),
        ("b.tar.gz", true),
        ("c.ZIP", false),
        ("d.pdf", false),
    ] {
        let path = dir.path().join(name);
        std::fs::write(&path, b"x").unwrap();
        let file = LocalFile::open(&path).await.unwrap();
        assert_eq!(info.precheck(&file, &creds).is_ok(), allowed, "{name}");
    }
}

#[tokio::test]
async fn something_that_is_not_a_link_is_an_error_not_a_result() {
    let case = CASES[0];
    for body in [
        "Sorry, the file is too big\n",
        "https://0x0.st/AbCd.bin\nand then some prose",
        "javascript:alert(1)",
        "ftp://0x0.st/AbCd.bin",
        "",
    ] {
        let mock = Mock::start(move |_| Reply::text(body)).await;
        let run = run_default(&*builtin(case.provider), &mock).await;
        assert!(
            matches!(run.error(), UploadError::Protocol { .. }),
            "`{body}` was accepted"
        );
    }
}

#[tokio::test]
async fn a_json_reply_without_the_link_is_an_error() {
    let mock =
        Mock::start(|_| Reply::json(json!({"success": false, "description": "File too large"})))
            .await;
    let run = run_default(&*builtin("uguu"), &mock).await;
    let message = run.error().to_string();
    assert!(message.contains("/files/0/url"), "{message}");
}

#[tokio::test]
async fn a_refusal_from_the_host_carries_its_status() {
    let mock = Mock::start(|_| Reply::text("file too large").status(413)).await;
    let run = run_default(&*builtin("x0.at"), &mock).await;
    assert!(run.error().to_string().contains("413"));
}
