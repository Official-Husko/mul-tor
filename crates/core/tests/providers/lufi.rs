//! Lufi: slices encrypted in the client, sent over a WebSocket, key kept in the link.

use aes::Aes128;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use ccm::Ccm;
use ccm::aead::{Aead, KeyInit};
use ccm::consts::{U8, U11, U12, U13};
use mul_tor_core::UploadError;
use serde_json::{Value, json};
use sha2::Sha256;

use crate::common::*;

const INDEX_JS: &str =
    "var ws_url  = 'wss://upload.adminforge.de/upload';\nvar maxSize = 2147483648;";
const PAGE: &str = r#"<select id="delete-day"><option value="1" >24 hours</option>
    <option value="7" selected="selected">7 days</option><option value="30" >30 days</option></select>"#;

fn site(index_js: &'static str, page: &'static str) -> impl Fn(&Recorded) -> Reply {
    move |r| {
        if r.is("GET", "upload.adminforge.de", "/partial/index.js") {
            Reply::text(index_js)
        } else if r.is("GET", "upload.adminforge.de", "/") {
            Reply::html(page)
        } else {
            Reply::not_found()
        }
    }
}

/// What the instance answers to each slice: success, with the id it hands out.
fn accepting(text: &str) -> Vec<String> {
    let (meta, _) = text.split_once("XXMOJOXX").unwrap();
    let meta: Value = serde_json::from_str(meta).unwrap();
    vec![
        json!({
            "success": true, "j": meta["part"], "parts": meta["total"], "short": "SHORT123",
            "name": meta["name"], "size": meta["size"], "delay": meta["delay"],
            "token": "deletion-token", "i": 0,
        })
        .to_string(),
    ]
}

/// What the web page does to read a slice back: sjcl's decryption.
fn decrypt(key: &str, sjcl_json: &str) -> Vec<u8> {
    let json: Value = serde_json::from_str(sjcl_json).unwrap();
    let field = |name: &str| STANDARD.decode(json[name].as_str().unwrap()).unwrap();
    let (iv, salt, sealed) = (field("iv"), field("salt"), field("ct"));
    let mut derived = [0u8; 16];
    pbkdf2::pbkdf2_hmac::<Sha256>(key.as_bytes(), &salt, 10_000, &mut derived);
    let length_bytes = (2..4)
        .find(|l| (sealed.len() - 8) >> (8 * l) == 0)
        .unwrap_or(4);
    macro_rules! open {
        ($size:ty, $n:expr) => {
            Ccm::<Aes128, U8, $size>::new((&derived).into())
                .decrypt(
                    (&<[u8; $n]>::try_from(&iv[..$n]).unwrap()).into(),
                    sealed.as_slice(),
                )
                .expect("the slice decrypts with the key from the link")
        };
    }
    match length_bytes {
        2 => open!(U13, 13),
        3 => open!(U12, 12),
        _ => open!(U11, 11),
    }
}

/// `(meta, decrypted slice)` of a message as the server receives it.
fn read_message(key: &str, text: &str) -> (Value, Vec<u8>) {
    let (meta, body) = text.split_once("XXMOJOXX").unwrap();
    // The encrypted text was stringified once more by the client.
    let encrypted: String = serde_json::from_str(body).unwrap();
    let base64_text = decrypt(key, &encrypted);
    let slice = STANDARD.decode(base64_text).unwrap();
    (serde_json::from_str(meta).unwrap(), slice)
}

#[tokio::test]
async fn lufi_sends_encrypted_slices_and_keeps_the_key_in_the_link() {
    let content: Vec<u8> = (0..1_600_000u32).map(|n| (n % 251) as u8).collect();
    let mock = Mock::start_with_socket(site(INDEX_JS, PAGE), accepting).await;
    let run = run(
        &*builtin("adminforge"),
        &mock,
        RunConfig {
            name: "backup.bin".into(),
            content: content.clone(),
            ..Default::default()
        },
    )
    .await;

    let (address, key) = run
        .url()
        .split_once('#')
        .expect("the key is in the fragment");
    assert_eq!(address, "https://upload.adminforge.de/r/SHORT123");
    assert_eq!(STANDARD.decode(key).unwrap().len(), 32, "256 random bits");

    let messages = mock.messages();
    assert_eq!(messages.len(), 3, "750000 bytes per slice");
    let mut rebuilt = Vec::new();
    for (index, message) in messages.iter().enumerate() {
        let (meta, slice) = read_message(key, message);
        assert_eq!(meta["part"], index);
        assert_eq!(meta["total"], 3);
        assert_eq!(meta["name"], "backup.bin");
        assert_eq!(meta["size"], 1_600_000);
        // The longest retention the page offers.
        assert_eq!(meta["delay"], 30);
        assert_eq!(meta["zipped"], false);
        // The first slice has no id yet; the rest carry the one the server gave.
        assert_eq!(
            meta["id"],
            if index == 0 {
                Value::Null
            } else {
                json!("SHORT123")
            }
        );
        rebuilt.extend(slice);
    }
    assert_eq!(rebuilt, content);
    assert_eq!(run.progress.sent(), 1_600_000);
    // The key never travels to the server.
    assert!(messages.iter().all(|m| !m.contains(key)));
}

/// Just the metadata of a message, which is not encrypted.
fn meta_of(message: &str) -> Value {
    serde_json::from_str(message.split_once("XXMOJOXX").unwrap().0).unwrap()
}

#[tokio::test]
async fn lufi_asks_for_no_expiry_when_the_page_offers_it() {
    let page = r#"<select id="delete-day"><option value="0">No limit</option><option value="365">1 year</option></select>"#;
    let mock = Mock::start_with_socket(site(INDEX_JS, page), accepting).await;
    run_default(&*builtin("adminforge"), &mock).await.url();

    assert_eq!(meta_of(&mock.messages()[0])["delay"], 0);
}

#[tokio::test]
async fn lufi_does_not_connect_to_a_socket_off_the_instance() {
    let index_js = "var ws_url = 'wss://evil.example/upload';";
    let mock = Mock::start_with_socket(site(index_js, PAGE), accepting).await;
    let run = run_default(&*builtin("adminforge"), &mock).await;

    assert!(matches!(run.error(), UploadError::Protocol { .. }));
    assert!(mock.messages().is_empty());
    assert!(mock.requests().iter().all(|r| r.path != "/upload"));
}

#[tokio::test]
async fn lufi_stops_when_the_instance_refuses_a_slice() {
    let mock = Mock::start_with_socket(site(INDEX_JS, PAGE), |_| {
        vec![json!({"success": false, "msg": "File too large"}).to_string()]
    })
    .await;
    let run = run_default(&*builtin("adminforge"), &mock).await;

    let message = run.error().to_string();
    assert!(message.contains("File too large"), "{message}");
    assert_eq!(mock.messages().len(), 1);
}

#[tokio::test]
async fn lufi_refuses_an_id_that_is_not_plain() {
    let mock = Mock::start_with_socket(site(INDEX_JS, PAGE), |_| {
        vec![json!({"success": true, "short": "../../x"}).to_string()]
    })
    .await;
    let run = run_default(&*builtin("adminforge"), &mock).await;
    assert!(matches!(run.error(), UploadError::Protocol { .. }));
}

#[tokio::test]
async fn lufi_fails_clearly_when_the_page_lists_no_retention() {
    let mock = Mock::start_with_socket(site(INDEX_JS, "<html>changed</html>"), accepting).await;
    let run = run_default(&*builtin("adminforge"), &mock).await;
    assert!(run.error().to_string().contains("retention"));
    assert!(mock.messages().is_empty());
}

#[tokio::test]
async fn disroot_has_its_own_host_and_choices() {
    let mock = Mock::start_with_socket(
        |r| {
            if r.is("GET", "upload.disroot.org", "/partial/index.js") {
                Reply::text("var ws_url  = 'wss://upload.disroot.org/upload';")
            } else if r.is("GET", "upload.disroot.org", "/") {
                Reply::html(
                    r#"<select id="delete-day"><option value="1">24 hours</option><option value="90">90 days</option><option value="30">30 days</option></select>"#,
                )
            } else {
                Reply::not_found()
            }
        },
        accepting,
    )
    .await;
    let run = run_default(&*builtin("disroot"), &mock).await;

    assert!(
        run.url()
            .starts_with("https://upload.disroot.org/r/SHORT123#")
    );
    assert_eq!(meta_of(&mock.messages()[0])["delay"], 90);
}
