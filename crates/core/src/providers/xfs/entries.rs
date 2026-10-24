//! The hosts, as data.

use super::XfsProvider;
use crate::provider::ProviderInfo;
use crate::size::SizeLimit;

/// What the older script's own upload form sends for an anonymous visitor.
const CLASSIC_FORM: &[(&str, &str)] =
    &[("upload_type", "file"), ("sess_id", ""), ("utype", "anon")];

/// What the upload page of an XFileSharing Pro host sends for a visitor without an account.
const NODE_FORM: &[(&str, &str)] = &[("utype", "anon"), ("sess_id", "")];

const ANON_FORM: &[(&str, &str)] = &[
    ("utype", "anon"),
    ("file_descr", "Uploaded using Mul-Tor on GitHub!"),
    ("file_public", "1"),
    ("keepalive", "1"),
];

/// Generic pattern: the form's `action="https://…"` attribute.
const FORM_ACTION: &str = r#"action\s*=\s*["'](https?://[^\s"']+)["']"#;

pub(in crate::providers) fn providers() -> Vec<XfsProvider> {
    vec![
        XfsProvider::new(
            ProviderInfo::new(
                "ClicknUpload",
                "https://clicknupload.click",
                "https://clicknupload.click",
            )
            .max_size(SizeLimit::gb(2)),
            "https://clicknupload.click",
            r"https://\w+\.clicknupload\.net/cgi-bin/upload\.cgi",
            "file",
            ANON_FORM,
            "https://clicknupload.vip/{code}",
        ),
        XfsProvider::new(
            ProviderInfo::new(
                "Datanodes",
                "https://datanodes.to/",
                "https://datanodes.to/",
            )
            .aliases(&["Datanodes.to"])
            .max_size(SizeLimit::gb(3)),
            "https://datanodes.to/upload",
            r"https://[\w-]+\.datanodes\.to/cgi-bin/upload\.cgi\?upload_type=file&(?:amp;)?utype=anon",
            "file_0",
            NODE_FORM,
            "https://datanodes.to/{code}",
        ),
        XfsProvider::new(
            ProviderInfo::new("Dosya.co", "https://dosya.co/", "https://dosya.co/")
                .max_size(SizeLimit::gb(2))
                .verified("2026-10-24"),
            "https://dosya.co/",
            r"https://\w+\.dosya\.co/cgi-bin/upload\.cgi\?upload_id=",
            "file_1",
            CLASSIC_FORM,
            "https://dosya.co/{code}",
        )
        .redirecting(),
        XfsProvider::new(
            ProviderInfo::new(
                "Filespace",
                "https://filespace.com/",
                "https://filespace.com/",
            )
            .max_size(SizeLimit::mb(150))
            .verified("2026-10-24"),
            "https://filespace.com/",
            r"https://\w+\.filespace\.com/cgi-bin/upload\.cgi\?upload_id=",
            "file_1",
            CLASSIC_FORM,
            "https://filespace.com/{code}",
        )
        .redirecting(),
        XfsProvider::new(
            ProviderInfo::new(
                "HexUpload",
                "https://hexupload.net/",
                "https://hexupload.net/",
            )
            .max_size(SizeLimit::gb(2))
            .blocked_extensions(&[".jpeg", ".jpg", ".png"]),
            "https://hexupload.net/",
            FORM_ACTION,
            "file_0",
            ANON_FORM,
            "https://hexupload.net/{code}",
        ),
        XfsProvider::new(
            ProviderInfo::new("Send.now", "https://send.now/", "https://send.now/upload"),
            "https://send.now/upload",
            r"https://[\w-]+\.send\.now/cgi-bin/upload\.cgi\?upload_type=file&(?:amp;)?utype=anon",
            "file_0",
            NODE_FORM,
            "https://send.now/{code}",
        ),
        XfsProvider::new(
            ProviderInfo::new(
                "UserCloud",
                "https://userscloud.com/",
                "https://userscloud.com/",
            )
            .aliases(&["Userscloud"])
            .max_size(SizeLimit::gb(5)),
            "https://userscloud.com/",
            FORM_ACTION,
            "file_0",
            ANON_FORM,
            "https://userscloud.com/{code}",
        ),
        XfsProvider::new(
            ProviderInfo::new(
                "UsersDrive",
                "https://usersdrive.com/",
                "https://usersdrive.com/",
            )
            .max_size(SizeLimit::gb(2)),
            "https://usersdrive.com/",
            r"https://\S+\.userdrive\.(?:me|org)/cgi-bin/upload\.cgi\?upload_type=file",
            "file_0",
            &[("utype", "anon"), ("keepalive", "1")],
            "https://usersdrive.com/{code}.html",
        )
        .requiring_status(),
    ]
}
