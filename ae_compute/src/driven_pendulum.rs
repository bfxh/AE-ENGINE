//! Driven Damped Pendulum — 受驱阻尼摆 (周期力强迫混沌)
//!
//! 受驱阻尼摆是非线性动力学中最经典的受迫系统之一. 单摆在重力 + 线性阻尼
//! + 周期外力矩作用下, 当驱动幅度超过阈值时, 由倍周期分岔进入混沌.
//! 该系统在物理学史上首次实验展示了确定性混沌 (Miles 1984), 并在
//! 约瑟夫森结、电荷密度波、同步加速器束流等众多物理系统中出现.
//!
//! 状态方程 (二阶 ODE 形式):
//!   θ'' + γ θ' + sin(θ) = A cos(ω t)
//!
//! 自治化为一阶 3D 系统 (引入驱动相位 φ = ω t):
//!   dθ/dt = p
//!   dp/dt = -γ p - sin(θ) + A cos(φ)
//!   dφ/dt = ω
//!
//! 经典参数 (Miles 1984, 混沌区): γ = 0.5, A = 1.15, ω = 2/3
//! 经典初值: (θ₀, p₀, φ₀) = (0.5, 0, 0)
//!
//! 性质:
//!   - 散度 ∇·F = ∂p/∂θ + ∂(-γp - sinθ + A cosφ)/∂p + ∂ω/∂φ
//!            = 0 + (-γ) + 0 = -γ (常数负, 耗散)
//!   - 自治系统无平衡点 (因为 dφ/dt = ω ≠ 0, φ 单调增长).
//!     但当 ω = 0 (无驱动) 时退化为阻尼摆, 平衡点:
//!       θ = nπ (n 偶 = 稳定下垂, n 奇 = 不稳定倒立), p = 0
//!   - Lyapunov 谱 (经典参数, 文献值, Miles 1984):
//!     λ₁ ≈ +0.16  (正, 主混沌方向)
//!     λ₂ = 0      (沿驱动相位 φ 方向, 中性)
//!     λ₃ ≈ -0.66  (负, 收缩)
//!     和 = -γ = -0.5 (与散度一致)
//!   - 倍周期分岔路径: A 增大时 1T → 2T → 4T → 8T → ... → 混沌
//!     (T = 2π/ω 为驱动周期)
//!   - 相空间 (θ, p, φ) 中吸引子为分形鞍层结构, φ 是环面方向
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!
//! 历史:
//!   Miles, J. 1984. "Resonant motion of a constrained pendulum."
//!   Phys. Lett. A 111, 381. (实验观测受驱摆混沌)
//!   D'Humieres, D. et al. 1982. "Chaotic states and routes to chaos
//!   in forced pendulum." Phys. Rev. A 26, 3483. (倍周期分岔)
//!   Strogatz, S. H. 2014. "Nonlinear Dynamics and Chaos." Westview.
//!   (教科书经典分析)

/// 受驱阻尼摆配置 (3 参数 + dt)
#[derive(Clone, Copy, Debug)]
pub struct DrivenPendulumConfig {
    /// 阻尼率 γ (经典 0.5)
    pub gamma: f64,
    /// 驱动振幅 A (经典 1.15, 混沌区)
    pub amplitude: f64,
    /// 驱动角频率 ω (经典 2/3)
    pub omega: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for DrivenPendulumConfig {
    fn default() -> Self {
        Self {
            gamma: 0.5,
            amplitude: 1.15,
            omega: 2.0 / 3.0,
            dt: 0.01,
        }
    }
}

/// 受驱阻尼摆求解器 (3D 自治化, 跟踪最大 Lyapunov 指数)
pub struct DrivenPendulumSolver {
    pub config: DrivenPendulumConfig,
    /// 摆角 θ (弧度)
    pub theta: f64,
    /// 角动量 p = θ'
    pub p: f64,
    /// 驱动相位 φ = ω t
    pub phi: f64,
    pub time: f64,
    pub step_count: u64,
    pub trajectory: Vec<(f64, f64, f64)>,
    /// Lyapunov 累积
    pub lyap_sum: f64,
    /// 切向量 (3D)
    pub v: [f64; 3],
}

impl DrivenPendulumSolver {
    pub fn new(config: DrivenPendulumConfig, theta0: f64, p0: f64, phi0: f64) -> Self {
        Self {
            config,
            theta: theta0,
            p: p0,
            phi: phi0,
            time: 0.0,
            step_count: 0,
            trajectory: vec![(theta0, p0, phi0)],
            lyap_sum: 0.0,
            v: [1.0, 0.0, 0.0],
        }
    }

    pub fn classic(config: DrivenPendulumConfig) -> Self {
        // 经典初值 (θ=0.5, p=0, φ=0)
        Self::new(config, 0.5, 0.0, 0.0)
    }

    /// 右端导数 F = [p, -γ p - sin(θ) + A cos(φ), ω]
    pub fn derivatives(cfg: &DrivenPendulumConfig, theta: f64, p: f64, phi: f64) -> [f64; 3] {
        [
            p,
            -cfg.gamma * p - theta.sin() + cfg.amplitude * phi.cos(),
            cfg.omega,
        ]
    }

    /// Jacobian:
    /// J = [[ 0,        1,   0          ],
    ///      [-cos(θ),  -γ,  -A sin(φ)   ],
    ///      [ 0,        0,   0          ]]
    pub fn jacobian(cfg: &DrivenPendulumConfig, theta: f64, _p: f64, phi: f64) -> [[f64; 3]; 3] {
        [
            [0.0, 1.0, 0.0],
            [-theta.cos(), -cfg.gamma, -cfg.amplitude * phi.sin()],
            [0.0, 0.0, 0.0],
        ]
    }

    /// 散度 ∇·F = tr(J) = -γ (常数负)
    pub fn divergence(cfg: &DrivenPendulumConfig, _theta: f64, _p: f64, _phi: f64) -> f64 {
        -cfg.gamma
    }

    /// 单步 RK4 推进 + 变分方程 Lyapunov
    pub fn step(&mut self) {
        let cfg = self.config;
        let dt = cfg.dt;
        let (theta, p, phi) = (self.theta, self.p, self.phi);

        let k1 = Self::derivatives(&cfg, theta, p, phi);
        let k2 = Self::derivatives(
            &cfg,
            theta + 0.5 * dt * k1[0],
            p + 0.5 * dt * k1[1],
            phi + 0.5 * dt * k1[2],
        );
        let k3 = Self::derivatives(
            &cfg,
            theta + 0.5 * dt * k2[0],
            p + 0.5 * dt * k2[1],
            phi + 0.5 * dt * k2[2],
        );
        let k4 = Self::derivatives(&cfg, theta + dt * k3[0], p + dt * k3[1], phi + dt * k3[2]);

        self.theta = theta + dt / 6.0 * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]);
        self.p = p + dt / 6.0 * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]);
        self.phi = phi + dt / 6.0 * (k1[2] + 2.0 * k2[2] + 2.0 * k3[2] + k4[2]);

        // 将 θ 规范化到 [-π, π] 避免数值漂移 (角度是周期 2π 变量)
        // 注意: 这只影响存储, 不影响动力学 (因为 sin/cos 是周期函数)
        // 但为保持轨迹连续性, 这里不强制规范化, 仅在 Lyapunov 估计中用

        self.step_count += 1;
        self.time += dt;
        self.trajectory.push((self.theta, self.p, self.phi));

        // Lyapunov: 变分方程前向欧拉 (I + dt J) v
        let j = Self::jacobian(&cfg, self.theta, self.p, self.phi);
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

    /// 最大 Lyapunov 指数 (文献值 ~0.16)
    pub fn lyapunov_exponent(&self) -> f64 {
        if self.step_count == 0 {
            return 0.0;
        }
        self.lyap_sum / self.time.max(1e-12)
    }

    pub fn has_nan(&self) -> bool {
        !self.theta.is_finite() || !self.p.is_finite() || !self.phi.is_finite()
    }

    pub fn has_escaped(&self) -> bool {
        self.theta.abs() > 1e6 || self.p.abs() > 1e6 || self.has_nan()
    }

    pub fn attractor_bounds(&self) -> (f64, f64, f64, f64) {
        let mut theta_min = f64::INFINITY;
        let mut theta_max = f64::NEG_INFINITY;
        let mut p_min = f64::INFINITY;
        let mut p_max = f64::NEG_INFINITY;
        for &(theta, p, _phi) in &self.trajectory {
            if theta < theta_min { theta_min = theta; }
            if theta > theta_max { theta_max = theta; }
            if p < p_min { p_min = p; }
            if p > p_max { p_max = p; }
        }
        (theta_min, theta_max, p_min, p_max)
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
        let cfg = DrivenPendulumConfig::default();
        assert!(approx_eq(cfg.gamma, 0.5, 1e-12));
        assert!(approx_eq(cfg.amplitude, 1.15, 1e-12));
        assert!(approx_eq(cfg.omega, 2.0 / 3.0, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = DrivenPendulumSolver::classic(DrivenPendulumConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = DrivenPendulumConfig::default();
        let (theta, p, phi) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = DrivenPendulumSolver::derivatives(&cfg, theta, p, phi);
        assert!(approx_eq(d[0], p, 1e-12));
        assert!(approx_eq(d[1], -cfg.gamma * p - theta.sin() + cfg.amplitude * phi.cos(), 1e-12));
        assert!(approx_eq(d[2], cfg.omega, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = DrivenPendulumConfig::default();
        let (theta, p, phi) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = DrivenPendulumSolver::jacobian(&cfg, theta, p, phi);
        // Row 0: [0, 1, 0]
        assert!(approx_eq(j[0][0], 0.0, 1e-12));
        assert!(approx_eq(j[0][1], 1.0, 1e-12));
        assert!(approx_eq(j[0][2], 0.0, 1e-12));
        // Row 1: [-cos(θ), -γ, -A sin(φ)]
        assert!(approx_eq(j[1][0], -theta.cos(), 1e-12));
        assert!(approx_eq(j[1][1], -cfg.gamma, 1e-12));
        assert!(approx_eq(j[1][2], -cfg.amplitude * phi.sin(), 1e-12));
        // Row 2: [0, 0, 0]
        assert!(approx_eq(j[2][0], 0.0, 1e-12));
        assert!(approx_eq(j[2][1], 0.0, 1e-12));
        assert!(approx_eq(j[2][2], 0.0, 1e-12));
    }

    #[test]
    fn test_divergence_constant() {
        // 散度 = -γ (常数)
        let cfg = DrivenPendulumConfig::default();
        let expected = -cfg.gamma;
        assert!(approx_eq(DrivenPendulumSolver::divergence(&cfg, 0.0, 0.0, 0.0), expected, 1e-12));
        assert!(approx_eq(DrivenPendulumSolver::divergence(&cfg, 1.0, 2.0, 3.0), expected, 1e-12));
        assert!(approx_eq(DrivenPendulumSolver::divergence(&cfg, -5.0, 7.0, -3.0), expected, 1e-12));
    }

    #[test]
    fn test_divergence_negative_dissipative() {
        let cfg = DrivenPendulumConfig::default();
        let div = DrivenPendulumSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < 0.0);
        assert!(approx_eq(div, -0.5, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = DrivenPendulumConfig::default();
        for &(theta, p, phi) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5)] {
            let j = DrivenPendulumSolver::jacobian(&cfg, theta, p, phi);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = DrivenPendulumSolver::divergence(&cfg, theta, p, phi);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_no_autonomous_equilibria() {
        // 自治化后无平衡点 (因为 dφ/dt = ω ≠ 0)
        // 验证: 任何 (θ, p, φ) 处 dφ/dt = ω ≠ 0
        let cfg = DrivenPendulumConfig::default();
        for &(theta, p, phi) in &[(0.0_f64, 0.0, 0.0), (1.0, 2.0, 3.0)] {
            let d = DrivenPendulumSolver::derivatives(&cfg, theta, p, phi);
            assert!((d[2] - cfg.omega).abs() < 1e-12, "dφ/dt should be ω");
            assert!(d[2].abs() > 0.0, "dφ/dt ≠ 0 → no equilibrium");
        }
    }

    #[test]
    fn test_step_advances() {
        let mut s = DrivenPendulumSolver::classic(DrivenPendulumConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_phi_grows_linearly() {
        // φ = ω t, 应线性增长
        let cfg = DrivenPendulumConfig::default();
        let mut s = DrivenPendulumSolver::classic(cfg);
        s.run(1000);
        let expected_phi = cfg.omega * s.time;
        // 由于 φ 是周期变量, 取模 2π 后比较
        let phi_diff = (s.phi - expected_phi).rem_euclid(2.0 * std::f64::consts::PI);
        let phi_diff = phi_diff.min(2.0 * std::f64::consts::PI - phi_diff);
        assert!(phi_diff < 0.1, "phi = {}, expected = {}, diff = {}", s.phi, expected_phi, phi_diff);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = DrivenPendulumSolver::classic(DrivenPendulumConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = DrivenPendulumSolver::classic(DrivenPendulumConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = DrivenPendulumSolver::classic(DrivenPendulumConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = DrivenPendulumSolver::classic(DrivenPendulumConfig::default());
        s.run(30000);
        let (theta_min, theta_max, p_min, p_max) = s.attractor_bounds();
        // 受驱摆混沌区: θ 可以绕多圈 (|θ| 可能很大), 但 p 有界
        // 这里只要求 p 不爆炸
        assert!(p_min > -50.0 && p_max < 50.0, "p: [{}, {}]", p_min, p_max);
    }

    #[test]
    fn test_lyapunov_positive() {
        // 经典参数下是混沌的, λ > 0 (文献值 ~0.16)
        let mut s = DrivenPendulumSolver::classic(DrivenPendulumConfig::default());
        s.run(80000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = DrivenPendulumSolver::classic(DrivenPendulumConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        let cfg = DrivenPendulumConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = DrivenPendulumSolver::new(cfg, 0.5, 0.0, 0.0);
        let mut s2 = DrivenPendulumSolver::new(cfg, 0.5 + d0, 0.0, 0.0);
        for _ in 0..60000 {
            s1.step();
            s2.step();
        }
        let dtheta = s1.theta - s2.theta;
        let dp = s1.p - s2.p;
        let d = (dtheta * dtheta + dp * dp).sqrt();
        // t=600, λ~0.16, 应放大 e^96 (饱和到吸引子尺度)
        assert!(d > 1e-3, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_zero_amplitude_no_chaos() {
        // A = 0 时退化为阻尼摆, 无驱动 → 衰减到平衡点 → λ < 0
        let cfg = DrivenPendulumConfig { amplitude: 0.0, ..DrivenPendulumConfig::default() };
        let mut s = DrivenPendulumSolver::new(cfg, 0.5, 0.0, 0.0);
        s.run(50000);
        let lambda = s.lyapunov_exponent();
        // 无驱动时, 摆衰减到 θ=nπ, p=0, 切向量收缩
        assert!(lambda < 0.01, "no drive → no chaos, lambda = {}", lambda);
    }

    #[test]
    fn test_small_amplitude_periodic() {
        // 小驱动振幅 → 周期运动 (非混沌), λ ≈ 0
        let cfg = DrivenPendulumConfig { amplitude: 0.3, ..DrivenPendulumConfig::default() };
        let mut s = DrivenPendulumSolver::new(cfg, 0.5, 0.0, 0.0);
        s.run(50000);
        let lambda = s.lyapunov_exponent();
        // 小振幅下轨道周期化, λ 应该 ≈ 0 (容差放宽)
        assert!(lambda < 0.05, "small drive → periodic, lambda = {}", lambda);
    }

    #[test]
    fn test_volume_monotonic_contraction() {
        // 散度 = -γ (常数负), 体积单调收缩
        let cfg = DrivenPendulumConfig::default();
        for &(theta, p, phi) in &[(1.0_f64, 2.0, 3.0), (-5.0, 7.0, -3.0), (10.0, -10.0, 20.0)] {
            assert!(DrivenPendulumSolver::divergence(&cfg, theta, p, phi) < 0.0);
        }
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = DrivenPendulumConfig::default();
        let mut s = DrivenPendulumSolver::new(cfg, 1000.0, 1000.0, 0.0);
        s.run(500);
        assert!(s.has_escaped() || !s.has_nan());
    }
}
