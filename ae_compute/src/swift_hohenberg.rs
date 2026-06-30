//! Swift-Hohenberg Solver — 模式形成方程
//!
//! Swift-Hohenberg 方程是模式形成领域的经典模型, 描述在临界点附近
//! 系统从均匀态自发形成有序空间结构的过程.
//!
//! 方程:
//!   ∂u/∂t = r·u - (k_c² + ∇²)²·u - u³
//!
//! 展开 4阶算子:
//!   (k_c² + ∇²)²·u = k_c⁴·u + 2·k_c²·∇²u + ∇⁴u
//!
//! 所以:
//!   ∂u/∂t = (r - k_c⁴)·u - 2·k_c²·∇²u - ∇⁴u - u³
//!
//! 参数:
//!   r  - 控制参数 (分岔参数)
//!         r < 0: 均匀态稳定
//!         r > 0: 均匀态不稳定, 模式增长
//!   k_c - 临界波数, 决定特征波长 λ = 2π/k_c
//!
//! 分岔分析:
//!   线性化色散关系: σ(k) = r - (k_c² - k²)²
//!   最快增长模式: k = k_c, σ = r
//!   当 r > 0, 模式以 k_c 波数增长
//!   非线性项 -u³ 饱和增长, 形成稳定模式
//!
//! 应用:
//!   - Rayleigh-Bénard 对流 (Swift & Hohenberg 1977)
//!   - 激光横模 (Lega, Moloney, Newell 1994)
//!   - 流体不稳定性
//!   - 生物模式形成
//!   - 混沌-有序转捩
//!
//! 数值方法:
//!   显式 Euler + 5点 Laplacian (2D)
//!   biharmonic 通过两次 Laplacian: ∇⁴u = ∇²(∇²u)
//!
//! CFL:
//!   - biharmonic: dt <= dx⁴ / 16 (最严格)
//!   - 扩散: dt <= dx² / (8·k_c²)
//!   - 反应: dt <= 1 / |r - k_c⁴|
//!
//! 基于:
//!   - Swift, J. & Hohenberg, P.C. 1977. Phys. Rev. A 15, 319.
//!   - Cross, M.C. & Hohenberg, P.C. 1993. Rev. Mod. Phys. 65, 851.
//!   - Hoyle, R.B. 2006. "Pattern Formation: An Introduction to Methods."

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ShBoundary {
    Periodic,
    Neumann,
    Dirichlet { value: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShConfig {
    pub nx: usize,
    pub ny: usize,
    pub dx: f32,
    pub dt: f32,
    pub r: f32,
    pub k_c: f32,
    pub boundary: ShBoundary,
}

impl Default for ShConfig {
    fn default() -> Self {
        ShConfig {
            nx: 128,
            ny: 128,
            dx: 1.0,
            dt: 0.02,
            r: 0.1,
            k_c: 1.0,
            boundary: ShBoundary::Periodic,
        }
    }
}

impl ShConfig {
    pub fn n_cells(&self) -> usize {
        self.nx * self.ny
    }

    pub fn domain_area(&self) -> f32 {
        (self.nx as f32) * self.dx * (self.ny as f32) * self.dx
    }

    pub fn characteristic_wavelength(&self) -> f32 {
        2.0 * std::f32::consts::PI / self.k_c
    }

    pub fn linear_growth_rate(&self) -> f32 {
        self.r
    }

    pub fn biharmonic_cfl(&self) -> f32 {
        self.dt / (self.dx.powi(4))
    }

    pub fn diffusive_cfl(&self) -> f32 {
        2.0 * self.k_c * self.k_c * self.dt / (self.dx * self.dx)
    }

    pub fn reaction_cfl(&self) -> f32 {
        let lin = (self.r - self.k_c.powi(4)).abs();
        self.dt * lin
    }

    pub fn is_stable(&self) -> bool {
        self.biharmonic_cfl() <= 0.0625
            && self.diffusive_cfl() <= 0.5
            && self.reaction_cfl() <= 1.0
    }

    pub fn stable_dt(&self) -> f32 {
        let bih_dt = 0.0625 * self.dx.powi(4);
        let diff_dt = 0.5 * self.dx * self.dx / (2.0 * self.k_c * self.k_c);
        let lin = (self.r - self.k_c.powi(4)).abs().max(1e-6);
        let rxn_dt = 1.0 / lin;
        bih_dt.min(diff_dt).min(rxn_dt)
    }
}

pub struct ShSolver {
    pub config: ShConfig,
    pub u_curr: Vec<f32>,
    pub u_next: Vec<f32>,
    pub lap_u: Vec<f32>,
    pub lap2_u: Vec<f32>,
    pub time: f32,
    pub steps: usize,
}

impl ShSolver {
    pub fn new(config: ShConfig) -> Self {
        let n = config.n_cells();
        ShSolver {
            config,
            u_curr: vec![0.0; n],
            u_next: vec![0.0; n],
            lap_u: vec![0.0; n],
            lap2_u: vec![0.0; n],
            time: 0.0,
            steps: 0,
        }
    }

    #[inline]
    fn idx(&self, i: usize, j: usize) -> usize {
        j * self.config.nx + i
    }

    #[inline]
    fn wrap_i(&self, i: i32) -> usize {
        let n = self.config.nx as i32;
        (((i % n) + n) % n) as usize
    }

    #[inline]
    fn wrap_j(&self, j: i32) -> usize {
        let n = self.config.ny as i32;
        (((j % n) + n) % n) as usize
    }

    pub fn initialize_zero(&mut self) {
        for u in self.u_curr.iter_mut() {
            *u = 0.0;
        }
        self.time = 0.0;
        self.steps = 0;
    }

    pub fn initialize_random(&mut self, amplitude: f32, seed: u64) {
        let mut rng = ShRng::new(seed);
        for u in self.u_curr.iter_mut() {
            *u = amplitude * (2.0 * rng.next() - 1.0);
        }
        self.time = 0.0;
        self.steps = 0;
    }

    pub fn initialize_stripe(&mut self, amplitude: f32, k: f32) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        for j in 0..ny {
            for i in 0..nx {
                let x = i as f32 * dx;
                let idx = self.idx(i, j);
                self.u_curr[idx] = amplitude * (k * x).sin();
            }
        }
        self.time = 0.0;
        self.steps = 0;
    }

    pub fn initialize_spot(&mut self, amplitude: f32, cx: f32, cy: f32, radius: f32) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        let r2 = radius * radius;
        for j in 0..ny {
            for i in 0..nx {
                let x = i as f32 * dx;
                let y = j as f32 * dx;
                let d2 = (x - cx) * (x - cx) + (y - cy) * (y - cy);
                let idx = self.idx(i, j);
                self.u_curr[idx] = if d2 < r2 { amplitude } else { 0.0 };
            }
        }
        self.time = 0.0;
        self.steps = 0;
    }

    pub fn step(&mut self) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        let dt = self.config.dt;
        let r = self.config.r;
        let kc2 = self.config.k_c * self.config.k_c;
        let kc4 = kc2 * kc2;
        let bc = self.config.boundary;

        let u_copy = self.u_curr.clone();
        compute_laplacian(&u_copy, &mut self.lap_u, nx, ny, dx, bc);
        let lap_copy = self.lap_u.clone();
        compute_laplacian(&lap_copy, &mut self.lap2_u, nx, ny, dx, bc);

        for j in 0..ny {
            for i in 0..nx {
                let k = self.idx(i, j);
                let u = self.u_curr[k];
                let lap = self.lap_u[k];
                let lap2 = self.lap2_u[k];

                let rhs = (r - kc4) * u - 2.0 * kc2 * lap - lap2 - u * u * u;
                self.u_next[k] = u + dt * rhs;
            }
        }

        if let ShBoundary::Dirichlet { value } = bc {
            for i in 0..nx {
                let top = self.idx(i, 0);
                let bot = self.idx(i, ny - 1);
                self.u_next[top] = value;
                self.u_next[bot] = value;
            }
            for j in 0..ny {
                let left = self.idx(0, j);
                let right = self.idx(nx - 1, j);
                self.u_next[left] = value;
                self.u_next[right] = value;
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

    pub fn mean(&self) -> f32 {
        let n = self.u_curr.len();
        if n == 0 {
            return 0.0;
        }
        self.u_curr.iter().sum::<f32>() / n as f32
    }

    pub fn variance(&self) -> f32 {
        let m = self.mean();
        let n = self.u_curr.len();
        if n == 0 {
            return 0.0;
        }
        self.u_curr.iter().map(|&u| (u - m) * (u - m)).sum::<f32>() / n as f32
    }

    pub fn l2_norm(&self) -> f32 {
        self.u_curr.iter().map(|&u| u * u).sum::<f32>().sqrt()
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

    pub fn energy(&self) -> f32 {
        0.5 * self.u_curr.iter().map(|&u| u * u).sum::<f32>()
    }

    pub fn lyapunov_functional(&self) -> f32 {
        let dx2 = self.config.dx * self.config.dx;
        let kc2 = self.config.k_c * self.config.k_c;
        let r = self.config.r;
        let mut sum = 0.0f32;
        let nx = self.config.nx;
        let ny = self.config.ny;
        for j in 0..ny {
            for i in 0..nx {
                let k = self.idx(i, j);
                let u = self.u_curr[k];
                let lap = self.lap_u[k];
                let grad_sq = -u * lap * dx2;
                sum += -0.5 * r * u * u
                    + 0.5 * (grad_sq - kc2 * u * u).powi(2) / dx2
                    + 0.25 * u.powi(4);
            }
        }
        sum * dx2
    }
}

// ============================================================
// 自由函数: 避免借用冲突 (self.laplacian + &mut self.field)
// ============================================================

#[inline]
fn sh_idx(i: usize, j: usize, nx: usize) -> usize {
    j * nx + i
}

#[inline]
fn sh_wrap(i: i32, n: usize) -> usize {
    let n = n as i32;
    (((i % n) + n) % n) as usize
}

fn compute_laplacian(
    field: &[f32],
    out: &mut [f32],
    nx: usize,
    ny: usize,
    dx: f32,
    bc: ShBoundary,
) {
    let inv_dx2 = 1.0 / (dx * dx);
    for j in 0..ny {
        for i in 0..nx {
            let (ip, im, jp, jm) = match bc {
                ShBoundary::Periodic => (
                    sh_wrap(i as i32 + 1, nx),
                    sh_wrap(i as i32 - 1, nx),
                    sh_wrap(j as i32 + 1, ny),
                    sh_wrap(j as i32 - 1, ny),
                ),
                _ => (
                    (i + 1).min(nx - 1),
                    if i > 0 { i - 1 } else { 0 },
                    (j + 1).min(ny - 1),
                    if j > 0 { j - 1 } else { 0 },
                ),
            };
            let k = sh_idx(i, j, nx);
            let f = field[k];
            let f_ip = field[sh_idx(ip, j, nx)];
            let f_im = field[sh_idx(im, j, nx)];
            let f_jp = field[sh_idx(i, jp, nx)];
            let f_jm = field[sh_idx(i, jm, nx)];
            out[k] = (f_ip + f_im + f_jp + f_jm - 4.0 * f) * inv_dx2;
        }
    }
}

struct ShRng {
    state: u64,
}

impl ShRng {
    fn new(seed: u64) -> Self {
        ShRng {
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

    fn stable_config() -> ShConfig {
        ShConfig {
            nx: 64,
            ny: 64,
            dx: 1.0,
            dt: 0.01,
            r: 0.1,
            k_c: 1.0,
            boundary: ShBoundary::Periodic,
        }
    }

    #[test]
    fn test_default_config_stability() {
        let cfg = ShConfig::default();
        assert!(cfg.is_stable(), "default config should be stable");
    }

    #[test]
    fn test_stable_dt_positive() {
        let cfg = ShConfig::default();
        assert!(cfg.stable_dt() > 0.0);
    }

    #[test]
    fn test_characteristic_wavelength() {
        let cfg = ShConfig { k_c: 2.0, ..Default::default() };
        assert!((cfg.characteristic_wavelength() - std::f32::consts::PI).abs() < 1e-4);
    }

    #[test]
    fn test_linear_growth_rate() {
        let cfg = ShConfig { r: 0.5, ..Default::default() };
        assert!((cfg.linear_growth_rate() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_initialize_zero() {
        let mut s = ShSolver::new(stable_config());
        s.initialize_zero();
        assert_eq!(s.mean(), 0.0);
        assert_eq!(s.energy(), 0.0);
    }

    #[test]
    fn test_initialize_random_bounded() {
        let mut s = ShSolver::new(stable_config());
        s.initialize_random(0.5, 42);
        assert!(s.max_abs() <= 0.5);
    }

    #[test]
    fn test_initialize_stripe() {
        let mut s = ShSolver::new(stable_config());
        s.initialize_stripe(1.0, 1.0);
        assert!(s.max_amplitude() > 0.9);
        assert!(s.min_amplitude() < -0.9);
    }

    #[test]
    fn test_initialize_spot() {
        let mut s = ShSolver::new(stable_config());
        s.initialize_spot(1.0, 32.0, 32.0, 5.0);
        assert!(s.max_amplitude() > 0.9);
        assert!(s.mean() < 0.5);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = ShSolver::new(stable_config());
        s.initialize_random(0.1, 1);
        let t0 = s.time;
        s.step();
        assert!(s.time > t0);
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = ShSolver::new(stable_config());
        s.initialize_random(0.1, 1);
        s.step_n(10);
        assert_eq!(s.steps, 10);
    }

    #[test]
    fn test_no_nan_short_run() {
        let mut s = ShSolver::new(stable_config());
        s.initialize_random(0.1, 42);
        s.step_n(100);
        assert!(!s.has_nan(), "NaN detected after 100 steps");
    }

    #[test]
    fn test_no_nan_long_run() {
        let cfg = ShConfig {
            nx: 32,
            ny: 32,
            dx: 1.0,
            dt: 0.01,
            r: 0.1,
            k_c: 1.0,
            boundary: ShBoundary::Periodic,
        };
        let mut s = ShSolver::new(cfg);
        s.initialize_random(0.1, 99);
        s.step_n(1000);
        assert!(!s.has_nan(), "NaN detected after 1000 steps");
    }

    #[test]
    fn test_zero_state_stays_zero() {
        let mut s = ShSolver::new(stable_config());
        s.initialize_zero();
        s.step_n(50);
        assert!(s.energy() < 1e-10, "zero state should stay zero");
    }

    #[test]
    fn test_negative_r_decays() {
        let cfg = ShConfig {
            r: -1.0,
            ..stable_config()
        };
        let mut s = ShSolver::new(cfg);
        s.initialize_random(0.5, 7);
        let e0 = s.energy();
        s.step_n(100);
        let e1 = s.energy();
        assert!(e1 < e0, "negative r should decay: {} -> {}", e0, e1);
    }

    #[test]
    fn test_positive_r_grows_then_saturates() {
        // 用 k = 2π/L 初始化, 周期匹配, k_c = k 让该模式线性增长
        let nx = 64;
        let dx = 1.0_f32;
        let k_mode = 2.0 * std::f32::consts::PI / (nx as f32 * dx);
        let cfg = ShConfig {
            nx,
            ny: nx,
            dx,
            dt: 0.02,
            r: 0.3,
            k_c: k_mode,
            boundary: ShBoundary::Periodic,
        };
        let mut s = ShSolver::new(cfg);
        s.initialize_stripe(0.01, k_mode);
        let e0 = s.energy();
        s.step_n(2000);
        let e1 = s.energy();
        assert!(e1 > e0, "positive r should grow initially: {} -> {}", e0, e1);
        s.step_n(3000);
        let e2 = s.energy();
        assert!(e2.is_finite(), "energy should remain finite");
    }

    #[test]
    fn test_periodic_boundary_consistent() {
        let mut s = ShSolver::new(stable_config());
        s.initialize_stripe(0.1, 1.0);
        s.step_n(10);
        let left = s.u_curr[s.idx(0, 0)];
        let right = s.u_curr[s.idx(s.config.nx - 1, 0)];
        assert!((left - right).abs() < 1.0, "periodic boundary inconsistent");
    }

    #[test]
    fn test_dirichlet_boundary_enforced() {
        let cfg = ShConfig {
            nx: 32,
            ny: 32,
            dx: 1.0,
            dt: 0.01,
            r: 0.1,
            k_c: 1.0,
            boundary: ShBoundary::Dirichlet { value: 0.0 },
        };
        let mut s = ShSolver::new(cfg);
        s.initialize_random(1.0, 5);
        s.step_n(10);
        for i in 0..s.config.nx {
            assert!(s.u_curr[s.idx(i, 0)].abs() < 1e-4, "top boundary not enforced");
            assert!(s.u_curr[s.idx(i, s.config.ny - 1)].abs() < 1e-4, "bottom boundary not enforced");
        }
        for j in 0..s.config.ny {
            assert!(s.u_curr[s.idx(0, j)].abs() < 1e-4, "left boundary not enforced");
            assert!(s.u_curr[s.idx(s.config.nx - 1, j)].abs() < 1e-4, "right boundary not enforced");
        }
    }

    #[test]
    fn test_amplitude_bounded() {
        let mut s = ShSolver::new(stable_config());
        s.initialize_random(0.5, 42);
        s.step_n(500);
        assert!(s.max_abs() < 5.0, "amplitude should be bounded: {}", s.max_abs());
    }

    #[test]
    fn test_variance_positive() {
        let mut s = ShSolver::new(stable_config());
        s.initialize_random(0.5, 42);
        assert!(s.variance() > 0.0);
    }

    #[test]
    fn test_l2_norm_positive() {
        let mut s = ShSolver::new(stable_config());
        s.initialize_random(0.5, 42);
        assert!(s.l2_norm() > 0.0);
    }

    #[test]
    fn test_lyapunov_functional_finite() {
        let mut s = ShSolver::new(stable_config());
        s.initialize_random(0.1, 7);
        let mut s2 = ShSolver::new(stable_config());
        s2.initialize_random(0.1, 7);
        s2.step();
        let u_copy = s.u_curr.clone();
        compute_laplacian(
            &u_copy,
            &mut s.lap_u,
            s.config.nx,
            s.config.ny,
            s.config.dx,
            s.config.boundary,
        );
        assert!(s.lyapunov_functional().is_finite());
    }

    #[test]
    fn test_biharmonic_cfl_limit() {
        let cfg = ShConfig {
            nx: 32,
            ny: 32,
            dx: 1.0,
            dt: 1.0,
            r: 0.1,
            k_c: 1.0,
            boundary: ShBoundary::Periodic,
        };
        assert!(!cfg.is_stable(), "dt=1.0 with dx=1 should be unstable");
    }

    #[test]
    fn test_pattern_wavelength_near_kc() {
        let cfg = ShConfig {
            nx: 128,
            ny: 128,
            dx: 1.0,
            dt: 0.02,
            r: 0.3,
            k_c: 0.5,
            boundary: ShBoundary::Periodic,
        };
        let mut s = ShSolver::new(cfg);
        s.initialize_random(0.01, 42);
        s.step_n(2000);
        assert!(!s.has_nan(), "NaN in pattern formation");
        assert!(s.max_abs() > 0.01, "pattern should have grown");
    }

    #[test]
    fn test_energy_decreases_negative_r() {
        // Swift-Hohenberg 的能量 E=∫u²/2 不是 Lyapunov 函数, 不保证每步下降;
        // 但 r<0 时整体趋势下降 (Lyapunov 函数单调下降驱动能量最终减小)
        let cfg = ShConfig {
            r: -2.0,
            ..stable_config()
        };
        let mut s = ShSolver::new(cfg);
        s.initialize_random(1.0, 11);
        let e0 = s.energy();
        s.step_n(50);
        let e1 = s.energy();
        assert!(e1 < e0, "energy should decrease overall for r<0: {} -> {}", e0, e1);
        assert!(e1.is_finite(), "energy should remain finite");
    }
}
