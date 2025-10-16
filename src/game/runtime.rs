// ============================================================================
// FILE: src/game/runtime.rs - Enhanced with Better Error Handling
// ============================================================================
use std::path::PathBuf;
use std::time::Duration;
use mlua::{Lua, Function};
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
    _game_folder: PathBuf,
    lua: Lua,
    _secret_key: String,
    initialized: bool,
}

impl Game {
    pub fn new(info: GameInfo, game_folder: PathBuf) -> Self {
        let lua = Lua::new();
        
        Self {
            info,
            _game_folder: game_folder,
            lua,
            _secret_key: String::new(),
            initialized: false,
        }
    }

    pub fn initialize(&mut self, secret_key: String) -> Result<(), CacaoError> {
        if !self.info.verify_secret_key(&secret_key) {
            return Err(CacaoError::GameLoadError("Invalid secret key".to_string()));
        }
        
        self._secret_key = secret_key;
        self.setup_lua_api()?;
        
        let main_script_path = self._game_folder.join(&self.info.entry_point);
        let script_content = std::fs::read_to_string(&main_script_path)?;
        
        self.lua.load(&script_content).exec()
            .map_err(|e| CacaoError::ScriptError(format!("Failed to load main script: {}", e)))?;
        
        if let Ok(init_fn) = self.lua.globals().get::<_, Function>("init") {
            init_fn.call::<_, ()>(())
                .map_err(|e| CacaoError::ScriptError(format!("Init function failed: {}", e)))?;
        }
        
        self.initialized = true;
        Ok(())
    }

    pub fn update(&mut self, delta_time: Duration, _input: &mut InputManager, _audio: &mut AudioSystem, _saves: &mut SaveManager) {
        if !self.initialized {
            return;
        }

        if let Ok(update_fn) = self.lua.globals().get::<_, Function>("update") {
            let dt = delta_time.as_secs_f32();
            if let Err(e) = update_fn.call::<_, ()>(dt) {
                log::error!("Update function error: {}", e);
            }
        }
    }

    pub fn render(&self, _renderer: &mut Renderer) -> Result<(), CacaoError> {
        if !self.initialized {
            return Ok(());
        }

        if let Ok(render_fn) = self.lua.globals().get::<_, Function>("render") {
            render_fn.call::<_, ()>(())
                .map_err(|e| CacaoError::ScriptError(format!("Render function failed: {}", e)))?;
        }
        
        Ok(())
    }

    fn setup_lua_api(&self) -> Result<(), CacaoError> {
        let globals = self.lua.globals();
        let cacao_table = self.lua.create_table()?;
        globals.set("cacao", cacao_table)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_info(&self) -> &GameInfo {
        &self.info
    }
}
