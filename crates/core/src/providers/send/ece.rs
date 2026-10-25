//! Encrypted content-coding for HTTP (RFC 8188, `aes128gcm`), the way Firefox
//! Send applies it: 64 KiB records, the raw secret as the input keying material.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes128Gcm, Nonce};
use hkdf::Hkdf;
use rand::RngExt;
use sha2::Sha256;

/// Bytes in a record once encrypted.
pub(super) const RECORD_SIZE: usize = 64 * 1024;

/// A record holds this much of the file: the rest is the delimiter and the tag.
pub(super) const DATA_PER_RECORD: usize = RECORD_SIZE - 17;

const TAG_SIZE: usize = 16;

pub(super) struct Encryptor {
    cipher: Aes128Gcm,
    nonce_base: [u8; 12],
    sequence: u32,
}

impl Encryptor {
    /// The encryptor, and the header that must open the stream.
    pub(super) fn new(secret: &[u8; 16]) -> (Self, Vec<u8>) {
        let salt: [u8; 16] = rand::rng().random();
        let hkdf = Hkdf::<Sha256>::new(Some(&salt), secret);
        let mut key = [0u8; 16];
        hkdf.expand(b"Content-Encoding: aes128gcm\0", &mut key)
            .expect("16 bytes is a valid HKDF output length");
        let mut nonce_base = [0u8; 12];
        hkdf.expand(b"Content-Encoding: nonce\0", &mut nonce_base)
            .expect("12 bytes is a valid HKDF output length");

        // salt, record size (4 bytes), key id length (1 byte, no key id).
        let mut header = salt.to_vec();
        header.extend_from_slice(&(RECORD_SIZE as u32).to_be_bytes());
        header.push(0);

        let encryptor = Self {
            cipher: Aes128Gcm::new(&key.into()),
            nonce_base,
            sequence: 0,
        };
        (encryptor, header)
    }

    /// The next record, for up to [`DATA_PER_RECORD`] bytes of the file. Every
    /// record but the last is padded to the full size and ends its data with 1;
    /// the last ends with 2.
    pub(super) fn record(&mut self, data: &[u8], last: bool) -> Vec<u8> {
        debug_assert!(data.len() <= DATA_PER_RECORD);
        let mut plain = data.to_vec();
        if last {
            plain.push(2);
        } else {
            plain.resize(RECORD_SIZE - TAG_SIZE, 0);
            plain[data.len()] = 1;
        }
        let nonce = self.next_nonce();
        self.cipher
            .encrypt(&nonce, plain.as_slice())
            .expect("a record is far below the AES-GCM limit")
    }

    /// The base nonce with the record number XORed into its last four bytes.
    fn next_nonce(&mut self) -> Nonce<aes_gcm::aead::consts::U12> {
        let mut nonce = self.nonce_base;
        for (byte, count) in nonce[8..].iter_mut().zip(self.sequence.to_be_bytes()) {
            *byte ^= count;
        }
        self.sequence += 1;
        nonce.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_is_salt_record_size_and_no_key_id() {
        let (_, header) = Encryptor::new(&[1; 16]);
        assert_eq!(header.len(), 21);
        assert_eq!(&header[16..20], &[0, 1, 0, 0]);
        assert_eq!(header[20], 0);
    }

    #[test]
    fn records_have_the_sizes_a_receiver_expects() {
        let (mut ece, _) = Encryptor::new(&[1; 16]);
        let full = vec![9u8; DATA_PER_RECORD];
        assert_eq!(ece.record(&full, false).len(), RECORD_SIZE);
        assert_eq!(ece.record(&[1, 2, 3], false).len(), RECORD_SIZE);
        // 23 bytes of data, the delimiter and the tag, as in a one-record file.
        assert_eq!(ece.record(&[0; 23], true).len(), 40);
    }

    #[test]
    fn each_record_uses_a_new_nonce() {
        let (mut ece, _) = Encryptor::new(&[1; 16]);
        let a = ece.record(b"same", true);
        let b = ece.record(b"same", true);
        assert_ne!(a, b);
    }
}
