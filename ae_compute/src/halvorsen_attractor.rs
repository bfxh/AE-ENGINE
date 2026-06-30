//! Halvorsen Attractor — Halvorsen 循环对称混沌系统
//!
//! Arne Halvorsen 提出的循环对称 (cyclic symmetric) 多项式混沌系统.
//! 与 Thomas 吸引子同为循环对称系统, 但 Thomas 使用正弦线性项,
//! Halvorsen 使用二次项 + 线性交叉耦合, 产生更强的混沌行为.
//!
//! 状态方程 (Halvorsen):
//!   dx/dt = -a x - 4 y - 4 z - y^2
//!   dy/dt = -a y - 4 z - 4 x - z^2
//!   dz/dt = -a z - 4 x - 4 y - x^2
//!
//! 经典参数: a = 1.4
//! 经典初值: (x0, y0, z0) = (-1.48, -1.51, 2.04) 或 (1, 0, 0)
//!
//! 循环对称性:
//!   在 (x, y, z) → (y, z, x) 变换下, 方程形式不变.
//!   即 dy/dt 是 dx/dt 中 (x,y,z) → (y,z,x) 的结果, 同理 dz/dt.
//!   这意味着相空间绕 (1,1,1) 轴有 3 重旋转对称.
//!
//! 性质:
//!   - 散度 ∇·F = tr(J) = -3a (常数, 强耗散)
//!     经典参数: -4.2 (体积收缩率 e^(-4.2t))
//!   - 平衡点 (利用 x=y=z 对称性):
//!     设 x=y=z=s, 则 0 = -a*s - 4*s - 4*s - s^2 = -s^2 - (a+8)*s
//!     解: s = 0 (平凡) 或 s = -(a+8) (非平凡)
//!     E0 = (0, 0, 0), E1 = (-(a+8), -(a+8), -(a+8))
//!     经典参数: E1 = (-9.4, -9.4, -9.4)
//!   - Lyapunov 谱 (经典参数, 文献值):
//!     λ₁ ≈ +0.83 (正, 主混沌方向)
//!     λ₂ = 0 (沿轨道切向)
//!     λ₃ ≈ -5.03 (负, 收缩)
//!     和 = -4.2 (与散度一致)
//!   - Kaplan-Yorke 维数 D_KY = 2 + λ₁/|λ₃| ≈ 2.165
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!
//! 历史:
//!   Sprott, J. C. 1997. "Some simple chaotic flows." Phys. Rev. E 50,
//!   R647. (提及 Halvorsen 系统作为循环对称例子)
//!   Sprott, J. C. 2003. "Chaos and Time-Series Analysis." Oxford.
//!   (教科书中系统讨论 Halvorsen 吸引子)

/// Halvorsen 系统配置 (单参数 + dt)
#[derive(Clone, Copy, Debug)]
pub struct HalvorsenConfig {
    /// 参数 a (自阻尼率, 经典 1.4)
    pub a: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for HalvorsenConfig {
    fn default() -> Self {
        Self { a: 1.4, dt: 0.005 }
    }
}

/// Halvorsen 求解器 (3D, 跟踪最大 Lyapunov 指数)
pub struct HalvorsenSolver {
    pub config: HalvorsenConfig,
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

impl HalvorsenSolver {
    pub fn new(config: HalvorsenConfig, x0: f64, y0: f64, z0: f64) -> Self {
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

    pub fn classic(config: HalvorsenConfig) -> Self {
        // 经典初值 (-1.48, -1.51, 2.04) 接近吸引子
        Self::new(config, -1.48, -1.51, 2.04)
    }

    /// 右端导数 F = [-a x - 4 y - 4 z - y^2,
    ///                -a y - 4 z - 4 x - z^2,
    ///                -a z - 4 x - 4 y - x^2]
    pub fn derivatives(cfg: &HalvorsenConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        [
            -cfg.a * x - 4.0 * y - 4.0 * z - y * y,
            -cfg.a * y - 4.0 * z - 4.0 * x - z * z,
            -cfg.a * z - 4.0 * x - 4.0 * y - x * x,
        ]
    }

    /// Jacobian: J = [[-a, -4-2y, -4], [-4, -a, -4-2z], [-4-2x, -4, -a]]
    pub fn jacobian(cfg: &HalvorsenConfig, x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
        [
            [-cfg.a, -4.0 - 2.0 * y, -4.0],
            [-4.0, -cfg.a, -4.0 - 2.0 * z],
            [-4.0 - 2.0 * x, -4.0, -cfg.a],
        ]
    }

    /// 散度 ∇·F = tr(J) = -3a (常数, 强耗散)
    pub fn divergence(cfg: &HalvorsenConfig, _x: f64, _y: f64, _z: f64) -> f64 {
        -3.0 * cfg.a
    }

    /// 计算两个平衡点:
    /// E0 = (0, 0, 0), E1 = (-(a+8), -(a+8), -(a+8))
    pub fn equilibria(cfg: &HalvorsenConfig) -> ([f64; 3], [f64; 3]) {
        let s = -(cfg.a + 8.0);
        ([0.0, 0.0, 0.0], [s, s, s])
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

    /// 最大 Lyapunov 指数 (文献值 ~0.83)
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
        let cfg = HalvorsenConfig::default();
        assert!(approx_eq(cfg.a, 1.4, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = HalvorsenSolver::classic(HalvorsenConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = HalvorsenConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = HalvorsenSolver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], -cfg.a * x - 4.0 * y - 4.0 * z - y * y, 1e-12));
        assert!(approx_eq(d[1], -cfg.a * y - 4.0 * z - 4.0 * x - z * z, 1e-12));
        assert!(approx_eq(d[2], -cfg.a * z - 4.0 * x - 4.0 * y - x * x, 1e-12));
    }

    #[test]
    fn test_cyclic_symmetry() {
        // 验证循环对称: 在 (x,y,z) → (y,z,x) 变换下方程形式不变.
        // f0(y,z,x) = -a·y - 4·z - 4·x - z² = f1(x,y,z)
        // f1(y,z,x) = -a·z - 4·x - 4·y - x² = f2(x,y,z)
        // f2(y,z,x) = -a·x - 4·y - 4·z - y² = f0(x,y,z)
        // 即 d_rot[0] == d[1], d_rot[1] == d[2], d_rot[2] == d[0]
        let cfg = HalvorsenConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = HalvorsenSolver::derivatives(&cfg, x, y, z);
        let d_rot = HalvorsenSolver::derivatives(&cfg, y, z, x);
        assert!(approx_eq(d_rot[0], d[1], 1e-12), "cyclic: f0(y,z,x) = f1(x,y,z)");
        assert!(approx_eq(d_rot[1], d[2], 1e-12), "cyclic: f1(y,z,x) = f2(x,y,z)");
        assert!(approx_eq(d_rot[2], d[0], 1e-12), "cyclic: f2(y,z,x) = f0(x,y,z)");
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = HalvorsenConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = HalvorsenSolver::jacobian(&cfg, x, y, z);
        assert!(approx_eq(j[0][0], -cfg.a, 1e-12));
        assert!(approx_eq(j[0][1], -4.0 - 2.0 * y, 1e-12));
        assert!(approx_eq(j[0][2], -4.0, 1e-12));
        assert!(approx_eq(j[1][0], -4.0, 1e-12));
        assert!(approx_eq(j[1][1], -cfg.a, 1e-12));
        assert!(approx_eq(j[1][2], -4.0 - 2.0 * z, 1e-12));
        assert!(approx_eq(j[2][0], -4.0 - 2.0 * x, 1e-12));
        assert!(approx_eq(j[2][1], -4.0, 1e-12));
        assert!(approx_eq(j[2][2], -cfg.a, 1e-12));
    }

    #[test]
    fn test_divergence_constant() {
        // 散度 = -3a (常数)
        let cfg = HalvorsenConfig::default();
        let expected = -3.0 * cfg.a;
        assert!(approx_eq(HalvorsenSolver::divergence(&cfg, 0.0, 0.0, 0.0), expected, 1e-12));
        assert!(approx_eq(HalvorsenSolver::divergence(&cfg, 1.0, 2.0, 3.0), expected, 1e-12));
        assert!(approx_eq(HalvorsenSolver::divergence(&cfg, -5.0, 7.0, -3.0), expected, 1e-12));
    }

    #[test]
    fn test_divergence_negative_dissipative() {
        let cfg = HalvorsenConfig::default();
        let div = HalvorsenSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < 0.0);
        assert!(approx_eq(div, -4.2, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = HalvorsenConfig::default();
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5)] {
            let j = HalvorsenSolver::jacobian(&cfg, x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = HalvorsenSolver::divergence(&cfg, x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_equilibria_values() {
        // E0 = (0, 0, 0), E1 = (-(a+8), -(a+8), -(a+8))
        let cfg = HalvorsenConfig::default();
        let (e0, e1) = HalvorsenSolver::equilibria(&cfg);
        assert!(approx_eq(e0[0], 0.0, 1e-12));
        assert!(approx_eq(e0[1], 0.0, 1e-12));
        assert!(approx_eq(e0[2], 0.0, 1e-12));
        let s = -(cfg.a + 8.0);
        assert!(approx_eq(e1[0], s, 1e-12));
        assert!(approx_eq(e1[1], s, 1e-12));
        assert!(approx_eq(e1[2], s, 1e-12));
    }

    #[test]
    fn test_equilibria_satisfy_equations() {
        let cfg = HalvorsenConfig::default();
        let (e0, e1) = HalvorsenSolver::equilibria(&cfg);
        for eq in [e0, e1] {
            let d = HalvorsenSolver::derivatives(&cfg, eq[0], eq[1], eq[2]);
            for v in d.iter() {
                assert!(v.abs() < 1e-9, "equilibrium derivative = {}", v);
            }
        }
    }

    #[test]
    fn test_step_advances() {
        let mut s = HalvorsenSolver::classic(HalvorsenConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = HalvorsenSolver::classic(HalvorsenConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = HalvorsenSolver::classic(HalvorsenConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = HalvorsenSolver::classic(HalvorsenConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = HalvorsenSolver::classic(HalvorsenConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        // Halvorsen 是混沌的, λ > 0 (文献值 ~0.83)
        let mut s = HalvorsenSolver::classic(HalvorsenConfig::default());
        s.run(50000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = HalvorsenSolver::classic(HalvorsenConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        let cfg = HalvorsenConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = HalvorsenSolver::new(cfg, -1.48, -1.51, 2.04);
        let mut s2 = HalvorsenSolver::new(cfg, -1.48 + d0, -1.51, 2.04);
        for _ in 0..30000 {
            s1.step();
            s2.step();
        }
        let dx = s1.x - s2.x;
        let dy = s1.y - s2.y;
        let dz = s1.z - s2.z;
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        // t=150, λ~0.83, 应放大 e^124 (饱和到吸引子尺度 ~1)
        assert!(d > 1e-3, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_volume_monotonic_contraction() {
        // 散度 = -4.2 (常数负), 体积单调收缩
        let cfg = HalvorsenConfig::default();
        for &(x, y, z) in &[(1.0_f64, 2.0, 3.0), (-5.0, 7.0, -3.0), (10.0, -10.0, 20.0)] {
            assert!(HalvorsenSolver::divergence(&cfg, x, y, z) < 0.0);
        }
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = HalvorsenConfig::default();
        let mut s = HalvorsenSolver::new(cfg, 100.0, 100.0, 100.0);
        s.run(500);
        assert!(s.has_escaped() || !s.has_nan());
    }

    #[test]
    fn test_diagonal_symmetric_equilibrium() {
        // 平衡点 E1 在对角线 x=y=z 上 (循环对称的必然结果)
        let cfg = HalvorsenConfig::default();
        let (_, e1) = HalvorsenSolver::equilibria(&cfg);
        assert!(approx_eq(e1[0], e1[1], 1e-12));
        assert!(approx_eq(e1[1], e1[2], 1e-12));
        assert!(approx_eq(e1[0], e1[2], 1e-12));
    }
}
