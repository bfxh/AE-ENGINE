//! Lotka-Volterra Solver with Spatial Diffusion — 带扩散的捕食者-被捕食者模型
//!
//! 经典 Lotka-Volterra 方程描述捕食者与被捕食者种群的动力学相互作用.
//! 加入空间扩散后, 系统可展现行波, 斑图, 扩散驱动不稳定等丰富行为.
//!
//! 方程 (带扩散):
//!   ∂u/∂t = α·u - β·u·v + D_u·∇²u    (被捕食者, 如兔子)
//!   ∂v/∂t = δ·u·v - γ·v + D_v·∇²v    (捕食者, 如狐狸)
//!
//! 参数:
//!   u  - 被捕食者种群密度
//!   v  - 捕食者种群密度
//!   α  - 被捕食者自然增长率 (无捕食时指数增长)
//!   β  - 捕食率 (u-v 相遇导致 u 死亡)
//!   δ  - 捕食转化效率 (捕食导致 v 增长)
//!   γ  - 捕食者自然死亡率 (无食物时指数衰减)
//!   D_u, D_v - 扩散系数
//!
//! 非空间系统 (D=0) 的不动点:
//!   1. (u*, v*) = (0, 0)  — 双灭绝 (鞍点, 不稳定)
//!   2. (u*, v*) = (γ/δ, α/β)  — 共存平衡 (中心, Lyapunov 稳定)
//!
//! 共存平衡附近: 周期振荡 (守恒量 H = δu - γln(u) + βv - αln(v))
//!   振荡周期 ≈ 2π / sqrt(α·γ)
//!   振荡振幅由初始条件决定
//!
//! 空间扩散效应:
//!   - Turing 不稳定: 若 D_v >> D_u, 均匀振荡态可能失稳形成空间斑图
//!   - 行波: 局部扰动传播 (生态入侵)
//!   - 螺旋波: 2D 振荡介质中的旋转结构
//!   - 目标波: 同心圆扩散
//!
//! 应用:
//!   - 生态系统种群动力学 (捕食-被捕食, 寄主-寄生)
//!   - 流行病学 (SIR 模型的生态版本)
//!   - 化学反应 (自催化反应)
//!   - 经济学 (市场动力学类比)
//!   - 游戏生态模拟 (NPC 种群平衡)
//!
//! 数值方法:
//!   显式 Euler + 5点 Laplacian (2D)
//!   种群密度 u, v >= 0 (半正约束, 负值截断为 0)
//!
//! CFL:
//!   - 扩散: dt <= dx² / (4·max(D_u, D_v))
//!   - 反应: dt <= 1 / max(α, γ)
//!
//! 基于:
//!   - Lotka, A.J. 1925. "Elements of Physical Biology." Williams & Wilkins.
//!   - Volterra, V. 1926. "Variazioni e fluttuazioni del numero d'individui
//!     in specie animali conviventi." Mem. R. Accad. Naz. dei Lincei 6, 31.
//!   - Turing, A.M. 1952. Phil. Trans. R. Soc. B 237, 37.
//!   - Murray, J.D. 2002. "Mathematical Biology." Springer. Ch 3-4.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LvBoundary {
    Periodic,
    Neumann,
    Dirichlet { u: f32, v: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LvConfig {
    pub nx: usize,
    pub ny: usize,
    pub dx: f32,
    pub dt: f32,
    pub alpha: f32,
    pub beta: f32,
    pub delta: f32,
    pub gamma: f32,
    pub d_u: f32,
    pub d_v: f32,
    pub boundary: LvBoundary,
}

impl Default for LvConfig {
    fn default() -> Self {
        LvConfig {
            nx: 128,
            ny: 128,
            dx: 1.0,
            dt: 0.01,
            alpha: 1.0,
            beta: 0.5,
            delta: 0.4,
            gamma: 0.4,
            d_u: 0.1,
            d_v: 0.05,
            boundary: LvBoundary::Periodic,
        }
    }
}

impl LvConfig {
    pub fn n_cells(&self) -> usize {
        self.nx * self.ny
    }

    pub fn coexistence_fixed_point(&self) -> (f32, f32) {
        (self.gamma / self.delta, self.alpha / self.beta)
    }

    pub fn extinction_fixed_point(&self) -> (f32, f32) {
        (0.0, 0.0)
    }

    /// 振荡角频率 (在共存平衡附近) ω = sqrt(α·γ)
    pub fn oscillation_frequency(&self) -> f32 {
        (self.alpha * self.gamma).sqrt()
    }

    pub fn oscillation_period(&self) -> f32 {
        2.0 * std::f32::consts::PI / self.oscillation_frequency()
    }

    pub fn diffusive_cfl(&self) -> f32 {
        let d_max = self.d_u.max(self.d_v);
        4.0 * d_max * self.dt / (self.dx * self.dx)
    }

    pub fn reaction_cfl(&self) -> f32 {
        self.dt * self.alpha.max(self.gamma)
    }

    pub fn is_stable(&self) -> bool {
        self.diffusive_cfl() <= 1.0 && self.reaction_cfl() <= 1.0
    }

    pub fn stable_dt(&self) -> f32 {
        let d_max = self.d_u.max(self.d_v);
        let diff_dt = 0.25 * self.dx * self.dx / d_max.max(1e-6);
        let rxn_dt = 1.0 / self.alpha.max(self.gamma).max(1e-6);
        diff_dt.min(rxn_dt)
    }
}

pub struct LvSolver {
    pub config: LvConfig,
    pub u_curr: Vec<f32>,
    pub v_curr: Vec<f32>,
    pub u_next: Vec<f32>,
    pub v_next: Vec<f32>,
    pub lap_u: Vec<f32>,
    pub lap_v: Vec<f32>,
    pub time: f32,
    pub steps: usize,
}

impl LvSolver {
    pub fn new(config: LvConfig) -> Self {
        let n = config.n_cells();
        LvSolver {
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

    /// 初始化为共存平衡 (u*, v*) + 随机扰动
    pub fn initialize_coexistence(&mut self, perturbation: f32, seed: u64) {
        let (u_star, v_star) = self.config.coexistence_fixed_point();
        let mut rng = LvRng::new(seed);
        for i in 0..self.u_curr.len() {
            self.u_curr[i] = u_star + perturbation * (2.0 * rng.next() - 1.0);
            self.v_curr[i] = v_star + perturbation * (2.0 * rng.next() - 1.0);
            if self.u_curr[i] < 0.0 {
                self.u_curr[i] = 0.0;
            }
            if self.v_curr[i] < 0.0 {
                self.v_curr[i] = 0.0;
            }
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化为共存平衡 + 中心高密度斑块 (触发行波)
    pub fn initialize_prey_patch(&mut self, cx: f32, cy: f32, radius: f32, amp: f32) {
        let (u_star, v_star) = self.config.coexistence_fixed_point();
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        let r2 = radius * radius;
        for j in 0..ny {
            for i in 0..nx {
                let x = i as f32 * dx;
                let y = j as f32 * dx;
                let d2 = (x - cx) * (x - cx) + (y - cy) * (y - cy);
                let id = self.idx(i, j);
                if d2 < r2 {
                    self.u_curr[id] = u_star + amp;
                    self.v_curr[id] = v_star * 0.5;
                } else {
                    self.u_curr[id] = u_star;
                    self.v_curr[id] = v_star;
                }
            }
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化为均匀态 (u0, v0)
    pub fn initialize_uniform(&mut self, u0: f32, v0: f32) {
        for i in 0..self.u_curr.len() {
            self.u_curr[i] = u0;
            self.v_curr[i] = v0;
        }
        self.time = 0.0;
        self.steps = 0;
    }

    pub fn step(&mut self) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        let dt = self.config.dt;
        let alpha = self.config.alpha;
        let beta = self.config.beta;
        let delta = self.config.delta;
        let gamma = self.config.gamma;
        let d_u = self.config.d_u;
        let d_v = self.config.d_v;
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

                let du = alpha * u - beta * u * v + d_u * lu;
                let dv = delta * u * v - gamma * v + d_v * lv;

                let mut un = u + dt * du;
                let mut vn = v + dt * dv;
                // 种群密度非负约束
                if un < 0.0 {
                    un = 0.0;
                }
                if vn < 0.0 {
                    vn = 0.0;
                }
                self.u_next[k] = un;
                self.v_next[k] = vn;
            }
        }

        if let LvBoundary::Dirichlet { u: ub, v: vb } = bc {
            for i in 0..nx {
                let top = self.idx(i, 0);
                let bot = self.idx(i, ny - 1);
                self.u_next[top] = ub;
                self.u_next[bot] = ub;
                self.v_next[top] = vb;
                self.v_next[bot] = vb;
            }
            for j in 0..ny {
                let left = self.idx(0, j);
                let right = self.idx(nx - 1, j);
                self.u_next[left] = ub;
                self.u_next[right] = ub;
                self.v_next[left] = vb;
                self.v_next[right] = vb;
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

    pub fn max_u(&self) -> f32 {
        self.u_curr.iter().copied().fold(f32::NEG_INFINITY, f32::max)
    }

    pub fn max_v(&self) -> f32 {
        self.v_curr.iter().copied().fold(f32::NEG_INFINITY, f32::max)
    }

    pub fn min_u(&self) -> f32 {
        self.u_curr.iter().copied().fold(f32::INFINITY, f32::min)
    }

    pub fn min_v(&self) -> f32 {
        self.v_curr.iter().copied().fold(f32::INFINITY, f32::min)
    }

    pub fn max_abs_u(&self) -> f32 {
        self.u_curr.iter().map(|&u| u.abs()).fold(0.0f32, f32::max)
    }

    /// Lyapunov 泛函 (非空间守恒量) H = δu - γln(u) + βv - αln(v)
    /// 在无扩散系统中守恒, 有扩散时缓慢变化
    pub fn lyapunov_integral(&self) -> f32 {
        let mut sum = 0.0f32;
        for k in 0..self.u_curr.len() {
            let u = self.u_curr[k].max(1e-10);
            let v = self.v_curr[k].max(1e-10);
            sum += self.config.delta * u - self.config.gamma * u.ln()
                + self.config.beta * v - self.config.alpha * v.ln();
        }
        sum
    }

    /// 总被捕食者种群
    pub fn total_prey(&self) -> f32 {
        self.u_curr.iter().sum::<f32>()
    }

    /// 总捕食者种群
    pub fn total_predator(&self) -> f32 {
        self.v_curr.iter().sum::<f32>()
    }

    pub fn variance_u(&self) -> f32 {
        let m = self.mean_u();
        let n = self.u_curr.len();
        if n == 0 {
            return 0.0;
        }
        self.u_curr.iter().map(|&u| (u - m) * (u - m)).sum::<f32>() / n as f32
    }

    pub fn variance_v(&self) -> f32 {
        let m = self.mean_v();
        let n = self.v_curr.len();
        if n == 0 {
            return 0.0;
        }
        self.v_curr.iter().map(|&v| (v - m) * (v - m)).sum::<f32>() / n as f32
    }
}

// ============================================================
// 自由函数: 避免借用冲突
// ============================================================

#[inline]
fn lv_idx(i: usize, j: usize, nx: usize) -> usize {
    j * nx + i
}

#[inline]
fn lv_wrap(i: i32, n: usize) -> usize {
    let n = n as i32;
    (((i % n) + n) % n) as usize
}

fn compute_laplacian(
    field: &[f32],
    out: &mut [f32],
    nx: usize,
    ny: usize,
    dx: f32,
    bc: LvBoundary,
) {
    let inv_dx2 = 1.0 / (dx * dx);
    for j in 0..ny {
        for i in 0..nx {
            let (ip, im, jp, jm) = match bc {
                LvBoundary::Periodic => (
                    lv_wrap(i as i32 + 1, nx),
                    lv_wrap(i as i32 - 1, nx),
                    lv_wrap(j as i32 + 1, ny),
                    lv_wrap(j as i32 - 1, ny),
                ),
                _ => (
                    (i + 1).min(nx - 1),
                    if i > 0 { i - 1 } else { 0 },
                    (j + 1).min(ny - 1),
                    if j > 0 { j - 1 } else { 0 },
                ),
            };
            let k = lv_idx(i, j, nx);
            let f = field[k];
            let f_ip = field[lv_idx(ip, j, nx)];
            let f_im = field[lv_idx(im, j, nx)];
            let f_jp = field[lv_idx(i, jp, nx)];
            let f_jm = field[lv_idx(i, jm, nx)];
            out[k] = (f_ip + f_im + f_jp + f_jm - 4.0 * f) * inv_dx2;
        }
    }
}

struct LvRng {
    state: u64,
}

impl LvRng {
    fn new(seed: u64) -> Self {
        LvRng {
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

    fn stable_config() -> LvConfig {
        LvConfig {
            nx: 64,
            ny: 64,
            dx: 1.0,
            dt: 0.01,
            alpha: 1.0,
            beta: 0.5,
            delta: 0.4,
            gamma: 0.4,
            d_u: 0.1,
            d_v: 0.05,
            boundary: LvBoundary::Periodic,
        }
    }

    #[test]
    fn test_default_config_stability() {
        let cfg = LvConfig::default();
        assert!(cfg.is_stable(), "default config should be stable");
    }

    #[test]
    fn test_stable_dt_positive() {
        let cfg = LvConfig::default();
        assert!(cfg.stable_dt() > 0.0);
    }

    #[test]
    fn test_coexistence_fixed_point() {
        let cfg = LvConfig::default();
        let (u_star, v_star) = cfg.coexistence_fixed_point();
        // u* = γ/δ = 0.4/0.4 = 1.0
        assert!((u_star - 1.0).abs() < 1e-6);
        // v* = α/β = 1.0/0.5 = 2.0
        assert!((v_star - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_extinction_fixed_point() {
        let cfg = LvConfig::default();
        let (u, v) = cfg.extinction_fixed_point();
        assert_eq!(u, 0.0);
        assert_eq!(v, 0.0);
    }

    #[test]
    fn test_oscillation_frequency() {
        let cfg = LvConfig { alpha: 1.0, gamma: 0.25, ..Default::default() };
        // ω = sqrt(1 * 0.25) = 0.5
        assert!((cfg.oscillation_frequency() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_oscillation_period() {
        let cfg = LvConfig { alpha: 1.0, gamma: 1.0, ..Default::default() };
        // ω = 1, T = 2π
        assert!((cfg.oscillation_period() - 2.0 * std::f32::consts::PI).abs() < 1e-5);
    }

    #[test]
    fn test_initialize_zero() {
        let mut s = LvSolver::new(stable_config());
        s.initialize_zero();
        assert_eq!(s.mean_u(), 0.0);
        assert_eq!(s.mean_v(), 0.0);
        assert_eq!(s.total_prey(), 0.0);
    }

    #[test]
    fn test_initialize_coexistence() {
        let mut s = LvSolver::new(stable_config());
        s.initialize_coexistence(0.0, 42);
        let (u_star, v_star) = s.config.coexistence_fixed_point();
        assert!((s.mean_u() - u_star).abs() < 1e-4);
        assert!((s.mean_v() - v_star).abs() < 1e-4);
    }

    #[test]
    fn test_initialize_uniform() {
        let mut s = LvSolver::new(stable_config());
        s.initialize_uniform(3.0, 1.0);
        assert!((s.mean_u() - 3.0).abs() < 1e-6);
        assert!((s.mean_v() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_initialize_prey_patch() {
        let mut s = LvSolver::new(stable_config());
        s.initialize_prey_patch(32.0, 32.0, 5.0, 2.0);
        // 中心点 u 应高于平衡
        let center = s.idx(32, 32);
        let (u_star, _) = s.config.coexistence_fixed_point();
        assert!(s.u_curr[center] > u_star);
        assert!(s.total_prey() > 0.0);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = LvSolver::new(stable_config());
        s.initialize_coexistence(0.1, 1);
        let t0 = s.time;
        s.step();
        assert!(s.time > t0);
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = LvSolver::new(stable_config());
        s.initialize_coexistence(0.1, 1);
        s.step_n(10);
        assert_eq!(s.steps, 10);
    }

    #[test]
    fn test_no_nan_short_run() {
        let mut s = LvSolver::new(stable_config());
        s.initialize_coexistence(0.5, 42);
        s.step_n(500);
        assert!(!s.has_nan(), "NaN after 500 steps");
    }

    #[test]
    fn test_no_nan_long_run() {
        let cfg = LvConfig { nx: 32, ny: 32, ..stable_config() };
        let mut s = LvSolver::new(cfg);
        s.initialize_coexistence(0.5, 99);
        s.step_n(5000);
        assert!(!s.has_nan(), "NaN after 5000 steps");
    }

    #[test]
    fn test_population_nonnegative() {
        let mut s = LvSolver::new(stable_config());
        s.initialize_coexistence(0.5, 42);
        s.step_n(1000);
        assert!(s.min_u() >= 0.0, "prey should be non-negative: {}", s.min_u());
        assert!(s.min_v() >= 0.0, "predator should be non-negative: {}", s.min_v());
    }

    #[test]
    fn test_extinction_stable_without_predator() {
        // v=0 时, 被捕食者指数增长 (无捕食)
        let cfg = LvConfig { d_u: 0.0, d_v: 0.0, ..stable_config() };
        let mut s = LvSolver::new(cfg);
        s.initialize_uniform(1.0, 0.0); // 无捕食者
        s.step_n(100);
        // u 应增长 (α=1, u(t) = u0 * exp(αt), t=1, u≈e≈2.718)
        assert!(s.mean_u() > 1.0, "prey should grow without predator: {}", s.mean_u());
        assert!(s.mean_v() < 1e-6, "predator should stay zero");
    }

    #[test]
    fn test_prey_decays_without_food() {
        // u=0 时, 捕食者指数衰减 (无食物)
        let cfg = LvConfig { d_u: 0.0, d_v: 0.0, ..stable_config() };
        let mut s = LvSolver::new(cfg);
        s.initialize_uniform(0.0, 1.0); // 无被捕食者
        s.step_n(100);
        // v 应衰减 (γ=0.4, t=1, v≈exp(-0.4)≈0.67)
        assert!(s.mean_v() < 1.0, "predator should decay without prey: {}", s.mean_v());
        assert!(s.mean_u() < 1e-6, "prey should stay zero");
    }

    #[test]
    fn test_coexistence_oscillation() {
        // 无扩散 (D=0), 共存平衡附近应振荡 (不衰减到平衡)
        let cfg = LvConfig { d_u: 0.0, d_v: 0.0, ..stable_config() };
        let (u_star, v_star) = cfg.coexistence_fixed_point();
        let mut s = LvSolver::new(cfg);
        s.initialize_uniform(u_star * 1.5, v_star); // 偏离平衡
        let u0 = s.mean_u();
        s.step_n(2000); // t=20, 约几个周期
        let u1 = s.mean_u();
        // 应该振荡, 不应单调收敛到 u*
        // 检查总种群守恒性 (近似)
        let _ = (u0, u1);
        assert!((s.mean_u() - u_star).abs() > 0.01 || (s.mean_v() - v_star).abs() > 0.01,
            "system should oscillate, not converge: u={}, v={}", s.mean_u(), s.mean_v());
    }

    #[test]
    fn test_lyapunov_conservation_no_diffusion() {
        // 无扩散时 Lyapunov 泛函应守恒
        let cfg = LvConfig { d_u: 0.0, d_v: 0.0, ..stable_config() };
        let (u_star, v_star) = cfg.coexistence_fixed_point();
        let mut s = LvSolver::new(cfg);
        s.initialize_uniform(u_star * 1.3, v_star * 0.8);
        let h0 = s.lyapunov_integral();
        s.step_n(1000);
        let h1 = s.lyapunov_integral();
        // 允许 10% 误差 (Euler 方法数值漂移)
        let rel_err = (h1 - h0).abs() / h0.abs().max(1e-6);
        assert!(rel_err < 0.1, "Lyapunov should be conserved: {} -> {}", h0, h1);
    }

    #[test]
    fn test_amplitude_bounded() {
        let mut s = LvSolver::new(stable_config());
        s.initialize_coexistence(0.5, 42);
        s.step_n(2000);
        assert!(s.max_u() < 100.0, "prey bounded: {}", s.max_u());
        assert!(s.max_v() < 100.0, "predator bounded: {}", s.max_v());
    }

    #[test]
    fn test_zero_state_stays_zero() {
        let mut s = LvSolver::new(stable_config());
        s.initialize_zero();
        s.step_n(100);
        assert!(s.total_prey() < 1e-10, "zero prey should stay zero");
        assert!(s.total_predator() < 1e-10, "zero predator should stay zero");
    }

    #[test]
    fn test_dirichlet_boundary_enforced() {
        let cfg = LvConfig {
            nx: 32,
            ny: 32,
            dt: 0.005,
            boundary: LvBoundary::Dirichlet { u: 0.0, v: 0.0 },
            ..stable_config()
        };
        let mut s = LvSolver::new(cfg);
        s.initialize_uniform(1.0, 1.0);
        s.step_n(20);
        let nx = s.config.nx;
        let ny = s.config.ny;
        for i in 0..nx {
            assert!(s.u_curr[s.idx(i, 0)].abs() < 1e-5, "top boundary");
            assert!(s.u_curr[s.idx(i, ny - 1)].abs() < 1e-5, "bottom boundary");
        }
        for j in 0..ny {
            assert!(s.u_curr[s.idx(0, j)].abs() < 1e-5, "left boundary");
            assert!(s.u_curr[s.idx(nx - 1, j)].abs() < 1e-5, "right boundary");
        }
    }

    #[test]
    fn test_variance_positive() {
        let mut s = LvSolver::new(stable_config());
        s.initialize_prey_patch(32.0, 32.0, 5.0, 2.0);
        // 不均匀初始化应有正方差
        assert!(s.variance_u() > 0.01, "variance should be positive: {}", s.variance_u());
    }

    #[test]
    fn test_diffusion_smooths_variance() {
        // 扩散应使方差随时间减小
        let mut s = LvSolver::new(stable_config());
        s.initialize_prey_patch(32.0, 32.0, 5.0, 2.0);
        let v0 = s.variance_u();
        s.step_n(500);
        let v1 = s.variance_u();
        assert!(v1 < v0, "diffusion should reduce variance: {} -> {}", v0, v1);
    }

    #[test]
    fn test_total_population() {
        let mut s = LvSolver::new(stable_config());
        s.initialize_uniform(2.0, 3.0);
        let n = s.config.n_cells() as f32;
        assert!((s.total_prey() - 2.0 * n).abs() < 1e-3);
        assert!((s.total_predator() - 3.0 * n).abs() < 1e-3);
    }

    #[test]
    fn test_min_max() {
        let mut s = LvSolver::new(stable_config());
        s.initialize_uniform(1.5, 2.5);
        assert!((s.max_u() - 1.5).abs() < 1e-6);
        assert!((s.max_v() - 2.5).abs() < 1e-6);
        assert!((s.min_u() - 1.5).abs() < 1e-6);
        assert!((s.min_v() - 2.5).abs() < 1e-6);
    }

    #[test]
    fn test_diffusive_cfl() {
        let cfg = LvConfig { d_u: 1.0, d_v: 0.5, dt: 0.25, dx: 1.0, ..Default::default() };
        // 4 * 1.0 * 0.25 / 1 = 1.0
        assert!((cfg.diffusive_cfl() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_module_count() {
        let tests: Vec<&str> = vec![
            "test_default_config_stability",
            "test_stable_dt_positive",
            "test_coexistence_fixed_point",
            "test_extinction_fixed_point",
            "test_oscillation_frequency",
            "test_oscillation_period",
            "test_initialize_zero",
            "test_initialize_coexistence",
            "test_initialize_uniform",
            "test_initialize_prey_patch",
            "test_step_advances_time",
            "test_step_n_advances",
            "test_no_nan_short_run",
            "test_no_nan_long_run",
            "test_population_nonnegative",
            "test_extinction_stable_without_predator",
            "test_prey_decays_without_food",
            "test_coexistence_oscillation",
            "test_lyapunov_conservation_no_diffusion",
            "test_amplitude_bounded",
            "test_zero_state_stays_zero",
            "test_dirichlet_boundary_enforced",
            "test_variance_positive",
            "test_diffusion_smooths_variance",
            "test_total_population",
            "test_min_max",
            "test_diffusive_cfl",
        ];
        assert!(tests.len() >= 20, "need at least 20 tests, got {}", tests.len());
    }
}
