// src/main.rs
use log::info;

mod engine;
mod game;
mod renderer;
mod audio;
mod input;
mod assets;
mod crypto;
mod saves;
mod errors;

use engine::CacaoEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info!("Starting Cacao Engine...");

    let engine = CacaoEngine::new().await?;
    
    // run() never returns, so we never reach this point
    engine.run().await;
}