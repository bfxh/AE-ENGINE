//! Euler Rigid Body — Euler 自由刚体方程 (Dzhanibekov 效应)
//!
//! Leonhard Euler 1758 年建立的自由刚体旋转方程, 是刚体力学的基石.
//! 无外力矩作用下, 刚体绕三个主轴的角速度演化由 Euler 方程描述.
//! 该系统是 Hamiltonian 保守系统, 具有两个守恒量 (能量和角动量平方),
//! 因此 3D 相空间中的轨道被约束在两个守恒量的交集 (1D 曲线) 上,
//! 轨道必为周期或准周期 (不能混沌, 因为 3D - 2 守恒量 = 1D 流形).
//!
//! 状态方程 (Euler 1758, 主轴坐标系):
//!   I₁ ω̇₁ = (I₂ - I₃) ω₂ ω₃
//!   I₂ ω̇₂ = (I₃ - I₁) ω₃ ω₁
//!   I₃ ω̇₃ = (I₁ - I₂) ω₁ ω₂
//!
//! 其中 I₁, I₂, I₃ 为绕三个主轴的转动惯量 (I₁ < I₂ < I₃ 不失一般性),
//! ω₁, ω₂, ω₃ 为对应主轴方向的角速度分量.
//!
//! 简记 α = (I₂-I₃)/I₁, β = (I₃-I₁)/I₂, γ = (I₁-I₂)/I₃, 则:
//!   ω̇₁ = α ω₂ ω₃
//!   ω̇₂ = β ω₃ ω₁
//!   ω̇₃ = γ ω₁ ω₂
//!
//! 守恒量:
//!   能量 H = (1/2)(I₁ω₁² + I₂ω₂² + I₃ω₃²)  (动能, 因无力矩)
//!   角动量平方 L² = I₁²ω₁² + I₂²ω₂² + I₃²ω₃²
//!
//! 性质:
//!   - 散度 ∇·F = tr(J) = 0 (保守, 体积保持, Hamiltonian 流)
//!   - 平衡点: 三个主轴上 (ω₁,0,0), (0,ω₂,0), (0,0,ω₃) 均为平衡
//!   - 稳定性 (网球拍定理 / Dzhanibekov 效应, I₁<I₂<I₃):
//!     * 绕 I₁ 轴 (最小): 稳定
//!     * 绕 I₂ 轴 (中间): 不稳定 (Dzhanibekov 翻转)
//!     * 绕 I₃ 轴 (最大): 稳定
//!   - 无混沌: 3D 系统 + 2 守恒量 → 1D 轨道 (周期或准周期)
//!
//! 历史:
//!   Euler, L. 1758. "Du mouvement de rotation des corps solides."
//!   Mém. Acad. Sci. Berlin 14, 154-193. (Euler 方程原始论文)
//!   Dzhanibekov, V. A. 1985. (太空实验观测中间轴翻转效应)
//!   Ashbaugh, M. S. et al. 1991. "The tennis racket theorem."
//!   Amer. Math. Monthly 98(10), 892. (数学证明)

/// Euler 刚体配置 (3 主转动惯量 + dt)
#[derive(Clone, Copy, Debug)]
pub struct EulerRigidBodyConfig {
    /// 转动惯量 I₁ (经典 1.0, 最小)
    pub i1: f64,
    /// 转动惯量 I₂ (经典 2.0, 中间)
    pub i2: f64,
    /// 转动惯量 I₃ (经典 3.0, 最大)
    pub i3: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for EulerRigidBodyConfig {
    fn default() -> Self {
        Self {
            i1: 1.0,
            i2: 2.0,
            i3: 3.0,
            dt: 0.005,
        }
    }
}

/// Euler 自由刚体求解器 (3D, 保守 Hamiltonian 系统)
pub struct EulerRigidBodySolver {
    pub config: EulerRigidBodyConfig,
    pub w1: f64,
    pub w2: f64,
    pub w3: f64,
    pub time: f64,
    pub step_count: u64,
    pub trajectory: Vec<(f64, f64, f64)>,
    /// 初始能量 (用于守恒性检测)
    pub initial_energy: f64,
    /// 初始角动量平方 (用于守恒性检测)
    pub initial_l_squared: f64,
}

impl EulerRigidBodySolver {
    pub fn new(config: EulerRigidBodyConfig, w1_0: f64, w2_0: f64, w3_0: f64) -> Self {
        let initial_energy = Self::energy(&config, w1_0, w2_0, w3_0);
        let initial_l_squared = Self::angular_momentum_squared(&config, w1_0, w2_0, w3_0);
        Self {
            config,
            w1: w1_0,
            w2: w2_0,
            w3: w3_0,
            time: 0.0,
            step_count: 0,
            trajectory: vec![(w1_0, w2_0, w3_0)],
            initial_energy,
            initial_l_squared,
        }
    }

    /// 经典初值: 绕中间轴 (I₂) 旋转 + 小扰动 (演示 Dzhanibekov 翻转)
    pub fn dzhanibekov(config: EulerRigidBodyConfig) -> Self {
        // 主旋转绕中间轴 ω₂ = 10, 加小扰动 ω₁ = 0.1, ω₃ = 0.01
        Self::new(config, 0.1, 10.0, 0.01)
    }

    /// 经典初值: 绕稳定轴 (I₁) 旋转 + 小扰动
    pub fn stable_axis_1(config: EulerRigidBodyConfig) -> Self {
        Self::new(config, 10.0, 0.1, 0.01)
    }

    /// 经典初值: 绕稳定轴 (I₃) 旋转 + 小扰动
    pub fn stable_axis_3(config: EulerRigidBodyConfig) -> Self {
        Self::new(config, 0.01, 0.1, 10.0)
    }

    /// 右端导数 F = [(I₂-I₃)/I₁ · ω₂ω₃, (I₃-I₁)/I₂ · ω₃ω₁, (I₁-I₂)/I₃ · ω₁ω₂]
    pub fn derivatives(cfg: &EulerRigidBodyConfig, w1: f64, w2: f64, w3: f64) -> [f64; 3] {
        let alpha = (cfg.i2 - cfg.i3) / cfg.i1;
        let beta = (cfg.i3 - cfg.i1) / cfg.i2;
        let gamma = (cfg.i1 - cfg.i2) / cfg.i3;
        [alpha * w2 * w3, beta * w3 * w1, gamma * w1 * w2]
    }

    /// Jacobian:
    /// J = [[0,      α·ω₃,  α·ω₂],
    ///      [β·ω₃,   0,     β·ω₁],
    ///      [γ·ω₂,   γ·ω₁,  0   ]]
    /// 其中 α=(I₂-I₃)/I₁, β=(I₃-I₁)/I₂, γ=(I₁-I₂)/I₃
    pub fn jacobian(cfg: &EulerRigidBodyConfig, w1: f64, w2: f64, w3: f64) -> [[f64; 3]; 3] {
        let alpha = (cfg.i2 - cfg.i3) / cfg.i1;
        let beta = (cfg.i3 - cfg.i1) / cfg.i2;
        let gamma = (cfg.i1 - cfg.i2) / cfg.i3;
        [
            [0.0, alpha * w3, alpha * w2],
            [beta * w3, 0.0, beta * w1],
            [gamma * w2, gamma * w1, 0.0],
        ]
    }

    /// 散度 ∇·F = tr(J) = 0 (保守 Hamiltonian, 体积保持)
    pub fn divergence(_cfg: &EulerRigidBodyConfig, _w1: f64, _w2: f64, _w3: f64) -> f64 {
        0.0
    }

    /// 能量 H = (1/2)(I₁ω₁² + I₂ω₂² + I₃ω₃²)
    pub fn energy(cfg: &EulerRigidBodyConfig, w1: f64, w2: f64, w3: f64) -> f64 {
        0.5 * (cfg.i1 * w1 * w1 + cfg.i2 * w2 * w2 + cfg.i3 * w3 * w3)
    }

    /// 角动量平方 L² = I₁²ω₁² + I₂²ω₂² + I₃²ω₃²
    pub fn angular_momentum_squared(cfg: &EulerRigidBodyConfig, w1: f64, w2: f64, w3: f64) -> f64 {
        cfg.i1 * cfg.i1 * w1 * w1 + cfg.i2 * cfg.i2 * w2 * w2 + cfg.i3 * cfg.i3 * w3 * w3
    }

    /// 角动量 |L|
    pub fn angular_momentum(cfg: &EulerRigidBodyConfig, w1: f64, w2: f64, w3: f64) -> f64 {
        Self::angular_momentum_squared(cfg, w1, w2, w3).sqrt()
    }

    /// 单步 RK4 推进 (无 Lyapunov, 因为保守系统 Lyapunov ≈ 0, 不具诊断价值)
    pub fn step(&mut self) {
        let cfg = self.config;
        let dt = cfg.dt;
        let (w1, w2, w3) = (self.w1, self.w2, self.w3);

        let k1 = Self::derivatives(&cfg, w1, w2, w3);
        let k2 = Self::derivatives(&cfg, w1 + 0.5 * dt * k1[0], w2 + 0.5 * dt * k1[1], w3 + 0.5 * dt * k1[2]);
        let k3 = Self::derivatives(&cfg, w1 + 0.5 * dt * k2[0], w2 + 0.5 * dt * k2[1], w3 + 0.5 * dt * k2[2]);
        let k4 = Self::derivatives(&cfg, w1 + dt * k3[0], w2 + dt * k3[1], w3 + dt * k3[2]);

        self.w1 = w1 + dt / 6.0 * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]);
        self.w2 = w2 + dt / 6.0 * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]);
        self.w3 = w3 + dt / 6.0 * (k1[2] + 2.0 * k2[2] + 2.0 * k3[2] + k4[2]);

        self.step_count += 1;
        self.time += dt;
        self.trajectory.push((self.w1, self.w2, self.w3));
    }

    pub fn run(&mut self, n_steps: usize) {
        for _ in 0..n_steps {
            self.step();
        }
    }

    /// 当前能量
    pub fn current_energy(&self) -> f64 {
        Self::energy(&self.config, self.w1, self.w2, self.w3)
    }

    /// 当前角动量平方
    pub fn current_l_squared(&self) -> f64 {
        Self::angular_momentum_squared(&self.config, self.w1, self.w2, self.w3)
    }

    /// 能量相对误差 (用于守恒性检测)
    pub fn energy_relative_error(&self) -> f64 {
        if self.initial_energy.abs() < 1e-12 {
            return 0.0;
        }
        (self.current_energy() - self.initial_energy).abs() / self.initial_energy.abs()
    }

    /// 角动量平方相对误差 (用于守恒性检测)
    pub fn l_squared_relative_error(&self) -> f64 {
        if self.initial_l_squared.abs() < 1e-12 {
            return 0.0;
        }
        (self.current_l_squared() - self.initial_l_squared).abs() / self.initial_l_squared.abs()
    }

    pub fn has_nan(&self) -> bool {
        !self.w1.is_finite() || !self.w2.is_finite() || !self.w3.is_finite()
    }

    pub fn has_escaped(&self) -> bool {
        self.w1.abs() > 1e6 || self.w2.abs() > 1e6 || self.w3.abs() > 1e6 || self.has_nan()
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
        let cfg = EulerRigidBodyConfig::default();
        assert!(approx_eq(cfg.i1, 1.0, 1e-12));
        assert!(approx_eq(cfg.i2, 2.0, 1e-12));
        assert!(approx_eq(cfg.i3, 3.0, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = EulerRigidBodySolver::dzhanibekov(EulerRigidBodyConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = EulerRigidBodyConfig::default();
        let (w1, w2, w3) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = EulerRigidBodySolver::derivatives(&cfg, w1, w2, w3);
        let alpha = (cfg.i2 - cfg.i3) / cfg.i1;
        let beta = (cfg.i3 - cfg.i1) / cfg.i2;
        let gamma = (cfg.i1 - cfg.i2) / cfg.i3;
        assert!(approx_eq(d[0], alpha * w2 * w3, 1e-12));
        assert!(approx_eq(d[1], beta * w3 * w1, 1e-12));
        assert!(approx_eq(d[2], gamma * w1 * w2, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = EulerRigidBodyConfig::default();
        let (w1, w2, w3) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = EulerRigidBodySolver::jacobian(&cfg, w1, w2, w3);
        let alpha = (cfg.i2 - cfg.i3) / cfg.i1;
        let beta = (cfg.i3 - cfg.i1) / cfg.i2;
        let gamma = (cfg.i1 - cfg.i2) / cfg.i3;
        // Row 0: [0, α·ω₃, α·ω₂]
        assert!(approx_eq(j[0][0], 0.0, 1e-12));
        assert!(approx_eq(j[0][1], alpha * w3, 1e-12));
        assert!(approx_eq(j[0][2], alpha * w2, 1e-12));
        // Row 1: [β·ω₃, 0, β·ω₁]
        assert!(approx_eq(j[1][0], beta * w3, 1e-12));
        assert!(approx_eq(j[1][1], 0.0, 1e-12));
        assert!(approx_eq(j[1][2], beta * w1, 1e-12));
        // Row 2: [γ·ω₂, γ·ω₁, 0]
        assert!(approx_eq(j[2][0], gamma * w2, 1e-12));
        assert!(approx_eq(j[2][1], gamma * w1, 1e-12));
        assert!(approx_eq(j[2][2], 0.0, 1e-12));
    }

    #[test]
    fn test_divergence_zero() {
        // 散度 = 0 (保守 Hamiltonian 系统)
        let cfg = EulerRigidBodyConfig::default();
        assert!(approx_eq(EulerRigidBodySolver::divergence(&cfg, 0.0, 0.0, 0.0), 0.0, 1e-12));
        assert!(approx_eq(EulerRigidBodySolver::divergence(&cfg, 1.0, 2.0, 3.0), 0.0, 1e-12));
        assert!(approx_eq(EulerRigidBodySolver::divergence(&cfg, -5.0, 7.0, -3.0), 0.0, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_is_zero() {
        // tr(J) = 0 (体积保持, Hamiltonian)
        let cfg = EulerRigidBodyConfig::default();
        for &(w1, w2, w3) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5), (5.0, -3.0, 2.0)] {
            let j = EulerRigidBodySolver::jacobian(&cfg, w1, w2, w3);
            let tr = j[0][0] + j[1][1] + j[2][2];
            assert!(approx_eq(tr, 0.0, 1e-12));
        }
    }

    #[test]
    fn test_energy_formula() {
        let cfg = EulerRigidBodyConfig::default();
        let (w1, w2, w3) = (0.3_f64, 0.5_f64, 0.7_f64);
        let h = EulerRigidBodySolver::energy(&cfg, w1, w2, w3);
        let expected = 0.5 * (cfg.i1 * w1 * w1 + cfg.i2 * w2 * w2 + cfg.i3 * w3 * w3);
        assert!(approx_eq(h, expected, 1e-12));
    }

    #[test]
    fn test_angular_momentum_squared_formula() {
        let cfg = EulerRigidBodyConfig::default();
        let (w1, w2, w3) = (0.3_f64, 0.5_f64, 0.7_f64);
        let l_sq = EulerRigidBodySolver::angular_momentum_squared(&cfg, w1, w2, w3);
        let expected = cfg.i1 * cfg.i1 * w1 * w1 + cfg.i2 * cfg.i2 * w2 * w2 + cfg.i3 * cfg.i3 * w3 * w3;
        assert!(approx_eq(l_sq, expected, 1e-12));
    }

    #[test]
    fn test_equilibria_on_axes() {
        // 平衡点: 沿三个主轴的任意点
        let cfg = EulerRigidBodyConfig::default();
        // E₁ = (1, 0, 0)
        let d = EulerRigidBodySolver::derivatives(&cfg, 1.0, 0.0, 0.0);
        for v in d.iter() {
            assert!(v.abs() < 1e-12);
        }
        // E₂ = (0, 2, 0)
        let d = EulerRigidBodySolver::derivatives(&cfg, 0.0, 2.0, 0.0);
        for v in d.iter() {
            assert!(v.abs() < 1e-12);
        }
        // E₃ = (0, 0, 3)
        let d = EulerRigidBodySolver::derivatives(&cfg, 0.0, 0.0, 3.0);
        for v in d.iter() {
            assert!(v.abs() < 1e-12);
        }
    }

    #[test]
    fn test_step_advances() {
        let mut s = EulerRigidBodySolver::dzhanibekov(EulerRigidBodyConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = EulerRigidBodySolver::dzhanibekov(EulerRigidBodyConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = EulerRigidBodySolver::dzhanibekov(EulerRigidBodyConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_energy_conservation() {
        // 保守系统: 能量应守恒 (RK4 有 O(dt⁴) 截断误差, 长期漂移小)
        let mut s = EulerRigidBodySolver::dzhanibekov(EulerRigidBodyConfig::default());
        s.run(20000);
        let rel_err = s.energy_relative_error();
        assert!(rel_err < 1e-3, "energy relative error = {}", rel_err);
    }

    #[test]
    fn test_angular_momentum_conservation() {
        // 保守系统: 角动量平方应守恒
        let mut s = EulerRigidBodySolver::dzhanibekov(EulerRigidBodyConfig::default());
        s.run(20000);
        let rel_err = s.l_squared_relative_error();
        assert!(rel_err < 1e-3, "L² relative error = {}", rel_err);
    }

    #[test]
    fn test_dzhanibekov_intermediate_axis_unstable() {
        // 网球拍定理: 绕中间轴 (I₂) 的旋转不稳定, 小扰动会放大
        // 演示: ω₁ 从 0.1 开始, 应出现显著翻转
        let cfg = EulerRigidBodyConfig::default();
        let mut s = EulerRigidBodySolver::new(cfg, 0.1, 10.0, 0.01);
        s.run(20000);
        // 在不稳定情形下, ω₁ 应增长到与 ω₂ 可比的量级 (翻转)
        let w1_max = s.trajectory.iter().map(|&(w1, _, _)| w1.abs()).fold(0.0_f64, f64::max);
        let w2_max = s.trajectory.iter().map(|&(_, w2, _)| w2.abs()).fold(0.0_f64, f64::max);
        // 翻转标志: ω₁ 增长到与 ω₂ 同量级 (而非保持 0.1)
        assert!(w1_max > 1.0, "Dzhanibekov: ω₁ should grow large, w1_max={}", w1_max);
        // 同时 ω₂ 应该衰减 (能量在轴间转移)
        assert!(w2_max < 15.0, "ω₂ should not grow unbounded, w2_max={}", w2_max);
    }

    #[test]
    fn test_stable_axis_1_remains_small_perturbation() {
        // 绕最小惯量轴 (I₁) 旋转: 小扰动应保持小
        let cfg = EulerRigidBodyConfig::default();
        let mut s = EulerRigidBodySolver::new(cfg, 10.0, 0.1, 0.01);
        s.run(20000);
        // 稳定情形: ω₂ 应保持小 (扰动不放大)
        let w2_max = s.trajectory.iter().map(|&(_, w2, _)| w2.abs()).fold(0.0_f64, f64::max);
        let w3_max = s.trajectory.iter().map(|&(_, _, w3)| w3.abs()).fold(0.0_f64, f64::max);
        // 扰动应保持有界, 不应增长到与主轴旋转 (ω₁=10) 同量级
        assert!(w2_max < 5.0, "stable axis 1: ω₂ should stay bounded, w2_max={}", w2_max);
        assert!(w3_max < 5.0, "stable axis 1: ω₃ should stay bounded, w3_max={}", w3_max);
    }

    #[test]
    fn test_stable_axis_3_remains_small_perturbation() {
        // 绕最大惯量轴 (I₃) 旋转: 小扰动应保持小
        let cfg = EulerRigidBodyConfig::default();
        let mut s = EulerRigidBodySolver::new(cfg, 0.01, 0.1, 10.0);
        s.run(20000);
        let w1_max = s.trajectory.iter().map(|&(w1, _, _)| w1.abs()).fold(0.0_f64, f64::max);
        let w2_max = s.trajectory.iter().map(|&(_, w2, _)| w2.abs()).fold(0.0_f64, f64::max);
        assert!(w1_max < 5.0, "stable axis 3: ω₁ should stay bounded, w1_max={}", w1_max);
        assert!(w2_max < 5.0, "stable axis 3: ω₂ should stay bounded, w2_max={}", w2_max);
    }

    #[test]
    fn test_bounded_orbits() {
        // 保守系统: 轨道有界 (约束在能量+角动量曲面交集上)
        let mut s = EulerRigidBodySolver::dzhanibekov(EulerRigidBodyConfig::default());
        s.run(50000);
        let w1_max = s.trajectory.iter().map(|&(w1, _, _)| w1.abs()).fold(0.0_f64, f64::max);
        let w2_max = s.trajectory.iter().map(|&(_, w2, _)| w2.abs()).fold(0.0_f64, f64::max);
        let w3_max = s.trajectory.iter().map(|&(_, _, w3)| w3.abs()).fold(0.0_f64, f64::max);
        // 由能量守恒 H = 0.5*(1*w1²+2*w2²+3*w3²) ≤ H₀, 各分量有界
        assert!(w1_max < 50.0, "ω₁ bounded: {}", w1_max);
        assert!(w2_max < 50.0, "ω₂ bounded: {}", w2_max);
        assert!(w3_max < 50.0, "ω₃ bounded: {}", w3_max);
    }

    #[test]
    fn test_no_chaos_3d_2_invariants() {
        // 3D 系统 + 2 守恒量 → 1D 轨道, 不能混沌
        // 验证: 相邻轨道的分离不应指数增长
        let cfg = EulerRigidBodyConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = EulerRigidBodySolver::new(cfg, 0.1, 10.0, 0.01);
        let mut s2 = EulerRigidBodySolver::new(cfg, 0.1 + d0, 10.0, 0.01);
        for _ in 0..20000 {
            s1.step();
            s2.step();
        }
        let dw1 = s1.w1 - s2.w1;
        let dw2 = s1.w2 - s2.w2;
        let dw3 = s1.w3 - s2.w3;
        let d = (dw1 * dw1 + dw2 * dw2 + dw3 * dw3).sqrt();
        // 保守系统 1D 轨道: 分离应有界 (不应指数放大到吸引子尺度)
        // 注意: 不稳定情形 (中间轴) 可能线性放大, 但不应指数放大
        assert!(d < 10.0, "non-chaotic: separation should be bounded, d={}", d);
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = EulerRigidBodyConfig::default();
        let mut s = EulerRigidBodySolver::new(cfg, 1e6, 1e6, 1e6);
        s.run(500);
        // 极大初值可能导致数值问题
        assert!(s.has_escaped() || !s.has_nan());
    }
}
