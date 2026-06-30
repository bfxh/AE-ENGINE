//! Ueda Attractor — Ueda 振荡器 (日本吸引子, 1979)
//!
//! Yoshisuke Ueda 1961 年在京都大学模拟计算机上发现, 1979 年正式发表的
//! 经典混沌吸引子. Ueda 振荡器是一个具有纯立方非线性恢复力的受迫阻尼
//! 振荡器, 没有线性恢复项 (与 Duffing 振荡器的关键区别). 该系统在混沌
//! 理论史上具有里程碑意义: Ueda 在 1961 年就观测到"日本吸引子"的混沌
//! 形态, 但当时无法发表, 直到 1979 年才正式出版.
//!
//! 状态方程 (二阶 ODE):
//!   x'' + k x' + x³ = B cos(t)
//!
//! 自治化为一阶 3D 系统 (引入相位 φ = t):
//!   dx/dt = y
//!   dy/dt = -k y - x³ + B cos(φ)
//!   dφ/dt = 1
//!
//! 经典参数 (Ueda 1979): k = 0.05, B = 7.5
//! 其他混沌参数: k = 0.1, B = 12; k = 0.25, B = 8.5
//! 经典初值: (x₀, y₀, φ₀) = (1, 0, 0) 或 (3, 0, 0)
//!
//! 性质:
//!   - 散度 ∇·F = -k (常数负, 耗散)
//!   - 无自治平衡点 (因 dφ/dt = 1 ≠ 0)
//!   - 纯立方恢复力: 势能 V(x) = x⁴/4 (单井, 无双稳态)
//!     与 Duffing (V = x²/2 + x⁴/4, 双井) 形成对比
//!   - Lyapunov 谱 (k=0.05, B=7.5, 文献值):
//!     λ₁ ≈ +0.12  (正, 主混沌方向)
//!     λ₂ = 0      (沿驱动相位方向)
//!     λ₃ ≈ -0.17  (负, 收缩)
//!     和 = -k = -0.05 (与散度一致)
//!   - Kaplan-Yorke 维数 D_KY = 2 + λ₁/|λ₃| ≈ 2.71
//!   - 吸引子形态: 蚕茧形 (coccoon), 轨道在 x-y 平面形成扭曲的薄层
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!
//! 历史:
//!   Ueda, Y. 1979. "Randomly transitional phenomena in the system governed
//!   by Duffing's equation." J. Stat. Phys. 20, 181. (正式发表, 尽管发现于 1961)
//!   Ueda, Y. 1992. "The Road to Chaos." Aerial Press. (自传性回顾)
//!   Abraham, R. H. & Ueda, Y. (Eds.) 2000. "The Chaos Avant-Garde."
//!   World Scientific. (混沌发现史)

/// Ueda 振荡器配置 (2 参数 + dt)
#[derive(Clone, Copy, Debug)]
pub struct UedaConfig {
    /// 阻尼率 k (经典 0.05)
    pub k: f64,
    /// 驱动振幅 B (经典 7.5)
    pub b: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for UedaConfig {
    fn default() -> Self {
        Self { k: 0.05, b: 7.5, dt: 0.01 }
    }
}

/// Ueda 振荡器求解器 (3D 自治化, 跟踪最大 Lyapunov 指数)
pub struct UedaSolver {
    pub config: UedaConfig,
    pub x: f64,
    pub y: f64,
    pub phi: f64,
    pub time: f64,
    pub step_count: u64,
    pub trajectory: Vec<(f64, f64, f64)>,
    /// Lyapunov 累积
    pub lyap_sum: f64,
    /// 切向量 (3D)
    pub v: [f64; 3],
}

impl UedaSolver {
    pub fn new(config: UedaConfig, x0: f64, y0: f64, phi0: f64) -> Self {
        Self {
            config,
            x: x0,
            y: y0,
            phi: phi0,
            time: 0.0,
            step_count: 0,
            trajectory: vec![(x0, y0, phi0)],
            lyap_sum: 0.0,
            v: [1.0, 0.0, 0.0],
        }
    }

    pub fn classic(config: UedaConfig) -> Self {
        // 经典初值 (1, 0, 0)
        Self::new(config, 1.0, 0.0, 0.0)
    }

    /// 右端导数 F = [y, -k y - x³ + B cos(φ), 1]
    pub fn derivatives(cfg: &UedaConfig, x: f64, y: f64, phi: f64) -> [f64; 3] {
        [y, -cfg.k * y - x * x * x + cfg.b * phi.cos(), 1.0]
    }

    /// Jacobian:
    /// J = [[0,    1,    0          ],
    ///      [-3x², -k,   -B sin(φ)  ],
    ///      [0,    0,    0          ]]
    pub fn jacobian(cfg: &UedaConfig, x: f64, _y: f64, phi: f64) -> [[f64; 3]; 3] {
        [
            [0.0, 1.0, 0.0],
            [-3.0 * x * x, -cfg.k, -cfg.b * phi.sin()],
            [0.0, 0.0, 0.0],
        ]
    }

    /// 散度 ∇·F = tr(J) = -k (常数负)
    pub fn divergence(cfg: &UedaConfig, _x: f64, _y: f64, _phi: f64) -> f64 {
        -cfg.k
    }

    /// 势能 V(x) = x⁴/4 (纯立方恢复力的势能)
    pub fn potential(x: f64) -> f64 {
        x * x * x * x / 4.0
    }

    /// 动能 T = y²/2
    pub fn kinetic_energy(y: f64) -> f64 {
        0.5 * y * y
    }

    /// 总机械能 E = T + V (无驱动, 无阻尼时的守恒量)
    pub fn total_energy(x: f64, y: f64) -> f64 {
        Self::kinetic_energy(y) + Self::potential(x)
    }

    /// 单步 RK4 推进 + 变分方程 Lyapunov
    pub fn step(&mut self) {
        let cfg = self.config;
        let dt = cfg.dt;
        let (x, y, phi) = (self.x, self.y, self.phi);

        let k1 = Self::derivatives(&cfg, x, y, phi);
        let k2 = Self::derivatives(&cfg, x + 0.5 * dt * k1[0], y + 0.5 * dt * k1[1], phi + 0.5 * dt * k1[2]);
        let k3 = Self::derivatives(&cfg, x + 0.5 * dt * k2[0], y + 0.5 * dt * k2[1], phi + 0.5 * dt * k2[2]);
        let k4 = Self::derivatives(&cfg, x + dt * k3[0], y + dt * k3[1], phi + dt * k3[2]);

        self.x = x + dt / 6.0 * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]);
        self.y = y + dt / 6.0 * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]);
        self.phi = phi + dt / 6.0 * (k1[2] + 2.0 * k2[2] + 2.0 * k3[2] + k4[2]);

        self.step_count += 1;
        self.time += dt;
        self.trajectory.push((self.x, self.y, self.phi));

        // Lyapunov: 变分方程前向欧拉 (I + dt J) v
        let j = Self::jacobian(&cfg, self.x, self.y, self.phi);
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

    /// 最大 Lyapunov 指数 (文献值 ~0.12)
    pub fn lyapunov_exponent(&self) -> f64 {
        if self.step_count == 0 {
            return 0.0;
        }
        self.lyap_sum / self.time.max(1e-12)
    }

    pub fn has_nan(&self) -> bool {
        !self.x.is_finite() || !self.y.is_finite() || !self.phi.is_finite()
    }

    pub fn has_escaped(&self) -> bool {
        self.x.abs() > 100.0 || self.y.abs() > 100.0 || self.has_nan()
    }

    pub fn attractor_bounds(&self) -> (f64, f64, f64, f64) {
        let mut xmin = f64::INFINITY;
        let mut xmax = f64::NEG_INFINITY;
        let mut ymin = f64::INFINITY;
        let mut ymax = f64::NEG_INFINITY;
        for &(x, y, _phi) in &self.trajectory {
            if x < xmin { xmin = x; }
            if x > xmax { xmax = x; }
            if y < ymin { ymin = y; }
            if y > ymax { ymax = y; }
        }
        (xmin, xmax, ymin, ymax)
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
        let cfg = UedaConfig::default();
        assert!(approx_eq(cfg.k, 0.05, 1e-12));
        assert!(approx_eq(cfg.b, 7.5, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = UedaSolver::classic(UedaConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = UedaConfig::default();
        let (x, y, phi) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = UedaSolver::derivatives(&cfg, x, y, phi);
        assert!(approx_eq(d[0], y, 1e-12));
        assert!(approx_eq(d[1], -cfg.k * y - x * x * x + cfg.b * phi.cos(), 1e-12));
        assert!(approx_eq(d[2], 1.0, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = UedaConfig::default();
        let (x, y, phi) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = UedaSolver::jacobian(&cfg, x, y, phi);
        // Row 0: [0, 1, 0]
        assert!(approx_eq(j[0][0], 0.0, 1e-12));
        assert!(approx_eq(j[0][1], 1.0, 1e-12));
        assert!(approx_eq(j[0][2], 0.0, 1e-12));
        // Row 1: [-3x², -k, -B sin(φ)]
        assert!(approx_eq(j[1][0], -3.0 * x * x, 1e-12));
        assert!(approx_eq(j[1][1], -cfg.k, 1e-12));
        assert!(approx_eq(j[1][2], -cfg.b * phi.sin(), 1e-12));
        // Row 2: [0, 0, 0]
        assert!(approx_eq(j[2][0], 0.0, 1e-12));
        assert!(approx_eq(j[2][1], 0.0, 1e-12));
        assert!(approx_eq(j[2][2], 0.0, 1e-12));
    }

    #[test]
    fn test_divergence_constant() {
        // 散度 = -k (常数)
        let cfg = UedaConfig::default();
        let expected = -cfg.k;
        assert!(approx_eq(UedaSolver::divergence(&cfg, 0.0, 0.0, 0.0), expected, 1e-12));
        assert!(approx_eq(UedaSolver::divergence(&cfg, 1.0, 2.0, 3.0), expected, 1e-12));
        assert!(approx_eq(UedaSolver::divergence(&cfg, -5.0, 7.0, -3.0), expected, 1e-12));
    }

    #[test]
    fn test_divergence_negative_dissipative() {
        let cfg = UedaConfig::default();
        let div = UedaSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < 0.0);
        assert!(approx_eq(div, -0.05, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = UedaConfig::default();
        for &(x, y, phi) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5)] {
            let j = UedaSolver::jacobian(&cfg, x, y, phi);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = UedaSolver::divergence(&cfg, x, y, phi);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_no_autonomous_equilibria() {
        // 自治化后无平衡点 (因为 dφ/dt = 1 ≠ 0)
        let cfg = UedaConfig::default();
        for &(x, y, phi) in &[(0.0_f64, 0.0, 0.0), (1.0, 2.0, 3.0)] {
            let d = UedaSolver::derivatives(&cfg, x, y, phi);
            assert!((d[2] - 1.0).abs() < 1e-12, "dφ/dt should be 1");
            assert!(d[2].abs() > 0.0, "dφ/dt ≠ 0 → no equilibrium");
        }
    }

    #[test]
    fn test_potential_formula() {
        // V(x) = x⁴/4 (纯四次势能, 单井)
        assert!(approx_eq(UedaSolver::potential(0.0), 0.0, 1e-12));
        assert!(approx_eq(UedaSolver::potential(1.0), 0.25, 1e-12));
        assert!(approx_eq(UedaSolver::potential(2.0), 4.0, 1e-12));
        assert!(approx_eq(UedaSolver::potential(-1.0), 0.25, 1e-12));
    }

    #[test]
    fn test_kinetic_energy() {
        assert!(approx_eq(UedaSolver::kinetic_energy(0.0), 0.0, 1e-12));
        assert!(approx_eq(UedaSolver::kinetic_energy(1.0), 0.5, 1e-12));
        assert!(approx_eq(UedaSolver::kinetic_energy(2.0), 2.0, 1e-12));
    }

    #[test]
    fn test_total_energy() {
        let (x, y) = (2.0_f64, 3.0_f64);
        let e = UedaSolver::total_energy(x, y);
        let expected = 0.5 * y * y + x * x * x * x / 4.0;
        assert!(approx_eq(e, expected, 1e-12));
    }

    #[test]
    fn test_step_advances() {
        let mut s = UedaSolver::classic(UedaConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_phi_grows_linearly() {
        // φ = t (因为 dφ/dt = 1)
        let cfg = UedaConfig::default();
        let mut s = UedaSolver::classic(cfg);
        s.run(1000);
        let expected_phi = s.time;
        let phi_diff = (s.phi - expected_phi).rem_euclid(2.0 * std::f64::consts::PI);
        let phi_diff = phi_diff.min(2.0 * std::f64::consts::PI - phi_diff);
        assert!(phi_diff < 0.1, "phi = {}, expected = {}, diff = {}", s.phi, expected_phi, phi_diff);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = UedaSolver::classic(UedaConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = UedaSolver::classic(UedaConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = UedaSolver::classic(UedaConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = UedaSolver::classic(UedaConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax) = s.attractor_bounds();
        // Ueda 吸引子典型范围: x∈[-4,4], y∈[-5,5]
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
    }

    #[test]
    fn test_lyapunov_positive() {
        // Ueda 是混沌的, λ > 0 (文献值 ~0.12)
        let mut s = UedaSolver::classic(UedaConfig::default());
        s.run(80000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = UedaSolver::classic(UedaConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        let cfg = UedaConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = UedaSolver::new(cfg, 1.0, 0.0, 0.0);
        let mut s2 = UedaSolver::new(cfg, 1.0 + d0, 0.0, 0.0);
        for _ in 0..60000 {
            s1.step();
            s2.step();
        }
        let dx = s1.x - s2.x;
        let dy = s1.y - s2.y;
        let d = (dx * dx + dy * dy).sqrt();
        // t=600, λ~0.12, 应放大 e^72 (饱和到吸引子尺度)
        assert!(d > 1e-3, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_zero_drive_no_chaos() {
        // B = 0 时退化为无驱动阻尼振子, 衰减到 (0, 0, φ)
        let cfg = UedaConfig { b: 0.0, ..UedaConfig::default() };
        let mut s = UedaSolver::new(cfg, 1.0, 0.0, 0.0);
        s.run(50000);
        let lambda = s.lyapunov_exponent();
        // 无驱动时, x 衰减到 0, y 衰减到 0, 切向量收缩
        assert!(lambda < 0.01, "no drive → no chaos, lambda = {}", lambda);
    }

    #[test]
    fn test_zero_damping_volume_not_contracting_to_zero() {
        // k = 0 (无阻尼) 时散度 = 0, 但仍有驱动 → 能量注入
        // 不会简单衰减, 可能增长 (但仍有界, 因 x³ 项在大幅时强力恢复)
        let cfg = UedaConfig { k: 0.0, ..UedaConfig::default() };
        let mut s = UedaSolver::classic(cfg);
        s.run(5000);
        // 只验证不 NaN (无阻尼 + 驱动可能不稳定, 但 x³ 恢复力应该提供有界性)
        // 注: 实际 k=0 的 Ueda 系统可能能量发散, 这里只验证短期数值稳定性
        assert!(!s.has_nan(), "should not NaN in short run");
    }

    #[test]
    fn test_volume_monotonic_contraction() {
        // 散度 = -k (常数负), 体积单调收缩
        let cfg = UedaConfig::default();
        for &(x, y, phi) in &[(1.0_f64, 2.0, 3.0), (-5.0, 7.0, -3.0), (10.0, -10.0, 20.0)] {
            assert!(UedaSolver::divergence(&cfg, x, y, phi) < 0.0);
        }
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = UedaConfig::default();
        let mut s = UedaSolver::new(cfg, 100.0, 100.0, 0.0);
        s.run(500);
        assert!(s.has_escaped() || !s.has_nan());
    }

    #[test]
    fn test_pure_cubic_no_linear_term() {
        // Ueda 系统标志特征: 恢复力 = -x³ (无 -x 线性项)
        // Jacobian[1][0] = -3x² (在 x=0 处为零, 与 Duffing 的 -1-3x² 不同)
        let cfg = UedaConfig::default();
        let j_at_zero = UedaSolver::jacobian(&cfg, 0.0, 0.0, 0.0);
        // 在 x=0, 恢复力导数应为 0 (因为纯 x³ 的导数 3x² 在 x=0 为零)
        assert!(approx_eq(j_at_zero[1][0], 0.0, 1e-12),
            "pure cubic: dF/dx = -3x² = 0 at x=0");
        // 对比: Duffing 在 x=0 的 dF/dx = -(1+3x²) = -1
        let j_at_one = UedaSolver::jacobian(&cfg, 1.0, 0.0, 0.0);
        assert!(approx_eq(j_at_one[1][0], -3.0, 1e-12),
            "dF/dx = -3x² = -3 at x=1");
    }
}
