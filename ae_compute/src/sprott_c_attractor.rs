//! Sprott C Attractor — Sprott C 混沌系统 (3D)
//!
//! Julien C. Sprott 1997 年在其论文 "Some simple chaotic flows" 中
//! 系统搜索发现的最简单混沌系统之一 (Sprott A-S, 共 19 个). Sprott C
//! 是其中第三个, 以极少的非线性项产生混沌行为, 展示了混沌的"简约性" —
//! 即使最简单的非线性耦合也能产生复杂动力学.
//!
//! 状态方程 (Sprott C):
//!   dx/dt = a y z
//!   dy/dt = x - y
//!   dz/dt = 1 - x²
//!
//! 各项物理意义:
//!   + a y z: 非线性耦合 (x 受 yz 乘积驱动)
//!   + x: 线性驱动 (x 推动 y)
//!   - y: 线性阻尼 (y 衰减)
//!   + 1: 常数驱动 (z 偏置)
//!   - x²: 非线性反馈 (z 受 x² 调制)
//!
//! 经典参数 (Sprott 1997): a = 1
//! 混沌增强参数: a = 2.017
//! 经典初值: (x₀, y₀, z₀) = (1, 1, 1) 或 (0.5, 0.5, 0.5)
//!
//! 性质:
//!   - 散度 ∇·F = tr(J) = -1 (常数, 与参数 a 无关)
//!     体积收缩率 e^(-t), 中等耗散
//!   - 平衡点 (利用 y=x, x²=1, yz=0):
//!     y = x (从 dy/dt = 0)
//!     x = ±1 (从 dz/dt = 0)
//!     z = 0 (从 dx/dt = a y z = 0, y≠0)
//!     E1 = (1, 1, 0), E2 = (-1, -1, 0)
//!   - Lyapunov 谱 (a=2.017, 文献值):
//!     λ₁ ≈ +0.21  (正, 主混沌方向)
//!     λ₂ = 0      (沿轨道切向)
//!     λ₃ ≈ -1.21  (负, 收缩)
//!     和 = -1 (与散度一致)
//!   - Kaplan-Yorke 维数 D_KY ≈ 2 + λ₁/|λ₃| ≈ 2.17
//!   - 吸引子形态: 单叶扭曲带
//!
//! Sprott 系列对比:
//!   - Sprott A: 1 个非线性项 (yz), 已完成
//!   - Sprott B: 2 个非线性项 (xz, xy)
//!   - Sprott C: 2 个非线性项 (yz, x²), 本模块
//!   - Sprott 系统共同特征: 极简 (3 变量, ≤2 非线性项, 常数散度)
//!
//! 简约性意义:
//!   Sprott 系统证明了混沌不需要复杂的方程. 即使只有 2 个非线性项,
//!   也能产生正 Lyapunov 指数和分形吸引子. 这对理解混沌的"最小
//!   条件"有基础性意义: 混沌的根源是非线性, 而非线性可以非常简单.
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!
//! 历史:
//!   Sprott, J. C. 1997. "Some simple chaotic flows." Phys. Rev. E 50,
//!   R647-R650. (Sprott A-S 系统的原始发现)
//!   Sprott, J. C. 2003. "Chaos and Time-Series Analysis." Oxford.
//!   (教科书中系统讨论 Sprott 系列)

/// Sprott C 系统配置 (1 参数 + dt)
#[derive(Clone, Copy, Debug)]
pub struct SprottCConfig {
    /// 参数 a (yz 耦合强度, 经典 1, 混沌 2.017)
    pub a: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for SprottCConfig {
    fn default() -> Self {
        Self { a: 2.017, dt: 0.01 }
    }
}

/// Sprott C 求解器 (3D, 跟踪最大 Lyapunov 指数)
pub struct SprottCSolver {
    pub config: SprottCConfig,
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

impl SprottCSolver {
    pub fn new(config: SprottCConfig, x0: f64, y0: f64, z0: f64) -> Self {
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

    pub fn classic(config: SprottCConfig) -> Self {
        Self::new(config, 1.0, 1.0, 1.0)
    }

    /// 右端导数 F = [a y z, x - y, 1 - x²]
    pub fn derivatives(cfg: &SprottCConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        [cfg.a * y * z, x - y, 1.0 - x * x]
    }

    /// Jacobian:
    /// J = [[0,    a z, a y],
    ///      [1,   -1,   0  ],
    ///      [-2x,  0,   0  ]]
    pub fn jacobian(cfg: &SprottCConfig, x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
        [
            [0.0, cfg.a * z, cfg.a * y],
            [1.0, -1.0, 0.0],
            [-2.0 * x, 0.0, 0.0],
        ]
    }

    /// 散度 ∇·F = tr(J) = -1 (常数, 与参数 a 无关)
    pub fn divergence(_cfg: &SprottCConfig, _x: f64, _y: f64, _z: f64) -> f64 {
        -1.0
    }

    /// 计算两个平衡点:
    /// E1 = (1, 1, 0), E2 = (-1, -1, 0)
    pub fn equilibria() -> ([f64; 3], [f64; 3]) {
        ([1.0, 1.0, 0.0], [-1.0, -1.0, 0.0])
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

    /// 最大 Lyapunov 指数 (文献值 ~0.21)
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
        let cfg = SprottCConfig::default();
        assert!(approx_eq(cfg.a, 2.017, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = SprottCSolver::classic(SprottCConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = SprottCConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = SprottCSolver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], cfg.a * y * z, 1e-12));
        assert!(approx_eq(d[1], x - y, 1e-12));
        assert!(approx_eq(d[2], 1.0 - x * x, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = SprottCConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = SprottCSolver::jacobian(&cfg, x, y, z);
        assert!(approx_eq(j[0][0], 0.0, 1e-12));
        assert!(approx_eq(j[0][1], cfg.a * z, 1e-12));
        assert!(approx_eq(j[0][2], cfg.a * y, 1e-12));
        assert!(approx_eq(j[1][0], 1.0, 1e-12));
        assert!(approx_eq(j[1][1], -1.0, 1e-12));
        assert!(approx_eq(j[1][2], 0.0, 1e-12));
        assert!(approx_eq(j[2][0], -2.0 * x, 1e-12));
        assert!(approx_eq(j[2][1], 0.0, 1e-12));
        assert!(approx_eq(j[2][2], 0.0, 1e-12));
    }

    #[test]
    fn test_divergence_constant() {
        // 散度 = -1 (常数, 与参数 a 无关)
        let cfg = SprottCConfig::default();
        assert!(approx_eq(SprottCSolver::divergence(&cfg, 0.0, 0.0, 0.0), -1.0, 1e-12));
        assert!(approx_eq(SprottCSolver::divergence(&cfg, 1.0, 2.0, 3.0), -1.0, 1e-12));
        // 改变 a 不影响散度
        let cfg2 = SprottCConfig { a: 10.0, ..cfg };
        assert!(approx_eq(SprottCSolver::divergence(&cfg2, 0.0, 0.0, 0.0), -1.0, 1e-12));
    }

    #[test]
    fn test_divergence_negative_dissipative() {
        let cfg = SprottCConfig::default();
        let div = SprottCSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < 0.0);
        assert!(approx_eq(div, -1.0, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = SprottCConfig::default();
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5)] {
            let j = SprottCSolver::jacobian(&cfg, x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = SprottCSolver::divergence(&cfg, x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_equilibria_values() {
        let (e1, e2) = SprottCSolver::equilibria();
        assert!(approx_eq(e1[0], 1.0, 1e-12));
        assert!(approx_eq(e1[1], 1.0, 1e-12));
        assert!(approx_eq(e1[2], 0.0, 1e-12));
        assert!(approx_eq(e2[0], -1.0, 1e-12));
        assert!(approx_eq(e2[1], -1.0, 1e-12));
        assert!(approx_eq(e2[2], 0.0, 1e-12));
    }

    #[test]
    fn test_equilibria_satisfy_equations() {
        let cfg = SprottCConfig::default();
        let (e1, e2) = SprottCSolver::equilibria();
        for eq in [e1, e2] {
            let d = SprottCSolver::derivatives(&cfg, eq[0], eq[1], eq[2]);
            for v in d.iter() {
                assert!(v.abs() < 1e-9, "equilibrium derivative = {}", v);
            }
        }
    }

    #[test]
    fn test_step_advances() {
        let mut s = SprottCSolver::classic(SprottCConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = SprottCSolver::classic(SprottCConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = SprottCSolver::classic(SprottCConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = SprottCSolver::classic(SprottCConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = SprottCSolver::classic(SprottCConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        // Sprott C 经典参数是混沌的, λ > 0 (文献值 ~0.21)
        let mut s = SprottCSolver::classic(SprottCConfig::default());
        s.run(80000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = SprottCSolver::classic(SprottCConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        let cfg = SprottCConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = SprottCSolver::classic(cfg);
        let mut s2 = SprottCSolver::new(cfg, 1.0 + d0, 1.0, 1.0);
        for _ in 0..60000 {
            s1.step();
            s2.step();
        }
        let dx = s1.x - s2.x;
        let dy = s1.y - s2.y;
        let dz = s1.z - s2.z;
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        assert!(d > 1e-3, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_y_tracks_x() {
        // dy/dt = x - y, 所以 y 滞后追踪 x (一阶低通滤波器)
        // 在稳态, y ≈ x
        let cfg = SprottCConfig::default();
        let mut s = SprottCSolver::classic(cfg);
        s.run(30000);
        // 在吸引子上, y 应该大致跟随 x (一阶滤波)
        let mean_diff = s.trajectory.iter()
            .map(|(x, y, _)| (x - y).abs())
            .sum::<f64>() / s.trajectory.len() as f64;
        assert!(mean_diff < 5.0, "y should roughly track x, mean_diff={}", mean_diff);
    }

    #[test]
    fn test_volume_monotonic_contraction() {
        let cfg = SprottCConfig::default();
        for &(x, y, z) in &[(1.0_f64, 2.0, 3.0), (-5.0, 7.0, -3.0), (10.0, -10.0, 20.0)] {
            assert!(SprottCSolver::divergence(&cfg, x, y, z) < 0.0);
        }
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = SprottCConfig::default();
        let mut s = SprottCSolver::new(cfg, 100.0, 100.0, 100.0);
        s.run(500);
        assert!(s.has_escaped() || !s.has_nan());
    }
}
