// src/assets/mod.rs
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use crate::{
    errors::CacaoError,
    renderer::{Texture, Sprite},
    game::AssetType,
};

pub struct AssetManager {
    sprites: HashMap<String, Arc<Sprite>>,
    textures: HashMap<String, Arc<Texture>>,
    audio_clips: HashMap<String, Arc<AudioClip>>,
    scripts: HashMap<String, String>,
    fonts: HashMap<String, Arc<Font>>,
    data_files: HashMap<String, Vec<u8>>,
    
    // Asset loading state
    loading_tasks: Vec<tokio::task::JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub struct AudioClip {
    pub data: Vec<u8>,
    pub format: AudioFormat,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone)]
pub enum AudioFormat {
    Wav,
    Ogg,
    Mp3,
}

#[derive(Debug, Clone)]
pub struct Font {
    pub data: Vec<u8>,
    pub name: String,
    pub size: f32,
}

impl AssetManager {
    pub fn new() -> Self {
        Self {
            sprites: HashMap::new(),
            textures: HashMap::new(),
            audio_clips: HashMap::new(),
            scripts: HashMap::new(),
            fonts: HashMap::new(),
            data_files: HashMap::new(),
            loading_tasks: Vec::new(),
        }
    }

    pub async fn load_asset(&mut self, path: &Path, asset_type: AssetType) -> Result<(), CacaoError> {
        let path_str = path.to_string_lossy().to_string();
        let file_name = path.file_name()
            .ok_or_else(|| CacaoError::IoError(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid file path")))?
            .to_string_lossy()
            .to_string();

        match asset_type {
            AssetType::Sprite => {
                let texture = self.load_texture_from_file(path).await?;
                let sprite = Arc::new(Sprite::new(texture));
                self.sprites.insert(file_name.clone(), sprite);
                log::info!("Loaded sprite: {}", file_name);
            }
            AssetType::Audio => {
                let audio_clip = self.load_audio_from_file(path).await?;
                self.audio_clips.insert(file_name.clone(), Arc::new(audio_clip));
                log::info!("Loaded audio: {}", file_name);
            }
            AssetType::Script => {
                let script_content = tokio::fs::read_to_string(path).await?;
                self.scripts.insert(file_name.clone(), script_content);
                log::info!("Loaded script: {}", file_name);
            }
            AssetType::Font => {
                let font = self.load_font_from_file(path).await?;
                self.fonts.insert(file_name.clone(), Arc::new(font));
                log::info!("Loaded font: {}", file_name);
            }
            AssetType::Data => {
                let data = tokio::fs::read(path).await?;
                self.data_files.insert(file_name.clone(), data);
                log::info!("Loaded data file: {}", file_name);
            }
        }

        Ok(())
    }

    async fn load_texture_from_file(&self, path: &Path) -> Result<Texture, CacaoError> {
        let bytes = tokio::fs::read(path).await?;
        
        // We need access to the GPU device here
        // For now, we'll return an error and implement this properly when we have renderer context
        Err(CacaoError::RenderError("Texture loading requires renderer context".to_string()))
    }

    async fn load_audio_from_file(&self, path: &Path) -> Result<AudioClip, CacaoError> {
        let bytes = tokio::fs::read(path).await?;
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        let format = match extension.as_str() {
            "wav" => AudioFormat::Wav,
            "ogg" => AudioFormat::Ogg,
            "mp3" => AudioFormat::Mp3,
            _ => return Err(CacaoError::AudioError(format!("Unsupported audio format: {}", extension))),
        };

        // Basic WAV parsing for now
        let (sample_rate, channels) = if matches!(format, AudioFormat::Wav) {
            parse_wav_header(&bytes)?
        } else {
            (44100, 2) // Default values for other formats
        };

        Ok(AudioClip {
            data: bytes,
            format,
            sample_rate,
            channels,
        })
    }

    async fn load_font_from_file(&self, path: &Path) -> Result<Font, CacaoError> {
        let bytes = tokio::fs::read(path).await?;
        let name = path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(Font {
            data: bytes,
            name,
            size: 16.0, // Default size
        })
    }

    // Asset getters
    pub fn get_sprite(&self, name: &str) -> Option<Arc<Sprite>> {
        self.sprites.get(name).cloned()
    }

    pub fn get_texture(&self, name: &str) -> Option<Arc<Texture>> {
        self.textures.get(name).cloned()
    }

    pub fn get_audio_clip(&self, name: &str) -> Option<Arc<AudioClip>> {
        self.audio_clips.get(name).cloned()
    }

    pub fn get_script(&self, name: &str) -> Option<&String> {
        self.scripts.get(name)
    }

    pub fn get_font(&self, name: &str) -> Option<Arc<Font>> {
        self.fonts.get(name).cloned()
    }

    pub fn get_data_file(&self, name: &str) -> Option<&Vec<u8>> {
        self.data_files.get(name)
    }

    pub fn list_assets(&self) -> AssetListing {
        AssetListing {
            sprites: self.sprites.keys().cloned().collect(),
            textures: self.textures.keys().cloned().collect(),
            audio_clips: self.audio_clips.keys().cloned().collect(),
            scripts: self.scripts.keys().cloned().collect(),
            fonts: self.fonts.keys().cloned().collect(),
            data_files: self.data_files.keys().cloned().collect(),
        }
    }

    pub fn clear_assets(&mut self) {
        self.sprites.clear();
        self.textures.clear();
        self.audio_clips.clear();
        self.scripts.clear();
        self.fonts.clear();
        self.data_files.clear();
        log::info!("Cleared all assets");
    }

    pub fn get_memory_usage(&self) -> AssetMemoryInfo {
        let mut sprite_memory = 0;
        let mut texture_memory = 0;
        let mut audio_memory = 0;
        let mut script_memory = 0;
        let mut font_memory = 0;
        let mut data_memory = 0;

        // Calculate approximate memory usage
        for sprite in self.sprites.values() {
            sprite_memory += (sprite.width * sprite.height * 4.0) as usize; // RGBA
        }

        for texture in self.textures.values() {
            texture_memory += (texture.width * texture.height * 4) as usize; // RGBA
        }

        for audio in self.audio_clips.values() {
            audio_memory += audio.data.len();
        }

        for script in self.scripts.values() {
            script_memory += script.len();
        }

        for font in self.fonts.values() {
            font_memory += font.data.len();
        }

        for data in self.data_files.values() {
            data_memory += data.len();
        }

        AssetMemoryInfo {
            sprite_memory,
            texture_memory,
            audio_memory,
            script_memory,
            font_memory,
            data_memory,
            total_memory: sprite_memory + texture_memory + audio_memory + script_memory + font_memory + data_memory,
        }
    }
}

#[derive(Debug)]
pub struct AssetListing {
    pub sprites: Vec<String>,
    pub textures: Vec<String>,
    pub audio_clips: Vec<String>,
    pub scripts: Vec<String>,
    pub fonts: Vec<String>,
    pub data_files: Vec<String>,
}

#[derive(Debug)]
pub struct AssetMemoryInfo {
    pub sprite_memory: usize,
    pub texture_memory: usize,
    pub audio_memory: usize,
    pub script_memory: usize,
    pub font_memory: usize,
    pub data_memory: usize,
    pub total_memory: usize,
}

// Helper function to parse WAV file headers
fn parse_wav_header(data: &[u8]) -> Result<(u32, u16), CacaoError> {
    if data.len() < 44 {
        return Err(CacaoError::AudioError("Invalid WAV file: too short".to_string()));
    }

    // Check RIFF header
    if &data[0..4] != b"RIFF" {
        return Err(CacaoError::AudioError("Invalid WAV file: missing RIFF header".to_string()));
    }

    // Check WAVE format
    if &data[8..12] != b"WAVE" {
        return Err(CacaoError::AudioError("Invalid WAV file: not WAVE format".to_string()));
    }

    // Extract sample rate (bytes 24-27)
    let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
    
    // Extract number of channels (bytes 22-23)
    let channels = u16::from_le_bytes([data[22], data[23]]);

    Ok((sample_rate, channels))
}

// Asset preloading and hot-reloading functionality
impl AssetManager {
    pub async fn preload_directory(&mut self, dir_path: &Path) -> Result<(), CacaoError> {
        let mut entries = tokio::fs::read_dir(dir_path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.is_file() {
                if let Some(asset_type) = determine_asset_type(&path) {
                    if let Err(e) = self.load_asset(&path, asset_type).await {
                        log::warn!("Failed to preload asset {}: {}", path.display(), e);
                    }
                }
            }
        }
        
        Ok(())
    }

    pub fn enable_hot_reloading(&mut self, watch_directory: PathBuf) -> Result<(), CacaoError> {
        // TODO: Implement file system watching for hot-reloading
        // This would use a library like `notify` to watch for file changes
        log::info!("Hot reloading enabled for directory: {}", watch_directory.display());
        Ok(())
    }
}

fn determine_asset_type(path: &Path) -> Option<AssetType> {
    let extension = path.extension()?.to_str()?.to_lowercase();
    
    match extension.as_str() {
        "png" | "jpg" | "jpeg" | "bmp" | "tga" | "gif" => Some(AssetType::Sprite),
        "wav" | "ogg" | "mp3" | "flac" => Some(AssetType::Audio),
        "lua" | "js" | "py" => Some(AssetType::Script),
        "ttf" | "otf" | "woff" | "woff2" => Some(AssetType::Font),
        "json" | "xml" | "yaml" | "toml" | "csv" => Some(AssetType::Data),
        _ => None,
    }
}