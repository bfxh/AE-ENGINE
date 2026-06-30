//! Van der Pol Oscillator — 自激振荡极限环 (非线性电路经典)
//!
//! Balthasar van der Pol 1927 年研究真空管电路时提出的极限环振子,
//! 是非线性动力学与自激振荡的范例. 不同于 Duffing (外驱动) 和
//! Lorenz (混沌), VdP 在无外驱动下产生稳定极限环 — 自维持周期
//! 振荡的典型范例.
//!
//! 方程:
//!   ẍ - μ (1 - x²) ẋ + x = 0
//!
//! 一阶系统:
//!   ẋ = v
//!   ẏ = μ (1 - x²) v - x
//!
//! 参数:
//!   μ = 非线性阻尼系数
//!     - μ = 0: 简谐振子 (ẍ + x = 0), 中心, 闭轨族
//!     - μ > 0: 极限环稳定 (能量在 |x|<1 注入, |x|>1 耗散)
//!     - μ < 0: 极限环不稳定 (能量反向)
//!
//! 极限环行为 (随 μ):
//!   - μ → 0: 接近正弦, 振幅 ~2, 周期 ~2π
//!   - μ >> 1: 弛豫振荡 (慢-快), 周期 ~ (3 - 2 ln 2) μ
//!     特征: 慢上升 x~+2, 快跳跃 x~+2→-2, 慢下降 x~-2, 快跳跃回 +2
//!
//! 强迫 VdP (周期外驱动):
//!   ẍ - μ(1-x²)ẋ + x = A cos(ω t)
//!   - 共振 / 反共振, 锁相 (Poincaré map 周期 N)
//!   - μ 大时混沌
//!
//! 数值方法:
//!   4 阶 Runge-Kutta (RK4)
//!
//! 应用:
//!   - 电子电路 (VdP 振荡器, 多谐振荡器)
//!   - 心脏起搏细胞 (Hodgkin-Huxley 简化版)
//!   - 生物节律 (昼夜节律, 神经振荡)
//!   - 声学 (管乐器发声)
//!   - 机械 (轮轴颤振, 摩擦振子)
//!
//! 基于:
//!   - van der Pol, B. 1927. "On relaxation-oscillations."
//!     Phil. Mag. 2, 978.
//!   - van der Pol, B. & van der Mark, J. 1928. "The heartbeat
//!     considered as a relaxation oscillation." Phil. Mag. 6, 763.
//!   - Strogatz, S.H. 2018. "Nonlinear Dynamics and Chaos." CRC Press.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VdpConfig {
    /// 时间步长
    pub dt: f32,
    /// 非线性阻尼 μ (>0 自激, <0 反向)
    pub mu: f32,
    /// 强迫振幅 (0 = 自由)
    pub forcing_amplitude: f32,
    /// 强迫频率
    pub forcing_omega: f32,
}

impl Default for VdpConfig {
    fn default() -> Self {
        VdpConfig {
            dt: 0.01,
            mu: 1.0,
            forcing_amplitude: 0.0,
            forcing_omega: 1.0,
        }
    }
}

impl VdpConfig {
    pub fn is_free(&self) -> bool {
        self.forcing_amplitude.abs() < 1e-6
    }

    pub fn is_self_oscillatory(&self) -> bool {
        self.mu > 0.0
    }

    /// 弛豫振荡周期估计 (μ 大时): T ≈ (3 - 2 ln 2) μ ≈ 1.614 μ
    pub fn relaxation_period(&self) -> f32 {
        (3.0 - 2.0 * (2.0_f32).ln()) * self.mu
    }

    /// 简谐近似周期 (μ → 0): T = 2π
    pub fn harmonic_period(&self) -> f32 {
        2.0 * std::f32::consts::PI
    }
}

pub struct VdpSolver {
    pub config: VdpConfig,
    /// 位置 x
    pub x: f32,
    /// 速度 v
    pub v: f32,
    pub time: f32,
    pub steps: usize,
    /// 轨迹缓存 (用于极限环分析)
    pub trajectory: Vec<(f32, f32, f32)>,
}

impl VdpSolver {
    pub fn new(config: VdpConfig) -> Self {
        VdpSolver {
            config,
            x: 0.0,
            v: 0.0,
            time: 0.0,
            steps: 0,
            trajectory: Vec::new(),
        }
    }

    pub fn with_recording(config: VdpConfig, record: bool) -> Self {
        let mut s = Self::new(config);
        if record {
            s.trajectory.reserve(10000);
        }
        s
    }

    pub fn initialize(&mut self, x0: f32, v0: f32) {
        self.x = x0;
        self.v = v0;
        self.time = 0.0;
        self.steps = 0;
        self.trajectory.clear();
    }

    fn derivatives(x: f32, v: f32, t: f32, cfg: &VdpConfig) -> (f32, f32) {
        let dx = v;
        let dv = cfg.mu * (1.0 - x * x) * v - x + cfg.forcing_amplitude * (cfg.forcing_omega * t).cos();
        (dx, dv)
    }

    /// 4 阶 Runge-Kutta 单步
    pub fn step(&mut self) {
        let dt = self.config.dt;
        let t = self.time;
        let x = self.x;
        let v = self.v;

        let (k1x, k1v) = Self::derivatives(x, v, t, &self.config);
        let (k2x, k2v) = Self::derivatives(x + 0.5 * dt * k1x, v + 0.5 * dt * k1v, t + 0.5 * dt, &self.config);
        let (k3x, k3v) = Self::derivatives(x + 0.5 * dt * k2x, v + 0.5 * dt * k2v, t + 0.5 * dt, &self.config);
        let (k4x, k4v) = Self::derivatives(x + dt * k3x, v + dt * k3v, t + dt, &self.config);

        self.x += (dt / 6.0) * (k1x + 2.0 * k2x + 2.0 * k3x + k4x);
        self.v += (dt / 6.0) * (k1v + 2.0 * k2v + 2.0 * k3v + k4v);
        self.time += dt;
        self.steps += 1;

        if self.trajectory.capacity() > 0 {
            self.trajectory.push((self.x, self.v, self.time));
        }
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    pub fn has_nan(&self) -> bool {
        !self.x.is_finite() || !self.v.is_finite()
    }

    /// 当前距原点距离 (极限环 ~2)
    pub fn radius(&self) -> f32 {
        (self.x * self.x + self.v * self.v).sqrt()
    }

    /// 与另一轨迹的距离
    pub fn distance_to(&self, other: &VdpSolver) -> f32 {
        let dx = self.x - other.x;
        let dv = self.v - other.v;
        (dx * dx + dv * dv).sqrt()
    }

    /// 轨迹的 x 范围 (用于估计极限环大小)
    pub fn trajectory_bounds(&self) -> (f32, f32, f32, f32) {
        let mut x_min = f32::INFINITY;
        let mut x_max = f32::NEG_INFINITY;
        let mut v_min = f32::INFINITY;
        let mut v_max = f32::NEG_INFINITY;
        for &(x, v, _) in &self.trajectory {
            if x < x_min {
                x_min = x;
            }
            if x > x_max {
                x_max = x;
            }
            if v < v_min {
                v_min = v;
            }
            if v > v_max {
                v_max = v;
            }
        }
        (x_min, x_max, v_min, v_max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = VdpConfig::default();
        assert!(cfg.dt > 0.0);
        assert_eq!(cfg.mu, 1.0);
        assert!(cfg.is_free());
        assert!(cfg.is_self_oscillatory());
    }

    #[test]
    fn test_harmonic_period() {
        let cfg = VdpConfig::default();
        assert!((cfg.harmonic_period() - 2.0 * std::f32::consts::PI).abs() < 1e-5);
    }

    #[test]
    fn test_relaxation_period() {
        let cfg = VdpConfig {
            mu: 10.0,
            ..Default::default()
        };
        let t = cfg.relaxation_period();
        // T ≈ 1.614 * 10 = 16.14
        assert!((t - 16.14).abs() < 0.5, "relaxation period: {}", t);
    }

    #[test]
    fn test_solver_creation() {
        let s = VdpSolver::new(VdpConfig::default());
        assert_eq!(s.steps, 0);
        assert_eq!(s.time, 0.0);
    }

    #[test]
    fn test_initialize() {
        let mut s = VdpSolver::new(VdpConfig::default());
        s.initialize(1.5, 0.5);
        assert!((s.x - 1.5).abs() < 1e-6);
        assert!((s.v - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = VdpSolver::new(VdpConfig::default());
        s.initialize(1.0, 0.0);
        let t0 = s.time;
        s.step();
        assert!(s.time > t0);
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = VdpSolver::new(VdpConfig::default());
        s.initialize(1.0, 0.0);
        s.step_n(100);
        assert_eq!(s.steps, 100);
    }

    #[test]
    fn test_no_nan_short_run() {
        let mut s = VdpSolver::new(VdpConfig::default());
        s.initialize(1.0, 0.0);
        s.step_n(2000);
        assert!(!s.has_nan(), "NaN after 2000 steps");
    }

    #[test]
    fn test_no_nan_long_run() {
        let cfg = VdpConfig {
            dt: 0.005,
            ..Default::default()
        };
        let mut s = VdpSolver::new(cfg);
        s.initialize(1.0, 0.5);
        s.step_n(20000);
        assert!(!s.has_nan(), "NaN after 20000 steps");
    }

    #[test]
    fn test_no_nan_large_mu() {
        // μ 大: 弛豫振荡, 数值上更挑战
        let cfg = VdpConfig {
            mu: 10.0,
            dt: 0.001,
            ..Default::default()
        };
        let mut s = VdpSolver::new(cfg);
        s.initialize(1.0, 0.0);
        s.step_n(20000);
        assert!(!s.has_nan(), "NaN with large mu");
    }

    #[test]
    fn test_no_nan_small_mu() {
        // μ 小: 近简谐
        let cfg = VdpConfig {
            mu: 0.1,
            ..Default::default()
        };
        let mut s = VdpSolver::new(cfg);
        s.initialize(0.5, 0.0);
        s.step_n(5000);
        assert!(!s.has_nan(), "NaN with small mu");
    }

    #[test]
    fn test_limit_cycle_amplitude() {
        // 自激 VdP 应收敛到极限环 (振幅 ~2)
        let cfg = VdpConfig {
            mu: 1.0,
            dt: 0.005,
            ..Default::default()
        };
        let mut s = VdpSolver::with_recording(cfg, true);
        s.initialize(0.1, 0.0);
        // 演化足够长使其落到极限环上
        s.step_n(5000);
        // 再演化一个周期收集轨迹
        s.trajectory.clear();
        s.step_n(2000);
        let (x_min, x_max, _, _) = s.trajectory_bounds();
        // 极限环 x 范围应 ~ [-2, 2]
        assert!(
            x_max > 1.5 && x_max < 2.5,
            "limit cycle x_max: {}",
            x_max
        );
        assert!(
            x_min < -1.5 && x_min > -2.5,
            "limit cycle x_min: {}",
            x_min
        );
    }

    #[test]
    fn test_limit_cycle_attracts_inside() {
        // 从小初值出发应增大到极限环
        let cfg = VdpConfig {
            mu: 1.0,
            dt: 0.005,
            ..Default::default()
        };
        let mut s = VdpSolver::new(cfg);
        s.initialize(0.01, 0.0);
        let r0 = s.radius();
        s.step_n(5000);
        let r1 = s.radius();
        assert!(r1 > r0, "should grow from small IC: {} -> {}", r0, r1);
        // 极限环半径 ~2
        assert!(
            r1 < 3.0 && r1 > 1.5,
            "should approach limit cycle radius: {}",
            r1
        );
    }

    #[test]
    fn test_limit_cycle_attracts_outside() {
        // 从大初值出发应减小到极限环
        let cfg = VdpConfig {
            mu: 1.0,
            dt: 0.005,
            ..Default::default()
        };
        let mut s = VdpSolver::new(cfg);
        s.initialize(5.0, 5.0);
        let r0 = s.radius();
        s.step_n(5000);
        let r1 = s.radius();
        assert!(r1 < r0, "should shrink from large IC: {} -> {}", r0, r1);
        assert!(
            r1 < 5.0,
            "should approach limit cycle: {}",
            r1
        );
    }

    #[test]
    fn test_zero_mu_harmonic_oscillation() {
        // μ=0: 简谐振子, 闭轨族 (能量守恒)
        let cfg = VdpConfig {
            mu: 0.0,
            dt: 0.001,
            ..Default::default()
        };
        let mut s = VdpSolver::new(cfg);
        s.initialize(1.0, 0.0);
        let r0 = s.radius(); // 应为 1
        s.step_n(2000);
        let r1 = s.radius();
        // 简谐振子: r 应近似不变
        assert!(
            (r1 - r0).abs() < 0.05 * r0,
            "harmonic oscillator radius not conserved: {} -> {}",
            r0,
            r1
        );
    }

    #[test]
    fn test_negative_mu_unstable() {
        // μ<0: 极限环不稳定, 原点变稳定
        let cfg = VdpConfig {
            mu: -1.0,
            dt: 0.005,
            ..Default::default()
        };
        let mut s = VdpSolver::new(cfg);
        s.initialize(1.0, 0.0);
        s.step_n(5000);
        // 应衰减到原点
        assert!(
            s.radius() < 0.5,
            "negative mu should decay to origin: {}",
            s.radius()
        );
    }

    #[test]
    fn test_relaxation_oscillation_period() {
        // μ 大: 弛豫振荡, 周期随 μ 增长 (渐近 T ~ 1.614 μ)
        // 仅验证周期随 μ 增大, 数量级合理
        let cfg = VdpConfig {
            mu: 10.0,
            dt: 0.001,
            ..Default::default()
        };
        let mut s = VdpSolver::with_recording(cfg.clone(), true);
        s.initialize(1.0, 0.0);
        // 演化足够长使其落到极限环
        s.step_n(50000);
        s.trajectory.clear();
        s.step_n(100000); // 长时间收集
        // 统计 x 正向零点跨越: 一个周期一次
        let mut zero_crossings = 0;
        let mut prev_x = s.trajectory.first().map(|p| p.0).unwrap_or(0.0);
        for &(x, _, _) in s.trajectory.iter().skip(1) {
            if prev_x < 0.0 && x >= 0.0 {
                zero_crossings += 1;
            }
            prev_x = x;
        }
        let total_time = s.trajectory.last().map(|p| p.2).unwrap_or(0.0)
            - s.trajectory.first().map(|p| p.2).unwrap_or(0.0);
        assert!(zero_crossings >= 2, "insufficient zero crossings: {}", zero_crossings);
        let period = total_time / (zero_crossings as f32);
        let expected = cfg.relaxation_period();
        // 渐近公式 (3 - 2 ln 2) μ 在 μ=10 时精度有限, 容忍 50%
        assert!(
            (period - expected).abs() < 0.6 * expected,
            "relaxation period: got {}, expected ~{}",
            period,
            expected
        );
    }

    #[test]
    fn test_distance_to_identical() {
        let s1 = VdpSolver::new(VdpConfig::default());
        let s2 = VdpSolver::new(VdpConfig::default());
        assert!(s1.distance_to(&s2) < 1e-6);
    }

    #[test]
    fn test_radius_zero_at_origin() {
        let s = VdpSolver::new(VdpConfig::default());
        assert!(s.radius() < 1e-6);
    }

    #[test]
    fn test_forced_bounded() {
        // 强迫 VdP: 振幅应有界
        let cfg = VdpConfig {
            mu: 1.0,
            forcing_amplitude: 1.0,
            forcing_omega: 1.0,
            dt: 0.005,
        };
        let mut s = VdpSolver::new(cfg);
        s.initialize(1.0, 0.0);
        s.step_n(10000);
        assert!(!s.has_nan(), "NaN in forced case");
        assert!(s.radius() < 10.0, "forced should be bounded: {}", s.radius());
    }

    #[test]
    fn test_trajectory_bounds() {
        let cfg = VdpConfig {
            mu: 1.0,
            dt: 0.005,
            ..Default::default()
        };
        let mut s = VdpSolver::with_recording(cfg, true);
        s.initialize(1.0, 0.0);
        s.step_n(2000);
        let (x_min, x_max, v_min, v_max) = s.trajectory_bounds();
        assert!(x_min < x_max);
        assert!(v_min < v_max);
    }

    #[test]
    fn test_phase_space_radius_growth_from_origin() {
        // 从原点 (不稳定不动点) 微扰出发, 半径应增长
        let cfg = VdpConfig {
            mu: 1.0,
            dt: 0.005,
            ..Default::default()
        };
        let mut s = VdpSolver::new(cfg);
        s.initialize(0.001, 0.0);
        let r0 = s.radius();
        s.step_n(2000);
        let r1 = s.radius();
        assert!(r1 > r0, "origin is unstable for mu>0: {} -> {}", r0, r1);
    }
}
