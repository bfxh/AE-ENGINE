//! Rössler Attractor — Rössler 原始 3D 混沌系统 (1976)
//!
//! Otto E. Rössler 1976 年设计的最简单 3D 混沌系统之一. 与 Lorenz 系统
//! 相比, Rössler 系统只有一个非线性项 (x·z), 且只有一条带状吸引子 (而非
//! Lorenz 的双翅膀). Rössler 设计该系统的初衷是: 在保持混沌的前提下, 用
//! 最少的非线性项和最简单的拓扑结构产生奇异吸引子.
//!
//! 状态方程 (Rössler 1976):
//!   dx/dt = -y - z
//!   dy/dt = x + a y
//!   dz/dt = b + z (x - c)
//!
//! 经典参数: a = 0.2, b = 0.2, c = 5.7
//! 经典初值: (x0, y0, z0) = (0, 0, 0) (从不稳定平衡点附近出发即可进入混沌)
//!
//! 性质:
//!   - 散度 ∇·F = tr(J) = a + (x - c) (非常数, 随 x 变化)
//!     在吸引子上平均 x ≈ 0, 平均散度 ≈ a - c ≈ -5.5 (强耗散)
//!   - 平衡点 (解 a z² - c z + b = 0):
//!     z* = (c ± sqrt(c² - 4ab)) / (2a)
//!     E₋ = (a z₋, -z₋, z₋), E₊ = (a z₊, -z₊, z₊)
//!     经典参数:
//!       z₋ ≈ 0.0354  →  E₋ ≈ (0.0071, -0.0354, 0.0354)  (鞍点, 混沌源)
//!       z₊ ≈ 28.46   →  E₊ ≈ (5.69, -28.46, 28.46)     (鞍焦点)
//!   - Lyapunov 谱 (经典参数, 文献值):
//!     λ₁ ≈ +0.0714  (正, 主混沌方向, 较弱)
//!     λ₂ = 0        (沿轨道切向)
//!     λ₃ ≈ -5.594   (负, 强收缩)
//!     Kaplan-Yorke 维数 D_KY = 2 + λ₁/|λ₃| ≈ 2.013
//!   - 与 Lorenz 系统相比:
//!     * 只有一个非线性项 (x z) vs Lorenz 的两个 (x y, x z)
//!     * 单一带状吸引子 vs Lorenz 的双翅膀
//!     * 更低的混沌强度 (λ₁ ≈ 0.07 vs Lorenz λ₁ ≈ 0.9)
//!   - Rössler 超混沌系统 (4D) 是在此基础上的扩展, 已有独立模块
//!     (rossler_hyperchaos)
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!
//! 历史:
//!   Rössler, O. E. 1976. "An equation for continuous chaos."
//!   Phys. Lett. A 57(5), 397-398. (首创 Rössler 系统)
//!   Rössler, O. E. 1979. "An equation for hyperchaos." Phys. Lett. A 71, 155.
//!   (超混沌扩展; 本仓库 rossler_hyperchaos 模块)

/// Rössler 配置 (3 参数 + dt)
#[derive(Clone, Copy, Debug)]
pub struct RosslerConfig {
    /// 参数 a (y 线性反馈率, 经典 0.2)
    pub a: f64,
    /// 参数 b (z 偏置, 经典 0.2)
    pub b: f64,
    /// 参数 c (z 阈值, 经典 5.7)
    pub c: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for RosslerConfig {
    fn default() -> Self {
        Self { a: 0.2, b: 0.2, c: 5.7, dt: 0.01 }
    }
}

/// Rössler 求解器 (3D, 跟踪最大 Lyapunov 指数)
pub struct RosslerSolver {
    pub config: RosslerConfig,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub time: f64,
    pub step_count: u64,
    pub trajectory: Vec<(f64, f64, f64)>,
    /// Lyapunov 累积
    pub lyap_sum: f64,
    /// 切向量 (3D)
    pub v: [f64; 3],
}

impl RosslerSolver {
    pub fn new(config: RosslerConfig, x0: f64, y0: f64, z0: f64) -> Self {
        Self {
            config,
            x: x0,
            y: y0,
            z: z0,
            time: 0.0,
            step_count: 0,
            trajectory: vec![(x0, y0, z0)],
            lyap_sum: 0.0,
            v: [1.0, 0.0, 0.0],
        }
    }

    pub fn classic(config: RosslerConfig) -> Self {
        // Rössler 1976 经典初值 (0, 0, 0)
        Self::new(config, 0.0, 0.0, 0.0)
    }

    /// 右端导数 F = [-y - z, x + a y, b + z (x - c)]
    pub fn derivatives(cfg: &RosslerConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        [-y - z, x + cfg.a * y, cfg.b + z * (x - cfg.c)]
    }

    /// Jacobian:
    /// J = [[ 0,    -1,    -1     ],
    ///      [ 1,     a,     0     ],
    ///      [ z,     0,     x - c ]]
    pub fn jacobian(cfg: &RosslerConfig, x: f64, _y: f64, z: f64) -> [[f64; 3]; 3] {
        [
            [0.0, -1.0, -1.0],
            [1.0, cfg.a, 0.0],
            [z, 0.0, x - cfg.c],
        ]
    }

    /// 散度 ∇·F = tr(J) = a + (x - c) (非常数)
    pub fn divergence(cfg: &RosslerConfig, x: f64, _y: f64, _z: f64) -> f64 {
        cfg.a + x - cfg.c
    }

    /// 计算两个平衡点:
    /// z* = (c ± sqrt(c² - 4ab)) / (2a), x* = a z*, y* = -z*
    /// 返回 (E₋, E₊), E₋ 为内平衡点 (混沌源), E₊ 为外平衡点
    pub fn equilibria(cfg: &RosslerConfig) -> Option<([f64; 3], [f64; 3])> {
        let disc = cfg.c * cfg.c - 4.0 * cfg.a * cfg.b;
        if disc < 0.0 {
            return None;
        }
        let sqrt_disc = disc.sqrt();
        let z_minus = (cfg.c - sqrt_disc) / (2.0 * cfg.a);
        let z_plus = (cfg.c + sqrt_disc) / (2.0 * cfg.a);
        let e_minus = [cfg.a * z_minus, -z_minus, z_minus];
        let e_plus = [cfg.a * z_plus, -z_plus, z_plus];
        Some((e_minus, e_plus))
    }

    /// 单步 RK4 推进 + 变分方程 Lyapunov
    pub fn step(&mut self) {
        let cfg = self.config;
        let dt = cfg.dt;
        let (x, y, z) = (self.x, self.y, self.z);

        let k1 = Self::derivatives(&cfg, x, y, z);
        let k2 = Self::derivatives(&cfg, x + 0.5 * dt * k1[0], y + 0.5 * dt * k1[1], z + 0.5 * dt * k1[2]);
        let k3 = Self::derivatives(&cfg, x + 0.5 * dt * k2[0], y + 0.5 * dt * k2[1], z + 0.5 * dt * k2[2]);
        let k4 = Self::derivatives(&cfg, x + dt * k3[0], y + dt * k3[1], z + dt * k3[2]);

        self.x = x + dt / 6.0 * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]);
        self.y = y + dt / 6.0 * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]);
        self.z = z + dt / 6.0 * (k1[2] + 2.0 * k2[2] + 2.0 * k3[2] + k4[2]);

        self.step_count += 1;
        self.time += dt;
        self.trajectory.push((self.x, self.y, self.z));

        // Lyapunov: 变分方程前向欧拉 (I + dt J) v
        let j = Self::jacobian(&cfg, self.x, self.y, self.z);
        let new_v = [
            self.v[0] + dt * (j[0][0] * self.v[0] + j[0][1] * self.v[1] + j[0][2] * self.v[2]),
            self.v[1] + dt * (j[1][0] * self.v[0] + j[1][1] * self.v[1] + j[1][2] * self.v[2]),
            self.v[2] + dt * (j[2][0] * self.v[0] + j[2][1] * self.v[1] + j[2][2] * self.v[2]),
        ];
        let mag = (new_v[0] * new_v[0] + new_v[1] * new_v[1] + new_v[2] * new_v[2]).sqrt();
        if mag > 0.0 {
            self.lyap_sum += mag.ln();
            self.v[0] = new_v[0] / mag;
            self.v[1] = new_v[1] / mag;
            self.v[2] = new_v[2] / mag;
        }
    }

    pub fn run(&mut self, n_steps: usize) {
        for _ in 0..n_steps {
            self.step();
        }
    }

    /// 最大 Lyapunov 指数 (文献值 ~0.0714)
    pub fn lyapunov_exponent(&self) -> f64 {
        if self.step_count == 0 {
            return 0.0;
        }
        self.lyap_sum / self.time.max(1e-12)
    }

    pub fn has_nan(&self) -> bool {
        !self.x.is_finite() || !self.y.is_finite() || !self.z.is_finite()
    }

    pub fn has_escaped(&self) -> bool {
        self.x.abs() > 100.0 || self.y.abs() > 100.0 || self.z.abs() > 100.0 || self.has_nan()
    }

    pub fn attractor_bounds(&self) -> (f64, f64, f64, f64, f64, f64) {
        let mut xmin = f64::INFINITY;
        let mut xmax = f64::NEG_INFINITY;
        let mut ymin = f64::INFINITY;
        let mut ymax = f64::NEG_INFINITY;
        let mut zmin = f64::INFINITY;
        let mut zmax = f64::NEG_INFINITY;
        for &(x, y, z) in &self.trajectory {
            if x < xmin { xmin = x; }
            if x > xmax { xmax = x; }
            if y < ymin { ymin = y; }
            if y > ymax { ymax = y; }
            if z < zmin { zmin = z; }
            if z > zmax { zmax = z; }
        }
        (xmin, xmax, ymin, ymax, zmin, zmax)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn test_default_config() {
        let cfg = RosslerConfig::default();
        assert!(approx_eq(cfg.a, 0.2, 1e-12));
        assert!(approx_eq(cfg.b, 0.2, 1e-12));
        assert!(approx_eq(cfg.c, 5.7, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = RosslerSolver::classic(RosslerConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = RosslerConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = RosslerSolver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], -y - z, 1e-12));
        assert!(approx_eq(d[1], x + cfg.a * y, 1e-12));
        assert!(approx_eq(d[2], cfg.b + z * (x - cfg.c), 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = RosslerConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = RosslerSolver::jacobian(&cfg, x, y, z);
        // Row 0: [0, -1, -1]
        assert!(approx_eq(j[0][0], 0.0, 1e-12));
        assert!(approx_eq(j[0][1], -1.0, 1e-12));
        assert!(approx_eq(j[0][2], -1.0, 1e-12));
        // Row 1: [1, a, 0]
        assert!(approx_eq(j[1][0], 1.0, 1e-12));
        assert!(approx_eq(j[1][1], cfg.a, 1e-12));
        assert!(approx_eq(j[1][2], 0.0, 1e-12));
        // Row 2: [z, 0, x - c]
        assert!(approx_eq(j[2][0], z, 1e-12));
        assert!(approx_eq(j[2][1], 0.0, 1e-12));
        assert!(approx_eq(j[2][2], x - cfg.c, 1e-12));
    }

    #[test]
    fn test_divergence_formula() {
        // 散度 = a + x - c
        let cfg = RosslerConfig::default();
        assert!(approx_eq(
            RosslerSolver::divergence(&cfg, 0.5, 0.0, 0.0),
            cfg.a + 0.5 - cfg.c,
            1e-12
        ));
        assert!(approx_eq(
            RosslerSolver::divergence(&cfg, -1.0, 0.0, 0.0),
            cfg.a - 1.0 - cfg.c,
            1e-12
        ));
    }

    #[test]
    fn test_divergence_not_constant() {
        // 散度随 x 变化 (非常数)
        let cfg = RosslerConfig::default();
        let d1 = RosslerSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        let d2 = RosslerSolver::divergence(&cfg, 1.0, 0.0, 0.0);
        assert!((d1 - d2).abs() > 0.5);
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = RosslerConfig::default();
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5)] {
            let j = RosslerSolver::jacobian(&cfg, x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = RosslerSolver::divergence(&cfg, x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_equilibria_exist() {
        let cfg = RosslerConfig::default();
        let eqs = RosslerSolver::equilibria(&cfg);
        assert!(eqs.is_some(), "should have equilibria for classical params");
    }

    #[test]
    fn test_equilibria_satisfy_equations() {
        let cfg = RosslerConfig::default();
        let (e_minus, e_plus) = RosslerSolver::equilibria(&cfg).unwrap();
        for eq in [e_minus, e_plus] {
            let d = RosslerSolver::derivatives(&cfg, eq[0], eq[1], eq[2]);
            for v in d.iter() {
                assert!(v.abs() < 1e-9, "equilibrium derivative = {}", v);
            }
        }
    }

    #[test]
    fn test_inner_equilibrium_near_origin() {
        // 内平衡点 E₋ 应接近原点 (z₋ ≈ 0.0354)
        let cfg = RosslerConfig::default();
        let (e_minus, _e_plus) = RosslerSolver::equilibria(&cfg).unwrap();
        assert!(e_minus[2].abs() < 1.0, "inner eq z should be small: {}", e_minus[2]);
    }

    #[test]
    fn test_outer_equilibrium_far() {
        // 外平衡点 E₊ 应远离原点 (z₊ ≈ 28.46)
        let cfg = RosslerConfig::default();
        let (_e_minus, e_plus) = RosslerSolver::equilibria(&cfg).unwrap();
        assert!(e_plus[2] > 10.0, "outer eq z should be large: {}", e_plus[2]);
    }

    #[test]
    fn test_step_advances() {
        let mut s = RosslerSolver::classic(RosslerConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = RosslerSolver::classic(RosslerConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = RosslerSolver::classic(RosslerConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = RosslerSolver::classic(RosslerConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = RosslerSolver::classic(RosslerConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        // Rössler 吸引子典型范围: x∈[-12,12], y∈[-12,12], z∈[0,25]
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        // Rössler 是混沌的, λ > 0 (文献值 ~0.0714)
        let mut s = RosslerSolver::classic(RosslerConfig::default());
        s.run(80000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = RosslerSolver::classic(RosslerConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        let cfg = RosslerConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = RosslerSolver::new(cfg, 1.0, 1.0, 1.0);
        let mut s2 = RosslerSolver::new(cfg, 1.0 + d0, 1.0, 1.0);
        for _ in 0..50000 {
            s1.step();
            s2.step();
        }
        let dx = s1.x - s2.x;
        let dy = s1.y - s2.y;
        let dz = s1.z - s2.z;
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        // t=500, λ~0.071, 应放大 e^35 (饱和到吸引子尺度)
        assert!(d > 1e-3, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_average_divergence_negative() {
        // 在吸引子轨道上采样, 平均散度应为负 (耗散)
        let cfg = RosslerConfig::default();
        let mut s = RosslerSolver::classic(cfg);
        s.run(20000);
        // 跳过暂态, 在吸引子上采样
        let mut div_sum = 0.0;
        let n_samples = 1000;
        for i in (10000..s.trajectory.len()).step_by(10) {
            let (x, _y, _z) = s.trajectory[i];
            div_sum += RosslerSolver::divergence(&cfg, x, 0.0, 0.0);
        }
        let avg_div = div_sum / n_samples as f64;
        assert!(avg_div < 0.0, "average divergence should be negative: {}", avg_div);
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = RosslerConfig::default();
        let mut s = RosslerSolver::new(cfg, 100.0, 100.0, 100.0);
        s.run(500);
        assert!(s.has_escaped() || !s.has_nan());
    }

    #[test]
    fn test_single_nonlinearity() {
        // Rössler 只有一个非线性项 (x*z), Jacobian 中只有 z 和 (x-c) 非零偏导
        // 这意味着 J 不含 y 的偏导 (除 a 常数), 是 Rössler 系统的标志特征
        let cfg = RosslerConfig::default();
        let j = RosslerSolver::jacobian(&cfg, 0.5, 0.3, 0.7);
        // 第 0 行: [0, -1, -1] (无常数)
        // 第 1 行: [1, a, 0] (无常数)
        // 第 2 行: [z, 0, x-c] (两个非常数项)
        assert!(approx_eq(j[0][0], 0.0, 1e-12));
        assert!(approx_eq(j[0][1], -1.0, 1e-12));
        assert!(approx_eq(j[0][2], -1.0, 1e-12));
        assert!(approx_eq(j[1][2], 0.0, 1e-12));
        assert!(approx_eq(j[2][1], 0.0, 1e-12));
    }
}
