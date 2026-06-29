//! NovaApp — winit 主循环 + wgpu 初始化 + RenderGraph 调度集成
//!
//! P0 优先级：把 winit + wgpu + RenderGraph 三者封装成一个开箱即用的应用骨架。
//!
//! 设计参考：
//! - hello_triangle.rs（最小 winit 0.x + wgpu 24 demo）
//! - bevy 的 App + Plugins builder pattern
//! - rend3 的 RawRenderer + window 集成
//!
//! # 使用示例
//!
//! ```no_run
//! use nova_render::application::NovaApp;
//! use nova_render::render_graph::RenderGraph;
//!
//! fn main() {
//!     env_logger::try_init().ok();
//!     let graph = RenderGraph::new();
//!     NovaApp::builder()
//!         .title("Nova Render Demo")
//!         .size(1280, 720)
//!         .with_render_graph(graph)
//!         .on_render(|app| {
//!             let _device = app.device();
//!             let _queue = app.queue();
//!         })
//!         .on_resize(|_app, w, h| {
//!             log::info!("Window resized to {}x{}", w, h);
//!         })
//!         .build()
//!         .run();
//! }
//! ```

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use crate::render_graph::{RenderGraph, ResourceTable};

/// NovaApp — 封装 winit EventLoop + wgpu 初始化 + RenderGraph 调度
///
/// 通过 [`NovaApp::builder()`] 创建，调用 [`NovaApp::run()`] 进入主循环。
pub struct NovaApp {
    pub(crate) event_loop: Option<EventLoop<()>>,
    pub(crate) window: Option<Arc<Window>>,
    pub(crate) instance: Option<wgpu::Instance>,
    pub(crate) surface: Option<wgpu::Surface<'static>>,
    pub(crate) adapter: Option<wgpu::Adapter>,
    pub(crate) device: Option<wgpu::Device>,
    pub(crate) queue: Option<wgpu::Queue>,
    pub(crate) config: Option<wgpu::SurfaceConfiguration>,
    pub(crate) graph: RenderGraph,
    /// 跨 pass 资源表（每帧 clear 后由 pass 注册/查询）
    pub(crate) resource_table: ResourceTable,
    /// 帧计数（TAA / 动画用）
    pub(crate) frame_count: u64,
    /// 启动时间戳（秒）
    pub(crate) start_time: std::time::Instant,
    pub(crate) title: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) on_render: Option<Box<dyn FnMut(&mut NovaApp)>>,
    pub(crate) on_resize: Option<Box<dyn FnMut(&mut NovaApp, u32, u32)>>,
}
impl NovaApp {
    /// 创建 Builder
    pub fn builder() -> NovaAppBuilder {
        NovaAppBuilder::default()
    }

    /// 进入 winit 事件循环
    ///
    /// 消耗 self 并在当前线程阻塞运行事件循环。
    /// 必须在主线程调用（winit 限制）。
    pub fn run(mut self) {
        let event_loop = self
            .event_loop
            .take()
            .unwrap_or_else(|| EventLoop::new().expect("Failed to create winit EventLoop"));
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop
            .run_app(&mut self)
            .expect("NovaApp: winit EventLoop run_app failed");
    }

    /// wgpu Device（resumed 之后可用）
    pub fn device(&self) -> &wgpu::Device {
        self.device
            .as_ref()
            .expect("NovaApp::device() called before wgpu initialization (need resumed event)")
    }

    /// wgpu Queue
    pub fn queue(&self) -> &wgpu::Queue {
        self.queue
            .as_ref()
            .expect("NovaApp::queue() called before wgpu initialization (need resumed event)")
    }

    /// winit Window
    pub fn window(&self) -> &Window {
        self.window
            .as_ref()
            .expect("NovaApp::window() called before window creation (need resumed event)")
    }

    /// Window 的 Arc 引用（用于跨结构共享）
    pub fn window_arc(&self) -> Arc<Window> {
        self.window
            .as_ref()
            .expect("NovaApp::window_arc() called before window creation")
            .clone()
    }

    /// wgpu Instance
    pub fn instance(&self) -> &wgpu::Instance {
        self.instance
            .as_ref()
            .expect("NovaApp::instance() called before wgpu initialization")
    }

    /// wgpu Adapter
    pub fn adapter(&self) -> &wgpu::Adapter {
        self.adapter
            .as_ref()
            .expect("NovaApp::adapter() called before wgpu initialization")
    }

    /// wgpu Surface
    pub fn surface(&self) -> &wgpu::Surface<'static> {
        self.surface
            .as_ref()
            .expect("NovaApp::surface() called before wgpu initialization")
    }

    /// 当前 SurfaceConfiguration
    pub fn surface_config(&self) -> &wgpu::SurfaceConfiguration {
        self.config
            .as_ref()
            .expect("NovaApp::surface_config() called before wgpu initialization")
    }

    /// 当前 surface 颜色格式
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_config().format
    }

    /// 可变借用 RenderGraph
    pub fn graph_mut(&mut self) -> &mut RenderGraph {
        &mut self.graph
    }

    /// 不可变借用 RenderGraph
    pub fn graph(&self) -> &RenderGraph {
        &self.graph
    }

    /// 当前窗口尺寸（来自 surface config，保证非 0）
    pub fn current_size(&self) -> (u32, u32) {
        if let Some(c) = self.config.as_ref() {
            (c.width.max(1), c.height.max(1))
        } else {
            (self.width.max(1), self.height.max(1))
        }
    }
    // ====== 内部初始化 ======

    /// 异步初始化 wgpu（Instance/Adapter/Device/Queue/Surface）
    async fn init_renderer(&mut self, window: Arc<Window>) {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance
            .create_surface(window.clone())
            .expect("NovaApp: failed to create wgpu Surface");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("NovaApp: failed to find suitable wgpu Adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("nova_render device"),
                    required_features: wgpu::Features::default(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .expect("NovaApp: failed to request wgpu Device");

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
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

        self.window = Some(window);
        self.instance = Some(instance);
        self.surface = Some(surface);
        self.adapter = Some(adapter);
        self.device = Some(device);
        self.queue = Some(queue);
        self.config = Some(config);
    }

    /// 重新配置 surface（窗口尺寸变化或 surface lost 时调用）
    fn reconfigure_surface(&mut self) {
        let (device, surface, config) = match (
            self.device.as_ref(),
            self.surface.as_ref(),
            self.config.as_ref(),
        ) {
            (Some(d), Some(s), Some(c)) => (d, s, c),
            _ => return,
        };
        surface.configure(device, config);
    }

    /// 处理 Resized 事件
    fn handle_resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if let Some(config) = self.config.as_mut() {
            config.width = width;
            config.height = height;
        }
        self.reconfigure_surface();
        if let Some(mut f) = self.on_resize.take() {
            f(self, width, height);
            self.on_resize = Some(f);
        }
    }
    /// 处理 RedrawRequested 事件
    ///
    /// 流程：get_current_texture → create_view → on_render hook → graph.execute → present
    fn handle_redraw(&mut self) {
        // 1. 检查 device/queue/surface 是否就绪
        if self.device.is_none() || self.queue.is_none() || self.surface.is_none() {
            return;
        }

        // 2. 获取当前帧纹理
        let frame = {
            let surface = self.surface.as_ref().unwrap();
            match surface.get_current_texture() {
                Ok(f) => f,
                Err(e) => {
                    log::warn!("NovaApp: surface get_current_texture failed: {:?}", e);
                    // Lost / OutOfMemory：重新配置 surface 并跳过本帧
                    self.reconfigure_surface();
                    return;
                }
            }
        };

        // 3. 创建 swapchain texture view（作为最终输出目标传给 RenderGraph）
        let surface_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // 4. 用户渲染前回调（可更新 uniform / 摄像机 / graph 节点状态）
        if let Some(mut f) = self.on_render.take() {
            f(self);
            self.on_render = Some(f);
        }

        // 5. 创建 CommandEncoder + 清空资源表 + 执行 RenderGraph
        //    利用 Rust disjoint field borrows：&mut self.graph 与 &mut self.resource_table
        //    与 &self.device / &self.queue 同时存在
        let time = self.start_time.elapsed().as_secs_f32();
        let frame_num = self.frame_count;
        self.resource_table.clear();
        let encoder_desc = wgpu::CommandEncoderDescriptor {
            label: Some("nova_render frame encoder"),
        };
        let mut encoder = self
            .device
            .as_ref()
            .unwrap()
            .create_command_encoder(&encoder_desc);

        let report = {
            let device = self.device.as_ref().unwrap();
            let queue = self.queue.as_ref().unwrap();
            let (sw, sh) = self.current_size();
            let graph = &mut self.graph;
            let resources = &mut self.resource_table;
            graph.execute(
                device,
                queue,
                &mut encoder,
                resources,
                Some(&surface_view),
                (sw, sh),
                time,
                frame_num,
            )
        };

        if !report.is_ok() {
            log::warn!(
                "NovaApp: RenderGraph executed with {} / {} nodes failed; errors: {:?}",
                report.nodes_failed,
                report.nodes_executed + report.nodes_failed,
                report.errors
            );
        }

        // 6. submit + present
        let cmd_buf = encoder.finish();
        self.queue.as_ref().unwrap().submit([cmd_buf]);
        self.frame_count += 1;
        frame.present();
    }
}

impl ApplicationHandler for NovaApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // 已有窗口则跳过（移动设备 / 多次 resume）
        if self.window.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title(&self.title)
                        .with_inner_size(winit::dpi::LogicalSize::new(self.width, self.height)),
                )
                .expect("NovaApp: failed to create winit Window"),
        );
        pollster::block_on(self.init_renderer(window));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                log::info!("NovaApp: CloseRequested, exiting event loop");
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                self.handle_resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => {
                self.handle_redraw();
                // Poll 模式下持续触发下一帧
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }
}
// ====== Builder ======

/// NovaApp Builder — 通过 [`NovaApp::builder()`] 获取实例
pub struct NovaAppBuilder {
    title: String,
    width: u32,
    height: u32,
    graph: RenderGraph,
    on_render: Option<Box<dyn FnMut(&mut NovaApp)>>,
    on_resize: Option<Box<dyn FnMut(&mut NovaApp, u32, u32)>>,
}

impl Default for NovaAppBuilder {
    fn default() -> Self {
        Self {
            title: "Nova Render".to_string(),
            width: 1280,
            height: 720,
            graph: RenderGraph::new(),
            on_render: None,
            on_resize: None,
        }
    }
}

impl NovaAppBuilder {
    /// 设置窗口标题
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }

    /// 设置初始窗口尺寸（逻辑像素）
    pub fn size(mut self, w: u32, h: u32) -> Self {
        self.width = w;
        self.height = h;
        self
    }

    /// 注入预构建的 RenderGraph
    pub fn with_render_graph(mut self, g: RenderGraph) -> Self {
        self.graph = g;
        self
    }

    /// 注册每帧渲染前回调
    ///
    /// 回调签名 `FnMut(&mut NovaApp)`，可在回调内访问 device/queue/graph 等。
    pub fn on_render<F: FnMut(&mut NovaApp) + 'static>(mut self, f: F) -> Self {
        self.on_render = Some(Box::new(f));
        self
    }

    /// 注册窗口 resize 回调
    ///
    /// 回调签名 `FnMut(&mut NovaApp, u32, u32)`，参数为新尺寸 (width, height)。
    pub fn on_resize<F: FnMut(&mut NovaApp, u32, u32) + 'static>(mut self, f: F) -> Self {
        self.on_resize = Some(Box::new(f));
        self
    }

    /// 构建 NovaApp
    ///
    /// 注意：EventLoop 在 [`NovaApp::run`] 中延迟创建，避免在 build 阶段就占用主线程资源。
    pub fn build(self) -> NovaApp {
        NovaApp {
            event_loop: None,
            window: None,
            instance: None,
            surface: None,
            adapter: None,
            device: None,
            queue: None,
            config: None,
            graph: self.graph,
            resource_table: ResourceTable::new(),
            frame_count: 0,
            start_time: std::time::Instant::now(),
            title: self.title,
            width: self.width,
            height: self.height,
            on_render: self.on_render,
            on_resize: self.on_resize,
        }
    }
}