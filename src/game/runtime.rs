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