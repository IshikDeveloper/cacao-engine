use std::fmt;
use std::fs::File;
use std::io::Read;
use mlua::{Lua, LuaError};

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

// --------------------
// Application Logic
// --------------------

fn read_file(path: &str) -> Result<String, CacaoError> {
    let mut file = File::open(path)?; // io::Error -> CacaoError
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

fn run_lua_script(script: &str) -> Result<(), CacaoError> {
    let lua = Lua::new();
    lua.load(script).exec()?; // LuaError -> CacaoError
    Ok(())
}

fn main() -> Result<(), CacaoError> {
    // Example: Reading a file
    match read_file("example.txt") {
        Ok(contents) => println!("File contents: {}", contents),
        Err(e) => println!("Failed to read file: {}", e),
    }

    // Example: Running Lua script
    let script = r#"
        print("Hello from Lua!")
    "#;
    run_lua_script(script)?;

    Ok(())
}
