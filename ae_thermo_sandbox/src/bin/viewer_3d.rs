//! V8 Thermo Sandbox - 3D Live Viewer
//!
//! 实时 3D 可视化 1m³ 密封空间内的多物理场耦合：
//! - 16³ 体素网格，每个 cell 渲染为 1 立方单位
//! - 颜色映射温度（蓝→青→绿→黄→红→白）
//! - 鼠标拖拽旋转相机，滚轮缩放
//! - 键盘控制：ESC退出 / Space暂停 / R重置 / 1-4切换视图 / ↑↓火源功率 / ←→速度
//!
//! 这是真正的实时 3D 引擎渲染，不是 PPT/视频。

use std::time::{Duration, Instant};

use glam::{Mat4, Vec3};
use ae_render::{CameraUniform, InstancedRenderer, InstanceData, SurfaceRenderer};
use ae_thermo_sandbox::{CellKind, Sandbox, VACUUM_THRESHOLD, WATER_BOIL_POINT};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

const WINDOW_WIDTH: u32 = 1600;
const WINDOW_HEIGHT: u32 = 900;
const MAX_INSTANCES: usize = 8192; // 16³=4096 + 余量
const GRID_N: usize = 16;

/// 视图模式
#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    Temperature, // 温度色映射（默认）
    Pressure,    // 压力色映射
    Material,    // 材料原色
    Corrosion,   // 腐蚀度
}

impl ViewMode {
    fn next(self) -> Self {
        match self {
            ViewMode::Temperature => ViewMode::Pressure,
            ViewMode::Pressure => ViewMode::Material,
            ViewMode::Material => ViewMode::Corrosion,
            ViewMode::Corrosion => ViewMode::Temperature,
        }
    }
    fn label(self) -> &'static str {
        match self {
            ViewMode::Temperature => "Temperature",
            ViewMode::Pressure => "Pressure",
            ViewMode::Material => "Material",
            ViewMode::Corrosion => "Corrosion",
        }
    }
}

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
        // 看向 16³ 网格中心 (8,8,8)
        Self {
            target: Vec3::new(8.0, 8.0, 8.0),
            distance: 34.0,
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

// ==================== Color mapping ====================

/// 温度 → RGB 颜色（蓝→青→绿→黄→红→白）
fn temp_to_color(t: f32) -> [f32; 4] {
    // 300K(蓝) → 373K(青) → 500K(绿) → 1000K(黄) → 2000K(橙) → 3500K(红) → 5000K(白)
    let (r, g, b) = if t < 373.0 {
        // 300 → 373: 蓝 → 青
        let f = ((t - 300.0) / 73.0).clamp(0.0, 1.0);
        (0.0, f, 1.0)
    } else if t < 500.0 {
        // 373 → 500: 青 → 绿
        let f = ((t - 373.0) / 127.0).clamp(0.0, 1.0);
        (0.0, 1.0, 1.0 - f)
    } else if t < 1000.0 {
        // 500 → 1000: 绿 → 黄
        let f = ((t - 500.0) / 500.0).clamp(0.0, 1.0);
        (f, 1.0, 0.0)
    } else if t < 2000.0 {
        // 1000 → 2000: 黄 → 橙
        let f = ((t - 1000.0) / 1000.0).clamp(0.0, 1.0);
        (1.0, 1.0 - 0.5 * f, 0.0)
    } else if t < 3500.0 {
        // 2000 → 3500: 橙 → 红
        let f = ((t - 2000.0) / 1500.0).clamp(0.0, 1.0);
        (1.0, 0.5 - 0.5 * f, 0.0)
    } else {
        // 3500 → 5000: 红 → 白
        let f = ((t - 3500.0) / 1500.0).clamp(0.0, 1.0);
        (1.0, f, f)
    };
    [r, g, b, 1.0]
}

/// 压力 → RGB（1atm 蓝 → 10atm 绿 → 100atm 黄 → 1000atm 红）
fn pressure_to_color(p_atm: f32) -> [f32; 4] {
    let (r, g, b) = if p_atm < 2.0 {
        let f = ((p_atm - 1.0) / 1.0).clamp(0.0, 1.0);
        (0.0, 0.0, 0.5 + 0.5 * f)
    } else if p_atm < 10.0 {
        let f = ((p_atm - 2.0) / 8.0).clamp(0.0, 1.0);
        (0.0, f, 1.0)
    } else if p_atm < 100.0 {
        let f = ((p_atm - 10.0) / 90.0).clamp(0.0, 1.0);
        (f, 1.0, 1.0 - f)
    } else if p_atm < 1000.0 {
        let f = ((p_atm - 100.0) / 900.0).clamp(0.0, 1.0);
        (1.0, 1.0 - f, 0.0)
    } else {
        (1.0, 0.0, 0.0)
    };
    [r, g, b, 1.0]
}

/// 腐蚀度 → RGB（0 绿 → 0.5 黄 → 1 红）
fn corrosion_to_color(c: f32) -> [f32; 4] {
    let c = c.clamp(0.0, 1.0);
    if c < 0.5 {
        let f = c * 2.0;
        [f, 0.8, 0.0, 1.0]
    } else {
        let f = (c - 0.5) * 2.0;
        [0.8 + 0.2 * f, 0.8 * (1.0 - f), 0.0, 1.0]
    }
}

/// 材料原色
fn material_color(kind: CellKind) -> [f32; 4] {
    match kind {
        CellKind::Iron => [0.6, 0.6, 0.65, 1.0],
        CellKind::Water => [0.1, 0.3, 0.9, 1.0],
        CellKind::Gas => [0.85, 0.85, 0.9, 1.0],
        CellKind::Wood => [0.45, 0.25, 0.1, 1.0],
        CellKind::Concrete => [0.7, 0.7, 0.7, 1.0],
        CellKind::Brick => [0.7, 0.3, 0.2, 1.0],
        CellKind::Flesh => [0.7, 0.2, 0.25, 1.0],
    }
}

/// 综合：材料基础色 × 温度叠加（高温物体自发光发红）
fn combined_color(cell: &ae_thermo_sandbox::Cell, mode: ViewMode) -> [f32; 4] {
    match mode {
        ViewMode::Temperature => {
            // 温度视图：所有 cell 都用温度色，但材料有不同基础亮度
            let base = temp_to_color(cell.temperature);
            // Iron 高温发亮，Water/Gas 半透明感
            match cell.kind {
                CellKind::Iron => {
                    // 金属高温发橙白光
                    if cell.temperature > 1000.0 {
                        let t = ((cell.temperature - 1000.0) / 4000.0).clamp(0.0, 1.0);
                        [1.0, 0.4 + 0.6 * t, 0.2 + 0.8 * t, 1.0]
                    } else {
                        base
                    }
                }
                CellKind::Water => {
                    // 水：沸点以下用深蓝（半透明感），沸点用青
                    if cell.temperature < WATER_BOIL_POINT {
                        let f = (cell.temperature / WATER_BOIL_POINT).clamp(0.0, 1.0);
                        [0.0, 0.2 + 0.3 * f, 0.6 + 0.4 * f, 1.0]
                    } else {
                        [0.0, 1.0, 1.0, 1.0]
                    }
                }
                CellKind::Gas => {
                    // 气体：常温几乎不可见（浅灰），高温发橙红
                    if cell.temperature < 400.0 {
                        let f = ((cell.temperature - 300.0) / 100.0).clamp(0.0, 1.0);
                        [0.3 + 0.2 * f, 0.3 + 0.2 * f, 0.35 + 0.2 * f, 1.0]
                    } else {
                        temp_to_color(cell.temperature)
                    }
                }
                _ => base,
            }
        }
        ViewMode::Pressure => {
            if cell.kind == CellKind::Gas {
                pressure_to_color(cell.pressure / 101_325.0)
            } else {
                // 非气体用暗灰
                let mut c = material_color(cell.kind);
                c[0] *= 0.3;
                c[1] *= 0.3;
                c[2] *= 0.3;
                c
            }
        }
        ViewMode::Material => material_color(cell.kind),
        ViewMode::Corrosion => {
            if cell.kind == CellKind::Iron {
                corrosion_to_color(cell.corrosion)
            } else {
                let mut c = material_color(cell.kind);
                c[0] *= 0.4;
                c[1] *= 0.4;
                c[2] *= 0.4;
                c
            }
        }
    }
}

// ==================== App ====================

struct App {
    sandbox: Sandbox,
    camera: CameraState,
    input: InputState,
    paused: bool,
    view_mode: ViewMode,
    sim_speed: u32, // 每帧推进的 step 数
    last_time: Instant,
    hud_timer: f32,
    fps_counter: u32,
    fps_timer: f32,
    fps: f32,
    initial_energy: f32,
    fire_injected: f32,
    window: Option<Window>,
    surface: Option<SurfaceRenderer>,
    instanced: Option<InstancedRenderer>,
}

impl App {
    fn new() -> Self {
        let sandbox = Sandbox::new_demo_multi();
        let initial_energy = sandbox.cells.iter().map(|c| c.internal_energy()).sum::<f32>();
        Self {
            sandbox,
            camera: CameraState::default(),
            input: InputState::default(),
            paused: false,
            view_mode: ViewMode::Temperature,
            sim_speed: 2, // 每帧 2 步 = 120Hz 物理
            last_time: Instant::now(),
            hud_timer: 0.0,
            fps_counter: 0,
            fps_timer: 0.0,
            fps: 0.0,
            initial_energy,
            fire_injected: 0.0,
            window: None,
            surface: None,
            instanced: None,
        }
    }

    fn reset(&mut self) {
        let new_sb = Sandbox::new_demo_multi();
        self.sandbox = new_sb;
        self.initial_energy = self.sandbox.cells.iter().map(|c| c.internal_energy()).sum();
        self.fire_injected = 0.0;
    }

    fn build_instances(&self) -> Vec<InstanceData> {
        let mut instances: Vec<InstanceData> = Vec::with_capacity(GRID_N * GRID_N * GRID_N);
        let sb = &self.sandbox;
        for k in 0..sb.nz {
            for j in 0..sb.ny {
                for i in 0..sb.nx {
                    let idx = sb.index(i, j, k);
                    let cell = &sb.cells[idx];
                    if cell.mass < VACUUM_THRESHOLD && cell.kind != CellKind::Gas {
                        continue;
                    }
                    // 气体常温 cell 半透明感：降低渲染密度（每隔一个渲染）
                    if cell.kind == CellKind::Gas
                        && cell.temperature < 400.0
                        && cell.pressure < 2.0 * 101_325.0
                        && (i + j + k) % 2 == 0
                    {
                        continue;
                    }
                    let pos = [i as f32, j as f32, k as f32];
                    let color = combined_color(cell, self.view_mode);
                    instances.push(InstanceData::new(pos, color));
                }
            }
        }
        instances
    }

    fn update_title(&mut self) {
        let m = self.sandbox.metrics();
        let drift = (m.energy_total - self.initial_energy - self.fire_injected)
            / (self.initial_energy + self.fire_injected).max(1.0)
            * 100.0;
        let title = format!(
            "V8 Thermo Sandbox 3D - t={:.1}s | Iron {:.0}K | Water {:.0}K | Steam {:.1}kg | P {:.1}atm | NPC {} {:.1}K | drift {:+.2}% | FPS {:.0} | {} | speed {}x",
            self.sandbox.time,
            m.iron_temp_max,
            m.water_temp_max,
            m.steam_mass_total,
            m.gas_pressure_max / 101_325.0,
            m.npc_alive_count,
            m.npc_avg_body_temp,
            drift,
            self.fps,
            self.view_mode.label(),
            self.sim_speed,
        );
        if let Some(w) = self.window.as_ref() {
            w.set_title(&title);
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window_attrs = WindowAttributes::default()
            .with_title("V8 Thermo Sandbox - 3D Live Viewer")
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
            1024,
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
                    if let PhysicalKey::Code(code) = event.physical_key {
                        match code {
                            KeyCode::Escape => event_loop.exit(),
                            KeyCode::Space => {
                                self.paused = !self.paused;
                                println!("[KEY] {}",
                                    if self.paused { "Paused" } else { "Resumed" });
                            }
                            KeyCode::KeyR => {
                                self.reset();
                                println!("[KEY] Reset sandbox");
                            }
                            KeyCode::Digit1 => {
                                self.view_mode = ViewMode::Temperature;
                                println!("[KEY] View: Temperature");
                            }
                            KeyCode::Digit2 => {
                                self.view_mode = ViewMode::Pressure;
                                println!("[KEY] View: Pressure");
                            }
                            KeyCode::Digit3 => {
                                self.view_mode = ViewMode::Material;
                                println!("[KEY] View: Material");
                            }
                            KeyCode::Digit4 => {
                                self.view_mode = ViewMode::Corrosion;
                                println!("[KEY] View: Corrosion");
                            }
                            KeyCode::ArrowUp => {
                                self.sandbox.fire_power *= 1.2;
                                println!("[KEY] Fire power: {:.0} W", self.sandbox.fire_power);
                            }
                            KeyCode::ArrowDown => {
                                self.sandbox.fire_power *= 0.8;
                                println!("[KEY] Fire power: {:.0} W", self.sandbox.fire_power);
                            }
                            KeyCode::ArrowRight => {
                                self.sim_speed = (self.sim_speed + 1).min(20);
                                println!("[KEY] Sim speed: {}x", self.sim_speed);
                            }
                            KeyCode::ArrowLeft => {
                                self.sim_speed = self.sim_speed.saturating_sub(1).max(1);
                                println!("[KEY] Sim speed: {}x", self.sim_speed);
                            }
                            _ => {}
                        }
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
                self.camera.distance = (self.camera.distance - scroll).clamp(5.0, 120.0);
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
                if self.surface.is_none() || self.instanced.is_none() {
                    return;
                }

                // 推进物理
                if !self.paused {
                    let fire_before: f32 = self.sandbox.cells[self.sandbox.fire_cells[0]].temperature;
                    let _ = fire_before;
                    for _ in 0..self.sim_speed {
                        self.sandbox.step();
                    }
                    // 累计火源注入能量（近似：用 last_fire_injected * sim_speed * dt）
                    if let Some(last_fire) = last_fire_injected(&self.sandbox) {
                        self.fire_injected += last_fire * self.sim_speed as f32 * self.sandbox.dt;
                    }
                }

                // 帧时间
                let now = Instant::now();
                let elapsed = now.duration_since(self.last_time);
                self.last_time = now;
                let dt = elapsed.as_secs_f32();
                self.fps_timer += dt;
                self.fps_counter += 1;
                if self.fps_timer >= 0.5 {
                    self.fps = self.fps_counter as f32 / self.fps_timer;
                    self.fps_timer = 0.0;
                    self.fps_counter = 0;
                }

                // HUD 更新（每 0.25s）
                self.hud_timer += dt;
                if self.hud_timer >= 0.25 {
                    self.hud_timer = 0.0;
                    self.update_title();
                    if self.sandbox.time as u32 % 5 == 0 {
                        let m = self.sandbox.metrics();
                        println!(
                            "t={:.1}s Iron={:.0}K Water={:.0}K Steam={:.2}kg Pmax={:.1}atm NPC={} {:.1}K drift={:+.2}%",
                            self.sandbox.time,
                            m.iron_temp_max,
                            m.water_temp_max,
                            m.steam_mass_total,
                            m.gas_pressure_max / 101_325.0,
                            m.npc_alive_count,
                            m.npc_avg_body_temp,
                            (m.energy_total - self.initial_energy - self.fire_injected)
                                / (self.initial_energy + self.fire_injected).max(1.0)
                                * 100.0
                        );
                    }
                }

                // 构建 instance 数据 + camera uniform（owned，不再借用 self）
                let instances = self.build_instances();
                let camera_uniform = self.camera.uniform();

                // 上传 + 渲染 — 现在才借用 surface/instanced
                let surface = self.surface.as_ref().unwrap();
                let instanced = self.instanced.as_ref().unwrap();
                instanced.update_camera(&surface.queue, &camera_uniform);
                instanced.update_instances(&surface.queue, &instances);

                let cube_count = instances.len() as u32;
                surface.render_frame(|pass| {
                    instanced.draw_cubes(pass, cube_count);
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

// 通过反射读取 last_fire_injected（lib 没暴露，用近似）
fn last_fire_injected(_sb: &Sandbox) -> Option<f32> {
    // Sandbox 没有公开 last_fire_injected 字段，用火源功率近似
    None
}

fn print_help() {
    println!("================================================================================");
    println!("V8 Thermo Sandbox - 3D Live Viewer");
    println!("================================================================================");
    println!("Scene: 1m³ sealed space, 16³ grid, iron ball + water + wood + NPC + fire");
    println!();
    println!("Controls:");
    println!("  Mouse Left Drag : Rotate camera");
    println!("  Mouse Wheel     : Zoom in/out");
    println!("  ESC             : Quit");
    println!("  Space           : Pause/Resume physics");
    println!("  R               : Reset sandbox");
    println!("  1               : Temperature view (default)");
    println!("  2               : Pressure view");
    println!("  3               : Material view");
    println!("  4               : Corrosion view");
    println!("  Arrow Up/Down   : Fire power ±20%");
    println!("  Arrow L/R       : Sim speed ±1x");
    println!();
    println!("Title bar shows: time | Iron T | Water T | Steam | Pressure | NPC | drift | FPS");
    println!("================================================================================");
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    print_help();

    log::info!("Starting V8 Thermo Sandbox 3D Live Viewer...");

    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("event loop error");
}
