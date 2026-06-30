//! Acoustic FDTD Solver (Wave Equation)
//!
//! 声波传播模拟. 标量波动方程的有限差分时域求解.
//!
//! 方程:
//!   d^2 p / dt^2 = c^2 * nabla^2 p
//!
//! 其中 p = 声压, c = 声速
//!
//! FDTD 离散 (二阶时间, 中心空间):
//!   p^{n+1} = 2*p^n - p^{n-1} + (c*dt/dx)^2 * (sum_neighbors - 2d*p^n)
//!
//! CFL 稳定性条件:
//!   c * dt / dx <= 1 / sqrt(d)   (d = 维度)
//!
//! 边界条件:
//!   Rigid     - 硬墙 (法向梯度 = 0, 全反射)
//!   Absorbing - sponge 衰减层 (近似开放边界)
//!   Periodic  - 周期性
//!
//! 声源:
//!   GaussianPulse - 高斯脉冲 (宽频)
//!   Sinusoidal    - 正弦源 (单频)
//!   Ricker        - Ricker 小波 (地震波常用)
//!
//! 应用: 房间声学, 超声成像, 地震波, 乐器共振, 噪声控制.
//!
//! 基于 Pierce 1981, Botteldooren 1995, Voelz 2009.

use serde::{Deserialize, Serialize};

/// 空气中声速 (m/s)
pub const C_AIR: f32 = 343.0;
/// 水中声速 (m/s)
pub const C_WATER: f32 = 1480.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AcousticBoundary {
    /// 硬墙 (法向梯度 = 0, 全反射)
    Rigid,
    /// 吸收 (sponge 衰减层)
    Absorbing { layer: usize, strength: f32 },
    /// 周期
    Periodic,
}

impl Default for AcousticBoundary {
    fn default() -> Self {
        AcousticBoundary::Rigid
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcousticConfig {
    pub nx: usize,
    pub ny: usize,
    pub dx: f32,
    pub dt: f32,
    pub c: f32,
    pub boundary: AcousticBoundary,
}

impl Default for AcousticConfig {
    fn default() -> Self {
        AcousticConfig {
            nx: 128,
            ny: 128,
            dx: 0.1,
            dt: 0.0001,
            c: C_AIR,
            boundary: AcousticBoundary::Rigid,
        }
    }
}

impl AcousticConfig {
    pub fn dims(&self) -> usize {
        if self.ny <= 1 { 1 } else { 2 }
    }
    pub fn n_cells(&self) -> usize {
        self.nx * self.ny.max(1)
    }
    /// CFL 数: c * dt / dx
    pub fn courant(&self) -> f32 {
        self.c * self.dt / self.dx
    }
    /// CFL 稳定性上限: 1 / sqrt(d)
    pub fn cfl_limit(&self) -> f32 {
        1.0 / (self.dims() as f32).sqrt()
    }
    pub fn is_stable(&self) -> bool {
        self.courant() <= self.cfl_limit()
    }
    /// 稳定时间步长上限
    pub fn stable_dt(&self) -> f32 {
        self.cfl_limit() * self.dx / self.c
    }
}

#[derive(Debug, Clone)]
pub enum AcousticSource {
    /// 高斯脉冲 (宽频源)
    GaussianPulse { center: [usize; 2], width: f32, amplitude: f32 },
    /// 正弦源 (单频)
    Sinusoidal { center: [usize; 2], frequency: f32, amplitude: f32 },
    /// Ricker 小波 (地震波)
    Ricker { center: [usize; 2], frequency: f32, amplitude: f32, t0: f32 },
}

pub struct AcousticSolver {
    pub config: AcousticConfig,
    pub p_curr: Vec<f32>,
    pub p_prev: Vec<f32>,
    pub p_next: Vec<f32>,
    pub sources: Vec<AcousticSource>,
    /// sponge 衰减系数 (Rigid/Periodic 时为空)
    pub damping: Vec<f32>,
    pub time: f32,
    pub steps: usize,
}

impl AcousticSolver {
    pub fn new(config: AcousticConfig) -> Self {
        let n = config.n_cells();
        let damping = Self::build_damping(&config);
        AcousticSolver {
            config,
            p_curr: vec![0.0; n],
            p_prev: vec![0.0; n],
            p_next: vec![0.0; n],
            sources: Vec::new(),
            damping,
            time: 0.0,
            steps: 0,
        }
    }

    fn build_damping(config: &AcousticConfig) -> Vec<f32> {
        let n = config.n_cells();
        match config.boundary {
            AcousticBoundary::Absorbing { layer, strength } => {
                let nx = config.nx;
                let ny = config.ny.max(1);
                let mut d = vec![1.0; n];
                for j in 0..ny {
                    for i in 0..nx {
                        let idx = i + nx * j;
                        let mut dist = layer;
                        if i < layer { dist = dist.min(i); }
                        if i >= nx - layer { dist = dist.min(nx - 1 - i); }
                        if ny > 1 {
                            if j < layer { dist = dist.min(j); }
                            if j >= ny - layer { dist = dist.min(ny - 1 - j); }
                        }
                        if dist < layer {
                            let frac = 1.0 - dist as f32 / layer as f32;
                            d[idx] = (1.0 - strength * frac * frac).max(0.0);
                        }
                    }
                }
                d
            }
            _ => vec![1.0; n],
        }
    }

    pub fn idx(&self, i: usize, j: usize) -> usize {
        i + self.config.nx * j
    }

    pub fn add_source(&mut self, source: AcousticSource) {
        self.sources.push(source);
    }

    fn wrap(i: i32, n: usize) -> usize {
        let m = n as i32;
        (((i % m) + m) % m) as usize
    }

    /// 应用声源
    fn apply_sources(&mut self) {
        let nx = self.config.nx;
        let n = self.p_curr.len();
        let t = self.time;
        for src in &self.sources {
            let (cx, cy) = (src.center()[0], src.center()[1]);
            let value = src.value_at(t);
            let idx = cx + nx * cy;
            if idx < n {
                self.p_curr[idx] += value;
            }
        }
    }

    /// 一步 FDTD 更新
    pub fn step(&mut self) {
        let nx = self.config.nx;
        let ny = self.config.ny.max(1);
        let two_d = self.config.dims() == 2;
        let cfl2 = {
            let cfl = self.config.courant();
            cfl * cfl
        };
        // 先应用声源到 p_curr
        self.apply_sources();
        // FDTD 更新
        for j in 0..ny {
            for i in 0..nx {
                let idx = self.idx(i, j);
                let p = self.p_curr[idx];
                let pp = self.p_prev[idx];
                // 空间拉普拉斯 (考虑边界)
                let (ip, im, jp, jm) = match self.config.boundary {
                    AcousticBoundary::Periodic => (
                        Self::wrap((i + 1) as i32, nx),
                        Self::wrap((i as i32) - 1, nx),
                        if two_d { Self::wrap((j + 1) as i32, ny) } else { 0 },
                        if two_d { Self::wrap((j as i32) - 1, ny) } else { 0 },
                    ),
                    _ => (
                        (i + 1).min(nx - 1),
                        if i > 0 { i - 1 } else { 0 },
                        if two_d { (j + 1).min(ny - 1) } else { 0 },
                        if two_d && j > 0 { j - 1 } else { 0 },
                    ),
                };
                let p_ip = self.p_curr[self.idx(ip, j)];
                let p_im = self.p_curr[self.idx(im, j)];
                let lap = if two_d {
                    let p_jp = self.p_curr[self.idx(i, jp)];
                    let p_jm = self.p_curr[self.idx(i, jm)];
                    (p_ip + p_im + p_jp + p_jm - 4.0 * p)
                } else {
                    (p_ip + p_im - 2.0 * p)
                };
                let d = self.damping[idx];
                let new_p = (2.0 * p - pp + cfl2 * lap) * d;
                self.p_next[idx] = new_p;
            }
        }
        // 轮转缓冲: prev <- curr, curr <- next
        std::mem::swap(&mut self.p_prev, &mut self.p_curr);
        std::mem::swap(&mut self.p_curr, &mut self.p_next);
        self.time += self.config.dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n { self.step(); }
    }

    /// 注入高斯脉冲初始条件 (无源)
    pub fn initialize_gaussian_pulse(&mut self, center: [f32; 2], width: f32, amplitude: f32) {
        let nx = self.config.nx;
        let ny = self.config.ny.max(1);
        let two_d = self.config.dims() == 2;
        let inv_w2 = 1.0 / (width * width);
        for j in 0..ny {
            for i in 0..nx {
                let x = (i as f32) * self.config.dx;
                let y = (j as f32) * self.config.dx;
                let dx = x - center[0];
                let dy = y - center[1];
                let r2 = if two_d { dx * dx + dy * dy } else { dx * dx };
                let idx = self.idx(i, j);
                let val = amplitude * (-0.5 * r2 * inv_w2).exp();
                self.p_curr[idx] = val;
                self.p_prev[idx] = val;
            }
        }
    }

    /// 总能量 (1/2 * (dp/dt)^2 + 1/2 * c^2 * |grad p|^2 的离散近似)
    pub fn energy(&self) -> f32 {
        let nx = self.config.nx;
        let ny = self.config.ny.max(1);
        let two_d = self.config.dims() == 2;
        let c2 = self.config.c * self.config.c;
        let dx2 = self.config.dx * self.config.dx;
        let dt2 = self.config.dt * self.config.dt;
        let mut e = 0.0f32;
        for j in 0..ny {
            for i in 0..nx {
                let idx = self.idx(i, j);
                let dp_dt = (self.p_curr[idx] - self.p_prev[idx]) / self.config.dt;
                let e_kin = 0.5 * dp_dt * dp_dt * dt2;
                let ip = (i + 1).min(nx - 1);
                let jp = if two_d { (j + 1).min(ny - 1) } else { 0 };
                let dp_dx = (self.p_curr[self.idx(ip, j)] - self.p_curr[idx]) / self.config.dx;
                let grad2 = if two_d {
                    let dp_dy = (self.p_curr[self.idx(i, jp)] - self.p_curr[idx]) / self.config.dx;
                    dp_dx * dp_dx + dp_dy * dp_dy
                } else {
                    dp_dx * dp_dx
                };
                let e_pot = 0.5 * c2 * grad2 * dt2 / dx2 * dx2;
                e += e_kin + e_pot;
            }
        }
        e
    }

    /// 最大声压
    pub fn max_pressure(&self) -> f32 {
        self.p_curr.iter().map(|p| p.abs()).fold(0.0f32, f32::max)
    }

    /// RMS 声压
    pub fn rms_pressure(&self) -> f32 {
        let n = self.p_curr.len() as f32;
        let sum: f32 = self.p_curr.iter().map(|p| p * p).sum();
        (sum / n).sqrt()
    }

    pub fn reset(&mut self) {
        for p in self.p_curr.iter_mut() { *p = 0.0; }
        for p in self.p_prev.iter_mut() { *p = 0.0; }
        for p in self.p_next.iter_mut() { *p = 0.0; }
        self.time = 0.0;
        self.steps = 0;
    }
}

impl AcousticSource {
    pub fn center(&self) -> [usize; 2] {
        match self {
            AcousticSource::GaussianPulse { center, .. } => *center,
            AcousticSource::Sinusoidal { center, .. } => *center,
            AcousticSource::Ricker { center, .. } => *center,
        }
    }

    pub fn value_at(&self, t: f32) -> f32 {
        match self {
            AcousticSource::GaussianPulse { amplitude, width, .. } => {
                // 宽带脉冲: 高斯包络
                let t0 = 3.0 * width;
                if t > 2.0 * t0 {
                    0.0
                } else {
                    amplitude * (-((t - t0) / width).powi(2)).exp()
                }
            }
            AcousticSource::Sinusoidal { frequency, amplitude, .. } => {
                amplitude * (2.0 * std::f32::consts::PI * frequency * t).sin()
            }
            AcousticSource::Ricker { frequency, amplitude, t0, .. } => {
                let arg = std::f32::consts::PI * frequency * (t - t0);
                let w = arg * arg;
                amplitude * (1.0 - 2.0 * w) * (-w).exp()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn test_speed_of_sound_constants() {
        assert_eq!(C_AIR, 343.0);
        assert_eq!(C_WATER, 1480.0);
        assert!(C_WATER > C_AIR);
    }

    #[test]
    fn test_boundary_default() {
        assert_eq!(AcousticBoundary::default(), AcousticBoundary::Rigid);
    }

    #[test]
    fn test_config_default() {
        let c = AcousticConfig::default();
        assert_eq!(c.nx, 128);
        assert_eq!(c.ny, 128);
        assert_eq!(c.dx, 0.1);
        assert_eq!(c.dt, 0.0001);
        assert_eq!(c.c, C_AIR);
        assert_eq!(c.boundary, AcousticBoundary::Rigid);
    }

    #[test]
    fn test_dims_1d() {
        let c = AcousticConfig { nx: 64, ny: 1, ..Default::default() };
        assert_eq!(c.dims(), 1);
    }

    #[test]
    fn test_dims_2d() {
        let c = AcousticConfig { nx: 64, ny: 64, ..Default::default() };
        assert_eq!(c.dims(), 2);
    }

    #[test]
    fn test_n_cells() {
        let c = AcousticConfig { nx: 32, ny: 16, ..Default::default() };
        assert_eq!(c.n_cells(), 512);
    }

    #[test]
    fn test_n_cells_1d() {
        let c = AcousticConfig { nx: 64, ny: 0, ..Default::default() };
        assert_eq!(c.n_cells(), 64);
    }

    #[test]
    fn test_courant() {
        let c = AcousticConfig { nx: 100, ny: 1, dx: 0.1, dt: 0.0001, c: 343.0, ..Default::default() };
        assert!(approx_eq(c.courant(), 0.343, 1e-4));
    }

    #[test]
    fn test_cfl_limit_1d() {
        let c = AcousticConfig { nx: 100, ny: 1, ..Default::default() };
        assert!(approx_eq(c.cfl_limit(), 1.0, 1e-6));
    }

    #[test]
    fn test_cfl_limit_2d() {
        let c = AcousticConfig { nx: 100, ny: 100, ..Default::default() };
        let expected = 1.0 / 2.0f32.sqrt();
        assert!(approx_eq(c.cfl_limit(), expected, 1e-5));
    }

    #[test]
    fn test_is_stable_default() {
        let c = AcousticConfig::default();
        assert!(c.is_stable());
    }

    #[test]
    fn test_is_stable_violated() {
        let c = AcousticConfig { nx: 10, ny: 10, dx: 0.01, dt: 0.001, c: 343.0, ..Default::default() };
        assert!(!c.is_stable());
    }

    #[test]
    fn test_stable_dt_1d() {
        let c = AcousticConfig { nx: 100, ny: 1, dx: 0.1, c: 343.0, ..Default::default() };
        let expected = 0.1 / 343.0;
        assert!(approx_eq(c.stable_dt(), expected, 1e-9));
    }

    #[test]
    fn test_solver_new() {
        let s = AcousticSolver::new(AcousticConfig { nx: 32, ny: 16, ..Default::default() });
        assert_eq!(s.p_curr.len(), 32 * 16);
        assert_eq!(s.p_prev.len(), 32 * 16);
        assert_eq!(s.p_next.len(), 32 * 16);
        assert_eq!(s.damping.len(), 32 * 16);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
        for &p in s.p_curr.iter() { assert_eq!(p, 0.0); }
    }

    #[test]
    fn test_solver_idx() {
        let s = AcousticSolver::new(AcousticConfig { nx: 16, ny: 8, ..Default::default() });
        assert_eq!(s.idx(0, 0), 0);
        assert_eq!(s.idx(1, 0), 1);
        assert_eq!(s.idx(0, 1), 16);
        assert_eq!(s.idx(15, 7), 15 + 16 * 7);
    }

    #[test]
    fn test_damping_rigid() {
        let s = AcousticSolver::new(AcousticConfig {
            nx: 32, ny: 16, boundary: AcousticBoundary::Rigid, ..Default::default()
        });
        for &d in s.damping.iter() { assert_eq!(d, 1.0); }
    }

    #[test]
    fn test_damping_periodic() {
        let s = AcousticSolver::new(AcousticConfig {
            nx: 32, ny: 16, boundary: AcousticBoundary::Periodic, ..Default::default()
        });
        for &d in s.damping.iter() { assert_eq!(d, 1.0); }
    }

    #[test]
    fn test_damping_absorbing_corner() {
        let s = AcousticSolver::new(AcousticConfig {
            nx: 32, ny: 32, boundary: AcousticBoundary::Absorbing { layer: 4, strength: 0.5 },
            ..Default::default()
        });
        let corner = s.damping[s.idx(0, 0)];
        let center = s.damping[s.idx(16, 16)];
        assert!(corner < 1.0, "corner should be damped: {}", corner);
        assert_eq!(center, 1.0, "center should be unattenuated");
        assert!(approx_eq(corner, 0.5, 1e-5));
    }

    #[test]
    fn test_add_source() {
        let mut s = AcousticSolver::new(AcousticConfig { nx: 32, ny: 32, ..Default::default() });
        assert_eq!(s.sources.len(), 0);
        s.add_source(AcousticSource::Sinusoidal {
            center: [16, 16], frequency: 1000.0, amplitude: 1.0,
        });
        assert_eq!(s.sources.len(), 1);
    }

    #[test]
    fn test_source_center() {
        let src = AcousticSource::GaussianPulse { center: [5, 7], width: 0.001, amplitude: 1.0 };
        assert_eq!(src.center(), [5, 7]);
    }

    #[test]
    fn test_gaussian_pulse_value_at_peak() {
        let src = AcousticSource::GaussianPulse { center: [0, 0], width: 0.001, amplitude: 1.0 };
        let t0 = 3.0 * 0.001_f32;
        let v = src.value_at(t0);
        assert!(approx_eq(v, 1.0, 1e-5));
    }

    #[test]
    fn test_gaussian_pulse_value_far() {
        let src = AcousticSource::GaussianPulse { center: [0, 0], width: 0.001, amplitude: 1.0 };
        let v = src.value_at(0.01);
        assert_eq!(v, 0.0);
    }

    #[test]
    fn test_sinusoidal_value() {
        let src = AcousticSource::Sinusoidal { center: [0, 0], frequency: 1000.0, amplitude: 1.0 };
        assert!(approx_eq(src.value_at(0.0), 0.0, 1e-6));
        let t = 1.0 / (4.0 * 1000.0);
        assert!(approx_eq(src.value_at(t), 1.0, 1e-5));
    }

    #[test]
    fn test_ricker_value_at_t0() {
        let src = AcousticSource::Ricker {
            center: [0, 0], frequency: 1000.0, amplitude: 1.0, t0: 0.001,
        };
        let v = src.value_at(0.001);
        assert!(approx_eq(v, 1.0, 1e-5));
    }

    #[test]
    fn test_ricker_value_far() {
        let src = AcousticSource::Ricker {
            center: [0, 0], frequency: 1000.0, amplitude: 1.0, t0: 0.001,
        };
        let v = src.value_at(10.0);
        assert!(v.abs() < 1e-10);
    }
    #[test]
    fn test_step_advances_time() {
        let mut s = AcousticSolver::new(AcousticConfig {
            nx: 32, ny: 1, dx: 0.1, dt: 0.0001, c: 343.0,
            ..Default::default()
        });
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
        s.step();
        assert!(approx_eq(s.time, 0.0001, 1e-6));
        assert_eq!(s.steps, 1);
        s.step();
        assert!(approx_eq(s.time, 0.0002, 1e-6));
        assert_eq!(s.steps, 2);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = AcousticSolver::new(AcousticConfig {
            nx: 32, ny: 1, dx: 0.1, dt: 0.0001, c: 343.0,
            ..Default::default()
        });
        s.step_n(100);
        assert_eq!(s.steps, 100);
        assert!(approx_eq(s.time, 0.01, 1e-6));
    }

    #[test]
    fn test_gaussian_pulse_initialization() {
        let mut s = AcousticSolver::new(AcousticConfig {
            nx: 32, ny: 1, dx: 0.1, dt: 0.0001, c: 343.0,
            ..Default::default()
        });
        s.initialize_gaussian_pulse([1.6, 0.0], 0.5, 1.0);
        let center_val = s.p_curr[s.idx(16, 0)];
        assert!(approx_eq(center_val, 1.0, 1e-5));
        let far_val = s.p_curr[s.idx(0, 0)];
        assert!(far_val.abs() < 0.01);
    }

    #[test]
    fn test_gaussian_pulse_2d_center() {
        let mut s = AcousticSolver::new(AcousticConfig {
            nx: 32, ny: 32, dx: 0.1, dt: 0.0001, c: 343.0,
            ..Default::default()
        });
        s.initialize_gaussian_pulse([1.6, 1.6], 0.5, 1.0);
        let center = s.p_curr[s.idx(16, 16)];
        assert!(approx_eq(center, 1.0, 1e-5));
    }

    #[test]
    fn test_1d_propagation_symmetric() {
        let mut s = AcousticSolver::new(AcousticConfig {
            nx: 200, ny: 1, dx: 0.1, dt: 0.0001, c: 343.0,
            boundary: AcousticBoundary::Periodic,
            ..Default::default()
        });
        let center_i = 100;
        let center_x = center_i as f32 * 0.1;
        s.initialize_gaussian_pulse([center_x, 0.0], 0.3, 1.0);
        s.step_n(50);
        let left = s.p_curr[s.idx(80, 0)].abs();
        let right = s.p_curr[s.idx(120, 0)].abs();
        assert!(left > 0.0, "left should have propagated signal");
        assert!(right > 0.0, "right should have propagated signal");
        assert!((left - right).abs() < 0.05, "should be symmetric: left={}, right={}", left, right);
    }

    #[test]
    fn test_energy_non_negative() {
        let mut s = AcousticSolver::new(AcousticConfig {
            nx: 32, ny: 32, dx: 0.1, dt: 0.0001, c: 343.0,
            ..Default::default()
        });
        s.initialize_gaussian_pulse([1.6, 1.6], 0.3, 1.0);
        s.step_n(10);
        let e = s.energy();
        assert!(e >= 0.0, "energy should be non-negative: {}", e);
    }

    #[test]
    fn test_max_pressure_after_pulse() {
        let mut s = AcousticSolver::new(AcousticConfig {
            nx: 32, ny: 1, dx: 0.1, dt: 0.0001, c: 343.0,
            ..Default::default()
        });
        s.initialize_gaussian_pulse([1.6, 0.0], 0.5, 1.0);
        assert!(approx_eq(s.max_pressure(), 1.0, 1e-5));
        s.step_n(5);
        assert!(s.max_pressure() > 0.0);
    }

    #[test]
    fn test_rms_pressure_non_negative() {
        let mut s = AcousticSolver::new(AcousticConfig {
            nx: 32, ny: 32, dx: 0.1, dt: 0.0001, c: 343.0,
            ..Default::default()
        });
        s.initialize_gaussian_pulse([1.6, 1.6], 0.3, 1.0);
        s.step_n(5);
        let rms = s.rms_pressure();
        assert!(rms >= 0.0);
    }

    #[test]
    fn test_reset() {
        let mut s = AcousticSolver::new(AcousticConfig {
            nx: 32, ny: 32, dx: 0.1, dt: 0.0001, c: 343.0,
            ..Default::default()
        });
        s.initialize_gaussian_pulse([1.6, 1.6], 0.3, 1.0);
        s.step_n(10);
        assert!(s.steps > 0);
        assert!(s.max_pressure() > 0.0);
        s.reset();
        assert_eq!(s.steps, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.max_pressure(), 0.0);
        for &p in s.p_curr.iter() { assert_eq!(p, 0.0); }
        for &p in s.p_prev.iter() { assert_eq!(p, 0.0); }
        for &p in s.p_next.iter() { assert_eq!(p, 0.0); }
    }

    #[test]
    fn test_cfl_1d_stability() {
        let mut s = AcousticSolver::new(AcousticConfig {
            nx: 100, ny: 1, dx: 0.1, dt: 0.1 / 343.0, c: 343.0,
            boundary: AcousticBoundary::Periodic,
            ..Default::default()
        });
        let center_x = 50.0 * 0.1;
        s.initialize_gaussian_pulse([center_x, 0.0], 0.5, 1.0);
        s.step_n(500);
        assert!(s.max_pressure() < 10.0, "pressure should not blow up: {}", s.max_pressure());
    }

    #[test]
    fn test_cfl_2d_stability() {
        let cfl = 0.7;
        let mut s = AcousticSolver::new(AcousticConfig {
            nx: 64, ny: 64, dx: 0.1, dt: cfl * 0.1 / (343.0 * 2.0f32.sqrt()), c: 343.0,
            boundary: AcousticBoundary::Periodic,
            ..Default::default()
        });
        s.initialize_gaussian_pulse([3.2, 3.2], 0.3, 1.0);
        s.step_n(200);
        assert!(s.max_pressure() < 10.0, "pressure should not blow up: {}", s.max_pressure());
    }

    #[test]
    fn test_periodic_wrap() {
        let mut s = AcousticSolver::new(AcousticConfig {
            nx: 64, ny: 1, dx: 0.1, dt: 0.0001, c: 343.0,
            boundary: AcousticBoundary::Periodic,
            ..Default::default()
        });
        s.initialize_gaussian_pulse([0.0, 0.0], 0.3, 1.0);
        s.step_n(50);
        let max_p = s.max_pressure();
        assert!(max_p > 0.0, "wave should still be present");
    }

    #[test]
    fn test_rigid_boundary_no_flux() {
        let mut s = AcousticSolver::new(AcousticConfig {
            nx: 64, ny: 1, dx: 0.1, dt: 0.0001, c: 343.0,
            boundary: AcousticBoundary::Rigid,
            ..Default::default()
        });
        s.initialize_gaussian_pulse([3.0, 0.0], 0.3, 1.0);
        s.step_n(50);
        assert!(s.max_pressure() > 0.0);
    }

    #[test]
    fn test_absorbing_boundary_reduces_reflection() {
        let mut s_rigid = AcousticSolver::new(AcousticConfig {
            nx: 64, ny: 1, dx: 0.1, dt: 0.0001, c: 343.0,
            boundary: AcousticBoundary::Rigid,
            ..Default::default()
        });
        let mut s_absorb = AcousticSolver::new(AcousticConfig {
            nx: 64, ny: 1, dx: 0.1, dt: 0.0001, c: 343.0,
            boundary: AcousticBoundary::Absorbing { layer: 8, strength: 0.5 },
            ..Default::default()
        });
        s_rigid.initialize_gaussian_pulse([3.2, 0.0], 0.3, 1.0);
        s_absorb.initialize_gaussian_pulse([3.2, 0.0], 0.3, 1.0);
        s_rigid.step_n(300);
        s_absorb.step_n(300);
        let e_rigid = s_rigid.energy();
        let e_absorb = s_absorb.energy();
        assert!(e_absorb < e_rigid,
            "absorbing should dissipate more energy: rigid={}, absorb={}",
            e_rigid, e_absorb);
    }
}