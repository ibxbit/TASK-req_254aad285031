// Field-level AES-256-GCM encryption with a local 32-byte key file.
//
// Wire format: nonce (12 bytes) || ciphertext || auth tag (16 bytes).
// Total stored length = 12 + plaintext_len + 16.
//
// The key is loaded once from disk on startup (see `Encryptor::from_file`)
// and kept in memory inside Rocket's State. Plaintext never leaves this
// module; `mask_identifier` is used to derive a safe display form that can
// be rendered in API/UI output alongside the ciphertext blob in the DB.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;
use std::fs;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("key file not found or unreadable: {0}")]
    KeyFileMissing(String),
    #[error("key file must be exactly 32 bytes (got {0})")]
    KeyLenInvalid(usize),
    #[error("ciphertext is malformed or truncated")]
    MalformedCiphertext,
    #[error("AES-GCM authentication failed (tampered or wrong key)")]
    AuthFailed,
    #[error("encryption failed")]
    EncryptFailed,
}

#[derive(Clone)]
pub struct Encryptor {
    cipher: Aes256Gcm,
}

impl Encryptor {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, CryptoError> {
        let p = path.as_ref();
        let bytes =
            fs::read(p).map_err(|_| CryptoError::KeyFileMissing(p.display().to_string()))?;
        if bytes.len() != 32 {
            return Err(CryptoError::KeyLenInvalid(bytes.len()));
        }
        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&bytes);
        Ok(Self {
            cipher: Aes256Gcm::new(key),
        })
    }

    /// For tests only: build an encryptor from a raw 32-byte key.
    #[cfg(test)]
    pub fn from_key_bytes(bytes: &[u8; 32]) -> Self {
        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(bytes);
        Self {
            cipher: Aes256Gcm::new(key),
        }
    }

    /// Encrypt plaintext -> nonce || ciphertext || tag. A fresh random
    /// 96-bit nonce is generated per call, so the output is non-deterministic
    /// even for identical plaintext — required by GCM for security.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ct = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| CryptoError::EncryptFailed)?;
        let mut out = Vec::with_capacity(12 + ct.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ct);
        Ok(out)
    }

    /// Decrypt nonce || ciphertext || tag back to plaintext. Fails with
    /// `AuthFailed` on tamper or wrong key (GCM tag verification).
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if data.len() < 12 + 16 {
            return Err(CryptoError::MalformedCiphertext);
        }
        let (nonce_bytes, ct) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        self.cipher
            .decrypt(nonce, ct)
            .map_err(|_| CryptoError::AuthFailed)
    }
}

/// Mask a sensitive identifier for safe display in logs/responses.
///
/// - Strings ≤ 4 chars become all X.
/// - Otherwise keeps last 4 chars visible; the rest become X.
/// - Non-alphanumeric characters (dashes, spaces) are preserved.
pub fn mask_identifier(raw: &str) -> String {
    let trimmed: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    let visible_tail = 4usize;
    let total = trimmed.chars().count();
    if total == 0 {
        return String::new();
    }
    if total <= visible_tail {
        return "X".repeat(total);
    }
    let hidden = total - visible_tail;
    let mut out = String::with_capacity(total);
    for (i, c) in trimmed.chars().enumerate() {
        if i < hidden {
            if c.is_alphanumeric() {
                out.push('X');
            } else {
                out.push(c);
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc() -> Encryptor {
        Encryptor::from_key_bytes(&[7u8; 32])
    }

    #[test]
    fn roundtrip_ok() {
        let e = enc();
        let ct = e.encrypt(b"secret-id-12345").unwrap();
        assert_ne!(&ct[..], b"secret-id-12345");
        let pt = e.decrypt(&ct).unwrap();
        assert_eq!(pt, b"secret-id-12345");
    }

    #[test]
    fn nonce_is_fresh_per_call() {
        let e = enc();
        let a = e.encrypt(b"same").unwrap();
        let b = e.encrypt(b"same").unwrap();
        assert_ne!(a, b, "GCM nonce reuse would break confidentiality");
    }

    #[test]
    fn tamper_detected() {
        let e = enc();
        let mut ct = e.encrypt(b"secret").unwrap();
        let last = ct.len() - 1;
        ct[last] ^= 0x01;
        assert!(matches!(e.decrypt(&ct), Err(CryptoError::AuthFailed)));
    }

    #[test]
    fn truncated_fails_cleanly() {
        let e = enc();
        let ct = vec![0u8; 10];
        assert!(matches!(
            e.decrypt(&ct),
            Err(CryptoError::MalformedCiphertext)
        ));
    }

    #[test]
    fn mask_preserves_dashes_and_last_four() {
        assert_eq!(mask_identifier("123-45-6789"), "XXX-XX-6789");
        assert_eq!(mask_identifier("ABCD"), "XXXX");
        assert_eq!(mask_identifier("A1B2C3D4E5"), "XXXXXXD4E5");
        assert_eq!(mask_identifier(""), "");
    }

    #[test]
    fn mask_short_all_x() {
        assert_eq!(mask_identifier("abc"), "XXX");
    }
}
