// src/game/mod.rs
pub mod loader;
pub mod info;
pub mod runtime;

pub use loader::GameLoader;
pub use info::{GameInfo, AssetInfo, AssetType, GAEM_MAGIC, GAEM_VERSION};
pub use runtime::Game;

use crate::{
    input::InputManager,
    audio::AudioSystem,
    saves::SaveManager,
    renderer::Renderer,
    errors::CacaoError,
};