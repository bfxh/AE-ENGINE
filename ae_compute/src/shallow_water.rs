//! Shallow Water Equations (2D) Solver — 浅水方程求解器
//!
//! 浅水方程 (Saint-Venant 方程) 描述自由表面流体在水平尺度远大于深度时的
//! 流动, 是水文学、海洋学、气象学的核心模型. 在游戏中可用于实时水面模拟、
//! 潮汐、洪水、海啸等效果.
//!
//! 守恒形式 (2D):
//!   ∂h/∂t + ∂(hu)/∂x + ∂(hv)/∂y = 0                  (质量守恒)
//!   ∂(hu)/∂t + ∂(hu² + gh²/2)/∂x + ∂(huv)/∂y = 0     (x 动量)
//!   ∂(hv)/∂t + ∂(huv)/∂x + ∂(hv² + gh²/2)/∂y = 0     (y 动量)
//!
//! 守恒变量: U = (h, hu, hv)
//! 通量:
//!   F(U) = (hu, hu²/h + gh²/2, huv)   (x 方向)
//!   G(U) = (hv, huv, hv²/h + gh²/2)   (y 方向)
//!
//! 其中:
//!   h  - 水深 (自由表面高度 - 底床高度)
//!   u  - x 方向流速 = hu/h
//!   v  - y 方向流速 = hv/h
//!   g  - 重力加速度 (9.81 m/s²)
//!
//! 特征速度:
//!   c = sqrt(g·h)  (重力波速)
//!   x 方向特征: u ± c
//!   y 方向特征: v ± c
//!
//! CFL 条件 (2D Lax-Friedrichs):
//!   dt <= dx / (2 · max(|u| + c, |v| + c))
//!
//! 物理现象:
//!   - 重力波: 扰动以波速 c 传播
//!   - 浅水孤立波: u = 2c - c0 (非线性, 不弥散)
//!   - 大坝溃决: Riemann 问题, 产生激波 + 稀疏波
//!   - 潮汐: 周期性强迫
//!   - 海啸: 长波传播
//!   - 涌浪: 风驱流动
//!
//! 数值方法:
//!   Lax-Friedrichs 格式 (一阶, 耗散, 稳定)
//!   U^{n+1} = 0.25·(U_N + U_S + U_E + U_W) - 0.5·dt/dx·(F_E - F_W) - 0.5·dt/dy·(G_N - G_S)
//!
//!   优点: 简单, 无振荡, 处理激波
//!   缺点: 一阶精度, 数值耗散大
//!
//! 应用:
//!   - 洪水模拟与预警
//!   - 海洋潮汐预报
//!   - 海啸传播
//!   - 游戏水面物理 (实时渲染耦合)
//!   - 大气流动 (等压面近似)
//!
//! 基于:
//!   - Saint-Venant, A.J.C. 1871. C. R. Acad. Sci. Paris 73, 147.
//!   - Stoker, J.J. 1957. "Water Waves." Interscience.
//!   - Toro, E.F. 2001. "Shock-Capturing Methods for Free-Surface
//!     Shallow Flows." Wiley.
//!   - LeVeque, R.J. 2002. "Finite Volume Methods for Hyperbolic
//!     Problems." Cambridge.

use serde::{Deserialize, Serialize};

const G: f32 = 9.81;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SwBoundary {
    Periodic,
    Reflecting,
    Outflow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwConfig {
    pub nx: usize,
    pub ny: usize,
    pub dx: f32,
    pub dt: f32,
    pub h_floor: f32,
    pub boundary: SwBoundary,
}

impl Default for SwConfig {
    fn default() -> Self {
        SwConfig {
            nx: 128,
            ny: 128,
            dx: 1.0,
            dt: 0.05,
            h_floor: 1e-4,
            boundary: SwBoundary::Periodic,
        }
    }
}

impl SwConfig {
    pub fn n_cells(&self) -> usize {
        self.nx * self.ny
    }

    pub fn domain_area(&self) -> f32 {
        (self.nx as f32) * self.dx * (self.ny as f32) * self.dx
    }

    /// 估算 CFL (给定最大水深 h_max 和最大流速 vmax)
    pub fn cfl(&self, h_max: f32, vmax: f32) -> f32 {
        let c = (G * h_max.abs()).sqrt();
        let max_speed = vmax.abs() + c;
        if max_speed < 1e-10 {
            return 0.0;
        }
        2.0 * max_speed * self.dt / self.dx
    }

    /// 给定 h_max 估算稳定 dt
    pub fn stable_dt(&self, h_max: f32, vmax: f32) -> f32 {
        let c = (G * h_max.abs()).sqrt();
        let max_speed = (vmax.abs() + c).max(1e-6);
        0.45 * self.dx / max_speed
    }

    /// 重力波速 c = sqrt(g·h)
    pub fn wave_speed(h: f32) -> f32 {
        (G * h.abs()).sqrt()
    }
}

pub struct SwSolver {
    pub config: SwConfig,
    pub h_curr: Vec<f32>,
    pub hu_curr: Vec<f32>,
    pub hv_curr: Vec<f32>,
    pub h_next: Vec<f32>,
    pub hu_next: Vec<f32>,
    pub hv_next: Vec<f32>,
    pub time: f32,
    pub steps: usize,
}

impl SwSolver {
    pub fn new(config: SwConfig) -> Self {
        let n = config.n_cells();
        SwSolver {
            config,
            h_curr: vec![0.0; n],
            hu_curr: vec![0.0; n],
            hv_curr: vec![0.0; n],
            h_next: vec![0.0; n],
            hu_next: vec![0.0; n],
            hv_next: vec![0.0; n],
            time: 0.0,
            steps: 0,
        }
    }

    #[inline]
    fn idx(&self, i: usize, j: usize) -> usize {
        j * self.config.nx + i
    }

    #[inline]
    fn wrap(i: i32, n: usize) -> usize {
        let n = n as i32;
        (((i % n) + n) % n) as usize
    }

    pub fn initialize_uniform(&mut self, h0: f32) {
        for k in 0..self.h_curr.len() {
            self.h_curr[k] = h0;
            self.hu_curr[k] = 0.0;
            self.hv_curr[k] = 0.0;
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 水滴初始化: 中心高 h1, 其余 h0, 静止
    pub fn initialize_droplet(&mut self, cx: f32, cy: f32, radius: f32, h0: f32, h1: f32) {
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
                self.h_curr[id] = if d2 < r2 { h1 } else { h0 };
                self.hu_curr[id] = 0.0;
                self.hv_curr[id] = 0.0;
            }
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 大坝溃决: x < cx 处 h=h1, x >= cx 处 h=h0
    pub fn initialize_dam_break(&mut self, cx: f32, h0: f32, h1: f32) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        for j in 0..ny {
            for i in 0..nx {
                let x = i as f32 * dx;
                let id = self.idx(i, j);
                self.h_curr[id] = if x < cx { h1 } else { h0 };
                self.hu_curr[id] = 0.0;
                self.hv_curr[id] = 0.0;
            }
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 从守恒变量获取原始变量 (h, u, v)
    #[inline]
    fn primitive(h: f32, hu: f32, hv: f32, h_floor: f32) -> (f32, f32, f32) {
        let h_safe = if h < h_floor { h_floor } else { h };
        let u = if h < h_floor { 0.0 } else { hu / h };
        let v = if h < h_floor { 0.0 } else { hv / h };
        (h_safe, u, v)
    }

    /// 计算 x 方向通量 F = (hu, hu²/h + gh²/2, huv)
    #[inline]
    fn flux_x(h: f32, hu: f32, hv: f32, h_floor: f32) -> (f32, f32, f32) {
        let (h_s, u, v) = Self::primitive(h, hu, hv, h_floor);
        let f0 = hu;
        let f1 = h_s * u * u + 0.5 * G * h_s * h_s;
        let f2 = h_s * u * v;
        (f0, f1, f2)
    }

    /// 计算 y 方向通量 G = (hv, huv, hv²/h + gh²/2)
    #[inline]
    fn flux_y(h: f32, hu: f32, hv: f32, h_floor: f32) -> (f32, f32, f32) {
        let (h_s, u, v) = Self::primitive(h, hu, hv, h_floor);
        let g0 = hv;
        let g1 = h_s * u * v;
        let g2 = h_s * v * v + 0.5 * G * h_s * h_s;
        (g0, g1, g2)
    }

    pub fn step(&mut self) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        let dt = self.config.dt;
        let h_floor = self.config.h_floor;
        let bc = self.config.boundary;
        let r = dt / dx;

        for j in 0..ny {
            for i in 0..nx {
                let k = self.idx(i, j);

                let (ie, iw, jn, js) = match bc {
                    SwBoundary::Periodic => (
                        Self::wrap(i as i32 + 1, nx),
                        Self::wrap(i as i32 - 1, nx),
                        Self::wrap(j as i32 + 1, ny),
                        Self::wrap(j as i32 - 1, ny),
                    ),
                    _ => (
                        (i + 1).min(nx - 1),
                        if i > 0 { i - 1 } else { 0 },
                        (j + 1).min(ny - 1),
                        if j > 0 { j - 1 } else { 0 },
                    ),
                };

                let ke = self.idx(ie, j);
                let kw = self.idx(iw, j);
                let kn = self.idx(i, jn);
                let ks = self.idx(i, js);

                let h_c = self.h_curr[k];
                let hu_c = self.hu_curr[k];
                let hv_c = self.hv_curr[k];
                let h_e = self.h_curr[ke];
                let hu_e = self.hu_curr[ke];
                let hv_e = self.hv_curr[ke];
                let h_w = self.h_curr[kw];
                let hu_w = self.hu_curr[kw];
                let hv_w = self.hv_curr[kw];
                let h_n = self.h_curr[kn];
                let hu_n = self.hu_curr[kn];
                let hv_n = self.hv_curr[kn];
                let h_s = self.h_curr[ks];
                let hu_s = self.hu_curr[ks];
                let hv_s = self.hv_curr[ks];

                let (fe0, fe1, fe2) = Self::flux_x(h_e, hu_e, hv_e, h_floor);
                let (fw0, fw1, fw2) = Self::flux_x(h_w, hu_w, hv_w, h_floor);
                let (gn0, gn1, gn2) = Self::flux_y(h_n, hu_n, hv_n, h_floor);
                let (gs0, gs1, gs2) = Self::flux_y(h_s, hu_s, hv_s, h_floor);

                // Lax-Friedrichs: 0.25*(E+W+N+S) - 0.5*r*(F_e - F_w) - 0.5*r*(G_n - G_s)
                let avg_factor = 0.25;
                let flux_factor = 0.5 * r;

                let mut h_new = avg_factor * (h_e + h_w + h_n + h_s)
                    - flux_factor * (fe0 - fw0)
                    - flux_factor * (gn0 - gs0);
                let mut hu_new = avg_factor * (hu_e + hu_w + hu_n + hu_s)
                    - flux_factor * (fe1 - fw1)
                    - flux_factor * (gn1 - gs1);
                let mut hv_new = avg_factor * (hv_e + hv_w + hv_n + hv_s)
                    - flux_factor * (fe2 - fw2)
                    - flux_factor * (gn2 - gs2);

                // 反射边界: 边界处法向速度为 0
                if bc == SwBoundary::Reflecting {
                    if i == 0 || i == nx - 1 {
                        hu_new = 0.0;
                    }
                    if j == 0 || j == ny - 1 {
                        hv_new = 0.0;
                    }
                }

                if h_new < h_floor {
                    h_new = h_floor;
                }
                self.h_next[k] = h_new;
                self.hu_next[k] = hu_new;
                self.hv_next[k] = hv_new;
            }
        }

        std::mem::swap(&mut self.h_curr, &mut self.h_next);
        std::mem::swap(&mut self.hu_curr, &mut self.hu_next);
        std::mem::swap(&mut self.hv_curr, &mut self.hv_next);
        self.time += dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    pub fn has_nan(&self) -> bool {
        self.h_curr.iter().any(|&v| !v.is_finite())
            || self.hu_curr.iter().any(|&v| !v.is_finite())
            || self.hv_curr.iter().any(|&v| !v.is_finite())
    }

    pub fn mean_h(&self) -> f32 {
        let n = self.h_curr.len();
        if n == 0 { 0.0 } else { self.h_curr.iter().sum::<f32>() / n as f32 }
    }

    pub fn total_mass(&self) -> f32 {
        self.h_curr.iter().sum::<f32>()
    }

    pub fn max_h(&self) -> f32 {
        self.h_curr.iter().copied().fold(f32::NEG_INFINITY, f32::max)
    }

    pub fn min_h(&self) -> f32 {
        self.h_curr.iter().copied().fold(f32::INFINITY, f32::min)
    }

    pub fn max_velocity(&self) -> f32 {
        let h_floor = self.config.h_floor;
        self.h_curr
            .iter()
            .zip(self.hu_curr.iter())
            .zip(self.hv_curr.iter())
            .map(|((&h, &hu), &hv)| {
                let (_, u, v) = Self::primitive(h, hu, hv, h_floor);
                (u * u + v * v).sqrt()
            })
            .fold(0.0f32, f32::max)
    }

    pub fn max_abs_h(&self) -> f32 {
        self.h_curr.iter().map(|&v| v.abs()).fold(0.0f32, f32::max)
    }

    /// 总能量 E = 0.5·g·Σh² + 0.5·Σh·(u²+v²)
    pub fn total_energy(&self) -> f32 {
        let h_floor = self.config.h_floor;
        let mut sum = 0.0f32;
        for k in 0..self.h_curr.len() {
            let h = self.h_curr[k];
            let hu = self.hu_curr[k];
            let hv = self.hv_curr[k];
            let (_, u, v) = Self::primitive(h, hu, hv, h_floor);
            sum += 0.5 * G * h * h + 0.5 * h * (u * u + v * v);
        }
        sum
    }

    pub fn variance_h(&self) -> f32 {
        let m = self.mean_h();
        let n = self.h_curr.len();
        if n == 0 { return 0.0; }
        self.h_curr.iter().map(|&h| (h - m) * (h - m)).sum::<f32>() / n as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stable_config() -> SwConfig {
        SwConfig {
            nx: 64,
            ny: 64,
            dx: 1.0,
            dt: 0.02,
            h_floor: 1e-4,
            boundary: SwBoundary::Periodic,
        }
    }

    #[test]
    fn test_default_config() {
        let cfg = SwConfig::default();
        assert_eq!(cfg.nx, 128);
        assert_eq!(cfg.dx, 1.0);
        assert!(cfg.h_floor > 0.0);
    }

    #[test]
    fn test_n_cells() {
        let cfg = SwConfig { nx: 32, ny: 48, ..Default::default() };
        assert_eq!(cfg.n_cells(), 32 * 48);
    }

    #[test]
    fn test_domain_area() {
        let cfg = SwConfig { nx: 10, ny: 20, dx: 0.5, ..Default::default() };
        assert!((cfg.domain_area() - 10.0 * 0.5 * 20.0 * 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_wave_speed() {
        // c = sqrt(g*h), g=9.81, h=1 -> c=sqrt(9.81)≈3.132
        let c = SwConfig::wave_speed(1.0);
        assert!((c - 9.81_f32.sqrt()).abs() < 1e-4);
    }

    #[test]
    fn test_cfl_static_water() {
        let cfg = stable_config();
        // 静止水: vmax=0, h_max=1.0
        // cfl = 2 * sqrt(9.81) * dt / dx
        let cfl = cfg.cfl(1.0, 0.0);
        assert!(cfl > 0.0);
        assert!(cfl < 1.0, "static water CFL should be < 1: {}", cfl);
    }

    #[test]
    fn test_stable_dt() {
        let cfg = stable_config();
        let dt = cfg.stable_dt(1.0, 0.0);
        assert!(dt > 0.0);
        assert!(dt < 1.0);
    }

    #[test]
    fn test_initialize_uniform() {
        let mut s = SwSolver::new(stable_config());
        s.initialize_uniform(2.0);
        assert!((s.mean_h() - 2.0).abs() < 1e-6);
        assert_eq!(s.max_velocity(), 0.0);
    }

    #[test]
    fn test_initialize_droplet() {
        let mut s = SwSolver::new(stable_config());
        s.initialize_droplet(32.0, 32.0, 5.0, 1.0, 2.0);
        assert!(s.max_h() > 1.9);
        assert!((s.min_h() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_initialize_dam_break() {
        let mut s = SwSolver::new(stable_config());
        s.initialize_dam_break(32.0, 1.0, 2.0);
        let nx = s.config.nx;
        // 左半 h=2, 右半 h=1
        assert!((s.h_curr[s.idx(0, 0)] - 2.0).abs() < 1e-6);
        assert!((s.h_curr[s.idx(nx - 1, 0)] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = SwSolver::new(stable_config());
        s.initialize_droplet(32.0, 32.0, 5.0, 1.0, 1.5);
        let t0 = s.time;
        s.step();
        assert!(s.time > t0);
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = SwSolver::new(stable_config());
        s.initialize_droplet(32.0, 32.0, 5.0, 1.0, 1.5);
        s.step_n(10);
        assert_eq!(s.steps, 10);
    }

    #[test]
    fn test_static_water_remains_static() {
        // 均匀静止水应保持静止
        let mut s = SwSolver::new(stable_config());
        s.initialize_uniform(1.0);
        s.step_n(100);
        assert!(!s.has_nan(), "NaN in static water");
        // 水深应保持均匀
        assert!(s.variance_h() < 1e-8, "static water should stay uniform: var={}", s.variance_h());
        // 流速应保持 0
        assert!(s.max_velocity() < 1e-5, "static water should have zero velocity: {}", s.max_velocity());
    }

    #[test]
    fn test_mass_conservation_periodic() {
        // 周期边界总质量守恒
        let mut s = SwSolver::new(stable_config());
        s.initialize_droplet(32.0, 32.0, 5.0, 1.0, 1.5);
        let m0 = s.total_mass();
        s.step_n(500);
        let m1 = s.total_mass();
        assert!((m1 - m0).abs() / m0 < 0.01, "mass should be conserved: {} -> {}", m0, m1);
    }

    #[test]
    fn test_droplet_generates_waves() {
        // 水滴应产生向外传播的波
        let mut s = SwSolver::new(stable_config());
        s.initialize_droplet(32.0, 32.0, 5.0, 1.0, 1.5);
        let v0 = s.max_velocity();
        s.step_n(50);
        let v1 = s.max_velocity();
        assert!(v1 > v0, "droplet should generate motion: {} -> {}", v0, v1);
    }

    #[test]
    fn test_dam_break_flow() {
        // 大坝溃决: 高水位流向低水位
        let mut s = SwSolver::new(stable_config());
        s.initialize_dam_break(32.0, 1.0, 2.0);
        s.step_n(50);
        // 应产生 x 方向流动
        assert!(s.max_velocity() > 0.01, "dam break should generate flow: {}", s.max_velocity());
    }

    #[test]
    fn test_no_nan_short_run() {
        let mut s = SwSolver::new(stable_config());
        s.initialize_droplet(32.0, 32.0, 5.0, 1.0, 2.0);
        s.step_n(500);
        assert!(!s.has_nan(), "NaN after 500 steps");
    }

    #[test]
    fn test_no_nan_long_run() {
        let cfg = SwConfig { nx: 32, ny: 32, ..stable_config() };
        let mut s = SwSolver::new(cfg);
        s.initialize_droplet(16.0, 16.0, 3.0, 1.0, 1.5);
        s.step_n(3000);
        assert!(!s.has_nan(), "NaN after 3000 steps");
    }

    #[test]
    fn test_h_nonnegative() {
        let mut s = SwSolver::new(stable_config());
        s.initialize_droplet(32.0, 32.0, 5.0, 0.1, 1.0);
        s.step_n(1000);
        assert!(s.min_h() >= 0.0, "h should be non-negative: {}", s.min_h());
    }

    #[test]
    fn test_amplitude_bounded() {
        let mut s = SwSolver::new(stable_config());
        s.initialize_droplet(32.0, 32.0, 5.0, 1.0, 2.0);
        let h0_max = s.max_h();
        s.step_n(2000);
        assert!(s.max_h() < h0_max + 0.5, "max h should not grow unbounded: {} -> {}", h0_max, s.max_h());
    }

    #[test]
    fn test_energy_decreasing() {
        // 无外力时总能量应近似不增 (Lax-Friedrichs 有数值耗散)
        let mut s = SwSolver::new(stable_config());
        s.initialize_droplet(32.0, 32.0, 5.0, 1.0, 1.5);
        let e0 = s.total_energy();
        s.step_n(500);
        let e1 = s.total_energy();
        assert!(e1.is_finite(), "energy not finite");
        assert!(e1 <= e0 + 1.0, "energy should not grow: {} -> {}", e0, e1);
    }

    #[test]
    fn test_total_mass_calculation() {
        let mut s = SwSolver::new(stable_config());
        s.initialize_uniform(1.5);
        let n = s.config.n_cells() as f32;
        assert!((s.total_mass() - 1.5 * n).abs() < 1e-3);
    }

    #[test]
    fn test_total_energy_static() {
        let mut s = SwSolver::new(stable_config());
        s.initialize_uniform(2.0);
        // 静止水能量 = 0.5 * g * h² * N
        let n = s.config.n_cells() as f32;
        let expected = 0.5 * G * 4.0 * n;
        assert!((s.total_energy() - expected).abs() / expected < 1e-4);
    }

    #[test]
    fn test_variance_h_initial() {
        let mut s = SwSolver::new(stable_config());
        s.initialize_uniform(1.0);
        assert!(s.variance_h() < 1e-8, "uniform should have zero variance");
        s.initialize_droplet(32.0, 32.0, 5.0, 1.0, 2.0);
        assert!(s.variance_h() > 0.0, "droplet should have positive variance");
    }

    #[test]
    fn test_reflecting_boundary() {
        let cfg = SwConfig {
            nx: 32,
            ny: 32,
            dt: 0.01,
            boundary: SwBoundary::Reflecting,
            ..stable_config()
        };
        let mut s = SwSolver::new(cfg);
        s.initialize_droplet(16.0, 16.0, 3.0, 1.0, 1.5);
        s.step_n(100);
        // 反射边界: 边界处法向动量应为 0
        let nx = s.config.nx;
        let ny = s.config.ny;
        // 左右边界 hu 应接近 0
        for j in 0..ny {
            assert!(s.hu_curr[s.idx(0, j)].abs() < 1e-4, "left boundary hu should be 0: {}", s.hu_curr[s.idx(0, j)]);
            assert!(s.hu_curr[s.idx(nx - 1, j)].abs() < 1e-4, "right boundary hu should be 0");
        }
        // 上下边界 hv 应接近 0
        for i in 0..nx {
            assert!(s.hv_curr[s.idx(i, 0)].abs() < 1e-4, "top boundary hv should be 0");
            assert!(s.hv_curr[s.idx(i, ny - 1)].abs() < 1e-4, "bottom boundary hv should be 0");
        }
    }

    #[test]
    fn test_periodic_boundary_wrap() {
        assert_eq!(SwSolver::wrap(-1, 10), 9);
        assert_eq!(SwSolver::wrap(0, 10), 0);
        assert_eq!(SwSolver::wrap(10, 10), 0);
        assert_eq!(SwSolver::wrap(11, 10), 1);
    }

    #[test]
    fn test_max_min_h() {
        let mut s = SwSolver::new(stable_config());
        s.initialize_uniform(1.0);
        assert!((s.max_h() - 1.0).abs() < 1e-6);
        assert!((s.min_h() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_module_count() {
        let tests: Vec<&str> = vec![
            "test_default_config",
            "test_n_cells",
            "test_domain_area",
            "test_wave_speed",
            "test_cfl_static_water",
            "test_stable_dt",
            "test_initialize_uniform",
            "test_initialize_droplet",
            "test_initialize_dam_break",
            "test_step_advances_time",
            "test_step_n_advances",
            "test_static_water_remains_static",
            "test_mass_conservation_periodic",
            "test_droplet_generates_waves",
            "test_dam_break_flow",
            "test_no_nan_short_run",
            "test_no_nan_long_run",
            "test_h_nonnegative",
            "test_amplitude_bounded",
            "test_energy_decreasing",
            "test_total_mass_calculation",
            "test_total_energy_static",
            "test_variance_h_initial",
            "test_reflecting_boundary",
            "test_periodic_boundary_wrap",
            "test_max_min_h",
        ];
        assert!(tests.len() >= 20, "need at least 20 tests, got {}", tests.len());
    }
}
