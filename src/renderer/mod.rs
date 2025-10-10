// src/renderer/mod.rs
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

    pub fn end_frame(&mut self) -> Result<(), CacaoError> {
        // Flush all renderers in order: primitives -> sprites -> text
        // This ensures proper layering (primitives in back, text in front)
        if let (Some(encoder), Some(view)) = (&mut self.current_encoder, &self.current_view) {
            self.primitive_renderer.flush(
                encoder,
                view,
                &self.queue,
                &mut self.camera,
            );
            
            self.sprite_renderer.flush(
                encoder,
                view,
                &self.device,
                &self.queue,
                &mut self.camera,
            );
            
            self.text_renderer.flush(
                encoder,
                view,
                &self.queue,
                &mut self.camera,
            );
        }
        
        // Submit commands
        if let Some(encoder) = self.current_encoder.take() {
            self.queue.submit(std::iter::once(encoder.finish()));
        }
        
        // Present
        if let Some(output) = self.current_output.take() {
            output.present();
        }
        
        self.current_view = None;
        Ok(())
    }

    pub fn clear_screen(&mut self, color: [f32; 4]) {
        if let (Some(encoder), Some(view)) = (&mut self.current_encoder, &self.current_view) {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: color[0] as f64,
                            g: color[1] as f64,
                            b: color[2] as f64,
                            a: color[3] as f64,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
        }
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