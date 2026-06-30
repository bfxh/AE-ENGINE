//! PBR 材质系统

use crate::texture::Texture;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::{BindGroup, BindGroupLayout, Device};

bitflags::bitflags! {
    /// 材质标志位
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct MaterialFlags: u32 {
        const HAS_BASE_COLOR_TEXTURE = 1 << 0;
        const HAS_NORMAL_TEXTURE = 1 << 1;
        const HAS_METALLIC_ROUGHNESS_TEXTURE = 1 << 2;
        const HAS_OCCLUSION_TEXTURE = 1 << 3;
        const HAS_EMISSIVE_TEXTURE = 1 << 4;
        const TWO_SIDED = 1 << 5;
        const ALPHA_BLEND = 1 << 6;
        const ALPHA_MASK = 1 << 7;
        const UNLIT = 1 << 8;
    }
}

/// PBR 材质参数（GPU 上传用，必须 16 字节对齐）
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct PbrMaterialParams {
    /// 漫反射颜色 (sRGB linear)
    pub base_color: [f32; 4],
    /// x: metallic, y: roughness, z: emissive_strength, w: alpha_cutoff
    pub metallic_roughness: [f32; 4],
    /// 自发光颜色 (linear)
    pub emissive: [f32; 4],
    /// 标志位（MaterialFlags bits）
    pub flags: u32,
    /// 法线缩放
    pub normal_scale: f32,
    /// AO 强度
    pub occlusion_strength: f32,
    /// 保留对齐
    pub _pad: f32,
}

impl PbrMaterialParams {
    pub fn new() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic_roughness: [0.0, 0.5, 1.0, 0.5],
            emissive: [0.0; 4],
            flags: 0,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            _pad: 0.0,
        }
    }

    pub fn with_base_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.base_color = [r, g, b, a];
        self
    }

    pub fn with_metallic(mut self, m: f32) -> Self {
        self.metallic_roughness[0] = m;
        self
    }

    pub fn with_roughness(mut self, r: f32) -> Self {
        self.metallic_roughness[1] = r;
        self
    }

    pub fn with_emissive(mut self, r: f32, g: f32, b: f32, strength: f32) -> Self {
        self.emissive = [r, g, b, 0.0];
        self.metallic_roughness[2] = strength;
        self
    }

    pub fn set_flag(&mut self, flag: MaterialFlags, on: bool) {
        if on {
            self.flags |= flag.bits();
        } else {
            self.flags &= !flag.bits();
        }
    }
}

/// PBR 材质（CPU 端描述 + GPU 资源）
pub struct PbrMaterial {
    pub params: PbrMaterialParams,
    pub base_color_texture: Option<Arc<Texture>>,
    pub normal_texture: Option<Arc<Texture>>,
    pub metallic_roughness_texture: Option<Arc<Texture>>,
    pub occlusion_texture: Option<Arc<Texture>>,
    pub emissive_texture: Option<Arc<Texture>>,
    pub bind_group: Option<BindGroup>,
}

impl PbrMaterial {
    pub fn new(params: PbrMaterialParams) -> Self {
        Self {
            params,
            base_color_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
            occlusion_texture: None,
            emissive_texture: None,
            bind_group: None,
        }
    }

    pub fn with_base_color_texture(mut self, tex: Arc<Texture>) -> Self {
        self.params.set_flag(MaterialFlags::HAS_BASE_COLOR_TEXTURE, true);
        self.base_color_texture = Some(tex);
        self
    }

    pub fn with_normal_texture(mut self, tex: Arc<Texture>) -> Self {
        self.params.set_flag(MaterialFlags::HAS_NORMAL_TEXTURE, true);
        self.normal_texture = Some(tex);
        self
    }

    pub fn with_metallic_roughness_texture(mut self, tex: Arc<Texture>) -> Self {
        self.params.set_flag(MaterialFlags::HAS_METALLIC_ROUGHNESS_TEXTURE, true);
        self.metallic_roughness_texture = Some(tex);
        self
    }

    pub fn with_occlusion_texture(mut self, tex: Arc<Texture>) -> Self {
        self.params.set_flag(MaterialFlags::HAS_OCCLUSION_TEXTURE, true);
        self.occlusion_texture = Some(tex);
        self
    }

    pub fn with_emissive_texture(mut self, tex: Arc<Texture>) -> Self {
        self.params.set_flag(MaterialFlags::HAS_EMISSIVE_TEXTURE, true);
        self.emissive_texture = Some(tex);
        self
    }

    pub fn two_sided(mut self, on: bool) -> Self {
        self.params.set_flag(MaterialFlags::TWO_SIDED, on);
        self
    }

    pub fn alpha_blend(mut self, on: bool) -> Self {
        self.params.set_flag(MaterialFlags::ALPHA_BLEND, on);
        self
    }

    pub fn alpha_mask(mut self, on: bool) -> Self {
        self.params.set_flag(MaterialFlags::ALPHA_MASK, on);
        self
    }

    pub fn unlit(mut self, on: bool) -> Self {
        self.params.set_flag(MaterialFlags::UNLIT, on);
        self
    }
}

/// 材质类型枚举（用于管线选择）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaterialType {
    Pbr,
    Unlit,
    Wireframe,
    Sky,
    Terrain,
    PostProcess,
}

/// 材质 trait（未来扩展用）
pub trait Material: Send + Sync {
    fn material_type(&self) -> MaterialType;
    fn flags(&self) -> MaterialFlags;
}

impl Material for PbrMaterial {
    fn material_type(&self) -> MaterialType {
        if self.params.flags & MaterialFlags::UNLIT.bits() != 0 {
            MaterialType::Unlit
        } else {
            MaterialType::Pbr
        }
    }
    fn flags(&self) -> MaterialFlags {
        MaterialFlags::from_bits_truncate(self.params.flags)
    }
}

/// 创建材质 BindGroupLayout（5 个纹理 + 1 个 uniform）
pub fn create_material_bind_group_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("pbr material layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 5,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 6,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<PbrMaterialParams>() as u64,
                    ),
                },
                count: None,
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_params_default() {
        let p = PbrMaterialParams::new();
        assert_eq!(p.base_color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(p.metallic_roughness[0], 0.0);
        assert_eq!(p.metallic_roughness[1], 0.5);
        assert_eq!(p.flags, 0);
    }

    #[test]
    fn material_flags_bitops() {
        let mut p = PbrMaterialParams::new();
        p.set_flag(MaterialFlags::HAS_BASE_COLOR_TEXTURE, true);
        p.set_flag(MaterialFlags::TWO_SIDED, true);
        assert!(p.flags & MaterialFlags::HAS_BASE_COLOR_TEXTURE.bits() != 0);
        assert!(p.flags & MaterialFlags::TWO_SIDED.bits() != 0);
        assert!(p.flags & MaterialFlags::ALPHA_BLEND.bits() == 0);

        p.set_flag(MaterialFlags::TWO_SIDED, false);
        assert!(p.flags & MaterialFlags::TWO_SIDED.bits() == 0);
    }

    #[test]
    fn material_builder_chain() {
        // 只测试 flag 设置，不实际创建 Texture（避免 GPU 资源）
        let mut m = PbrMaterial::new(PbrMaterialParams::new());
        m.params.set_flag(MaterialFlags::HAS_BASE_COLOR_TEXTURE, true);
        m = m.two_sided(true).alpha_blend(true);
        assert!(m.params.flags & MaterialFlags::HAS_BASE_COLOR_TEXTURE.bits() != 0);
        assert!(m.params.flags & MaterialFlags::TWO_SIDED.bits() != 0);
        assert!(m.params.flags & MaterialFlags::ALPHA_BLEND.bits() != 0);
    }

    #[test]
    fn material_type_from_unlit_flag() {
        let mut m = PbrMaterial::new(PbrMaterialParams::new());
        assert_eq!(m.material_type(), MaterialType::Pbr);
        m = m.unlit(true);
        assert_eq!(m.material_type(), MaterialType::Unlit);
    }

    #[test]
    fn params_size_aligned() {
        // 16+16+16+4+4+4+4 = 64 bytes
        assert_eq!(std::mem::size_of::<PbrMaterialParams>(), 64);
    }
}
