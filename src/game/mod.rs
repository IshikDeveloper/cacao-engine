// ============================================================================
// FILE: src/game/mod.rs - Module Exports
// ============================================================================
pub mod loader;
pub mod info;
pub mod runtime;

pub use loader::GameLoader;
pub use info::{GameInfo, AssetInfo, AssetType, GAEM_MAGIC, GAEM_VERSION};
pub use runtime::Game;