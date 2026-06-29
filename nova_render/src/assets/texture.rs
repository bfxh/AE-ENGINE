//! Texture 资源

use crate::core::Handle;

/// CPU 侧 Texture 数据
pub struct TextureData {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
}

/// GPU 侧 Texture
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub format: wgpu::TextureFormat,
    pub width: u32,
    pub height: u32,
}

pub type TextureHandle = Handle<Texture>;

impl Texture {
    pub fn from_data(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &TextureData,
        label: Option<&str>,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d { width: data.width, height: data.height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: data.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &data.pixels,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(data.width * data.format.block_copy_size(None).unwrap_or(4)), rows_per_image: Some(data.height) },
            wgpu::Extent3d { width: data.width, height: data.height, depth_or_array_layers: 1 },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { texture, view, format: data.format, width: data.width, height: data.height }
    }
}