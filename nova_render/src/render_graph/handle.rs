//! RenderGraph 资源 handle

use wgpu::TextureFormat;

/// 资源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Texture,
    Buffer,
    TextureView,
}

/// 资源描述（用于自动分配）
#[derive(Debug, Clone)]
pub struct ResourceDesc {
    pub ty: ResourceType,
    pub width: u32,
    pub height: u32,
    pub format: Option<TextureFormat>,
    pub usage: wgpu::TextureUsages,
    pub label: Option<String>,
    /// Buffer size（仅 ResourceType::Buffer 使用）
    pub buffer_size: u64,
    /// Buffer usage（仅 ResourceType::Buffer 使用）
    pub buffer_usage: wgpu::BufferUsages,
}

impl ResourceDesc {
    /// 创建 Texture 描述
    pub fn texture(width: u32, height: u32, format: TextureFormat, usage: wgpu::TextureUsages) -> Self {
        Self {
            ty: ResourceType::Texture,
            width, height,
            format: Some(format),
            usage,
            label: None,
            buffer_size: 0,
            buffer_usage: wgpu::BufferUsages::empty(),
        }
    }

    /// 创建 Buffer 描述
    pub fn buffer(size: u64, usage: wgpu::BufferUsages) -> Self {
        Self {
            ty: ResourceType::Buffer,
            width: 0, height: 0,
            format: None,
            usage: wgpu::TextureUsages::empty(),
            label: None,
            buffer_size: size,
            buffer_usage: usage,
        }
    }

    /// 设置 label（builder 风格）
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// 资源 Handle（在 RenderGraph 中引用资源）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceHandle(pub u64);

impl ResourceHandle {
    pub fn new(id: u64) -> Self { Self(id) }
}
