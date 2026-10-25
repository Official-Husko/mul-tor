//! Firefox Send and its forks: records encrypted with a secret that stays in the link.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes128Gcm, Nonce};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hkdf::Hkdf;
use mul_tor_core::UploadError;
use serde_json::{Value, json};
use sha2::Sha256;

use crate::common::*;

fn config(r: &Recorded) -> Reply {
    if r.is("GET", "send.vis.ee", "/config") {
        Reply::json(json!({"LIMITS": {
            "MAX_FILE_SIZE": 2_684_354_560u64, "MAX_DOWNLOADS": 20, "MAX_EXPIRE_SECONDS": 259_200}}))
    } else {
        Reply::not_found()
    }
}

/// What the server says: the id after the metadata, "ok" after the closing zero byte.
fn server(frame: Frame<'_>) -> Vec<String> {
    match frame {
        Frame::Text(_) => vec![
            json!({"url": "https://send.vis.ee/download/abc123def456/",
                   "ownerToken": "owner", "id": "abc123def456"})
            .to_string(),
        ],
        Frame::Binary([0]) => vec![json!({"ok": true}).to_string()],
        Frame::Binary(_) => Vec::new(),
    }
}

fn expand<const N: usize>(salt: Option<&[u8]>, secret: &[u8], info: &[u8]) -> [u8; N] {
    let mut out = [0u8; N];
    Hkdf::<Sha256>::new(salt, secret)
        .expand(info, &mut out)
        .unwrap();
    out
}

fn secret_of(link: &str) -> [u8; 16] {
    let secret = link
        .split_once('#')
        .expect("the secret is in the fragment")
        .1;
    URL_SAFE_NO_PAD.decode(secret).unwrap().try_into().unwrap()
}

/// What a receiver does with the stream: header, then records of the size it names.
fn decrypt_stream(secret: &[u8; 16], frames: &[Vec<u8>]) -> Vec<u8> {
    let (header, records) = frames.split_first().unwrap();
    assert_eq!(header.len(), 21);
    assert_eq!(
        u32::from_be_bytes(header[16..20].try_into().unwrap()),
        65536
    );
    assert_eq!(header[20], 0, "no key id");
    let salt = &header[..16];
    let key: [u8; 16] = expand(Some(salt), secret, b"Content-Encoding: aes128gcm\0");
    let base: [u8; 12] = expand(Some(salt), secret, b"Content-Encoding: nonce\0");
    let cipher = Aes128Gcm::new(&key.into());

    let mut plain = Vec::new();
    for (index, record) in records.iter().enumerate() {
        let mut nonce = base;
        for (byte, count) in nonce[8..].iter_mut().zip((index as u32).to_be_bytes()) {
            *byte ^= count;
        }
        let mut opened = cipher
            .decrypt(&Nonce::from(nonce), record.as_slice())
            .unwrap();
        // Drop the zero padding, then the delimiter: 1 inside the stream, 2 at its end.
        while opened.last() == Some(&0) {
            opened.pop();
        }
        let last = index + 1 == records.len();
        assert_eq!(
            opened.pop(),
            Some(if last { 2 } else { 1 }),
            "record {index}"
        );
        if !last {
            assert_eq!(record.len(), 65536, "only the last record may be short");
        }
        plain.extend(opened);
    }
    plain
}

async fn upload(content: Vec<u8>) -> (Run, Mock) {
    let mock = Mock::start_with_frames(config, server).await;
    let run = run(
        &*builtin("sendvis"),
        &mock,
        RunConfig {
            name: "backup.bin".into(),
            content,
            ..Default::default()
        },
    )
    .await;
    (run, mock)
}

#[tokio::test]
async fn send_encrypts_the_file_in_records_and_keeps_the_secret_in_the_link() {
    let content: Vec<u8> = (0..200_000u32).map(|n| (n % 251) as u8).collect();
    let (run, mock) = upload(content.clone()).await;

    let link = run.url().to_string();
    assert!(
        link.starts_with("https://send.vis.ee/download/abc123def456/#"),
        "{link}"
    );
    let secret = secret_of(&link);

    let frames = mock.binary_frames();
    // The header, four records (65519 bytes of the file each, at most) and the zero byte.
    assert_eq!(frames.len(), 1 + 4 + 1);
    assert_eq!(frames.last().unwrap(), &vec![0u8]);
    let stream = &frames[..frames.len() - 1];
    assert_eq!(decrypt_stream(&secret, stream), content);
    assert_eq!(run.progress.sent(), 200_000);

    // The secret never goes over the wire, in any form.
    let text = URL_SAFE_NO_PAD.encode(secret);
    assert!(mock.messages().iter().all(|m| !m.contains(&text)));
    assert!(frames.iter().all(|f| !f.windows(16).any(|w| w == secret)));
}

#[tokio::test]
async fn send_asks_for_the_longest_time_and_most_downloads_the_instance_allows() {
    let (run, mock) = upload(b"0123456789abcdef".to_vec()).await;
    let secret = secret_of(run.url());

    let first: Value = serde_json::from_str(&mock.messages()[0]).unwrap();
    assert_eq!(first["timeLimit"], 259_200);
    assert_eq!(first["dlimit"], 20);
    assert!(first.get("bearer").is_none(), "no account");

    // The owner proof is the key derived from the secret, as the server expects.
    let auth: [u8; 64] = expand(None, &secret, b"authentication");
    assert_eq!(
        first["authorization"],
        format!("send-v1 {}", URL_SAFE_NO_PAD.encode(auth))
    );

    // The metadata opens with the key derived for it, under an all-zero nonce.
    let meta_key: [u8; 16] = expand(None, &secret, b"metadata");
    let sealed = URL_SAFE_NO_PAD
        .decode(first["fileMetadata"].as_str().unwrap())
        .unwrap();
    let opened = Aes128Gcm::new(&meta_key.into())
        .decrypt(&Nonce::default(), sealed.as_slice())
        .unwrap();
    let metadata: Value = serde_json::from_slice(&opened).unwrap();
    assert_eq!(metadata["name"], "backup.bin");
    assert_eq!(metadata["size"], 16);
    assert_eq!(metadata["manifest"]["files"][0]["name"], "backup.bin");
}

#[tokio::test]
async fn send_does_not_add_an_empty_record_when_the_file_fills_the_last_one() {
    // Two full records of 65519 bytes, and one byte more than one record holds.
    for (size, records) in [(65_519usize, 1usize), (131_038, 2), (65_520, 2)] {
        let content = vec![7u8; size];
        let (run, mock) = upload(content.clone()).await;
        let frames = mock.binary_frames();
        assert_eq!(frames.len(), 1 + records + 1, "{size} bytes");
        let secret = secret_of(run.url());
        assert_eq!(
            decrypt_stream(&secret, &frames[..frames.len() - 1]),
            content
        );
    }
}

#[tokio::test]
async fn send_reports_a_refusal_from_the_server() {
    let mock = Mock::start_with_socket(config, |_| vec![json!({"error": 413}).to_string()]).await;
    let run = run_default(&*builtin("sendvis"), &mock).await;
    let message = format!("{:?}", run.error());
    assert!(message.contains("413"), "{message}");
    assert!(
        mock.binary_frames().is_empty(),
        "nothing is sent after a refusal"
    );
}

#[tokio::test]
async fn send_refuses_an_id_that_is_not_plain_and_a_missing_limit() {
    let mock = Mock::start_with_socket(config, |_| {
        vec![json!({"url": "x", "id": "../../x"}).to_string()]
    })
    .await;
    let run = run_default(&*builtin("sendvis"), &mock).await;
    assert!(matches!(run.error(), UploadError::Protocol { .. }));

    let mock = Mock::start_with_frames(|_| Reply::json(json!({"LIMITS": {}})), server).await;
    let run = run_default(&*builtin("sendvis"), &mock).await;
    assert!(run.error().to_string().contains("MAX_EXPIRE_SECONDS"));
    assert!(mock.messages().is_empty());
}

#[tokio::test]
async fn every_instance_uses_its_own_host_and_the_limits_it_publishes() {
    let mock = Mock::start_with_frames(
        |r| {
            if r.is("GET", "send.mni.li", "/config") {
                Reply::json(json!({"LIMITS": {
                    "MAX_FILE_SIZE": 8_589_934_592u64, "MAX_DOWNLOADS": 25, "MAX_EXPIRE_SECONDS": 604_800}}))
            } else {
                Reply::not_found()
            }
        },
        server,
    )
    .await;
    let run = run_default(&*builtin("send.mni.li"), &mock).await;

    assert!(
        run.url()
            .starts_with("https://send.mni.li/download/abc123def456/#"),
        "{}",
        run.url()
    );
    let first: Value = serde_json::from_str(&mock.messages()[0]).unwrap();
    assert_eq!(first["timeLimit"], 604_800);
    assert_eq!(first["dlimit"], 25);
}
