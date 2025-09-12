// src/game/info.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Magic bytes for .gaem files: "GAEM" in ASCII
pub const GAEM_MAGIC: [u8; 4] = [0x47, 0x41, 0x45, 0x4D];
pub const GAEM_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInfo {
    pub id: Uuid,
    pub title: String,
    pub author: String,
    pub version: String,
    pub description: String,
    pub secret_key_hash: String,  // SHA-256 hash of the secret key
    pub entry_point: String,      // Main script file
    pub required_assets: Vec<AssetInfo>,
    pub engine_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub path: String,
    pub checksum: String,  // SHA-256 checksum
    pub size: u64,
    pub asset_type: AssetType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetType {
    Sprite,
    Audio,
    Script,
    Data,
    Font,
}

impl GameInfo {
    pub fn new(title: String, author: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            title,
            author,
            version: "1.0.0".to_string(),
            description: String::new(),
            secret_key_hash: String::new(),
            entry_point: "main.lua".to_string(),
            required_assets: Vec::new(),
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub fn set_secret_key(&mut self, key: &str) {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        self.secret_key_hash = format!("{:x}", hasher.finalize());
    }

    pub fn verify_secret_key(&self, key: &str) -> bool {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let computed_hash = format!("{:x}", hasher.finalize());
        computed_hash == self.secret_key_hash
    }
}