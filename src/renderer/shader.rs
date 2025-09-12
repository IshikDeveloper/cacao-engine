// src/renderer/shader.rs
use wgpu::ShaderModuleDescriptor;
use crate::errors::CacaoError;

pub struct ShaderManager {
    device: wgpu::Device,
}

impl ShaderManager {
    pub fn new(device: wgpu::Device) -> Self {
        Self { device }
    }

    pub fn create_shader_from_source(&self, source: &str, label: Option<&str>) -> Result<wgpu::ShaderModule, CacaoError> {
        let shader = self.device.create_shader_module(ShaderModuleDescriptor {
            label,
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        Ok(shader)
    }
}