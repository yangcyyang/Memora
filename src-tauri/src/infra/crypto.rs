//! AES-256-GCM encryption for API keys at rest.

use aes_gcm::Key;

fn get_encryption_key() -> Key<aes_gcm::Aes256Gcm> {
    use sha2::{Digest, Sha256};
    let machine_id = machine_uid::get().unwrap_or_else(|_| "memora-fallback-id-456".to_string());
    let mut hasher = Sha256::new();
    hasher.update(b"memora-api-key-salt");
    hasher.update(machine_id.as_bytes());
    let result = hasher.finalize();
    *Key::<aes_gcm::Aes256Gcm>::from_slice(result.as_slice())
}

pub fn encrypt_api_key(plain: &str) -> String {
    use aes_gcm::{
        Aes256Gcm, KeyInit,
        aead::{Aead, AeadCore, OsRng},
    };
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    if plain.is_empty() {
        return String::new();
    }
    let key = get_encryption_key();
    let cipher = Aes256Gcm::new(&key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    match cipher.encrypt(&nonce, plain.as_bytes()) {
        Ok(ciphertext) => {
            let mut combined = nonce.to_vec();
            combined.extend_from_slice(&ciphertext);
            BASE64.encode(combined)
        }
        Err(_) => plain.to_string(),
    }
}

pub fn decrypt_api_key(encoded: &str) -> String {
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    if encoded.is_empty() {
        return String::new();
    }
    let Ok(decoded) = BASE64.decode(encoded) else {
        return encoded.to_string();
    };
    if decoded.len() < 12 {
        return encoded.to_string();
    }
    let (nonce_bytes, ciphertext) = decoded.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let key = get_encryption_key();
    let cipher = Aes256Gcm::new(&key);
    match cipher.decrypt(nonce, ciphertext) {
        Ok(plaintext) => String::from_utf8(plaintext).unwrap_or_else(|_| encoded.to_string()),
        Err(_) => encoded.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let original = "test-key-12345";
        let encrypted = encrypt_api_key(original);
        let decrypted = decrypt_api_key(&encrypted);
        assert_eq!(decrypted, original, "roundtrip failed");
    }

    #[test]
    fn test_empty_key() {
        assert_eq!(encrypt_api_key(""), "");
        assert_eq!(decrypt_api_key(""), "");
    }
}

