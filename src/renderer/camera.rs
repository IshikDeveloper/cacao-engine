// src/renderer/camera.rs
use glam::{Mat4, Vec2, Vec3};

#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vec2,
    pub zoom: f32,
    pub rotation: f32,
    viewport_width: f32,
    viewport_height: f32,
    view_matrix: Mat4,
    projection_matrix: Mat4,
    view_projection_matrix: Mat4,
    dirty: bool,
}

impl Camera {
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        let mut camera = Self {
            position: Vec2::ZERO,
            zoom: 1.0,
            rotation: 0.0,
            viewport_width,
            viewport_height,
            view_matrix: Mat4::IDENTITY,
            projection_matrix: Mat4::IDENTITY,
            view_projection_matrix: Mat4::IDENTITY,
            dirty: true,
        };
        camera.update_matrices();
        camera
    }

    pub fn set_position(&mut self, position: Vec2) {
        self.position = position;
        self.dirty = true;
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.max(0.1); // Prevent negative/zero zoom
        self.dirty = true;
    }

    pub fn set_rotation(&mut self, rotation: f32) {
        self.rotation = rotation;
        self.dirty = true;
    }

    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.viewport_width = width;
        self.viewport_height = height;
        self.dirty = true;
    }

    pub fn translate(&mut self, delta: Vec2) {
        self.position += delta;
        self.dirty = true;
    }

    pub fn get_view_projection_matrix(&mut self) -> Mat4 {
        if self.dirty {
            self.update_matrices();
        }
        self.view_projection_matrix
    }

    fn update_matrices(&mut self) {
        // Create orthographic projection matrix
        let left = -self.viewport_width / 2.0;
        let right = self.viewport_width / 2.0;
        let bottom = -self.viewport_height / 2.0;
        let top = self.viewport_height / 2.0;
        
        self.projection_matrix = Mat4::orthographic_rh(left, right, bottom, top, -1000.0, 1000.0);

        // Create view matrix
        let translation = Mat4::from_translation(Vec3::new(-self.position.x, -self.position.y, 0.0));
        let rotation = Mat4::from_rotation_z(-self.rotation);
        let scale = Mat4::from_scale(Vec3::new(self.zoom, self.zoom, 1.0));
        
        self.view_matrix = scale * rotation * translation;
        self.view_projection_matrix = self.projection_matrix * self.view_matrix;
        self.dirty = false;
    }

    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2 {
        // Convert screen coordinates to world coordinates
        let normalized_x = (screen_pos.x / self.viewport_width) * 2.0 - 1.0;
        let normalized_y = -((screen_pos.y / self.viewport_height) * 2.0 - 1.0);
        
        let world_pos = Vec2::new(
            (normalized_x * self.viewport_width / 2.0) / self.zoom + self.position.x,
            (normalized_y * self.viewport_height / 2.0) / self.zoom + self.position.y,
        );
        
        world_pos
    }
}