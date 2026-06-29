//! Shader 资源

use crate::core::Handle;

/// Shader 源码
#[derive(Debug, Clone)]
pub enum ShaderSource {
    Wgsl(String),
    SpirV(Vec<u32>),
}

/// Shader
pub struct Shader {
    pub source: ShaderSource,
    pub module: wgpu::ShaderModule,
}

pub type ShaderHandle = Handle<Shader>;

impl Shader {
    pub fn from_wgsl(device: &wgpu::Device, source: impl Into<String>, label: Option<&str>) -> Self {
        let source_str: String = source.into();
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label,
            source: wgpu::ShaderSource::Wgsl(source_str.clone().into()),
        });
        Self { source: ShaderSource::Wgsl(source_str), module }
    }
}