//! Wasteland Editor — main entry point.
//!
//! Sets up the winit window, wgpu graphics context, egui immediate-mode GUI,
//! and runs the main event loop.

#![allow(dead_code)]
#![allow(clippy::all)]

mod app;
mod camera;
mod commands;
mod engine_bridge;
mod engine_types;
mod gizmo;
mod mcp;
mod panels;
mod plugin;
mod render;
mod scene;
mod scene_io;
mod selection;
mod settings;
mod shortcut;
mod undo_redo;

#[allow(dead_code)]
use app::{EditorAction, EditorApp};
use egui_wgpu::wgpu;
use render::scene_renderer::SceneRenderer;
use render::grid_renderer::GridRenderer;
use render::gizmo_renderer::{GizmoRenderer3D, GizmoMode as WgpuGizmoMode};
use crate::gizmo::GizmoMode;
use shortcut::ShortcutHandler;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::ModifiersState,
    window::{Window, WindowId},
};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting Wasteland Editor v0.1");

    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            log::error!("Failed to create event loop: {}", e);
            return;
        },
    };

    let mut editor = EditorApplication::new();

    if let Err(e) = event_loop.run_app(&mut editor) {
        log::error!("Event loop error: {}", e);
    }

    log::info!("Editor shut down.");
}

/// Top-level application state for the winit event loop.
struct EditorApplication {
    /// The editor state.
    app: Option<EditorApp>,

    /// The winit window.
    window: Option<Arc<Window>>,

    /// wgpu state.
    gpu: Option<GpuState>,

    /// egui platform integration (winit backend).
    egui_winit: Option<egui_winit::State>,

    /// Keyboard shortcut handler.
    shortcut_handler: ShortcutHandler,

    /// Tracked modifiers for shortcut dispatch.
    modifiers: ModifiersState,

    /// Window size for resize handling.
    window_size: PhysicalSize<u32>,
}

/// wgpu related state.
struct GpuState {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    egui_renderer: egui_wgpu::Renderer,
    scene_renderer: SceneRenderer,
    grid_renderer: GridRenderer,
    gizmo_renderer: GizmoRenderer3D,
    viewport_texture: wgpu::Texture,
    viewport_view: wgpu::TextureView,
    viewport_size: (u32, u32),
    viewport_egui_id: egui::TextureId,
    /// Current grid step size (used to detect when rebuild is needed).
    grid_step: f32,
}

impl EditorApplication {
    fn new() -> Self {
        Self {
            app: None,
            window: None,
            gpu: None,
            egui_winit: None,
            shortcut_handler: ShortcutHandler::new(),
            modifiers: ModifiersState::default(),
            window_size: PhysicalSize::new(1280, 720),
        }
    }

    /// Initialise wgpu and egui once the window is created.
    fn init_graphics(&mut self, window: Arc<Window>) {
        let size = window.inner_size();
        self.window_size = size;
        let width = size.width.max(1);
        let height = size.height.max(1);

        // Create wgpu instance.
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Create surface from the window.
        let surface =
            instance.create_surface(window.clone()).expect("Failed to create wgpu surface");

        // Request adapter.
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("Failed to find suitable GPU adapter");

        // Request device.
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Wasteland Editor GPU"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
            None,
        ))
        .expect("Failed to create wgpu device");

        // Configure the surface.
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        // Initialise egui renderer.
        let mut egui_renderer = egui_wgpu::Renderer::new(&device, surface_format, None, 1, false);

        // Initialise egui-winit state.
        let egui_winit = egui_winit::State::new(
            egui::Context::default(),
            egui::ViewportId::default(),
            &window,
            None,
            None,
            None,
        );

        // --- 3D viewport renderers ---
        let scene_renderer = SceneRenderer::new(&device, &config, 1);
        let grid_renderer = GridRenderer::new(&device, &config, 1);
        let gizmo_renderer = GizmoRenderer3D::new(&device, &config, 1);

        // --- Offscreen viewport texture ---
        let vp_w = width.max(1);
        let vp_h = height.max(1);
        let viewport_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Viewport Render Target"),
            size: wgpu::Extent3d { width: vp_w, height: vp_h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let viewport_view = viewport_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Register the viewport texture with egui_wgpu so we get a TextureId
        // that egui::Image can display.
        let viewport_egui_id = egui_renderer.register_native_texture(
            &device,
            &viewport_view,
            wgpu::FilterMode::Linear,
        );

        self.gpu =
            Some(GpuState {
                instance, surface, adapter, device, queue, config, egui_renderer,
                scene_renderer, grid_renderer, gizmo_renderer,
                viewport_texture, viewport_view, viewport_size: (vp_w, vp_h),
                viewport_egui_id,
                grid_step: 1.0,
            });
        self.egui_winit = Some(egui_winit);
    }

    /// Handle window resize.
    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.window_size = new_size;
        if let Some(ref mut gpu) = self.gpu {
            if new_size.width > 0 && new_size.height > 0 {
                gpu.config.width = new_size.width;
                gpu.config.height = new_size.height;
                gpu.surface.configure(&gpu.device, &gpu.config);
            }
        }
    }

    /// Render a single frame.
    fn render_frame(&mut self) -> Result<(), String> {
        let gpu = self.gpu.as_mut().ok_or("GPU not initialised")?;
        let egui_winit = self.egui_winit.as_mut().ok_or("egui_winit not initialised")?;
        let window = self.window.as_ref().ok_or("Window not created")?;
        let app = self.app.as_mut().ok_or("App not initialised")?;

        // --- Render 3D scene to offscreen viewport texture ---
        {
            // Dynamic resize: if the viewport panel changed size since last
            // frame, recreate the offscreen texture to match. This avoids
            // rendering at the wrong resolution when dock panels are resized.
            if let Some((_, _, rw, rh)) = app.viewport_rect {
                let new_w = (rw as u32).max(1);
                let new_h = (rh as u32).max(1);
                if (new_w, new_h) != gpu.viewport_size {
                    let new_texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("Viewport Render Target"),
                        size: wgpu::Extent3d {
                            width: new_w,
                            height: new_h,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: gpu.config.format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    });
                    let new_view =
                        new_texture.create_view(&wgpu::TextureViewDescriptor::default());
                    let new_id = gpu.egui_renderer.register_native_texture(
                        &gpu.device,
                        &new_view,
                        wgpu::FilterMode::Linear,
                    );
                    gpu.viewport_texture = new_texture;
                    gpu.viewport_view = new_view;
                    gpu.viewport_size = (new_w, new_h);
                    gpu.viewport_egui_id = new_id;
                    gpu.scene_renderer.resize(&gpu.device, new_w, new_h);
                    log::debug!("Viewport resized to {}x{}", new_w, new_h);
                }
            }

            let (vp_w, vp_h) = gpu.viewport_size;
            let aspect = if vp_h > 0 { vp_w as f32 / vp_h as f32 } else { 1.0 };
            let view_proj = app.camera.view_projection_matrix(aspect);
            let cam_pos = app.camera.position;

            // Update camera uniforms for all renderers.
            gpu.scene_renderer.update_camera(&gpu.queue, view_proj, cam_pos);
            gpu.grid_renderer.update_camera(&gpu.queue, view_proj);

            // Rebuild grid if step size changed (from SettingsPanel.grid_size).
            let desired_step = app.settings_panel.as_ref().map(|s| s.grid_size).unwrap_or(1.0);
            if (desired_step - gpu.grid_step).abs() > 0.001 {
                gpu.grid_renderer.rebuild(&gpu.device, 10.0, desired_step);
                gpu.grid_step = desired_step;
            }

            // Update gizmo: place at selected node's position.
            let gizmo_model = if let Some(id) = app.selection.selected_id {
                if let Some(node) = app.scene.find_node(id) {
                    glam::Mat4::from_translation(node.transform.translation)
                } else {
                    glam::Mat4::IDENTITY
                }
            } else {
                glam::Mat4::IDENTITY
            };
            gpu.gizmo_renderer.update(&gpu.queue, view_proj, gizmo_model);

            // Ensure depth texture exists for the viewport size.
            if gpu.scene_renderer.depth_texture.is_none() {
                gpu.scene_renderer.resize(&gpu.device, vp_w, vp_h);
            }

            let depth_view = gpu.scene_renderer.depth_texture.as_ref()
                .expect("depth texture should exist")
                .create_view(&wgpu::TextureViewDescriptor::default());

            let mut scene_encoder = gpu.device.create_command_encoder(
                &wgpu::CommandEncoderDescriptor { label: Some("3D Viewport Encoder") }
            );
            {
                let mut pass = scene_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("3D Viewport Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &gpu.viewport_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.12, g: 0.14, b: 0.18, a: 1.0 }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                // Render grid first (background) — conditional on viewport setting.
                if app.viewport_panel.show_grid {
                    gpu.grid_renderer.render(&mut pass);
                }

                // Render scene nodes (meshes, lights, cameras).
                gpu.scene_renderer.render_scene(&mut pass, &app.scene, &app.selection, &gpu.queue, view_proj);

                // Render gizmo on top (if something is selected).
                if app.selection.selected_id.is_some() {
                    let mode = match app.gizmo.mode {
                        GizmoMode::Translate => WgpuGizmoMode::Translate,
                        GizmoMode::Rotate => WgpuGizmoMode::Rotate,
                        GizmoMode::Scale => WgpuGizmoMode::Scale,
                    };
                    gpu.gizmo_renderer.render(&mut pass, mode);
                }
            }
            gpu.queue.submit(std::iter::once(scene_encoder.finish()));

            // Tell the editor what to display in the viewport panel.
            app.viewport_texture_id = Some(gpu.viewport_egui_id);
            app.viewport_texture_size = gpu.viewport_size;
        }

        // Begin egui frame.
        let raw_input = egui_winit.take_egui_input(window);
        let full_output = egui_winit.egui_ctx().run(raw_input, |ctx| {
            // Render editor panels.
            app.render(ctx);
        });

        // Handle egui output (clipboard, etc.).
        egui_winit.handle_platform_output(window, full_output.platform_output);

        // Prepare paint jobs.
        let paint_jobs =
            egui_winit.egui_ctx().tessellate(full_output.shapes, full_output.pixels_per_point);

        // Render with wgpu.
        let output = gpu
            .surface
            .get_current_texture()
            .map_err(|e| format!("Failed to acquire surface texture: {}", e))?;

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Upload egui resources (encoder 1).
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [gpu.config.width, gpu.config.height],
            pixels_per_point: window.scale_factor() as f32,
        };

        let tdelta = full_output.textures_delta;

        let mut upload_encoder =
            gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Editor Upload Encoder"),
            });

        // Upload textures (must happen before update_buffers).
        for (id, image_delta) in &tdelta.set {
            gpu.egui_renderer.update_texture(&gpu.device, &gpu.queue, *id, image_delta);
        }

        gpu.egui_renderer.update_buffers(
            &gpu.device,
            &gpu.queue,
            &mut upload_encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        gpu.queue.submit(std::iter::once(upload_encoder.finish()));

        // Clear and render (encoder 2).
        let mut render_encoder =
            gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Editor Render Encoder"),
            });

        let render_pass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Editor Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.15, g: 0.16, b: 0.18, a: 1.0 }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // wgpu 0.24: forget_lifetime() converts RenderPass<'_> → RenderPass<'static>
        // needed for egui_wgpu 0.31 compatibility
        let mut static_pass = wgpu::RenderPass::forget_lifetime(render_pass);
        gpu.egui_renderer.render(&mut static_pass, &paint_jobs, &screen_descriptor);
        drop(static_pass);

        gpu.queue.submit(std::iter::once(render_encoder.finish()));

        // Free textures.
        for id in tdelta.free {
            gpu.egui_renderer.free_texture(&id);
        }

        output.present();

        Ok(())
    }
}

impl ApplicationHandler for EditorApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create the window on first resume.
        if self.window.is_none() {
            let window_attrs = Window::default_attributes()
                .with_title("Wasteland Editor")
                .with_inner_size(self.window_size);

            let window =
                Arc::new(event_loop.create_window(window_attrs).expect("Failed to create window"));

            self.init_graphics(window.clone());
            self.window = Some(window);
            let mut app = EditorApp::new();
            app.init_plugins();
            self.app = Some(app);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Let egui-winit process the event first.
        if let Some(ref mut egui_winit) = self.egui_winit {
            if let Some(ref window) = self.window {
                let _ = egui_winit.on_window_event(window, &event);
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                if let Some(ref mut app) = self.app {
                    app.pending_action = Some(EditorAction::Exit);
                }
            },

            WindowEvent::Resized(new_size) => {
                self.resize(new_size);
            },

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: _,
                        logical_key: key,
                        state: ElementState::Pressed,
                        repeat: false,
                        ..
                    },
                ..
            } => {
                self.shortcut_handler.set_modifiers(self.modifiers);
                if let Some(shortcut_action) = self.shortcut_handler.handle_key_press(&key) {
                    if let Some(ref mut app) = self.app {
                        let action = ShortcutHandler::to_editor_action(shortcut_action);
                        app.pending_action = Some(action);
                    }
                }
            },

            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
                self.shortcut_handler.set_modifiers(self.modifiers);
            },

            WindowEvent::RedrawRequested => {
                // Render a frame.
                if let Err(e) = self.render_frame() {
                    log::error!("Render error: {}", e);
                }

                // Check for exit after rendering.
                if let Some(ref app) = self.app {
                    if app.should_exit {
                        event_loop.exit();
                        return;
                    }
                }

                // Request next frame.
                if let Some(ref window) = self.window {
                    window.request_redraw();
                }
            },

            _ => {},
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Execute any pending actions (file dialogs, save/load, etc.).
        if let Some(ref mut app) = self.app {
            app.execute_pending_action();
        }

        // Request redraw to keep the UI responsive.
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}
