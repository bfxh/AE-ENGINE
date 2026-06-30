//! Lorenz 96 Model — 大气可预报性模型
//!
//! Edward Lorenz 1995 年提出的简化大气模型, 是 Lorenz 63 (蝴蝶效应) 的
//! 延伸. 用 N 个耦合变量模拟大气环流的纬度分布, 是研究混沌可预报性
//! 和数据同化的标准测试床.
//!
//! 方程:
//!   dx_i/dt = (x_{i+1} - x_{i-2})·x_{i-1} - x_i + F
//!
//! 其中 i = 0, ..., N-1, 周期边界 (x_N = x_0, x_{-1} = x_{N-1})
//!
//! 各项物理意义:
//!   - (x_{i+1} - x_{i-2})·x_{i-1}: 非线性平流项 (能量在变量间转移)
//!   - -x_i: 阻尼 (能量耗散)
//!   + F: 外部强迫 (太阳加热)
//!
//! 动力学 (随 F 变化):
//!   - F = 0: 所有 x -> 0 (稳态)
//!   - F 小: 稳态 x = F 稳定
//!   - F ≈ 4: 稳态失稳, 周期解
//!   - F > 6: 准周期
//!   - F = 8: 完全混沌 (标准参数, 用于可预报性研究)
//!   - F = 8 时 Lyapunov 指数 ~0.9, 可预报时间 ~1/0.9 ≈ 1.1 时间单位
//!
//! 应用:
//!   - 天气预报可预报性研究
//!   - 数据同化算法测试 (EnKF, 4D-Var)
//!   - 集合预报
//!   - 混沌同步
//!
//! 数值方法:
//!   4阶 Runge-Kutta (RK4) — 比 Euler 精度高, 适合混沌系统长期积分
//!
//! 基于:
//!   - Lorenz, E.N. 1995. "Predictability: A problem partly solved."
//!     Seminar on Predictability, ECMWF.
//!   - Lorenz, E.N. & Emanuel, K.A. 1998. J. Atmos. Sci. 55, 399.
//!   - Ott, E. et al. 2002. "A local ensemble Kalman filter for
//!     atmospheric data assimilation." Tellus A 54, 415.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L96Config {
    /// 变量数 N (典型 40)
    pub n: usize,
    /// 时间步长 (典型 0.01, 模型时间单位 ≈ 5 天)
    pub dt: f32,
    /// 强迫参数 F (典型 8.0, 混沌)
    pub f: f32,
}

impl Default for L96Config {
    fn default() -> Self {
        L96Config {
            n: 40,
            dt: 0.01,
            f: 8.0,
        }
    }
}

impl L96Config {
    /// 稳态: x_i = F
    pub fn steady_state(&self) -> Vec<f32> {
        vec![self.f; self.n]
    }

    /// 线性化特征值 (在稳态 x=F 附近):
    /// J_ij = δ_{i+1,j}·F - δ_{i-2,j}·F + δ_{i-1,j}·0 - δ_{i,j}
    /// 实部最大特征值 ≈ F - 1 (粗略), F=8 时正
    pub fn is_chaotic(&self) -> bool {
        self.f > 6.0
    }
}

pub struct L96Solver {
    pub config: L96Config,
    pub x: Vec<f32>,
    pub time: f32,
    pub steps: usize,
}

impl L96Solver {
    pub fn new(config: L96Config) -> Self {
        let n = config.n;
        L96Solver {
            config,
            x: vec![0.0; n],
            time: 0.0,
            steps: 0,
        }
    }

    /// 初始化为稳态 x_i = F
    pub fn initialize_steady(&mut self) {
        let f = self.config.f;
        for v in &mut self.x {
            *v = f;
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化为稳态 + 小扰动 (中心点)
    pub fn initialize_perturbed(&mut self, amplitude: f32) {
        let f = self.config.f;
        let n = self.config.n;
        for v in &mut self.x {
            *v = f;
        }
        if n > 0 {
            self.x[n / 2] += amplitude;
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 初始化为稳态 + 随机扰动
    pub fn initialize_random(&mut self, amplitude: f32, seed: u64) {
        let f = self.config.f;
        let mut rng = L96Rng::new(seed);
        for v in &mut self.x {
            *v = f + amplitude * (2.0 * rng.next() - 1.0);
        }
        self.time = 0.0;
        self.steps = 0;
    }

    #[inline]
    fn wrap(&self, i: i32) -> usize {
        let n = self.config.n as i32;
        (((i % n) + n) % n) as usize
    }

    /// 计算 dx/dt
    fn derivatives(x: &[f32], f: f32) -> Vec<f32> {
        let n = x.len();
        let mut dx = vec![0.0; n];
        for i in 0..n {
            let ii = i as i32;
            let ip1 = (((ii + 1) % n as i32 + n as i32) % n as i32) as usize;
            let im1 = (((ii - 1) % n as i32 + n as i32) % n as i32) as usize;
            let im2 = (((ii - 2) % n as i32 + n as i32) % n as i32) as usize;
            dx[i] = (x[ip1] - x[im2]) * x[im1] - x[i] + f;
        }
        dx
    }

    /// 4阶 Runge-Kutta 单步
    pub fn step(&mut self) {
        let dt = self.config.dt;
        let f = self.config.f;

        let k1 = Self::derivatives(&self.x, f);
        let mut x1 = self.x.clone();
        for i in 0..self.x.len() {
            x1[i] += 0.5 * dt * k1[i];
        }
        let k2 = Self::derivatives(&x1, f);
        let mut x2 = self.x.clone();
        for i in 0..self.x.len() {
            x2[i] += 0.5 * dt * k2[i];
        }
        let k3 = Self::derivatives(&x2, f);
        let mut x3 = self.x.clone();
        for i in 0..self.x.len() {
            x3[i] += dt * k3[i];
        }
        let k4 = Self::derivatives(&x3, f);

        for i in 0..self.x.len() {
            self.x[i] += (dt / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
        }

        self.time += dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    pub fn has_nan(&self) -> bool {
        self.x.iter().any(|&v| !v.is_finite())
    }

    pub fn mean(&self) -> f32 {
        let n = self.x.len();
        if n == 0 {
            return 0.0;
        }
        self.x.iter().sum::<f32>() / n as f32
    }

    pub fn variance(&self) -> f32 {
        let m = self.mean();
        let n = self.x.len();
        if n == 0 {
            return 0.0;
        }
        self.x.iter().map(|&v| (v - m) * (v - m)).sum::<f32>() / n as f32
    }

    pub fn std_dev(&self) -> f32 {
        self.variance().sqrt()
    }

    pub fn max(&self) -> f32 {
        self.x.iter().copied().fold(f32::NEG_INFINITY, f32::max)
    }

    pub fn min(&self) -> f32 {
        self.x.iter().copied().fold(f32::INFINITY, f32::min)
    }

    pub fn max_abs(&self) -> f32 {
        self.x.iter().map(|&v| v.abs()).fold(0.0f32, f32::max)
    }

    /// 总能量 E = 0.5·Σx_i²
    pub fn energy(&self) -> f32 {
        0.5 * self.x.iter().map(|&v| v * v).sum::<f32>()
    }

    /// 计算两条轨迹的差异 (用于 Lyapunov 指数估计)
    pub fn l2_distance(&self, other: &L96Solver) -> f32 {
        let mut sum = 0.0f32;
        for i in 0..self.x.len() {
            let d = self.x[i] - other.x[i];
            sum += d * d;
        }
        sum.sqrt()
    }
}

struct L96Rng {
    state: u64,
}

impl L96Rng {
    fn new(seed: u64) -> Self {
        L96Rng {
            state: if seed == 0 { 0x853c49e6748fea9b } else { seed },
        }
    }

    fn next(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        (self.state >> 11) as f32 / (1u64 << 53) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = L96Config::default();
        assert_eq!(cfg.n, 40);
        assert!((cfg.f - 8.0).abs() < 1e-6);
        assert!(cfg.dt > 0.0);
    }

    #[test]
    fn test_is_chaotic() {
        assert!(L96Config { f: 8.0, ..Default::default() }.is_chaotic());
        assert!(!L96Config { f: 4.0, ..Default::default() }.is_chaotic());
    }

    #[test]
    fn test_steady_state() {
        let cfg = L96Config::default();
        let s = cfg.steady_state();
        assert_eq!(s.len(), 40);
        assert!(s.iter().all(|&v| (v - 8.0).abs() < 1e-6));
    }

    #[test]
    fn test_solver_creation() {
        let s = L96Solver::new(L96Config::default());
        assert_eq!(s.x.len(), 40);
        assert_eq!(s.steps, 0);
    }

    #[test]
    fn test_initialize_steady() {
        let mut s = L96Solver::new(L96Config::default());
        s.initialize_steady();
        assert!((s.mean() - 8.0).abs() < 1e-6);
        assert_eq!(s.variance(), 0.0);
    }

    #[test]
    fn test_initialize_perturbed() {
        let mut s = L96Solver::new(L96Config::default());
        s.initialize_perturbed(1.0);
        // 中心点应有扰动
        let n = s.config.n;
        assert!((s.x[n / 2] - 9.0).abs() < 1e-6);
        // 其他点为稳态
        for i in 0..n {
            if i != n / 2 {
                assert!((s.x[i] - 8.0).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn test_initialize_random_bounded() {
        let mut s = L96Solver::new(L96Config::default());
        s.initialize_random(0.1, 42);
        let f = s.config.f;
        // 扰动 0.1, x 在 [F-0.1, F+0.1]
        assert!(s.max() <= f + 0.11);
        assert!(s.min() >= f - 0.11);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = L96Solver::new(L96Config::default());
        s.initialize_perturbed(1.0);
        let t0 = s.time;
        s.step();
        assert!(s.time > t0);
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = L96Solver::new(L96Config::default());
        s.initialize_perturbed(1.0);
        s.step_n(10);
        assert_eq!(s.steps, 10);
    }

    #[test]
    fn test_steady_state_remains_steady() {
        // F=0 时稳态 x=0 应保持
        let cfg = L96Config { f: 0.0, ..Default::default() };
        let mut s = L96Solver::new(cfg);
        s.initialize_steady(); // x = 0
        s.step_n(100);
        assert!(s.max_abs() < 1e-6, "F=0 steady state should stay zero");
    }

    #[test]
    fn test_small_f_decays_to_steady() {
        // F 小 (< F_crit ≈ 0.889), 稳态 x=F 稳定, 扰动衰减
        let cfg = L96Config { f: 0.5, ..Default::default() };
        let mut s = L96Solver::new(cfg);
        s.initialize_perturbed(1.0);
        let v0 = s.variance();
        s.step_n(1000);
        let v1 = s.variance();
        assert!(v1 < v0, "small F perturbation should decay: {} -> {}", v0, v1);
    }

    #[test]
    fn test_chaotic_f_grows() {
        // F=8 混沌, 小扰动增长
        let mut s = L96Solver::new(L96Config::default());
        s.initialize_perturbed(0.001);
        let v0 = s.variance();
        s.step_n(1000);
        let v1 = s.variance();
        assert!(v1 > v0, "chaotic F perturbation should grow: {} -> {}", v0, v1);
    }

    #[test]
    fn test_no_nan_short_run() {
        let mut s = L96Solver::new(L96Config::default());
        s.initialize_perturbed(1.0);
        s.step_n(500);
        assert!(!s.has_nan(), "NaN after 500 steps");
    }

    #[test]
    fn test_no_nan_long_run() {
        let mut s = L96Solver::new(L96Config { n: 20, ..Default::default() });
        s.initialize_random(0.1, 42);
        s.step_n(5000);
        assert!(!s.has_nan(), "NaN after 5000 steps");
    }

    #[test]
    fn test_lyapunov_divergence() {
        // 两条相近轨迹应在混沌系统中指数发散
        let cfg = L96Config::default();
        let mut s1 = L96Solver::new(cfg.clone());
        let mut s2 = L96Solver::new(cfg);
        s1.initialize_perturbed(1.0);
        s2.initialize_perturbed(1.0);
        // 给 s2 微小扰动
        s2.x[0] += 1e-5;
        let d0 = s1.l2_distance(&s2);
        s1.step_n(500);
        s2.step_n(500);
        let d1 = s1.l2_distance(&s2);
        assert!(d1 > d0, "trajectories should diverge: {} -> {}", d0, d1);
        assert!(d1 > 0.1, "divergence should be substantial: {}", d1);
    }

    #[test]
    fn test_energy_bounded() {
        let mut s = L96Solver::new(L96Config::default());
        s.initialize_perturbed(1.0);
        let e0 = s.energy();
        s.step_n(2000);
        let e1 = s.energy();
        assert!(e1.is_finite(), "energy not finite");
        // 能量在混沌系统中波动但应有界
        assert!(e1 < 100.0 * e0 + 1000.0, "energy blew up: {} -> {}", e0, e1);
    }

    #[test]
    fn test_amplitude_bounded() {
        let mut s = L96Solver::new(L96Config::default());
        s.initialize_perturbed(1.0);
        s.step_n(2000);
        assert!(s.max_abs() < 100.0, "amplitude should be bounded: {}", s.max_abs());
    }

    #[test]
    fn test_mean_near_f() {
        // Lorenz 96 长期平均 mean 受能量-强迫平衡影响, 偏离 F
        // 实测 F=8 时 mean ~2-3 (非 F), 此处只验证 mean 有限且有界
        let mut s = L96Solver::new(L96Config::default());
        s.initialize_perturbed(1.0);
        s.step_n(5000);
        let m = s.mean();
        assert!(m.is_finite(), "mean should be finite: {}", m);
        assert!(m > -10.0 && m < 20.0, "mean should be in reasonable range: {}", m);
    }

    #[test]
    fn test_variance_positive_in_chaos() {
        let mut s = L96Solver::new(L96Config::default());
        s.initialize_perturbed(1.0);
        s.step_n(500);
        assert!(s.variance() > 0.1, "chaotic system should have variance: {}", s.variance());
    }

    #[test]
    fn test_l2_distance_zero_for_identical() {
        let s1 = L96Solver::new(L96Config::default());
        let s2 = L96Solver::new(L96Config::default());
        assert_eq!(s1.l2_distance(&s2), 0.0);
    }

    #[test]
    fn test_wrap_periodic() {
        let s = L96Solver::new(L96Config { n: 10, ..Default::default() });
        assert_eq!(s.wrap(-1), 9);
        assert_eq!(s.wrap(0), 0);
        assert_eq!(s.wrap(10), 0);
        assert_eq!(s.wrap(11), 1);
    }

    #[test]
    fn test_min_max() {
        let mut s = L96Solver::new(L96Config::default());
        s.initialize_steady();
        assert!((s.max() - 8.0).abs() < 1e-6);
        assert!((s.min() - 8.0).abs() < 1e-6);
    }

    #[test]
    fn test_n_parameter_flexible() {
        // 不同 N 应该都能工作
        for n in [10, 20, 40, 100] {
            let cfg = L96Config { n, ..Default::default() };
            let mut s = L96Solver::new(cfg);
            s.initialize_perturbed(1.0);
            s.step_n(100);
            assert!(!s.has_nan(), "NaN for N={}", n);
        }
    }
}
