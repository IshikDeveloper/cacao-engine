// src/engine/mod.rs - Enhanced with proper game browser
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::{
    assets::AssetManager,
    audio::AudioSystem,
    errors::CacaoError,
    game::{Game, GameInfo, GameLoader},
    input::InputManager,
    renderer::Renderer,
    saves::SaveManager,
};

#[derive(Debug, Clone)]
struct GameEntry {
    info: GameInfo,
    file_path: PathBuf,
}

enum EngineState {
    Browser {
        games: Vec<GameEntry>,
        selected_index: usize,
    },
    Playing,
    Loading {
        progress: f32,
    },
}

pub struct CacaoEngine {
    event_loop: Option<EventLoop<()>>,
    window: Window,
    renderer: Renderer,
    audio: AudioSystem,
    input: InputManager,
    assets: AssetManager,
    saves: SaveManager,
    game_loader: GameLoader,
    current_game: Option<Game>,

    state: EngineState,
    games_dir: PathBuf,
    saves_dir: PathBuf,

    last_frame: Instant,
    target_fps: u32,
}

impl CacaoEngine {
    pub async fn new() -> Result<Self, CacaoError> {
        env_logger::init();
        log::info!("Initializing Cacao Engine...");

        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title("Cacao Engine - Game Browser")
            .with_inner_size(winit::dpi::LogicalSize::new(1024, 768))
            .build(&event_loop)
            .map_err(|e| CacaoError::RenderError(format!("Window creation failed: {}", e)))?;

        let renderer = Renderer::new(&window).await?;
        let audio = AudioSystem::new()?;
        let input = InputManager::new();

        let games_dir = std::env::current_dir()?.join("games");
        let saves_dir = std::env::current_dir()?.join("saves");

        // Create directories if they don't exist
        std::fs::create_dir_all(&games_dir)?;
        std::fs::create_dir_all(&saves_dir)?;

        log::info!("Games directory: {}", games_dir.display());
        log::info!("Saves directory: {}", saves_dir.display());

        let assets = AssetManager::new();
        let saves = SaveManager::new(saves_dir.clone());
        let game_loader = GameLoader::new(games_dir.clone());

        // Discover available games
        let games = Self::discover_games(&game_loader)?;
        log::info!("Found {} games", games.len());

        let state = EngineState::Browser {
            games: games.clone(),
            selected_index: 0,
        };

        Ok(Self {
            event_loop: Some(event_loop),
            window,
            renderer,
            audio,
            input,
            assets,
            saves,
            game_loader,
            current_game: None,
            state,
            games_dir,
            saves_dir,
            last_frame: Instant::now(),
            target_fps: 60,
        })
    }

    fn discover_games(loader: &GameLoader) -> Result<Vec<GameEntry>, CacaoError> {
        let game_files = loader.discover_games()?;
        let mut entries = Vec::new();

        for path in game_files {
            match loader.parse_gaem_file_engine(&path) {
                Ok(info) => {
                    log::info!("Found game: {} by {}", info.title, info.author);
                    entries.push(GameEntry {
                        info,
                        file_path: path,
                    });
                }
                Err(e) => {
                    log::warn!("Failed to parse game file {:?}: {}", path, e);
                }
            }
        }

        Ok(entries)
    }

    pub async fn run(mut self) -> Result<(), CacaoError> {
        let event_loop = self.event_loop.take().unwrap();
        let target_frame_time = Duration::from_millis(1000 / self.target_fps as u64);

        event_loop.run(move |event, _, control_flow| match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == self.window.id() => match event {
                WindowEvent::CloseRequested => {
                    log::info!("Window close requested");
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::Resized(physical_size) => {
                    self.renderer.resize(*physical_size);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    self.renderer.resize(**new_inner_size);
                }
                _ => {
                    self.input.handle_window_event(event);
                }
            },
            Event::RedrawRequested(window_id) if window_id == self.window.id() => {
                let now = Instant::now();
                let delta_time = now.duration_since(self.last_frame);

                if delta_time >= target_frame_time {
                    self.update(delta_time);
                    match self.render() {
                        Ok(_) => {}
                        Err(e) => {
                            log::error!("Render error: {}", e);
                        }
                    }
                    self.last_frame = now;
                }
            }
            Event::MainEventsCleared => {
                self.window.request_redraw();
            }
            _ => {}
        })
    }

    fn update(&mut self, delta_time: Duration) {
        self.input.update();

        // Take out the state so we don't double-borrow `self`
        let state = std::mem::replace(
            &mut self.state,
            EngineState::Browser {
                games: vec![],
                selected_index: 0,
            },
        );

        self.state = match state {
            EngineState::Browser {
                mut games,
                mut selected_index,
            } => {
                // Now we can safely call methods on `self`
                self.update_browser(&mut games, &mut selected_index);
                EngineState::Browser {
                    games,
                    selected_index,
                }
            }

            EngineState::Playing => {
                if let Some(ref mut game) = self.current_game {
                    game.update(
                        delta_time,
                        &mut self.input,
                        &mut self.audio,
                        &mut self.saves,
                    );
                }

                // Check for escape to return to browser
                if self
                    .input
                    .is_key_just_pressed(winit::event::VirtualKeyCode::Escape)
                {
                    self.unload_game();
                }

                EngineState::Playing
            }

            EngineState::Loading { mut progress } => {
                // Simulate loading progress
                progress += delta_time.as_secs_f32() * 0.5;
                if progress >= 1.0 {
                    EngineState::Playing
                } else {
                    EngineState::Loading { progress }
                }
            }

            other => other, // fallback for any extra states
        };
    }

    fn update_browser(&mut self, games: &[GameEntry], selected_index: &mut usize) {
        use winit::event::VirtualKeyCode;

        if games.is_empty() {
            return;
        }

        // Navigate with arrow keys
        if self.input.is_key_just_pressed(VirtualKeyCode::Up) {
            if *selected_index > 0 {
                *selected_index -= 1;
            }
        }

        if self.input.is_key_just_pressed(VirtualKeyCode::Down) {
            if *selected_index < games.len() - 1 {
                *selected_index += 1;
            }
        }

        // Load game with Enter
        if self.input.is_key_just_pressed(VirtualKeyCode::Return) {
            let game_entry = &games[*selected_index];
            log::info!("Loading game: {}", game_entry.info.title);

            // Start async load
            let game_path = game_entry.file_path.clone();
            if let Err(e) = self.start_loading_game(&game_path) {
                log::error!("Failed to start loading game: {}", e);
            }
        }
    }

    fn update_state(&mut self) {
        use std::mem;

        let state = std::mem::replace(
            &mut self.state,
            EngineState::Browser {
                games: vec![],
                selected_index: 0,
            },
        );

        self.state = match state {
            EngineState::Browser {
                mut games,
                mut selected_index,
            } => {
                self.update_browser(&mut games, &mut selected_index);
                EngineState::Browser {
                    games,
                    selected_index,
                }
            }
            other => other,
        };
    }

    fn start_loading_game(&mut self, game_path: &Path) -> Result<(), CacaoError> {
        self.state = EngineState::Loading { progress: 0.0 };

        // In a real implementation, this would be async
        // For now, we'll do synchronous loading
        pollster::block_on(self.load_game_internal(game_path))?;

        Ok(())
    }

    async fn load_game_internal(&mut self, game_path: &Path) -> Result<(), CacaoError> {
        let device = self.renderer.get_device();
        let queue = self.renderer.get_queue();

        let mut game = self
            .game_loader
            .load_game(game_path, &mut self.assets, device, queue)
            .await?;

        // Initialize game with a default secret key for now
        // In production, this should come from secure storage or user input
        let secret_key = "default_key".to_string();
        game.initialize(secret_key)?;

        self.current_game = Some(game);
        self.state = EngineState::Playing;

        Ok(())
    }

    fn unload_game(&mut self) {
        log::info!("Unloading game...");
        self.current_game = None;
        self.assets.clear_assets();

        // Return to browser
        let games = Self::discover_games(&self.game_loader).unwrap_or_default();
        self.state = EngineState::Browser {
            games,
            selected_index: 0,
        };

        self.window.set_title("Cacao Engine - Game Browser");
    }

    fn render(&mut self) -> Result<(), CacaoError> {
        self.renderer.begin_frame()?;

        // Take ownership of state temporarily
        let state = std::mem::replace(
            &mut self.state,
            EngineState::Browser {
                games: vec![],
                selected_index: 0,
            }, // temporary placeholder
        );

        // Handle rendering
        match &state {
            EngineState::Browser {
                games,
                selected_index,
            } => {
                self.render_game_browser(games, *selected_index)?;
            }
            EngineState::Playing => {
                if let Some(ref game) = self.current_game {
                    game.render(&mut self.renderer)?;
                }
            }
            EngineState::Loading { progress } => {
                self.render_loading_screen(*progress)?;
            }
        }

        // Put the state back
        self.state = state;

        self.renderer.end_frame()?;
        Ok(())
    }

    fn render_game_browser(
        &mut self,
        games: &[GameEntry],
        selected_index: usize,
    ) -> Result<(), CacaoError> {
        // Dark blue background
        self.renderer.clear_screen([0.1, 0.1, 0.2, 1.0]);

        // TODO: Render text for each game in the list
        // For now, we'll just have the background
        // In a full implementation, you'd use the text rendering system

        log::debug!(
            "Rendering browser with {} games, selected: {}",
            games.len(),
            selected_index
        );

        Ok(())
    }

    fn render_loading_screen(&mut self, progress: f32) -> Result<(), CacaoError> {
        // Darker background for loading
        self.renderer.clear_screen([0.05, 0.05, 0.1, 1.0]);

        // TODO: Render loading bar
        log::debug!("Loading progress: {:.1}%", progress * 100.0);

        Ok(())
    }

    pub async fn load_game(&mut self, game_path: &Path) -> Result<(), CacaoError> {
        self.load_game_internal(game_path).await
    }
}

// Make GameLoader methods public
impl crate::game::GameLoader {
    pub fn parse_gaem_file_engine(&self, file_path: &Path) -> Result<GameInfo, CacaoError> {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(file_path)?;

        // Read and verify magic bytes
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if magic != crate::game::GAEM_MAGIC {
            return Err(CacaoError::GameLoadError(
                "Invalid .gaem file format".to_string(),
            ));
        }

        // Read version
        let mut version_bytes = [0u8; 2];
        file.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);
        if version != crate::game::GAEM_VERSION {
            return Err(CacaoError::GameLoadError(format!(
                "Unsupported .gaem version: {}",
                version
            )));
        }

        // Read header size
        let mut header_size_bytes = [0u8; 4];
        file.read_exact(&mut header_size_bytes)?;
        let header_size = u32::from_le_bytes(header_size_bytes) as usize;

        // Read game info JSON
        let mut info_buffer = vec![0u8; header_size];
        file.read_exact(&mut info_buffer)?;
        let game_info: GameInfo = serde_json::from_slice(&info_buffer)
            .map_err(|e| CacaoError::GameLoadError(format!("Failed to parse game info: {}", e)))?;

        Ok(game_info)
    }
}
