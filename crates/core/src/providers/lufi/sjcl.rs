//! The encryption the Lufi web client applies to every slice: `sjcl.encrypt`
//! with its defaults, so the download page can decrypt what is sent.
//!
//! That is AES-128-CCM with an 8-byte tag, keyed by PBKDF2-HMAC-SHA256 (10000
//! rounds, an 8-byte salt) over the password, and written as the JSON text sjcl
//! writes. sjcl clamps its 16-byte IV to a nonce that depends on the length of
//! the message: 13 bytes below 64 KiB, 12 bytes below 16 MiB, 11 above.

use aes::Aes128;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use ccm::Ccm;
use ccm::aead::{Aead, KeyInit};
use ccm::consts::{U8, U11, U12, U13};
use rand::RngExt;
use sha2::Sha256;

const ROUNDS: u32 = 10_000;

/// `plaintext` encrypted under `password`, as sjcl's JSON text.
pub(super) fn encrypt(password: &str, plaintext: &[u8]) -> String {
    let mut rng = rand::rng();
    let iv: [u8; 16] = rng.random();
    let salt: [u8; 8] = rng.random();
    encrypt_with(password, plaintext, iv, salt)
}

fn encrypt_with(password: &str, plaintext: &[u8], iv: [u8; 16], salt: [u8; 8]) -> String {
    let mut key = [0u8; 16];
    pbkdf2::pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, ROUNDS, &mut key);
    let sealed = seal(&key, &iv, plaintext);
    format!(
        r#"{{"iv":"{}","v":1,"iter":{ROUNDS},"ks":128,"ts":64,"mode":"ccm","adata":"","cipher":"aes","salt":"{}","ct":"{}"}}"#,
        STANDARD.encode(iv),
        STANDARD.encode(salt),
        STANDARD.encode(sealed),
    )
}

/// Ciphertext followed by the tag.
fn seal(key: &[u8; 16], iv: &[u8; 16], plaintext: &[u8]) -> Vec<u8> {
    // sjcl: the smallest L (2 to 4) whose bytes can hold the length of the message.
    let length_bytes = (2..4)
        .find(|l| plaintext.len() >> (8 * l) == 0)
        .unwrap_or(4);
    macro_rules! with_nonce {
        ($size:ty, $nonce_len:expr) => {{
            let cipher = Ccm::<Aes128, U8, $size>::new(key.into());
            let nonce: [u8; $nonce_len] = iv[..$nonce_len].try_into().expect("IV is 16 bytes");
            // Only fails for a message too long for the nonce, which L rules out.
            cipher
                .encrypt((&nonce).into(), plaintext)
                .expect("message fits the nonce")
        }};
    }
    match length_bytes {
        2 => with_nonce!(U13, 13),
        3 => with_nonce!(U12, 12),
        _ => with_nonce!(U11, 11),
    }
}

#[cfg(test)]
mod tests {
    use sha2::Digest;

    use super::*;

    const IV: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    const SALT: [u8; 8] = [0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17];
    const PASSWORD: &str = "hunter2+/=key";

    // Produced by the sjcl.js a Lufi instance serves, with this IV and salt.
    #[test]
    fn a_short_message_matches_what_sjcl_writes() {
        assert_eq!(
            encrypt_with(PASSWORD, b"SGVsbG8gTHVmaQ==", IV, SALT),
            r#"{"iv":"AAECAwQFBgcICQoLDA0ODw==","v":1,"iter":10000,"ks":128,"ts":64,"mode":"ccm","adata":"","cipher":"aes","salt":"EBESExQVFhc=","ct":"xgxTmPz5M1j0vvNKOQGVgkr4IylHsvud"}"#
        );
    }

    #[test]
    fn an_empty_message_is_only_the_tag() {
        assert_eq!(
            encrypt_with(PASSWORD, b"", IV, SALT),
            r#"{"iv":"AAECAwQFBgcICQoLDA0ODw==","v":1,"iter":10000,"ks":128,"ts":64,"mode":"ccm","adata":"","cipher":"aes","salt":"EBESExQVFhc=","ct":"c3sN7zFe0gI="}"#
        );
    }

    #[test]
    fn a_message_over_64_kib_uses_the_shorter_nonce_like_sjcl() {
        let sealed = encrypt_with(PASSWORD, &vec![b'A'; 70_000], IV, SALT);
        let digest: String = Sha256::digest(sealed.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            digest,
            "b8c19e335849aa48b87243e0cebcecd87e8b8b1afe852a9eb109a5541b981dce"
        );
    }

    #[test]
    fn every_call_picks_a_new_salt_and_iv() {
        let a = encrypt("k", b"x");
        let b = encrypt("k", b"x");
        assert_ne!(a, b);
    }
}
