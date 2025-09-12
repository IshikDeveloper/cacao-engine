// src/main.rs
use std::path::PathBuf;
use log::info;

mod engine;
mod game;
mod renderer;
mod audio;
mod input;
mod assets;
mod crypto;
mod saves;

use engine::CacaoEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info!("Starting Cacao Engine...");

    let engine = CacaoEngine::new().await?;
    engine.run().await?;

    Ok(())
}