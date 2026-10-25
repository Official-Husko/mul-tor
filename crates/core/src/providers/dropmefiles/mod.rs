//! DropMeFiles — the page names the server to use and the chunk size; a
//! transfer is created, the file goes up in raw chunks (the way the page's
//! Plupload does it), a password flag is set and the transfer is saved.
//!
//! The longest time the page offers, 14 days, is always asked for.

mod session;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::error::UploadError;
use crate::provider::{Provider, ProviderInfo};
use crate::reply::Reply;
use crate::size::SizeLimit;
use crate::upload::{LOWER_ALNUM, UploadContext, UploadedFile, chunk_ranges, random_string};
use session::{Session, chunk_url};

const INFO: ProviderInfo = ProviderInfo::new(
    "DropMeFiles",
    "https://dropmefiles.com/",
    "https://dropmefiles.com/",
)
.max_size(SizeLimit::gb(50));

/// The "14 days" choice of the page; 2 is 7 days.
const PERIOD: &str = "3";

/// Archives and programs up to this size go through the site's virus check,
/// which has an endpoint of its own.
const SCAN_LIMIT: u64 = 78_643_200;
const SCANNED: &[&str] = &[
    "zip", "rar", "7z", "tar", "apk", "bat", "bin", "cgi", "com", "cpp", "exe", "gadget", "jar",
    "msi", "msu", "pif", "scr", "vb", "wsf", "action", "app", "command", "csh", "ipa", "workflow",
];

pub struct DropMeFiles;

#[async_trait]
impl Provider for DropMeFiles {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        cx.progress.set_message("Reading the page's settings");
        let mut session = Session::open(cx).await?;
        let size = cx.file.size;

        cx.progress.set_message("Creating the transfer");
        let created = session
            .send(session.post(session.api("upload/create")).form(&[
                ("runtime", "html5"),
                ("server", "0"),
                ("group", ""),
                ("updirType", "abc"),
                ("count", "1"),
                ("size", &size.to_string()),
                ("period", PERIOD),
                ("name", ""),
                ("comment", ""),
            ]))
            .await?;
        let uid = transfer_id(&created)?;
        // Names the file in the chunk headers and again in the save call.
        let file_id = format!("o_{}", random_string(25, LOWER_ALNUM));
        let upload_id = format!("{uid}_{file_id}");

        let endpoint = session.api(if scanned(&cx.file.name, size) {
            "uploadch"
        } else {
            "uploadrmbl"
        });
        let chunks = size.div_ceil(session.chunk_size).max(1);
        let started = std::time::Instant::now();
        for (index, (offset, len)) in chunk_ranges(size, session.chunk_size).enumerate() {
            cx.progress
                .set_message(format!("Chunk {}/{}", index + 1, chunks));
            let url = chunk_url(&endpoint, &cx.file.name, index as u64, chunks, &uid)?;
            // The resumable-upload headers of the page's uploader.
            let request = session
                .post(url)
                .header("Content-Type", "application/octet-stream")
                .header(
                    "Content-Disposition",
                    format!("attachment; filename=\"{upload_id}\""),
                )
                .header(
                    "Content-Range",
                    format!("bytes {offset}-{}/{size}", offset + len - 1),
                )
                .header("Session-ID", &upload_id)
                .header("Content-Length", len)
                .body(cx.range_body(offset, len).await?);
            session.send(request).await?;
        }

        let request = session.post(session.api("upload/password")).form(&[
            ("uid", uid.as_str()),
            ("updirType", "abc"),
            ("password", "0"),
        ]);
        session.send(request).await?;

        cx.progress.set_message("Saving");
        let speed = size / started.elapsed().as_secs().max(1);
        let request = session.post(session.api("upload/save")).form(&[
            ("files", files_json(cx, &uid, &file_id).as_str()),
            ("uid", uid.as_str()),
            ("group", ""),
            ("count", "1"),
            ("size", &size.to_string()),
            ("speed", &speed.to_string()),
            ("period", PERIOD),
            ("name", ""),
            ("comment", ""),
        ]);
        let saved = session.send(request).await?;
        if saved.json_string("/result")? != "Saved" {
            return Err(UploadError::protocol(
                "the transfer was not saved",
                &saved.body,
            ));
        }
        Ok(UploadedFile::new(format!("https://dropmefiles.com/{uid}")))
    }
}

/// The id of the new transfer; it becomes part of addresses, so only plain
/// letters and digits pass.
fn transfer_id(created: &Reply) -> Result<String, UploadError> {
    let uid = created.json_string("/result")?;
    if uid.is_empty() || !uid.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(UploadError::protocol("unusable transfer id", &created.body));
    }
    Ok(uid)
}

/// The `files` field of the save call: what the page's uploader reports for
/// the file once it is up.
fn files_json(cx: &UploadContext<'_>, uid: &str, file_id: &str) -> String {
    let size = cx.file.size;
    Value::Array(vec![json!({
        "id": file_id,
        "name": cx.file.name,
        "type": "application/octet-stream",
        "relativePath": "",
        "size": size,
        "origSize": size,
        "loaded": size,
        "percent": 100,
        "status": 5,
        "dir": uid,
        "logstatus": 2,
    })])
    .to_string()
}

/// Does the site run this file through its virus check?
fn scanned(name: &str, size: u64) -> bool {
    let extension = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
    size <= SCAN_LIMIT && extension.is_some_and(|e| SCANNED.contains(&e.as_str()))
}
