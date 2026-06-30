//! Surface renderer: manages wgpu surface, swapchain, depth buffer.
//!
//! Encapsulates the window-surface-device-queue-config boilerplate so that
//! the application code only needs to call `render_frame()` with a draw callback.

use wgpu::{Adapter, Device, Instance, Queue, Surface, SurfaceConfiguration, TextureView};

/// Clear color (dark blue-gray).
pub const CLEAR_COLOR: wgpu::Color = wgpu::Color { r: 0.1, g: 0.1, b: 0.15, a: 1.0 };

/// Surface renderer: owns the wgpu surface, device, queue, and depth buffer.
///
/// Usage:
/// ```ignore
/// let mut sr = SurfaceRenderer::new(&window).await;
/// sr.resize(width, height);
///
/// // Each frame:
/// sr.render_frame(|pass, view| {
///     // draw using pass (RenderPass) and view (TextureView)
/// });
/// ```
pub struct SurfaceRenderer {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub config: SurfaceConfiguration,
    pub depth_format: wgpu::TextureFormat,
    pub depth_view: TextureView,
}

impl SurfaceRenderer {
    /// Create from a winit window. Acquires high-performance GPU adapter.
    pub async fn new(window: &winit::window::Window) -> Self {
        let instance = wgpu::Instance::default();

        // SAFETY: Surface<'_> borrows window; we extend to 'static because
        // the window lives as long as SurfaceRenderer (both owned by App).
        let surface = instance.create_surface(window).expect("failed to create surface");
        let surface: Surface<'static> = unsafe { std::mem::transmute(surface) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("no suitable GPU adapter found");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("ae_render surface device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    ..Default::default()
                },
                None,
            )
            .await
            .expect("failed to request device");

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let depth_format = wgpu::TextureFormat::Depth32Float;
        let depth_view = create_depth_view(&device, &config, depth_format);

        Self {
            instance,
            adapter,
            device,
            queue,
            surface,
            config,
            depth_format,
            depth_view,
        }
    }

    /// Get the current surface color format.
    pub fn color_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// Resize the surface and recreate the depth buffer.
    /// Call this when the window is resized.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.depth_view = create_depth_view(&self.device, &self.config, self.depth_format);
    }

    /// Render one frame. The `draw` callback receives a `RenderPass` (already configured
    /// with clear color + depth) and the surface `TextureView`.
    ///
    /// Handles surface loss/outdated automatically (reconfigures and returns without drawing).
    pub fn render_frame<F>(&self, draw: F)
    where
        F: FnOnce(&mut wgpu::RenderPass<'_>),
    {
        let output = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(_) => return,
        };
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("surface render encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("surface render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            draw(&mut pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}

/// Create a depth texture view matching the surface configuration.
fn create_depth_view(
    device: &Device,
    config: &SurfaceConfiguration,
    format: wgpu::TextureFormat,
) -> TextureView {
    let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth texture"),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    depth_texture.create_view(&wgpu::TextureViewDescriptor::default())
}
