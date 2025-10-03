// src/renderer/text.rs
use wgpu::util::DeviceExt;
use crate::errors::CacaoError;
use super::Camera;

// Simple bitmap font - 8x8 pixel characters
const FONT_WIDTH: u32 = 8;
const FONT_HEIGHT: u32 = 8;
const CHARS_PER_ROW: u32 = 16;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GlyphVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
    color: [f32; 4],
}

impl GlyphVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x4,
    ];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GlyphVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TextUniform {
    view_proj: [[f32; 4]; 4],
}

pub struct TextRenderer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    font_texture_bind_group: wgpu::BindGroup,
    
    vertices: Vec<GlyphVertex>,
    indices: Vec<u16>,
    max_chars: usize,
}

impl TextRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
    ) -> Result<Self, CacaoError> {
        // Create simple white font texture (we'll use color to tint it)
        let font_texture = Self::create_font_texture(device, queue)?;
        
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Text Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/text.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Text Uniform Buffer"),
            size: std::mem::size_of::<TextUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Text Uniform Bind Group Layout"),
        });

        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
            label: Some("Text Texture Bind Group Layout"),
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("Text Uniform Bind Group"),
        });

        let font_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&font_texture.1),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&font_texture.2),
                },
            ],
            label: Some("Font Texture Bind Group"),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Text Render Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Text Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[GlyphVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let max_chars = 1024;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Text Vertex Buffer"),
            size: (max_chars * 4 * std::mem::size_of::<GlyphVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Text Index Buffer"),
            size: (max_chars * 6 * std::mem::size_of::<u16>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            uniform_bind_group,
            font_texture_bind_group,
            vertices: Vec::new(),
            indices: Vec::new(),
            max_chars,
        })
    }

    fn create_font_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(wgpu::Texture, wgpu::TextureView, wgpu::Sampler), CacaoError> {
        // Create a simple 128x128 bitmap font texture (16x16 characters, 8x8 pixels each)
        let size = wgpu::Extent3d {
            width: 128,
            height: 128,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Font Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Generate simple ASCII font bitmap
        let font_data = Self::generate_simple_font();
        
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &font_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(128),
                rows_per_image: Some(128),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok((texture, view, sampler))
    }

    fn generate_simple_font() -> Vec<u8> {
        let mut data = vec![0u8; 128 * 128];
        
        // Generate a very simple 5x7 font for ASCII 32-126
        for ascii in 32u8..=126 {
            let char_x = ((ascii - 32) % 16) as usize * 8;
            let char_y = ((ascii - 32) / 16) as usize * 8;
            
            // Simple pattern: draw a rectangle outline for each character
            // This is just placeholder - you'd load a real font here
            for py in 1..7 {
                for px in 1..7 {
                    let should_draw = px == 1 || px == 6 || py == 1 || py == 6;
                    if should_draw {
                        let idx = (char_y + py) * 128 + (char_x + px);
                        data[idx] = 255;
                    }
                }
            }
        }
        
        data
    }

    pub fn draw_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: [f32; 4],
    ) {
        let mut cursor_x = x;
        let cursor_y = y;

        for ch in text.chars() {
            if ch == '\n' {
                // Newline not supported in this simple version
                continue;
            }

            let ascii = ch as u8;
            if ascii < 32 || ascii > 126 {
                continue; // Skip non-printable characters
            }

            let char_index = ascii - 32;
            let tex_x = (char_index % 16) as f32 / 16.0;
            let tex_y = (char_index / 16) as f32 / 16.0;
            let tex_w = 1.0 / 16.0;
            let tex_h = 1.0 / 16.0;

            let vert_idx = self.vertices.len() as u16;

            // Create quad for this character
            self.vertices.push(GlyphVertex {
                position: [cursor_x, cursor_y],
                tex_coords: [tex_x, tex_y],
                color,
            });
            self.vertices.push(GlyphVertex {
                position: [cursor_x + size, cursor_y],
                tex_coords: [tex_x + tex_w, tex_y],
                color,
            });
            self.vertices.push(GlyphVertex {
                position: [cursor_x + size, cursor_y + size],
                tex_coords: [tex_x + tex_w, tex_y + tex_h],
                color,
            });
            self.vertices.push(GlyphVertex {
                position: [cursor_x, cursor_y + size],
                tex_coords: [tex_x, tex_y + tex_h],
                color,
            });

            self.indices.extend_from_slice(&[
                vert_idx, vert_idx + 1, vert_idx + 2,
                vert_idx + 2, vert_idx + 3, vert_idx,
            ]);

            cursor_x += size * 0.6; // Character spacing
        }
    }

    pub fn flush(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        camera: &mut Camera,
    ) {
        if self.vertices.is_empty() {
            return;
        }

        // Update uniform
        let view_proj = camera.get_view_projection_matrix();
        let uniform = TextUniform {
            view_proj: view_proj.to_cols_array_2d(),
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));

        // Upload vertices and indices
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices));

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Text Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_bind_group(1, &self.font_texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);

        drop(render_pass);

        self.vertices.clear();
        self.indices.clear();
    }
}