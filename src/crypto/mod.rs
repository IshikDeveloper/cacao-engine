// src/crypto/mod.rs
use aes_gcm::{Aes256Gcm, Nonce, aead::{Aead, KeyInit}};
use sha2::{Sha256, Digest};
use rand::RngCore;
use crate::errors::CacaoError;

pub fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, CacaoError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CacaoError::CryptoError(format!("Failed to init cipher: {:?}", e)))?;

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let encrypted = cipher.encrypt(nonce, data)
        .map_err(|e| CacaoError::CryptoError(format!("Encryption failed: {}", e)))?;

    let mut result = Vec::with_capacity(12 + encrypted.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&encrypted);

    Ok(result)
}

pub fn decrypt_data(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, CacaoError> {
    if data.len() < 12 {
        return Err(CacaoError::CryptoError("Invalid encrypted data: too short".to_string()));
    }

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CacaoError::CryptoError(format!("Failed to init cipher: {:?}", e)))?;

    let nonce = Nonce::from_slice(&data[0..12]);
    let encrypted_data = &data[12..];

    let decrypted = cipher.decrypt(nonce, encrypted_data)
        .map_err(|e| CacaoError::CryptoError(format!("Decryption failed: {}", e)))?;

    Ok(decrypted)
}

pub fn hash_data(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}