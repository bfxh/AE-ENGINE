//! Sprott B Attractor — Sprott B 简单混沌系统 (3D)
//!
//! Julien C. Sprott 1994 年在其论文 "Some simple chaotic flows" 中
//! 系统搜索发现的最简单混沌系统之一 (Sprott A-S, 共 19 个). Sprott B
//! 是其中第二个, 以极少的非线性项产生混沌行为.
//!
//! 与 Sprott C 的对比:
//!   Sprott B: dx/dt = y·z,  dy/dt = x - y,  dz/dt = 1 - x·y
//!   Sprott C: dx/dt = y·z,  dy/dt = x - y,  dz/dt = 1 - x²
//!   两者仅第三项不同 (xy vs x²), 但动力学有微妙差异:
//!   - Sprott B 散度 = -1 (常数, 均匀收缩)
//!   - Sprott C 散度 = -1 (常数, 均匀收缩)
//!   - Sprott B 的非线性项 xy (双变量耦合)
//!   - Sprott C 的非线性项 x² (单变量平方)
//!
//! 状态方程 (Sprott B 1994, 参数化版本):
//!   dx/dt = a·y·z
//!   dy/dt = x - y
//!   dz/dt = 1 - x·y
//!
//! 各项物理意义:
//!   + a·y·z: 非线性耦合 (x 受 yz 乘积驱动)
//!   + x: 线性驱动 (x 推动 y)
//!   - y: 线性阻尼 (y 衰减, 一阶低通滤波)
//!   + 1: 常数驱动 (z 偏置)
//!   - x·y: 非线性反馈 (z 受 xy 乘积调制, 双变量耦合)
//!
//! 经典参数 (Sprott 1994): a = 1 (原始无参数版)
//! 经典初值: (x₀, y₀, z₀) = (0.05, 0.05, 0.05) 或 (1, 1, 1)
//!
//! 性质:
//!   - 散度 ∇·F = tr(J) = -1 (常数负, 耗散)
//!     体积收缩率 e^(-t), 中等耗散
//!   - 平衡点 (利用 y=x, z=0, xy=1):
//!     dx/dt = a·y·z = 0 → y=0 或 z=0
//!     若 z=0: dy/dt = x-y = 0 → y=x
//!            dz/dt = 1-xy = 0 → x²=1 → x=±1
//!     E1 = (1, 1, 0), E2 = (-1, -1, 0)
//!   - Lyapunov 谱 (a=1, 文献值):
//!     λ₁ ≈ +0.21  (正, 主混沌方向)
//!     λ₂ = 0      (沿轨道切向)
//!     λ₃ ≈ -1.21  (负, 收缩)
//!     和 = -1 (与散度一致)
//!   - Kaplan-Yorke 维数 D_KY ≈ 2 + λ₁/|λ₃| ≈ 2.17
//!   - 吸引子形态: 单叶扭曲带
//!
//! Sprott 系列对比 (已实现):
//!   - Sprott A (sprott_attractor): 1 个非线性项 (yz)
//!   - Sprott B (本模块): 2 个非线性项 (yz, xy)
//!   - Sprott C (sprott_c_attractor): 2 个非线性项 (yz, x²)
//!   - Sprott B vs C: 仅第三项不同 (xy vs x²), 均常数散度 -1
//!
//! 简约性意义:
//!   Sprott B 证明了即使 2 个非线性项 (yz, xy) 也能产生混沌.
//!   与 Sprott C (yz, x²) 对比, 展示了不同非线性耦合方式的等价混沌性.
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!
//! 历史:
//!   Sprott, J. C. 1994. "Some simple chaotic flows." Phys. Rev. E 50,
//!   R647-R650. (原始论文, 19 个最简单混沌系统)
//!   Sprott, J. C. 2003. "Chaos and Time-Series Analysis." Oxford.

/// Sprott B 系统配置 (1 参数 + dt)
#[derive(Clone, Copy, Debug)]
pub struct SprottBConfig {
    /// 非线性耦合强度 a (经典 1)
    pub a: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for SprottBConfig {
    fn default() -> Self {
        Self { a: 1.0, dt: 0.01 }
    }
}

/// Sprott B 求解器 (3D, 跟踪最大 Lyapunov 指数)
pub struct SprottBSolver {
    pub config: SprottBConfig,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub time: f64,
    pub step_count: u64,
    pub trajectory: Vec<(f64, f64, f64)>,
    pub lyap_sum: f64,
    pub v: [f64; 3],
}

impl SprottBSolver {
    pub fn new(config: SprottBConfig, x0: f64, y0: f64, z0: f64) -> Self {
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

    pub fn classic(config: SprottBConfig) -> Self {
        Self::new(config, 0.5, 0.5, 0.5)
    }

    /// 右端导数 F = [a·y·z, x - y, 1 - x·y]
    pub fn derivatives(cfg: &SprottBConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        [cfg.a * y * z, x - y, 1.0 - x * y]
    }

    /// Jacobian:
    /// J = [[0,    a·z, a·y],
    ///      [1,   -1,   0  ],
    ///      [-y,  -x,   0  ]]
    pub fn jacobian(cfg: &SprottBConfig, x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
        [
            [0.0, cfg.a * z, cfg.a * y],
            [1.0, -1.0, 0.0],
            [-y, -x, 0.0],
        ]
    }

    /// 散度 ∇·F = tr(J) = -1 (常数负)
    pub fn divergence(_cfg: &SprottBConfig, _x: f64, _y: f64, _z: f64) -> f64 {
        -1.0
    }

    /// 计算两个平衡点: E1 = (1, 1, 0), E2 = (-1, -1, 0)
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
        let cfg = SprottBConfig::default();
        assert!(approx_eq(cfg.a, 1.0, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = SprottBSolver::classic(SprottBConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = SprottBConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = SprottBSolver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], cfg.a * y * z, 1e-12));
        assert!(approx_eq(d[1], x - y, 1e-12));
        assert!(approx_eq(d[2], 1.0 - x * y, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = SprottBConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = SprottBSolver::jacobian(&cfg, x, y, z);
        // Row 0: [0, a·z, a·y]
        assert!(approx_eq(j[0][0], 0.0, 1e-12));
        assert!(approx_eq(j[0][1], cfg.a * z, 1e-12));
        assert!(approx_eq(j[0][2], cfg.a * y, 1e-12));
        // Row 1: [1, -1, 0]
        assert!(approx_eq(j[1][0], 1.0, 1e-12));
        assert!(approx_eq(j[1][1], -1.0, 1e-12));
        assert!(approx_eq(j[1][2], 0.0, 1e-12));
        // Row 2: [-y, -x, 0]
        assert!(approx_eq(j[2][0], -y, 1e-12));
        assert!(approx_eq(j[2][1], -x, 1e-12));
        assert!(approx_eq(j[2][2], 0.0, 1e-12));
    }

    #[test]
    fn test_divergence_constant() {
        // 散度 = -1 (常数, 与参数和位置无关)
        let cfg = SprottBConfig::default();
        assert!(approx_eq(SprottBSolver::divergence(&cfg, 0.0, 0.0, 0.0), -1.0, 1e-12));
        assert!(approx_eq(SprottBSolver::divergence(&cfg, 1.0, 2.0, 3.0), -1.0, 1e-12));
        assert!(approx_eq(SprottBSolver::divergence(&cfg, -5.0, 7.0, -3.0), -1.0, 1e-12));
        // 改参数也应不变
        let cfg2 = SprottBConfig { a: 2.0, dt: 0.01 };
        assert!(approx_eq(SprottBSolver::divergence(&cfg2, 0.0, 0.0, 0.0), -1.0, 1e-12));
    }

    #[test]
    fn test_divergence_negative_dissipative() {
        let cfg = SprottBConfig::default();
        let div = SprottBSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < 0.0);
        assert!(approx_eq(div, -1.0, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = SprottBConfig::default();
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5), (2.0, -1.0, 0.3)] {
            let j = SprottBSolver::jacobian(&cfg, x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = SprottBSolver::divergence(&cfg, x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_equilibria_values() {
        let (e1, e2) = SprottBSolver::equilibria();
        assert!(approx_eq(e1[0], 1.0, 1e-12));
        assert!(approx_eq(e1[1], 1.0, 1e-12));
        assert!(approx_eq(e1[2], 0.0, 1e-12));
        assert!(approx_eq(e2[0], -1.0, 1e-12));
        assert!(approx_eq(e2[1], -1.0, 1e-12));
        assert!(approx_eq(e2[2], 0.0, 1e-12));
    }

    #[test]
    fn test_equilibria_satisfy_equations() {
        let cfg = SprottBConfig::default();
        let (e1, e2) = SprottBSolver::equilibria();
        for eq in [e1, e2] {
            let d = SprottBSolver::derivatives(&cfg, eq[0], eq[1], eq[2]);
            for v in d.iter() {
                assert!(v.abs() < 1e-9, "equilibrium derivative = {}", v);
            }
        }
    }

    #[test]
    fn test_step_advances() {
        let mut s = SprottBSolver::classic(SprottBConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = SprottBSolver::classic(SprottBConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_y_tracks_x() {
        // dy/dt = x - y 是一阶低通滤波, y 跟踪 x
        let cfg = SprottBConfig::default();
        let mut s = SprottBSolver::classic(cfg);
        s.run(1000);
        let diff = (s.x - s.y).abs();
        assert!(diff < 5.0, "y should track x: |x-y|={}", diff);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = SprottBSolver::classic(SprottBConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = SprottBSolver::classic(SprottBConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = SprottBSolver::classic(SprottBConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        // Sprott B 经典参数是混沌的, λ > 0 (文献值 ~0.21)
        let mut s = SprottBSolver::classic(SprottBConfig::default());
        s.run(100000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = SprottBSolver::classic(SprottBConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        // 混沌系统: 微小扰动指数放大
        let cfg = SprottBConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = SprottBSolver::classic(cfg);
        let mut s2 = SprottBSolver::new(cfg, 0.5 + d0, 0.5, 0.5);
        s1.run(50000);
        s2.run(50000);
        let d = ((s1.x - s2.x).powi(2) + (s1.y - s2.y).powi(2) + (s1.z - s2.z).powi(2)).sqrt();
        assert!(d > 1e-3, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_escape_for_large_initial() {
        let mut s = SprottBSolver::new(SprottBConfig::default(), 1000.0, 1000.0, 1000.0);
        s.run(1000);
        assert!(s.has_escaped(), "should escape: x={} y={} z={}", s.x, s.y, s.z);
    }
}
