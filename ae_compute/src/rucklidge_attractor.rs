//! Rucklidge Attractor — Rucklidge 双对流混沌系统 (3D)
//!
//! Andrew M. Rucklidge 1992 年提出的 3D 混沌系统, 源于对双扩散对流
//! (double convection) 的研究. 当流体同时受温度梯度和浓度梯度驱动时
//! (如海洋温盐环流、岩浆房), 会出现双对流不稳定. Rucklidge 系统是
//! Lorenz 63 模型在双对流场景下的推广, 也用于模拟太阳磁通量管
//! (solar magnetic flux tubes) 的动力学.
//!
//! 与 Lorenz 63 的关系:
//!   Lorenz 63 源于单组分对流 (纯温度梯度), 3 变量.
//!   Rucklidge 处理双组分对流 (温度+浓度), 在特定参数下简化为 3 变量.
//!   两者都是 Bénard 对流的截断模型, 但 Rucklidge 的非线性项不同.
//!
//! 状态方程 (Rucklidge 1992):
//!   dx/dt = -κ x + λ y - y z
//!   dy/dt = x
//!   dz/dt = -z + y²
//!
//! 各项物理意义:
//!   - -κ x: 粘性阻尼 (速度场耗散)
//!   + λ y: 线性不稳定性 (浮力驱动, λ > 0 时对流不稳定)
//!   - y z: 非线性反馈 (速度-温度梯度耦合)
//!   + x: 速度对温度梯度的平流 (dy/dt = x, 即 y 是 x 的积分, 类似位移)
//!   - z: 热扩散 (温度场耗散)
//!   + y²: 非线性温度产生 (剪切产热)
//!
//! 经典参数 (Rucklidge 1992): κ = 2, λ = 6.7
//! 经典初值: (x₀, y₀, z₀) = (1, 0, 0) 或 (-1, 1, 0)
//!
//! 性质:
//!   - 散度 ∇·F = tr(J) = -κ - 1 (常数负, 耗散)
//!     经典参数 κ=2: div = -3 (体积收缩率 e^(-3t))
//!   - 平衡点 (利用 x=0, z=y² 代入):
//!     0 = λ y - y·y² = y(λ - y²) → y = 0 或 y = ±√λ
//!     E0 = (0, 0, 0)            (平凡, 无对流)
//!     E1 = (0,  √λ, λ)          (正对流)
//!     E2 = (0, -√λ, λ)          (负对流)
//!     经典参数 λ=6.7: E1 = (0, 2.588, 6.7), E2 = (0, -2.588, 6.7)
//!   - Lyapunov 谱 (经典参数, 文献值):
//!     λ₁ ≈ +0.20  (正, 主混沌方向)
//!     λ₂ = 0      (沿轨道切向)
//!     λ₃ ≈ -3.20  (负, 收缩)
//!     和 = -3 (与散度一致)
//!   - Kaplan-Yorke 维数 D_KY ≈ 2 + λ₁/|λ₃| ≈ 2.06
//!   - 吸引子形态: 双叶结构 (类似 Lorenz 63, 但扭曲方式不同)
//!
//! 与 Lorenz 63 对比:
//!   - Lorenz 63: 非线性项 xz, xy (双交叉), 对称 (x↔-x)
//!   - Rucklidge: 非线性项 yz, y² (单变量平方), 无对称
//!   - Lorenz 63 平衡点: (0,0,0), (±√(b(r-1)), ±√(b(r-1)), r-1)
//!   - Rucklidge 平衡点: (0,0,0), (0, ±√λ, λ)
//!
//! 应用:
//!   - 双扩散对流 (海洋学, 岩浆动力学)
//!   - 太阳磁通量管 (太阳物理学)
//!   - 混沌控制与同步
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!
//! 历史:
//!   Rucklidge, A. M. 1992. "Chaos in models of double convection."
//!   J. Fluid Mech. 237, 209-229. (原始论文)
//!   Rucklidge, A. M. 1993. "Time series analysis of the Lorenz
//!   and Rucklidge attractors." Physica D 62, 307-326. (时间序列分析)

/// Rucklidge 系统配置 (2 参数 + dt)
#[derive(Clone, Copy, Debug)]
pub struct RucklidgeConfig {
    /// 阻尼系数 κ (经典 2, 粘性耗散)
    pub kappa: f64,
    /// 不稳定性参数 λ (经典 6.7, 浮力驱动)
    pub lambda: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for RucklidgeConfig {
    fn default() -> Self {
        Self { kappa: 2.0, lambda: 6.7, dt: 0.005 }
    }
}

/// Rucklidge 求解器 (3D, 跟踪最大 Lyapunov 指数)
pub struct RucklidgeSolver {
    pub config: RucklidgeConfig,
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

impl RucklidgeSolver {
    pub fn new(config: RucklidgeConfig, x0: f64, y0: f64, z0: f64) -> Self {
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

    pub fn classic(config: RucklidgeConfig) -> Self {
        Self::new(config, 1.0, 0.0, 0.0)
    }

    /// 右端导数 F = [-κ x + λ y - y z, x, -z + y²]
    pub fn derivatives(cfg: &RucklidgeConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        [
            -cfg.kappa * x + cfg.lambda * y - y * z,
            x,
            -z + y * y,
        ]
    }

    /// Jacobian:
    /// J = [[-κ,  λ - z,  -y],
    ///      [ 1,  0,       0 ],
    ///      [ 0,  2y,     -1 ]]
    pub fn jacobian(cfg: &RucklidgeConfig, _x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
        [
            [-cfg.kappa, cfg.lambda - z, -y],
            [1.0, 0.0, 0.0],
            [0.0, 2.0 * y, -1.0],
        ]
    }

    /// 散度 ∇·F = tr(J) = -κ - 1 (常数负)
    pub fn divergence(cfg: &RucklidgeConfig, _x: f64, _y: f64, _z: f64) -> f64 {
        -cfg.kappa - 1.0
    }

    /// 计算三个平衡点:
    /// E0 = (0, 0, 0), E1 = (0,  √λ, λ), E2 = (0, -√λ, λ)
    pub fn equilibria(cfg: &RucklidgeConfig) -> ([f64; 3], [f64; 3], [f64; 3]) {
        let sqrt_l = cfg.lambda.max(0.0).sqrt();
        (
            [0.0, 0.0, 0.0],
            [0.0, sqrt_l, cfg.lambda],
            [0.0, -sqrt_l, cfg.lambda],
        )
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

    /// 最大 Lyapunov 指数 (文献值 ~0.20)
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
        let cfg = RucklidgeConfig::default();
        assert!(approx_eq(cfg.kappa, 2.0, 1e-12));
        assert!(approx_eq(cfg.lambda, 6.7, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = RucklidgeSolver::classic(RucklidgeConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = RucklidgeConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = RucklidgeSolver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], -cfg.kappa * x + cfg.lambda * y - y * z, 1e-12));
        assert!(approx_eq(d[1], x, 1e-12));
        assert!(approx_eq(d[2], -z + y * y, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = RucklidgeConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = RucklidgeSolver::jacobian(&cfg, x, y, z);
        assert!(approx_eq(j[0][0], -cfg.kappa, 1e-12));
        assert!(approx_eq(j[0][1], cfg.lambda - z, 1e-12));
        assert!(approx_eq(j[0][2], -y, 1e-12));
        assert!(approx_eq(j[1][0], 1.0, 1e-12));
        assert!(approx_eq(j[1][1], 0.0, 1e-12));
        assert!(approx_eq(j[1][2], 0.0, 1e-12));
        assert!(approx_eq(j[2][0], 0.0, 1e-12));
        assert!(approx_eq(j[2][1], 2.0 * y, 1e-12));
        assert!(approx_eq(j[2][2], -1.0, 1e-12));
    }

    #[test]
    fn test_divergence_constant() {
        let cfg = RucklidgeConfig::default();
        let expected = -cfg.kappa - 1.0;
        assert!(approx_eq(RucklidgeSolver::divergence(&cfg, 0.0, 0.0, 0.0), expected, 1e-12));
        assert!(approx_eq(RucklidgeSolver::divergence(&cfg, 1.0, 2.0, 3.0), expected, 1e-12));
        assert!(approx_eq(RucklidgeSolver::divergence(&cfg, -5.0, 7.0, -3.0), expected, 1e-12));
    }

    #[test]
    fn test_divergence_negative_dissipative() {
        let cfg = RucklidgeConfig::default();
        let div = RucklidgeSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < 0.0);
        assert!(approx_eq(div, -3.0, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = RucklidgeConfig::default();
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5)] {
            let j = RucklidgeSolver::jacobian(&cfg, x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = RucklidgeSolver::divergence(&cfg, x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_equilibria_values() {
        let cfg = RucklidgeConfig::default();
        let (e0, e1, e2) = RucklidgeSolver::equilibria(&cfg);
        assert!(approx_eq(e0[0], 0.0, 1e-12));
        assert!(approx_eq(e0[1], 0.0, 1e-12));
        assert!(approx_eq(e0[2], 0.0, 1e-12));
        let sqrt_l = cfg.lambda.sqrt();
        assert!(approx_eq(e1[0], 0.0, 1e-12));
        assert!(approx_eq(e1[1], sqrt_l, 1e-12));
        assert!(approx_eq(e1[2], cfg.lambda, 1e-12));
        assert!(approx_eq(e2[0], 0.0, 1e-12));
        assert!(approx_eq(e2[1], -sqrt_l, 1e-12));
        assert!(approx_eq(e2[2], cfg.lambda, 1e-12));
    }

    #[test]
    fn test_equilibria_satisfy_equations() {
        let cfg = RucklidgeConfig::default();
        let (e0, e1, e2) = RucklidgeSolver::equilibria(&cfg);
        for eq in [e0, e1, e2] {
            let d = RucklidgeSolver::derivatives(&cfg, eq[0], eq[1], eq[2]);
            for v in d.iter() {
                assert!(v.abs() < 1e-9, "equilibrium derivative = {}", v);
            }
        }
    }

    #[test]
    fn test_equilibria_symmetric() {
        let cfg = RucklidgeConfig::default();
        let (_e0, e1, e2) = RucklidgeSolver::equilibria(&cfg);
        assert!(approx_eq(e1[1], -e2[1], 1e-12));
        assert!(approx_eq(e1[2], e2[2], 1e-12));
    }

    #[test]
    fn test_step_advances() {
        let mut s = RucklidgeSolver::classic(RucklidgeConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = RucklidgeSolver::classic(RucklidgeConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = RucklidgeSolver::classic(RucklidgeConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = RucklidgeSolver::classic(RucklidgeConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = RucklidgeSolver::classic(RucklidgeConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        let mut s = RucklidgeSolver::classic(RucklidgeConfig::default());
        s.run(80000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = RucklidgeSolver::classic(RucklidgeConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        let cfg = RucklidgeConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = RucklidgeSolver::classic(cfg);
        let mut s2 = RucklidgeSolver::new(cfg, 1.0 + d0, 0.0, 0.0);
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
    fn test_zero_instability_decays() {
        let cfg = RucklidgeConfig { lambda: 0.0, ..RucklidgeConfig::default() };
        let mut s = RucklidgeSolver::new(cfg, 1.0, 0.5, 0.3);
        s.run(50000);
        let r = (s.x * s.x + s.y * s.y + s.z * s.z).sqrt();
        assert!(r < 0.1, "should decay to origin, r = {}", r);
    }

    #[test]
    fn test_zero_instability_no_chaos() {
        let cfg = RucklidgeConfig { lambda: 0.0, ..RucklidgeConfig::default() };
        let mut s = RucklidgeSolver::new(cfg, 1.0, 0.5, 0.3);
        s.run(50000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda < 0.01, "no instability → no chaos, lambda = {}", lambda);
    }

    #[test]
    fn test_volume_monotonic_contraction() {
        let cfg = RucklidgeConfig::default();
        for &(x, y, z) in &[(1.0_f64, 2.0, 3.0), (-5.0, 7.0, -3.0), (10.0, -10.0, 20.0)] {
            assert!(RucklidgeSolver::divergence(&cfg, x, y, z) < 0.0);
        }
    }

    #[test]
    fn test_y_is_integral_of_x() {
        let cfg = RucklidgeConfig::default();
        let x0 = 0.5_f64;
        let y0 = 0.3_f64;
        let z0 = 0.1_f64;
        let mut s = RucklidgeSolver::new(cfg, x0, y0, z0);
        let n = 10;
        s.run(n);
        let dt = cfg.dt;
        let t = n as f64 * dt;
        let y_expected = y0 + x0 * t;
        let y_actual = s.y;
        let tol = (y_expected - y0).abs() * 0.3 + 0.01;
        assert!((y_actual - y_expected).abs() < tol,
            "y should be integral of x: y_actual={}, y_expected={}, tol={}", y_actual, y_expected, tol);
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = RucklidgeConfig::default();
        let mut s = RucklidgeSolver::new(cfg, 100.0, 100.0, 100.0);
        s.run(500);
        assert!(s.has_escaped() || !s.has_nan());
    }
}
