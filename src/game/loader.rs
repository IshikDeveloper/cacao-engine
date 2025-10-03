// src/game/loader.rs
use super::{Game, GameInfo, GAEM_MAGIC, GAEM_VERSION};
use crate::{assets::AssetManager, errors::CacaoError, game::AssetInfo};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};

pub struct GameLoader {
    games_dir: PathBuf,
}

impl GameLoader {
    pub fn new(games_dir: PathBuf) -> Self {
        Self { games_dir }
    }

    pub async fn load_game(
        &self,
        game_file: &Path,
        assets: &mut AssetManager,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Game, CacaoError> {
        let game_info = self.parse_gaem_file(game_file)?;
        let game_folder = self.find_game_folder(&game_info)?;

        // Load all required assets
        for asset_info in &game_info.required_assets {
            let asset_path = game_folder.join(&asset_info.path);
            self.verify_asset(&asset_path, asset_info)?;

            #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
            pub enum AssetType {
                Sprite,
                Audio,
                Script,
                Data,
                Font,
            }

            assets
                .load_asset(&asset_path, asset_info.asset_type.clone(), device, queue)
                .await?;
        }

        let game = Game::new(game_info, game_folder);
        Ok(game)
    }

    fn parse_gaem_file(&self, file_path: &Path) -> Result<GameInfo, CacaoError> {
        let mut file = File::open(file_path)?;

        // Read and verify magic bytes
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if magic != GAEM_MAGIC {
            return Err(CacaoError::GameLoadError(
                "Invalid .gaem file format".to_string(),
            ));
        }

        // Read version
        let mut version_bytes = [0u8; 2];
        file.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);
        if version != GAEM_VERSION {
            return Err(CacaoError::GameLoadError(format!(
                "Unsupported .gaem version: {}",
                version
            )));
        }

        // Read header size
        let mut header_size_bytes = [0u8; 4];
        file.read_exact(&mut header_size_bytes)?;
        let header_size = u32::from_le_bytes(header_size_bytes) as usize;

        // Read game info JSON
        let mut info_buffer = vec![0u8; header_size];
        file.read_exact(&mut info_buffer)?;
        let game_info: GameInfo = serde_json::from_slice(&info_buffer)
            .map_err(|e| CacaoError::GameLoadError(format!("Failed to parse game info: {}", e)))?;

        Ok(game_info)
    }

    fn find_game_folder(&self, game_info: &GameInfo) -> Result<PathBuf, CacaoError> {
        let folder_name = sanitize_filename(&game_info.title);
        let game_folder = self.games_dir.join(&folder_name);

        if game_folder.exists() && game_folder.is_dir() {
            Ok(game_folder)
        } else {
            Err(CacaoError::GameLoadError(format!(
                "Game folder not found: {}",
                folder_name
            )))
        }
    }

    fn verify_asset(&self, asset_path: &Path, asset_info: &AssetInfo) -> Result<(), CacaoError> {
        use sha2::{Digest, Sha256};

        let mut file = File::open(asset_path).map_err(|_| {
            CacaoError::GameLoadError(format!("Asset not found: {}", asset_path.display()))
        })?;

        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)?;
        let computed_checksum = format!("{:x}", hasher.finalize());

        if computed_checksum != asset_info.checksum {
            return Err(CacaoError::GameLoadError(format!(
                "Asset checksum mismatch: {}",
                asset_path.display()
            )));
        }

        Ok(())
    }

    pub fn discover_games(&self) -> Result<Vec<PathBuf>, CacaoError> {
        let mut games = Vec::new();

        for entry in std::fs::read_dir(&self.games_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("gaem") {
                games.push(path);
            }
        }

        Ok(games)
    }
}

pub fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
            ' ' => '_',
            _ => '_',
        })
        .collect()
}
