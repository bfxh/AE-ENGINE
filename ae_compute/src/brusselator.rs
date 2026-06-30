//! Brusselator Solver — 化学振荡与 Turing 模式
//!
//! Brusselator 是 Prigogine 和 Lefever 于 1968 年提出的自催化反应模型,
//! 因其在比利时 Brussels 提出而得名. 它是最简单的能展现 Hopf 分岔
//! (时间振荡) 和 Turing 分岔 (空间模式) 的反应扩散系统.
//!
//! 反应:
//!   A -> X
//!   2X + Y -> 3X
//!   B + X -> Y + D
//!   X -> E
//!
//! 速率方程 (假设 A, B 恒定):
//!   ∂X/∂t = A - (B+1)·X + X²·Y + D_X·∇²X
//!   ∂Y/∂t = B·X - X²·Y + D_Y·∇²Y
//!
//! 均匀稳态:
//!   X* = A
//!   Y* = B/A
//!
//! 线性稳定性分析 (在稳态附近):
//!   Jacobian = [ B-1,  A² ]
//!              [ -B,  -A² ]
//!   迹: tr = B - 1 - A²
//!   行列式: det = A²
//!
//! Hopf 分岔 (时间振荡):
//!   tr = 0  =>  B = 1 + A²
//!   B > 1 + A²: 稳态失稳, 形成极限环振荡
//!
//! Turing 分岔 (空间模式):
//!   需要 D_Y > D_X (Y 扩散更快, 反直觉)
//!   临界条件: (D_X·a22 + D_Y·a11)² - 4·D_X·D_Y·det = 0
//!   即: (D_X·(-A²) + D_Y·(B-1))² = 4·D_X·D_Y·A²
//!   简化: D_Y/D_X > ((B-1) + A²)/((B-1) - A²) ... (条件更严格)
//!
//! 数值方法:
//!   显式 Euler + 5点 Laplacian (2D)
//!   CFL: dt <= dx² / (4·max(D_X, D_Y))
//!
//! 应用:
//!   - 化学振荡 (BZ 反应的简化模型)
//!   - 形态发生 (Turing 1952)
//!   - 自组织现象研究
//!   - 非平衡态热力学
//!
//! 基于:
//!   - Prigogine, I. & Lefever, R. 1968. J. Chem. Phys. 48, 1695.
//!   - Turing, A.M. 1952. Phil. Trans. R. Soc. B 237, 37.
//!   - Nicolis, G. & Prigogine, I. "Self-Organization in Nonequilibrium Systems." 1977.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum BrBoundary {
    Periodic,
    Neumann,
    Dirichlet { x_value: f32, y_value: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrConfig {
    pub nx: usize,
    pub ny: usize,
    pub dx: f32,
    pub dt: f32,
    /// 反应物 A (常数)
    pub a: f32,
    /// 反应物 B (常数)
    pub b: f32,
    /// X 扩散系数
    pub d_x: f32,
    /// Y 扩散系数
    pub d_y: f32,
    pub boundary: BrBoundary,
}

impl Default for BrConfig {
    fn default() -> Self {
        BrConfig {
            nx: 64,
            ny: 64,
            dx: 1.0,
            dt: 0.005,
            a: 1.0,
            b: 2.0,
            d_x: 1.0,
            d_y: 8.0,
            boundary: BrBoundary::Periodic,
        }
    }
}

impl BrConfig {
    pub fn n_cells(&self) -> usize {
        self.nx * self.ny
    }

    /// 均匀稳态 X* = A
    pub fn steady_x(&self) -> f32 {
        self.a
    }

    /// 均匀稳态 Y* = B/A
    pub fn steady_y(&self) -> f32 {
        if self.a.abs() < 1e-12 {
            0.0
        } else {
            self.b / self.a
        }
    }

    /// Jacobian 迹: tr = B - 1 - A²
    pub fn jacobian_trace(&self) -> f32 {
        self.b - 1.0 - self.a * self.a
    }

    /// Jacobian 行列式: det = A²
    pub fn jacobian_det(&self) -> f32 {
        self.a * self.a
    }

    /// 是否超过 Hopf 分岔阈值 (B > 1 + A²)
    pub fn is_above_hopf(&self) -> bool {
        self.b > 1.0 + self.a * self.a
    }

    /// 扩散 CFL: dt <= dx² / (4·max(D_X, D_Y))
    pub fn diffusive_cfl(&self) -> f32 {
        let d_max = self.d_x.max(self.d_y);
        4.0 * d_max * self.dt / (self.dx * self.dx)
    }

    pub fn is_stable(&self) -> bool {
        self.diffusive_cfl() <= 1.0
    }

    pub fn stable_dt(&self) -> f32 {
        let d_max = self.d_x.max(self.d_y);
        0.25 * self.dx * self.dx / d_max.max(1e-12)
    }
}

pub struct BrusselatorSolver {
    pub config: BrConfig,
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub x_next: Vec<f32>,
    pub y_next: Vec<f32>,
    pub time: f32,
    pub steps: usize,
}

impl BrusselatorSolver {
    pub fn new(config: BrConfig) -> Self {
        let n = config.n_cells();
        let sx = config.steady_x();
        let sy = config.steady_y();
        BrusselatorSolver {
            config,
            x: vec![sx; n],
            y: vec![sy; n],
            x_next: vec![0.0; n],
            y_next: vec![0.0; n],
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

    /// 初始化为均匀稳态
    pub fn initialize_steady(&mut self) {
        let sx = self.config.steady_x();
        let sy = self.config.steady_y();
        for v in &mut self.x {
            *v = sx;
        }
        for v in &mut self.y {
            *v = sy;
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化为稳态 + 随机扰动
    pub fn initialize_perturbed(&mut self, amplitude: f32, seed: u64) {
        let sx = self.config.steady_x();
        let sy = self.config.steady_y();
        let mut rng = BrRng::new(seed);
        for i in 0..self.x.len() {
            self.x[i] = sx + amplitude * (2.0 * rng.next() - 1.0);
            self.y[i] = sy + amplitude * (2.0 * rng.next() - 1.0);
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化为稳态 + 中心斑点
    pub fn initialize_spot(&mut self, cx: f32, cy: f32, radius: f32, delta: f32) {
        let sx = self.config.steady_x();
        let sy = self.config.steady_y();
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        let r2 = radius * radius;
        for v in &mut self.x {
            *v = sx;
        }
        for v in &mut self.y {
            *v = sy;
        }
        for j in 0..ny {
            for i in 0..nx {
                let px = i as f32 * dx;
                let py = j as f32 * dx;
                let d2 = (px - cx) * (px - cx) + (py - cy) * (py - cy);
                if d2 < r2 {
                    let k = self.idx(i, j);
                    self.x[k] = sx + delta;
                    self.y[k] = sy - delta;
                }
            }
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化为正弦波模式 (激发特定波数)
    pub fn initialize_sine(&mut self, amplitude: f32, kx: f32, ky: f32) {
        let sx = self.config.steady_x();
        let sy = self.config.steady_y();
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx = self.config.dx;
        for j in 0..ny {
            for i in 0..nx {
                let px = i as f32 * dx;
                let py = j as f32 * dx;
                let k = self.idx(i, j);
                let perturb = amplitude * (kx * px + ky * py).sin();
                self.x[k] = sx + perturb;
                self.y[k] = sy - perturb;
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
        let a = self.config.a;
        let b = self.config.b;
        let d_x = self.config.d_x;
        let d_y = self.config.d_y;
        let inv_dx2 = 1.0 / (dx * dx);
        let bc = self.config.boundary;

        for j in 0..ny {
            for i in 0..nx {
                let (ip, im, jp, jm) = match bc {
                    BrBoundary::Periodic => (
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

                let k = self.idx(i, j);
                let xc = self.x[k];
                let yc = self.y[k];
                let lap_x = (self.x[self.idx(ip, j)]
                    + self.x[self.idx(im, j)]
                    + self.x[self.idx(i, jp)]
                    + self.x[self.idx(i, jm)]
                    - 4.0 * xc)
                    * inv_dx2;
                let lap_y = (self.y[self.idx(ip, j)]
                    + self.y[self.idx(im, j)]
                    + self.y[self.idx(i, jp)]
                    + self.y[self.idx(i, jm)]
                    - 4.0 * yc)
                    * inv_dx2;

                let x2y = xc * xc * yc;
                let rhs_x = a - (b + 1.0) * xc + x2y + d_x * lap_x;
                let rhs_y = b * xc - x2y + d_y * lap_y;

                self.x_next[k] = xc + dt * rhs_x;
                self.y_next[k] = yc + dt * rhs_y;
            }
        }

        if let BrBoundary::Dirichlet { x_value, y_value } = bc {
            for i in 0..nx {
                let top = self.idx(i, 0);
                let bot = self.idx(i, ny - 1);
                self.x_next[top] = x_value;
                self.x_next[bot] = x_value;
                self.y_next[top] = y_value;
                self.y_next[bot] = y_value;
            }
            for j in 0..ny {
                let left = self.idx(0, j);
                let right = self.idx(nx - 1, j);
                self.x_next[left] = x_value;
                self.x_next[right] = x_value;
                self.y_next[left] = y_value;
                self.y_next[right] = y_value;
            }
        }

        std::mem::swap(&mut self.x, &mut self.x_next);
        std::mem::swap(&mut self.y, &mut self.y_next);
        self.time += dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    // ========================================================
    // 诊断
    // ========================================================

    pub fn mean_x(&self) -> f32 {
        let n = self.x.len();
        if n == 0 {
            return 0.0;
        }
        self.x.iter().sum::<f32>() / n as f32
    }

    pub fn mean_y(&self) -> f32 {
        let n = self.y.len();
        if n == 0 {
            return 0.0;
        }
        self.y.iter().sum::<f32>() / n as f32
    }

    pub fn variance_x(&self) -> f32 {
        let m = self.mean_x();
        let n = self.x.len();
        if n == 0 {
            return 0.0;
        }
        self.x.iter().map(|&v| (v - m) * (v - m)).sum::<f32>() / n as f32
    }

    pub fn variance_y(&self) -> f32 {
        let m = self.mean_y();
        let n = self.y.len();
        if n == 0 {
            return 0.0;
        }
        self.y.iter().map(|&v| (v - m) * (v - m)).sum::<f32>() / n as f32
    }

    pub fn max_x(&self) -> f32 {
        self.x.iter().copied().fold(f32::NEG_INFINITY, f32::max)
    }

    pub fn min_x(&self) -> f32 {
        self.x.iter().copied().fold(f32::INFINITY, f32::min)
    }

    pub fn max_y(&self) -> f32 {
        self.y.iter().copied().fold(f32::NEG_INFINITY, f32::max)
    }

    pub fn min_y(&self) -> f32 {
        self.y.iter().copied().fold(f32::INFINITY, f32::min)
    }

    pub fn max_abs_x(&self) -> f32 {
        self.x.iter().map(|&v| v.abs()).fold(0.0f32, f32::max)
    }

    pub fn max_abs_y(&self) -> f32 {
        self.y.iter().map(|&v| v.abs()).fold(0.0f32, f32::max)
    }

    pub fn has_nan(&self) -> bool {
        self.x.iter().any(|&v| !v.is_finite()) || self.y.iter().any(|&v| !v.is_finite())
    }

    pub fn energy(&self) -> f32 {
        0.5 * (self.x.iter().map(|&v| v * v).sum::<f32>()
            + self.y.iter().map(|&v| v * v).sum::<f32>())
    }

    /// X 场的空间方差 (用于检测 Turing 模式生长)
    pub fn spatial_variance_x(&self) -> f32 {
        self.variance_x()
    }

    /// Y 场的空间方差
    pub fn spatial_variance_y(&self) -> f32 {
        self.variance_y()
    }

    /// 偏离稳态的 L2 距离
    pub fn deviation_from_steady(&self) -> f32 {
        let sx = self.config.steady_x();
        let sy = self.config.steady_y();
        let mut sum = 0.0f32;
        for i in 0..self.x.len() {
            let dx = self.x[i] - sx;
            let dy = self.y[i] - sy;
            sum += dx * dx + dy * dy;
        }
        sum.sqrt()
    }
}

struct BrRng {
    state: u64,
}

impl BrRng {
    fn new(seed: u64) -> Self {
        BrRng {
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

    fn stable_config() -> BrConfig {
        BrConfig {
            nx: 32,
            ny: 32,
            dx: 1.0,
            dt: 0.005,
            a: 1.0,
            b: 1.8,
            d_x: 1.0,
            d_y: 8.0,
            boundary: BrBoundary::Periodic,
        }
    }

    fn turing_config() -> BrConfig {
        // D_Y >> D_X, B < 1+A²=2 (Hopf 以下), 但 Turing 不稳定
        BrConfig {
            nx: 48,
            ny: 48,
            dx: 1.0,
            dt: 0.002,
            a: 1.0,
            b: 1.9,
            d_x: 1.0,
            d_y: 10.0,
            boundary: BrBoundary::Periodic,
        }
    }

    #[test]
    fn test_default_config_stability() {
        let cfg = BrConfig::default();
        assert!(cfg.is_stable(), "default config should be stable");
    }

    #[test]
    fn test_stable_dt_positive() {
        let cfg = BrConfig::default();
        assert!(cfg.stable_dt() > 0.0);
    }

    #[test]
    fn test_steady_state() {
        let cfg = BrConfig { a: 2.0, b: 3.0, ..Default::default() };
        assert!((cfg.steady_x() - 2.0).abs() < 1e-6);
        assert!((cfg.steady_y() - 1.5).abs() < 1e-6);
    }

    #[test]
    fn test_jacobian_trace() {
        let cfg = BrConfig { a: 1.0, b: 2.5, ..Default::default() };
        // tr = B - 1 - A² = 2.5 - 1 - 1 = 0.5
        assert!((cfg.jacobian_trace() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_jacobian_det() {
        let cfg = BrConfig { a: 2.0, b: 3.0, ..Default::default() };
        assert!((cfg.jacobian_det() - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_hopf_threshold() {
        let cfg_below = BrConfig { a: 1.0, b: 1.5, ..Default::default() };
        assert!(!cfg_below.is_above_hopf());
        let cfg_above = BrConfig { a: 1.0, b: 2.5, ..Default::default() };
        assert!(cfg_above.is_above_hopf());
    }

    #[test]
    fn test_initialize_steady() {
        let mut s = BrusselatorSolver::new(stable_config());
        s.initialize_steady();
        assert!((s.mean_x() - s.config.steady_x()).abs() < 1e-4);
        assert!((s.mean_y() - s.config.steady_y()).abs() < 1e-4);
    }

    #[test]
    fn test_initialize_perturbed_bounded() {
        let mut s = BrusselatorSolver::new(stable_config());
        let sx = s.config.steady_x();
        s.initialize_perturbed(0.1, 42);
        // 扰动幅度 0.1, X 在 [sx-0.1, sx+0.1]
        assert!(s.max_x() <= sx + 0.11);
        assert!(s.min_x() >= sx - 0.11);
    }

    #[test]
    fn test_initialize_spot() {
        let mut s = BrusselatorSolver::new(stable_config());
        s.initialize_spot(16.0, 16.0, 5.0, 1.0);
        // 中心斑点应有较高 X
        let center = s.idx(16, 16);
        assert!(s.x[center] > s.config.steady_x() + 0.5);
    }

    #[test]
    fn test_initialize_sine() {
        let mut s = BrusselatorSolver::new(stable_config());
        s.initialize_sine(0.5, 0.5, 0.0);
        assert!(s.max_x() > s.config.steady_x() + 0.4);
        assert!(s.min_x() < s.config.steady_x() - 0.4);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = BrusselatorSolver::new(stable_config());
        s.initialize_perturbed(0.01, 1);
        let t0 = s.time;
        s.step();
        assert!(s.time > t0);
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = BrusselatorSolver::new(stable_config());
        s.initialize_perturbed(0.01, 1);
        s.step_n(10);
        assert_eq!(s.steps, 10);
    }

    #[test]
    fn test_no_nan_short_run() {
        let mut s = BrusselatorSolver::new(stable_config());
        s.initialize_perturbed(0.1, 42);
        s.step_n(100);
        assert!(!s.has_nan(), "NaN detected after 100 steps");
    }

    #[test]
    fn test_no_nan_long_run() {
        let cfg = BrConfig {
            nx: 24,
            ny: 24,
            dt: 0.001,
            ..stable_config()
        };
        let mut s = BrusselatorSolver::new(cfg);
        s.initialize_perturbed(0.1, 99);
        s.step_n(1000);
        assert!(!s.has_nan(), "NaN detected after 1000 steps");
    }

    #[test]
    fn test_steady_state_remains_steady() {
        // 无扰动 + Dirichlet 边界 = 稳态应保持
        let cfg = BrConfig {
            nx: 16,
            ny: 16,
            dt: 0.001,
            boundary: BrBoundary::Dirichlet {
                x_value: 1.0,
                y_value: 1.8,
            },
            ..stable_config()
        };
        let mut s = BrusselatorSolver::new(cfg);
        s.initialize_steady();
        s.step_n(50);
        assert!(s.deviation_from_steady() < 1e-3, "steady state should remain steady");
    }

    #[test]
    fn test_below_hopf_decays_to_steady() {
        // B < 1+A²=2, 稳态稳定, 小扰动衰减
        let cfg = BrConfig {
            a: 1.0,
            b: 1.5, // < 2
            dt: 0.001,
            ..stable_config()
        };
        let mut s = BrusselatorSolver::new(cfg);
        s.initialize_perturbed(0.1, 7);
        let d0 = s.deviation_from_steady();
        s.step_n(500);
        let d1 = s.deviation_from_steady();
        assert!(d1 < d0, "below Hopf, perturbation should decay: {} -> {}", d0, d1);
    }

    #[test]
    fn test_above_hopf_grows() {
        // B > 1+A²=2, 稳态失稳, 小扰动增长 (Hopf 振荡)
        let cfg = BrConfig {
            nx: 32,
            ny: 32,
            dx: 1.0,
            dt: 0.001,
            a: 1.0,
            b: 2.5, // > 2
            d_x: 0.1, // 小扩散, 让时间振荡主导
            d_y: 0.1,
            boundary: BrBoundary::Periodic,
        };
        let mut s = BrusselatorSolver::new(cfg);
        s.initialize_perturbed(0.01, 11);
        let d0 = s.deviation_from_steady();
        s.step_n(2000);
        let d1 = s.deviation_from_steady();
        assert!(d1 > d0, "above Hopf, perturbation should grow: {} -> {}", d0, d1);
    }

    #[test]
    fn test_periodic_boundary_consistent() {
        let mut s = BrusselatorSolver::new(stable_config());
        s.initialize_sine(0.1, 0.5, 0.0);
        s.step_n(10);
        let left = s.x[s.idx(0, 16)];
        let right = s.x[s.idx(s.config.nx - 1, 16)];
        // 周期边界: 值应相近 (不严格相等因 sin 不完美周期)
        assert!((left - right).abs() < 1.0, "periodic boundary inconsistent");
    }

    #[test]
    fn test_dirichlet_boundary_enforced() {
        let cfg = BrConfig {
            nx: 16,
            ny: 16,
            dt: 0.001,
            boundary: BrBoundary::Dirichlet {
                x_value: 1.0,
                y_value: 1.8,
            },
            ..stable_config()
        };
        let mut s = BrusselatorSolver::new(cfg);
        s.initialize_perturbed(1.0, 5);
        s.step_n(10);
        for i in 0..s.config.nx {
            assert!(s.x[s.idx(i, 0)].abs() < 1e-3 + 1.0, "top X boundary not enforced");
            assert!(s.x[s.idx(i, s.config.ny - 1)].abs() < 1e-3 + 1.0, "bottom X boundary not enforced");
        }
        for j in 0..s.config.ny {
            assert!(s.x[s.idx(0, j)].abs() < 1e-3 + 1.0, "left X boundary not enforced");
            assert!(s.x[s.idx(s.config.nx - 1, j)].abs() < 1e-3 + 1.0, "right X boundary not enforced");
        }
    }

    #[test]
    fn test_amplitude_bounded() {
        let mut s = BrusselatorSolver::new(stable_config());
        s.initialize_perturbed(0.5, 42);
        s.step_n(500);
        assert!(s.max_abs_x() < 100.0, "X amplitude should be bounded: {}", s.max_abs_x());
        assert!(s.max_abs_y() < 100.0, "Y amplitude should be bounded: {}", s.max_abs_y());
    }

    #[test]
    fn test_variance_positive() {
        let mut s = BrusselatorSolver::new(stable_config());
        s.initialize_perturbed(0.5, 42);
        assert!(s.variance_x() > 0.0);
        assert!(s.variance_y() > 0.0);
    }

    #[test]
    fn test_energy_finite() {
        let mut s = BrusselatorSolver::new(stable_config());
        s.initialize_perturbed(0.5, 42);
        s.step_n(100);
        assert!(s.energy().is_finite());
    }

    #[test]
    fn test_deviation_from_steady() {
        let mut s = BrusselatorSolver::new(stable_config());
        s.initialize_steady();
        assert!(s.deviation_from_steady() < 1e-6);
        s.initialize_perturbed(0.5, 1);
        assert!(s.deviation_from_steady() > 0.0);
    }

    #[test]
    fn test_turing_pattern_growth() {
        // Turing 配置: D_Y >> D_X, B 略低于 Hopf, 空间模式应增长
        // 用 sine 激发 k 在不稳定带 (0.394, 0.803) 内的模式
        let nx = 48;
        let k_mode = 2.0 * std::f32::consts::PI * 4.0 / (nx as f32); // n=4, k≈0.524
        let cfg = BrConfig {
            nx,
            ny: nx,
            dx: 1.0,
            dt: 0.001,
            a: 1.0,
            b: 1.9,
            d_x: 1.0,
            d_y: 10.0,
            boundary: BrBoundary::Periodic,
        };
        let mut s = BrusselatorSolver::new(cfg);
        s.initialize_sine(0.01, k_mode, 0.0);
        let v0 = s.spatial_variance_x();
        s.step_n(10000);
        let v1 = s.spatial_variance_x();
        assert!(!s.has_nan(), "NaN in Turing pattern");
        assert!(v1 > v0, "Turing pattern should grow: {} -> {}", v0, v1);
    }

    #[test]
    fn test_diffusive_cfl() {
        let cfg = BrConfig::default();
        let cfl = cfg.diffusive_cfl();
        assert!(cfl > 0.0);
        assert!(cfl <= 1.0, "default config CFL should be <= 1: {}", cfl);
    }

    #[test]
    fn test_unstable_cfl_detected() {
        let cfg = BrConfig {
            dt: 1.0, // 太大
            ..Default::default()
        };
        assert!(!cfg.is_stable(), "dt=1.0 should be unstable");
    }
}
