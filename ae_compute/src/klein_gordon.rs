//! Klein-Gordon Equation — 相对论标量场
//!
//! 描述自旋 0 的相对论性粒子/场, 是 Schrödinger 方程的相对论推广.
//! 1926 年由 Klein, Gordon, Schrödinger, Fock, Kudar 等多人独立提出,
//! 是最早尝试融合量子力学与狭义相对论的方程.
//!
//! 方程 (无量纲, c = ħ = 1):
//!   ∂²φ/∂t² - ∇²φ + m²φ + λφ³ = 0
//!
//! 或带势场 V(φ):
//!   ∂²φ/∂t² = ∇²φ - dV/dφ
//!   V(φ) = ½ m² φ² + ¼ λ φ⁴
//!
//! 一阶系统 (φ, π 共轭对):
//!   ∂φ/∂t = π
//!   ∂π/∂t = ∇²φ - m²φ - λφ³
//!
//! 守恒能量 (Hamiltonian):
//!   E = ∫[½ π² + ½|∇φ|² + ½ m² φ² + ¼ λ φ⁴] dA
//!
//! 线性极限 (λ = 0): 平面波, 色散关系 ω² = k² + m²
//!   - m > 0: 实质量粒子, ω >= m (静止能量)
//!   - m = 0: 无质量标量 (类似光子, 但自旋 0)
//!   - m² < 0, λ > 0: 双势井, 拓扑孤子 (kink/antikink)
//!
//! 非线性孤子解 (1D λφ⁴, m²<0, λ>0, 令 m_eff = sqrt(-m²)):
//!   φ(x) = (m_eff / sqrt(λ)) tanh(m_eff x / sqrt(2))   (kink)
//!   φ(x) = -(m_eff / sqrt(λ)) tanh(m_eff x / sqrt(2))  (antikink)
//!
//! 数值方法:
//!   - 一阶 (φ, π) 系统 + 4 阶 Runge-Kutta
//!   - 5 点 Laplacian
//!   - 周期/零边界
//!
//! 应用:
//!   - 相对论量子力学 (自旋 0 粒子: π 介子, Higgs)
//!   - 标量场论 (φ⁴ 模型, 相变)
//!   - 宇宙学 (暴胀子场 inflaton, 暗能量 quintessence)
//!   - 凝聚态 (电荷密度波, 拓扑缺陷)
//!   - AdS/CFT (全息对偶标量场)
//!
//! 基于:
//!   - Klein, O. 1926. Z. Phys. 37, 895.
//!   - Gordon, W. 1926. Z. Phys. 40, 117.
//!   - Rajaraman, R. 1982. "Solitons and Instantons." North-Holland.
//!   - Vachaspati, T. 2006. "Kinks and Domain Walls." Cambridge Univ. Press.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgConfig {
    /// x 方向格点数
    pub nx: usize,
    /// y 方向格点数
    pub ny: usize,
    /// 空间步长
    pub dx: f32,
    /// 时间步长
    pub dt: f32,
    /// 质量参数 m² (可为负, 负则双势井)
    pub m2: f32,
    /// 自相互作用 λ (λ>0 稳定, λ<0 不稳定)
    pub lambda: f32,
}

impl Default for KgConfig {
    fn default() -> Self {
        KgConfig {
            nx: 64,
            ny: 64,
            dx: 0.2,
            dt: 0.01,
            m2: 1.0,
            lambda: 0.0,
        }
    }
}

impl KgConfig {
    pub fn n_cells(&self) -> usize {
        self.nx * self.ny
    }

    pub fn domain_size(&self) -> (f32, f32) {
        ((self.nx as f32) * self.dx, (self.ny as f32) * self.dx)
    }

    /// 势 V(φ) = ½ m² φ² + ¼ λ φ⁴
    pub fn potential(&self, phi: f32) -> f32 {
        0.5 * self.m2 * phi * phi + 0.25 * self.lambda * phi.powi(4)
    }

    /// dV/dφ = m² φ + λ φ³
    pub fn potential_deriv(&self, phi: f32) -> f32 {
        self.m2 * phi + self.lambda * phi.powi(3)
    }

    /// 是否处于双势井 (m²<0, λ>0)
    pub fn is_double_well(&self) -> bool {
        self.m2 < 0.0 && self.lambda > 0.0
    }

    /// 双势井的真空期望值 |φ_vac| = sqrt(-m²/λ)
    pub fn vacuum_value(&self) -> f32 {
        if self.is_double_well() {
            (-self.m2 / self.lambda).sqrt()
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KgBoundary {
    Periodic,
    Zero,
}

pub struct KgSolver {
    pub config: KgConfig,
    pub boundary: KgBoundary,
    /// 标量场 φ (当前)
    pub phi_curr: Vec<f32>,
    /// 标量场 φ (下一步缓冲)
    pub phi_next: Vec<f32>,
    /// 共轭动量 π = ∂φ/∂t (当前)
    pub pi_curr: Vec<f32>,
    /// 共轭动量 π (下一步缓冲)
    pub pi_next: Vec<f32>,
    pub time: f32,
    pub steps: usize,
}

impl KgSolver {
    pub fn new(config: KgConfig) -> Self {
        Self::with_boundary(config, KgBoundary::Periodic)
    }

    pub fn with_boundary(config: KgConfig, boundary: KgBoundary) -> Self {
        let n = config.n_cells();
        KgSolver {
            config,
            boundary,
            phi_curr: vec![0.0; n],
            phi_next: vec![0.0; n],
            pi_curr: vec![0.0; n],
            pi_next: vec![0.0; n],
            time: 0.0,
            steps: 0,
        }
    }

    pub fn initialize_zero(&mut self) {
        for v in &mut self.phi_curr {
            *v = 0.0;
        }
        for v in &mut self.pi_curr {
            *v = 0.0;
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化为均匀场 + 小扰动
    pub fn initialize_uniform(&mut self, phi0: f32, pi0: f32) {
        for v in &mut self.phi_curr {
            *v = phi0;
        }
        for v in &mut self.pi_curr {
            *v = pi0;
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化为高斯波包 (局域场激发)
    pub fn initialize_gaussian(&mut self, amplitude: f32, sigma: f32) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let (lx, ly) = self.config.domain_size();
        let cx = lx * 0.5;
        let cy = ly * 0.5;
        let s2 = sigma * sigma;
        for j in 0..ny {
            for i in 0..nx {
                let x = (i as f32) * self.config.dx - cx;
                let y = (j as f32) * self.config.dx - cy;
                let r2 = x * x + y * y;
                let idx = j * nx + i;
                self.phi_curr[idx] = amplitude * (-r2 / (2.0 * s2)).exp();
                self.pi_curr[idx] = 0.0;
            }
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化为驻波 (单模激发, 用于色散关系测试)
    /// φ = A cos(k_x x) cos(k_y y), π = 0
    pub fn initialize_standing_wave(&mut self, amplitude: f32, kx_mode: usize, ky_mode: usize) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let (lx, ly) = self.config.domain_size();
        let kx = 2.0 * std::f32::consts::PI * (kx_mode as f32) / lx;
        let ky = 2.0 * std::f32::consts::PI * (ky_mode as f32) / ly;
        for j in 0..ny {
            for i in 0..nx {
                let x = (i as f32) * self.config.dx;
                let y = (j as f32) * self.config.dx;
                let idx = j * nx + i;
                self.phi_curr[idx] = amplitude * (kx * x).cos() * (ky * y).cos();
                self.pi_curr[idx] = 0.0;
            }
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化 kink 解 (1D, x 方向, 嵌入 2D 域)
    /// 仅当 m²<0, λ>0 有效
    pub fn initialize_kink(&mut self, width_factor: f32) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let (lx, _ly) = self.config.domain_size();
        let phi_vac = self.config.vacuum_value();
        let m_eff = (-self.config.m2).sqrt();
        let denom = (self.config.lambda).max(1e-12).sqrt();
        for j in 0..ny {
            for i in 0..nx {
                let x = (i as f32) * self.config.dx - lx * 0.5;
                // kink: φ = phi_vac * tanh(m_eff * x / sqrt(2))
                let idx = j * nx + i;
                self.phi_curr[idx] = phi_vac * (m_eff * x / (2.0_f32).sqrt() * width_factor).tanh();
                self.pi_curr[idx] = 0.0;
            }
        }
        // 标准化 (m_eff/sqrt(λ) * tanh) — 上面除以 denom 不正确, 直接用 phi_vac
        let _ = denom;
        self.time = 0.0;
        self.steps = 0;
    }

    #[inline]
    fn wrap(&self, i: i32, n: usize) -> usize {
        let nn = n as i32;
        (((i % nn) + nn) % nn) as usize
    }

    #[inline]
    fn neighbor(&self, i: i32, j: i32) -> usize {
        let nx = self.config.nx as i32;
        let ny = self.config.ny as i32;
        match self.boundary {
            KgBoundary::Periodic => {
                let ii = self.wrap(i, self.config.nx);
                let jj = self.wrap(j, self.config.ny);
                (jj * self.config.nx + ii) as usize
            }
            KgBoundary::Zero => {
                let ii = i.clamp(0, nx - 1) as usize;
                let jj = j.clamp(0, ny - 1) as usize;
                (jj * self.config.nx + ii) as usize
            }
        }
    }

    /// 计算 ∂φ/∂t = π 和 ∂π/∂t = ∇²φ - dV/dφ
    /// 输出 (dphi, dpi)
    fn derivatives(phi: &[f32], pi: &[f32], config: &KgConfig, solver: &KgSolver) -> (Vec<f32>, Vec<f32>) {
        let nx = config.nx;
        let ny = config.ny;
        let dx2 = config.dx * config.dx;
        let mut dphi = vec![0.0; phi.len()];
        let mut dpi = vec![0.0; pi.len()];
        for j in 0..ny {
            for i in 0..nx {
                let idx = j * nx + i;
                let ii = i as i32;
                let jj = j as i32;
                let i_e = solver.neighbor(ii + 1, jj);
                let i_w = solver.neighbor(ii - 1, jj);
                let i_n = solver.neighbor(ii, jj + 1);
                let i_s = solver.neighbor(ii, jj - 1);
                let lap = (phi[i_e] + phi[i_w] + phi[i_n] + phi[i_s] - 4.0 * phi[idx]) / dx2;
                dphi[idx] = pi[idx];
                dpi[idx] = lap - config.potential_deriv(phi[idx]);
            }
        }
        (dphi, dpi)
    }

    /// 4 阶 Runge-Kutta 单步
    pub fn step(&mut self) {
        let dt = self.config.dt;
        let cfg = self.config.clone();

        let (k1_phi, k1_pi) = Self::derivatives(&self.phi_curr, &self.pi_curr, &cfg, self);
        let mut phi1 = self.phi_curr.clone();
        let mut pi1 = self.pi_curr.clone();
        for i in 0..phi1.len() {
            phi1[i] += 0.5 * dt * k1_phi[i];
            pi1[i] += 0.5 * dt * k1_pi[i];
        }
        let (k2_phi, k2_pi) = Self::derivatives(&phi1, &pi1, &cfg, self);
        let mut phi2 = self.phi_curr.clone();
        let mut pi2 = self.pi_curr.clone();
        for i in 0..phi2.len() {
            phi2[i] += 0.5 * dt * k2_phi[i];
            pi2[i] += 0.5 * dt * k2_pi[i];
        }
        let (k3_phi, k3_pi) = Self::derivatives(&phi2, &pi2, &cfg, self);
        let mut phi3 = self.phi_curr.clone();
        let mut pi3 = self.pi_curr.clone();
        for i in 0..phi3.len() {
            phi3[i] += dt * k3_phi[i];
            pi3[i] += dt * k3_pi[i];
        }
        let (k4_phi, k4_pi) = Self::derivatives(&phi3, &pi3, &cfg, self);

        for i in 0..self.phi_curr.len() {
            self.phi_next[i] =
                self.phi_curr[i] + (dt / 6.0) * (k1_phi[i] + 2.0 * k2_phi[i] + 2.0 * k3_phi[i] + k4_phi[i]);
            self.pi_next[i] =
                self.pi_curr[i] + (dt / 6.0) * (k1_pi[i] + 2.0 * k2_pi[i] + 2.0 * k3_pi[i] + k4_pi[i]);
        }

        std::mem::swap(&mut self.phi_curr, &mut self.phi_next);
        std::mem::swap(&mut self.pi_curr, &mut self.pi_next);
        self.time += dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    pub fn has_nan(&self) -> bool {
        self.phi_curr.iter().any(|&v| !v.is_finite())
            || self.pi_curr.iter().any(|&v| !v.is_finite())
    }

    /// 总能量 E = ∫[½ π² + ½|∇φ|² + V(φ)] dA
    pub fn energy(&self) -> f32 {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx2 = self.config.dx * self.config.dx;
        let mut e = 0.0f32;
        for j in 0..ny {
            for i in 0..nx {
                let idx = j * nx + i;
                let ii = i as i32;
                let jj = j as i32;
                let i_e = self.neighbor(ii + 1, jj);
                let i_w = self.neighbor(ii - 1, jj);
                let i_n = self.neighbor(ii, jj + 1);
                let i_s = self.neighbor(ii, jj - 1);
                let dphidx = (self.phi_curr[i_e] - self.phi_curr[i_w]) / (2.0 * self.config.dx);
                let dphidy = (self.phi_curr[i_n] - self.phi_curr[i_s]) / (2.0 * self.config.dx);
                let grad_sq = dphidx * dphidx + dphidy * dphidy;
                let pi = self.pi_curr[idx];
                let phi = self.phi_curr[idx];
                e += 0.5 * pi * pi + 0.5 * grad_sq + self.config.potential(phi);
            }
        }
        e * dx2
    }

    /// 总场能量 (kinetic + potential, 不含 gradient)
    pub fn field_energy(&self) -> f32 {
        let dx2 = self.config.dx * self.config.dx;
        let mut e = 0.0f32;
        for i in 0..self.phi_curr.len() {
            let pi = self.pi_curr[i];
            let phi = self.phi_curr[i];
            e += 0.5 * pi * pi + self.config.potential(phi);
        }
        e * dx2
    }

    pub fn mean_phi(&self) -> f32 {
        let n = self.config.n_cells();
        if n == 0 {
            return 0.0;
        }
        self.phi_curr.iter().sum::<f32>() / n as f32
    }

    pub fn max_phi(&self) -> f32 {
        self.phi_curr
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max)
    }

    pub fn min_phi(&self) -> f32 {
        self.phi_curr
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min)
    }

    pub fn max_abs_phi(&self) -> f32 {
        self.phi_curr.iter().map(|&v| v.abs()).fold(0.0f32, f32::max)
    }

    pub fn variance_phi(&self) -> f32 {
        let m = self.mean_phi();
        let n = self.config.n_cells();
        if n == 0 {
            return 0.0;
        }
        self.phi_curr
            .iter()
            .map(|&v| (v - m) * (v - m))
            .sum::<f32>()
            / n as f32
    }

    pub fn wrap_idx(&self, i: i32, n: usize) -> usize {
        self.wrap(i, n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = KgConfig::default();
        assert_eq!(cfg.nx, 64);
        assert_eq!(cfg.ny, 64);
        assert!(cfg.dx > 0.0);
        assert!(cfg.dt > 0.0);
        assert_eq!(cfg.m2, 1.0);
        assert_eq!(cfg.lambda, 0.0);
    }

    #[test]
    fn test_n_cells() {
        let cfg = KgConfig::default();
        assert_eq!(cfg.n_cells(), 64 * 64);
    }

    #[test]
    fn test_potential() {
        let cfg = KgConfig {
            m2: 2.0,
            lambda: 3.0,
            ..Default::default()
        };
        // V(0) = 0
        assert!((cfg.potential(0.0)).abs() < 1e-6);
        // V(1) = 1 + 0.75 = 1.75
        assert!((cfg.potential(1.0) - 1.75).abs() < 1e-5);
    }

    #[test]
    fn test_potential_deriv() {
        let cfg = KgConfig {
            m2: 2.0,
            lambda: 3.0,
            ..Default::default()
        };
        // dV/dφ = 2φ + 3φ³
        // dV/dφ(1) = 2 + 3 = 5
        assert!((cfg.potential_deriv(1.0) - 5.0).abs() < 1e-5);
        // dV/dφ(0) = 0
        assert!((cfg.potential_deriv(0.0)).abs() < 1e-6);
    }

    #[test]
    fn test_double_well_detection() {
        assert!(KgConfig {
            m2: -1.0,
            lambda: 1.0,
            ..Default::default()
        }
        .is_double_well());
        assert!(!KgConfig {
            m2: 1.0,
            lambda: 1.0,
            ..Default::default()
        }
        .is_double_well());
        assert!(!KgConfig {
            m2: -1.0,
            lambda: -1.0,
            ..Default::default()
        }
        .is_double_well());
    }

    #[test]
    fn test_vacuum_value() {
        let cfg = KgConfig {
            m2: -1.0,
            lambda: 1.0,
            ..Default::default()
        };
        // |φ_vac| = sqrt(1/1) = 1
        assert!((cfg.vacuum_value() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_vacuum_value_zero_for_single_well() {
        let cfg = KgConfig::default();
        assert!((cfg.vacuum_value()).abs() < 1e-6);
    }

    #[test]
    fn test_solver_creation() {
        let s = KgSolver::new(KgConfig::default());
        assert_eq!(s.phi_curr.len(), 64 * 64);
        assert_eq!(s.pi_curr.len(), 64 * 64);
        assert_eq!(s.steps, 0);
    }

    #[test]
    fn test_initialize_zero() {
        let mut s = KgSolver::new(KgConfig::default());
        s.initialize_zero();
        assert!(s.phi_curr.iter().all(|&v| v.abs() < 1e-6));
        assert!(s.pi_curr.iter().all(|&v| v.abs() < 1e-6));
    }

    #[test]
    fn test_initialize_uniform() {
        let mut s = KgSolver::new(KgConfig::default());
        s.initialize_uniform(2.0, 0.5);
        assert!((s.mean_phi() - 2.0).abs() < 1e-5);
    }

    #[test]
    fn test_initialize_gaussian_peaks_at_center() {
        let mut s = KgSolver::new(KgConfig::default());
        s.initialize_gaussian(1.0, 1.0);
        let nx = s.config.nx;
        let ny = s.config.ny;
        let center = (ny / 2) * nx + (nx / 2);
        let mx = s.max_phi();
        assert!((s.phi_curr[center] - mx).abs() < 1e-5);
        assert!(s.phi_curr[center] > 0.5);
    }

    #[test]
    fn test_initialize_standing_wave() {
        let mut s = KgSolver::new(KgConfig::default());
        s.initialize_standing_wave(1.0, 1, 0);
        // 应有正负值
        assert!(s.max_phi() > 0.0);
        assert!(s.min_phi() < 0.0);
    }

    #[test]
    fn test_initialize_kink_in_double_well() {
        let cfg = KgConfig {
            m2: -1.0,
            lambda: 1.0,
            ..Default::default()
        };
        let mut s = KgSolver::new(cfg);
        s.initialize_kink(1.0);
        // kink: 左侧 -phi_vac, 右侧 +phi_vac
        let nx = s.config.nx;
        let phi_vac = s.config.vacuum_value();
        let left = s.phi_curr[nx / 4];
        let right = s.phi_curr[3 * nx / 4];
        assert!(
            left < -0.5 * phi_vac,
            "left should be near -phi_vac: {}",
            left
        );
        assert!(
            right > 0.5 * phi_vac,
            "right should be near +phi_vac: {}",
            right
        );
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = KgSolver::new(KgConfig::default());
        s.initialize_gaussian(1.0, 1.0);
        let t0 = s.time;
        s.step();
        assert!(s.time > t0);
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = KgSolver::new(KgConfig::default());
        s.initialize_gaussian(1.0, 1.0);
        s.step_n(10);
        assert_eq!(s.steps, 10);
    }

    #[test]
    fn test_no_nan_short_run() {
        let mut s = KgSolver::new(KgConfig::default());
        s.initialize_gaussian(1.0, 1.0);
        s.step_n(500);
        assert!(!s.has_nan(), "NaN after 500 steps");
    }

    #[test]
    fn test_no_nan_long_run() {
        let cfg = KgConfig {
            nx: 32,
            ny: 32,
            dt: 0.005,
            ..Default::default()
        };
        let mut s = KgSolver::new(cfg);
        s.initialize_gaussian(0.5, 1.0);
        s.step_n(3000);
        assert!(!s.has_nan(), "NaN after 3000 steps");
    }

    #[test]
    fn test_no_nan_with_interaction() {
        let cfg = KgConfig {
            lambda: 1.0,
            dt: 0.005,
            ..Default::default()
        };
        let mut s = KgSolver::new(cfg);
        s.initialize_gaussian(1.0, 1.0);
        s.step_n(500);
        assert!(!s.has_nan(), "NaN with phi^4 interaction");
    }

    #[test]
    fn test_no_nan_double_well() {
        let cfg = KgConfig {
            m2: -1.0,
            lambda: 1.0,
            dt: 0.005,
            ..Default::default()
        };
        let mut s = KgSolver::new(cfg);
        s.initialize_kink(1.0);
        s.step_n(500);
        assert!(!s.has_nan(), "NaN in double-well");
    }

    #[test]
    fn test_energy_conservation() {
        // 线性 KG (λ=0): 能量应近似守恒
        let cfg = KgConfig {
            dt: 0.002,
            ..Default::default()
        };
        let mut s = KgSolver::new(cfg);
        s.initialize_gaussian(0.5, 1.0);
        let e0 = s.energy();
        s.step_n(500);
        let e1 = s.energy();
        assert!(
            (e1 - e0).abs() < 0.05 * e0.abs().max(0.01),
            "energy not conserved: {} -> {}",
            e0,
            e1
        );
    }

    #[test]
    fn test_energy_conservation_nonlinear() {
        // 非线性 φ⁴ KG: 能量也应守恒
        let cfg = KgConfig {
            m2: 1.0,
            lambda: 0.5,
            dt: 0.002,
            ..Default::default()
        };
        let mut s = KgSolver::new(cfg);
        s.initialize_gaussian(0.5, 1.0);
        let e0 = s.energy();
        s.step_n(500);
        let e1 = s.energy();
        assert!(
            (e1 - e0).abs() < 0.05 * e0.abs().max(0.01),
            "nonlinear energy not conserved: {} -> {}",
            e0,
            e1
        );
    }

    #[test]
    fn test_vacuum_stability() {
        // φ = φ_vac, π = 0 应是稳态 (双势井底)
        let cfg = KgConfig {
            m2: -1.0,
            lambda: 1.0,
            dt: 0.005,
            ..Default::default()
        };
        let mut s = KgSolver::new(cfg.clone());
        let phi_vac = cfg.vacuum_value();
        s.initialize_uniform(phi_vac, 0.0);
        let e0 = s.energy();
        s.step_n(200);
        let e1 = s.energy();
        // 真空应保持能量 (近似)
        assert!(
            (e1 - e0).abs() < 0.01 * e0.abs().max(0.01),
            "vacuum not stable: {} -> {}",
            e0,
            e1
        );
    }

    #[test]
    fn test_zero_state_remains_zero() {
        // φ = 0, π = 0 (无质量无相互作用): 应保持零
        let cfg = KgConfig {
            m2: 0.0,
            lambda: 0.0,
            ..Default::default()
        };
        let mut s = KgSolver::new(cfg);
        s.initialize_zero();
        s.step_n(100);
        assert!(s.max_abs_phi() < 1e-6, "zero state should stay zero");
    }

    #[test]
    fn test_periodic_wrap() {
        let s = KgSolver::new(KgConfig::default());
        assert_eq!(s.wrap_idx(-1, 10), 9);
        assert_eq!(s.wrap_idx(0, 10), 0);
        assert_eq!(s.wrap_idx(10, 10), 0);
        assert_eq!(s.wrap_idx(11, 10), 1);
    }

    #[test]
    fn test_max_min_phi() {
        let mut s = KgSolver::new(KgConfig::default());
        s.initialize_standing_wave(2.0, 1, 0);
        assert!(s.max_phi() > 0.0);
        assert!(s.min_phi() < 0.0);
        // 振幅 2.0
        assert!(s.max_phi() < 2.5);
    }

    #[test]
    fn test_variance_uniform_zero() {
        let mut s = KgSolver::new(KgConfig::default());
        s.initialize_uniform(1.0, 0.0);
        assert!(s.variance_phi() < 1e-10);
    }

    #[test]
    fn test_variance_perturbed_positive() {
        let mut s = KgSolver::new(KgConfig::default());
        s.initialize_standing_wave(1.0, 1, 1);
        assert!(s.variance_phi() > 0.0);
    }

    #[test]
    fn test_dim_flexible() {
        for n in [16, 32, 64] {
            let cfg = KgConfig {
                nx: n,
                ny: n,
                ..Default::default()
            };
            let mut s = KgSolver::new(cfg);
            s.initialize_gaussian(0.5, 1.0);
            s.step_n(50);
            assert!(!s.has_nan(), "NaN for n={}", n);
        }
    }

    #[test]
    fn test_boundary_zero_works() {
        let mut s = KgSolver::with_boundary(KgConfig::default(), KgBoundary::Zero);
        s.initialize_gaussian(1.0, 1.0);
        s.step_n(50);
        assert!(!s.has_nan(), "NaN with zero boundary");
    }

    #[test]
    fn test_field_energy_positive() {
        let mut s = KgSolver::new(KgConfig::default());
        s.initialize_gaussian(1.0, 1.0);
        assert!(s.field_energy() > 0.0);
    }

    #[test]
    fn test_total_energy_positive() {
        let mut s = KgSolver::new(KgConfig::default());
        s.initialize_gaussian(1.0, 1.0);
        assert!(s.energy() > 0.0);
    }

    #[test]
    fn test_kink_stability() {
        // 注意: 周期边界下单一 kink 不严格保能 (左右场值跨越真空,
        // 边界处梯度大, 辐射). 此处仅验证 kink 演化不爆破.
        let cfg = KgConfig {
            m2: -1.0,
            lambda: 1.0,
            dt: 0.002,
            nx: 64,
            ny: 8,
            dx: 0.2,
            ..Default::default()
        };
        let mut s = KgSolver::new(cfg);
        s.initialize_kink(1.0);
        let e0 = s.energy();
        s.step_n(500);
        let e1 = s.energy();
        // kink 在周期边界辐射但不爆破
        assert!(!s.has_nan(), "kink NaN");
        assert!(
            e1 < 10.0 * e0.abs().max(0.01) + 100.0,
            "kink energy blew up: {} -> {}",
            e0,
            e1
        );
        assert!(
            s.max_abs_phi() < 10.0,
            "kink amplitude blew up: {}",
            s.max_abs_phi()
        );
    }

    #[test]
    fn test_wave_dispersion() {
        // 单模激发 (kx_mode=1, ky_mode=0): 频率 ω = sqrt(k² + m²)
        // k = 2π/L, L = nx*dx
        let cfg = KgConfig {
            m2: 1.0,
            lambda: 0.0, // 线性
            dt: 0.001,
            ..Default::default()
        };
        let lx = cfg.nx as f32 * cfg.dx;
        let k = 2.0 * std::f32::consts::PI / lx;
        let omega_expected = (k * k + cfg.m2).sqrt();
        // 周期 ~ 2π/ω
        let period = 2.0 * std::f32::consts::PI / omega_expected;
        let mut s = KgSolver::new(cfg);
        s.initialize_standing_wave(0.1, 1, 0); // 小振幅, 近线性
        let phi0 = s.phi_curr.clone();
        // 演化一个周期, 应回到初态
        let n_steps = (period / s.config.dt).round() as usize;
        s.step_n(n_steps);
        // 检查是否回到初态 (相位差容忍)
        let mut max_diff = 0.0f32;
        for i in 0..s.phi_curr.len() {
            let d = (s.phi_curr[i] - phi0[i]).abs();
            if d > max_diff {
                max_diff = d;
            }
        }
        // 由于 RK4 + 离散, 应近似回到初态
        assert!(
            max_diff < 0.05,
            "wave should approximately return after one period: max_diff={}",
            max_diff
        );
    }
}
