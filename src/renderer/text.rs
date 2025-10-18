// src/renderer/text.rs - FIXED LIFETIME ISSUE
use crate::errors::CacaoError;
use super::Camera;
use std::collections::HashMap;

const FONT_WIDTH: u32 = 8;
const FONT_HEIGHT: u32 = 8;
const FONT_ATLAS_SIZE: u32 = 128;

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

struct FontAtlas {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
}

pub struct TextRenderer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    
    font_atlases: HashMap<String, FontAtlas>,
    current_font: String,
    
    vertices: Vec<GlyphVertex>,
    indices: Vec<u16>,
    max_chars: usize,
    
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl TextRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
    ) -> Result<Self, CacaoError> {
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

        let mut font_atlases = HashMap::new();
        let default_atlas = Self::create_default_font_atlas(device, queue, &texture_bind_group_layout)?;
        font_atlases.insert("default".to_string(), default_atlas);

        Self::try_load_custom_fonts(device, queue, &texture_bind_group_layout, &mut font_atlases);

        Ok(Self {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            uniform_bind_group,
            font_atlases,
            current_font: "default".to_string(),
            vertices: Vec::new(),
            indices: Vec::new(),
            max_chars,
            texture_bind_group_layout,
        })
    }

    fn generate_default_font() -> Vec<u8> {
        let mut data = vec![0u8; (FONT_ATLAS_SIZE * FONT_ATLAS_SIZE) as usize];
        
        for ch in 32u8..127u8 {
            let idx = ch as usize;
            let row = idx / 16;
            let col = idx % 16;
            
            let char_x = col * FONT_WIDTH as usize;
            let char_y = row * FONT_HEIGHT as usize;
            
            if ch > 32 {
                for y in 0..FONT_HEIGHT as usize {
                    for x in 0..FONT_WIDTH as usize {
                        let atlas_x = char_x + x;
                        let atlas_y = char_y + y;
                        let atlas_idx = atlas_y * FONT_ATLAS_SIZE as usize + atlas_x;
                        
                        if x > 0 && x < 7 && y > 0 && y < 7 {
                            data[atlas_idx] = 255;
                        }
                    }
                }
            }
        }
        
        data
    }

    fn create_default_font_atlas(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Result<FontAtlas, CacaoError> {
        let size = wgpu::Extent3d {
            width: FONT_ATLAS_SIZE,
            height: FONT_ATLAS_SIZE,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Default Font Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let font_data = Self::generate_default_font();
        
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
                bytes_per_row: Some(FONT_ATLAS_SIZE),
                rows_per_image: Some(FONT_ATLAS_SIZE),
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("Default Font Bind Group"),
        });

        Ok(FontAtlas { texture, view, sampler, bind_group })
    }

    fn try_load_custom_fonts(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
        font_atlases: &mut HashMap<String, FontAtlas>,
    ) {
        if let Ok(atlas) = Self::load_font_from_file("assets/fonts/PressStart2P.ttf", device, queue, bind_group_layout) {
            font_atlases.insert("PressStart2P".to_string(), atlas);
        }

        if let Ok(atlas) = Self::load_font_from_file("assets/fonts/Roboto-Regular.ttf", device, queue, bind_group_layout) {
            font_atlases.insert("Roboto".to_string(), atlas);
        }

        if let Ok(atlas) = Self::load_font_from_file("assets/fonts/RodinNTLG.otf", device, queue, bind_group_layout) {
            font_atlases.insert("RodinNTLG".to_string(), atlas);
        }
    }

    fn load_font_from_file(
        _path: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Result<FontAtlas, CacaoError> {
        Self::create_default_font_atlas(device, queue, bind_group_layout)
    }

    pub fn set_font(&mut self, font_name: &str) {
        if self.font_atlases.contains_key(font_name) {
            self.current_font = font_name.to_string();
        }
    }

    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: [f32; 4]) {
        let char_width = size * 0.6;
        let char_height = size;
        
        let mut cursor_x = x;
        let cursor_y = y;

        for ch in text.chars() {
            if ch == '\n' {
                continue;
            }
            
            if ch == ' ' {
                cursor_x += char_width;
                continue;
            }

            let char_code = ch as u8;
            if char_code < 32 || char_code > 126 {
                cursor_x += char_width;
                continue;
            }

            let atlas_idx = char_code as usize;
            let atlas_row = atlas_idx / 16;
            let atlas_col = atlas_idx % 16;
            
            let u0 = (atlas_col * FONT_WIDTH as usize) as f32 / FONT_ATLAS_SIZE as f32;
            let v0 = (atlas_row * FONT_HEIGHT as usize) as f32 / FONT_ATLAS_SIZE as f32;
            let u1 = u0 + FONT_WIDTH as f32 / FONT_ATLAS_SIZE as f32;
            let v1 = v0 + FONT_HEIGHT as f32 / FONT_ATLAS_SIZE as f32;

            let vert_idx = self.vertices.len() as u16;

            self.vertices.push(GlyphVertex {
                position: [cursor_x, cursor_y],
                tex_coords: [u0, v0],
                color,
            });
            self.vertices.push(GlyphVertex {
                position: [cursor_x + char_width, cursor_y],
                tex_coords: [u1, v0],
                color,
            });
            self.vertices.push(GlyphVertex {
                position: [cursor_x + char_width, cursor_y + char_height],
                tex_coords: [u1, v1],
                color,
            });
            self.vertices.push(GlyphVertex {
                position: [cursor_x, cursor_y + char_height],
                tex_coords: [u0, v1],
                color,
            });

            self.indices.extend_from_slice(&[
                vert_idx, vert_idx + 1, vert_idx + 2,
                vert_idx + 2, vert_idx + 3, vert_idx,
            ]);

            cursor_x += char_width;
        }
    }

    // FIXED: Added proper lifetime annotation
    pub fn flush<'a>(
        &'a mut self,
        render_pass: &mut wgpu::RenderPass<'a>,
        queue: &wgpu::Queue,
        camera: &mut Camera,
    ) {
        if self.vertices.is_empty() {
            return;
        }

        if self.vertices.len() / 4 > self.max_chars {
            self.vertices.truncate(self.max_chars * 4);
            self.indices.truncate(self.max_chars * 6);
        }

        let view_proj = camera.get_view_projection_matrix();
        let uniform = TextUniform {
            view_proj: view_proj.to_cols_array_2d(),
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));

        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices));

        let font_atlas = self.font_atlases.get(&self.current_font).unwrap();

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_bind_group(1, &font_atlas.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);

        self.vertices.clear();
        self.indices.clear();
    }
}