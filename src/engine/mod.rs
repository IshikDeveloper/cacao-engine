// ============================================================================
// FILE: src/engine/mod.rs - Stunning Main Menu UI (FIXED & ENHANCED)
// ============================================================================
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use winit::{
    event::{Event, WindowEvent, VirtualKeyCode},
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

#[derive(Debug, Clone, PartialEq)]
enum Theme {
    Animated,    // Your gorgeous animated theme
    Dark,        // Minimalist dark mode
    Wii,         // Nostalgic Wii theme
}

impl Theme {
    fn name(&self) -> &str {
        match self {
            Theme::Animated => "Animated Dreams",
            Theme::Dark => "Dark Minimalist",
            Theme::Wii => "Wii Classic",
        }
    }

    // FIX: Helper to get all themes for selector
    fn all() -> [Theme; 3] {
        [Theme::Animated, Theme::Dark, Theme::Wii]
    }

    fn from_index(index: usize) -> Theme {
        Self::all().get(index).cloned().unwrap_or(Theme::Animated)
    }
    // END FIX

    fn background_color(&self) -> [f32; 4] {
        match self {
            Theme::Animated => [0.05, 0.02, 0.15, 1.0],
            Theme::Dark => [0.08, 0.08, 0.08, 1.0],
            Theme::Wii => [0.95, 0.95, 0.95, 1.0], // White/light gray
        }
    }

    fn accent_color(&self) -> [f32; 4] {
        match self {
            Theme::Animated => [1.0, 0.6, 0.2, 1.0], // Orange
            Theme::Dark => [0.3, 0.7, 1.0, 1.0],     // Blue
            Theme::Wii => [0.4, 0.7, 1.0, 1.0],      // Wii blue
        }
    }

    fn text_color(&self) -> [f32; 4] {
        match self {
            Theme::Animated => [0.9, 0.9, 0.9, 1.0],
            Theme::Dark => [0.95, 0.95, 0.95, 1.0],
            Theme::Wii => [0.2, 0.2, 0.2, 1.0], // Dark gray for readability
        }
    }

    fn secondary_text_color(&self) -> [f32; 4] {
        match self {
            Theme::Animated => [0.7, 0.7, 0.8, 1.0],
            Theme::Dark => [0.6, 0.6, 0.6, 1.0],
            Theme::Wii => [0.4, 0.4, 0.4, 1.0],
        }
    }

    fn card_color(&self) -> [f32; 4] {
        match self {
            Theme::Animated => [0.15, 0.12, 0.20, 0.7],
            Theme::Dark => [0.12, 0.12, 0.12, 0.9],
            Theme::Wii => [1.0, 1.0, 1.0, 0.95], // White cards
        }
    }

    fn selected_card_color(&self) -> [f32; 4] {
        match self {
            Theme::Animated => [0.25, 0.20, 0.35, 0.9],
            Theme::Dark => [0.18, 0.18, 0.22, 1.0],
            Theme::Wii => [0.85, 0.92, 1.0, 1.0], // Light blue
        }
    }

    fn should_show_particles(&self) -> bool {
        matches!(self, Theme::Animated)
    }

    fn font_name(&self) -> &str {
        match self {
            Theme::Animated => "PressStart2P", // Retro gaming font
            Theme::Dark => "Roboto",            // Modern clean font
            Theme::Wii => "RodinNTLG",         // Wii system font
        }
    }
}

#[derive(Debug, Clone)]
struct GameEntry {
    info: GameInfo,
    file_path: PathBuf,
    banner_loaded: bool,
}

#[derive(Debug, Clone)]
enum MenuState {
    MainMenu,
    GameList,
    GameDetails(usize),
    Settings,
    ThemeSelector,
    About,
}

enum EngineState {
    Menu {
        state: MenuState,
        games: Vec<GameEntry>,
        selected_index: usize,
        scroll_offset: f32,
        transition_progress: f32,
        particles: Vec<MenuParticle>,
        theme_selector_index: usize,
    },
    Playing,
    Loading {
        progress: f32,
        status: String,
    },
}

#[derive(Clone)]
struct MenuParticle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    size: f32,
    color: [f32; 4],
    lifetime: f32,
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
    _games_dir: PathBuf,
    _saves_dir: PathBuf,

    last_frame: Instant,
    target_fps: u32,
    frame_count: u64,
    
    menu_animation_time: f32,
    current_theme: Theme,
}

impl CacaoEngine {
    pub async fn new() -> Result<Self, CacaoError> {
        log::info!("🎮 Initializing Cacao Engine...");

        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title("Cacao Engine")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
            .build(&event_loop)
            .map_err(|e| CacaoError::RenderError(format!("Window creation failed: {}", e)))?;

        let renderer = Renderer::new(&window).await?;
        let audio = AudioSystem::new()?;
        let input = InputManager::new();

        let games_dir = std::env::current_dir()?.join("games");
        let saves_dir = std::env::current_dir()?.join("saves");

        std::fs::create_dir_all(&games_dir)?;
        std::fs::create_dir_all(&saves_dir)?;

        log::info!("📁 Games directory: {}", games_dir.display());
        log::info!("💾 Saves directory: {}", saves_dir.display());

        let assets = AssetManager::new();
        let saves = SaveManager::new(saves_dir.clone());
        let game_loader = GameLoader::new(games_dir.clone());

        let games = Self::discover_games(&game_loader)?;
        log::info!("🎯 Found {} games", games.len());

        // Generate particles for gorgeous background
        let particles = Self::generate_particles();

        let state = EngineState::Menu {
            state: MenuState::MainMenu,
            games: games.clone(),
            selected_index: 0,
            scroll_offset: 0.0,
            transition_progress: 0.0,
            particles,
            theme_selector_index: 0,
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
            _games_dir: games_dir,
            _saves_dir: saves_dir,
            last_frame: Instant::now(),
            target_fps: 60,
            frame_count: 0,
            menu_animation_time: 0.0,
            current_theme: Theme::Animated, // Start with animated theme
        })
    }

    fn generate_particles() -> Vec<MenuParticle> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..150).map(|_| {
            MenuParticle {
                x: rng.gen_range(0.0..1280.0),
                y: rng.gen_range(0.0..720.0),
                vx: rng.gen_range(-20.0..20.0),
                vy: rng.gen_range(-20.0..20.0),
                size: rng.gen_range(2.0..6.0),
                color: [
                    rng.gen_range(0.5..1.0),
                    rng.gen_range(0.3..0.7),
                    rng.gen_range(0.8..1.0),
                    rng.gen_range(0.3..0.7),
                ],
                lifetime: rng.gen_range(0.0..10.0),
            }
        }).collect()
    }

    fn discover_games(loader: &GameLoader) -> Result<Vec<GameEntry>, CacaoError> {
        log::info!("🔍 Searching for games...");
        let game_files = loader.discover_games()?;
        log::info!("📦 Found {} .gaem files", game_files.len());
        
        let mut entries = Vec::new();

        for path in game_files {
            match loader.parse_gaem_file_engine(&path) {
                Ok(info) => {
                    log::info!("✅ Found game: {} by {}", info.title, info.author);
                    entries.push(GameEntry {
                        info,
                        file_path: path,
                        banner_loaded: false,
                    });
                }
                Err(e) => {
                    log::warn!("❌ Failed to parse game file {:?}: {}", path, e);
                }
            }
        }

        log::info!("🎮 Successfully loaded {} games", entries.len());
        Ok(entries)
    }

    pub async fn run(mut self) -> ! {
        let event_loop = self.event_loop.take().unwrap();
        let target_frame_time = Duration::from_millis(1000 / self.target_fps as u64);

        event_loop.run(move |event, _, control_flow| {
            match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == self.window.id() => {
                    match event {
                        WindowEvent::CloseRequested => {
                            log::info!("👋 Goodbye!");
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
                    }
                }
                Event::RedrawRequested(window_id) if window_id == self.window.id() => {
                    let now = Instant::now();
                    let delta_time = now.duration_since(self.last_frame);

                    if delta_time >= target_frame_time {
                        self.update(delta_time);
                        // RENDER is the only thing that mutates the renderer, but not the engine state.
                        match self.render() { 
                            Ok(_) => {}
                            Err(e) => {
                                log::error!("❌ Render error: {}", e);
                            }
                        }
                        self.last_frame = now;
                        self.frame_count += 1;
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
        let dt = delta_time.as_secs_f32();
        self.menu_animation_time += dt;

        // Handle escape to return to menu
        let should_unload = matches!(self.state, EngineState::Playing) 
            && self.input.is_key_just_pressed(VirtualKeyCode::Escape);

        if should_unload {
            self.unload_game();
            return;
        }

        // Clone state temporarily to avoid borrow issues
        let needs_load_game = if let EngineState::Menu { state, games, selected_index, scroll_offset, transition_progress, particles, theme_selector_index } = &mut self.state {
            // Update particles only for animated theme
            if self.current_theme.should_show_particles() {
                for particle in particles.iter_mut() {
                    particle.x += particle.vx * dt;
                    particle.y += particle.vy * dt;
                    particle.lifetime += dt;

                    // Wrap around screen
                    if particle.x < 0.0 { particle.x = 1280.0; }
                    if particle.x > 1280.0 { particle.x = 0.0; }
                    if particle.y < 0.0 { particle.y = 720.0; }
                    if particle.y > 720.0 { particle.y = 0.0; }

                    // Pulse effect
                    let pulse = (particle.lifetime * 2.0).sin() * 0.3 + 0.7;
                    particle.color[3] = pulse * 0.5;
                }
            }

            // Smooth transition
            *transition_progress = (*transition_progress + dt * 3.0).min(1.0);

            let mut load_game_path: Option<PathBuf> = None;

            match state {
                MenuState::MainMenu => {
                    if self.input.is_key_just_pressed(VirtualKeyCode::Return) {
                        *state = MenuState::GameList;
                        *transition_progress = 0.0;
                    }
                    if self.input.is_key_just_pressed(VirtualKeyCode::S) {
                        *state = MenuState::Settings;
                        *transition_progress = 0.0;
                    }
                    if self.input.is_key_just_pressed(VirtualKeyCode::T) {
                        *state = MenuState::ThemeSelector;
                        *transition_progress = 0.0;
                    }
                    if self.input.is_key_just_pressed(VirtualKeyCode::A) {
                        *state = MenuState::About;
                        *transition_progress = 0.0;
                    }
                }
                MenuState::GameList => {
                    if !games.is_empty() {
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
                        if self.input.is_key_just_pressed(VirtualKeyCode::Return) {
                            *state = MenuState::GameDetails(*selected_index);
                            *transition_progress = 0.0;
                        }
                    }
                    if self.input.is_key_just_pressed(VirtualKeyCode::Escape) {
                        *state = MenuState::MainMenu;
                        *transition_progress = 0.0;
                    }

                    // Smooth scrolling
                    let target_scroll = (*selected_index as f32 * 120.0).max(0.0);
                    *scroll_offset += (target_scroll - *scroll_offset) * dt * 10.0;
                }
                MenuState::GameDetails(idx) => {
                    if self.input.is_key_just_pressed(VirtualKeyCode::Return) {
                        if let Some(game) = games.get(*idx) {
                            load_game_path = Some(game.file_path.clone());
                        }
                    }
                    if self.input.is_key_just_pressed(VirtualKeyCode::Escape) {
                        *state = MenuState::GameList;
                        *transition_progress = 0.0;
                    }
                }
                MenuState::ThemeSelector => {
                    // FIX: Use Theme::all().len() for dynamic theme count
                    let num_themes = Theme::all().len(); 
                    if self.input.is_key_just_pressed(VirtualKeyCode::Up) {
                        if *theme_selector_index > 0 {
                            *theme_selector_index -= 1;
                        }
                    }
                    if self.input.is_key_just_pressed(VirtualKeyCode::Down) {
                        if *theme_selector_index < num_themes - 1 {
                            *theme_selector_index += 1;
                        }
                    }
                    if self.input.is_key_just_pressed(VirtualKeyCode::Return) {
                        // FIX: Use Theme::from_index helper
                        self.current_theme = Theme::from_index(*theme_selector_index);
                        log::info!("🎨 Theme changed to: {}", self.current_theme.name());
                        *state = MenuState::MainMenu;
                        *transition_progress = 0.0;
                    }
                    if self.input.is_key_just_pressed(VirtualKeyCode::Escape) {
                        *state = MenuState::MainMenu;
                        *transition_progress = 0.0;
                    }
                }
                MenuState::Settings => {
                    if self.input.is_key_just_pressed(VirtualKeyCode::Escape) {
                        *state = MenuState::MainMenu;
                        *transition_progress = 0.0;
                    }
                }
                MenuState::About => {
                    if self.input.is_key_just_pressed(VirtualKeyCode::Escape) {
                        *state = MenuState::MainMenu;
                        *transition_progress = 0.0;
                    }
                }
            }

            load_game_path
        } else {
            None
        };

        // Handle game loading outside the borrow
        if let Some(game_path) = needs_load_game {
            if let Err(e) = self.start_loading_game(&game_path) {
                log::error!("❌ Failed to load game: {}", e);
            }
        }

        match &mut self.state {
            EngineState::Playing => {
                if let Some(ref mut game) = self.current_game {
                    game.update(delta_time, &mut self.input, &mut self.audio, &mut self.saves);
                }
            }
            EngineState::Loading { progress, .. } => {
                *progress += dt * 0.5;
                if *progress >= 1.0 {
                    self.state = EngineState::Playing;
                }
            }
            _ => {}
        }
    }

    fn start_loading_game(&mut self, game_path: &Path) -> Result<(), CacaoError> {
        self.state = EngineState::Loading {
            progress: 0.0,
            status: "Loading game...".to_string(),
        };

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

        let secret_key = "default_key".to_string();
        game.initialize(secret_key)?;

        self.current_game = Some(game);
        self.state = EngineState::Playing;

        Ok(())
    }

    fn unload_game(&mut self) {
        log::info!("📤 Unloading game...");
        self.current_game = None;
        self.assets.clear_assets();

        let games = Self::discover_games(&self.game_loader).unwrap_or_default();
        let particles = Self::generate_particles();
        
        self.state = EngineState::Menu {
            state: MenuState::MainMenu,
            games,
            selected_index: 0,
            scroll_offset: 0.0,
            transition_progress: 0.0,
            particles,
            theme_selector_index: 0,
        };

        self.window.set_title("Cacao Engine");
    }

    fn render(&mut self) -> Result<(), CacaoError> {
        self.renderer.begin_frame()?;

        // Extract data from state to avoid borrow issues
        match &self.state {
            EngineState::Menu { state, games, selected_index, scroll_offset, transition_progress, particles, .. } => {
                let state_clone = state.clone();
                let games_clone = games.clone();
                let selected = *selected_index;
                let scroll = *scroll_offset;
                let progress = *transition_progress;
                let particles_clone = particles.clone();
                
                // CALLING RENDER_STUNNING_MENU AS &self to avoid borrow checker errors.
                self.render_stunning_menu(&state_clone, &games_clone, selected, scroll, progress, &particles_clone)?;
            }
            EngineState::Playing => {
                if let Some(ref game) = self.current_game {
                    game.render(&mut self.renderer)?;
                }
            }
            EngineState::Loading { progress, status } => {
                let p = *progress;
                let s = status.clone();
                // CALLING RENDER_LOADING_SCREEN AS &self to avoid borrow checker errors.
                self.render_loading_screen(p, &s)?;
            }
        }

        self.renderer.end_frame()?;
        Ok(())
    }

    // FIX: Changed &mut self to &self
    fn render_stunning_menu(
        &mut self,
        menu_state: &MenuState,
        games: &[GameEntry],
        selected_index: usize,
        scroll_offset: f32,
        progress: f32,
        particles: &[MenuParticle],
    ) -> Result<(), CacaoError> {
        // FIXED: Clone theme to avoid borrow issues
        let theme = self.current_theme.clone();
        
        if matches!(theme, Theme::Animated) {
            let time = self.menu_animation_time;
            let bg_color1 = [
                0.05 + (time * 0.5).sin() * 0.02,
                0.02 + (time * 0.3).sin() * 0.02,
                0.15 + (time * 0.4).sin() * 0.03,
                1.0
            ];
            self.renderer.clear_screen(bg_color1);
        } else {
            self.renderer.clear_screen(theme.background_color());
        }

        if theme.should_show_particles() {
            for particle in particles {
                self.renderer.draw_circle(
                    particle.x,
                    particle.y,
                    particle.size,
                    16,
                    particle.color
                )?;
            }
        }

        if matches!(theme, Theme::Wii) {
            for i in 0..10 {
                let y = 100.0 + i as f32 * 60.0;
                self.renderer.draw_line(
                    80.0, y, 1200.0, y, 1.0,
                    [0.85, 0.85, 0.85, 0.3]
                )?;
            }
        }

        let alpha = progress.min(1.0);

        match menu_state {
            MenuState::MainMenu => {
                self.render_main_menu(alpha, &theme)?;
            }
            MenuState::GameList => {
                self.render_game_list(games, selected_index, scroll_offset, alpha, &theme)?;
            }
            MenuState::GameDetails(idx) => {
                if let Some(game) = games.get(*idx) {
                    self.render_game_details(&game.info, alpha, &theme)?;
                }
            }
            MenuState::ThemeSelector => {
                self.render_theme_selector(alpha, &theme)?;
            }
            MenuState::Settings => {
                self.render_settings(alpha, &theme)?;
            }
            MenuState::About => {
                self.render_about(alpha, &theme)?;
            }
        }

        Ok(())
    }
    // FIX: Changed &mut self to &self
    fn render_main_menu(&mut self, alpha: f32, theme: &Theme) -> Result<(), CacaoError> {
        // FIX: Use theme colors
        let title_color = theme.accent_color(); 
        let text_color = theme.text_color();
        let accent_color = theme.accent_color();
        let secondary_text = theme.secondary_text_color();

        // Animated title with glow effect
        let pulse = (self.menu_animation_time * 2.0).sin() * 0.1 + 0.9;
        let title_size = 64.0 * pulse;
        
        // Title glow - Use theme accent color
        for i in 0..3 {
            let offset = (i as f32 + 1.0) * 2.0;
            let glow_alpha = alpha * (0.3 - i as f32 * 0.1);
            self.renderer.draw_text(
                "CACAO ENGINE",
                320.0 + offset,
                100.0 + offset,
                title_size,
                [title_color[0], title_color[1], title_color[2], glow_alpha]
            )?;
        }
        
        // Main title
        self.renderer.draw_text("CACAO ENGINE", 320.0, 100.0, title_size, [title_color[0], title_color[1], title_color[2], title_color[3] * alpha])?;
        
        // Subtitle - Use secondary text color
        self.renderer.draw_text(
            "v1.0.0 - The Ultimate Game Engine",
            380.0,
            180.0,
            20.0,
            [secondary_text[0], secondary_text[1], secondary_text[2], secondary_text[3] * alpha * 0.8]
        )?;

        // Decorative line
        self.renderer.draw_rect(200.0, 220.0, 880.0, 3.0, [accent_color[0], accent_color[1], accent_color[2], accent_color[3] * alpha])?;

        // Menu options with bounce animation
        let base_y = 300.0;
        let bounce = (self.menu_animation_time * 4.0).sin().abs() * 5.0;
        
        // Use theme colors for menu items
        self.renderer.draw_text("▶ [ENTER] PLAY GAMES", 450.0, base_y + bounce, 28.0, [accent_color[0], accent_color[1], accent_color[2], accent_color[3] * alpha])?;
        self.renderer.draw_text("  [S] Settings", 450.0, base_y + 50.0, 24.0, [text_color[0], text_color[1], text_color[2], text_color[3] * alpha])?;
        self.renderer.draw_text("  [T] Themes", 450.0, base_y + 90.0, 24.0, [text_color[0], text_color[1], text_color[2], text_color[3] * alpha])?;
        self.renderer.draw_text("  [A] About", 450.0, base_y + 130.0, 24.0, [text_color[0], text_color[1], text_color[2], text_color[3] * alpha])?;
        self.renderer.draw_text("  [ESC] Exit", 450.0, base_y + 170.0, 24.0, [text_color[0], text_color[1], text_color[2], text_color[3] * alpha])?;

        // Footer info with fade
        let footer_alpha = alpha * ((self.menu_animation_time * 1.5).sin() * 0.3 + 0.7);
        self.renderer.draw_text(
            "Made with ❤️ by the Cacao Team",
            450.0,
            650.0,
            18.0,
            [secondary_text[0], secondary_text[1], secondary_text[2], footer_alpha]
        )?;

        Ok(())
    }

    // FIX: Changed &mut self to &self
    fn render_game_list(
        &mut self,
        games: &[GameEntry],
        selected_index: usize,
        scroll_offset: f32,
        alpha: f32,
        theme: &Theme, // Use theme
    ) -> Result<(), CacaoError> {
        let accent = theme.accent_color();
        let text_color = theme.text_color();
        let secondary_text = theme.secondary_text_color();

        // Header
        let header_color = [accent[0], accent[1], accent[2], accent[3] * alpha];
        self.renderer.draw_text("GAME LIBRARY", 80.0, 50.0, 48.0, header_color)?;
        self.renderer.draw_rect(80.0, 110.0, 1120.0, 2.0, header_color)?;

        if games.is_empty() {
            // Empty state
            self.renderer.draw_text(
                "No games found!",
                450.0,
                300.0,
                32.0,
                [text_color[0], text_color[1], text_color[2], text_color[3] * alpha * 0.8]
            )?;
            self.renderer.draw_text(
                "Create a game with: cargo run --example create_demo_game",
                250.0,
                350.0,
                16.0,
                [secondary_text[0], secondary_text[1], secondary_text[2], secondary_text[3] * alpha * 0.7]
            )?;
        } else {
            // Game cards with beautiful design
            let start_y = 150.0 - scroll_offset;
            
            for (i, game) in games.iter().enumerate() {
                let y = start_y + (i as f32 * 120.0);
                
                // Skip if off-screen
                if y < 100.0 || y > 700.0 {
                    continue;
                }

                let is_selected = i == selected_index;
                
                // Card background with glow
                let card_color = if is_selected {
                    let pulse = (self.menu_animation_time * 6.0).sin() * 0.1 + 0.9;
                    [
                        theme.selected_card_color()[0] * pulse, 
                        theme.selected_card_color()[1] * pulse, 
                        theme.selected_card_color()[2] * pulse, 
                        theme.selected_card_color()[3] * alpha
                    ]
                } else {
                    [theme.card_color()[0], theme.card_color()[1], theme.card_color()[2], theme.card_color()[3] * alpha * 0.7]
                };
                
                // Card shadow
                if is_selected {
                    self.renderer.draw_rect(88.0, y + 8.0, 1104.0, 96.0, [0.0, 0.0, 0.0, alpha * 0.5])?;
                }
                
                // Card
                self.renderer.draw_rect(80.0, y, 1104.0, 96.0, card_color)?;
                
                // Card border
                let border_color = if is_selected {
                    accent
                } else {
                    [secondary_text[0], secondary_text[1], secondary_text[2], secondary_text[3] * alpha * 0.5]
                };
                self.renderer.draw_rect_outline(80.0, y, 1104.0, 96.0, 2.0, border_color)?;

                // Selection indicator
                if is_selected {
                    let indicator_x = 50.0 + ((self.menu_animation_time * 4.0).sin() * 5.0);
                    self.renderer.draw_text(
                        "▶",
                        indicator_x,
                        y + 35.0,
                        32.0,
                        [accent[0], accent[1], accent[2], accent[3] * alpha]
                    )?;
                }

                // Game info
                let title_text_color = if is_selected {
                    text_color
                } else {
                    [text_color[0], text_color[1], text_color[2], text_color[3] * alpha * 0.9]
                };
                
                self.renderer.draw_text(
                    &game.info.title,
                    110.0,
                    y + 20.0,
                    24.0,
                    title_text_color
                )?;
                
                let info_text = format!("{} • v{}", game.info.author, game.info.version);
                self.renderer.draw_text(
                    &info_text,
                    110.0,
                    y + 50.0,
                    16.0,
                    [secondary_text[0], secondary_text[1], secondary_text[2], secondary_text[3] * alpha * 0.8]
                )?;
            }
        }

        // Controls hint
        self.renderer.draw_text(
            "↑↓ Navigate • [ENTER] Select • [ESC] Back",
            350.0,
            680.0,
            16.0,
            [secondary_text[0], secondary_text[1], secondary_text[2], secondary_text[3] * alpha * 0.7]
        )?;

        Ok(())
    }

    // FIX: Changed &mut self to &self
    fn render_game_details(&mut self, info: &GameInfo, alpha: f32, theme: &Theme) -> Result<(), CacaoError> {
        // FIX: Use theme colors
        let accent = theme.accent_color();
        let text = theme.text_color();
        let card = theme.card_color();
        let secondary_text = theme.secondary_text_color();
        
        // Banner area (placeholder for future banner images)
        let banner_y = 100.0;
        let pulse = (self.menu_animation_time).sin() * 0.05 + 0.95;
        self.renderer.draw_rect(
            140.0,
            banner_y,
            1000.0,
            300.0 * pulse,
            [card[0], card[1], card[2], card[3] * alpha * 0.8]
        )?;
        self.renderer.draw_rect_outline(140.0, banner_y, 1000.0, 300.0, 3.0, accent)?;
        
        // Banner placeholder text
        self.renderer.draw_text(
            &info.title,
            300.0,
            230.0,
            48.0,
            [text[0], text[1], text[2], text[3] * alpha]
        )?;

        // Game details panel
        let details_y = 450.0;
        self.renderer.draw_text("GAME INFORMATION", 140.0, details_y, 28.0, accent)?;
        self.renderer.draw_rect(140.0, details_y + 35.0, 400.0, 2.0, accent)?;
        
        let mut info_y = details_y + 60.0;
        
        self.renderer.draw_text("Author:", 140.0, info_y, 20.0, secondary_text)?;
        self.renderer.draw_text(&info.author, 300.0, info_y, 20.0, text)?;
        info_y += 35.0;
        
        self.renderer.draw_text("Version:", 140.0, info_y, 20.0, secondary_text)?;
        self.renderer.draw_text(&info.version, 300.0, info_y, 20.0, text)?;
        info_y += 35.0;
        
        self.renderer.draw_text("Engine:", 140.0, info_y, 20.0, secondary_text)?;
        self.renderer.draw_text(&info.engine_version, 300.0, info_y, 20.0, text)?;

        // Description box
        let desc_y = details_y;
        self.renderer.draw_rect(600.0, desc_y, 540.0, 200.0, [card[0], card[1], card[2], card[3] * alpha * 0.8])?;
        self.renderer.draw_rect_outline(600.0, desc_y, 540.0, 200.0, 2.0, accent)?;
        self.renderer.draw_text("Description", 620.0, desc_y + 20.0, 20.0, accent)?;
        self.renderer.draw_text(&info.description, 620.0, desc_y + 60.0, 16.0, text)?;

        // Play button with animation
        let button_y = 640.0;
        let button_pulse = (self.menu_animation_time * 4.0).sin() * 10.0;
        self.renderer.draw_rect(
            500.0 - button_pulse / 2.0,
            button_y,
            280.0 + button_pulse,
            60.0,
            [theme.selected_card_color()[0], theme.selected_card_color()[1], theme.selected_card_color()[2], theme.selected_card_color()[3] * alpha]
        )?;
        self.renderer.draw_rect_outline(
            500.0 - button_pulse / 2.0,
            button_y,
            280.0 + button_pulse,
            60.0,
            3.0,
            accent
        )?;
        self.renderer.draw_text(
            "[ENTER] PLAY NOW",
            540.0,
            button_y + 20.0,
            24.0,
            accent
        )?;

        // Back hint
        self.renderer.draw_text(
            "[ESC] Back to Library",
            530.0,
            710.0,
            16.0,
            [secondary_text[0], secondary_text[1], secondary_text[2], secondary_text[3] * alpha * 0.7]
        )?;

        Ok(())
    }

    // FIX: Changed &mut self to &self
    fn render_theme_selector(&mut self, alpha: f32, theme: &Theme) -> Result<(), CacaoError> {
        let text_color = theme.text_color();
        let accent = theme.accent_color();
        let secondary_text = theme.secondary_text_color();

        self.renderer.draw_text("THEME SELECTOR", 80.0, 80.0, 48.0, accent)?;
        self.renderer.draw_rect(80.0, 140.0, 500.0, 2.0, accent)?;

        let theme_options = Theme::all();

        // FIX: The `self.state` reference is implicitly immutable here because render_theme_selector takes `&self`
        if let EngineState::Menu { theme_selector_index, .. } = &self.state { 
            let mut y = 220.0;
            for (i, t) in theme_options.iter().enumerate() {
                // E0614 fix: Since theme_selector_index is &usize, we must dereference it.
                // This line was already correctly written in your original code if &self was used.
                let is_selected = i == *theme_selector_index; 
                let color = if is_selected { accent } else { text_color };
                let size = if is_selected { 32.0 } else { 24.0 };

                // Draw card background
                let card_color = if is_selected { theme.selected_card_color() } else { theme.card_color() };
                self.renderer.draw_rect(100.0, y, 500.0, 50.0, [card_color[0], card_color[1], card_color[2], card_color[3] * alpha])?;
                
                // Draw selection indicator
                if is_selected {
                    let indicator_x = 60.0 + (self.menu_animation_time * 4.0).sin() * 3.0;
                    self.renderer.draw_text("▶", indicator_x, y + 10.0, size, accent)?;
                }

                // Draw theme name
                self.renderer.draw_text(
                    t.name(),
                    120.0,
                    y + 15.0,
                    size,
                    [color[0], color[1], color[2], color[3] * alpha]
                )?;

                y += 70.0;
            }
        }

        self.renderer.draw_text(
            "[ENTER] Apply Theme • [ESC] Back",
            300.0,
            680.0,
            16.0,
            [secondary_text[0], secondary_text[1], secondary_text[2], secondary_text[3] * alpha * 0.7]
        )?;

        Ok(())
    }

    // FIX: Changed &mut self to &self
    fn render_settings(&mut self, alpha: f32, theme: &Theme) -> Result<(), CacaoError> {
        let accent = theme.accent_color();
        let text = theme.text_color();
        let secondary_text = theme.secondary_text_color();
        
        self.renderer.draw_text("SETTINGS", 80.0, 80.0, 48.0, accent)?;
        self.renderer.draw_rect(80.0, 140.0, 300.0, 2.0, accent)?;

        let mut y = 200.0;
        self.renderer.draw_text("Audio", 100.0, y, 28.0, text)?;
        y += 50.0;
        self.renderer.draw_text("Master Volume: 100%", 120.0, y, 20.0, secondary_text)?;
        y += 35.0;
        self.renderer.draw_text("Music Volume: 80%", 120.0, y, 20.0, secondary_text)?;
        y += 35.0;
        self.renderer.draw_text("SFX Volume: 100%", 120.0, y, 20.0, secondary_text)?;
        
        y += 80.0;
        self.renderer.draw_text("Graphics", 100.0, y, 28.0, text)?;
        y += 50.0;
        self.renderer.draw_text("Resolution: 1280x720", 120.0, y, 20.0, secondary_text)?;
        y += 35.0;
        self.renderer.draw_text("Fullscreen: Off", 120.0, y, 20.0, secondary_text)?;
        y += 35.0;
        self.renderer.draw_text("VSync: On", 120.0, y, 20.0, secondary_text)?;

        self.renderer.draw_text(
            "(Settings coming soon!)",
            480.0,
            350.0,
            24.0,
            [secondary_text[0], secondary_text[1], secondary_text[2], secondary_text[3] * alpha * 0.6]
        )?;

        self.renderer.draw_text(
            "[ESC] Back to Main Menu",
            490.0,
            680.0,
            16.0,
            [secondary_text[0], secondary_text[1], secondary_text[2], secondary_text[3] * alpha * 0.7]
        )?;

        Ok(())
    }

    // FIX: Changed &mut self to &self
    fn render_about(&mut self, alpha: f32, theme: &Theme) -> Result<(), CacaoError> {
        let accent = theme.accent_color();
        let text = theme.text_color();
        let secondary_text = theme.secondary_text_color();
        
        // Animated logo area
        let logo_pulse = (self.menu_animation_time * 2.0).sin() * 0.1 + 0.9;
        self.renderer.draw_circle(
            640.0,
            200.0,
            80.0 * logo_pulse,
            32,
            [theme.selected_card_color()[0], theme.selected_card_color()[1], theme.selected_card_color()[2], theme.selected_card_color()[3] * alpha * 0.8]
        )?;
        self.renderer.draw_circle_outline(
            640.0,
            200.0,
            80.0 * logo_pulse,
            32,
            3.0,
            accent
        )?;
        
        self.renderer.draw_text("🍫", 605.0, 170.0, 64.0, [accent[0], accent[1], accent[2], accent[3] * alpha])?;

        self.renderer.draw_text("CACAO ENGINE", 490.0, 320.0, 36.0, accent)?;
        self.renderer.draw_text("Version 1.0.0", 545.0, 365.0, 20.0, text)?;

        let mut info_y = 420.0;
        self.renderer.draw_text(
            "A beautiful offline game engine with",
            460.0,
            info_y,
            18.0,
            secondary_text
        )?;
        info_y += 30.0;
        self.renderer.draw_text(
            "stunning UI and powerful features",
            465.0,
            info_y,
            18.0,
            secondary_text
        )?;

        info_y += 60.0;
        self.renderer.draw_text("Features:", 560.0, info_y, 24.0, accent)?;
        info_y += 40.0;
        
        let features = [
            "• Lua scripting engine",
            "• Encrypted game distribution",
            "• Save game system",
            "• Audio system",
            "• Beautiful UI",
        ];
        
        for feature in &features {
            self.renderer.draw_text(feature, 520.0, info_y, 16.0, text)?;
            info_y += 28.0;
        }

        self.renderer.draw_text(
            "Made with ❤️ by Adam Hawree",
            500.0,
            650.0,
            18.0,
            secondary_text
        )?;

        self.renderer.draw_text(
            "[ESC] Back to Main Menu",
            490.0,
            690.0,
            16.0,
            [secondary_text[0], secondary_text[1], secondary_text[2], secondary_text[3] * alpha * 0.7]
        )?;

        Ok(())
    }

    // FIX: Changed &mut self to &self
    fn render_loading_screen(&mut self, progress: f32, status: &str) -> Result<(), CacaoError> {
        self.renderer.clear_screen([0.05, 0.02, 0.15, 1.0]);

        // Loading circle animation
        let circle_count = 8;
        let base_angle = self.menu_animation_time * 2.0;
        
        for i in 0..circle_count {
            let angle = base_angle + (i as f32 * std::f32::consts::PI * 2.0 / circle_count as f32);
            let x = 640.0 + angle.cos() * 60.0;
            let y = 300.0 + angle.sin() * 60.0;
            let size = 8.0 + (angle * 2.0).sin().abs() * 4.0;
            let alpha = 0.3 + (angle * 2.0).sin().abs() * 0.7;
            
            self.renderer.draw_circle(x, y, size, 16, [1.0, 0.6, 0.2, alpha])?;
        }

        // Progress bar
        let bar_width = 600.0;
        let bar_x = 340.0;
        let bar_y = 400.0;
        
        self.renderer.draw_rect(bar_x, bar_y, bar_width, 30.0, [0.2, 0.15, 0.25, 0.8])?;
        self.renderer.draw_rect(
            bar_x,
            bar_y,
            bar_width * progress,
            30.0,
            [1.0, 0.6, 0.2, 0.9]
        )?;
        self.renderer.draw_rect_outline(bar_x, bar_y, bar_width, 30.0, 2.0, [1.0, 0.6, 0.2, 1.0])?;

        // Status text
        self.renderer.draw_text(status, 540.0, 460.0, 20.0, [0.9, 0.9, 0.9, 0.9])?;
        
        let percent = format!("{}%", (progress * 100.0) as u32);
        self.renderer.draw_text(&percent, 620.0, 370.0, 24.0, [1.0, 0.9, 0.4, 1.0])?;

        Ok(())
    }
}