// ============================================================================
// FILE: src/renderer/sprite.rs - FIXED
// ============================================================================
use wgpu::util::DeviceExt;
use crate::{errors::CacaoError, renderer::Camera};
use super::Texture;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SpriteVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

impl SpriteVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
    ];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SpriteUniform {
    view_proj: [[f32; 4]; 4],
    transform: [[f32; 4]; 4],
    color: [f32; 4],
}

pub struct Sprite {
    pub texture: Texture,
    pub width: f32,
    pub height: f32,
}

impl Sprite {
    pub fn new(texture: Texture) -> Self {
        Self {
            width: texture.width() as f32,
            height: texture.height() as f32,
            texture,
        }
    }
}

struct SpriteDrawCall {
    texture: Texture,
    transform: glam::Mat4,
    color: [f32; 4],
}

pub struct SpriteRenderer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    sprite_queue: Vec<SpriteDrawCall>,
}

impl SpriteRenderer {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Result<Self, CacaoError> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sprite Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/sprite.wgsl").into()),
        });

        let quad_vertices = vec![
            SpriteVertex { position: [-0.5, -0.5], tex_coords: [0.0, 1.0] },
            SpriteVertex { position: [ 0.5, -0.5], tex_coords: [1.0, 1.0] },
            SpriteVertex { position: [ 0.5,  0.5], tex_coords: [1.0, 0.0] },
            SpriteVertex { position: [-0.5,  0.5], tex_coords: [0.0, 0.0] },
        ];

        let quad_indices: Vec<u16> = vec![0, 1, 2, 2, 3, 0];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Vertex Buffer"),
            contents: bytemuck::cast_slice(&quad_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Index Buffer"),
            contents: bytemuck::cast_slice(&quad_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sprite Uniform Buffer"),
            size: std::mem::size_of::<SpriteUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Sprite Uniform Bind Group Layout"),
        });

        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("Texture Bind Group Layout"),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sprite Render Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sprite Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[SpriteVertex::desc()],
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
                cull_mode: Some(wgpu::Face::Back),
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

        Ok(Self {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            uniform_bind_group_layout,
            texture_bind_group_layout,
            sprite_queue: Vec::new(),
        })
    }

    pub fn draw_sprite(
        &mut self, 
        sprite: &Sprite, 
        x: f32, 
        y: f32, 
        rotation: f32, 
        scale: f32, 
        _camera: &Camera
    ) {
        use glam::{Mat4, Vec3, Quat};
        
        let translation = Mat4::from_translation(Vec3::new(x, y, 0.0));
        let rotation_mat = Mat4::from_quat(Quat::from_rotation_z(rotation));
        let scale_mat = Mat4::from_scale(Vec3::new(
            sprite.width * scale,
            sprite.height * scale,
            1.0,
        ));
        
        let transform = translation * rotation_mat * scale_mat;
        
        self.sprite_queue.push(SpriteDrawCall {
            texture: sprite.texture.clone(),
            transform,
            color: [1.0, 1.0, 1.0, 1.0],
        });
    }

    // FIXED: Added device parameter and fixed bind group creation
    pub fn flush<'a>(
        &'a mut self,
        render_pass: &mut wgpu::RenderPass<'a>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera: &mut Camera,
    ) {
        if self.sprite_queue.is_empty() {
            return;
        }
        
        let view_proj = camera.get_view_projection_matrix();
        
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        
        for draw_call in &self.sprite_queue {
            let uniform = SpriteUniform {
                view_proj: view_proj.to_cols_array_2d(),
                transform: draw_call.transform.to_cols_array_2d(),
                color: draw_call.color,
            };
            
            queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));
            
            let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.uniform_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                }],
                label: Some("Sprite Uniform Bind Group"),
            });
            
            let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(draw_call.texture.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(draw_call.texture.sampler()),
                    },
                ],
                label: Some("Sprite Texture Bind Group"),
            });
            
            render_pass.set_bind_group(0, &uniform_bind_group, &[]);
            render_pass.set_bind_group(1, &texture_bind_group, &[]);
            render_pass.draw_indexed(0..6, 0, 0..1);
        }
        
        self.sprite_queue.clear();
    }
}