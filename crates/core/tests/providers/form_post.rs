//! The hosts that take one multipart POST and answer with the link.

use serde_json::json;

use crate::common::*;
use crate::support::*;

/// One host of the family: where it is, what it wants, what it says back.
#[derive(Clone, Copy)]
pub struct Case {
    pub provider: &'static str,
    pub host: &'static str,
    pub path: &'static str,
    pub file_field: &'static str,
    pub reply: fn() -> Reply,
    pub link: &'static str,
}

pub const CASES: [Case; 10] = [
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
        provider: "ayaya.beauty",
        host: "ayaya.beauty",
        path: "/-/upload",
        file_field: "file",
        reply: || {
            Reply::json(json!({
                "link": "https://ayaya.beauty/b3hjT",
                "directLink": "https://ayaya.beauty/-/file/b3hjT",
                "id": "b3hjT",
            }))
        },
        link: "https://ayaya.beauty/b3hjT",
    },
    Case {
        provider: "end2end.tech",
        host: "api.end2end.tech",
        path: "/upload",
        file_field: "file",
        reply: || {
            Reply::json(json!({
                "Status": "OK",
                "FileID": "32db6add236f",
                "URL": "https://end2end.tech/32db6add236f",
                "RemovePassword": "11675fom",
            }))
        },
        link: "https://end2end.tech/32db6add236f",
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

pub fn serving(case: Case) -> impl Fn(&Recorded) -> Reply {
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
