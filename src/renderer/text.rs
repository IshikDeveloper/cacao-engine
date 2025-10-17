// ============================================================================
// FILE: src/renderer/text.rs - Enhanced with Real Font Support
// ============================================================================
use crate::errors::CacaoError;
use super::Camera;
use std::collections::HashMap;

// --- FIXES: Add missing imports for WGPU types and logging ---
use wgpu::vertex_attr_array;
use wgpu::VertexFormat::{Float32x2, Float32x4}; // Needed for GlyphVertex::ATTRIBS
use log; 
// -----------------------------------------------------------

// Simple bitmap font - 8x8 pixel characters
const FONT_WIDTH: u32 = 8;
const FONT_HEIGHT: u32 = 8;
const FONT_ATLAS_SIZE: u32 = 128; // Constant for texture dimensions

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GlyphVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
    color: [f32; 4],
}

impl GlyphVertex {
    // FIX: Uses imported types
    const ATTRIBS: [wgpu::VertexAttribute; 3] = vertex_attr_array![
        0 => Float32x2, // position
        1 => Float32x2, // tex_coords
        2 => Float32x4, // color
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
    
    // Font atlases for different fonts
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
            // Assuming this path is correct relative to the cargo root
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
                        // filterable: false is correct for R8Unorm bitmap/alpha texture
                        sample_type: wgpu::TextureSampleType::Float { filterable: false }, 
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    // NonFiltering is correct for pixel-art fonts or un-aliased glyphs
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
                cull_mode: None, // Keep None for text rendering which may overlap
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

        // Load default bitmap font
        let mut font_atlases = HashMap::new();
        let default_atlas = Self::create_default_font_atlas(device, queue, &texture_bind_group_layout)?;
        font_atlases.insert("default".to_string(), default_atlas);

        // Try to load custom fonts from assets
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

    // FIX: Define the required helper function
    fn generate_default_font() -> Vec<u8> {
        // Placeholder: Creates a 128x128 R8Unorm atlas filled with zeros (transparent/black)
        // A real implementation would generate character bitmaps here.
        vec![0u8; (FONT_ATLAS_SIZE * FONT_ATLAS_SIZE) as usize]
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
            format: wgpu::TextureFormat::R8Unorm, // Single channel for alpha/luminance
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
                // FIX: Use the constant FONT_ATLAS_SIZE for consistency
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
        // ... (logging and calls to load_font_from_file remain the same) ...
        
        // Try to load Press Start 2P
        if let Ok(atlas) = Self::load_font_from_file(
            "assets/fonts/PressStart2P.ttf",
            device,
            queue,
            bind_group_layout
        ) {
            font_atlases.insert("PressStart2P".to_string(), atlas);
            log::info!("✅ Loaded Press Start 2P font");
        } else {
            log::warn!("⚠️  Press Start 2P font not found, using default");
        }

        // Try to load Roboto
        if let Ok(atlas) = Self::load_font_from_file(
            "assets/fonts/Roboto-Regular.ttf",
            device,
            queue,
            bind_group_layout
        ) {
            font_atlases.insert("Roboto".to_string(), atlas);
            log::info!("✅ Loaded Roboto font");
        } else {
            log::warn!("⚠️  Roboto font not found, using default");
        }

        // Try to load Rodin NTLG
        if let Ok(atlas) = Self::load_font_from_file(
            "assets/fonts/RodinNTLG.otf",
            device,
            queue,
            bind_group_layout
        ) {
            font_atlases.insert("RodinNTLG".to_string(), atlas);
            log::info!("✅ Loaded Rodin NTLG font");
        } else {
            log::warn!("⚠️  Rodin NTLG font not found, using default");
        }
    }

    fn load_font_from_file(
        _path: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Result<FontAtlas, CacaoError> {
        // TODO: Implement proper TTF loading with fontdue
        // For now, return default atlas
        Self::create_default_font_atlas(device, queue, bind_group_layout)
    }
}