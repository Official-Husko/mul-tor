//! Sending the bytes to the presigned storage URLs.

use std::collections::HashMap;

use reqwest::header::ACCEPT;
use serde_json::{Value, json};

use super::{API, OWNER_HEADER};
use crate::error::UploadError;
use crate::reply::{Reply, SendReply};
use crate::upload::{UploadContext, chunk_ranges, https_url_within};

/// Where the signed URLs must lead.
const STORAGE_DOMAIN: &str = "r2.cloudflarestorage.com";

/// Parts asked for in one call when the first reply did not list them all.
const URLS_PER_CALL: usize = 10;

/// What the init call decided.
pub(super) enum Plan {
    Single {
        key: String,
        url: String,
        headers: Vec<(String, String)>,
    },
    Multipart {
        key: String,
        upload_id: String,
        owner_token: String,
        part_size: u64,
        total_parts: u64,
        urls: HashMap<u64, String>,
    },
}

impl Plan {
    /// Read the init reply. Nothing from it is kept in an error: it holds signed
    /// URLs and the owner token.
    pub(super) fn from_reply(init: &Reply) -> Result<Self, UploadError> {
        let json = init.json()?;
        let text = |key: &str| {
            json.get(key)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| UploadError::unexpected(format!("init reply has no `{key}`")))
        };
        let number = |key: &str| {
            json.get(key)
                .and_then(Value::as_u64)
                .filter(|n| *n > 0)
                .ok_or_else(|| UploadError::unexpected(format!("init reply has no `{key}`")))
        };
        let key = text("r2_key")?;
        match text("type")?.as_str() {
            "single" => {
                let headers = json
                    .get("headers")
                    .and_then(Value::as_object)
                    .map(|map| {
                        map.iter()
                            .filter_map(|(name, value)| {
                                let first = value.as_array()?.first()?.as_str()?;
                                Some((name.clone(), first.to_string()))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(Plan::Single {
                    key,
                    url: storage_url(&text("upload_url")?)?,
                    headers,
                })
            }
            "multipart" => {
                let mut urls = HashMap::new();
                if let Some(listed) = json.get("initial_urls").and_then(Value::as_object) {
                    for (number, url) in listed {
                        let (Ok(number), Some(url)) = (number.parse(), url.as_str()) else {
                            continue;
                        };
                        urls.insert(number, storage_url(url)?);
                    }
                }
                Ok(Plan::Multipart {
                    key,
                    upload_id: text("upload_id")?,
                    owner_token: text("owner_token")?,
                    part_size: number("part_size")?,
                    total_parts: number("total_parts")?,
                    urls,
                })
            }
            other => Err(UploadError::unexpected(format!(
                "unknown upload type `{other}`"
            ))),
        }
    }

    pub(super) fn key(&self) -> &str {
        match self {
            Plan::Single { key, .. } | Plan::Multipart { key, .. } => key,
        }
    }

    pub(super) fn owner_token(&self) -> Option<&str> {
        match self {
            Plan::Single { .. } => None,
            Plan::Multipart { owner_token, .. } => Some(owner_token),
        }
    }
}

/// A signed URL is requested with the file: it must lead to the storage service.
fn storage_url(url: &str) -> Result<String, UploadError> {
    https_url_within(url, STORAGE_DOMAIN)
        .map(String::from)
        .ok_or_else(|| UploadError::unexpected("unusable storage URL"))
}

pub(super) async fn send_single(cx: &UploadContext<'_>, plan: &Plan) -> Result<(), UploadError> {
    let Plan::Single { url, headers, .. } = plan else {
        return Ok(());
    };
    cx.progress.set_message("Uploading");
    let mut request = cx.http.put(url);
    // The URL was signed for exactly these headers.
    for (name, value) in headers {
        request = request.header(name, value);
    }
    cx.raw_upload(request).await?.send_reply().await?.ok()?;
    Ok(())
}

pub(super) async fn send_multipart(cx: &UploadContext<'_>, plan: &Plan) -> Result<(), UploadError> {
    let Plan::Multipart {
        upload_id,
        owner_token,
        part_size,
        total_parts,
        urls,
        ..
    } = plan
    else {
        return Ok(());
    };
    let mut urls = urls.clone();
    let mut etags = Vec::new();
    for (index, (offset, len)) in chunk_ranges(cx.file.size, *part_size).enumerate() {
        let number = index as u64 + 1;
        cx.progress
            .set_message(format!("Part {number}/{total_parts}"));
        if !urls.contains_key(&number) {
            urls.extend(more_urls(cx, upload_id, owner_token, number, *total_parts).await?);
        }
        let url = urls
            .get(&number)
            .ok_or_else(|| UploadError::unexpected(format!("no URL for part {number}")))?;
        let request = cx.http.put(url).body(cx.range_body(offset, len).await?);
        let reply = request
            .header(reqwest::header::CONTENT_LENGTH, len)
            .send_reply()
            .await?
            .ok()?;
        let etag = reply.header("etag").ok_or_else(|| {
            UploadError::unexpected(format!("part {number} came back without an ETag"))
        })?;
        etags.push(json!({"partNumber": number, "etag": etag}));
    }

    cx.progress.set_message("Assembling");
    let done = cx
        .http
        .post(format!("{API}/complete-multipart"))
        .header(ACCEPT, "application/json")
        .header(OWNER_HEADER, owner_token)
        .json(&json!({"upload_id": upload_id, "parts": etags}))
        .send_reply()
        .await?
        .ok()?;
    if done.json()?.get("success").and_then(Value::as_bool) != Some(true) {
        return Err(UploadError::protocol(
            "the parts were not assembled",
            &done.body,
        ));
    }
    Ok(())
}

/// Signed URLs for the parts from `first` on that the init reply did not list.
async fn more_urls(
    cx: &UploadContext<'_>,
    upload_id: &str,
    owner_token: &str,
    first: u64,
    total_parts: u64,
) -> Result<HashMap<u64, String>, UploadError> {
    let last = (first + URLS_PER_CALL as u64 - 1).min(total_parts);
    let wanted: Vec<u64> = (first..=last).collect();
    let reply = cx
        .http
        .post(format!("{API}/parts"))
        .header(ACCEPT, "application/json")
        .header(OWNER_HEADER, owner_token)
        .json(&json!({"upload_id": upload_id, "part_numbers": wanted}))
        .send_reply()
        .await?
        .ok()?;
    let mut urls = HashMap::new();
    let listed = reply.json()?;
    for (number, url) in listed
        .get("urls")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        if let (Ok(number), Some(url)) = (number.parse(), url.as_str()) {
            urls.insert(number, storage_url(url)?);
        }
    }
    Ok(urls)
}
