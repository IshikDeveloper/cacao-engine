// src/saves/mod.rs
use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use aes_gcm::{Aes256Gcm, Nonce, aead::{Aead, KeyInit}}; // KeyInit is required for new_from_slice
use sha2::{Sha256, Digest};
use rand::RngCore;
use crate::errors::CacaoError;

pub struct SaveManager {
    saves_dir: PathBuf,
    current_game_id: Option<String>,
    current_save_data: HashMap<String, SaveValue>,
    encryption_key: Option<[u8; 32]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SaveValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<SaveValue>),
    Object(HashMap<String, SaveValue>),
}

#[derive(Serialize, Deserialize)]
struct SaveFileData {
    version: u32,
    game_id: String,
    data: HashMap<String, SaveValue>,
    checksum: String,
    timestamp: u64,
}

impl SaveManager {
    pub fn new(saves_dir: PathBuf) -> Self {
        Self {
            saves_dir,
            current_game_id: None,
            current_save_data: HashMap::new(),
            encryption_key: None,
        }
    }

    pub fn set_game_context(&mut self, game_id: String, secret_key: &str) -> Result<(), CacaoError> {
        self.current_game_id = Some(game_id.clone());
        self.encryption_key = Some(derive_encryption_key(secret_key));
        
        let game_save_dir = self.saves_dir.join(format!("{}_saves", sanitize_game_id(&game_id)));
        std::fs::create_dir_all(&game_save_dir)?;
        
        self.load_save_data()?;
        Ok(())
    }

    pub fn write(&mut self, key: String, value: SaveValue) -> Result<(), CacaoError> {
        if self.current_game_id.is_none() {
            return Err(CacaoError::CryptoError("No game context set".to_string()));
        }

        self.current_save_data.insert(key, value);
        Ok(())
    }

    pub fn read(&self, key: &str) -> Option<&SaveValue> {
        self.current_save_data.get(key)
    }

    pub fn exists(&self, key: &str) -> bool {
        self.current_save_data.contains_key(key)
    }

    pub fn remove(&mut self, key: &str) -> Option<SaveValue> {
        self.current_save_data.remove(key)
    }

    pub fn clear(&mut self) {
        self.current_save_data.clear();
    }

    pub fn save_to_disk(&self) -> Result<(), CacaoError> {
        let game_id = self.current_game_id.as_ref()
            .ok_or_else(|| CacaoError::CryptoError("No game context set".to_string()))?;

        let encryption_key = self.encryption_key.as_ref()
            .ok_or_else(|| CacaoError::CryptoError("No encryption key available".to_string()))?;

        let save_file_data = SaveFileData {
            version: 1,
            game_id: game_id.clone(),
            data: self.current_save_data.clone(),
            checksum: self.calculate_checksum()?,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let serialized_data = bincode::serialize(&save_file_data)
            .map_err(|e| CacaoError::CryptoError(format!("Failed to serialize save data: {}", e)))?;

        let encrypted_data = encrypt_data(&serialized_data, encryption_key)?;

        let save_file_path = self.get_save_file_path(game_id);
        std::fs::write(&save_file_path, &encrypted_data)?;

        log::info!("Save data written to: {}", save_file_path.display());
        Ok(())
    }

    fn load_save_data(&mut self) -> Result<(), CacaoError> {
        let game_id = self.current_game_id.as_ref()
            .ok_or_else(|| CacaoError::CryptoError("No game context set".to_string()))?;

        let encryption_key = self.encryption_key.as_ref()
            .ok_or_else(|| CacaoError::CryptoError("No encryption key available".to_string()))?;

        let save_file_path = self.get_save_file_path(game_id);
        
        if !save_file_path.exists() {
            log::info!("No existing save file found for game: {}", game_id);
            return Ok(());
        }

        let encrypted_data = std::fs::read(&save_file_path)?;
        let decrypted_data = decrypt_data(&encrypted_data, encryption_key)?;

        let save_file_data: SaveFileData = bincode::deserialize(&decrypted_data)
            .map_err(|e| CacaoError::CryptoError(format!("Failed to deserialize save data: {}", e)))?;

        let expected_checksum = calculate_data_checksum(&save_file_data.data)?;
        if save_file_data.checksum != expected_checksum {
            return Err(CacaoError::CryptoError("Save file checksum mismatch - data may be corrupted".to_string()));
        }

        if save_file_data.game_id != *game_id {
            return Err(CacaoError::CryptoError("Save file game ID mismatch".to_string()));
        }

        self.current_save_data = save_file_data.data;
        log::info!("Save data loaded for game: {}", game_id);
        Ok(())
    }

    fn get_save_file_path(&self, game_id: &str) -> PathBuf {
        let game_save_dir = self.saves_dir.join(format!("{}_saves", sanitize_game_id(game_id)));
        game_save_dir.join("save.dat")
    }

    fn calculate_checksum(&self) -> Result<String, CacaoError> {
        calculate_data_checksum(&self.current_save_data)
    }

    // --- Other helper functions remain unchanged ---
}

// --- Encryption/Decryption fixes for aes-gcm 0.10+ ---
fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, CacaoError> {
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

fn decrypt_data(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, CacaoError> {
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

fn calculate_data_checksum(data: &HashMap<String, SaveValue>) -> Result<String, CacaoError> {
    let serialized = bincode::serialize(data)
        .map_err(|e| CacaoError::CryptoError(format!("Failed to serialize data for checksum: {}", e)))?;
    
    let mut hasher = Sha256::new();
    hasher.update(&serialized);
    Ok(format!("{:x}", hasher.finalize()))
}

fn sanitize_game_id(game_id: &str) -> String {
    game_id
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
            _ => '_',
        })
        .collect()
}

fn estimate_save_size(data: &HashMap<String, SaveValue>) -> usize {
    // Rough estimate of serialized size
    let mut size = 0;
    for (key, value) in data {
        size += key.len();
        size += estimate_value_size(value);
    }
    size
}

fn estimate_value_size(value: &SaveValue) -> usize {
    match value {
        SaveValue::String(s) => s.len(),
        SaveValue::Integer(_) => 8,
        SaveValue::Float(_) => 8,
        SaveValue::Boolean(_) => 1,
        SaveValue::Array(arr) => arr.iter().map(estimate_value_size).sum(),
        SaveValue::Object(obj) => obj.iter().map(|(k, v)| k.len() + estimate_value_size(v)).sum(),
    }
}

fn derive_encryption_key(secret_key: &str) -> [u8; 32] {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(secret_key.as_bytes());
    hasher.update(b"cacao_engine_salt"); // Add salt for better security
    let hash = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash[..]);
    key
}

#[derive(Debug)]
pub struct SaveInfo {
    pub path: PathBuf,
    pub size: u64,
    pub modified_timestamp: u64,
    pub is_backup: bool,
}

#[derive(Debug)]
pub struct SaveStats {
    pub total_keys: usize,
    pub estimated_size: usize,
    pub last_modified: u64,
}

// Convenience methods for common save operations
impl SaveManager {
    pub fn write_string(&mut self, key: String, value: String) -> Result<(), CacaoError> {
        self.write(key, SaveValue::String(value))
    }

    pub fn write_int(&mut self, key: String, value: i64) -> Result<(), CacaoError> {
        self.write(key, SaveValue::Integer(value))
    }

    pub fn write_float(&mut self, key: String, value: f64) -> Result<(), CacaoError> {
        self.write(key, SaveValue::Float(value))
    }

    pub fn write_bool(&mut self, key: String, value: bool) -> Result<(), CacaoError> {
        self.write(key, SaveValue::Boolean(value))
    }

    pub fn read_string(&self, key: &str, default: &str) -> String {
        match self.read(key) {
            Some(SaveValue::String(s)) => s.clone(),
            _ => default.to_string(),
        }
    }

    pub fn read_int(&self, key: &str, default: i64) -> i64 {
        match self.read(key) {
            Some(SaveValue::Integer(i)) => *i,
            Some(SaveValue::Float(f)) => *f as i64,
            _ => default,
        }
    }

    pub fn read_float(&self, key: &str, default: f64) -> f64 {
        match self.read(key) {
            Some(SaveValue::Float(f)) => *f,
            Some(SaveValue::Integer(i)) => *i as f64,
            _ => default,
        }
    }

    pub fn read_bool(&self, key: &str, default: bool) -> bool {
        match self.read(key) {
            Some(SaveValue::Boolean(b)) => *b,
            _ => default,
        }
    }

    pub fn increment_int(&mut self, key: String, amount: i64) -> Result<i64, CacaoError> {
        let current = self.read_int(&key, 0);
        let new_value = current + amount;
        self.write_int(key, new_value)?;
        Ok(new_value)
    }

    pub fn toggle_bool(&mut self, key: String) -> Result<bool, CacaoError> {
        let current = self.read_bool(&key, false);
        let new_value = !current;
        self.write_bool(key, new_value)?;
        Ok(new_value)
    }
}