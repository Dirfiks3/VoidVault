use crate::{
    error::{err, AppResult},
    models::KdfParams,
};
use aes_gcm::{
    aead::{Aead, Payload},
    Aes256Gcm, KeyInit, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD, Engine};
use rand::RngCore;
use std::ops::{Deref, DerefMut};
use zeroize::Zeroizing;

pub const KEY_BYTES: usize = 32;
pub const NONCE_BYTES: usize = 12;
pub const BACKUP_HEADER: &str = "VOIDVAULT 1";
pub const VERIFIER_PLAINTEXT: &str = "VOIDVAULT-OK";
pub const AAD_VERIFIER: &str = "voidvault/verifier/v1";
pub const AAD_BACKUP: &str = "voidvault/backup/v1";

// The key is allocated on the heap so its address is stable, then pinned in
// physical memory when the OS allows it. Every clone gets its own pinned
// buffer; Drop unpins before Zeroizing wipes the bytes.
pub struct MasterKey {
    bytes: Zeroizing<Vec<u8>>,
}

impl MasterKey {
    pub fn zeroed() -> Self {
        Self::from_bytes(&[0u8; KEY_BYTES])
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut buffer = vec![0u8; bytes.len()];
        buffer.copy_from_slice(bytes);
        lock_pages(&mut buffer);
        Self {
            bytes: Zeroizing::new(buffer),
        }
    }
}

impl Clone for MasterKey {
    fn clone(&self) -> Self {
        Self::from_bytes(&self.bytes)
    }
}

impl Drop for MasterKey {
    fn drop(&mut self) {
        unlock_pages(&mut self.bytes);
    }
}

impl Deref for MasterKey {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl DerefMut for MasterKey {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bytes
    }
}

#[cfg(windows)]
fn lock_pages(bytes: &mut [u8]) {
    use windows_sys::Win32::System::Memory::VirtualLock;
    if bytes.is_empty() {
        return;
    }
    let pointer = bytes.as_mut_ptr().cast();
    // Best effort: failure only means the key may be paged out.
    unsafe {
        let _ = VirtualLock(pointer, bytes.len());
    }
}

#[cfg(windows)]
fn unlock_pages(bytes: &mut [u8]) {
    use windows_sys::Win32::System::Memory::VirtualUnlock;
    if bytes.is_empty() {
        return;
    }
    unsafe {
        let _ = VirtualUnlock(bytes.as_mut_ptr().cast(), bytes.len());
    }
}

#[cfg(unix)]
fn lock_pages(bytes: &mut [u8]) {
    if bytes.is_empty() {
        return;
    }
    unsafe {
        let _ = libc::mlock(bytes.as_mut_ptr().cast(), bytes.len());
    }
}

#[cfg(unix)]
fn unlock_pages(bytes: &mut [u8]) {
    if bytes.is_empty() {
        return;
    }
    unsafe {
        let _ = libc::munlock(bytes.as_mut_ptr().cast(), bytes.len());
    }
}

#[cfg(not(any(unix, windows)))]
fn lock_pages(_bytes: &mut [u8]) {}

#[cfg(not(any(unix, windows)))]
fn unlock_pages(_bytes: &mut [u8]) {}

pub fn field_aad(item_id: &str, field: &str) -> String {
    format!("voidvault/field/v1/{item_id}/{field}")
}

pub fn random_salt() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    STANDARD.encode(bytes)
}

pub fn derive_key(password: &str, kdf: &KdfParams) -> AppResult<MasterKey> {
    let salt = STANDARD.decode(&kdf.salt).map_err(err)?;
    let params = Params::new(
        kdf.memory_kib,
        kdf.iterations,
        kdf.parallelism,
        Some(kdf.hash_len),
    )
    .map_err(err)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = MasterKey::zeroed();
    argon2
        .hash_password_into(password.as_bytes(), &salt, &mut key[..])
        .map_err(err)?;
    Ok(key)
}

fn cipher(key: &MasterKey) -> AppResult<Aes256Gcm> {
    Aes256Gcm::new_from_slice(&key[..]).map_err(err)
}

pub fn encrypt_bytes(key: &MasterKey, aad: &str, plaintext: &[u8]) -> AppResult<String> {
    let mut nonce_bytes = [0u8; NONCE_BYTES];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher(key)?
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: plaintext,
                aad: aad.as_bytes(),
            },
        )
        .map_err(err)?;
    let mut blob = Vec::with_capacity(NONCE_BYTES + ciphertext.len());
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&ciphertext);
    Ok(STANDARD.encode(blob))
}

pub fn decrypt_bytes(key: &MasterKey, aad: &str, blob: &str) -> AppResult<Vec<u8>> {
    if blob.is_empty() {
        return Ok(Vec::new());
    }
    let raw = STANDARD.decode(blob).map_err(err)?;
    if raw.len() < NONCE_BYTES {
        return Err(crate::error::AppError::new("Повреждённые зашифрованные данные"));
    }
    let (nonce, ciphertext) = raw.split_at(NONCE_BYTES);
    cipher(key)?
        .decrypt(
            Nonce::from_slice(nonce),
            Payload {
                msg: ciphertext,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| crate::error::AppError::new("Не удалось расшифровать данные"))
}

pub fn encrypt_field(key: &MasterKey, aad: &str, value: &str) -> AppResult<String> {
    encrypt_bytes(key, aad, value.as_bytes())
}

pub fn decrypt_field(key: &MasterKey, aad: &str, blob: &str) -> AppResult<String> {
    let bytes = decrypt_bytes(key, aad, blob)?;
    String::from_utf8(bytes).map_err(err)
}

pub fn wrap_base64(encoded: &str, width: usize) -> String {
    let mut out = String::with_capacity(encoded.len() + encoded.len() / width);
    for (index, chunk) in encoded.as_bytes().chunks(width).enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(std::str::from_utf8(chunk).unwrap_or_default());
    }
    out
}

pub fn unwrap_base64(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Параметры намеренно облегчены, чтобы тесты не тратили секунды на Argon2id.
    fn test_kdf() -> KdfParams {
        KdfParams {
            algorithm: "argon2id".to_string(),
            memory_kib: 1024,
            iterations: 1,
            parallelism: 1,
            hash_len: KEY_BYTES,
            salt: random_salt(),
        }
    }

    #[test]
    fn derive_key_is_deterministic_for_same_kdf() {
        let kdf = test_kdf();
        let first = derive_key("correct horse battery staple", &kdf).unwrap();
        let second = derive_key("correct horse battery staple", &kdf).unwrap();
        assert_eq!(&first[..], &second[..]);
    }

    #[test]
    fn different_salts_produce_different_keys() {
        let left = derive_key("same password", &test_kdf()).unwrap();
        let right = derive_key("same password", &test_kdf()).unwrap();
        assert_ne!(&left[..], &right[..]);
    }

    #[test]
    fn field_encryption_roundtrips_and_binds_aad() {
        let key = derive_key("unit-test-password", &test_kdf()).unwrap();
        let ciphertext = encrypt_field(&key, "voidvault/test/field", "Секрет ✓").unwrap();
        assert_ne!(ciphertext, "Секрет ✓");
        assert_eq!(
            decrypt_field(&key, "voidvault/test/field", &ciphertext).unwrap(),
            "Секрет ✓"
        );
        assert!(decrypt_field(&key, "voidvault/test/other", &ciphertext).is_err());
    }

    #[test]
    fn tampered_ciphertext_is_rejected() {
        let key = derive_key("unit-test-password", &test_kdf()).unwrap();
        let ciphertext = encrypt_field(&key, "voidvault/test/field", "important").unwrap();
        let mut bytes = ciphertext.into_bytes();
        let position = bytes.len() - 2;
        bytes[position] = if bytes[position] == b'A' { b'B' } else { b'A' };
        let tampered = String::from_utf8(bytes).unwrap();
        assert!(decrypt_field(&key, "voidvault/test/field", &tampered).is_err());
    }

    #[test]
    fn empty_field_is_encrypted_as_empty() {
        let key = derive_key("unit-test-password", &test_kdf()).unwrap();
        let ciphertext = encrypt_field(&key, "voidvault/test/empty", "").unwrap();
        assert_eq!(decrypt_field(&key, "voidvault/test/empty", &ciphertext).unwrap(), "");
    }

    #[test]
    fn base64_wrapping_survives_line_breaks() {
        let encoded = STANDARD.encode(vec![7u8; 200]);
        let wrapped = wrap_base64(&encoded, 40);
        assert!(wrapped.contains('\n'));
        assert_eq!(unwrap_base64(&wrapped), encoded);
    }

    #[test]
    fn master_key_clone_owns_separate_buffer() {
        let mut original = MasterKey::from_bytes(&[9u8; KEY_BYTES]);
        let clone = original.clone();
        original[0] = 1;
        assert_eq!(clone[0], 9);
        assert_ne!(&original[..], &clone[..]);
    }

    #[test]
    fn field_aad_binds_item_and_field() {
        assert_eq!(
            field_aad("item-42", "password"),
            "voidvault/field/v1/item-42/password"
        );
    }
}
