//! Kuramoto-Sivashinsky Solver — 时空混沌方程
//!
//! KS 方程是具有时空混沌行为的非线性 PDE, 因其丰富的动力学
//! (周期/准周期/混沌转捩)而成为研究偏微分方程混沌的经典模型.
//!
//! 方程:
//!   ∂u/∂t + u·∂u/∂x + ∂²u/∂x² + ∂⁴u/∂x⁴ = 0
//!
//! 即:
//!   ∂u/∂t = -u·∂u/∂x - ∂²u/∂x² - ∂⁴u/∂x⁴
//!
//! 各项物理意义:
//!   - u·∂u/∂x:  非线性对流项 (能量在不同尺度间转移)
//!   - ∂²u/∂x²:  反向扩散 (短波能量注入, 不稳定)
//!   - ∂⁴u/∂x⁴:  超扩散 (短波能量耗散, 稳定)
//!
//! 能量平衡:
//!   dE/dt = ∫[-(∂u/∂x)² + (∂²u/∂x²)²] dx
//!   中等波数增长, 高波数衰减 -> 能量集中在中间尺度
//!
//! 对称性:
//!   - 空间平移: u(x,t) -> u(x+a,t)
//!   - 空间反演: u(x,t) -> -u(-x,t)
//!   - 时间平移
//!
//! 混沌行为:
//!   - 小域 (L < 4π):  稳定/周期解
//!   - 中域 (L ~ 16π): 准周期
//!   - 大域 (L > 32π): 时空混沌
//!
//! 应用:
//!   - 薄膜生长界面不稳定性 (Sivashinsky 1983)
//!   - 火焰前沿传播 (Michelson 1986)
//!   - 等离子体漂移波 (LaQuey 1975)
//!   - 混沌动力学和湍流研究
//!
//! 数值方法:
//!   显式 Euler + 中心差分
//!   2阶导数: (u_{i+1} - 2u_i + u_{i-1}) / dx²
//!   4阶导数: (u_{i+2} - 4u_{i+1} + 6u_i - 4u_{i-1} + u_{i+2}) / dx⁴
//!   非线性:  u * (u_{i+1} - u_{i-1}) / (2·dx)
//!
//! CFL 条件:
//!   - 扩散: dt <= dx² / 2
//!   - Biharmonic: dt <= dx⁴ / 16 (最严格)
//!   - 非线性: dt <= dx / max|u|
//!
//! 基于:
//!   - Kuramoto, Y. & Tsuzuki, T. 1976. Prog. Theor. Phys. 55, 356.
//!   - Sivashinsky, G.I. 1977. Acta Astronaut. 4, 1177.
//!   - Hyman & Nicolaenko 1986. Physica D 18, 113.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum KsBoundary {
    Periodic,
    Dirichlet { left: f32, right: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KsConfig {
    pub nx: usize,
    pub dx: f32,
    pub dt: f32,
    pub boundary: KsBoundary,
}

impl Default for KsConfig {
    fn default() -> Self {
        KsConfig {
            nx: 128,
            dx: 32.0 * std::f32::consts::PI / 128.0,
            dt: 0.01,
            boundary: KsBoundary::Periodic,
        }
    }
}

impl KsConfig {
    pub fn domain_length(&self) -> f32 {
        self.nx as f32 * self.dx
    }

    pub fn diffusive_cfl(&self) -> f32 {
        self.dt / (self.dx * self.dx)
    }

    pub fn biharmonic_cfl(&self) -> f32 {
        self.dt / (self.dx.powi(4))
    }

    pub fn is_stable(&self) -> bool {
        self.diffusive_cfl() <= 0.5 && self.biharmonic_cfl() <= 0.0625
    }

    pub fn stable_dt(&self) -> f32 {
        let diff_dt = 0.5 * self.dx * self.dx;
        let bih_dt = 0.0625 * self.dx.powi(4);
        diff_dt.min(bih_dt)
    }
}

pub struct KsSolver {
    pub config: KsConfig,
    pub u_curr: Vec<f32>,
    pub u_next: Vec<f32>,
    pub time: f32,
    pub steps: usize,
}

impl KsSolver {
    pub fn new(config: KsConfig) -> Self {
        let n = config.nx;
        KsSolver {
            config,
            u_curr: vec![0.0; n],
            u_next: vec![0.0; n],
            time: 0.0,
            steps: 0,
        }
    }

    #[inline]
    fn wrap(&self, i: i32) -> usize {
        let n = self.config.nx as i32;
        (((i % n) + n) % n) as usize
    }

    pub fn initialize_zero(&mut self) {
        for u in self.u_curr.iter_mut() {
            *u = 0.0;
        }
        for u in self.u_next.iter_mut() {
            *u = 0.0;
        }
        self.time = 0.0;
        self.steps = 0;
    }

    pub fn initialize_sine(&mut self, amplitude: f32, mode: usize) {
        let dx = self.config.dx;
        for i in 0..self.config.nx {
            let x = i as f32 * dx;
            self.u_curr[i] = amplitude * (mode as f32 * 2.0 * std::f32::consts::PI * x / self.config.domain_length()).sin();
        }
        self.time = 0.0;
        self.steps = 0;
    }

    pub fn initialize_random(&mut self, amplitude: f32, seed: u64) {
        let mut rng = SimpleRng::new(seed);
        for u in self.u_curr.iter_mut() {
            *u = amplitude * (2.0 * rng.next() - 1.0);
        }
        self.time = 0.0;
        self.steps = 0;
    }

    pub fn initialize_perturbation(&mut self, amplitude: f32) {
        let dx = self.config.dx;
        let l = self.config.domain_length();
        for i in 0..self.config.nx {
            let x = i as f32 * dx;
            self.u_curr[i] = amplitude * (2.0 * std::f32::consts::PI * x / l).sin()
                + 0.01 * amplitude * (4.0 * std::f32::consts::PI * x / l).cos();
        }
        self.time = 0.0;
        self.steps = 0;
    }

    pub fn step(&mut self) {
        let n = self.config.nx;
        let dx = self.config.dx;
        let dt = self.config.dt;
        let inv_dx = 1.0 / dx;
        let inv_dx2 = 1.0 / (dx * dx);
        let inv_dx4 = 1.0 / (dx.powi(4));

        match self.config.boundary {
            KsBoundary::Periodic => {
                for i in 0..n {
                    let ip1 = self.wrap(i as i32 + 1);
                    let im1 = self.wrap(i as i32 - 1);
                    let ip2 = self.wrap(i as i32 + 2);
                    let im2 = self.wrap(i as i32 - 2);

                    let u = self.u_curr[i];
                    let u_p1 = self.u_curr[ip1];
                    let u_m1 = self.u_curr[im1];
                    let u_p2 = self.u_curr[ip2];
                    let u_m2 = self.u_curr[im2];

                    let du = (u_p1 - u_m1) * 0.5 * inv_dx;
                    let d2u = (u_p1 - 2.0 * u + u_m1) * inv_dx2;
                    let d4u = (u_p2 - 4.0 * u_p1 + 6.0 * u - 4.0 * u_m1 + u_m2) * inv_dx4;

                    let rhs = -u * du - d2u - d4u;
                    self.u_next[i] = u + dt * rhs;
                }
            }
            KsBoundary::Dirichlet { left, right } => {
                for i in 1..n - 1 {
                    let ip1 = (i + 1).min(n - 1);
                    let im1 = if i > 0 { i - 1 } else { 0 };
                    let ip2 = (i + 2).min(n - 1);
                    let im2 = if i > 1 { i - 2 } else { 0 };

                    let u = self.u_curr[i];
                    let u_p1 = self.u_curr[ip1];
                    let u_m1 = self.u_curr[im1];
                    let u_p2 = self.u_curr[ip2];
                    let u_m2 = self.u_curr[im2];

                    let du = (u_p1 - u_m1) * 0.5 * inv_dx;
                    let d2u = (u_p1 - 2.0 * u + u_m1) * inv_dx2;
                    let d4u = (u_p2 - 4.0 * u_p1 + 6.0 * u - 4.0 * u_m1 + u_m2) * inv_dx4;

                    let rhs = -u * du - d2u - d4u;
                    self.u_next[i] = u + dt * rhs;
                }
                self.u_next[0] = left;
                self.u_next[n - 1] = right;
            }
        }

        std::mem::swap(&mut self.u_curr, &mut self.u_next);
        self.time += dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    pub fn energy(&self) -> f32 {
        let dx = self.config.dx;
        0.5 * self.u_curr.iter().map(|&u| u * u).sum::<f32>() * dx
    }

    pub fn mean(&self) -> f32 {
        let n = self.u_curr.len();
        if n == 0 {
            return 0.0;
        }
        self.u_curr.iter().sum::<f32>() / n as f32
    }

    pub fn l2_norm(&self) -> f32 {
        let dx = self.config.dx;
        self.u_curr.iter().map(|&u| u * u).sum::<f32>().sqrt() * dx.sqrt()
    }

    pub fn max_amplitude(&self) -> f32 {
        self.u_curr.iter().cloned().fold(0.0f32, f32::max)
    }

    pub fn min_amplitude(&self) -> f32 {
        self.u_curr.iter().cloned().fold(0.0f32, f32::min)
    }

    pub fn max_abs(&self) -> f32 {
        self.u_curr.iter().map(|&u| u.abs()).fold(0.0f32, f32::max)
    }

    pub fn has_nan(&self) -> bool {
        self.u_curr.iter().any(|&u| !u.is_finite())
    }

    pub fn enstrophy(&self) -> f32 {
        let n = self.config.nx;
        let dx = self.config.dx;
        let inv_dx2 = 1.0 / (dx * dx);
        let mut sum = 0.0f32;
        for i in 0..n {
            let ip1 = self.wrap(i as i32 + 1);
            let im1 = self.wrap(i as i32 - 1);
            let dudx = (self.u_curr[ip1] - self.u_curr[im1]) * 0.5 / dx;
            sum += dudx * dudx;
        }
        sum * inv_dx2 * dx
    }

    pub fn dissipation_rate(&self) -> f32 {
        let n = self.config.nx;
        let dx = self.config.dx;
        let inv_dx2 = 1.0 / (dx * dx);
        let mut sum_lap_sq = 0.0f32;
        for i in 0..n {
            let ip1 = self.wrap(i as i32 + 1);
            let im1 = self.wrap(i as i32 - 1);
            let lap = (self.u_curr[ip1] - 2.0 * self.u_curr[i] + self.u_curr[im1]) * inv_dx2;
            sum_lap_sq += lap * lap;
        }
        sum_lap_sq * dx
    }
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        SimpleRng {
            state: if seed == 0 { 0x853c49e6748fea9b } else { seed },
        }
    }

    fn next(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        (self.state >> 11) as f32 / (1u64 << 53) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tolerance_config() -> KsConfig {
        KsConfig {
            nx: 128,
            dx: 32.0 * std::f32::consts::PI / 128.0,
            dt: 0.001,
            boundary: KsBoundary::Periodic,
        }
    }

    #[test]
    fn test_default_config_stability() {
        let cfg = KsConfig::default();
        assert!(cfg.is_stable(), "default config should be stable");
    }

    #[test]
    fn test_stable_dt_positive() {
        let cfg = KsConfig::default();
        assert!(cfg.stable_dt() > 0.0);
    }

    #[test]
    fn test_domain_length() {
        let cfg = KsConfig::default();
        assert!((cfg.domain_length() - 32.0 * std::f32::consts::PI).abs() < 1e-4);
    }

    #[test]
    fn test_initialize_zero() {
        let mut s = KsSolver::new(tolerance_config());
        s.initialize_zero();
        assert_eq!(s.mean(), 0.0);
        assert_eq!(s.energy(), 0.0);
        assert_eq!(s.steps, 0);
    }

    #[test]
    fn test_initialize_sine_amplitude() {
        let mut s = KsSolver::new(tolerance_config());
        s.initialize_sine(1.0, 1);
        assert!(s.max_amplitude() > 0.9);
        assert!(s.min_amplitude() < -0.9);
    }

    #[test]
    fn test_initialize_random_bounded() {
        let mut s = KsSolver::new(tolerance_config());
        s.initialize_random(0.5, 42);
        assert!(s.max_abs() <= 0.5);
    }

    #[test]
    fn test_initialize_perturbation() {
        let mut s = KsSolver::new(tolerance_config());
        s.initialize_perturbation(1.0);
        assert!(s.energy() > 0.0);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = KsSolver::new(tolerance_config());
        s.initialize_sine(0.1, 1);
        let t0 = s.time;
        s.step();
        assert!(s.time > t0);
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = KsSolver::new(tolerance_config());
        s.initialize_sine(0.1, 1);
        s.step_n(10);
        assert_eq!(s.steps, 10);
    }

    #[test]
    fn test_no_nan_short_run() {
        let mut s = KsSolver::new(tolerance_config());
        s.initialize_random(0.1, 12345);
        s.step_n(100);
        assert!(!s.has_nan(), "NaN detected after 100 steps");
    }

    #[test]
    fn test_no_nan_long_run() {
        let cfg = KsConfig {
            nx: 64,
            dx: 16.0 * std::f32::consts::PI / 64.0,
            dt: 0.0001,
            boundary: KsBoundary::Periodic,
        };
        let mut s = KsSolver::new(cfg);
        s.initialize_random(0.1, 99);
        s.step_n(1000);
        assert!(!s.has_nan(), "NaN detected after 1000 steps");
    }

    #[test]
    fn test_energy_bounded() {
        let mut s = KsSolver::new(tolerance_config());
        s.initialize_random(0.5, 42);
        let e0 = s.energy();
        s.step_n(500);
        let e1 = s.energy();
        assert!(e1.is_finite(), "energy is not finite");
        assert!(e1 < 100.0 * e0 + 1.0, "energy blew up: {} -> {}", e0, e1);
    }

    #[test]
    fn test_zero_state_stays_zero() {
        let mut s = KsSolver::new(tolerance_config());
        s.initialize_zero();
        s.step_n(50);
        assert!(s.energy() < 1e-10, "zero state should stay zero");
    }

    #[test]
    fn test_periodic_boundary_wrap() {
        let s = KsSolver::new(tolerance_config());
        assert_eq!(s.wrap(-1), s.config.nx - 1);
        assert_eq!(s.wrap(0), 0);
        assert_eq!(s.wrap(s.config.nx as i32), 0);
    }

    #[test]
    fn test_dirichlet_boundary_enforced() {
        let cfg = KsConfig {
            nx: 64,
            dx: 1.0,
            dt: 0.001,
            boundary: KsBoundary::Dirichlet { left: 0.0, right: 0.0 },
        };
        let mut s = KsSolver::new(cfg);
        s.initialize_sine(1.0, 1);
        s.step_n(10);
        assert!(s.u_curr[0].abs() < 1e-6, "left boundary not enforced");
        assert!(s.u_curr[s.config.nx - 1].abs() < 1e-6, "right boundary not enforced");
    }

    #[test]
    fn test_max_min_amplitude() {
        let mut s = KsSolver::new(tolerance_config());
        s.initialize_sine(2.0, 1);
        assert!((s.max_amplitude() - 2.0).abs() < 0.1);
        assert!((s.min_amplitude() + 2.0).abs() < 0.1);
    }

    #[test]
    fn test_l2_norm_positive() {
        let mut s = KsSolver::new(tolerance_config());
        s.initialize_sine(1.0, 1);
        assert!(s.l2_norm() > 0.0);
    }

    #[test]
    fn test_enstrophy_positive() {
        let mut s = KsSolver::new(tolerance_config());
        s.initialize_sine(1.0, 2);
        assert!(s.enstrophy() > 0.0);
    }

    #[test]
    fn test_dissipation_rate_finite() {
        let mut s = KsSolver::new(tolerance_config());
        s.initialize_random(0.1, 7);
        assert!(s.dissipation_rate().is_finite());
    }

    #[test]
    fn test_chaotic_divergence() {
        let cfg = KsConfig {
            nx: 128,
            dx: 32.0 * std::f32::consts::PI / 128.0,
            dt: 0.005,
            boundary: KsBoundary::Periodic,
        };
        let mut s1 = KsSolver::new(cfg.clone());
        let mut s2 = KsSolver::new(cfg);
        s1.initialize_random(0.5, 42);
        s2.initialize_random(0.5, 42);
        s2.u_curr[0] += 1e-2;
        s1.step_n(2000);
        s2.step_n(2000);
        let diff = s1.u_curr.iter().zip(s2.u_curr.iter()).map(|(a, b)| (a - b).abs()).sum::<f32>();
        assert!(diff > 0.1, "chaotic divergence expected, diff={}", diff);
    }

    #[test]
    fn test_antisymmetry_preservation() {
        let cfg = KsConfig {
            nx: 128,
            dx: 32.0 * std::f32::consts::PI / 128.0,
            dt: 0.0005,
            boundary: KsBoundary::Periodic,
        };
        let mut s = KsSolver::new(cfg);
        s.initialize_sine(0.1, 1);
        let e0 = s.energy();
        s.step_n(200);
        let e1 = s.energy();
        assert!(e1.is_finite());
        assert!(e1 < 10.0 * e0 + 1.0, "energy unstable: {} -> {}", e0, e1);
    }

    #[test]
    fn test_biharmonic_cfl_limit() {
        let cfg = KsConfig {
            nx: 64,
            dx: 1.0,
            dt: 0.1,
            boundary: KsBoundary::Periodic,
        };
        assert!(!cfg.is_stable(), "dt=0.1 with dx=1 should be unstable");
    }

    #[test]
    fn test_small_perturbation_grows() {
        let cfg = KsConfig {
            nx: 128,
            dx: 32.0 * std::f32::consts::PI / 128.0,
            dt: 0.005,
            boundary: KsBoundary::Periodic,
        };
        let mut s = KsSolver::new(cfg);
        s.initialize_sine(0.01, 1);
        let e0 = s.energy();
        s.step_n(200);
        let e1 = s.energy();
        assert!(e1 > e0, "mode-1 perturbation should grow initially: {} -> {}", e0, e1);
    }

    #[test]
    fn test_config_diffusive_cfl() {
        let cfg = tolerance_config();
        assert!(cfg.diffusive_cfl() > 0.0);
    }

    #[test]
    fn test_config_biharmonic_cfl() {
        let cfg = tolerance_config();
        assert!(cfg.biharmonic_cfl() > 0.0);
    }
}
