//! GraphicsServer trait（借鉴 Fyrox）

use std::any::Any;

/// 后端能力
#[derive(Debug, Clone)]
pub struct ServerCaps {
    pub max_texture_size: u32,
    pub max_storage_buffer_size: u64,
    pub max_compute_workgroups: [u32; 3],
    pub supports_raytracing: bool,
    pub supports_mesh_shaders: bool,
    pub supports_bindless: bool,
    pub min_uniform_buffer_offset: u32,
}

/// 后端信息
#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub device_name: String,
    pub driver_name: String,
    pub driver_info: String,
    pub backend: String,
}

/// GPU 后端抽象 trait
///
/// 借鉴 Fyrox 的 GraphicsServer trait，但适配 wgpu 抽象
pub trait GraphicsServer: Send + Sync + 'static {
    /// 获取后端信息
    fn info(&self) -> &ServerInfo;

    /// 获取能力
    fn caps(&self) -> &ServerCaps;

    /// 转为 Any（用于向下转型）
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
