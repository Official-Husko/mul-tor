//! The hosts that take one multipart POST and answer with the link.

use mul_tor_core::{Credentials, LocalFile, UploadError};
use serde_json::json;

use crate::common::*;
use crate::support::*;

/// One host of the family: where it is, what it wants, what it says back.
#[derive(Clone, Copy)]
struct Case {
    provider: &'static str,
    host: &'static str,
    path: &'static str,
    file_field: &'static str,
    reply: fn() -> Reply,
    link: &'static str,
}

const CASES: [Case; 8] = [
    Case {
        provider: "0x0.st",
        host: "0x0.st",
        path: "/",
        file_field: "file",
        reply: || Reply::text("https://0x0.st/AbCd.bin\n"),
        link: "https://0x0.st/AbCd.bin",
    },
    Case {
        provider: "easysend",
        host: "easysend.co",
        path: "/api/v1/upload",
        file_field: "files[]",
        reply: || {
            Reply::json(json!({"success": true, "share_link": "https://easysend.co/gJVf2"}))
                .status(201)
        },
        link: "https://easysend.co/gJVf2",
    },
    Case {
        provider: "qu.ax",
        host: "qu.ax",
        path: "/upload",
        file_field: "files[]",
        reply: || Reply::json(json!({"success": true, "files": [{"url": "https://qu.ax/IaTHM"}]})),
        link: "https://qu.ax/IaTHM",
    },
    Case {
        provider: "temp.sh",
        host: "temp.sh",
        path: "/upload",
        file_field: "file",
        reply: || Reply::text("https://temp.sh/szZad/sample.bin"),
        link: "https://temp.sh/szZad/sample.bin",
    },
    Case {
        provider: "tempfile.org",
        host: "tempfile.org",
        path: "/api/upload/local",
        file_field: "files",
        reply: || {
            Reply::json(
                json!({"success": true, "files": [{"url": "https://tempfile.org/kN8mP2xQvR7/"}]}),
            )
        },
        link: "https://tempfile.org/kN8mP2xQvR7/",
    },
    Case {
        provider: "tflink",
        host: "tmpfile.link",
        path: "/api/upload",
        file_field: "file",
        reply: || {
            Reply::json(
                json!({"fileName": "sample.bin", "downloadLink": "https://d.tmpfile.link/public/2026/sample.bin"}),
            )
        },
        link: "https://d.tmpfile.link/public/2026/sample.bin",
    },
    Case {
        provider: "uguu",
        host: "uguu.se",
        path: "/upload",
        file_field: "files[]",
        reply: || {
            Reply::json(
                json!({"success": true, "files": [{"url": "https://n.uguu.se/QeDbqotc.bin"}]}),
            )
        },
        link: "https://n.uguu.se/QeDbqotc.bin",
    },
    Case {
        provider: "x0.at",
        host: "x0.at",
        path: "/",
        file_field: "file",
        reply: || Reply::text("https://x0.at/LeGh.bin\n\n"),
        link: "https://x0.at/LeGh.bin",
    },
];

fn serving(case: Case) -> impl Fn(&Recorded) -> Reply {
    move |r| {
        if r.is("POST", case.host, case.path) {
            (case.reply)()
        } else {
            Reply::not_found()
        }
    }
}

#[tokio::test]
async fn every_host_gets_the_file_in_its_own_field_and_returns_the_link() {
    for case in CASES {
        let mock = Mock::start(serving(case)).await;
        let run = run_default(&*builtin(case.provider), &mock).await;

        assert_eq!(run.url(), case.link, "{}", case.provider);
        let request = only_request(&mock);
        let form = request.multipart();
        let file = field(&form, case.file_field);
        assert_eq!(file.data, b"0123456789abcdef", "{}", case.provider);
        assert_eq!(file.filename.as_deref(), Some("sample.bin"));
        request.assert_not_chunked();
        assert_eq!(run.progress.sent(), 16, "{}", case.provider);
    }
}

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
