//! nova_render 最小可运行示例：winit 窗口 + wgpu 三角形
//!
//! 编译运行：cargo run --example hello_triangle --target-dir d:\rj\wasteland_project\target2
//!
//! 这是 3A 标准 3D 渲染的最低门槛：能打开窗口、初始化 wgpu、画一个三角形。
//! 后续所有 pass（Forward/Shadow/SSAO/SSR/TAA/Bloom/VolumetricFog）都会挂到这个主循环上。

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

struct App {
    window: Option<Arc<Window>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    pipeline: Option<wgpu::RenderPipeline>,
}

impl App {
    fn new() -> Self {
        Self {
            window: None, device: None, queue: None,
            surface: None, surface_config: None, pipeline: None,
        }
    }

    async fn init_renderer(&mut self, window: Arc<Window>) {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.unwrap();

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("hello_triangle device"),
            required_features: wgpu::Features::default(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
        }, None).await.unwrap();

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("triangle shader"),
            source: wgpu::ShaderSource::Wgsl(r#"
@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>( 0.0,  0.5),
        vec2<f32>(-0.5, -0.5),
        vec2<f32>( 0.5, -0.5),
    );
    var colors = array<vec3<f32>, 3>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 0.0, 1.0),
    );
    return vec4<f32>(positions[vid], 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.5, 0.2, 1.0);
}
"#.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("triangle layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("triangle pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        self.window = Some(window);
        self.surface = Some(surface);
        self.device = Some(device);
        self.queue = Some(queue);
        self.surface_config = Some(config);
        self.pipeline = Some(pipeline);
    }

    fn render(&mut self) {
        let Some(device) = &self.device else { return; };
        let Some(queue) = &self.queue else { return; };
        let Some(surface) = &self.surface else { return; };
        let Some(pipeline) = &self.pipeline else { return; };

        let frame = match surface.get_current_texture() {
            Ok(f) => f,
            Err(e) => {
                log::warn!("Surface texture acquire failed: {:?}", e);
                return;
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("triangle encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("triangle pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.1, b: 0.15, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(pipeline);
            pass.draw(0..3, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }

    fn resize(&mut self, width: u32, height: u32) {
        let Some(device) = &self.device else { return; };
        let Some(surface) = &self.surface else { return; };
        let Some(config) = self.surface_config.as_mut() else { return; };
        if width == 0 || height == 0 { return; }
        config.width = width;
        config.height = height;
        surface.configure(device, config);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let window = Arc::new(event_loop.create_window(Window::default_attributes()
            .with_title("nova_render - hello_triangle")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
        ).unwrap());

        pollster::block_on(self.init_renderer(window));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                self.resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => {
                self.render();
                if let Some(w) = &self.window { w.request_redraw(); }
            }
            _ => {}
        }
    }
}

fn main() {
    env_logger::try_init().ok();
    log::info!("nova_render hello_triangle starting...");
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}