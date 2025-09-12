// src/errors.rs
use std::fmt;
use mlua::prelude::LuaError;

#[derive(Debug)]
pub enum CacaoError {
    IoError(std::io::Error),
    RenderError(String),
    GameLoadError(String),
    CryptoError(String),
    AudioError(String),
    ScriptError(String),
    LuaError(LuaError),
}

impl fmt::Display for CacaoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CacaoError::IoError(err) => write!(f, "IO Error: {}", err),
            CacaoError::RenderError(msg) => write!(f, "Render Error: {}", msg),
            CacaoError::GameLoadError(msg) => write!(f, "Game Load Error: {}", msg),
            CacaoError::CryptoError(msg) => write!(f, "Crypto Error: {}", msg),
            CacaoError::AudioError(msg) => write!(f, "Audio Error: {}", msg),
            CacaoError::ScriptError(msg) => write!(f, "Script Error: {}", msg),
            CacaoError::LuaError(err) => write!(f, "Lua Error: {}", err),
        }
    }
}

impl std::error::Error for CacaoError {}

impl From<std::io::Error> for CacaoError {
    fn from(err: std::io::Error) -> Self {
        CacaoError::IoError(err)
    }
}

impl From<LuaError> for CacaoError {
    fn from(err: LuaError) -> Self {
        CacaoError::LuaError(err)
    }
}