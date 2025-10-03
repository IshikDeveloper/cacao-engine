// examples/create_demo_game.rs
// Run with: cargo run --example create_demo_game

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use serde_json;
use uuid::Uuid;
use sha2::{Sha256, Digest};

// Copy the structs from your game module
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct GameInfo {
    id: Uuid,
    title: String,
    author: String,
    version: String,
    description: String,
    secret_key_hash: String,
    entry_point: String,
    required_assets: Vec<AssetInfo>,
    engine_version: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct AssetInfo {
    path: String,
    checksum: String,
    size: u64,
    asset_type: AssetType,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum AssetType {
    Sprite,
    Audio,
    Script,
    Data,
    Font,
}

const GAEM_MAGIC: [u8; 4] = [0x47, 0x41, 0x45, 0x4D]; // "GAEM"
const GAEM_VERSION: u16 = 1;

fn create_demo_game() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating demo game...");

    // Create games directory
    let games_dir = Path::new("games");
    fs::create_dir_all(games_dir)?;

    // Create game folder
    let game_folder = games_dir.join("demo_game");
    fs::create_dir_all(&game_folder)?;

    // Create main.lua script
    let main_lua = r#"
-- Demo Game for Cacao Engine
print("Demo Game Loaded!")

-- Global variables
local player_x = 400
local player_y = 300
local player_speed = 200

function init()
    print("Game initialized!")
end

function update(dt)
    -- Simple movement (will be implemented when input is connected)
    -- For now, just log that we're updating
    -- Uncomment when input API is ready:
    -- if cacao.input.is_key_pressed("W") then player_y = player_y - player_speed * dt end
    -- if cacao.input.is_key_pressed("S") then player_y = player_y + player_speed * dt end
    -- if cacao.input.is_key_pressed("A") then player_x = player_x - player_speed * dt end
    -- if cacao.input.is_key_pressed("D") then player_x = player_x + player_speed * dt end
end

function render()
    -- Clear screen with a nice blue color
    -- cacao.renderer.clear({0.2, 0.3, 0.8, 1.0})
    
    -- Draw player (when sprite API is ready)
    -- cacao.renderer.draw_sprite("player", player_x, player_y)
end
"#;

    let script_path = game_folder.join("main.lua");
    fs::write(&script_path, main_lua)?;
    println!("Created main.lua");

    // Calculate checksum for the script
    let script_data = fs::read(&script_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&script_data);
    let script_checksum = format!("{:x}", hasher.finalize());

    // Create game info
    let mut game_info = GameInfo {
        id: Uuid::new_v4(),
        title: "Demo Game".to_string(),
        author: "Cacao Engine".to_string(),
        version: "1.0.0".to_string(),
        description: "A simple demo game to test the Cacao Engine".to_string(),
        secret_key_hash: String::new(),
        entry_point: "main.lua".to_string(),
        required_assets: vec![
            AssetInfo {
                path: "main.lua".to_string(),
                checksum: script_checksum,
                size: script_data.len() as u64,
                asset_type: AssetType::Script,
            }
        ],
        engine_version: "0.1.0".to_string(),
    };

    // Set secret key
    let secret_key = "default_key";
    let mut hasher = Sha256::new();
    hasher.update(secret_key.as_bytes());
    game_info.secret_key_hash = format!("{:x}", hasher.finalize());

    // Create .gaem file
    let gaem_path = games_dir.join("demo_game.gaem");
    let mut gaem_file = File::create(&gaem_path)?;

    // Write magic bytes
    gaem_file.write_all(&GAEM_MAGIC)?;

    // Write version
    gaem_file.write_all(&GAEM_VERSION.to_le_bytes())?;

    // Serialize game info
    let info_json = serde_json::to_vec(&game_info)?;
    let info_size = info_json.len() as u32;

    // Write header size
    gaem_file.write_all(&info_size.to_le_bytes())?;

    // Write game info
    gaem_file.write_all(&info_json)?;

    println!("Created demo_game.gaem");
    println!("\nGame Details:");
    println!("  Title: {}", game_info.title);
    println!("  Author: {}", game_info.author);
    println!("  Version: {}", game_info.version);
    println!("  ID: {}", game_info.id);
    println!("\nGame created successfully!");
    println!("\nTo play:");
    println!("  1. Run: cargo run");
    println!("  2. Use arrow keys to navigate");
    println!("  3. Press Enter to load the game");
    println!("  4. Press Escape to return to browser");

    Ok(())
}

fn main() {
    if let Err(e) = create_demo_game() {
        eprintln!("Error creating demo game: {}", e);
        std::process::exit(1);
    }
}