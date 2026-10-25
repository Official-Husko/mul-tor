//! The keys of a Firefox Send upload, all derived from one random 16-byte secret
//! that is kept in the part of the link after `#` and never sent to the server.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes128Gcm, Nonce};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hkdf::Hkdf;
use rand::RngExt;
use serde_json::{Value, json};
use sha2::Sha256;

pub(super) struct Keys {
    secret: [u8; 16],
}

impl Keys {
    pub(super) fn random() -> Self {
        Self {
            secret: rand::rng().random(),
        }
    }

    pub(super) fn secret(&self) -> &[u8; 16] {
        &self.secret
    }

    /// The secret as it appears in the link.
    pub(super) fn secret_text(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.secret)
    }

    /// HKDF-SHA256 over the secret, with no salt.
    fn derive<const N: usize>(&self, info: &str) -> [u8; N] {
        let mut out = [0u8; N];
        Hkdf::<Sha256>::new(None, &self.secret)
            .expand(info.as_bytes(), &mut out)
            .expect("N is short enough for HKDF-SHA256");
        out
    }

    /// What the server keeps to recognise the owner of the file later.
    pub(super) fn authorization(&self) -> String {
        let key: [u8; 64] = self.derive("authentication");
        format!("send-v1 {}", URL_SAFE_NO_PAD.encode(key))
    }

    /// The name, size and type of the file, encrypted for the server to keep.
    pub(super) fn encrypted_metadata(&self, name: &str, size: u64) -> String {
        let file = json!({"name": name, "size": size, "type": "application/octet-stream"});
        let metadata = json!({
            "name": name,
            "size": size,
            "type": "application/octet-stream",
            "manifest": {"files": [file]},
        });
        URL_SAFE_NO_PAD.encode(seal_metadata(&self.derive("metadata"), &metadata))
    }
}

/// AES-128-GCM with an all-zero nonce, as the web client does (the key is used once).
fn seal_metadata(key: &[u8; 16], metadata: &Value) -> Vec<u8> {
    Aes128Gcm::new(key.into())
        .encrypt(&Nonce::default(), metadata.to_string().as_bytes())
        .expect("metadata is far below the AES-GCM limit")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_link_secret_is_22_url_safe_characters() {
        let text = Keys::random().secret_text();
        assert_eq!(text.len(), 22);
        assert!(
            text.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
    }

    #[test]
    fn the_authorization_is_a_64_byte_key() {
        let auth = Keys::random().authorization();
        let (scheme, key) = auth.split_once(' ').unwrap();
        assert_eq!(scheme, "send-v1");
        assert_eq!(URL_SAFE_NO_PAD.decode(key).unwrap().len(), 64);
    }

    #[test]
    fn the_same_secret_always_derives_the_same_keys() {
        let keys = Keys { secret: [7; 16] };
        let again = Keys { secret: [7; 16] };
        assert_eq!(keys.authorization(), again.authorization());
        assert_eq!(
            keys.encrypted_metadata("a.bin", 5),
            again.encrypted_metadata("a.bin", 5)
        );
    }
}
