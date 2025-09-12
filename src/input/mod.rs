// src/input/mod.rs
use std::collections::HashSet;
use winit::event::{WindowEvent, KeyboardInput, VirtualKeyCode, ElementState, MouseButton};
use glam::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    A, B, X, Y,
    DPadUp, DPadDown, DPadLeft, DPadRight,
    LeftShoulder, RightShoulder,
    LeftTrigger, RightTrigger,
    LeftStick, RightStick,
    Start, Select,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputButton {
    Key(VirtualKeyCode),
    Mouse(MouseButton),
    Gamepad(GamepadButton),
}

pub struct InputManager {
    // Keyboard state
    keys_pressed: HashSet<VirtualKeyCode>,
    keys_just_pressed: HashSet<VirtualKeyCode>,
    keys_just_released: HashSet<VirtualKeyCode>,
    
    // Mouse state
    mouse_buttons_pressed: HashSet<MouseButton>,
    mouse_buttons_just_pressed: HashSet<MouseButton>,
    mouse_buttons_just_released: HashSet<MouseButton>,
    mouse_position: Vec2,
    mouse_delta: Vec2,
    scroll_delta: Vec2,
    
    // Gamepad state (simplified for now)
    gamepad_buttons_pressed: HashSet<GamepadButton>,
    gamepad_buttons_just_pressed: HashSet<GamepadButton>,
    gamepad_buttons_just_released: HashSet<GamepadButton>,
    left_stick: Vec2,
    right_stick: Vec2,
    
    // Input mapping
    input_map: std::collections::HashMap<String, Vec<InputButton>>,
    
    // Previous frame state for delta calculations
    previous_mouse_position: Vec2,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            keys_pressed: HashSet::new(),
            keys_just_pressed: HashSet::new(),
            keys_just_released: HashSet::new(),
            mouse_buttons_pressed: HashSet::new(),
            mouse_buttons_just_pressed: HashSet::new(),
            mouse_buttons_just_released: HashSet::new(),
            mouse_position: Vec2::ZERO,
            mouse_delta: Vec2::ZERO,
            scroll_delta: Vec2::ZERO,
            gamepad_buttons_pressed: HashSet::new(),
            gamepad_buttons_just_pressed: HashSet::new(),
            gamepad_buttons_just_released: HashSet::new(),
            left_stick: Vec2::ZERO,
            right_stick: Vec2::ZERO,
            input_map: std::collections::HashMap::new(),
            previous_mouse_position: Vec2::ZERO,
        }
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                input: KeyboardInput {
                    state,
                    virtual_keycode: Some(keycode),
                    ..
                },
                ..
            } => {
                match state {
                    ElementState::Pressed => {
                        if !self.keys_pressed.contains(keycode) {
                            self.keys_just_pressed.insert(*keycode);
                        }
                        self.keys_pressed.insert(*keycode);
                    }
                    ElementState::Released => {
                        self.keys_pressed.remove(keycode);
                        self.keys_just_released.insert(*keycode);
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                match state {
                    ElementState::Pressed => {
                        if !self.mouse_buttons_pressed.contains(button) {
                            self.mouse_buttons_just_pressed.insert(*button);
                        }
                        self.mouse_buttons_pressed.insert(*button);
                    }
                    ElementState::Released => {
                        self.mouse_buttons_pressed.remove(button);
                        self.mouse_buttons_just_released.insert(*button);
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = Vec2::new(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        self.scroll_delta = Vec2::new(*x, *y);
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        self.scroll_delta = Vec2::new(pos.x as f32, pos.y as f32);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn update(&mut self) {
        // Calculate mouse delta
        self.mouse_delta = self.mouse_position - self.previous_mouse_position;
        self.previous_mouse_position = self.mouse_position;
        
        // Clear "just pressed/released" states
        self.keys_just_pressed.clear();
        self.keys_just_released.clear();
        self.mouse_buttons_just_pressed.clear();
        self.mouse_buttons_just_released.clear();
        self.gamepad_buttons_just_pressed.clear();
        self.gamepad_buttons_just_released.clear();
        
        // Reset scroll delta
        self.scroll_delta = Vec2::ZERO;
    }

    // Keyboard input methods
    pub fn is_key_pressed(&self, key: VirtualKeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn is_key_just_pressed(&self, key: VirtualKeyCode) -> bool {
        self.keys_just_pressed.contains(&key)
    }

    pub fn is_key_just_released(&self, key: VirtualKeyCode) -> bool {
        self.keys_just_released.contains(&key)
    }

    // Mouse input methods
    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_pressed.contains(&button)
    }

    pub fn is_mouse_button_just_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_just_pressed.contains(&button)
    }

    pub fn is_mouse_button_just_released(&self, button: MouseButton) -> bool {
        self.mouse_buttons_just_released.contains(&button)
    }

    pub fn get_mouse_position(&self) -> Vec2 {
        self.mouse_position
    }

    pub fn get_mouse_delta(&self) -> Vec2 {
        self.mouse_delta
    }

    pub fn get_scroll_delta(&self) -> Vec2 {
        self.scroll_delta
    }

    // Gamepad input methods
    pub fn is_gamepad_button_pressed(&self, button: GamepadButton) -> bool {
        self.gamepad_buttons_pressed.contains(&button)
    }

    pub fn is_gamepad_button_just_pressed(&self, button: GamepadButton) -> bool {
        self.gamepad_buttons_just_pressed.contains(&button)
    }

    pub fn is_gamepad_button_just_released(&self, button: GamepadButton) -> bool {
        self.gamepad_buttons_just_released.contains(&button)
    }

    pub fn get_left_stick(&self) -> Vec2 {
        self.left_stick
    }

    pub fn get_right_stick(&self) -> Vec2 {
        self.right_stick
    }

    // Input mapping system
    pub fn map_input(&mut self, action_name: String, buttons: Vec<InputButton>) {
        self.input_map.insert(action_name, buttons);
    }

    pub fn is_action_pressed(&self, action_name: &str) -> bool {
        if let Some(buttons) = self.input_map.get(action_name) {
            buttons.iter().any(|button| self.is_input_button_pressed(*button))
        } else {
            false
        }
    }

    pub fn is_action_just_pressed(&self, action_name: &str) -> bool {
        if let Some(buttons) = self.input_map.get(action_name) {
            buttons.iter().any(|button| self.is_input_button_just_pressed(*button))
        } else {
            false
        }
    }

    pub fn is_action_just_released(&self, action_name: &str) -> bool {
        if let Some(buttons) = self.input_map.get(action_name) {
            buttons.iter().any(|button| self.is_input_button_just_released(*button))
        } else {
            false
        }
    }

    fn is_input_button_pressed(&self, button: InputButton) -> bool {
        match button {
            InputButton::Key(key) => self.is_key_pressed(key),
            InputButton::Mouse(mouse_button) => self.is_mouse_button_pressed(mouse_button),
            InputButton::Gamepad(gamepad_button) => self.is_gamepad_button_pressed(gamepad_button),
        }
    }

    fn is_input_button_just_pressed(&self, button: InputButton) -> bool {
        match button {
            InputButton::Key(key) => self.is_key_just_pressed(key),
            InputButton::Mouse(mouse_button) => self.is_mouse_button_just_pressed(mouse_button),
            InputButton::Gamepad(gamepad_button) => self.is_gamepad_button_just_pressed(gamepad_button),
        }
    }

    fn is_input_button_just_released(&self, button: InputButton) -> bool {
        match button {
            InputButton::Key(key) => self.is_key_just_released(key),
            InputButton::Mouse(mouse_button) => self.is_mouse_button_just_released(mouse_button),
            InputButton::Gamepad(gamepad_button) => self.is_gamepad_button_just_released(gamepad_button),
        }
    }

    // Utility methods
    pub fn get_pressed_keys(&self) -> Vec<VirtualKeyCode> {
        self.keys_pressed.iter().cloned().collect()
    }

    pub fn get_just_pressed_keys(&self) -> Vec<VirtualKeyCode> {
        self.keys_just_pressed.iter().cloned().collect()
    }

    pub fn clear_input_map(&mut self) {
        self.input_map.clear();
    }

    pub fn remove_input_mapping(&mut self, action_name: &str) {
        self.input_map.remove(action_name);
    }

    // Common input mappings setup
    pub fn setup_default_mappings(&mut self) {
        // Movement
        self.map_input("move_up".to_string(), vec![
            InputButton::Key(VirtualKeyCode::W),
            InputButton::Key(VirtualKeyCode::Up),
            InputButton::Gamepad(GamepadButton::DPadUp),
        ]);
        
        self.map_input("move_down".to_string(), vec![
            InputButton::Key(VirtualKeyCode::S),
            InputButton::Key(VirtualKeyCode::Down),
            InputButton::Gamepad(GamepadButton::DPadDown),
        ]);
        
        self.map_input("move_left".to_string(), vec![
            InputButton::Key(VirtualKeyCode::A),
            InputButton::Key(VirtualKeyCode::Left),
            InputButton::Gamepad(GamepadButton::DPadLeft),
        ]);
        
        self.map_input("move_right".to_string(), vec![
            InputButton::Key(VirtualKeyCode::D),
            InputButton::Key(VirtualKeyCode::Right),
            InputButton::Gamepad(GamepadButton::DPadRight),
        ]);

        // Actions
        self.map_input("jump".to_string(), vec![
            InputButton::Key(VirtualKeyCode::Space),
            InputButton::Gamepad(GamepadButton::A),
        ]);

        self.map_input("action".to_string(), vec![
            InputButton::Key(VirtualKeyCode::Return),
            InputButton::Key(VirtualKeyCode::E),
            InputButton::Mouse(MouseButton::Left),
            InputButton::Gamepad(GamepadButton::B),
        ]);

        self.map_input("cancel".to_string(), vec![
            InputButton::Key(VirtualKeyCode::Escape),
            InputButton::Mouse(MouseButton::Right),
            InputButton::Gamepad(GamepadButton::Y),
        ]);
    }

    // Get movement input as a normalized vector
    pub fn get_movement_vector(&self) -> Vec2 {
        let mut movement = Vec2::ZERO;
        
        if self.is_action_pressed("move_up") {
            movement.y += 1.0;
        }
        if self.is_action_pressed("move_down") {
            movement.y -= 1.0;
        }
        if self.is_action_pressed("move_left") {
            movement.x -= 1.0;
        }
        if self.is_action_pressed("move_right") {
            movement.x += 1.0;
        }

        // Add gamepad stick input
        movement += self.left_stick;
        
        // Normalize to prevent faster diagonal movement
        if movement.length() > 1.0 {
            movement = movement.normalize();
        }
        
        movement
    }
}