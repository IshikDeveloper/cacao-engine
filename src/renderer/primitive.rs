// src/renderer/primitive.rs - FIXED SIGNATURE
use wgpu::util::DeviceExt;
use crate::errors::CacaoError;
use super::Camera;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PrimitiveVertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl PrimitiveVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x4,
    ];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<PrimitiveVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PrimitiveUniform {
    view_proj: [[f32; 4]; 4],
}

pub struct PrimitiveRenderer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    
    vertices: Vec<PrimitiveVertex>,
    indices: Vec<u16>,
    max_primitives: usize,
}

impl PrimitiveRenderer {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> Result<Self, CacaoError> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Primitive Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/primitive.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Primitive Uniform Buffer"),
            size: std::mem::size_of::<PrimitiveUniform>() as u64,
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
            label: Some("Primitive Uniform Bind Group Layout"),
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("Primitive Uniform Bind Group"),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Primitive Render Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Primitive Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[PrimitiveVertex::desc()],
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

        let max_primitives = 2048;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Primitive Vertex Buffer"),
            size: (max_primitives * 4 * std::mem::size_of::<PrimitiveVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Primitive Index Buffer"),
            size: (max_primitives * 6 * std::mem::size_of::<u16>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            uniform_bind_group,
            vertices: Vec::new(),
            indices: Vec::new(),
            max_primitives,
        })
    }

    pub fn draw_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        let vert_idx = self.vertices.len() as u16;

        self.vertices.push(PrimitiveVertex { position: [x, y], color });
        self.vertices.push(PrimitiveVertex { position: [x + width, y], color });
        self.vertices.push(PrimitiveVertex { position: [x + width, y + height], color });
        self.vertices.push(PrimitiveVertex { position: [x, y + height], color });

        self.indices.extend_from_slice(&[
            vert_idx, vert_idx + 1, vert_idx + 2,
            vert_idx + 2, vert_idx + 3, vert_idx,
        ]);
    }

    pub fn draw_rect_outline(&mut self, x: f32, y: f32, width: f32, height: f32, thickness: f32, color: [f32; 4]) {
        self.draw_rect(x, y, width, thickness, color);
        self.draw_rect(x, y + height - thickness, width, thickness, color);
        self.draw_rect(x, y + thickness, thickness, height - 2.0 * thickness, color);
        self.draw_rect(x + width - thickness, y + thickness, thickness, height - 2.0 * thickness, color);
    }

    pub fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: [f32; 4]) {
        let dx = x2 - x1;
        let dy = y2 - y1;
        let length = (dx * dx + dy * dy).sqrt();
        
        if length < 0.001 {
            return;
        }

        let angle = dy.atan2(dx);
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let half_thickness = thickness / 2.0;
        let perpx = -sin_a * half_thickness;
        let perpy = cos_a * half_thickness;

        let vert_idx = self.vertices.len() as u16;

        self.vertices.push(PrimitiveVertex { position: [x1 + perpx, y1 + perpy], color });
        self.vertices.push(PrimitiveVertex { position: [x2 + perpx, y2 + perpy], color });
        self.vertices.push(PrimitiveVertex { position: [x2 - perpx, y2 - perpy], color });
        self.vertices.push(PrimitiveVertex { position: [x1 - perpx, y1 - perpy], color });

        self.indices.extend_from_slice(&[
            vert_idx, vert_idx + 1, vert_idx + 2,
            vert_idx + 2, vert_idx + 3, vert_idx,
        ]);
    }

    pub fn draw_circle(&mut self, x: f32, y: f32, radius: f32, segments: u32, color: [f32; 4]) {
        if segments < 3 {
            return;
        }

        let center_idx = self.vertices.len() as u16;
        self.vertices.push(PrimitiveVertex { position: [x, y], color });

        for i in 0..=segments {
            let angle = 2.0 * std::f32::consts::PI * (i as f32) / (segments as f32);
            let px = x + radius * angle.cos();
            let py = y + radius * angle.sin();
            self.vertices.push(PrimitiveVertex { position: [px, py], color });

            if i > 0 {
                self.indices.extend_from_slice(&[
                    center_idx,
                    center_idx + i as u16,
                    center_idx + i as u16 + 1,
                ]);
            }
        }
    }

    pub fn draw_circle_outline(&mut self, x: f32, y: f32, radius: f32, segments: u32, thickness: f32, color: [f32; 4]) {
        if segments < 3 {
            return;
        }

        for i in 0..segments {
            let angle1 = 2.0 * std::f32::consts::PI * (i as f32) / (segments as f32);
            let angle2 = 2.0 * std::f32::consts::PI * ((i + 1) as f32) / (segments as f32);
            
            let x1 = x + radius * angle1.cos();
            let y1 = y + radius * angle1.sin();
            let x2 = x + radius * angle2.cos();
            let y2 = y + radius * angle2.sin();
            
            self.draw_line(x1, y1, x2, y2, thickness, color);
        }
    }

    pub fn draw_triangle(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, color: [f32; 4]) {
        let vert_idx = self.vertices.len() as u16;

        self.vertices.push(PrimitiveVertex { position: [x1, y1], color });
        self.vertices.push(PrimitiveVertex { position: [x2, y2], color });
        self.vertices.push(PrimitiveVertex { position: [x3, y3], color });

        self.indices.extend_from_slice(&[vert_idx, vert_idx + 1, vert_idx + 2]);
    }

    pub fn flush<'a>(
        &'a mut self,
        render_pass: &mut wgpu::RenderPass<'a>,
        queue: &wgpu::Queue,
        camera: &mut Camera,
    ) {
        if self.vertices.is_empty() {
            return;
        }

        if self.vertices.len() / 4 > self.max_primitives {
            self.vertices.truncate(self.max_primitives * 4);
            self.indices.truncate(self.max_primitives * 6);
        }

        let view_proj = camera.get_view_projection_matrix();
        let uniform = PrimitiveUniform {
            view_proj: view_proj.to_cols_array_2d(),
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));

        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices));

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);

        self.vertices.clear();
        self.indices.clear();
    }
}