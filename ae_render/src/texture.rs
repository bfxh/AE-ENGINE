//! 纹理系统：PNG / DDS / KTX2 加载、采样、绑定

use std::path::Path;
use std::sync::Arc;
use wgpu::{Device, Queue, Sampler, Texture as WgpuTexture, TextureView};

/// 纹理格式描述
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    R8Unorm,
    Rg8Unorm,
    Rgba8Unorm,
    Rgba8UnormSrgb,
    R16Float,
    Rg16Float,
    Rgba16Float,
    R32Float,
    Bc1RgbaUnorm,
    Bc1RgbaUnormSrgb,
    Bc3RgbaUnorm,
    Bc3RgbaUnormSrgb,
    Bc5RgUnorm,
    Bc7RgbaUnorm,
    Bc7RgbaUnormSrgb,
}

impl TextureFormat {
    pub fn to_wgpu(self) -> wgpu::TextureFormat {
        match self {
            Self::R8Unorm => wgpu::TextureFormat::R8Unorm,
            Self::Rg8Unorm => wgpu::TextureFormat::Rg8Unorm,
            Self::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
            Self::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
            Self::R16Float => wgpu::TextureFormat::R16Float,
            Self::Rg16Float => wgpu::TextureFormat::Rg16Float,
            Self::Rgba16Float => wgpu::TextureFormat::Rgba16Float,
            Self::R32Float => wgpu::TextureFormat::R32Float,
            Self::Bc1RgbaUnorm => wgpu::TextureFormat::Bc1RgbaUnorm,
            Self::Bc1RgbaUnormSrgb => wgpu::TextureFormat::Bc1RgbaUnormSrgb,
            Self::Bc3RgbaUnorm => wgpu::TextureFormat::Bc3RgbaUnorm,
            Self::Bc3RgbaUnormSrgb => wgpu::TextureFormat::Bc3RgbaUnormSrgb,
            Self::Bc5RgUnorm => wgpu::TextureFormat::Bc5RgUnorm,
            Self::Bc7RgbaUnorm => wgpu::TextureFormat::Bc7RgbaUnorm,
            Self::Bc7RgbaUnormSrgb => wgpu::TextureFormat::Bc7RgbaUnormSrgb,
        }
    }

    /// 该格式是否 sRGB
    pub fn is_srgb(self) -> bool {
        matches!(
            self,
            Self::Rgba8UnormSrgb
                | Self::Bc1RgbaUnormSrgb
                | Self::Bc3RgbaUnormSrgb
                | Self::Bc7RgbaUnormSrgb
        )
    }

    /// 像素大小（字节），压缩格式返回块大小估算
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            Self::R8Unorm => 1,
            Self::Rg8Unorm => 2,
            Self::Rgba8Unorm | Self::Rgba8UnormSrgb => 4,
            Self::R16Float => 2,
            Self::Rg16Float => 4,
            Self::Rgba16Float => 8,
            Self::R32Float => 4,
            // BC 压缩格式每 4x4 块 8 或 16 字节，按像素返回近似值
            Self::Bc1RgbaUnorm | Self::Bc1RgbaUnormSrgb => 1,
            Self::Bc3RgbaUnorm | Self::Bc3RgbaUnormSrgb | Self::Bc5RgUnorm => 1,
            Self::Bc7RgbaUnorm | Self::Bc7RgbaUnormSrgb => 1,
        }
    }
}

/// 纹理用途
#[derive(Debug, Clone, Copy, Default)]
pub struct TextureUsage {
    pub sampled: bool,
    pub storage: bool,
    pub render_target: bool,
}

impl TextureUsage {
    pub fn sampled() -> Self {
        Self { sampled: true, ..Default::default() }
    }
    pub fn render_target() -> Self {
        Self { render_target: true, sampled: true, ..Default::default() }
    }
    pub fn to_wgpu(self) -> wgpu::TextureUsages {
        let mut u = wgpu::TextureUsages::COPY_DST;
        if self.sampled {
            u |= wgpu::TextureUsages::TEXTURE_BINDING;
        }
        if self.storage {
            u |= wgpu::TextureUsages::STORAGE_BINDING;
        }
        if self.render_target {
            u |= wgpu::TextureUsages::RENDER_ATTACHMENT;
        }
        u
    }
}

/// 已上传到 GPU 的纹理资源
pub struct Texture {
    pub raw: WgpuTexture,
    pub view: TextureView,
    pub format: TextureFormat,
    pub width: u32,
    pub height: u32,
    pub mip_levels: u32,
}

impl Texture {
    /// 从原始像素数据创建 2D 纹理
    pub fn from_pixels(
        device: &Device,
        queue: &Queue,
        pixels: &[u8],
        format: TextureFormat,
        width: u32,
        height: u32,
        label: Option<&str>,
    ) -> Self {
        let wgpu_format = format.to_wgpu();
        let size = wgpu::Extent3d { width, height: height.max(1), depth_or_array_layers: 1 };
        let raw = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu_format,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &raw,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * format.bytes_per_pixel() as u32),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = raw.create_view(&wgpu::TextureViewDescriptor::default());
        Self { raw, view, format, width, height, mip_levels: 1 }
    }

    /// 创建默认采样器
    pub fn default_sampler(device: &Device) -> Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ae default sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            compare: None,
            anisotropy_clamp: 4,
            border_color: None,
        })
    }
}

/// 纹理加载器：支持 PNG/JPEG/BMP/TIFF（通过 image crate）和 DDS
pub struct TextureLoader;

#[derive(Debug)]
pub enum TextureLoadError {
    Io(std::io::Error),
    Image(String),
    Dds(String),
    UnsupportedFormat,
}

impl std::fmt::Display for TextureLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Image(e) => write!(f, "image decode error: {e}"),
            Self::Dds(e) => write!(f, "dds decode error: {e}"),
            Self::UnsupportedFormat => write!(f, "unsupported texture format"),
        }
    }
}

impl std::error::Error for TextureLoadError {}

/// 已解码的 CPU 端纹理数据
pub struct DecodedTexture {
    pub pixels: Vec<u8>,
    pub format: TextureFormat,
    pub width: u32,
    pub height: u32,
}

impl TextureLoader {
    /// 从文件加载并解码（根据扩展名选择解码器）
    pub fn load(path: &Path) -> Result<DecodedTexture, TextureLoadError> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();

        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "bmp" | "tif" | "tiff" => Self::load_image(path),
            "dds" => Self::load_dds(path),
            _ => Err(TextureLoadError::UnsupportedFormat),
        }
    }

    /// 通过 image crate 加载常见格式
    pub fn load_image(path: &Path) -> Result<DecodedTexture, TextureLoadError> {
        let img = image::open(path).map_err(|e| TextureLoadError::Image(e.to_string()))?;
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        Ok(DecodedTexture {
            pixels: rgba.into_raw(),
            format: TextureFormat::Rgba8UnormSrgb,
            width: w,
            height: h,
        })
    }

    /// 加载 DDS 文件（支持 BC1/BC3/BC7 压缩和 RGBA8 未压缩）
    pub fn load_dds(path: &Path) -> Result<DecodedTexture, TextureLoadError> {
        let file = std::fs::File::open(path).map_err(TextureLoadError::Io)?;
        let dds = ddsfile::Dds::read(file).map_err(|e| TextureLoadError::Dds(e.to_string()))?;

        let (format, _is_compressed) = match dds.get_dxgi_format() {
            Some(ddsfile::DxgiFormat::R8G8B8A8_UNorm) => (TextureFormat::Rgba8Unorm, false),
            Some(ddsfile::DxgiFormat::R8G8B8A8_UNorm_sRGB) => {
                (TextureFormat::Rgba8UnormSrgb, false)
            },
            Some(ddsfile::DxgiFormat::BC1_UNorm) => (TextureFormat::Bc1RgbaUnorm, true),
            Some(ddsfile::DxgiFormat::BC1_UNorm_sRGB) => (TextureFormat::Bc1RgbaUnormSrgb, true),
            Some(ddsfile::DxgiFormat::BC3_UNorm) => (TextureFormat::Bc3RgbaUnorm, true),
            Some(ddsfile::DxgiFormat::BC3_UNorm_sRGB) => (TextureFormat::Bc3RgbaUnormSrgb, true),
            Some(ddsfile::DxgiFormat::BC7_UNorm) => (TextureFormat::Bc7RgbaUnorm, true),
            Some(ddsfile::DxgiFormat::BC7_UNorm_sRGB) => (TextureFormat::Bc7RgbaUnormSrgb, true),
            _ => return Err(TextureLoadError::Dds("unsupported DDS format".into())),
        };

        let (w, h) = (dds.get_width(), dds.get_height());
        let pixels = dds.get_data(0).map_err(|e| TextureLoadError::Dds(e.to_string()))?.to_vec();

        Ok(DecodedTexture { pixels, format, width: w, height: h })
    }

    /// 将已解码纹理上传到 GPU
    pub fn upload(
        device: &Device,
        queue: &Queue,
        decoded: &DecodedTexture,
        label: Option<&str>,
    ) -> Texture {
        Texture::from_pixels(
            device,
            queue,
            &decoded.pixels,
            decoded.format,
            decoded.width,
            decoded.height,
            label,
        )
    }
}

/// 纹理缓存（按路径键），避免重复加载
pub struct TextureCache {
    cache: hashbrown::HashMap<std::path::PathBuf, Arc<Texture>>,
}

impl Default for TextureCache {
    fn default() -> Self {
        Self { cache: hashbrown::HashMap::new() }
    }
}

impl TextureCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, path: &Path) -> Option<Arc<Texture>> {
        self.cache.get(path).cloned()
    }

    pub fn insert(&mut self, path: std::path::PathBuf, tex: Arc<Texture>) {
        self.cache.insert(path, tex);
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_format_srgb_detection() {
        assert!(TextureFormat::Rgba8UnormSrgb.is_srgb());
        assert!(!TextureFormat::Rgba8Unorm.is_srgb());
        assert!(TextureFormat::Bc7RgbaUnormSrgb.is_srgb());
        assert!(!TextureFormat::Bc7RgbaUnorm.is_srgb());
    }

    #[test]
    fn texture_format_bytes_per_pixel() {
        assert_eq!(TextureFormat::R8Unorm.bytes_per_pixel(), 1);
        assert_eq!(TextureFormat::Rgba8Unorm.bytes_per_pixel(), 4);
        assert_eq!(TextureFormat::Rgba16Float.bytes_per_pixel(), 8);
    }

    #[test]
    fn texture_usage_flags() {
        let u = TextureUsage::sampled();
        assert!(u.sampled);
        assert!(!u.storage);
        assert!(!u.render_target);

        let u = TextureUsage::render_target();
        assert!(u.render_target);
        assert!(u.sampled);
    }

    #[test]
    fn texture_cache_basic() {
        let cache = TextureCache::new();
        assert!(cache.is_empty());
        assert!(cache.get(std::path::Path::new("foo.png")).is_none());
    }

    #[test]
    fn unsupported_format_rejected() {
        let err = TextureLoader::load(std::path::Path::new("foo.xyz"));
        assert!(matches!(err, Err(TextureLoadError::UnsupportedFormat)));
    }
}
