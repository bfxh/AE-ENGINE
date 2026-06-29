//! Material Trait（借鉴 rend3）
//!
//! 设计：
//! - Material 作为 Trait，定义绑定布局和参数
//! - 不同材质（PBR / Unlit / Water / Custom）实现 trait
//! - MaterialInstance：材质实例（值存储）

use crate::core::Handle;
use crate::assets::shader::ShaderHandle;

/// Material Trait
pub trait Material: Send + Sync + 'static {
    /// 名称
    fn name(&self) -> &str;
    /// Shader Handle
    fn shader(&self) -> Option<ShaderHandle> { None }
    /// Bind Group Layout
    fn bind_group_layout(&self) -> Option<&wgpu::BindGroupLayout> { None }
    /// 是否透明
    fn transparent(&self) -> bool { false }
    /// 双面
    fn double_sided(&self) -> bool { false }
}

/// Material Handle
pub type MaterialHandle = Handle<dyn Material>;

/// Material Instance（值存储 + trait object）
pub struct MaterialInstance {
    pub material: Box<dyn Material>,
    pub params: Vec<u8>,
}

impl MaterialInstance {
    pub fn new(material: Box<dyn Material>) -> Self {
        Self { material, params: Vec::new() }
    }
}