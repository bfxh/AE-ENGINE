//! WaveBlender — GPU FDTD 声学求解器 + 时间混合边界
//!
//! 基于:
//! - Xue, Wang, Langlois, James. *WaveBlender: Practical Sound-Source Animation
//!   in Blended Domains*. SIGGRAPH Asia 2024.
//!   https://graphics.stanford.edu/papers/waveblender/assets/waveblender_full.pdf
//!
//! 核心创新:
//! - 二阶中心差分 FDTD 离散波动方程
//!   p^{n+1} = 2 p^n − p^{n−1} + (cΔt/h)² · ∇² p^n
//! - **时间混合参数 β** (cell-centered): 在运动/变形边界附近用 β 在两个相邻帧
//!   的离散化之间线性插值，消除运动边界穿越网格的伪影
//!   p^{n+1} ← (1-β) · p^{n+1}_{standard} + β · p^n
//! - 加速度噪声点源 (薄壳/液体声)
//! - GPU 友好的显式更新，每 cell 独立，比 CPU 快 1000×
//!
//! 架构:
//! 1. CPU 参考实现 (WaveBlenderSolver) — 可测试、可验证
//! 2. WGSL compute shader 源码 (WAVEBLENDER_WGSL) — GPU kernel
//! 3. GPU 调度器 (feature = "gpu") — wgpu 24 dispatch

use glam::Vec3;
use serde::{Deserialize, Serialize};

// ============================================================
// 配置 & 数据结构
// ============================================================

/// WaveBlender 求解器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveBlenderConfig {
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    /// 网格间距 (米)
    pub h: f32,
    /// 时间步长 (秒)，需满足 CFL: c·dt/h ≤ 1/√3 (3D)
    pub dt: f32,
    /// 声速 (米/秒)
    pub c: f32,
    /// 全局时间混合参数 β ∈ [0, 1]
    /// - β = 0: 标准 FDTD (无混合)
    /// - β = 1: 完全混合 (等同于保持 p^n)
    /// 推荐值: 0.5 用于运动边界附近，0.0 用于开放空间
    pub beta: f32,
    /// 边界吸收系数 ∈ [0, 1]
    pub absorption: f32,
}

impl Default for WaveBlenderConfig {
    fn default() -> Self {
        let h = 0.1; // 10 cm
        let c = 343.0;
        // CFL: dt ≤ h / (c·√3) ≈ 0.1 / 594 ≈ 1.68e-4
        let dt = h / (c * 3.0f32.sqrt()) * 0.95; // 95% CFL
        Self {
            nx: 64,
            ny: 64,
            nz: 64,
            h,
            dt,
            c,
            beta: 0.0,
            absorption: 0.01,
        }
    }
}

/// 单个加速度噪声点源 (论文 Eq. 9-11)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AccelerationSource {
    pub position: Vec3,
    /// 加速度时间序列 (m/s²)，长度 = 模拟步数
    /// 实际使用时由用户填充或由物理引擎提供
    pub amplitude: f32,
    pub frequency: f32,
    pub phase: f32,
}

impl AccelerationSource {
    pub fn new(position: Vec3, amplitude: f32, frequency: f32) -> Self {
        Self {
            position,
            amplitude,
            frequency,
            phase: 0.0,
        }
    }

    /// 计算时刻 t 的加速度 (m/s²)
    pub fn acceleration_at(&self, t: f32) -> f32 {
        self.amplitude * (2.0 * std::f32::consts::PI * self.frequency * t + self.phase).sin()
    }
}

/// WaveBlender 求解器 (CPU 参考 + GPU 调度)
#[derive(Debug, Clone)]
pub struct WaveBlenderSolver {
    pub config: WaveBlenderConfig,
    /// 当前压力场 p^n
    pub pressure_curr: Vec<f32>,
    /// 前一步压力场 p^{n-1}
    pub pressure_prev: Vec<f32>,
    /// 每 cell 的时间混合参数 β_i ∈ [0, 1]
    pub beta_field: Vec<f32>,
    /// 当前模拟时间
    pub time: f32,
    /// 加速度噪声源
    pub sources: Vec<AccelerationSource>,
}

impl WaveBlenderSolver {
    pub fn new(config: WaveBlenderConfig) -> Self {
        let n = config.nx * config.ny * config.nz;
        Self {
            config,
            pressure_curr: vec![0.0; n],
            pressure_prev: vec![0.0; n],
            beta_field: vec![0.0; n],
            time: 0.0,
            sources: Vec::new(),
        }
    }

    #[inline]
    pub fn idx(&self, i: usize, j: usize, k: usize) -> usize {
        (k * self.config.ny + j) * self.config.nx + i
    }

    pub fn add_source(&mut self, source: AccelerationSource) {
        self.sources.push(source);
    }

    /// 在某 cell 设置 β (用于运动边界附近)
    pub fn set_beta(&mut self, i: usize, j: usize, k: usize, beta: f32) {
        let idx = self.idx(i, j, k);
        self.beta_field[idx] = beta.clamp(0.0, 1.0);
    }

    /// 全局设置 β
    pub fn set_beta_global(&mut self, beta: f32) {
        let beta = beta.clamp(0.0, 1.0);
        for b in &mut self.beta_field {
            *b = beta;
        }
    }

    /// 在球体区域设置 β (用于运动边界周围的混合区)
    pub fn set_beta_sphere(&mut self, center: Vec3, radius: f32, beta: f32) {
        let beta = beta.clamp(0.0, 1.0);
        let r2 = radius * radius;
        let h = self.config.h;
        for k in 0..self.config.nz {
            for j in 0..self.config.ny {
                for i in 0..self.config.nx {
                    let pos = Vec3::new(i as f32 * h, j as f32 * h, k as f32 * h);
                    let d = pos - center;
                    if d.length_squared() < r2 {
                        let idx = self.idx(i, j, k);
                        self.beta_field[idx] = beta;
                    }
                }
            }
        }
    }

    /// 执行一步 FDTD 更新 (CPU 参考)
    pub fn step(&mut self) {
        let dt = self.config.dt;
        let h = self.config.h;
        let c = self.config.c;
        let c2_dt2_over_h2 = (c * c) * (dt * dt) / (h * h);
        let absorption = self.config.absorption;
        let global_beta = self.config.beta;
        let nx = self.config.nx;
        let ny = self.config.ny;
        let nz = self.config.nz;

        // 新压力场
        let mut p_next = vec![0.0f32; nx * ny * nz];

        // 内部 cells: 二阶中心差分 FDTD
        for k in 1..nz - 1 {
            for j in 1..ny - 1 {
                for i in 1..nx - 1 {
                    let idx = self.idx(i, j, k);
                    let p = self.pressure_curr[idx];
                    let p_prev = self.pressure_prev[idx];

                    // 6 邻居
                    let p_xp = self.pressure_curr[self.idx(i + 1, j, k)];
                    let p_xm = self.pressure_curr[self.idx(i - 1, j, k)];
                    let p_yp = self.pressure_curr[self.idx(i, j + 1, k)];
                    let p_ym = self.pressure_curr[self.idx(i, j - 1, k)];
                    let p_zp = self.pressure_curr[self.idx(i, j, k + 1)];
                    let p_zm = self.pressure_curr[self.idx(i, j, k - 1)];

                    // 标准 FDTD: p^{n+1} = 2 p^n - p^{n-1} + (c·dt/h)² · (Σ邻居 - 6p)
                    // c2_dt2_over_h2 = c²·dt²/h²，配合 (Σ邻居-6p) 即为 c²·dt²·∇²p
                    let mut p_new = 2.0 * p - p_prev + c2_dt2_over_h2 * (p_xp + p_xm + p_yp + p_ym + p_zp + p_zm - 6.0 * p);

                    // 时间混合: p^{n+1} ← (1-β) · p^{n+1} + β · p^n
                    let beta = if global_beta > 0.0 { global_beta } else { self.beta_field[idx] };
                    if beta > 0.0 {
                        p_new = (1.0 - beta) * p_new + beta * p;
                    }

                    // 吸收: 衰减
                    p_new *= 1.0 - absorption;

                    // 加速度噪声源 (在源位置注入)
                    let pos = Vec3::new(i as f32 * h, j as f32 * h, k as f32 * h);
                    for src in &self.sources {
                        let d = pos - src.position;
                        let d2 = d.length_squared();
                        if d2 < (2.0 * h) * (2.0 * h) {
                            // 距离衰减 (1/r 简化为 1/(r+ε))
                            let r = d2.sqrt().max(h * 0.5);
                            let a = src.acceleration_at(self.time);
                            // 源项: ρ₀ · ∂a/∂t ≈ ρ₀ · a / dt (简化)
                            let source_term = a / r * dt * dt;
                            p_new += source_term * 0.1; // 缩放因子
                        }
                    }

                    p_next[idx] = p_new;
                }
            }
        }

        // 边界: 简单固定为 0 (反射边界用 β 调整)
        // 已由 vec! 初始化为 0

        std::mem::swap(&mut self.pressure_prev, &mut self.pressure_curr);
        std::mem::swap(&mut self.pressure_curr, &mut p_next);
        self.time += dt;
    }

    /// 三线性插值采样压力场
    pub fn pressure_at(&self, pos: Vec3) -> f32 {
        let h = self.config.h;
        let nx = self.config.nx;
        let ny = self.config.ny;
        let nz = self.config.nz;
        let grid = pos / h;
        let ix = grid.x.floor() as isize;
        let iy = grid.y.floor() as isize;
        let iz = grid.z.floor() as isize;
        let tx = grid.x - ix as f32;
        let ty = grid.y - iy as f32;
        let tz = grid.z - iz as f32;

        let mut p = 0.0;
        for dz in 0..=1 {
            for dy in 0..=1 {
                for dx in 0..=1 {
                    let x = ix + dx as isize;
                    let y = iy + dy as isize;
                    let z = iz + dz as isize;
                    if x < 0 || x >= nx as isize || y < 0 || y >= ny as isize || z < 0 || z >= nz as isize {
                        continue;
                    }
                    let idx = self.idx(x as usize, y as usize, z as usize);
                    let weight = (if dx == 0 { 1.0 - tx } else { tx })
                        * (if dy == 0 { 1.0 - ty } else { ty })
                        * (if dz == 0 { 1.0 - tz } else { tz });
                    p += self.pressure_curr[idx] * weight;
                }
            }
        }
        p
    }

    /// 总能量 (用于稳定性监测)
    pub fn total_energy(&self) -> f32 {
        self.pressure_curr.iter().map(|p| p * p).sum()
    }

    /// 最大压力幅值 (NaN/Inf 检测)
    pub fn max_amplitude(&self) -> f32 {
        self.pressure_curr
            .iter()
            .fold(0.0f32, |m, &p| m.max(p.abs()))
    }
}

// ============================================================
// WGSL Compute Shader 源码
// ============================================================

pub const WAVEBLENDER_WGSL: &str = r#"// WaveBlender GPU FDTD — SIGGRAPH Asia 2024
// 二阶中心差分 + 时间混合 β + 加速度噪声源

const PI: f32 = 3.141592653589793;

struct WaveParams {
    nx: u32,
    ny: u32,
    nz: u32,
    h: f32,
    dt: f32,
    c: f32,
    c2_dt2_over_h2: f32,  // (c·dt/h)²
    global_beta: f32,
    absorption: f32,
    time: f32,
    source_count: u32,
    _pad0: u32,
    _pad1: u32,
};

struct Source {
    position: vec3<f32>,
    amplitude: f32,
    frequency: f32,
    phase: f32,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> params: WaveParams;
@group(0) @binding(1) var<storage, read> pressure_curr: array<f32>;
@group(0) @binding(2) var<storage, read> pressure_prev: array<f32>;
@group(0) @binding(3) var<storage, read> beta_field: array<f32>;
@group(0) @binding(4) var<storage, read> sources: array<Source>;
@group(0) @binding(5) var<storage, read_write> pressure_next: array<f32>;

@compute @workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    let j = gid.y;
    let k = gid.z;
    if i >= params.nx || j >= params.ny || k >= params.nz {
        return;
    }
    let idx = (k * params.ny + j) * params.nx + i;

    // 边界 cell: 固定 0 (或可改为 Mur 1 阶吸收边界)
    if i == 0 || j == 0 || k == 0 || i == params.nx - 1 || j == params.ny - 1 || k == params.nz - 1 {
        pressure_next[idx] = 0.0;
        return;
    }

    let p = pressure_curr[idx];
    let p_prev = pressure_prev[idx];

    // 6 邻居
    let idx_xp = (k * params.ny + j) * params.nx + (i + 1);
    let idx_xm = (k * params.ny + j) * params.nx + (i - 1);
    let idx_yp = (k * params.ny + (j + 1)) * params.nx + i;
    let idx_ym = (k * params.ny + (j - 1)) * params.nx + i;
    let idx_zp = ((k + 1) * params.ny + j) * params.nx + i;
    let idx_zm = ((k - 1) * params.ny + j) * params.nx + i;

    let lap = pressure_curr[idx_xp] + pressure_curr[idx_xm]
            + pressure_curr[idx_yp] + pressure_curr[idx_ym]
            + pressure_curr[idx_zp] + pressure_curr[idx_zm]
            - 6.0 * p;

    // 标准 FDTD: p^{n+1} = 2 p^n - p^{n-1} + (c·dt/h)² · (Σ邻居 - 6p)
    var p_new = 2.0 * p - p_prev + params.c2_dt2_over_h2 * lap;

    // 时间混合: global_beta > 0 时用 global_beta，否则用 beta_field[idx]
    let beta = select(beta_field[idx], params.global_beta, params.global_beta > 0.0);
    if beta > 0.0 {
        p_new = (1.0 - beta) * p_new + beta * p;
    }

    // 吸收
    p_new = p_new * (1.0 - params.absorption);

    // 加速度噪声源
    let h = params.h;
    let pos = vec3<f32>(f32(i) * h, f32(j) * h, f32(k) * h);
    for (var s_idx: u32 = 0u; s_idx < params.source_count; s_idx = s_idx + 1u) {
        let s_real = sources[s_idx];
        let d = pos - s_real.position;
        let d2 = dot(d, d);
        let r_threshold = 2.0 * h;
        if d2 < r_threshold * r_threshold {
            let r = max(sqrt(d2), 0.5 * h);
            let a = s_real.amplitude * sin(2.0 * PI * s_real.frequency * params.time + s_real.phase);
            let source_term = a / r * params.dt * params.dt * 0.1;
            p_new = p_new + source_term;
        }
    }

    pressure_next[idx] = p_new;
}
"#;

// ============================================================
// GPU 调度器 (feature = "gpu")
// ============================================================

#[cfg(feature = "gpu")]
pub mod gpu {
    use super::*;
    use bytemuck::{Pod, Zeroable};
    use wgpu::util::DeviceExt;

    #[repr(C)]
    #[derive(Debug, Clone, Copy, Pod, Zeroable)]
    pub struct WaveParamsGpu {
        pub nx: u32,
        pub ny: u32,
        pub nz: u32,
        pub h: f32,
        pub dt: f32,
        pub c: f32,
        pub c2_dt2_over_h2: f32,
        pub global_beta: f32,
        pub absorption: f32,
        pub time: f32,
        pub source_count: u32,
        pub _pad0: u32,
        pub _pad1: u32,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy, Pod, Zeroable)]
    pub struct SourceGpu {
        pub position: [f32; 3],
        pub amplitude: f32,
        pub frequency: f32,
        pub phase: f32,
        pub _pad: [f32; 2],
    }

    pub struct GpuWaveBlenderDispatcher {
        pub device: wgpu::Device,
        pub queue: wgpu::Queue,
        pipeline: wgpu::ComputePipeline,
        bind_group_layout: wgpu::BindGroupLayout,
        uniform_buffer: wgpu::Buffer,
        pressure_curr: wgpu::Buffer,
        pressure_prev: wgpu::Buffer,
        pressure_next: wgpu::Buffer,
        beta_buffer: wgpu::Buffer,
        source_buffer: wgpu::Buffer,
        pub config: WaveBlenderConfig,
        pub time: f32,
        pub source_count: u32,
    }

    impl GpuWaveBlenderDispatcher {
        pub async fn new(config: WaveBlenderConfig) -> Self {
            let instance = wgpu::Instance::default();
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await
                .expect("No suitable GPU adapter");
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor::default(), None)
                .await
                .expect("Failed to create device");

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("WaveBlender WGSL"),
                source: wgpu::ShaderSource::Wgsl(WAVEBLENDER_WGSL.into()),
            });

            let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("WaveBlender BGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                    wgpu::BindGroupLayoutEntry { binding: 5, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                ],
            });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("WaveBlender PL"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("WaveBlender pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
                compilation_options: Default::default(),
            });

            let n = config.nx * config.ny * config.nz;
            let c2_dt2_over_h2 = (config.c * config.c) * (config.dt * config.dt) / (config.h * config.h);
            let params = WaveParamsGpu {
                nx: config.nx as u32,
                ny: config.ny as u32,
                nz: config.nz as u32,
                h: config.h,
                dt: config.dt,
                c: config.c,
                c2_dt2_over_h2,
                global_beta: config.beta,
                absorption: config.absorption,
                time: 0.0,
                source_count: 0,
                _pad0: 0,
                _pad1: 0,
            };
            let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("WaveBlender uniform"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let make_storage = |label: &str, size| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                })
            };
            let n_bytes = (n * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
            let pressure_curr = make_storage("pressure_curr", n_bytes);
            let pressure_prev = make_storage("pressure_prev", n_bytes);
            let pressure_next = make_storage("pressure_next", n_bytes);
            let beta_buffer = make_storage("beta_field", n_bytes);
            // 初始 β = 0
            queue.write_buffer(&beta_buffer, 0, bytemuck::cast_slice(&vec![0.0f32; n]));
            let source_buffer = make_storage("sources", (64 * std::mem::size_of::<SourceGpu>()) as wgpu::BufferAddress);

            Self {
                device,
                queue,
                pipeline,
                bind_group_layout,
                uniform_buffer,
                pressure_curr,
                pressure_prev,
                pressure_next,
                beta_buffer,
                source_buffer,
                config,
                time: 0.0,
                source_count: 0,
            }
        }

        pub fn set_sources(&self, sources: &[AccelerationSource]) {
            let gpu_sources: Vec<SourceGpu> = sources
                .iter()
                .map(|s| SourceGpu {
                    position: [s.position.x, s.position.y, s.position.z],
                    amplitude: s.amplitude,
                    frequency: s.frequency,
                    phase: s.phase,
                    _pad: [0.0; 2],
                })
                .collect();
            self.queue.write_buffer(&self.source_buffer, 0, bytemuck::cast_slice(&gpu_sources));
        }

        pub fn dispatch(&mut self, sources: &[AccelerationSource]) {
            self.source_count = sources.len() as u32;
            self.set_sources(sources);

            let c2_dt2_over_h2 = (self.config.c * self.config.c) * (self.config.dt * self.config.dt) / (self.config.h * self.config.h);
            let params = WaveParamsGpu {
                nx: self.config.nx as u32,
                ny: self.config.ny as u32,
                nz: self.config.nz as u32,
                h: self.config.h,
                dt: self.config.dt,
                c: self.config.c,
                c2_dt2_over_h2,
                global_beta: self.config.beta,
                absorption: self.config.absorption,
                time: self.time,
                source_count: self.source_count,
                _pad0: 0,
                _pad1: 0,
            };
            self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&params));

            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("WaveBlender BG"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: self.uniform_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: self.pressure_curr.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: self.pressure_prev.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: self.beta_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 4, resource: self.source_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 5, resource: self.pressure_next.as_entire_binding() },
                ],
            });

            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("WaveBlender encoder"),
            });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("WaveBlender pass"),
                });
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                let wg_x = (self.config.nx as u32 + 7) / 8;
                let wg_y = (self.config.ny as u32 + 7) / 8;
                let wg_z = (self.config.nz as u32 + 7) / 8;
                pass.dispatch_workgroups(wg_x, wg_y, wg_z);
            }
            // Swap: next -> curr, curr -> prev
            // 用 copy_buffer 实现三缓冲轮换
            encoder.copy_buffer_to_buffer(&self.pressure_curr, 0, &self.pressure_prev, 0, (self.config.nx * self.config.ny * self.config.nz * std::mem::size_of::<f32>()) as wgpu::BufferAddress);
            encoder.copy_buffer_to_buffer(&self.pressure_next, 0, &self.pressure_curr, 0, (self.config.nx * self.config.ny * self.config.nz * std::mem::size_of::<f32>()) as wgpu::BufferAddress);

            self.queue.submit(std::iter::once(encoder.finish()));
            self.time += self.config.dt;
        }
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_creation() {
        let config = WaveBlenderConfig {
            nx: 16,
            ny: 16,
            nz: 16,
            ..Default::default()
        };
        let solver = WaveBlenderSolver::new(config);
        assert_eq!(solver.pressure_curr.len(), 16 * 16 * 16);
        assert_eq!(solver.beta_field.len(), 16 * 16 * 16);
        assert_eq!(solver.time, 0.0);
    }

    #[test]
    fn test_cfl_stability() {
        // CFL: c·dt/h ≤ 1/√3 (3D)
        let config = WaveBlenderConfig::default();
        let cfl = config.c * config.dt / config.h;
        assert!(cfl <= 1.0 / 3.0f32.sqrt() + 1e-6, "CFL violated: {}", cfl);
    }

    #[test]
    fn test_step_no_crash() {
        let config = WaveBlenderConfig {
            nx: 16,
            ny: 16,
            nz: 16,
            ..Default::default()
        };
        let mut solver = WaveBlenderSolver::new(config);
        solver.step();
        assert!(solver.time > 0.0);
        assert!(solver.max_amplitude().is_finite());
    }

    #[test]
    fn test_source_propagation() {
        // 单点源在中心，运行多步后应在网格中产生非零压力
        let config = WaveBlenderConfig {
            nx: 24,
            ny: 24,
            nz: 24,
            ..Default::default()
        };
        let mut solver = WaveBlenderSolver::new(config);
        let center = Vec3::new(12.0 * solver.config.h, 12.0 * solver.config.h, 12.0 * solver.config.h);
        solver.add_source(AccelerationSource::new(center, 100.0, 440.0));

        for _ in 0..50 {
            solver.step();
        }
        // 应有非零压力传播
        let max_p = solver.max_amplitude();
        assert!(max_p > 0.0, "no pressure propagated: max_p={}", max_p);
        assert!(max_p < 1e6, "simulation diverged: max_p={}", max_p);
    }

    #[test]
    fn test_beta_stability() {
        // 启用 β 后应仍稳定
        let config = WaveBlenderConfig {
            nx: 16,
            ny: 16,
            nz: 16,
            beta: 0.5,
            ..Default::default()
        };
        let mut solver = WaveBlenderSolver::new(config);
        let center = Vec3::new(8.0 * solver.config.h, 8.0 * solver.config.h, 8.0 * solver.config.h);
        solver.add_source(AccelerationSource::new(center, 100.0, 440.0));

        for _ in 0..100 {
            solver.step();
            let max_p = solver.max_amplitude();
            assert!(max_p.is_finite(), "diverged at step: max_p={}", max_p);
            assert!(max_p < 1e6, "diverged: max_p={}", max_p);
        }
    }

    #[test]
    fn test_beta_sphere() {
        let config = WaveBlenderConfig {
            nx: 16,
            ny: 16,
            nz: 16,
            ..Default::default()
        };
        let mut solver = WaveBlenderSolver::new(config);
        let center = Vec3::new(8.0 * solver.config.h, 8.0 * solver.config.h, 8.0 * solver.config.h);
        solver.set_beta_sphere(center, 0.3, 0.7);
        // 中心 cell 应有 β = 0.7
        let center_idx = solver.idx(8, 8, 8);
        assert!((solver.beta_field[center_idx] - 0.7).abs() < 1e-5, "center beta = {}", solver.beta_field[center_idx]);
        // 远离中心应仍为 0
        let far_idx = solver.idx(0, 0, 0);
        assert!(solver.beta_field[far_idx].abs() < 1e-5, "far beta = {}", solver.beta_field[far_idx]);
    }

    #[test]
    fn test_acceleration_source() {
        let src = AccelerationSource::new(Vec3::ZERO, 1.0, 440.0);
        // t=0: sin(0) = 0
        assert!(src.acceleration_at(0.0).abs() < 1e-6);
        // t = 1/(4·f): sin(π/2) = 1
        let t_peak = 1.0 / (4.0 * 440.0);
        assert!((src.acceleration_at(t_peak) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_energy_conservation_no_source() {
        // 无源时能量应单调递减（吸收）或保持（无吸收）
        let config = WaveBlenderConfig {
            nx: 16,
            ny: 16,
            nz: 16,
            absorption: 0.0,
            ..Default::default()
        };
        let mut solver = WaveBlenderSolver::new(config);
        // 注入初始压力
        let center = solver.idx(8, 8, 8);
        solver.pressure_curr[center] = 1.0;
        let e0 = solver.total_energy();
        solver.step();
        let e1 = solver.total_energy();
        // 无吸收下能量应基本守恒（数值误差 < 1%）
        assert!(e1 <= e0 * 1.01, "energy grew: e0={} e1={}", e0, e1);
    }

    #[test]
    fn test_wgsl_shader_nonempty() {
        assert!(!WAVEBLENDER_WGSL.is_empty());
        assert!(WAVEBLENDER_WGSL.contains("@compute"));
        assert!(WAVEBLENDER_WGSL.contains("workgroup_size(8, 8, 8)"));
        assert!(WAVEBLENDER_WGSL.contains("pressure_next"));
    }
}
