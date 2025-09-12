// src/engine/mod.rs
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::{
    renderer::Renderer,
    audio::AudioSystem,
    input::InputManager,
    assets::AssetManager,
    game::{Game, GameLoader},
    saves::SaveManager,
    errors::CacaoError,
};

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
    
    // Directories
    games_dir: PathBuf,
    saves_dir: PathBuf,
    
    // Timing
    last_frame: Instant,
    target_fps: u32,
}

impl CacaoEngine {
    pub async fn new() -> Result<Self, CacaoError> {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title("Cacao Engine")
            .with_inner_size(winit::dpi::LogicalSize::new(1024, 768))
            .build(&event_loop)
            .map_err(|e| CacaoError::RenderError(format!("Window creation failed: {}", e)))?;

        let renderer = Renderer::new(&window).await?;
        let audio = AudioSystem::new()?;
        let input = InputManager::new();
        
        // Setup directories
        let games_dir = std::env::current_dir()?.join("games");
        let saves_dir = std::env::current_dir()?.join("saves");
        
        std::fs::create_dir_all(&games_dir)?;
        std::fs::create_dir_all(&saves_dir)?;
        
        let assets = AssetManager::new();
        let saves = SaveManager::new(saves_dir.clone());
        let game_loader = GameLoader::new(games_dir.clone());

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
            games_dir,
            saves_dir,
            last_frame: Instant::now(),
            target_fps: 60,
        })
    }

    pub async fn run(mut self) -> Result<(), CacaoError> {
        let event_loop = self.event_loop.take().unwrap();
        let target_frame_time = Duration::from_millis(1000 / self.target_fps as u64);

        event_loop.run(move |event, _, control_flow| {
            match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == self.window.id() => {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            self.renderer.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            self.renderer.resize(**new_inner_size);
                        }
                        _ => {
                            self.input.handle_window_event(event);
                        }
                    }
                }
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
            }
        })
    }

    fn update(&mut self, delta_time: Duration) {
        self.input.update();
        
        if let Some(ref mut game) = self.current_game {
            game.update(delta_time, &mut self.input, &mut self.audio, &mut self.saves);
        }
    }

    fn render(&mut self) -> Result<(), CacaoError> {
        self.renderer.begin_frame()?;
        
        if let Some(ref game) = self.current_game {
            game.render(&mut self.renderer)?;
        } else {
            // Render game selection screen
            self.render_game_browser()?;
        }
        
        self.renderer.end_frame()?;
        Ok(())
    }

    fn render_game_browser(&mut self) -> Result<(), CacaoError> {
        // TODO: Implement game browser UI
        self.renderer.clear_screen([0.1, 0.1, 0.2, 1.0]);
        Ok(())
    }

    pub async fn load_game(&mut self, game_path: &Path) -> Result<(), CacaoError> {
        let game = self.game_loader.load_game(game_path, &mut self.assets).await?;
        self.current_game = Some(game);
        Ok(())
    }
}