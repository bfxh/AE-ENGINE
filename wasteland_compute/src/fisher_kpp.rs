//! Fisher-KPP Equation Solver (Ecological/Epidemiological Spread)
//!
//! 反应扩散方程, 描述种群入侵、传染病传播、肿瘤生长.
//! Fisher 1937, Kolmogorov-Petrovsky-Piskounov 1937 独立提出.
//!
//! 方程:
//!   du/dt = D * laplacian(u) + r * u * (1 - u/K)
//!
//! 其中:
//!   u = 种群密度 (u >= 0)
//!   D = 扩散系数 (随机游走)
//!   r = 线性增长率
//!   K = 环境容量 (carrying capacity)
//!
//! 行波解:
//!   最小波速 c* = 2 * sqrt(r * D)
//!   行波连接 u=K (波前) 和 u=0 (波后)
//!   任何 c >= c* 存在行波解, c < c* 不存在
//!   精确解 (Fisher): u = K / (1 + C*exp(-sqrt(r/(6D))*(x-c*t)))^2, c = 5*sqrt(rD/6)
//!
//! 应用:
//!   - 入侵物种扩散 (生态学, Skellam 1951)
//!   - 疫情传播 (流行病学)
//!   - 肿瘤生长 (医学)
//!   - 谣言/信息传播
//!   - 农业技术扩散
//!
//! 数值方法: 显式 Euler + 5点 Laplacian (2D)
//!   u^{n+1} = u + dt * (D * lap_u + r * u * (1 - u/K))
//!
//! CFL 稳定性:
//!   扩散: 4*D*dt/dx^2 <= 1  (2D 5点模板)
//!   反应: r*dt <= 2  (u=K 处特征值 -r, 显式 Euler 稳定条件 |1-r*dt|<=1)
//!   综合: dt <= min(dx^2/(4*D), 2/r)
//!
//! 基于 Fisher 1937, Kolmogorov et al. 1937,
//! Skellam 1951, Murray 2002 (Mathematical Biology).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FkBoundary {
    Periodic,
    Neumann,
    Dirichlet { value: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FisherKppConfig {
    pub nx: usize,
    pub ny: usize,
    pub dx: f32,
    pub dt: f32,
    pub d: f32,
    pub r: f32,
    pub k: f32,
    pub boundary: FkBoundary,
}

impl Default for FisherKppConfig {
    fn default() -> Self {
        // diffusive CFL = 4*1*0.05/1 = 0.2 (stable)
        // reaction CFL = 1*0.05 = 0.05 (stable)
        FisherKppConfig {
            nx: 128,
            ny: 128,
            dx: 1.0,
            dt: 0.05,
            d: 1.0,
            r: 1.0,
            k: 1.0,
            boundary: FkBoundary::Periodic,
        }
    }
}

fn wrap_idx(i: i32, n: usize) -> usize {
    let m = n as i32;
    (((i % m) + m) % m) as usize
}

impl FisherKppConfig {
    pub fn n_cells(&self) -> usize {
        self.nx * self.ny
    }
    pub fn domain_area(&self) -> f32 {
        (self.nx as f32) * self.dx * (self.ny as f32) * self.dx
    }
    /// 扩散 CFL: 4*D*dt/dx^2 (2D 5点)
    pub fn diffusive_cfl(&self) -> f32 {
        4.0 * self.d * self.dt / (self.dx * self.dx)
    }
    /// 反应 CFL: r*dt
    pub fn reaction_cfl(&self) -> f32 {
        self.r * self.dt
    }
    pub fn is_stable(&self) -> bool {
        self.diffusive_cfl() <= 1.0 && self.reaction_cfl() <= 2.0
    }
    pub fn stable_dt(&self) -> f32 {
        let diff_dt = self.dx * self.dx / (4.0 * self.d);
        let rxn_dt = 2.0 / self.r;
        diff_dt.min(rxn_dt)
    }
    /// 行波最小波速 c* = 2*sqrt(r*D)
    pub fn wave_speed(&self) -> f32 {
        2.0 * (self.r * self.d).sqrt()
    }
}

pub struct FisherKppSolver {
    pub config: FisherKppConfig,
    pub u_curr: Vec<f32>,
    pub u_next: Vec<f32>,
    pub time: f32,
    pub steps: usize,
}

impl FisherKppSolver {
    pub fn new(config: FisherKppConfig) -> Self {
        let n = config.n_cells();
        FisherKppSolver {
            config,
            u_curr: vec![0.0; n],
            u_next: vec![0.0; n],
            time: 0.0,
            steps: 0,
        }
    }

    pub fn idx(&self, i: usize, j: usize) -> usize {
        j * self.config.nx + i
    }

    /// 显式 Euler + 5点 Laplacian 一步
    pub fn step(&mut self) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        let dt = self.config.dt;
        let d = self.config.d;
        let r = self.config.r;
        let k = self.config.k;
        let bc = self.config.boundary;
        let inv_dx2 = 1.0 / (dx * dx);
        let inv_k = 1.0 / k;

        for j in 0..ny {
            for i in 0..nx {
                let idx = j * nx + i;
                let u = self.u_curr[idx];
                let (ip, im, jp, jm) = match bc {
                    FkBoundary::Periodic => (
                        wrap_idx(i as i32 + 1, nx),
                        wrap_idx(i as i32 - 1, nx),
                        wrap_idx(j as i32 + 1, ny),
                        wrap_idx(j as i32 - 1, ny),
                    ),
                    _ => (
                        (i + 1).min(nx - 1),
                        if i > 0 { i - 1 } else { 0 },
                        (j + 1).min(ny - 1),
                        if j > 0 { j - 1 } else { 0 },
                    ),
                };
                let u_ip = self.u_curr[j * nx + ip];
                let u_im = self.u_curr[j * nx + im];
                let u_jp = self.u_curr[jp * nx + i];
                let u_jm = self.u_curr[jm * nx + i];
                let lap_u = (u_ip + u_im + u_jp + u_jm - 4.0 * u) * inv_dx2;
                let reaction = r * u * (1.0 - u * inv_k);
                self.u_next[idx] = u + dt * (d * lap_u + reaction);
            }
        }

        // Dirichlet 边界
        if let FkBoundary::Dirichlet { value } = bc {
            for i in 0..nx {
                self.u_next[0 * nx + i] = value;
                self.u_next[(ny - 1) * nx + i] = value;
            }
            for j in 0..ny {
                self.u_next[j * nx + 0] = value;
                self.u_next[j * nx + (nx - 1)] = value;
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

    /// 零初始化 (灭绝态)
    pub fn initialize_empty(&mut self) {
        for u in self.u_curr.iter_mut() { *u = 0.0; }
    }

    /// 满载初始化 (u = K 处处)
    pub fn initialize_saturated(&mut self) {
        let k = self.config.k;
        for u in self.u_curr.iter_mut() { *u = k; }
    }

    /// 阶跃初始条件: u=K (x<x_front), u=0 (x>=x_front) — 形成行波
    pub fn initialize_step(&mut self, x_front: f32) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        let k = self.config.k;
        for j in 0..ny {
            for i in 0..nx {
                let x = (i as f32) * dx;
                self.u_curr[j * nx + i] = if x < x_front { k } else { 0.0 };
            }
        }
    }

    /// 高斯种子: u = K * exp(-r^2/w^2)  (局部种群爆发)
    pub fn initialize_gaussian_seed(&mut self, cx: f32, cy: f32, width: f32) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        let k = self.config.k;
        let inv_w2 = 1.0 / (width * width);
        for j in 0..ny {
            for i in 0..nx {
                let x = (i as f32) * dx;
                let y = (j as f32) * dx;
                let dr2 = (x - cx) * (x - cx) + (y - cy) * (y - cy);
                self.u_curr[j * nx + i] = k * (-dr2 * inv_w2).exp();
            }
        }
    }

    /// 总种群量 integral u dx
    pub fn total_population(&self) -> f32 {
        self.u_curr.iter().sum::<f32>() * self.config.dx * self.config.dx
    }

    /// 平均密度
    pub fn mean_density(&self) -> f32 {
        self.u_curr.iter().sum::<f32>() / self.u_curr.len() as f32
    }

    pub fn max_density(&self) -> f32 {
        self.u_curr.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
    }

    pub fn min_density(&self) -> f32 {
        self.u_curr.iter().cloned().fold(f32::INFINITY, f32::min)
    }

    pub fn has_nan(&self) -> bool {
        self.u_curr.iter().any(|&u| u.is_nan() || u.is_infinite())
    }

    /// 寻找波前位置 (最大 |du/dx| 的列)
    pub fn wave_front_position(&self) -> usize {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let bc = self.config.boundary;
        let mut front = 0usize;
        let mut max_grad = 0.0f32;
        for i in 0..nx {
            let ip = match bc {
                FkBoundary::Periodic => wrap_idx(i as i32 + 1, nx),
                _ => (i + 1).min(nx - 1),
            };
            let mut grad_sum = 0.0f32;
            for j in 0..ny {
                let u_i = self.u_curr[j * nx + i];
                let u_ip = self.u_curr[j * nx + ip];
                grad_sum += (u_ip - u_i).abs();
            }
            if grad_sum > max_grad {
                max_grad = grad_sum;
                front = i;
            }
        }
        front
    }

    pub fn reset(&mut self) {
        for u in self.u_curr.iter_mut() { *u = 0.0; }
        for u in self.u_next.iter_mut() { *u = 0.0; }
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

    #[test]
    fn test_config_default() {
        let c = FisherKppConfig::default();
        assert_eq!(c.nx, 128);
        assert_eq!(c.ny, 128);
        assert_eq!(c.dx, 1.0);
        assert_eq!(c.dt, 0.05);
        assert_eq!(c.d, 1.0);
        assert_eq!(c.r, 1.0);
        assert_eq!(c.k, 1.0);
        assert_eq!(c.boundary, FkBoundary::Periodic);
    }

    #[test]
    fn test_n_cells() {
        let c = FisherKppConfig { nx: 64, ny: 32, ..Default::default() };
        assert_eq!(c.n_cells(), 64 * 32);
    }

    #[test]
    fn test_domain_area() {
        let c = FisherKppConfig { nx: 10, ny: 20, dx: 0.5, ..Default::default() };
        assert!(approx_eq(c.domain_area(), 10.0 * 0.5 * 20.0 * 0.5, 1e-6));
    }

    #[test]
    fn test_diffusive_cfl() {
        let c = FisherKppConfig { nx: 64, dx: 1.0, dt: 0.05, d: 1.0, ..Default::default() };
        // 4*1*0.05/1 = 0.2
        assert!(approx_eq(c.diffusive_cfl(), 0.2, 1e-6));
    }

    #[test]
    fn test_reaction_cfl() {
        let c = FisherKppConfig { r: 1.0, dt: 0.05, ..Default::default() };
        // 1*0.05 = 0.05
        assert!(approx_eq(c.reaction_cfl(), 0.05, 1e-6));
    }

    #[test]
    fn test_is_stable_default() {
        let c = FisherKppConfig::default();
        assert!(c.is_stable());
    }

    #[test]
    fn test_is_stable_unstable_diffusion() {
        let c = FisherKppConfig { dt: 0.3, d: 1.0, dx: 1.0, ..Default::default() };
        // 4*1*0.3/1 = 1.2 > 1
        assert!(!c.is_stable());
    }

    #[test]
    fn test_is_stable_unstable_reaction() {
        let c = FisherKppConfig { r: 10.0, dt: 0.3, ..Default::default() };
        // 10*0.3 = 3 > 2
        assert!(!c.is_stable());
    }

    #[test]
    fn test_stable_dt() {
        let c = FisherKppConfig { dx: 1.0, d: 1.0, r: 1.0, ..Default::default() };
        // min(1/(4*1), 2/1) = min(0.25, 2.0) = 0.25
        assert!(approx_eq(c.stable_dt(), 0.25, 1e-6));
    }

    #[test]
    fn test_wave_speed() {
        let c = FisherKppConfig { r: 1.0, d: 1.0, ..Default::default() };
        // c* = 2*sqrt(1*1) = 2.0
        assert!(approx_eq(c.wave_speed(), 2.0, 1e-6));
    }

    #[test]
    fn test_wave_speed_scaled() {
        let c = FisherKppConfig { r: 4.0, d: 0.25, ..Default::default() };
        // c* = 2*sqrt(4*0.25) = 2*sqrt(1) = 2.0
        assert!(approx_eq(c.wave_speed(), 2.0, 1e-6));
    }

    #[test]
    fn test_solver_new() {
        let s = FisherKppSolver::new(FisherKppConfig { nx: 32, ny: 16, ..Default::default() });
        assert_eq!(s.u_curr.len(), 32 * 16);
        assert_eq!(s.u_next.len(), 32 * 16);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
        for &u in s.u_curr.iter() { assert_eq!(u, 0.0); }
    }

    #[test]
    fn test_idx() {
        let s = FisherKppSolver::new(FisherKppConfig { nx: 10, ny: 5, ..Default::default() });
        assert_eq!(s.idx(3, 2), 2 * 10 + 3);
        assert_eq!(s.idx(0, 0), 0);
        assert_eq!(s.idx(9, 4), 4 * 10 + 9);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = FisherKppSolver::new(FisherKppConfig { nx: 16, ny: 16, dt: 0.01, ..Default::default() });
        s.step();
        assert!(approx_eq(s.time, 0.01, 1e-9));
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = FisherKppSolver::new(FisherKppConfig { nx: 16, ny: 16, dt: 0.01, ..Default::default() });
        s.step_n(50);
        assert_eq!(s.steps, 50);
        assert!(approx_eq(s.time, 0.5, 1e-6));
    }

    #[test]
    fn test_empty_stays_empty() {
        // u=0 is a stable equilibrium (no spontaneous generation)
        let mut s = FisherKppSolver::new(FisherKppConfig { nx: 16, ny: 16, ..Default::default() });
        s.initialize_empty();
        s.step_n(100);
        let max_u = s.max_density();
        assert!(max_u < 1e-6, "empty state grew: {}", max_u);
    }

    #[test]
    fn test_saturated_stays_saturated() {
        // u=K is a stable equilibrium
        let mut s = FisherKppSolver::new(FisherKppConfig { nx: 16, ny: 16, ..Default::default() });
        s.initialize_saturated();
        s.step_n(100);
        let max_dev = s.u_curr.iter().map(|&u| (u - 1.0).abs()).fold(0.0f32, f32::max);
        assert!(max_dev < 1e-4, "saturated state drifted: {}", max_dev);
    }

    #[test]
    fn test_logistic_growth_no_diffusion() {
        // Without diffusion (D=0), pure logistic growth: u -> K
        let mut s = FisherKppSolver::new(FisherKppConfig {
            nx: 4, ny: 4, dx: 1.0, dt: 0.01, d: 0.0, r: 1.0, k: 1.0,
            boundary: FkBoundary::Periodic,
        });
        // Initial: u=0.1 everywhere
        for u in s.u_curr.iter_mut() { *u = 0.1; }
        s.step_n(1000); // t=10.0, u(t)=K/(1+(K/u0-1)*e^{-rt})=1/(1+9*e^{-10})~0.9996
        let mean = s.mean_density();
        assert!(mean > 0.95, "logistic did not saturate: mean={}", mean);
    }

    #[test]
    fn test_logistic_growth_preserves_uniform() {
        // Uniform initial condition stays uniform (no spatial variation)
        let mut s = FisherKppSolver::new(FisherKppConfig {
            nx: 16, ny: 16, dx: 1.0, dt: 0.05, d: 1.0, r: 1.0, k: 1.0,
            boundary: FkBoundary::Periodic,
        });
        for u in s.u_curr.iter_mut() { *u = 0.3; }
        s.step_n(100);
        let max_u = s.max_density();
        let min_u = s.min_density();
        assert!(approx_eq(max_u, min_u, 1e-6), "uniform broke: [{}, {}]", min_u, max_u);
    }

    #[test]
    fn test_wave_propagates() {
        // Step initial condition -> traveling wave
        let mut s = FisherKppSolver::new(FisherKppConfig {
            nx: 256, ny: 4, dx: 1.0, dt: 0.05, d: 1.0, r: 1.0, k: 1.0,
            boundary: FkBoundary::Periodic,
        });
        s.initialize_step(32.0);
        let front0 = s.wave_front_position();
        s.step_n(400); // t=20, wave speed c*=2, distance ~40
        let front1 = s.wave_front_position();
        let dx_moved = (front1 as f32 - front0 as f32) * 1.0;
        let expected = 2.0 * 20.0; // c* * t = 40
        assert!(dx_moved > expected * 0.5,
            "wave did not propagate: moved {} expected ~{}", dx_moved, expected);
    }

    #[test]
    fn test_gaussian_seed_spreads() {
        let mut s = FisherKppSolver::new(FisherKppConfig {
            nx: 64, ny: 64, dx: 1.0, dt: 0.05, d: 1.0, r: 1.0, k: 1.0,
            boundary: FkBoundary::Neumann,
        });
        s.initialize_gaussian_seed(32.0, 32.0, 4.0);
        let pop0 = s.total_population();
        s.step_n(200);
        let pop1 = s.total_population();
        // Population should grow (logistic) and spread (diffusion)
        assert!(pop1 > pop0, "population did not grow: {} -> {}", pop0, pop1);
        // Should not exceed carrying capacity * area
        let max_possible = s.config.k * (s.config.nx as f32) * (s.config.ny as f32);
        assert!(pop1 < max_possible * 1.1, "population exceeded capacity: {}", pop1);
    }

    #[test]
    fn test_no_nan_long_run() {
        let mut s = FisherKppSolver::new(FisherKppConfig::default());
        s.initialize_gaussian_seed(64.0, 64.0, 4.0);
        s.step_n(1000);
        assert!(!s.has_nan(), "NaN detected after long run");
    }

    #[test]
    fn test_density_bounded() {
        let mut s = FisherKppSolver::new(FisherKppConfig::default());
        s.initialize_gaussian_seed(64.0, 64.0, 4.0);
        s.step_n(1000);
        let max_u = s.max_density();
        let min_u = s.min_density();
        // u should stay in [0, K*1.1] approximately
        assert!(max_u < 1.2, "u runaway positive: {}", max_u);
        assert!(min_u >= -0.1, "u went negative: {}", min_u);
    }

    #[test]
    fn test_periodic_boundary_no_nan() {
        let mut s = FisherKppSolver::new(FisherKppConfig {
            nx: 32, ny: 32, dx: 1.0, dt: 0.05, d: 1.0, r: 1.0, k: 1.0,
            boundary: FkBoundary::Periodic,
        });
        s.initialize_step(16.0);
        s.step_n(200);
        assert!(!s.has_nan(), "NaN with periodic boundary");
    }

    #[test]
    fn test_neumann_boundary_no_nan() {
        let mut s = FisherKppSolver::new(FisherKppConfig {
            nx: 32, ny: 32, dx: 1.0, dt: 0.05, d: 1.0, r: 1.0, k: 1.0,
            boundary: FkBoundary::Neumann,
        });
        s.initialize_step(16.0);
        s.step_n(200);
        assert!(!s.has_nan(), "NaN with Neumann boundary");
    }

    #[test]
    fn test_dirichlet_boundary() {
        let mut s = FisherKppSolver::new(FisherKppConfig {
            nx: 32, ny: 32, dx: 1.0, dt: 0.05, d: 1.0, r: 1.0, k: 1.0,
            boundary: FkBoundary::Dirichlet { value: 0.0 },
        });
        s.initialize_saturated();
        s.step_n(50);
        // Boundaries should be 0
        let corner = s.u_curr[0];
        assert!(approx_eq(corner, 0.0, 1e-6), "Dirichlet boundary not enforced: {}", corner);
    }

    #[test]
    fn test_reset() {
        let mut s = FisherKppSolver::new(FisherKppConfig { nx: 16, ny: 16, ..Default::default() });
        s.initialize_saturated();
        s.step_n(50);
        s.reset();
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
        for &u in s.u_curr.iter() { assert_eq!(u, 0.0); }
    }

    #[test]
    fn test_wave_front_advances() {
        // Verify wave front moves rightward over time
        let mut s = FisherKppSolver::new(FisherKppConfig {
            nx: 128, ny: 4, dx: 1.0, dt: 0.05, d: 1.0, r: 1.0, k: 1.0,
            boundary: FkBoundary::Periodic,
        });
        s.initialize_step(16.0);
        let f0 = s.wave_front_position();
        s.step_n(100); // t=5
        let f1 = s.wave_front_position();
        s.step_n(100); // t=10
        let f2 = s.wave_front_position();
        assert!(f1 > f0, "wave did not advance: {} -> {}", f0, f1);
        assert!(f2 > f1, "wave did not continue: {} -> {}", f1, f2);
    }
}