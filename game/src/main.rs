//! Wasteland Game Launcher - Visual Demo
//!
//! winit + wgpu 可视化 demo，展示引擎模拟的体素网格和元体。
//! 渲染层委托给 wasteland_render（SurfaceRenderer + InstancedRenderer）。
//! 鼠标拖拽旋转相机，滚轮缩放，ESC 退出。

use std::time::Instant;

use glam::{Mat4, Vec3};
use wasteland_engine::{Biome, GameWorld, MaterialProperties, WorldBounds};
use wasteland_render::{CameraUniform, InstancedRenderer, InstanceData, PointInstanceData, SurfaceRenderer};
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    application::ApplicationHandler,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

const WINDOW_WIDTH: u32 = 1600;
const WINDOW_HEIGHT: u32 = 900;
const MAX_INSTANCES: usize = 10000;
const POINT_MAX_INSTANCES: usize = 150_000;
const REPORT_INTERVAL: f32 = 5.0;

// ==================== Camera ====================

struct CameraState {
    target: Vec3,
    distance: f32,
    yaw: f32,
    pitch: f32,
    aspect: f32,
    fov: f32,
}

impl CameraState {
    fn position(&self) -> Vec3 {
        let cos_pitch = self.pitch.cos();
        self.target
            + Vec3::new(
                self.distance * cos_pitch * self.yaw.sin(),
                self.distance * self.pitch.sin(),
                self.distance * cos_pitch * self.yaw.cos(),
            )
    }

    fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position(), self.target, Vec3::Y)
    }

    fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect, 0.1, 1000.0)
    }

    fn view_proj(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    /// Build wasteland_render::CameraUniform (view_proj + view + proj + position, 208 bytes).
    fn uniform(&self) -> CameraUniform {
        let pos = self.position();
        CameraUniform {
            view_proj: self.view_proj().to_cols_array_2d(),
            view: self.view_matrix().to_cols_array_2d(),
            proj: self.projection_matrix().to_cols_array_2d(),
            position: [pos.x, pos.y, pos.z, 1.0],
        }
    }
}

impl Default for CameraState {
    fn default() -> Self {
        // 初始相机位置 (20, 20, -20) 看向原点
        Self {
            target: Vec3::ZERO,
            distance: 34.64,
            yaw: 2.356,
            pitch: 0.615,
            aspect: WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32,
            fov: std::f32::consts::FRAC_PI_4,
        }
    }
}

// ==================== Input ====================

#[derive(Default)]
struct InputState {
    mouse_dragging: bool,
    last_mouse_x: f32,
    last_mouse_y: f32,
}

// ==================== World ====================

fn create_world() -> GameWorld {
    let bounds = WorldBounds {
        min: Vec3::new(-1000.0, -100.0, -1000.0),
        max: Vec3::new(1000.0, 500.0, 1000.0),
    };
    let mut world = GameWorld::new(bounds);

    // 体素网格 10x10x10，混凝土材质
    world.spawn_voxel_grid([10, 10, 10], 1.0, Vec3::ZERO, MaterialProperties::concrete());

    // 元体：铁、水、混凝土
    world.spawn_meta_entity_iron(Vec3::new(12.0, 5.0, 0.0));
    world.spawn_meta_entity_water(Vec3::new(15.0, 5.0, 0.0));
    world.spawn_meta_entity_concrete(Vec3::new(18.0, 5.0, 0.0));

    // 生态系统
    world.spawn_ecosystem(
        "Wasteland Biome".to_string(),
        Biome::Wasteland,
        Vec3::new(-50.0, 0.0, -50.0),
        Vec3::new(50.0, 20.0, 50.0),
    );

    world
}

// ==================== Main ====================

struct App {
    world: GameWorld,
    camera: CameraState,
    input: InputState,
    last_time: Instant,
    report_timer: f32,
    window: Option<Window>,
    surface: Option<SurfaceRenderer>,
    instanced: Option<InstancedRenderer>,
}

impl App {
    fn new(world: GameWorld) -> Self {
        Self {
            world,
            camera: CameraState::default(),
            input: InputState::default(),
            last_time: Instant::now(),
            report_timer: 0.0,
            window: None,
            surface: None,
            instanced: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window_attrs = WindowAttributes::default()
            .with_title("Wasteland Engine - Visual Demo (wasteland_render)")
            .with_inner_size(winit::dpi::PhysicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));
        let window = event_loop
            .create_window(window_attrs)
            .expect("failed to create window");

        let surface = pollster::block_on(SurfaceRenderer::new(&window));
        let instanced = InstancedRenderer::new(
            &surface.device,
            surface.color_format(),
            surface.depth_format,
            MAX_INSTANCES,
            POINT_MAX_INSTANCES,
        );
        self.surface = Some(surface);
        self.instanced = Some(instanced);
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        if window_id != window.id() {
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    if let PhysicalKey::Code(KeyCode::Escape) = event.physical_key {
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                if button == MouseButton::Left {
                    self.input.mouse_dragging = state == ElementState::Pressed;
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let x = position.x as f32;
                let y = position.y as f32;
                if self.input.mouse_dragging {
                    let dx = x - self.input.last_mouse_x;
                    let dy = y - self.input.last_mouse_y;
                    self.camera.yaw -= dx * 0.01;
                    self.camera.pitch = (self.camera.pitch + dy * 0.01).clamp(-1.4, 1.4);
                }
                self.input.last_mouse_x = x;
                self.input.last_mouse_y = y;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y * 2.0,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 * 0.01,
                };
                self.camera.distance = (self.camera.distance - scroll).clamp(5.0, 200.0);
            }
            WindowEvent::Resized(size) => {
                if let Some(surface) = self.surface.as_mut() {
                    if size.width > 0 && size.height > 0 {
                        surface.resize(size.width, size.height);
                        self.camera.aspect = size.width as f32 / size.height as f32;
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let (Some(surface), Some(instanced)) = (self.surface.as_ref(), self.instanced.as_ref())
                else {
                    return;
                };
                self.world.tick();

                let now = Instant::now();
                let elapsed = now.duration_since(self.last_time);
                self.last_time = now;
                self.report_timer += elapsed.as_secs_f32();

                if self.report_timer >= REPORT_INTERVAL {
                    self.report_timer = 0.0;
                    let s = self.world.stats();
                    println!(
                        "t={:.1}s tick={} T={:.1}K rad={:.4} voxels={} meta={} eco={}",
                        s.time,
                        s.tick_count,
                        s.global_temperature,
                        s.global_radiation,
                        s.total_voxels,
                        s.meta_entity_count,
                        s.ecosystem_count
                    );
                }

                let mut instances: Vec<InstanceData> = Vec::new();

                // 体素
                for (pos, color) in self.world.get_voxel_mesh_data(0) {
                    instances.push(InstanceData::new(pos, color));
                }

                // 元体（用更亮的颜色标记）
                let positions = self.world.get_meta_entity_positions();
                let colors = self.world.get_meta_entity_colors();
                for (pos, color) in positions.iter().zip(colors.iter()) {
                    instances.push(InstanceData::new(
                        *pos,
                        [color[0].min(1.0), color[1].min(1.0), color[2].min(1.0), 1.0],
                    ));
                }

                // MpssBuffer 近场粒子（温度映射颜色，cube instancing）
                let mpss_remaining = MAX_INSTANCES.saturating_sub(instances.len());
                if mpss_remaining > 0 {
                    let particle_data = self.world.get_mpss_render_data();
                    for (pos, color) in particle_data.iter().take(mpss_remaining) {
                        instances.push(InstanceData::new(*pos, *color));
                    }
                }

                // MpssBuffer 中/远场粒子（点云渲染，billboard quad）
                let mid_far_data = self.world.get_mpss_mid_far_render_data();
                let point_instances: Vec<PointInstanceData> = mid_far_data
                    .iter()
                    .map(|(pos, color, size)| PointInstanceData::new(*pos, *size, *color))
                    .collect();

                // Upload + render
                instanced.update_camera(&surface.queue, &self.camera.uniform());
                instanced.update_instances(&surface.queue, &instances);
                instanced.update_points(&surface.queue, &point_instances);

                let cube_count = instances.len() as u32;
                let point_count = point_instances.len() as u32;

                surface.render_frame(|pass| {
                    instanced.draw_cubes(pass, cube_count);
                    instanced.draw_points(pass, point_count);
                });
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Wasteland Engine - Visual Demo starting (wasteland_render backend)...");

    let world = create_world();
    let stats = world.stats();
    log::info!(
        "World created: {} voxel grids, {} meta-entities, {} ecosystems",
        stats.voxel_grid_count, stats.meta_entity_count, stats.ecosystem_count
    );

    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut app = App::new(world);
    event_loop.run_app(&mut app).expect("event loop error");
}
