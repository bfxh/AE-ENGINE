//! GPU 设备与上下文抽象

use wgpu::{Adapter, Device, Instance, Queue};

/// 渲染上下文：持有 wgpu 的核心对象
pub struct RenderContext {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
}

#[derive(Debug)]
pub enum RenderContextError {
    NoAdapter,
    RequestDevice(wgpu::RequestDeviceError),
}

impl std::fmt::Display for RenderContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoAdapter => write!(f, "no suitable GPU adapter found"),
            Self::RequestDevice(e) => write!(f, "request_device failed: {e}"),
        }
    }
}

impl std::error::Error for RenderContextError {}

impl RenderContext {
    /// 创建无 Surface 的 headless 上下文（用于离屏渲染、测试）
    pub async fn new_headless() -> Result<Self, RenderContextError> {
        let instance = Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RenderContextError::NoAdapter)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("ae_render device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    ..Default::default()
                },
                None,
            )
            .await
            .map_err(RenderContextError::RequestDevice)?;

        Ok(Self { instance, adapter, device, queue })
    }

    /// 从已有 instance/adapter 创建（用于复用 window 的 instance）
    pub async fn from_adapter(adapter: Adapter) -> Result<Self, RenderContextError> {
        let instance = Instance::default();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("ae_render device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    ..Default::default()
                },
                None,
            )
            .await
            .map_err(RenderContextError::RequestDevice)?;
        Ok(Self { instance, adapter, device, queue })
    }
}
