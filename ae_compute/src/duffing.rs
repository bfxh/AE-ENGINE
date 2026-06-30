//! Duffing Oscillator — 强迫阻尼双井振子 (经典混沌系统)
//!
//! Georg Duffing 1918 年提出的非线性振子方程, 是研究强迫非线性
//! 振动、分岔、混沌与奇怪吸引子的标准模型. 双井势 (α<0, β>0)
//! 下展示丰富动力学: 周期、倍周期、混沌交替.
//!
//! 方程:
//!   ẍ + δ ẋ + α x + β x³ = γ cos(ω t)
//!
//! 一阶系统:
//!   ẋ = v
//!   ẏ = -δ v - α x - β x³ + γ cos(ω t)
//!
//! 参数物理意义:
//!   δ = 阻尼系数 (>0 耗散)
//!   α = 线性刚度 (α<0 双井, α>0 单井)
//!   β = 非线性刚度 (β>0 硬弹簧, β<0 软弹簧)
//!   γ = 外驱振幅
//!   ω = 外驱频率
//!
//! 守恒量 (无阻尼无驱动 δ=γ=0):
//!   E = ½ v² + ½ α x² + ¼ β x⁴
//!
//! 势能 (双井 α<0, β>0):
//!   V(x) = ½ α x² + ¼ β x⁴
//!   极小: x* = ±sqrt(-α/β), V(x*) = -α²/(4β)
//!   极大: x = 0 (势垒)
//!
//! 经典混沌参数 (Ueda 1985):
//!   δ=0.05, α=0, β=1, γ=7.5, ω=1 → 完全混沌
//!   δ=0.08, α=-1, β=1, γ=0.2, ω=1 → 双井混沌
//!
//! 数值方法:
//!   4 阶 Runge-Kutta (RK4)
//!
//! 应用:
//!   - 机械振动 (弹簧非线性, 结构动力学)
//!   - 电路 (Duffing 振子电路实现)
//!   - 等离子体 (带电粒子在非线性波中)
//!   - 经济周期建模
//!   - 神经科学 (神经元放电动力学)
//!
//! 基于:
//!   - Duffing, G. 1918. "Erzwungene Schwingungen bei veränderlicher
//!     Eigenfrequenz und ihre technische Bedeutung." Vieweg.
//!   - Ueda, Y. 1985. "Random phenomena resulting from non-linearity
//!     in the system described by Duffing's equation."
//!     Int. J. Non-Linear Mech. 20, 481.
//!   - Holmes, P.J. 1979. "A nonlinear oscillator with a strange
//!     attractor." Phil. Trans. R. Soc. A 292, 419.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuffingConfig {
    /// 时间步长
    pub dt: f32,
    /// 阻尼系数 δ
    pub delta: f32,
    /// 线性刚度 α (α<0 双井)
    pub alpha: f32,
    /// 非线性刚度 β
    pub beta: f32,
    /// 外驱振幅 γ
    pub gamma: f32,
    /// 外驱频率 ω
    pub omega: f32,
}

impl Default for DuffingConfig {
    fn default() -> Self {
        // 双井混沌经典参数 (Holmes 1979)
        DuffingConfig {
            dt: 0.01,
            delta: 0.08,
            alpha: -1.0,
            beta: 1.0,
            gamma: 0.2,
            omega: 1.0,
        }
    }
}

impl DuffingConfig {
    /// 双井势井底位置 x* = ±sqrt(-α/β) (当 α<0, β>0)
    pub fn well_positions(&self) -> Option<(f32, f32)> {
        if self.alpha < 0.0 && self.beta > 0.0 {
            let x_star = (-self.alpha / self.beta).sqrt();
            Some((-x_star, x_star))
        } else {
            None
        }
    }

    /// 势垒高度 (双井): V(0) - V(x*) = -α²/(4β) 与 V(0)=0, 高度 α²/(4β) (取负值绝对)
    pub fn barrier_height(&self) -> f32 {
        if self.alpha < 0.0 && self.beta > 0.0 {
            self.alpha * self.alpha / (4.0 * self.beta)
        } else {
            0.0
        }
    }

    /// 势能 V(x) = ½ α x² + ¼ β x⁴
    pub fn potential(&self, x: f32) -> f32 {
        0.5 * self.alpha * x * x + 0.25 * self.beta * x.powi(4)
    }

    /// 总能量 (无驱动无阻尼): E = ½ v² + V(x)
    pub fn energy(&self, x: f32, v: f32) -> f32 {
        0.5 * v * v + self.potential(x)
    }

    /// 是否为耗散系统 (δ > 0)
    pub fn is_dissipative(&self) -> bool {
        self.delta > 0.0
    }

    /// 是否为驱动系统 (γ > 0)
    pub fn is_driven(&self) -> bool {
        self.gamma > 0.0
    }
}

pub struct DuffingSolver {
    pub config: DuffingConfig,
    /// 位置 x
    pub x: f32,
    /// 速度 v
    pub v: f32,
    pub time: f32,
    pub steps: usize,
}

impl DuffingSolver {
    pub fn new(config: DuffingConfig) -> Self {
        DuffingSolver {
            config,
            x: 0.0,
            v: 0.0,
            time: 0.0,
            steps: 0,
        }
    }

    pub fn initialize(&mut self, x0: f32, v0: f32) {
        self.x = x0;
        self.v = v0;
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化到势井底 (x*, 0)
    pub fn initialize_at_well(&mut self, positive: bool) {
        if let Some((neg, pos)) = self.config.well_positions() {
            self.initialize(if positive { pos } else { neg }, 0.0);
        } else {
            self.initialize(0.0, 0.0);
        }
    }

    /// 计算导数 (dx/dt, dv/dt) 在给定 (x, v, t)
    fn derivatives(x: f32, v: f32, t: f32, cfg: &DuffingConfig) -> (f32, f32) {
        let dx = v;
        let dv = -cfg.delta * v - cfg.alpha * x - cfg.beta * x.powi(3)
            + cfg.gamma * (cfg.omega * t).cos();
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
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    pub fn has_nan(&self) -> bool {
        !self.x.is_finite() || !self.v.is_finite()
    }

    /// 当前总能量 (含驱动? 物理上能量守恒只对 δ=γ=0 成立)
    pub fn energy(&self) -> f32 {
        self.config.energy(self.x, self.v)
    }

    /// 当前势能
    pub fn potential_energy(&self) -> f32 {
        self.config.potential(self.x)
    }

    /// 当前动能
    pub fn kinetic_energy(&self) -> f32 {
        0.5 * self.v * self.v
    }

    /// 与另一轨迹的距离 (用于 Lyapunov 指数估计)
    pub fn distance_to(&self, other: &DuffingSolver) -> f32 {
        let dx = self.x - other.x;
        let dv = self.v - other.v;
        (dx * dx + dv * dv).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = DuffingConfig::default();
        assert!(cfg.dt > 0.0);
        assert!(cfg.delta >= 0.0);
        assert_eq!(cfg.alpha, -1.0);
        assert_eq!(cfg.beta, 1.0);
        assert!(cfg.gamma >= 0.0);
        assert!(cfg.omega > 0.0);
    }

    #[test]
    fn test_well_positions() {
        let cfg = DuffingConfig::default();
        let (neg, pos) = cfg.well_positions().unwrap();
        assert!((neg - (-1.0)).abs() < 1e-5);
        assert!((pos - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_no_well_for_single_minimum() {
        let cfg = DuffingConfig {
            alpha: 1.0, // positive: single well
            ..Default::default()
        };
        assert!(cfg.well_positions().is_none());
    }

    #[test]
    fn test_barrier_height() {
        let cfg = DuffingConfig::default();
        // α=-1, β=1 → 高度 = 1/(4*1) = 0.25
        assert!((cfg.barrier_height() - 0.25).abs() < 1e-5);
    }

    #[test]
    fn test_potential_at_well_bottom() {
        let cfg = DuffingConfig::default();
        let (_, pos) = cfg.well_positions().unwrap();
        let v = cfg.potential(pos);
        // V(±1) = -0.5 + 0.25 = -0.25
        assert!((v - (-0.25)).abs() < 1e-5);
    }

    #[test]
    fn test_potential_at_origin() {
        let cfg = DuffingConfig::default();
        // V(0) = 0 (势垒顶)
        assert!(cfg.potential(0.0).abs() < 1e-6);
    }

    #[test]
    fn test_solver_creation() {
        let s = DuffingSolver::new(DuffingConfig::default());
        assert_eq!(s.steps, 0);
        assert_eq!(s.time, 0.0);
    }

    #[test]
    fn test_initialize() {
        let mut s = DuffingSolver::new(DuffingConfig::default());
        s.initialize(1.5, -0.5);
        assert!((s.x - 1.5).abs() < 1e-6);
        assert!((s.v - (-0.5)).abs() < 1e-6);
    }

    #[test]
    fn test_initialize_at_well() {
        let mut s = DuffingSolver::new(DuffingConfig::default());
        s.initialize_at_well(true);
        assert!((s.x - 1.0).abs() < 1e-5);
        assert!(s.v.abs() < 1e-6);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = DuffingSolver::new(DuffingConfig::default());
        s.initialize(1.0, 0.0);
        let t0 = s.time;
        s.step();
        assert!(s.time > t0);
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = DuffingSolver::new(DuffingConfig::default());
        s.initialize(1.0, 0.0);
        s.step_n(100);
        assert_eq!(s.steps, 100);
    }

    #[test]
    fn test_no_nan_short_run() {
        let mut s = DuffingSolver::new(DuffingConfig::default());
        s.initialize(1.0, 0.0);
        s.step_n(1000);
        assert!(!s.has_nan(), "NaN after 1000 steps");
    }

    #[test]
    fn test_no_nan_long_run() {
        let cfg = DuffingConfig {
            dt: 0.005,
            ..Default::default()
        };
        let mut s = DuffingSolver::new(cfg);
        s.initialize(1.0, 0.5);
        s.step_n(20000);
        assert!(!s.has_nan(), "NaN after 20000 steps");
    }

    #[test]
    fn test_undamped_unforced_energy_conservation() {
        // 无阻尼无驱动 (δ=γ=0): 能量应严格守恒
        let cfg = DuffingConfig {
            delta: 0.0,
            gamma: 0.0,
            dt: 0.001,
            ..Default::default()
        };
        let mut s = DuffingSolver::new(cfg);
        s.initialize(1.0, 0.0); // 在势井底, v=0
        // 给个位置偏移到井外
        s.initialize(0.5, 0.0);
        let e0 = s.energy();
        s.step_n(2000);
        let e1 = s.energy();
        assert!(
            (e1 - e0).abs() < 0.01 * e0.abs().max(0.01),
            "energy not conserved: {} -> {}",
            e0,
            e1
        );
    }

    #[test]
    fn test_damped_decays_to_well() {
        // 有阻尼无驱动, 双井: 应衰减到其中一个势井
        let cfg = DuffingConfig {
            delta: 0.5,
            gamma: 0.0,
            dt: 0.005,
            ..Default::default()
        };
        let mut s = DuffingSolver::new(cfg);
        s.initialize(1.5, 0.0); // 在右井外
        s.step_n(5000);
        // 应收敛到 x=1 (右井底), v=0
        assert!(
            (s.x - 1.0).abs() < 0.1,
            "should converge to right well: x={}",
            s.x
        );
        assert!(s.v.abs() < 0.1, "velocity should decay: v={}", s.v);
    }

    #[test]
    fn test_damped_origin_falls_to_a_well() {
        // 从原点 (势垒顶) 启动, 微扰决定掉入哪边
        let cfg = DuffingConfig {
            delta: 0.3,
            gamma: 0.0,
            dt: 0.005,
            ..Default::default()
        };
        let mut s1 = DuffingSolver::new(cfg.clone());
        s1.initialize(0.0, 0.1); // 微扰向右
        s1.step_n(5000);
        let mut s2 = DuffingSolver::new(cfg);
        s2.initialize(0.0, -0.1); // 微扰向左
        s2.step_n(5000);
        // 一个在左井, 一个在右井
        assert!(
            (s1.x - s2.x).abs() > 1.0,
            "trajectories should split: {} vs {}",
            s1.x,
            s2.x
        );
    }

    #[test]
    fn test_driven_bounded() {
        // 强迫混沌: 振幅应有界 (奇怪吸引子)
        let cfg = DuffingConfig {
            dt: 0.005,
            ..Default::default()
        };
        let mut s = DuffingSolver::new(cfg);
        s.initialize(1.0, 0.0);
        s.step_n(20000);
        assert!(!s.has_nan(), "NaN in driven case");
        assert!(
            s.x.abs() < 10.0,
            "x should be bounded: {}",
            s.x
        );
        assert!(
            s.v.abs() < 10.0,
            "v should be bounded: {}",
            s.v
        );
    }

    #[test]
    fn test_lyapunov_divergence_in_chaos() {
        // 强混沌参数 (Ueda): 两条相近轨迹应指数发散
        // 需要足够长时间让轨迹展开到吸引子上
        let cfg = DuffingConfig {
            delta: 0.05,
            alpha: 0.0,
            beta: 1.0,
            gamma: 7.5,
            omega: 1.0,
            dt: 0.01,
        };
        let mut s1 = DuffingSolver::new(cfg.clone());
        let mut s2 = DuffingSolver::new(cfg);
        s1.initialize(1.0, 0.0);
        s2.initialize(1.001, 0.001); // 较大初始差异
        // 先各自演化使落到吸引子上
        s1.step_n(2000);
        s2.step_n(2000);
        // 此时同步化初值
        s2.initialize(s1.x + 1e-4, s1.v + 1e-4);
        let d0 = s1.distance_to(&s2);
        // 继续演化, 在吸引子上混沌发散
        s1.step_n(20000);
        s2.step_n(20000);
        let d1 = s1.distance_to(&s2);
        assert!(d1 > d0, "trajectories should diverge: {} -> {}", d0, d1);
        assert!(d1 > 0.01, "substantial divergence: {}", d1);
    }

    #[test]
    fn test_distance_to_identical() {
        let s1 = DuffingSolver::new(DuffingConfig::default());
        let s2 = DuffingSolver::new(DuffingConfig::default());
        assert!(s1.distance_to(&s2) < 1e-6);
    }

    #[test]
    fn test_kinetic_energy() {
        let mut s = DuffingSolver::new(DuffingConfig::default());
        s.initialize(0.0, 2.0);
        assert!((s.kinetic_energy() - 2.0).abs() < 1e-5);
    }

    #[test]
    fn test_potential_energy_at_well() {
        let cfg = DuffingConfig::default();
        let mut s = DuffingSolver::new(cfg);
        s.initialize_at_well(true);
        // V(1) = -0.25
        assert!((s.potential_energy() - (-0.25)).abs() < 1e-5);
    }

    #[test]
    fn test_is_dissipative() {
        assert!(DuffingConfig::default().is_dissipative());
        assert!(
            !DuffingConfig {
                delta: 0.0,
                ..Default::default()
            }
            .is_dissipative()
        );
    }

    #[test]
    fn test_is_driven() {
        assert!(DuffingConfig::default().is_driven());
        assert!(
            !DuffingConfig {
                gamma: 0.0,
                ..Default::default()
            }
            .is_driven()
        );
    }

    #[test]
    fn test_periodic_drive_phase() {
        // 驱动应随时间周期变化
        let cfg = DuffingConfig {
            delta: 0.0,
            alpha: 0.0,
            beta: 0.0, // 完全自由粒子受迫
            gamma: 1.0,
            omega: 1.0,
            dt: 0.001,
        };
        let mut s = DuffingSolver::new(cfg.clone());
        s.initialize(0.0, 0.0);
        s.step_n((2.0 * std::f32::consts::PI / cfg.dt) as usize);
        // 一个驱动周期后, 状态应有显著变化 (因为驱动累积)
        assert!(!s.has_nan());
        assert!(s.x.abs() > 0.0);
    }

    #[test]
    fn test_ueda_chaotic_parameters() {
        // Ueda 经典混沌参数: α=0, β=1, γ=7.5, ω=1, δ=0.05
        let cfg = DuffingConfig {
            delta: 0.05,
            alpha: 0.0,
            beta: 1.0,
            gamma: 7.5,
            omega: 1.0,
            dt: 0.005,
        };
        let mut s = DuffingSolver::new(cfg);
        s.initialize(1.0, 0.0);
        s.step_n(10000);
        assert!(!s.has_nan(), "Ueda NaN");
        // Ueda 吸引子 x 通常在 [-3, 3] 范围
        assert!(s.x.abs() < 10.0, "Ueda bounded: x={}", s.x);
    }

    #[test]
    fn test_energy_positive_at_large_amplitude() {
        let cfg = DuffingConfig::default();
        // 在 x=2, v=0: E = 0 + 0.5*(-1)*4 + 0.25*1*16 = -2 + 4 = 2
        let e = cfg.energy(2.0, 0.0);
        assert!((e - 2.0).abs() < 1e-5);
    }
}
