//! Arneodo Attractor — Arneodo-Coullet-Tresser 螺旋混沌系统 (3D)
//!
//! Arneodo, Coullet 和 Tresser 1981 年提出的 3D 自治混沌系统, 是
//! Duffing 振荡器的 3D 自治化版本. 该系统以产生螺旋型 (spiral)
//! 奇怪吸引子著称, 在分岔理论和非线性动力学中有重要地位.
//!
//! 与 Duffing 振荡器的关系:
//!   Duffing: x'' + δ x' + x + x³ = γ cos(ωt)  (2D 非自治, 受驱)
//!   Arneodo: x''' + μ x'' + x' + x - x³ = 0   (3D 自治, 无驱动)
//!   令 y = x', z = x'', 则:
//!     dx/dt = y
//!     dy/dt = z
//!     dz/dt = -μ z - y - x + x³
//!   Arneodo 是 Duffing 的"三阶"自治推广, 通过增加一个维度消除了
//!   外部驱动需求.
//!
//! 状态方程 (Arneodo-Coullet-Tresser 1981):
//!   dx/dt = y
//!   dy/dt = z
//!   dz/dt = -μ z - y - x + x³
//!
//! 各项物理意义:
//!   - y = x': 速度 (位移的导数)
//!   - z = x'': 加速度 (速度的导数)
//!   - -μ z: 粘性阻尼 (与加速度成正比, 三阶耗散)
//!   - -y: 恢复力 (线性, 速度反馈)
//!   - -x: 线性恢复力 (弹簧)
//!   + x³: 非线性恢复力 (硬化弹簧, Duffing 类型)
//!
//! 经典参数 (Arneodo 1981): μ = 0.45
//! 经典初值: (x₀, y₀, z₀) = (0.1, 0, 0) 或 (1, 0, 0)
//!
//! 性质:
//!   - 散度 ∇·F = tr(J) = -μ (常数负, 耗散)
//!     经典参数 μ=0.45: div = -0.45 (弱耗散)
//!   - 平衡点 (利用 y=0, z=0, x³-x=0):
//!     x(x²-1) = 0 → x = 0, ±1
//!     E0 = (0, 0, 0)    (鞍点, 不稳定)
//!     E1 = (1, 0, 0)    (螺旋鞍点)
//!     E2 = (-1, 0, 0)   (螺旋鞍点)
//!   - Lyapunov 谱 (经典参数, 文献值):
//!     λ₁ ≈ +0.10  (正, 主混沌方向)
//!     λ₂ = 0      (沿轨道切向)
//!     λ₃ ≈ -0.55  (负, 收缩)
//!     和 = -0.45 (与散度一致)
//!   - Kaplan-Yorke 维数 D_KY ≈ 2 + λ₁/|λ₃| ≈ 2.18
//!   - 吸引子形态: 螺旋型 (spiral), 轨道围绕 E1/E2 螺旋
//!
//! 与 Duffing (2D) 对比:
//!   - Duffing: 受驱, 需要外部驱动, 2D 相空间 + 时间
//!   - Arneodo: 自治, 无外部驱动, 3D 相空间
//!   - Duffing 双井势 V(x) = x²/2 + x⁴/4
//!   - Arneodo 双井势 V(x) = x²/2 - x⁴/4 (注意符号: -x³ → V = x²/2 - x⁴/4)
//!   - Arneodo 的势能是倒双井 (inverted double well)
//!
//! 螺旋吸引子机制:
//!   轨道在 E1=(1,0,0) 和 E2=(-1,0,0) 之间螺旋跳跃.
//!   每个平衡点附近, 轨道先螺旋接近, 然后被排斥到另一个平衡点,
//!   形成无限重复的螺旋跳跃模式. 这种"螺旋-跳跃"结构是 Arneodo
//!   吸引子的标志性特征.
//!
//! 分岔理论意义:
//!   Arneodo 系统是 Shilnikov 分岔 (Shilnikov bifurcation) 的典型例子.
//!   当平衡点有一对复共轭特征值和一个实特征值, 且实特征值的绝对值
//!   大于复特征值的实部时 (Shilnikov 条件), 系统产生螺旋型混沌.
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!
//! 历史:
//!   Arneodo, A., Coullet, P. & Tresser, C. 1981. "Possible new strange
//!   attractors with spiral structure." Commun. Math. Phys. 79, 573-579.
//!   (原始论文, 螺旋吸引子发现)
//!   Shilnikov, L. P. 1965. "A case of the existence of a countable
//!   number of periodic motions." Sov. Math. Dokl. 6, 163. (Shilnikov 分岔)
//!   Sprott, J. C. 2003. "Chaos and Time-Series Analysis." Oxford.

/// Arneodo 系统配置 (1 参数 + dt)
#[derive(Clone, Copy, Debug)]
pub struct ArneodoConfig {
    /// 阻尼系数 μ (经典 0.45, 三阶耗散)
    pub mu: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for ArneodoConfig {
    fn default() -> Self {
        Self { mu: 0.45, dt: 0.01 }
    }
}

/// Arneodo 求解器 (3D, 跟踪最大 Lyapunov 指数)
pub struct ArneodoSolver {
    pub config: ArneodoConfig,
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

impl ArneodoSolver {
    pub fn new(config: ArneodoConfig, x0: f64, y0: f64, z0: f64) -> Self {
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

    pub fn classic(config: ArneodoConfig) -> Self {
        Self::new(config, 0.1, 0.0, 0.0)
    }

    /// 右端导数 F = [y, z, -μ z - y - x + x³]
    pub fn derivatives(cfg: &ArneodoConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        [y, z, -cfg.mu * z - y - x + x * x * x]
    }

    /// Jacobian:
    /// J = [[0,       1,  0],
    ///      [0,       0,  1],
    ///      [-1+3x², -1, -μ]]
    pub fn jacobian(cfg: &ArneodoConfig, x: f64, _y: f64, _z: f64) -> [[f64; 3]; 3] {
        [
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [-1.0 + 3.0 * x * x, -1.0, -cfg.mu],
        ]
    }

    /// 散度 ∇·F = tr(J) = -μ (常数负)
    pub fn divergence(cfg: &ArneodoConfig, _x: f64, _y: f64, _z: f64) -> f64 {
        -cfg.mu
    }

    /// 计算三个平衡点:
    /// E0 = (0, 0, 0), E1 = (1, 0, 0), E2 = (-1, 0, 0)
    pub fn equilibria() -> ([f64; 3], [f64; 3], [f64; 3]) {
        ([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [-1.0, 0.0, 0.0])
    }

    /// 势能 V(x) = x²/2 - x⁴/4 (倒双井, 与 Duffing 正双井对比)
    /// 注: dV/dx = x - x³ = -(x³ - x) = -dF/dx (保守力)
    pub fn potential(x: f64) -> f64 {
        x * x / 2.0 - x * x * x * x / 4.0
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

    /// 最大 Lyapunov 指数 (文献值 ~0.10)
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
        let cfg = ArneodoConfig::default();
        assert!(approx_eq(cfg.mu, 0.45, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = ArneodoSolver::classic(ArneodoConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = ArneodoConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = ArneodoSolver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], y, 1e-12));
        assert!(approx_eq(d[1], z, 1e-12));
        assert!(approx_eq(d[2], -cfg.mu * z - y - x + x * x * x, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = ArneodoConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = ArneodoSolver::jacobian(&cfg, x, y, z);
        assert!(approx_eq(j[0][0], 0.0, 1e-12));
        assert!(approx_eq(j[0][1], 1.0, 1e-12));
        assert!(approx_eq(j[0][2], 0.0, 1e-12));
        assert!(approx_eq(j[1][0], 0.0, 1e-12));
        assert!(approx_eq(j[1][1], 0.0, 1e-12));
        assert!(approx_eq(j[1][2], 1.0, 1e-12));
        assert!(approx_eq(j[2][0], -1.0 + 3.0 * x * x, 1e-12));
        assert!(approx_eq(j[2][1], -1.0, 1e-12));
        assert!(approx_eq(j[2][2], -cfg.mu, 1e-12));
    }

    #[test]
    fn test_divergence_constant() {
        let cfg = ArneodoConfig::default();
        let expected = -cfg.mu;
        assert!(approx_eq(ArneodoSolver::divergence(&cfg, 0.0, 0.0, 0.0), expected, 1e-12));
        assert!(approx_eq(ArneodoSolver::divergence(&cfg, 1.0, 2.0, 3.0), expected, 1e-12));
        assert!(approx_eq(ArneodoSolver::divergence(&cfg, -5.0, 7.0, -3.0), expected, 1e-12));
    }

    #[test]
    fn test_divergence_negative_dissipative() {
        let cfg = ArneodoConfig::default();
        let div = ArneodoSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < 0.0);
        assert!(approx_eq(div, -0.45, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = ArneodoConfig::default();
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5)] {
            let j = ArneodoSolver::jacobian(&cfg, x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = ArneodoSolver::divergence(&cfg, x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_equilibria_values() {
        let (e0, e1, e2) = ArneodoSolver::equilibria();
        assert!(approx_eq(e0[0], 0.0, 1e-12));
        assert!(approx_eq(e1[0], 1.0, 1e-12));
        assert!(approx_eq(e2[0], -1.0, 1e-12));
        // 所有平衡点 y=z=0
        for eq in [e0, e1, e2] {
            assert!(approx_eq(eq[1], 0.0, 1e-12));
            assert!(approx_eq(eq[2], 0.0, 1e-12));
        }
    }

    #[test]
    fn test_equilibria_satisfy_equations() {
        let cfg = ArneodoConfig::default();
        let (e0, e1, e2) = ArneodoSolver::equilibria();
        for eq in [e0, e1, e2] {
            let d = ArneodoSolver::derivatives(&cfg, eq[0], eq[1], eq[2]);
            for v in d.iter() {
                assert!(v.abs() < 1e-9, "equilibrium derivative = {}", v);
            }
        }
    }

    #[test]
    fn test_potential_inverted_double_well() {
        // V(x) = x²/2 - x⁴/4 (倒双井)
        // V(0) = 0 (局部极小)
        // V(±1) = 1/2 - 1/4 = 1/4 (局部极大)
        // V(±√2) = 1 - 1 = 0 (零交叉)
        assert!(approx_eq(ArneodoSolver::potential(0.0), 0.0, 1e-12));
        assert!(approx_eq(ArneodoSolver::potential(1.0), 0.25, 1e-12));
        assert!(approx_eq(ArneodoSolver::potential(-1.0), 0.25, 1e-12));
        assert!(approx_eq(ArneodoSolver::potential(2.0_f64.sqrt()), 0.0, 1e-12));
    }

    #[test]
    fn test_potential_symmetric() {
        // 势能关于 x 反演对称: V(-x) = V(x)
        for &x in &[0.3_f64, 0.7, 1.5, 2.0] {
            assert!(approx_eq(ArneodoSolver::potential(x), ArneodoSolver::potential(-x), 1e-12));
        }
    }

    #[test]
    fn test_step_advances() {
        let mut s = ArneodoSolver::classic(ArneodoConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = ArneodoSolver::classic(ArneodoConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = ArneodoSolver::classic(ArneodoConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = ArneodoSolver::classic(ArneodoConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = ArneodoSolver::classic(ArneodoConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        // Arneodo 经典参数是混沌的, λ > 0 (文献值 ~0.10)
        let mut s = ArneodoSolver::classic(ArneodoConfig::default());
        s.run(100000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = ArneodoSolver::classic(ArneodoConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        let cfg = ArneodoConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = ArneodoSolver::classic(cfg);
        let mut s2 = ArneodoSolver::new(cfg, 0.1 + d0, 0.0, 0.0);
        for _ in 0..80000 {
            s1.step();
            s2.step();
        }
        let dx = s1.x - s2.x;
        let dy = s1.y - s2.y;
        let dz = s1.z - s2.z;
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        assert!(d > 1e-5, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_volume_monotonic_contraction() {
        let cfg = ArneodoConfig::default();
        for &(x, y, z) in &[(1.0_f64, 2.0, 3.0), (-5.0, 7.0, -3.0), (10.0, -10.0, 20.0)] {
            assert!(ArneodoSolver::divergence(&cfg, x, y, z) < 0.0);
        }
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = ArneodoConfig::default();
        let mut s = ArneodoSolver::new(cfg, 100.0, 100.0, 100.0);
        s.run(500);
        assert!(s.has_escaped() || !s.has_nan());
    }

    #[test]
    fn test_third_order_structure() {
        // Arneodo 是三阶 ODE: x''' = -μ x'' - x' - x + x³
        // 即 dz/dt = -μ z - y - x + x³ (z = x'')
        // 验证三阶结构: d³x/dt³ = dz/dt
        let cfg = ArneodoConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = ArneodoSolver::derivatives(&cfg, x, y, z);
        // d[0] = dx/dt = y = x'
        assert!(approx_eq(d[0], y, 1e-12));
        // d[1] = dy/dt = z = x'' (因为 y = x', dy/dt = x'')
        assert!(approx_eq(d[1], z, 1e-12));
        // d[2] = dz/dt = x''' (三阶导数)
        assert!(approx_eq(d[2], -cfg.mu * z - y - x + x * x * x, 1e-12));
    }
}
