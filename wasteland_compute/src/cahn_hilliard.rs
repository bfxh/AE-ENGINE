//! Cahn-Hilliard Phase Separation Solver (4th-order nonlinear diffusion)
//!
//! 描述二元混合物的相分离过程 (spinodal decomposition) 与粗化 (coarsening).
//! Cahn & Hilliard 1958 提出, 是相场模型 (phase-field) 的基础方程.
//!
//! 方程:
//!   dc/dt = M * laplacian(mu)
//!   mu = c^3 - c - kappa * laplacian(c)
//!
//! 其中:
//!   c = 浓度场 (c in [-1, 1], c=+1 纯 A 相, c=-1 纯 B 相)
//!   mu = 化学势
//!   M = 迁移率 (mobility)
//!   kappa = 梯度能系数 (gradient energy coefficient)
//!
//! 自由能 (Ginzburg-Landau):
//!   F = integral [ 0.5*kappa*|grad c|^2 + 0.25*(c^2-1)^2 ] dx
//!   体相: 0.25*(c^2-1)^2 -- 双势阱, c=+/-1 为极小
//!   梯度: 0.5*kappa*|grad c|^2 -- 界面能, 阻止锐变
//!
//! 物理:
//!   - Spinodal decomposition: c near 0 不稳定 (自由能凹), 小涨落放大 -> 相分离
//!   - 最快增长波数 k_max = 1/sqrt(2*kappa) (线性稳定性分析)
//!   - 界面宽度 xi = sqrt(2*kappa) (平衡 tanh 剖面)
//!   - 粗化 (coarsening): 大畴吞并小畴, L(t) ~ t^{1/3} (Lifshitz-Slyozov)
//!
//! 数值方法: 显式 Euler + 5点 Laplacian (2D), 两次应用得到 biharmonic
//!   1. lap_c = laplacian(c)
//!   2. mu = c^3 - c - kappa * lap_c
//!   3. lap_mu = laplacian(mu)
//!   4. c^{n+1} = c + dt * M * lap_mu
//!
//! CFL 稳定性 (显式, 2D, 5点模板两次):
//!   Nyquist: lambda_L = -8/dx^2, lambda_B = 64/dx^4
//!   eigenvalue gamma = -M*lambda_L - M*kappa*lambda_B^2/... (worst case)
//!   -> dt <= dx^4 / (32 * M * kappa)
//!
//! 守恒律:
//!   总质量 integral c dx 守恒 (周期边界下精确)
//!   自由能 F 单调递减 (H^{-1} 梯度流)
//!
//! 基于 Cahn & Hilliard 1958, Cahn 1961 (spinodal),
//! Lifshitz & Slyozov 1961 (coarsening), Yue et al. 2004 (numerical methods).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ChBoundary {
    Periodic,
    Neumann,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CahnHilliardConfig {
    pub nx: usize,
    pub ny: usize,
    pub dx: f32,
    pub dt: f32,
    pub kappa: f32,
    pub mobility: f32,
    pub boundary: ChBoundary,
}

impl Default for CahnHilliardConfig {
    fn default() -> Self {
        // biharmonic_cfl = 32 * 1 * 0.5 * 0.05 / 1 = 0.8 (stable)
        CahnHilliardConfig {
            nx: 128,
            ny: 128,
            dx: 1.0,
            dt: 0.05,
            kappa: 0.5,
            mobility: 1.0,
            boundary: ChBoundary::Periodic,
        }
    }
}

fn wrap_idx(i: i32, n: usize) -> usize {
    let m = n as i32;
    (((i % m) + m) % m) as usize
}

fn laplacian_2d(field: &[f32], out: &mut [f32], nx: usize, ny: usize, dx: f32, bc: ChBoundary) {
    let inv_dx2 = 1.0 / (dx * dx);
    for j in 0..ny {
        for i in 0..nx {
            let k = j * nx + i;
            let (ip, im, jp, jm) = match bc {
                ChBoundary::Periodic => (
                    wrap_idx(i as i32 + 1, nx),
                    wrap_idx(i as i32 - 1, nx),
                    wrap_idx(j as i32 + 1, ny),
                    wrap_idx(j as i32 - 1, ny),
                ),
                ChBoundary::Neumann => (
                    (i + 1).min(nx - 1),
                    if i > 0 { i - 1 } else { 0 },
                    (j + 1).min(ny - 1),
                    if j > 0 { j - 1 } else { 0 },
                ),
            };
            let f = field[k];
            let f_ip = field[j * nx + ip];
            let f_im = field[j * nx + im];
            let f_jp = field[jp * nx + i];
            let f_jm = field[jm * nx + i];
            out[k] = (f_ip + f_im + f_jp + f_jm - 4.0 * f) * inv_dx2;
        }
    }
}

impl CahnHilliardConfig {
    pub fn n_cells(&self) -> usize {
        self.nx * self.ny
    }
    pub fn domain_area(&self) -> f32 {
        (self.nx as f32) * self.dx * (self.ny as f32) * self.dx
    }
    /// 4阶 biharmonic CFL 数: 32*M*kappa*dt / dx^4
    pub fn biharmonic_cfl(&self) -> f32 {
        32.0 * self.mobility * self.kappa * self.dt / self.dx.powi(4)
    }
    pub fn is_stable(&self) -> bool {
        self.biharmonic_cfl() <= 1.0
    }
    pub fn stable_dt(&self) -> f32 {
        self.dx.powi(4) / (32.0 * self.mobility * self.kappa)
    }
    /// 最快增长波数 (spinodal decomposition, 线性理论)
    pub fn k_max(&self) -> f32 {
        1.0 / (2.0 * self.kappa).sqrt()
    }
    /// 最快增长波长
    pub fn lambda_max(&self) -> f32 {
        2.0 * std::f32::consts::PI * (2.0 * self.kappa).sqrt()
    }
    /// 平衡界面宽度
    pub fn interface_width(&self) -> f32 {
        (2.0 * self.kappa).sqrt()
    }
}

pub struct CahnHilliardSolver {
    pub config: CahnHilliardConfig,
    pub c_curr: Vec<f32>,
    pub c_next: Vec<f32>,
    pub mu: Vec<f32>,
    pub lap_c: Vec<f32>,
    pub time: f32,
    pub steps: usize,
}

impl CahnHilliardSolver {
    pub fn new(config: CahnHilliardConfig) -> Self {
        let n = config.n_cells();
        CahnHilliardSolver {
            config,
            c_curr: vec![0.0; n],
            c_next: vec![0.0; n],
            mu: vec![0.0; n],
            lap_c: vec![0.0; n],
            time: 0.0,
            steps: 0,
        }
    }

    pub fn idx(&self, i: usize, j: usize) -> usize {
        j * self.config.nx + i
    }

    /// 显式 Euler 一步: lap_c -> mu -> lap_mu -> c_next
    pub fn step(&mut self) {
        let n = self.c_curr.len();
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        let bc = self.config.boundary;
        let kappa = self.config.kappa;
        let m_mob = self.config.mobility;
        let dt = self.config.dt;

        // 1. lap_c = laplacian(c)
        laplacian_2d(&self.c_curr, &mut self.lap_c, nx, ny, dx, bc);
        // 2. mu = c^3 - c - kappa * lap_c
        for k in 0..n {
            let c = self.c_curr[k];
            self.mu[k] = c * c * c - c - kappa * self.lap_c[k];
        }
        // 3. lap_mu = laplacian(mu) (into c_next temporarily)
        laplacian_2d(&self.mu, &mut self.c_next, nx, ny, dx, bc);
        // 4. c^{n+1} = c + dt * M * lap_mu
        for k in 0..n {
            self.c_next[k] = self.c_curr[k] + dt * m_mob * self.c_next[k];
        }
        std::mem::swap(&mut self.c_curr, &mut self.c_next);
        self.time += dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    /// 随机噪声初始化 (spinodal decomposition 起点)
    /// c = mean + amplitude * uniform(-1, 1), clamped to [-1, 1]
    pub fn initialize_random(&mut self, mean: f32, amplitude: f32, seed: u64) {
        let mut state = seed;
        for c in self.c_curr.iter_mut() {
            // xorshift64
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let r = ((state as f64) / (u64::MAX as f64)) * 2.0 - 1.0;
            *c = (mean + amplitude * r as f32).clamp(-1.0, 1.0);
        }
    }

    /// 平滑圆形液滴: c = tanh((r - R) / xi)
    /// 内部 (r < R): c -> -1; 外部 (r > R): c -> +1
    pub fn initialize_bubble(&mut self, cx: f32, cy: f32, radius: f32) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        let xi = self.config.interface_width();
        for j in 0..ny {
            for i in 0..nx {
                let x = (i as f32) * dx;
                let y = (j as f32) * dx;
                let r = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
                let k = self.idx(i, j);
                self.c_curr[k] = ((r - radius) / xi).tanh();
            }
        }
    }

    /// 平面条带 (1D 界面测试): c = tanh((x - x_mid) / xi)
    pub fn initialize_strip(&mut self, x_mid: f32) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        let xi = self.config.interface_width();
        for j in 0..ny {
            for i in 0..nx {
                let x = (i as f32) * dx;
                let k = self.idx(i, j);
                self.c_curr[k] = ((x - x_mid) / xi).tanh();
            }
        }
    }

    /// 总质量 integral c dx (守恒量)
    pub fn mass(&self) -> f32 {
        self.c_curr.iter().sum::<f32>() * self.config.dx * self.config.dx
    }

    /// 平均浓度
    pub fn mean_concentration(&self) -> f32 {
        self.c_curr.iter().sum::<f32>() / self.c_curr.len() as f32
    }

    /// Ginzburg-Landau 自由能
    /// F = integral [ 0.5*kappa*|grad c|^2 + 0.25*(c^2-1)^2 ] dx
    pub fn free_energy(&self) -> f32 {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        let kappa = self.config.kappa;
        let bc = self.config.boundary;
        let inv_2dx = 1.0 / (2.0 * dx);
        let mut energy = 0.0f32;
        for j in 0..ny {
            for i in 0..nx {
                let k = j * nx + i;
                let c = self.c_curr[k];
                let (ip, im, jp, jm) = match bc {
                    ChBoundary::Periodic => (
                        wrap_idx(i as i32 + 1, nx),
                        wrap_idx(i as i32 - 1, nx),
                        wrap_idx(j as i32 + 1, ny),
                        wrap_idx(j as i32 - 1, ny),
                    ),
                    ChBoundary::Neumann => (
                        (i + 1).min(nx - 1),
                        if i > 0 { i - 1 } else { 0 },
                        (j + 1).min(ny - 1),
                        if j > 0 { j - 1 } else { 0 },
                    ),
                };
                let dc_dx = (self.c_curr[j * nx + ip] - self.c_curr[j * nx + im]) * inv_2dx;
                let dc_dy = (self.c_curr[jp * nx + i] - self.c_curr[jm * nx + i]) * inv_2dx;
                let grad_sq = dc_dx * dc_dx + dc_dy * dc_dy;
                let c2 = c * c;
                let bulk = 0.25 * (c2 - 1.0) * (c2 - 1.0);
                energy += 0.5 * kappa * grad_sq + bulk;
            }
        }
        energy * dx * dx
    }

    pub fn max_concentration(&self) -> f32 {
        self.c_curr.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
    }

    pub fn min_concentration(&self) -> f32 {
        self.c_curr.iter().cloned().fold(f32::INFINITY, f32::min)
    }

    pub fn has_nan(&self) -> bool {
        self.c_curr.iter().any(|&c| c.is_nan() || c.is_infinite())
    }

    /// 界面单元格数 (|grad c| > threshold 的格子数)
    pub fn interface_count(&self, threshold: f32) -> usize {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        let bc = self.config.boundary;
        let inv_2dx = 1.0 / (2.0 * dx);
        let mut count = 0usize;
        for j in 0..ny {
            for i in 0..nx {
                let k = j * nx + i;
                let (ip, im, jp, jm) = match bc {
                    ChBoundary::Periodic => (
                        wrap_idx(i as i32 + 1, nx),
                        wrap_idx(i as i32 - 1, nx),
                        wrap_idx(j as i32 + 1, ny),
                        wrap_idx(j as i32 - 1, ny),
                    ),
                    ChBoundary::Neumann => (
                        (i + 1).min(nx - 1),
                        if i > 0 { i - 1 } else { 0 },
                        (j + 1).min(ny - 1),
                        if j > 0 { j - 1 } else { 0 },
                    ),
                };
                let dc_dx = (self.c_curr[j * nx + ip] - self.c_curr[j * nx + im]) * inv_2dx;
                let dc_dy = (self.c_curr[jp * nx + i] - self.c_curr[jm * nx + i]) * inv_2dx;
                let grad = (dc_dx * dc_dx + dc_dy * dc_dy).sqrt();
                if grad > threshold {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn reset(&mut self) {
        for c in self.c_curr.iter_mut() { *c = 0.0; }
        for c in self.c_next.iter_mut() { *c = 0.0; }
        for m in self.mu.iter_mut() { *m = 0.0; }
        for l in self.lap_c.iter_mut() { *l = 0.0; }
        self.time = 0.0;
        self.steps = 0;
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    fn variance(data: &[f32]) -> f32 {
        let mean = data.iter().sum::<f32>() / data.len() as f32;
        data.iter().map(|&x| (x - mean) * (x - mean)).sum::<f32>() / data.len() as f32
    }

    #[test]
    fn test_config_default() {
        let c = CahnHilliardConfig::default();
        assert_eq!(c.nx, 128);
        assert_eq!(c.ny, 128);
        assert_eq!(c.dx, 1.0);
        assert_eq!(c.dt, 0.05);
        assert_eq!(c.kappa, 0.5);
        assert_eq!(c.mobility, 1.0);
        assert_eq!(c.boundary, ChBoundary::Periodic);
    }

    #[test]
    fn test_n_cells() {
        let c = CahnHilliardConfig { nx: 64, ny: 32, ..Default::default() };
        assert_eq!(c.n_cells(), 64 * 32);
    }

    #[test]
    fn test_domain_area() {
        let c = CahnHilliardConfig { nx: 10, ny: 20, dx: 0.5, ..Default::default() };
        assert!(approx_eq(c.domain_area(), 10.0 * 0.5 * 20.0 * 0.5, 1e-6));
    }

    #[test]
    fn test_biharmonic_cfl() {
        let c = CahnHilliardConfig { nx: 64, dx: 1.0, dt: 0.05, kappa: 0.5, mobility: 1.0, ..Default::default() };
        // 32 * 1 * 0.5 * 0.05 / 1 = 0.8
        assert!(approx_eq(c.biharmonic_cfl(), 0.8, 1e-6));
    }

    #[test]
    fn test_is_stable_default() {
        let c = CahnHilliardConfig::default();
        assert!(c.is_stable());
    }

    #[test]
    fn test_is_stable_unstable() {
        let c = CahnHilliardConfig { dt: 0.2, ..Default::default() };
        // 32 * 1 * 0.5 * 0.2 = 3.2 > 1
        assert!(!c.is_stable());
    }

    #[test]
    fn test_stable_dt() {
        let c = CahnHilliardConfig { dx: 1.0, kappa: 0.5, mobility: 1.0, ..Default::default() };
        // dx^4 / (32 * M * kappa) = 1 / 16 = 0.0625
        assert!(approx_eq(c.stable_dt(), 0.0625, 1e-6));
    }

    #[test]
    fn test_k_max() {
        let c = CahnHilliardConfig { kappa: 0.5, ..Default::default() };
        // 1/sqrt(2*0.5) = 1
        assert!(approx_eq(c.k_max(), 1.0, 1e-6));
    }

    #[test]
    fn test_lambda_max() {
        let c = CahnHilliardConfig { kappa: 0.5, ..Default::default() };
        // 2*pi*sqrt(1) = 2*pi
        assert!(approx_eq(c.lambda_max(), 2.0 * std::f32::consts::PI, 1e-5));
    }

    #[test]
    fn test_interface_width() {
        let c = CahnHilliardConfig { kappa: 0.5, ..Default::default() };
        // sqrt(2*0.5) = 1
        assert!(approx_eq(c.interface_width(), 1.0, 1e-6));
    }

    #[test]
    fn test_solver_new() {
        let s = CahnHilliardSolver::new(CahnHilliardConfig { nx: 32, ny: 16, ..Default::default() });
        assert_eq!(s.c_curr.len(), 32 * 16);
        assert_eq!(s.c_next.len(), 32 * 16);
        assert_eq!(s.mu.len(), 32 * 16);
        assert_eq!(s.lap_c.len(), 32 * 16);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
        for &c in s.c_curr.iter() { assert_eq!(c, 0.0); }
    }

    #[test]
    fn test_idx() {
        let s = CahnHilliardSolver::new(CahnHilliardConfig { nx: 10, ny: 5, ..Default::default() });
        assert_eq!(s.idx(3, 2), 2 * 10 + 3);
        assert_eq!(s.idx(0, 0), 0);
        assert_eq!(s.idx(9, 4), 4 * 10 + 9);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = CahnHilliardSolver::new(CahnHilliardConfig { nx: 16, ny: 16, dt: 0.01, ..Default::default() });
        s.step();
        assert!(approx_eq(s.time, 0.01, 1e-9));
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = CahnHilliardSolver::new(CahnHilliardConfig { nx: 16, ny: 16, dt: 0.01, ..Default::default() });
        s.step_n(50);
        assert_eq!(s.steps, 50);
        assert!(approx_eq(s.time, 0.5, 1e-6));
    }

    #[test]
    fn test_pure_phase_stable() {
        // c=+1 (pure phase) is a stable equilibrium: mu=0, lap_mu=0, no change
        let mut s = CahnHilliardSolver::new(CahnHilliardConfig { nx: 16, ny: 16, ..Default::default() });
        for c in s.c_curr.iter_mut() { *c = 1.0; }
        s.step_n(100);
        let max_dev = s.c_curr.iter().map(|&c| (c - 1.0).abs()).fold(0.0f32, f32::max);
        assert!(max_dev < 1e-4, "pure phase drifted: {}", max_dev);
    }

    #[test]
    fn test_mass_conservation_periodic() {
        let mut s = CahnHilliardSolver::new(CahnHilliardConfig {
            nx: 32, ny: 32, dx: 1.0, dt: 0.05, kappa: 0.5, mobility: 1.0,
            boundary: ChBoundary::Periodic,
        });
        s.initialize_random(0.0, 0.1, 42);
        let m0 = s.mass();
        s.step_n(200);
        let m1 = s.mass();
        let drift = (m1 - m0).abs() / m0.abs().max(1.0);
        assert!(drift < 1e-4, "mass drift: {} -> {} ({:.6}%)", m0, m1, drift * 100.0);
    }

    #[test]
    fn test_mass_conservation_neumann() {
        let mut s = CahnHilliardSolver::new(CahnHilliardConfig {
            nx: 32, ny: 32, dx: 1.0, dt: 0.05, kappa: 0.5, mobility: 1.0,
            boundary: ChBoundary::Neumann,
        });
        s.initialize_random(0.0, 0.1, 42);
        let m0 = s.mass();
        s.step_n(200);
        let m1 = s.mass();
        let drift = (m1 - m0).abs() / m0.abs().max(1.0);
        assert!(drift < 1e-4, "mass drift: {} -> {} ({:.6}%)", m0, m1, drift * 100.0);
    }

    #[test]
    fn test_free_energy_decreases() {
        let mut s = CahnHilliardSolver::new(CahnHilliardConfig {
            nx: 32, ny: 32, dx: 1.0, dt: 0.02, kappa: 0.5, mobility: 1.0,
            boundary: ChBoundary::Periodic,
        });
        s.initialize_random(0.0, 0.1, 42);
        let f0 = s.free_energy();
        s.step_n(200);
        let f1 = s.free_energy();
        assert!(f1 < f0, "free energy did not decrease: {} -> {}", f0, f1);
    }

    #[test]
    fn test_spinodal_decomposition() {
        let mut s = CahnHilliardSolver::new(CahnHilliardConfig {
            nx: 64, ny: 64, dx: 1.0, dt: 0.05, kappa: 0.5, mobility: 1.0,
            boundary: ChBoundary::Periodic,
        });
        s.initialize_random(0.0, 0.1, 42);
        let var0 = variance(&s.c_curr);
        s.step_n(500);
        let var1 = variance(&s.c_curr);
        assert!(var1 > var0, "spinodal did not amplify: var {} -> {}", var0, var1);
        let n_separated = s.c_curr.iter().filter(|&&c| c.abs() > 0.5).count();
        assert!(n_separated > 50, "not enough separated cells: {}", n_separated);
    }

    #[test]
    fn test_bubble_stability() {
        let mut s = CahnHilliardSolver::new(CahnHilliardConfig {
            nx: 64, ny: 64, dx: 1.0, dt: 0.02, kappa: 0.5, mobility: 1.0,
            boundary: ChBoundary::Periodic,
        });
        s.initialize_bubble(32.0, 32.0, 10.0);
        let f0 = s.free_energy();
        s.step_n(100);
        assert!(!s.has_nan(), "NaN in bubble evolution");
        let f1 = s.free_energy();
        assert!(f1 <= f0 + 1e-3, "bubble energy increased: {} -> {}", f0, f1);
    }

    #[test]
    fn test_strip_stability() {
        let mut s = CahnHilliardSolver::new(CahnHilliardConfig {
            nx: 64, ny: 8, dx: 1.0, dt: 0.05, kappa: 0.5, mobility: 1.0,
            boundary: ChBoundary::Periodic,
        });
        s.initialize_strip(32.0);
        let m0 = s.mass();
        s.step_n(100);
        assert!(!s.has_nan(), "NaN in strip evolution");
        let m1 = s.mass();
        let drift = (m1 - m0).abs() / m0.abs().max(1.0);
        assert!(drift < 1e-4, "strip mass drift: {}", drift);
    }

    #[test]
    fn test_no_nan_long_run() {
        let mut s = CahnHilliardSolver::new(CahnHilliardConfig {
            nx: 64, ny: 64, dx: 1.0, dt: 0.02, kappa: 0.5, mobility: 1.0,
            boundary: ChBoundary::Periodic,
        });
        s.initialize_random(0.0, 0.1, 42);
        s.step_n(1000);
        assert!(!s.has_nan(), "NaN detected after long run");
    }

    #[test]
    fn test_concentration_bounded() {
        let mut s = CahnHilliardSolver::new(CahnHilliardConfig::default());
        s.initialize_random(0.0, 0.1, 42);
        s.step_n(1000);
        let c_max = s.max_concentration();
        let c_min = s.min_concentration();
        // c should stay bounded (thermodynamically [-1, 1], numerically may slightly exceed)
        assert!(c_max < 1.2, "c runaway positive: {}", c_max);
        assert!(c_min > -1.2, "c runaway negative: {}", c_min);
    }

    #[test]
    fn test_mean_concentration_preserved() {
        let mut s = CahnHilliardSolver::new(CahnHilliardConfig {
            nx: 32, ny: 32, ..Default::default()
        });
        s.initialize_random(0.3, 0.05, 42);
        let mean0 = s.mean_concentration();
        s.step_n(200);
        let mean1 = s.mean_concentration();
        assert!(approx_eq(mean0, mean1, 1e-4), "mean not preserved: {} -> {}", mean0, mean1);
    }

    #[test]
    fn test_interface_count() {
        let mut s = CahnHilliardSolver::new(CahnHilliardConfig {
            nx: 64, ny: 64, dx: 1.0, kappa: 0.5, ..Default::default()
        });
        s.initialize_bubble(32.0, 32.0, 10.0);
        let i0 = s.interface_count(0.1);
        assert!(i0 > 0, "no interface detected");
        s.step_n(50);
        let i1 = s.interface_count(0.1);
        assert!(i1 > 0, "interface disappeared");
    }

    #[test]
    fn test_reset() {
        let mut s = CahnHilliardSolver::new(CahnHilliardConfig { nx: 16, ny: 16, ..Default::default() });
        s.initialize_random(0.0, 0.1, 42);
        s.step_n(50);
        s.reset();
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
        for &c in s.c_curr.iter() { assert_eq!(c, 0.0); }
    }

    #[test]
    fn test_coarsening_energy_decreases() {
        // Long run: energy should keep decreasing (coarsening regime)
        let mut s = CahnHilliardSolver::new(CahnHilliardConfig {
            nx: 48, ny: 48, dx: 1.0, dt: 0.05, kappa: 0.5, mobility: 1.0,
            boundary: ChBoundary::Periodic,
        });
        s.initialize_random(0.0, 0.1, 42);
        s.step_n(200);
        let f_mid = s.free_energy();
        s.step_n(800);
        let f_end = s.free_energy();
        assert!(f_end < f_mid, "coarsening did not decrease energy: {} -> {}", f_mid, f_end);
    }
}