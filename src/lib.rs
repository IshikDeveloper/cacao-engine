// ============================================================================
// FILE: src/lib.rs - Library Root
// ============================================================================
pub mod engine;
pub mod game;
pub mod renderer;
pub mod audio;
pub mod input;
pub mod assets;
pub mod crypto;
pub mod saves;
pub mod errors;

pub use engine::CacaoEngine;
pub use game::{Game, GameInfo};
pub use errors::CacaoError;