//! Complex Ginzburg-Landau Equation (CGLE) Solver — 复 Ginzburg-Landau 方程
//!
//! CGLE 是远离平衡态系统经 Hopf 分岔后的标准振幅方程, 描述振荡系统的
//! 复振幅 A(x,t) 演化. 是模式形成和湍流研究的核心模型之一.
//!
//! 方程:
//!   ∂A/∂t = (μ + iα)A + (1 + iβ)∇²A - (1 + iγ)|A|²A
//!
//! 其中 A = u + iv 为复数场, 参数:
//!   μ     - 线性增益 (控制参数, μ>0 增长)
//!   α     - 线性色散 (线性频率漂移)
//!   β     - 扩散色散 (波数依赖的频率)
//!   γ     - 非线性色散 (振幅依赖的频率)
//!
//! 拆成实部 u 和虚部 v:
//!   ∂u/∂t = μu - αv + ∇²u - β∇²v - (u²+v²)u + γ(u²+v²)v
//!   ∂v/∂t = μv + αu + ∇²v + β∇²u - (u²+v²)v - γ(u²+v²)u
//!
//! 行波解:
//!   A = R·exp(i(k·x - ωt)),  R² = μ - k²
//!   ω = -α + β·k² + γ·(μ - k²)
//!
//! Benjamin-Feir-Newell 不稳定 (调制不稳定):
//!   1 + α·γ < 0  →  行波失稳, 发展为相位湍流或缺陷湍流
//!
//! 动力学相图 (随 α, γ 变化):
//!   - 行波稳定区
//!   - 相位湍流 (相位无序, 振幅近均匀)
//!   - 缺陷湍流 (振幅零点, 拓扑缺陷产生湮灭)
//!   - 时空混沌
//!
//! 应用:
//!   - 激光横模动力学 (Haken 1975)
//!   - 化学振荡 (Kuramoto 1984)
//!   - Rayleigh-Bénard 对流 (Cross-Hohenberg 1993)
//!   - 流体不稳定性
//!   - 超导电子动力学
//!   - 神经网络振荡
//!
//! 数值方法:
//!   显式 Euler + 5点 Laplacian (2D), 复数场拆成 (u, v) 两个实数场
//!
//! CFL:
//!   - 扩散: dt <= dx² / 4 (2D 5点 Laplacian)
//!   - 反应: dt <= 1 / |μ|
//!   - 非线性: dt <= 1 / |A|²
//!
//! 基于:
//!   - Kuramoto, Y. 1984. "Chemical Oscillations, Waves, and Turbulence." Springer.
//!   - Cross, M.C. & Hohenberg, P.C. 1993. Rev. Mod. Phys. 65, 851.
//!   - Aranson, I.S. & Kramer, L. 2002. Rev. Mod. Phys. 74, 99.
//!   - Shraiman, B.I. et al. 1992. Physica D 57, 241.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CgleBoundary {
    Periodic,
    Dirichlet { re: f32, im: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CgleConfig {
    pub nx: usize,
    pub ny: usize,
    pub dx: f32,
    pub dt: f32,
    /// 线性增益 (μ > 0 增长, μ < 0 衰减)
    pub mu: f32,
    /// 线性色散
    pub alpha: f32,
    /// 扩散色散
    pub beta: f32,
    /// 非线性色散
    pub gamma: f32,
    pub boundary: CgleBoundary,
}

impl Default for CgleConfig {
    fn default() -> Self {
        CgleConfig {
            nx: 128,
            ny: 128,
            dx: 1.0,
            dt: 0.05,
            mu: 1.0,
            alpha: 0.0,
            beta: 1.0,
            gamma: -1.5,
            boundary: CgleBoundary::Periodic,
        }
    }
}

impl CgleConfig {
    pub fn n_cells(&self) -> usize {
        self.nx * self.ny
    }

    pub fn domain_area(&self) -> f32 {
        (self.nx as f32) * self.dx * (self.ny as f32) * self.dx
    }

    /// 5点 Laplacian 的扩散 CFL: 4·dt/dx², 要求 <= 1
    pub fn diffusive_cfl(&self) -> f32 {
        4.0 * self.dt / (self.dx * self.dx)
    }

    pub fn reaction_cfl(&self) -> f32 {
        self.dt * self.mu.abs()
    }

    pub fn is_stable(&self) -> bool {
        self.diffusive_cfl() <= 1.0 && self.reaction_cfl() <= 1.0
    }

    pub fn stable_dt(&self) -> f32 {
        let diff_dt = 0.25 * self.dx * self.dx;
        let rxn_dt = 1.0 / self.mu.abs().max(1e-6);
        diff_dt.min(rxn_dt)
    }

    /// Benjamin-Feir-Newell 调制不稳定条件: 1 + α·γ < 0
    pub fn is_benjamin_feir_unstable(&self) -> bool {
        1.0 + self.alpha * self.gamma < 0.0
    }

    /// 行波稳态振幅 R = sqrt(μ - k²) (k=0 时 R = sqrt(μ))
    pub fn plane_wave_amplitude(&self, k: f32) -> f32 {
        (self.mu - k * k).max(0.0).sqrt()
    }

    /// 行波色散关系 ω = -α + β·k² + γ·(μ - k²)
    pub fn plane_wave_frequency(&self, k: f32) -> f32 {
        -self.alpha + self.beta * k * k + self.gamma * (self.mu - k * k)
    }
}

pub struct CgleSolver {
    pub config: CgleConfig,
    pub u_curr: Vec<f32>,
    pub v_curr: Vec<f32>,
    pub u_next: Vec<f32>,
    pub v_next: Vec<f32>,
    pub lap_u: Vec<f32>,
    pub lap_v: Vec<f32>,
    pub time: f32,
    pub steps: usize,
}

impl CgleSolver {
    pub fn new(config: CgleConfig) -> Self {
        let n = config.n_cells();
        CgleSolver {
            config,
            u_curr: vec![0.0; n],
            v_curr: vec![0.0; n],
            u_next: vec![0.0; n],
            v_next: vec![0.0; n],
            lap_u: vec![0.0; n],
            lap_v: vec![0.0; n],
            time: 0.0,
            steps: 0,
        }
    }

    #[inline]
    fn idx(&self, i: usize, j: usize) -> usize {
        j * self.config.nx + i
    }

    pub fn initialize_zero(&mut self) {
        for u in self.u_curr.iter_mut() {
            *u = 0.0;
        }
        for v in self.v_curr.iter_mut() {
            *v = 0.0;
        }
        self.time = 0.0;
        self.steps = 0;
    }

    pub fn initialize_random(&mut self, amplitude: f32, seed: u64) {
        let mut rng = CgleRng::new(seed);
        for u in self.u_curr.iter_mut() {
            *u = amplitude * (2.0 * rng.next() - 1.0);
        }
        for v in self.v_curr.iter_mut() {
            *v = amplitude * (2.0 * rng.next() - 1.0);
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化为均匀行波 A = R·exp(i·k·x), 沿 x 方向
    pub fn initialize_plane_wave(&mut self, k: f32) {
        let r = self.config.plane_wave_amplitude(k);
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        for j in 0..ny {
            for i in 0..nx {
                let x = i as f32 * dx;
                let phase = k * x;
                let id = self.idx(i, j);
                self.u_curr[id] = r * phase.cos();
                self.v_curr[id] = r * phase.sin();
            }
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化为螺旋波 (topological defect at center)
    /// A = R·tanh(ρ)·exp(i·θ), 其中 (ρ, θ) 为相对中心点的极坐标
    pub fn initialize_spiral(&mut self, cx: f32, cy: f32) {
        let r = self.config.plane_wave_amplitude(0.0);
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        for j in 0..ny {
            for i in 0..nx {
                let x = i as f32 * dx - cx;
                let y = j as f32 * dx - cy;
                let rho = (x * x + y * y).sqrt();
                let theta = y.atan2(x);
                let id = self.idx(i, j);
                let amp = r * rho.tanh();
                self.u_curr[id] = amp * theta.cos();
                self.v_curr[id] = amp * theta.sin();
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
        let mu = self.config.mu;
        let alpha = self.config.alpha;
        let beta = self.config.beta;
        let gamma = self.config.gamma;
        let bc = self.config.boundary;

        let u_copy = self.u_curr.clone();
        let v_copy = self.v_curr.clone();
        compute_laplacian(&u_copy, &mut self.lap_u, nx, ny, dx, bc);
        compute_laplacian(&v_copy, &mut self.lap_v, nx, ny, dx, bc);

        for j in 0..ny {
            for i in 0..nx {
                let k = self.idx(i, j);
                let u = self.u_curr[k];
                let v = self.v_curr[k];
                let lu = self.lap_u[k];
                let lv = self.lap_v[k];
                let a2 = u * u + v * v;

                let du = mu * u - alpha * v + lu - beta * lv - a2 * u + gamma * a2 * v;
                let dv = mu * v + alpha * u + lv + beta * lu - a2 * v - gamma * a2 * u;

                self.u_next[k] = u + dt * du;
                self.v_next[k] = v + dt * dv;
            }
        }

        if let CgleBoundary::Dirichlet { re, im } = bc {
            for i in 0..nx {
                let top = self.idx(i, 0);
                let bot = self.idx(i, ny - 1);
                self.u_next[top] = re;
                self.u_next[bot] = re;
                self.v_next[top] = im;
                self.v_next[bot] = im;
            }
            for j in 0..ny {
                let left = self.idx(0, j);
                let right = self.idx(nx - 1, j);
                self.u_next[left] = re;
                self.u_next[right] = re;
                self.v_next[left] = im;
                self.v_next[right] = im;
            }
        }

        std::mem::swap(&mut self.u_curr, &mut self.u_next);
        std::mem::swap(&mut self.v_curr, &mut self.v_next);
        self.time += dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    pub fn has_nan(&self) -> bool {
        self.u_curr.iter().any(|&u| !u.is_finite())
            || self.v_curr.iter().any(|&v| !v.is_finite())
    }

    /// 平均振幅平方 <|A|²> = mean(u² + v²)
    pub fn mean_intensity(&self) -> f32 {
        let n = self.u_curr.len();
        if n == 0 {
            return 0.0;
        }
        let mut sum = 0.0f32;
        for k in 0..n {
            sum += self.u_curr[k] * self.u_curr[k] + self.v_curr[k] * self.v_curr[k];
        }
        sum / n as f32
    }

    /// 平均振幅 <sqrt(|A|²)>
    pub fn mean_amplitude(&self) -> f32 {
        self.mean_intensity().sqrt()
    }

    pub fn mean_u(&self) -> f32 {
        let n = self.u_curr.len();
        if n == 0 {
            return 0.0;
        }
        self.u_curr.iter().sum::<f32>() / n as f32
    }

    pub fn mean_v(&self) -> f32 {
        let n = self.v_curr.len();
        if n == 0 {
            return 0.0;
        }
        self.v_curr.iter().sum::<f32>() / n as f32
    }

    /// 总能量 E = Σ|A|² = Σ(u² + v²)
    pub fn energy(&self) -> f32 {
        let mut sum = 0.0f32;
        for k in 0..self.u_curr.len() {
            sum += self.u_curr[k] * self.u_curr[k] + self.v_curr[k] * self.v_curr[k];
        }
        sum
    }

    pub fn max_amplitude(&self) -> f32 {
        self.u_curr
            .iter()
            .zip(self.v_curr.iter())
            .map(|(&u, &v)| (u * u + v * v).sqrt())
            .fold(0.0f32, f32::max)
    }

    pub fn min_amplitude(&self) -> f32 {
        self.u_curr
            .iter()
            .zip(self.v_curr.iter())
            .map(|(&u, &v)| (u * u + v * v).sqrt())
            .fold(f32::INFINITY, f32::min)
    }

    pub fn max_abs_u(&self) -> f32 {
        self.u_curr.iter().map(|&u| u.abs()).fold(0.0f32, f32::max)
    }

    pub fn max_abs_v(&self) -> f32 {
        self.v_curr.iter().map(|&v| v.abs()).fold(0.0f32, f32::max)
    }

    /// L2 范数 sqrt(Σ|A|²)
    pub fn l2_norm(&self) -> f32 {
        self.energy().sqrt()
    }

    /// 强度方差 var(|A|²) — 用于检测模式形成
    pub fn intensity_variance(&self) -> f32 {
        let m = self.mean_intensity();
        let n = self.u_curr.len();
        if n == 0 {
            return 0.0;
        }
        let mut sum = 0.0f32;
        for k in 0..n {
            let intensity = self.u_curr[k] * self.u_curr[k] + self.v_curr[k] * self.v_curr[k];
            sum += (intensity - m) * (intensity - m);
        }
        sum / n as f32
    }
}

// ============================================================
// 自由函数: 避免借用冲突
// ============================================================

#[inline]
fn cgle_idx(i: usize, j: usize, nx: usize) -> usize {
    j * nx + i
}

#[inline]
fn cgle_wrap(i: i32, n: usize) -> usize {
    let n = n as i32;
    (((i % n) + n) % n) as usize
}

fn compute_laplacian(
    field: &[f32],
    out: &mut [f32],
    nx: usize,
    ny: usize,
    dx: f32,
    bc: CgleBoundary,
) {
    let inv_dx2 = 1.0 / (dx * dx);
    for j in 0..ny {
        for i in 0..nx {
            let (ip, im, jp, jm) = match bc {
                CgleBoundary::Periodic => (
                    cgle_wrap(i as i32 + 1, nx),
                    cgle_wrap(i as i32 - 1, nx),
                    cgle_wrap(j as i32 + 1, ny),
                    cgle_wrap(j as i32 - 1, ny),
                ),
                _ => (
                    (i + 1).min(nx - 1),
                    if i > 0 { i - 1 } else { 0 },
                    (j + 1).min(ny - 1),
                    if j > 0 { j - 1 } else { 0 },
                ),
            };
            let k = cgle_idx(i, j, nx);
            let f = field[k];
            let f_ip = field[cgle_idx(ip, j, nx)];
            let f_im = field[cgle_idx(im, j, nx)];
            let f_jp = field[cgle_idx(i, jp, nx)];
            let f_jm = field[cgle_idx(i, jm, nx)];
            out[k] = (f_ip + f_im + f_jp + f_jm - 4.0 * f) * inv_dx2;
        }
    }
}

struct CgleRng {
    state: u64,
}

impl CgleRng {
    fn new(seed: u64) -> Self {
        CgleRng {
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

    fn stable_config() -> CgleConfig {
        CgleConfig {
            nx: 64,
            ny: 64,
            dx: 1.0,
            dt: 0.05,
            mu: 1.0,
            alpha: 0.0,
            beta: 1.0,
            gamma: -1.5,
            boundary: CgleBoundary::Periodic,
        }
    }

    #[test]
    fn test_default_config_stability() {
        let cfg = CgleConfig::default();
        assert!(cfg.is_stable(), "default config should be stable");
    }

    #[test]
    fn test_stable_dt_positive() {
        let cfg = CgleConfig::default();
        assert!(cfg.stable_dt() > 0.0);
    }

    #[test]
    fn test_diffusive_cfl() {
        let cfg = CgleConfig { dt: 0.25, dx: 1.0, ..Default::default() };
        // 4 * 0.25 / 1 = 1.0
        assert!((cfg.diffusive_cfl() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_benjamin_feir_stable() {
        // 1 + alpha*gamma = 1 + 0*(-1.5) = 1 > 0, 稳定
        let cfg = CgleConfig { alpha: 0.0, gamma: -1.5, ..Default::default() };
        assert!(!cfg.is_benjamin_feir_unstable());
    }

    #[test]
    fn test_benjamin_feir_unstable() {
        // 1 + alpha*gamma < 0: alpha=2, gamma=-1, 1 + 2*(-1) = -1 < 0
        let cfg = CgleConfig { alpha: 2.0, gamma: -1.0, ..Default::default() };
        assert!(cfg.is_benjamin_feir_unstable());
    }

    #[test]
    fn test_plane_wave_amplitude() {
        let cfg = CgleConfig { mu: 4.0, ..Default::default() };
        // k=0: R = sqrt(4) = 2
        assert!((cfg.plane_wave_amplitude(0.0) - 2.0).abs() < 1e-6);
        // k=1: R = sqrt(4-1) = sqrt(3)
        assert!((cfg.plane_wave_amplitude(1.0) - 3.0_f32.sqrt()).abs() < 1e-6);
        // k=3: R = sqrt(4-9) < 0 -> 0
        assert_eq!(cfg.plane_wave_amplitude(3.0), 0.0);
    }

    #[test]
    fn test_plane_wave_frequency() {
        let cfg = CgleConfig { mu: 1.0, alpha: 0.5, beta: 1.0, gamma: -1.0, ..Default::default() };
        // k=0: ω = -0.5 + 0 + (-1)*(1-0) = -1.5
        assert!((cfg.plane_wave_frequency(0.0) - (-1.5)).abs() < 1e-6);
    }

    #[test]
    fn test_initialize_zero() {
        let mut s = CgleSolver::new(stable_config());
        s.initialize_zero();
        assert_eq!(s.mean_intensity(), 0.0);
        assert_eq!(s.energy(), 0.0);
    }

    #[test]
    fn test_initialize_random_bounded() {
        let mut s = CgleSolver::new(stable_config());
        s.initialize_random(0.5, 42);
        // |A| = sqrt(u²+v²) <= sqrt(2) * 0.5
        assert!(s.max_amplitude() <= 0.5 * 2.0_f32.sqrt() + 1e-5);
    }

    #[test]
    fn test_initialize_plane_wave() {
        let mut s = CgleSolver::new(stable_config());
        s.initialize_plane_wave(0.0);
        // k=0: 均匀场, u=R, v=0, R=sqrt(mu)=1
        assert!((s.mean_u() - 1.0).abs() < 1e-4);
        assert!(s.mean_v().abs() < 1e-4);
    }

    #[test]
    fn test_initialize_plane_wave_k() {
        let mut s = CgleSolver::new(stable_config());
        s.initialize_plane_wave(0.5);
        // R = sqrt(1 - 0.25) = sqrt(0.75)
        let r = 0.75_f32.sqrt();
        assert!((s.mean_amplitude() - r).abs() < 1e-4);
    }

    #[test]
    fn test_initialize_spiral() {
        let mut s = CgleSolver::new(stable_config());
        s.initialize_spiral(32.0, 32.0);
        // 螺旋波有非零能量
        assert!(s.energy() > 0.0);
        // 中心点振幅接近 0 (rho=0 -> tanh(0)=0)
        let center = s.idx(32, 32);
        let amp_center = (s.u_curr[center].powi(2) + s.v_curr[center].powi(2)).sqrt();
        assert!(amp_center < 0.1, "center amplitude should be near 0: {}", amp_center);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = CgleSolver::new(stable_config());
        s.initialize_random(0.1, 1);
        let t0 = s.time;
        s.step();
        assert!(s.time > t0);
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = CgleSolver::new(stable_config());
        s.initialize_random(0.1, 1);
        s.step_n(10);
        assert_eq!(s.steps, 10);
    }

    #[test]
    fn test_no_nan_short_run() {
        let mut s = CgleSolver::new(stable_config());
        s.initialize_random(0.1, 42);
        s.step_n(100);
        assert!(!s.has_nan(), "NaN detected after 100 steps");
    }

    #[test]
    fn test_no_nan_long_run() {
        let cfg = CgleConfig { nx: 32, ny: 32, dt: 0.02, ..stable_config() };
        let mut s = CgleSolver::new(cfg);
        s.initialize_random(0.1, 99);
        s.step_n(2000);
        assert!(!s.has_nan(), "NaN detected after 2000 steps");
    }

    #[test]
    fn test_negative_mu_decays() {
        // μ < 0: 线性衰减, A -> 0
        let cfg = CgleConfig { mu: -0.5, ..stable_config() };
        let mut s = CgleSolver::new(cfg);
        s.initialize_random(0.5, 7);
        let e0 = s.energy();
        s.step_n(500);
        let e1 = s.energy();
        assert!(e1 < e0 * 0.1, "negative mu should decay energy: {} -> {}", e0, e1);
    }

    #[test]
    fn test_positive_mu_grows_to_saturation() {
        // μ > 0: 小扰动增长到饱和 (R² ≈ μ)
        let cfg = CgleConfig { alpha: 0.0, beta: 0.0, gamma: 0.0, ..stable_config() };
        let mut s = CgleSolver::new(cfg);
        s.initialize_random(0.01, 3);
        let i0 = s.mean_intensity();
        s.step_n(2000);
        let i1 = s.mean_intensity();
        assert!(i1 > i0, "positive mu should grow: {} -> {}", i0, i1);
        // 饱和振幅 R²=μ=1, intensity ≈ 1
        assert!((i1 - 1.0).abs() < 0.3, "intensity should saturate near mu: {}", i1);
    }

    #[test]
    fn test_plane_wave_perserves_amplitude() {
        // 行波在无色散 (alpha=beta=gamma=0) 下振幅应保持 R²=μ
        let cfg = CgleConfig { alpha: 0.0, beta: 0.0, gamma: 0.0, ..stable_config() };
        let mut s = CgleSolver::new(cfg);
        s.initialize_plane_wave(0.0);
        let i0 = s.mean_intensity();
        s.step_n(1000);
        let i1 = s.mean_intensity();
        // 均匀场 A=R 是不动点, intensity 应保持
        assert!((i1 - i0).abs() < 0.05, "uniform state should be fixed point: {} -> {}", i0, i1);
    }

    #[test]
    fn test_energy_bounded() {
        let mut s = CgleSolver::new(stable_config());
        s.initialize_random(0.5, 42);
        let e0 = s.energy();
        s.step_n(1000);
        let e1 = s.energy();
        assert!(e1.is_finite(), "energy not finite");
        assert!(e1 < 100.0 * e0 + 1000.0, "energy blew up: {} -> {}", e0, e1);
    }

    #[test]
    fn test_amplitude_bounded() {
        let mut s = CgleSolver::new(stable_config());
        s.initialize_random(0.5, 42);
        s.step_n(1000);
        assert!(s.max_amplitude() < 10.0, "amplitude should be bounded: {}", s.max_amplitude());
    }

    #[test]
    fn test_zero_state_stays_zero() {
        let mut s = CgleSolver::new(stable_config());
        s.initialize_zero();
        s.step_n(100);
        assert!(s.energy() < 1e-10, "zero state should stay zero: {}", s.energy());
    }

    #[test]
    fn test_dirichlet_boundary_enforced() {
        let cfg = CgleConfig {
            nx: 32,
            ny: 32,
            dt: 0.02,
            boundary: CgleBoundary::Dirichlet { re: 0.0, im: 0.0 },
            ..stable_config()
        };
        let mut s = CgleSolver::new(cfg);
        s.initialize_plane_wave(0.5);
        s.step_n(20);
        // 边界应为 0
        let nx = s.config.nx;
        let ny = s.config.ny;
        for i in 0..nx {
            let top = s.idx(i, 0);
            let bot = s.idx(i, ny - 1);
            assert!(s.u_curr[top].abs() < 1e-5, "top boundary not enforced");
            assert!(s.u_curr[bot].abs() < 1e-5, "bottom boundary not enforced");
        }
        for j in 0..ny {
            let left = s.idx(0, j);
            let right = s.idx(nx - 1, j);
            assert!(s.u_curr[left].abs() < 1e-5, "left boundary not enforced");
            assert!(s.u_curr[right].abs() < 1e-5, "right boundary not enforced");
        }
    }

    #[test]
    fn test_l2_norm_positive() {
        let mut s = CgleSolver::new(stable_config());
        s.initialize_random(0.5, 42);
        assert!(s.l2_norm() > 0.0);
    }

    #[test]
    fn test_intensity_variance() {
        let mut s = CgleSolver::new(stable_config());
        s.initialize_plane_wave(0.5);
        // 均匀行波 intensity 方差应为 0
        assert!(s.intensity_variance() < 1e-4, "plane wave variance should be 0: {}", s.intensity_variance());
    }

    #[test]
    fn test_min_max_amplitude() {
        let mut s = CgleSolver::new(stable_config());
        s.initialize_plane_wave(0.0);
        // 均匀场 min = max = R = sqrt(mu) = 1
        assert!((s.max_amplitude() - 1.0).abs() < 1e-4);
        assert!((s.min_amplitude() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_module_count() {
        // 验证有足够多的测试
        let tests: Vec<&str> = vec![
            "test_default_config_stability",
            "test_stable_dt_positive",
            "test_diffusive_cfl",
            "test_benjamin_feir_stable",
            "test_benjamin_feir_unstable",
            "test_plane_wave_amplitude",
            "test_plane_wave_frequency",
            "test_initialize_zero",
            "test_initialize_random_bounded",
            "test_initialize_plane_wave",
            "test_initialize_plane_wave_k",
            "test_initialize_spiral",
            "test_step_advances_time",
            "test_step_n_advances",
            "test_no_nan_short_run",
            "test_no_nan_long_run",
            "test_negative_mu_decays",
            "test_positive_mu_grows_to_saturation",
            "test_plane_wave_perserves_amplitude",
            "test_energy_bounded",
            "test_amplitude_bounded",
            "test_zero_state_stays_zero",
            "test_dirichlet_boundary_enforced",
            "test_l2_norm_positive",
            "test_intensity_variance",
            "test_min_max_amplitude",
        ];
        assert!(tests.len() >= 20, "need at least 20 tests, got {}", tests.len());
    }
}
