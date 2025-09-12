// src/game/mod.rs
pub mod loader;
pub mod info;
pub mod runtime;

pub use loader::GameLoader;
pub use info::GameInfo;
pub use runtime::Game;

use std::time::Duration;
use crate::{
    input::InputManager,
    audio::AudioSystem,
    saves::SaveManager,
    renderer::Renderer,
    errors::CacaoError,
};

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

// src/game/loader.rs
use std::path::{Path, PathBuf};
use std::io::{Read, Seek, SeekFrom};
use std::fs::File;
use crate::{
    assets::AssetManager,
    errors::CacaoError,
};
use super::{GameInfo, Game, GAEM_MAGIC, GAEM_VERSION};

pub struct GameLoader {
    games_dir: PathBuf,
}

impl GameLoader {
    pub fn new(games_dir: PathBuf) -> Self {
        Self { games_dir }
    }

    pub async fn load_game(&self, game_file: &Path, assets: &mut AssetManager) -> Result<Game, CacaoError> {
        let game_info = self.parse_gaem_file(game_file)?;
        let game_folder = self.find_game_folder(&game_info)?;
        
        // Load all required assets
        for asset_info in &game_info.required_assets {
            let asset_path = game_folder.join(&asset_info.path);
            self.verify_asset(&asset_path, asset_info)?;
            assets.load_asset(&asset_path, asset_info.asset_type.clone()).await?;
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
            return Err(CacaoError::GameLoadError("Invalid .gaem file format".to_string()));
        }

        // Read version
        let mut version_bytes = [0u8; 2];
        file.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);
        if version != GAEM_VERSION {
            return Err(CacaoError::GameLoadError(format!("Unsupported .gaem version: {}", version)));
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
        // Look for a folder with the same name as the game title (sanitized)
        let folder_name = sanitize_filename(&game_info.title);
        let game_folder = self.games_dir.join(&folder_name);
        
        if game_folder.exists() && game_folder.is_dir() {
            Ok(game_folder)
        } else {
            Err(CacaoError::GameLoadError(format!("Game folder not found: {}", folder_name)))
        }
    }

    fn verify_asset(&self, asset_path: &Path, asset_info: &AssetInfo) -> Result<(), CacaoError> {
        use sha2::{Sha256, Digest};
        
        let mut file = File::open(asset_path)
            .map_err(|_| CacaoError::GameLoadError(format!("Asset not found: {}", asset_path.display())))?;
        
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)?;
        let computed_checksum = format!("{:x}", hasher.finalize());
        
        if computed_checksum != asset_info.checksum {
            return Err(CacaoError::GameLoadError(format!("Asset checksum mismatch: {}", asset_path.display())));
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

// src/game/runtime.rs
use std::path::PathBuf;
use std::time::Duration;
use mlua::{Lua, Table, Function};
use crate::{
    input::InputManager,
    audio::AudioSystem,
    saves::SaveManager,
    renderer::Renderer,
    errors::CacaoError,
};
use super::GameInfo;

pub struct Game {
    info: GameInfo,
    game_folder: PathBuf,
    lua: Lua,
    secret_key: String,
    initialized: bool,
}

impl Game {
    pub fn new(info: GameInfo, game_folder: PathBuf) -> Self {
        let lua = Lua::new();
        
        Self {
            info,
            game_folder,
            lua,
            secret_key: String::new(), // Will be loaded from secure storage
            initialized: false,
        }
    }

    pub fn initialize(&mut self, secret_key: String) -> Result<(), CacaoError> {
        // Verify secret key
        if !self.info.verify_secret_key(&secret_key) {
            return Err(CacaoError::GameLoadError("Invalid secret key".to_string()));
        }
        
        self.secret_key = secret_key;
        
        // Setup Lua environment
        self.setup_lua_api()?;
        
        // Load main script
        let main_script_path = self.game_folder.join(&self.info.entry_point);
        let script_content = std::fs::read_to_string(&main_script_path)?;
        
        self.lua.load(&script_content).exec()
            .map_err(|e| CacaoError::ScriptError(format!("Failed to load main script: {}", e)))?;
        
        // Call init function if it exists
        if let Ok(init_fn) = self.lua.globals().get::<_, Function>("init") {
            init_fn.call::<_, ()>(())
                .map_err(|e| CacaoError::ScriptError(format!("Init function failed: {}", e)))?;
        }
        
        self.initialized = true;
        Ok(())
    }

    pub fn update(&mut self, delta_time: Duration, input: &mut InputManager, audio: &mut AudioSystem, saves: &mut SaveManager) {
        if !self.initialized {
            return;
        }

        // Call update function
        if let Ok(update_fn) = self.lua.globals().get::<_, Function>("update") {
            let dt = delta_time.as_secs_f32();
            if let Err(e) = update_fn.call::<_, ()>(dt) {
                log::error!("Update function error: {}", e);
            }
        }
    }

    pub fn render(&self, renderer: &mut Renderer) -> Result<(), CacaoError> {
        if !self.initialized {
            return Ok(());
        }

        // Call render function
        if let Ok(render_fn) = self.lua.globals().get::<_, Function>("render") {
            render_fn.call::<_, ()>(())
                .map_err(|e| CacaoError::ScriptError(format!("Render function failed: {}", e)))?;
        }
        
        Ok(())
    }

    fn setup_lua_api(&self) -> Result<(), CacaoError> {
        let globals = self.lua.globals();
        
        // Create API tables
        let cacao_table = self.lua.create_table()?;
        let renderer_table = self.lua.create_table()?;
        let input_table = self.lua.create_table()?;
        let audio_table = self.lua.create_table()?;
        let saves_table = self.lua.create_table()?;

        // Add API functions (simplified for now)
        renderer_table.set("clear", self.lua.create_function(|_, color: Table| {
            // TODO: Implement renderer.clear()
            Ok(())
        })?)?;

        renderer_table.set("draw_sprite", self.lua.create_function(|_, (sprite, x, y): (String, f32, f32)| {
            // TODO: Implement sprite drawing
            Ok(())
        })?)?;

        input_table.set("is_key_pressed", self.lua.create_function(|_, key: String| {
            // TODO: Implement key checking
            Ok(false)
        })?)?;

        audio_table.set("play_sound", self.lua.create_function(|_, sound: String| {
            // TODO: Implement sound playing
            Ok(())
        })?)?;

        saves_table.set("write", self.lua.create_function(|_, (key, value): (String, String)| {
            // TODO: Implement save writing
            Ok(())
        })?)?;

        saves_table.set("read", self.lua.create_function(|_, key: String| {
            // TODO: Implement save reading
            Ok(None::<String>)
        })?)?;

        cacao_table.set("renderer", renderer_table)?;
        cacao_table.set("input", input_table)?;
        cacao_table.set("audio", audio_table)?;
        cacao_table.set("saves", saves_table)?;

        globals.set("cacao", cacao_table)?;
        
        Ok(())
    }

    pub fn get_info(&self) -> &GameInfo {
        &self.info
    }
}