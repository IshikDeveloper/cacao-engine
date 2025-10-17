// src/renderer/mod.rs (FIXED)
pub mod shader;
pub mod texture;
pub mod sprite;
pub mod camera;
pub mod text;
pub mod primitive;

use winit::window::Window;
use crate::errors::CacaoError;

pub use texture::Texture;
pub use sprite::{Sprite, SpriteRenderer};
pub use camera::Camera;
pub use text::TextRenderer;
pub use primitive::PrimitiveRenderer;

pub struct Renderer {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    
    sprite_renderer: SpriteRenderer,
    text_renderer: TextRenderer,
    primitive_renderer: PrimitiveRenderer,
    camera: Camera,
    
    // FIX: Field to store the clear color instead of using a temporary pass
    clear_color: wgpu::Color, 
    
    current_encoder: Option<wgpu::CommandEncoder>,
    current_output: Option<wgpu::SurfaceTexture>,
    current_view: Option<wgpu::TextureView>,
}

impl Renderer {
    pub async fn new(window: &Window) -> Result<Self, CacaoError> {
        let size = window.inner_size();
        
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        let surface = unsafe { instance.create_surface(window) }
            .map_err(|e| CacaoError::RenderError(format!("Failed to create surface: {}", e)))?;

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.ok_or_else(|| CacaoError::RenderError("Failed to find adapter".to_string()))?;

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None,
        ).await.map_err(|e| CacaoError::RenderError(format!("Failed to create device: {}", e)))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let sprite_renderer = SpriteRenderer::new(&device, &config)?;
        let text_renderer = TextRenderer::new(&device, &queue, &config)?;
        let primitive_renderer = PrimitiveRenderer::new(&device, &config)?;
        let camera = Camera::new(size.width as f32, size.height as f32);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            sprite_renderer,
            text_renderer,
            primitive_renderer,
            camera,
            // FIX: Initialize the clear color (default to black)
            clear_color: wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }, 
            current_encoder: None,
            current_output: None,
            current_view: None,
        })
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.camera.set_viewport(new_size.width as f32, new_size.height as f32);
        }
    }

    pub fn begin_frame(&mut self) -> Result<(), CacaoError> {
        let output = self.surface.get_current_texture()
            .map_err(|e| CacaoError::RenderError(format!("Failed to get surface texture: {}", e)))?;
        
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        
        self.current_output = Some(output);
        self.current_view = Some(view);
        self.current_encoder = Some(encoder);
        
        Ok(())
    }

    // FIX: Rewrite end_frame to create a single render pass and execute all drawing.
    pub fn end_frame(&mut self) -> Result<(), CacaoError> {
            // FIX 1 (Mutability E0596): Ensure 'encoder' is mutable when taken from self
            if let (Some(mut encoder), Some(view)) = (self.current_encoder.take(), self.current_view.take()) {
                
                // 1. Begin the single render pass
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Primary Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(self.clear_color), 
                            // FIX 2 (E0433 StoreOp): For older wgpu versions, use 'store: true'.
                            // For modern wgpu (0.19+), you may need to add 'use wgpu::StoreOp;' and use 'store: wgpu::StoreOp::Store,'
                            store: true, 
                        },
                    })],
                    depth_stencil_attachment: None,
                });

                // 2. Flush all renderers using the SAME render pass
                // FIX 3 (E0061 Argument Count): Remove the redundant 'self.get_device()' argument.
                // The Device is no longer passed here; the renderers must be modified below.
                self.primitive_renderer.flush(&mut render_pass, self.get_queue(), &mut self.camera);
                self.sprite_renderer.flush(&mut render_pass, self.get_queue(), &mut self.camera);
                self.text_renderer.flush(&mut render_pass, self.get_queue(), &mut self.camera);
                
                // 3. Render pass implicitly dropped here

                // 4. Submit command buffer to the queue
                self.queue.submit(std::iter::once(encoder.finish()));
            }

            if let Some(output) = self.current_output.take() {
                output.present();
            }

            Ok(())
        }

    // FIX: Update clear_screen to only set the clear_color field
    pub fn clear_screen(&mut self, color: [f32; 4]) {
        self.clear_color = wgpu::Color {
            r: color[0] as f64,
            g: color[1] as f64,
            b: color[2] as f64,
            a: color[3] as f64,
        };
    }

    pub fn draw_sprite(&mut self, sprite: &Sprite, x: f32, y: f32, rotation: f32, scale: f32) -> Result<(), CacaoError> {
        self.sprite_renderer.draw_sprite(sprite, x, y, rotation, scale, &self.camera);
        Ok(())
    }

    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: [f32; 4]) -> Result<(), CacaoError> {
        self.text_renderer.draw_text(text, x, y, size, color);
        Ok(())
    }

    pub fn draw_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) -> Result<(), CacaoError> {
        self.primitive_renderer.draw_rect(x, y, width, height, color);
        Ok(())
    }

    pub fn draw_rect_outline(&mut self, x: f32, y: f32, width: f32, height: f32, thickness: f32, color: [f32; 4]) -> Result<(), CacaoError> {
        self.primitive_renderer.draw_rect_outline(x, y, width, height, thickness, color);
        Ok(())
    }

    pub fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: [f32; 4]) -> Result<(), CacaoError> {
        self.primitive_renderer.draw_line(x1, y1, x2, y2, thickness, color);
        Ok(())
    }

    pub fn draw_circle(&mut self, x: f32, y: f32, radius: f32, segments: u32, color: [f32; 4]) -> Result<(), CacaoError> {
        self.primitive_renderer.draw_circle(x, y, radius, segments, color);
        Ok(())
    }

    pub fn draw_circle_outline(&mut self, x: f32, y: f32, radius: f32, segments: u32, thickness: f32, color: [f32; 4]) -> Result<(), CacaoError> {
        self.primitive_renderer.draw_circle_outline(x, y, radius, segments, thickness, color);
        Ok(())
    }

    pub fn draw_triangle(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, color: [f32; 4]) -> Result<(), CacaoError> {
        self.primitive_renderer.draw_triangle(x1, y1, x2, y2, x3, y3, color);
        Ok(())
    }

    pub fn get_camera(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn get_device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn get_queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}