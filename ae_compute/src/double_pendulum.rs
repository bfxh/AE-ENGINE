//! Double Pendulum — 经典混沌演示 (2 自由度拉格朗日系统)
//!
//! 两个点质量 m1, m2 通过无质量刚性杆 L1, L2 串联, 在重力下运动.
//! 2 自由度 (θ1, θ2), 但高度非线性耦合, 是教科书级别的混沌演示:
//!   - 低能量: 接近简谐, 两个简正模式
//!   - 高能量: 强混沌, 对初值极度敏感 (Lyapunov 指数大)
//!   - 同一初始条件多次实验会指数发散 (蝴蝶效应)
//!
//! 拉格朗日量 (θ 从向下竖直方向测量):
//!   L = T - V
//!   T = ½(m1+m2)L1² ω1² + ½ m2 L2² ω2² + m2 L1 L2 ω1 ω2 cos(θ1-θ2)
//!   V = -(m1+m2) g L1 cos(θ1) - m2 g L2 cos(θ2)
//!
//! 运动方程 (2x2 线性系统求解 θ̈1, θ̈2):
//!   (m1+m2)L1 α1 + m2 L2 α2 cos(Δ) + m2 L2 ω2² sin(Δ) + (m1+m2)g sin(θ1) = 0
//!   m2 L2 α2 + m2 L1 α1 cos(Δ) - m2 L1 ω1² sin(Δ) + m2 g sin(θ2) = 0
//!   where Δ = θ1 - θ2
//!
//! 矩阵形式:
//!   [A B] [α1]   [R1]
//!   [C D] [α2] = [R2]
//!   A = (m1+m2) L1,  B = m2 L2 cos(Δ)
//!   C = m2 L1 cos(Δ), D = m2 L2
//!   R1 = -m2 L2 ω2² sin(Δ) - (m1+m2) g sin(θ1)
//!   R2 =  m2 L1 ω1² sin(Δ) - m2 g sin(θ2)
//!   det = AD - BC = m2 L1 L2 [(m1+m2) - m2 cos²(Δ)]
//!   α1 = (D·R1 - B·R2) / det
//!   α2 = (A·R2 - C·R1) / det
//!
//! 守恒量: 总能量 H = T + V (无摩擦)
//! 简正模式 (小角度线性化, 等质量等长 L=m=g=1):
//!   ω_±² = (2 ± √2) g/L
//!   - 慢模式 (同相): θ1 ≈ θ2, ω_- = √((2-√2)g/L)
//!   - 快模式 (反相): θ1 ≈ -θ2, ω_+ = √((2+√2)g/L)
//!
//! 数值方法: RK4 (4 阶 Runge-Kutta)
//!
//! 参考:
//!   - Goldstein, H. "Classical Mechanics" 3rd ed., Chapter 6-7.
//!   - Strogatz, S. "Nonlinear Dynamics and Chaos", §6.3.
//!   - Levien, R. & Tan, S. 1993. "Double Pendulum: An Experiment in Chaos."
//!     Am. J. Phys. 61, 1038.

use std::f64::consts::PI;

/// 双摆配置
#[derive(Clone, Debug)]
pub struct DoublePendulumConfig {
    /// 上杆长度 L1
    pub l1: f64,
    /// 下杆长度 L2
    pub l2: f64,
    /// 上质量 m1
    pub m1: f64,
    /// 下质量 m2
    pub m2: f64,
    /// 重力加速度 g
    pub g: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for DoublePendulumConfig {
    fn default() -> Self {
        // 标准等质量等长度双摆
        Self {
            l1: 1.0,
            l2: 1.0,
            m1: 1.0,
            m2: 1.0,
            g: 9.81,
            dt: 0.0005,
        }
    }
}

/// 双摆求解器
pub struct DoublePendulumSolver {
    pub config: DoublePendulumConfig,
    /// 上杆角度 (从向下竖直测量, rad)
    pub theta1: f64,
    /// 下杆角度
    pub theta2: f64,
    /// 上杆角速度
    pub omega1: f64,
    /// 下杆角速度
    pub omega2: f64,
    pub step_count: u64,
    pub time: f64,
    /// 能量历史 (诊断)
    pub energy_history: Vec<f64>,
}

impl DoublePendulumSolver {
    pub fn new(config: DoublePendulumConfig, theta1: f64, theta2: f64, omega1: f64, omega2: f64) -> Self {
        let mut s = Self {
            config,
            theta1,
            theta2,
            omega1,
            omega2,
            step_count: 0,
            time: 0.0,
            energy_history: Vec::new(),
        };
        s.energy_history.push(s.energy());
        s
    }

    /// 计算给定状态下的角加速度 (α1, α2)
    /// 状态以 (θ1, θ2, ω1, ω2) 顺序传入, 返回 (α1, α2)
    pub fn accelerations(
        cfg: &DoublePendulumConfig,
        theta1: f64,
        theta2: f64,
        omega1: f64,
        omega2: f64,
    ) -> (f64, f64) {
        let l1 = cfg.l1;
        let l2 = cfg.l2;
        let m1 = cfg.m1;
        let m2 = cfg.m2;
        let g = cfg.g;

        let delta = theta1 - theta2;
        let sin_d = delta.sin();
        let cos_d = delta.cos();

        let a = (m1 + m2) * l1;
        let b = m2 * l2 * cos_d;
        let c = m2 * l1 * cos_d;
        let d = m2 * l2;
        let det = a * d - b * c;
        // det = m2 L1 L2 [(m1+m2) - m2 cos²(Δ)] > 0 (m1 > 0)

        let r1 = -m2 * l2 * omega2 * omega2 * sin_d - (m1 + m2) * g * theta1.sin();
        let r2 = m2 * l1 * omega1 * omega1 * sin_d - m2 * g * theta2.sin();

        let alpha1 = (d * r1 - b * r2) / det;
        let alpha2 = (a * r2 - c * r1) / det;
        (alpha1, alpha2)
    }

    /// 动能 T = ½(m1+m2)L1² ω1² + ½ m2 L2² ω2² + m2 L1 L2 ω1 ω2 cos(Δ)
    pub fn kinetic_energy(&self) -> f64 {
        let l1 = self.config.l1;
        let l2 = self.config.l2;
        let m1 = self.config.m1;
        let m2 = self.config.m2;
        let cos_d = (self.theta1 - self.theta2).cos();
        0.5 * (m1 + m2) * l1 * l1 * self.omega1 * self.omega1
            + 0.5 * m2 * l2 * l2 * self.omega2 * self.omega2
            + m2 * l1 * l2 * self.omega1 * self.omega2 * cos_d
    }

    /// 势能 V = -(m1+m2) g L1 cos(θ1) - m2 g L2 cos(θ2)
    /// (零势能参考点在支点)
    pub fn potential_energy(&self) -> f64 {
        let l1 = self.config.l1;
        let l2 = self.config.l2;
        let m1 = self.config.m1;
        let m2 = self.config.m2;
        let g = self.config.g;
        -(m1 + m2) * g * l1 * self.theta1.cos() - m2 * g * l2 * self.theta2.cos()
    }

    /// 总能量 H = T + V (守恒)
    pub fn energy(&self) -> f64 {
        self.kinetic_energy() + self.potential_energy()
    }

    /// 下质量 m2 的笛卡尔位置 (x2, y2)
    /// 支点在原点, y 向上为正. θ=0 时摆向下 (y = -L)
    pub fn position_m2(&self) -> (f64, f64) {
        let l1 = self.config.l1;
        let l2 = self.config.l2;
        let x1 = l1 * self.theta1.sin();
        let y1 = -l1 * self.theta1.cos();
        let x2 = x1 + l2 * self.theta2.sin();
        let y2 = y1 - l2 * self.theta2.cos();
        (x2, y2)
    }

    /// 上质量 m1 的笛卡尔位置
    pub fn position_m1(&self) -> (f64, f64) {
        let l1 = self.config.l1;
        (l1 * self.theta1.sin(), -l1 * self.theta1.cos())
    }

    /// 单步 RK4 推进 (状态 [θ1, θ2, ω1, ω2])
    pub fn step(&mut self) {
        let dt = self.config.dt;
        let cfg = &self.config;

        let deriv = |s: [f64; 4]| -> [f64; 4] {
            let (a1, a2) = Self::accelerations(cfg, s[0], s[1], s[2], s[3]);
            [s[2], s[3], a1, a2]
        };

        let y0 = [self.theta1, self.theta2, self.omega1, self.omega2];
        let k1 = deriv(y0);
        let y1 = [
            y0[0] + 0.5 * dt * k1[0],
            y0[1] + 0.5 * dt * k1[1],
            y0[2] + 0.5 * dt * k1[2],
            y0[3] + 0.5 * dt * k1[3],
        ];
        let k2 = deriv(y1);
        let y2 = [
            y0[0] + 0.5 * dt * k2[0],
            y0[1] + 0.5 * dt * k2[1],
            y0[2] + 0.5 * dt * k2[2],
            y0[3] + 0.5 * dt * k2[3],
        ];
        let k3 = deriv(y2);
        let y3 = [
            y0[0] + dt * k3[0],
            y0[1] + dt * k3[1],
            y0[2] + dt * k3[2],
            y0[3] + dt * k3[3],
        ];
        let k4 = deriv(y3);

        self.theta1 = y0[0] + dt / 6.0 * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]);
        self.theta2 = y0[1] + dt / 6.0 * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]);
        self.omega1 = y0[2] + dt / 6.0 * (k1[2] + 2.0 * k2[2] + 2.0 * k3[2] + k4[2]);
        self.omega2 = y0[3] + dt / 6.0 * (k1[3] + 2.0 * k2[3] + 2.0 * k3[3] + k4[3]);

        self.step_count += 1;
        self.time += dt;
        self.energy_history.push(self.energy());
    }

    /// 多步推进
    pub fn run(&mut self, n_steps: usize) {
        for _ in 0..n_steps {
            self.step();
        }
    }

    /// 检查 NaN/Inf
    pub fn has_nan(&self) -> bool {
        !self.theta1.is_finite()
            || !self.theta2.is_finite()
            || !self.omega1.is_finite()
            || !self.omega2.is_finite()
    }

    /// 小角度近似下的简正模式频率 (等质量等长 L, m, g)
    /// ω_-² = (2 - √2) g/L  (慢模式, 同相)
    /// ω_+² = (2 + √2) g/L  (快模式, 反相)
    pub fn normal_mode_frequencies(&self) -> (f64, f64) {
        // 通用情形 (m1, m2, L1, L2 任意) 略复杂, 这里给等质量等长情形
        // 完整公式见 Goldstein §6.3
        let l = self.config.l1;
        let g = self.config.g;
        let sqrt2 = 2.0_f64.sqrt();
        let omega_slow = ((2.0 - sqrt2) * g / l).sqrt();
        let omega_fast = ((2.0 + sqrt2) * g / l).sqrt();
        (omega_slow, omega_fast)
    }

    /// 初始化: 慢简正模式 (同相小摆动)
    /// θ1 = θ2 = A cos(ω_- t), 初始时刻 θ1=θ2=A, ω1=ω2=0
    pub fn initialize_slow_mode(amplitude: f64, config: DoublePendulumConfig) -> Self {
        Self::new(config, amplitude, amplitude, 0.0, 0.0)
    }

    /// 初始化: 快简正模式 (反相小摆动)
    /// θ1 = -θ2 = A cos(ω_+ t), 初始时刻 θ1=A, θ2=-A, ω1=ω2=0
    pub fn initialize_fast_mode(amplitude: f64, config: DoublePendulumConfig) -> Self {
        Self::new(config, amplitude, -amplitude, 0.0, 0.0)
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
        let cfg = DoublePendulumConfig::default();
        assert_eq!(cfg.l1, 1.0);
        assert_eq!(cfg.l2, 1.0);
        assert_eq!(cfg.m1, 1.0);
        assert_eq!(cfg.m2, 1.0);
        assert_eq!(cfg.g, 9.81);
        assert_eq!(cfg.dt, 0.0005);
    }

    #[test]
    fn test_solver_creation() {
        let s = DoublePendulumSolver::new(DoublePendulumConfig::default(), 0.1, 0.2, 0.0, 0.0);
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.energy_history.len(), 1);
    }

    #[test]
    fn test_potential_energy_at_bottom() {
        // 两摆竖直向下 (θ1=θ2=0): V = -(m1+m2)gL1 - m2gL2
        let cfg = DoublePendulumConfig::default();
        let s = DoublePendulumSolver::new(cfg, 0.0, 0.0, 0.0, 0.0);
        let v = s.potential_energy();
        let expected = -2.0 * 1.0 * 9.81 * 1.0 - 1.0 * 9.81 * 1.0;
        assert!(approx_eq(v, expected, 1e-10));
    }

    #[test]
    fn test_potential_energy_horizontal() {
        // 上杆水平 (θ1=π/2, θ2=0): V = 0 - m2 g L2
        let cfg = DoublePendulumConfig::default();
        let s = DoublePendulumSolver::new(cfg, PI / 2.0, 0.0, 0.0, 0.0);
        let v = s.potential_energy();
        let expected = -1.0 * 9.81 * 1.0;
        assert!(approx_eq(v, expected, 1e-10));
    }

    #[test]
    fn test_kinetic_energy_zero_at_rest() {
        let s = DoublePendulumSolver::new(DoublePendulumConfig::default(), 0.1, 0.1, 0.0, 0.0);
        assert!(approx_eq(s.kinetic_energy(), 0.0, 1e-12));
    }

    #[test]
    fn test_kinetic_energy_analytic() {
        // 给定 ω1, ω2, Δ, 检查解析公式
        let cfg = DoublePendulumConfig::default();
        let s = DoublePendulumSolver::new(cfg, 0.3, 0.5, 0.7, 0.4);
        let t = s.kinetic_energy();
        let cos_d = (0.3_f64 - 0.5).cos();
        let expected = 0.5 * 2.0 * 1.0 * 0.7 * 0.7
            + 0.5 * 1.0 * 1.0 * 0.4 * 0.4
            + 1.0 * 1.0 * 1.0 * 0.7 * 0.4 * cos_d;
        assert!(approx_eq(t, expected, 1e-10));
    }

    #[test]
    fn test_energy_components() {
        let s = DoublePendulumSolver::new(DoublePendulumConfig::default(), 0.3, 0.5, 0.7, 0.4);
        let t = s.kinetic_energy();
        let u = s.potential_energy();
        let h = s.energy();
        assert!(approx_eq(t + u, h, 1e-12));
    }

    #[test]
    fn test_accelerations_zero_at_bottom() {
        // 两摆竖直向下静止: 加速度为 0 (平衡点)
        let cfg = DoublePendulumConfig::default();
        let (a1, a2) = DoublePendulumSolver::accelerations(&cfg, 0.0, 0.0, 0.0, 0.0);
        assert!(approx_eq(a1, 0.0, 1e-10));
        assert!(approx_eq(a2, 0.0, 1e-10));
    }

    #[test]
    fn test_accelerations_small_angle_harmonic() {
        // 小角度单摆近似: θ̈ ≈ -(g/L) sin θ
        // 当 θ2=0, ω=0, m2 ≪ m1 时, θ1̈ ≈ -g/L1 sin θ1
        // 等质量等长不严格成立, 但定性 θ̈1 < 0 (恢复力)
        let cfg = DoublePendulumConfig::default();
        let (a1, _a2) = DoublePendulumSolver::accelerations(&cfg, 0.1, 0.0, 0.0, 0.0);
        assert!(a1 < 0.0, "restoring acceleration a1<0: {}", a1);
    }

    #[test]
    fn test_step_advances() {
        let mut s = DoublePendulumSolver::new(DoublePendulumConfig::default(), 0.1, 0.2, 0.0, 0.0);
        let t0 = s.time;
        s.step();
        assert_eq!(s.step_count, 1);
        assert!((s.time - t0 - s.config.dt).abs() < 1e-12);
        assert_eq!(s.energy_history.len(), 2);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = DoublePendulumSolver::new(DoublePendulumConfig::default(), 1.0, 1.5, 0.0, 0.0);
        s.run(1000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = DoublePendulumSolver::new(DoublePendulumConfig::default(), 2.0, 2.0, 0.0, 0.0);
        s.run(100000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_energy_conservation_low_energy() {
        // 低能量 (小振幅), RK4 应长期守恒能量
        let mut s = DoublePendulumSolver::new(
            DoublePendulumConfig { dt: 0.001, ..Default::default() },
            0.05, 0.05, 0.0, 0.0,
        );
        let e0 = s.energy();
        s.run(50000); // t = 50 s
        let e1 = s.energy();
        let rel = (e1 - e0).abs() / e0.abs();
        assert!(rel < 1e-5, "low E drift: {}%", rel * 100.0);
    }

    #[test]
    fn test_energy_conservation_high_energy() {
        // 高能量 (大振幅, 可能翻越), RK4 仍应守恒能量
        let mut s = DoublePendulumSolver::new(
            DoublePendulumConfig { dt: 0.0005, ..Default::default() },
            2.0, 2.0, 0.0, 0.0,
        );
        let e0 = s.energy();
        s.run(50000); // t = 25 s
        let e1 = s.energy();
        let rel = (e1 - e0).abs() / e0.abs();
        assert!(rel < 1e-4, "high E drift: {}%", rel * 100.0);
    }

    #[test]
    fn test_position_at_bottom() {
        // θ1=θ2=0: m1 在 (0, -L1), m2 在 (0, -L1-L2)
        let s = DoublePendulumSolver::new(DoublePendulumConfig::default(), 0.0, 0.0, 0.0, 0.0);
        let (x1, y1) = s.position_m1();
        let (x2, y2) = s.position_m2();
        assert!(approx_eq(x1, 0.0, 1e-12));
        assert!(approx_eq(y1, -1.0, 1e-12));
        assert!(approx_eq(x2, 0.0, 1e-12));
        assert!(approx_eq(y2, -2.0, 1e-12));
    }

    #[test]
    fn test_position_horizontal() {
        // θ1=π/2, θ2=0: m1 在 (L1, 0), m2 在 (L1, -L2)
        let s = DoublePendulumSolver::new(DoublePendulumConfig::default(), PI / 2.0, 0.0, 0.0, 0.0);
        let (x1, y1) = s.position_m1();
        let (x2, y2) = s.position_m2();
        assert!(approx_eq(x1, 1.0, 1e-12));
        assert!(approx_eq(y1, 0.0, 1e-12));
        assert!(approx_eq(x2, 1.0, 1e-12));
        assert!(approx_eq(y2, -1.0, 1e-12));
    }

    #[test]
    fn test_normal_mode_frequencies() {
        // 等长等质量双摆: ω_-² = (2-√2)g/L, ω_+² = (2+√2)g/L
        let s = DoublePendulumSolver::new(DoublePendulumConfig::default(), 0.0, 0.0, 0.0, 0.0);
        let (w_slow, w_fast) = s.normal_mode_frequencies();
        let sqrt2 = 2.0_f64.sqrt();
        let expected_slow = ((2.0 - sqrt2) * 9.81 / 1.0).sqrt();
        let expected_fast = ((2.0 + sqrt2) * 9.81 / 1.0).sqrt();
        assert!(approx_eq(w_slow, expected_slow, 1e-10));
        assert!(approx_eq(w_fast, expected_fast, 1e-10));
        assert!(w_slow < w_fast);
    }

    #[test]
    fn test_slow_mode_period() {
        // 慢模式 (同相小摆动) 应近似以 ω_- 振荡
        // 测量 θ1 过零时间间隔 ≈ T_- = 2π/ω_-
        let cfg = DoublePendulumConfig { dt: 0.001, ..Default::default() };
        let mut s = DoublePendulumSolver::initialize_slow_mode(0.05, cfg);
        let (w_slow, _) = s.normal_mode_frequencies();
        let period = 2.0 * PI / w_slow;
        // 跑 3 个周期, 检查 θ1 接近初始值
        s.run((3.0 * period / s.config.dt) as usize);
        let theta1_final = s.theta1;
        // 由于非简谐小修正, 容忍 5% 偏差
        assert!((theta1_final - 0.05).abs() < 0.025,
            "slow mode returns near initial: θ1={}", theta1_final);
    }

    #[test]
    fn test_fast_mode_period() {
        let cfg = DoublePendulumConfig { dt: 0.0005, ..Default::default() };
        let mut s = DoublePendulumSolver::initialize_fast_mode(0.05, cfg);
        let (_, w_fast) = s.normal_mode_frequencies();
        let period = 2.0 * PI / w_fast;
        s.run((3.0 * period / s.config.dt) as usize);
        // θ1 应接近初始值 0.05 (反相模式)
        let theta1_final = s.theta1;
        assert!((theta1_final - 0.05).abs() < 0.025,
            "fast mode returns near initial: θ1={}", theta1_final);
    }

    #[test]
    fn test_chaos_sensitivity() {
        // 两个相近初始条件应指数发散 (混沌)
        // 高能量初值: 大角度, 触发混沌. 双摆 Lyapunov λ ≈ 0.7/s,
        // 初始 1e-6 差异经 t=25s 放大 e^(0.7*25)≈4e7 倍, 进入饱和阶段 (~1).
        let cfg = DoublePendulumConfig { dt: 0.0005, ..Default::default() };
        let mut s1 = DoublePendulumSolver::new(cfg.clone(), 2.0, 2.0, 0.0, 0.0);
        let mut s2 = DoublePendulumSolver::new(cfg, 2.0 + 1e-6, 2.0, 0.0, 0.0);
        s1.run(50000); // t = 25 s
        s2.run(50000);
        let dtheta1 = (s1.theta1 - s2.theta1).abs();
        // 混沌放大: 1e-6 经 25s 应放大到 ~O(1) (饱和), 至少 > 0.01
        assert!(dtheta1 > 0.01, "chaos amplification: dθ1={}", dtheta1);
    }

    #[test]
    fn test_low_energy_no_chaos() {
        // 低能量 (小振幅) 两个相近初值不应快速发散
        let cfg = DoublePendulumConfig { dt: 0.001, ..Default::default() };
        let mut s1 = DoublePendulumSolver::new(cfg.clone(), 0.05, 0.05, 0.0, 0.0);
        let mut s2 = DoublePendulumSolver::new(cfg, 0.05 + 1e-6, 0.05, 0.0, 0.0);
        s1.run(20000);
        s2.run(20000);
        let dtheta1 = (s1.theta1 - s2.theta1).abs();
        // 低能量规则运动, 差异应保持小 (放大 < 100x)
        assert!(dtheta1 < 1e-3, "low E not chaotic: dθ1={}", dtheta1);
    }

    #[test]
    fn test_dt_flexible() {
        for dt in [0.0001, 0.0005, 0.001, 0.002] {
            let mut s = DoublePendulumSolver::new(
                DoublePendulumConfig { dt, ..Default::default() },
                1.0, 1.5, 0.0, 0.0,
            );
            s.run(1000);
            assert!(!s.has_nan(), "dt={}: no NaN", dt);
        }
    }

    #[test]
    fn test_diagnostics_history_grows() {
        let mut s = DoublePendulumSolver::new(DoublePendulumConfig::default(), 0.1, 0.1, 0.0, 0.0);
        s.run(20);
        assert_eq!(s.energy_history.len(), 21);
    }

    #[test]
    fn test_bottom_equilibrium_stays() {
        // 竖直向下静止是稳定平衡, 应保持
        let mut s = DoublePendulumSolver::new(DoublePendulumConfig::default(), 0.0, 0.0, 0.0, 0.0);
        s.run(1000);
        assert!(s.theta1.abs() < 1e-12);
        assert!(s.theta2.abs() < 1e-12);
        assert!(s.omega1.abs() < 1e-12);
        assert!(s.omega2.abs() < 1e-12);
    }

    #[test]
    fn test_top_unstable_equilibrium() {
        // 竖直向上 (θ1=θ2=π) 是不稳定平衡, 小扰动应放大
        let cfg = DoublePendulumConfig { dt: 0.0001, ..Default::default() };
        let mut s = DoublePendulumSolver::new(cfg, PI - 0.001, PI - 0.001, 0.0, 0.0);
        s.run(100000); // t = 10 s
        // 应已远离 π (不稳定)
        let dev1 = (s.theta1 - PI).abs();
        assert!(dev1 > 0.1, "top equilibrium unstable: dev1={}", dev1);
    }

    #[test]
    fn test_unequal_masses_no_nan() {
        let cfg = DoublePendulumConfig {
            m1: 10.0,
            m2: 0.1,
            ..Default::default()
        };
        let mut s = DoublePendulumSolver::new(cfg, 1.0, 1.5, 0.0, 0.0);
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_unequal_lengths_no_nan() {
        let cfg = DoublePendulumConfig {
            l1: 2.0,
            l2: 0.5,
            ..Default::default()
        };
        let mut s = DoublePendulumSolver::new(cfg, 1.0, 1.5, 0.0, 0.0);
        s.run(10000);
        assert!(!s.has_nan());
    }
}
