//! wgpu 后端实现

use super::server::{GraphicsServer, ServerCaps, ServerInfo};

/// wgpu 后端
pub struct WgpuBackend {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    info: ServerInfo,
    caps: ServerCaps,
}

impl WgpuBackend {
    /// 异步初始化
    pub async fn async_new(
        window: Option<&dyn raw_window_handle::HasWindowHandle>,
        backend: Option<wgpu::Backends>,
    ) -> anyhow::Result<Self> {
        let backends = backend.unwrap_or(wgpu::Backends::all());
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }).await.ok_or_else(|| anyhow::anyhow!("no suitable adapter"))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("nova_render device"),
                required_features: wgpu::Features::default()
                    | wgpu::Features::POLYGON_MODE_LINE
                    | wgpu::Features::POLYGON_MODE_POINT,
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            }, None)
            .await?;

        let info = ServerInfo {
            name: "wgpu".to_string(),
            device_name: adapter.get_info().name.clone(),
            driver_name: adapter.get_info().driver.clone(),
            driver_info: adapter.get_info().driver_info.clone(),
            backend: format!("{:?}", adapter.get_info().backend),
        };

        let caps = ServerCaps {
            max_texture_size: device.limits().max_texture_dimension_2d,
            max_storage_buffer_size: device.limits().max_storage_buffer_binding_size as u64,
            max_compute_workgroups: [
                device.limits().max_compute_workgroups_per_dimension,
                device.limits().max_compute_workgroups_per_dimension,
                device.limits().max_compute_workgroups_per_dimension,
            ],
            supports_raytracing: false,
            supports_mesh_shaders: false,
            supports_bindless: device.features().contains(wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING),
            min_uniform_buffer_offset: device.limits().min_uniform_buffer_offset_alignment,
        };

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            info,
            caps,
        })
    }

    pub fn instance(&self) -> &wgpu::Instance { &self.instance }
    pub fn adapter(&self) -> &wgpu::Adapter { &self.adapter }
    pub fn device(&self) -> &wgpu::Device { &self.device }
    pub fn queue(&self) -> &wgpu::Queue { &self.queue }
}

impl GraphicsServer for WgpuBackend {
    fn info(&self) -> &ServerInfo { &self.info }
    fn caps(&self) -> &ServerCaps { &self.caps }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
