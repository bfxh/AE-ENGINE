//! Mackey-Glass Equation — 生理混沌延迟微分方程 (DDE)
//!
//! Mackey-Glass 方程是延迟微分方程 (Delay Differential Equation, DDE) 的
//! 经典例子, 由 Mackey & Glass (1977) 提出用于描述血液白细胞的动态调节.
//! 它是最早被广泛研究的产生混沌的 DDE 之一, 在生理学、神经科学和控制论中
//! 有重要地位.
//!
//! 状态方程 (延迟微分方程, Mackey-Glass 1977 原始形式):
//!   dx/dt = β · x(t-τ) / (1 + x(t-τ)^n) - γ · x(t)
//!
//! 注意: 分子是 x(t-τ) (线性), 分母是 1 + x(t-τ)^n (Hill 抑制).
//! 这是负反馈形式: x 增大 → 分母增大 → 生产率下降.
//!
//! 经典参数:
//!   β = 0.2, γ = 0.1, n = 10, τ = 17
//!   (Mackey & Glass 1977 原始参数, τ > 16.8 时混沌)
//!
//! 动力学行为 (随 τ 变化):
//!   - τ < 4.53:  稳定不动点 x* = (β/γ - 1)^(1/n) = 1 (经典参数)
//!   - 4.53 < τ < 13.3:  极限环 (周期解)
//!   - 13.3 < τ < 16.8:  倍周期分岔
//!   - τ > 16.8:  混沌 (经典 τ=17)
//!
//! 性质:
//!   - DDE: 当前导数依赖历史状态, 相空间无穷维 (由历史函数决定)
//!   - Hill 抑制反馈: x/(1+x^n) 是钟形曲线 (先升后降, 峰值在 (1/(n-1))^(1/n))
//!   - 平衡点: 令 dx/dt=0, x_τ=x=x*: β/(1+x*^n) = γ → x* = (β/γ-1)^(1/n)
//!     经典参数 (β/γ=2): x* = 1^(1/n) = 1
//!   - 线性化稳定性: 平衡点稳定当 τ < τ_crit
//!   - Lyapunov 指数: 混沌时 λ₁ > 0 (文献 ~0.007)
//!   - 吸引子分形维数 D ≈ 6-7 (高维混沌, 因 DDE 无穷维)
//!   - ∂f/∂x_τ = β·(1-(n-1)x^n)/(1+x^n)^2, 在 x* 处为负 (负反馈)
//!
//! 物理意义:
//!   - 生理调节: 白细胞生产延迟反馈 → 周期性粒细胞波动
//!   - 神经科学: 神经元集群延迟耦合
//!   - 控制论: 延迟反馈导致不稳定和混沌
//!   - 经济学: 供应链牛鞭效应 (延迟响应)
//!
//! 数值方法:
//!   DDE 需要: (1) 初始历史函数 x(t) for t ∈ [-τ, 0];
//!             (2) 每步用历史值计算导数.
//!   主轨道用 RK4 + 线性插值获取中间延迟值 x(t-τ+dt/2).
//!   Lyapunov 指数用 Benettin 双轨道法 (Wolf 1985):
//!     - 同时演化主轨道 x 和扰动轨道 x_pert (相差 ε)
//!     - 周期性重置扰动轨道到主轨道附近, 累积 ln(d/d0)
//!     - λ = sum(ln(d/d0)) / T
//!   历史用常数 x0 初始化 (Mackey-Glass 标准做法).
//!
//! 历史:
//!   Mackey, M. C. & Glass, L. 1977. "Oscillation and chaos in physiological
//!   control systems." Science 197, 287. (首创方程与混沌分析)
//!   Farmer, J. D. 1982. "Chaotic attractors of an infinite-dimensional
//!   dynamical system." Physica D 4, 366. (DDE 分形维数分析)
//!   Wolf, A. et al. 1985. "Determining Lyapunov exponents from a time
//!   series." Physica D 16, 285. (双轨道 LE 算法)

/// Mackey-Glass 配置
#[derive(Clone, Copy, Debug)]
pub struct MackeyGlassConfig {
    /// 生产率 β
    pub beta: f64,
    /// 损耗率 γ
    pub gamma: f64,
    /// Hill 函数非线性指数 n
    pub n: f64,
    /// 延迟时间 τ
    pub tau: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for MackeyGlassConfig {
    fn default() -> Self {
        Self { beta: 0.2, gamma: 0.1, n: 10.0, tau: 17.0, dt: 0.1 }
    }
}

/// Mackey-Glass DDE 求解器 (含 Benettin 双轨道 Lyapunov)
pub struct MackeyGlassSolver {
    pub config: MackeyGlassConfig,
    /// 主轨道历史: history[i] = x(t - (len-1-i)*dt), history[len-1] = x(t)
    pub history: Vec<f64>,
    /// 扰动轨道历史 (用于 Lyapunov)
    pub pert_history: Vec<f64>,
    /// 延迟步数 (τ/dt, 浮点用于插值)
    pub delay_steps: f64,
    pub time: f64,
    pub step_count: u64,
    pub trajectory: Vec<f64>,
    /// Lyapunov 累积 (双轨道距离对数和)
    pub lyap_sum: f64,
    /// 初始扰动大小
    pub epsilon: f64,
    /// 重整化间隔步数
    pub rescale_interval: usize,
    /// 上次重整化时的步数
    pub last_rescale_step: u64,
}

impl MackeyGlassSolver {
    pub fn new(config: MackeyGlassConfig, x0: f64) -> Self {
        let delay_steps = (config.tau / config.dt).round();
        let delay_int = delay_steps as usize + 1;
        let history = vec![x0; delay_int + 1];
        let epsilon = 1e-8;
        // 扰动轨道初值 = x0 + epsilon
        let pert_history = vec![x0 + epsilon; delay_int + 1];
        Self {
            config,
            history,
            pert_history,
            delay_steps,
            time: 0.0,
            step_count: 0,
            trajectory: vec![x0],
            lyap_sum: 0.0,
            epsilon,
            rescale_interval: 10,
            last_rescale_step: 0,
        }
    }

    pub fn classic(config: MackeyGlassConfig) -> Self {
        // 初值 x0 = 1.2 (偏离平衡点 x*=1.0, 触发动力学)
        // 注: 若 x0 = x* = 1.0 且历史为常数, 系统停留在平衡点 (无动力学)
        Self::new(config, 1.2)
    }

    /// 生产率函数 p(x) = x / (1 + x^n) (Hill 抑制反馈)
    /// 注意: 分子是 x (线性), 不是 x^n
    pub fn production(cfg: &MackeyGlassConfig, x: f64) -> f64 {
        x / (1.0 + x.powf(cfg.n))
    }

    /// 右端导数 dx/dt = β · p(x_τ) - γ · x
    pub fn derivatives(cfg: &MackeyGlassConfig, x: f64, x_tau: f64) -> f64 {
        cfg.beta * Self::production(cfg, x_tau) - cfg.gamma * x
    }

    /// 平衡点 x* = (β/γ - 1)^(1/n) (令 dx/dt=0 且 x=x_τ=x*)
    pub fn equilibrium(cfg: &MackeyGlassConfig) -> Option<f64> {
        if cfg.beta / cfg.gamma <= 1.0 {
            return None;
        }
        Some((cfg.beta / cfg.gamma - 1.0).powf(1.0 / cfg.n))
    }

    /// 对延迟状态的偏导 ∂f/∂x_τ = β · p'(x)
    /// p(x) = x/(1+x^n), p'(x) = (1 - (n-1)x^n)/(1+x^n)^2
    pub fn df_dx_tau(cfg: &MackeyGlassConfig, x: f64) -> f64 {
        let n = cfg.n;
        let xn = x.powf(n);
        let p_prime = (1.0 - (n - 1.0) * xn) / (1.0 + xn).powi(2);
        cfg.beta * p_prime
    }

    /// 对当前状态的偏导 ∂f/∂x = -γ
    pub fn df_dx(cfg: &MackeyGlassConfig) -> f64 {
        -cfg.gamma
    }

    /// 从历史缓冲区获取 x(t - delay), delay 以步数计 (浮点, 线性插值)
    fn delayed(history: &[f64], delay_steps: f64) -> f64 {
        let len = history.len();
        let idx = (len as f64) - 1.0 - delay_steps;
        if idx <= 0.0 {
            return history[0];
        }
        if idx >= (len - 1) as f64 {
            return history[len - 1];
        }
        let lo = idx.floor() as usize;
        let hi = lo + 1;
        let frac = idx - lo as f64;
        history[lo] * (1.0 - frac) + history[hi] * frac
    }

    /// RK4 单步演化一条轨道 (用给定历史)
    fn rk4_step_one(cfg: &MackeyGlassConfig, history: &mut Vec<f64>, delay_steps: f64) -> f64 {
        let dt = cfg.dt;
        let x = *history.last().unwrap();
        let ds = delay_steps;

        let x_tau = Self::delayed(history, ds);
        let x_tau_half = Self::delayed(history, ds - 0.5);
        let x_tau_dt = Self::delayed(history, ds - 1.0);

        let k1 = Self::derivatives(cfg, x, x_tau);
        let k2 = Self::derivatives(cfg, x + 0.5 * dt * k1, x_tau_half);
        let k3 = Self::derivatives(cfg, x + 0.5 * dt * k2, x_tau_half);
        let k4 = Self::derivatives(cfg, x + dt * k3, x_tau_dt);

        let x_new = x + dt / 6.0 * (k1 + 2.0 * k2 + 2.0 * k3 + k4);

        history.push(x_new);
        history.remove(0);
        x_new
    }

    /// 单步推进 (主轨道 + 扰动轨道 + Lyapunov 重整化)
    pub fn step(&mut self) {
        let cfg = self.config;
        let ds = self.delay_steps;

        // 演化主轨道
        let x_new = Self::rk4_step_one(&cfg, &mut self.history, ds);

        // 演化扰动轨道
        let _x_pert_new = Self::rk4_step_one(&cfg, &mut self.pert_history, ds);

        self.trajectory.push(x_new);
        self.time += cfg.dt;
        self.step_count += 1;

        // 周期性重整化 (Benettin 算法)
        if self.step_count - self.last_rescale_step >= self.rescale_interval as u64 {
            let x_main = *self.history.last().unwrap();
            let x_pert = *self.pert_history.last().unwrap();
            let d = (x_pert - x_main).abs();
            if d > 0.0 {
                self.lyap_sum += d.ln() - self.epsilon.ln();
                // 重置扰动轨道: 保持方向, 距离重置为 epsilon
                let scale = self.epsilon / d;
                for i in 0..self.pert_history.len() {
                    self.pert_history[i] = self.history[i] + (self.pert_history[i] - self.history[i]) * scale;
                }
            }
            self.last_rescale_step = self.step_count;
        }
    }

    pub fn run(&mut self, n_steps: usize) {
        for _ in 0..n_steps {
            self.step();
        }
    }

    pub fn lyapunov_exponent(&self) -> f64 {
        if self.step_count == 0 {
            return 0.0;
        }
        self.lyap_sum / self.time.max(1e-12)
    }

    pub fn current_x(&self) -> f64 {
        *self.history.last().unwrap()
    }

    pub fn has_nan(&self) -> bool {
        !self.current_x().is_finite()
    }

    pub fn attractor_bounds(&self) -> (f64, f64) {
        let mut xmin = f64::INFINITY;
        let mut xmax = f64::NEG_INFINITY;
        for &x in &self.trajectory {
            if x < xmin { xmin = x; }
            if x > xmax { xmax = x; }
        }
        (xmin, xmax)
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
        let cfg = MackeyGlassConfig::default();
        assert!(approx_eq(cfg.beta, 0.2, 1e-12));
        assert!(approx_eq(cfg.gamma, 0.1, 1e-12));
        assert!(approx_eq(cfg.n, 10.0, 1e-12));
        assert!(approx_eq(cfg.tau, 17.0, 1e-12));
        assert!(approx_eq(cfg.dt, 0.1, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = MackeyGlassSolver::classic(MackeyGlassConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert!(approx_eq(s.current_x(), 1.2, 1e-12));
        assert!(s.history.len() >= 170);
        assert_eq!(s.history.len(), s.pert_history.len());
    }

    #[test]
    fn test_production_function() {
        // p(x) = x / (1 + x^n)
        // p(0) = 0, p(1) = 1/2, p(∞) → 0 (因 x^n 主导)
        let cfg = MackeyGlassConfig::default();
        assert!(approx_eq(MackeyGlassSolver::production(&cfg, 0.0), 0.0, 1e-12));
        assert!(approx_eq(MackeyGlassSolver::production(&cfg, 1.0), 0.5, 1e-12));
        assert!(MackeyGlassSolver::production(&cfg, 100.0) < 1e-7);
    }

    #[test]
    fn test_production_bell_shape() {
        // p(x) = x/(1+x^n) 钟形, 峰值在 x = (1/(n-1))^(1/n) ≈ 0.7743
        let cfg = MackeyGlassConfig::default();
        let x_peak = (1.0 / 9.0_f64).powf(0.1);
        let p_peak = MackeyGlassSolver::production(&cfg, x_peak);
        let p_low = MackeyGlassSolver::production(&cfg, 0.3);
        let p_high = MackeyGlassSolver::production(&cfg, 1.5);
        assert!(p_peak > p_low, "peak {} should > p(0.3)={}", p_peak, p_low);
        assert!(p_peak > p_high, "peak {} should > p(1.5)={}", p_peak, p_high);
    }

    #[test]
    fn test_derivatives_analytic() {
        // dx/dt = β · x_τ/(1+x_τ^n) - γ·x
        // 在 x=x_τ=1: β·0.5 - γ·1 = 0.2·0.5 - 0.1 = 0
        let cfg = MackeyGlassConfig::default();
        let d = MackeyGlassSolver::derivatives(&cfg, 1.0, 1.0);
        assert!(approx_eq(d, 0.0, 1e-12));
    }

    #[test]
    fn test_derivatives_at_equilibrium() {
        let cfg = MackeyGlassConfig::default();
        let x_star = MackeyGlassSolver::equilibrium(&cfg).unwrap();
        let d = MackeyGlassSolver::derivatives(&cfg, x_star, x_star);
        assert!(approx_eq(d, 0.0, 1e-9), "d={} at x*={}", d, x_star);
    }

    #[test]
    fn test_equilibrium_value() {
        // x* = (β/γ - 1)^(1/n) = (2-1)^(1/10) = 1
        let cfg = MackeyGlassConfig::default();
        let x_star = MackeyGlassSolver::equilibrium(&cfg).unwrap();
        assert!(approx_eq(x_star, 1.0, 1e-12), "x* = {}", x_star);
    }

    #[test]
    fn test_equilibrium_none_when_beta_le_gamma() {
        let cfg = MackeyGlassConfig { beta: 0.1, gamma: 0.2, ..MackeyGlassConfig::default() };
        assert!(MackeyGlassSolver::equilibrium(&cfg).is_none());
    }

    #[test]
    fn test_step_advances() {
        let mut s = MackeyGlassSolver::classic(MackeyGlassConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = MackeyGlassSolver::classic(MackeyGlassConfig::default());
        s.run(5000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = MackeyGlassSolver::classic(MackeyGlassConfig::default());
        s.run(20000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = MackeyGlassSolver::classic(MackeyGlassConfig::default());
        s.run(10000);
        let (xmin, xmax) = s.attractor_bounds();
        assert!(xmin > -1.0 && xmax < 5.0, "x: [{}, {}]", xmin, xmax);
    }

    #[test]
    fn test_attractor_positive() {
        let mut s = MackeyGlassSolver::classic(MackeyGlassConfig::default());
        s.run(10000);
        let (xmin, _) = s.attractor_bounds();
        assert!(xmin > -0.5, "xmin should be near positive: {}", xmin);
    }

    #[test]
    fn test_lyapunov_positive_for_tau_chaotic() {
        // τ=17 (混沌), λ₁ > 0 (文献值 ~0.007)
        // 双轨道法需要较长瞬态收敛
        let mut s = MackeyGlassSolver::classic(MackeyGlassConfig::default());
        s.run(30000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = MackeyGlassSolver::classic(MackeyGlassConfig::default());
        s.run(10000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda.abs() < 1.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        // 两条相近轨道应分离 (混沌)
        let cfg = MackeyGlassConfig::default();
        let d0 = 1e-8_f64;
        let mut s1 = MackeyGlassSolver::new(cfg, 1.2);
        let mut s2 = MackeyGlassSolver::new(cfg, 1.2 + d0);
        for _ in 0..20000 {
            s1.step();
            s2.step();
        }
        let d = (s1.current_x() - s2.current_x()).abs();
        // t=2000, λ~0.007, 应放大 e^14 ≈ 1.2e6 倍
        assert!(d > 1e-6, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_stable_fixed_point_small_tau() {
        // τ < 4.53 → 稳定不动点. 用 τ=2, 收敛到 x* = 1
        let cfg = MackeyGlassConfig { tau: 2.0, ..MackeyGlassConfig::default() };
        let mut s = MackeyGlassSolver::new(cfg, 1.5);
        s.run(5000);
        let x_star = MackeyGlassSolver::equilibrium(&cfg).unwrap();
        let x_final = s.current_x();
        assert!((x_final - x_star).abs() < 0.05, "should converge to x*={}: got {}", x_star, x_final);
    }

    #[test]
    fn test_stable_no_oscillation_large_gamma() {
        // γ >> β → 强损耗, 解快速衰减到 0
        let cfg = MackeyGlassConfig { beta: 0.01, gamma: 1.0, ..MackeyGlassConfig::default() };
        let mut s = MackeyGlassSolver::new(cfg, 1.0);
        s.run(1000);
        let x_final = s.current_x();
        assert!(x_final.abs() < 0.1, "should decay: x_final={}", x_final);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = MackeyGlassSolver::classic(MackeyGlassConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_df_dx_constant() {
        // ∂f/∂x = -γ (常数)
        let cfg = MackeyGlassConfig::default();
        assert!(approx_eq(MackeyGlassSolver::df_dx(&cfg), -cfg.gamma, 1e-12));
    }

    #[test]
    fn test_df_dx_tau_at_equilibrium() {
        // 在平衡点 x*=1, n=10:
        // p'(1) = (1-9·1)/4 = -2, ∂f/∂x_τ = 0.2·(-2) = -0.4
        let cfg = MackeyGlassConfig::default();
        let dfxt = MackeyGlassSolver::df_dx_tau(&cfg, 1.0);
        assert!(approx_eq(dfxt, -0.4, 1e-9), "df/dx_tau at x*=1: {}", dfxt);
    }

    #[test]
    fn test_df_dx_tau_negative_above_peak() {
        // ∂f/∂x_τ < 0 当 x > (1/(n-1))^(1/n) ≈ 0.7743 (峰值右侧)
        let cfg = MackeyGlassConfig::default();
        for &x in &[0.9, 1.0, 1.2, 1.5, 2.0] {
            let dfxt = MackeyGlassSolver::df_dx_tau(&cfg, x);
            assert!(dfxt < 0.0, "should be negative at x={}: {}", x, dfxt);
        }
    }

    #[test]
    fn test_df_dx_tau_positive_below_peak() {
        // ∂f/∂x_τ > 0 当 x < (1/(n-1))^(1/n) ≈ 0.7743 (峰值左侧)
        let cfg = MackeyGlassConfig::default();
        for &x in &[0.1, 0.3, 0.5, 0.7] {
            let dfxt = MackeyGlassSolver::df_dx_tau(&cfg, x);
            assert!(dfxt > 0.0, "should be positive at x={}: {}", x, dfxt);
        }
    }

    #[test]
    fn test_history_length_constant() {
        let mut s = MackeyGlassSolver::classic(MackeyGlassConfig::default());
        let initial_len = s.history.len();
        s.run(1000);
        assert_eq!(s.history.len(), initial_len, "history length should be constant");
        assert_eq!(s.pert_history.len(), initial_len, "pert history length should be constant");
    }

    #[test]
    fn test_periodicity_for_large_tau() {
        // τ=17 混沌, 解应非平凡 (方差 > 0)
        let mut s = MackeyGlassSolver::classic(MackeyGlassConfig::default());
        s.run(20000);
        let n = s.trajectory.len() as f64;
        let mean = s.trajectory.iter().sum::<f64>() / n;
        let var: f64 = s.trajectory.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
        assert!(var > 0.001, "variance should be non-trivial: {}", var);
        assert!(mean > 0.5 && mean < 2.0, "mean should be near x*: {}", mean);
    }

    #[test]
    fn test_delayed_value_interpolation() {
        // 验证延迟值插值正确 (通过 rk4_step_one 间接测试)
        // 用稳定参数, 验证收敛到已知平衡点
        let cfg = MackeyGlassConfig { tau: 1.0, dt: 0.1, ..MackeyGlassConfig::default() };
        let mut s = MackeyGlassSolver::new(cfg, 1.3);
        s.run(3000);
        let x_star = MackeyGlassSolver::equilibrium(&cfg).unwrap();
        assert!((s.current_x() - x_star).abs() < 0.05, "should converge to x*: {}", s.current_x());
    }

    #[test]
    fn test_n_affects_dynamics() {
        // n 较小 → 非线性弱; n 较大 → 非线性强
        let cfg_low_n = MackeyGlassConfig { n: 2.0, ..MackeyGlassConfig::default() };
        let cfg_high_n = MackeyGlassConfig { n: 20.0, ..MackeyGlassConfig::default() };
        let mut s_low = MackeyGlassSolver::classic(cfg_low_n);
        let mut s_high = MackeyGlassSolver::classic(cfg_high_n);
        s_low.run(5000);
        s_high.run(5000);
        let (_, xmax_low) = s_low.attractor_bounds();
        let (_, xmax_high) = s_high.attractor_bounds();
        assert!(xmax_low.is_finite() && xmax_high.is_finite());
        assert!(xmax_low > 0.0 && xmax_high > 0.0);
    }

    #[test]
    fn test_stays_at_equilibrium_with_constant_history() {
        // 若 x0 = x* 且历史为常数, 系统应停留在平衡点
        let cfg = MackeyGlassConfig::default();
        let x_star = MackeyGlassSolver::equilibrium(&cfg).unwrap();
        let mut s = MackeyGlassSolver::new(cfg, x_star);
        s.run(2000);
        assert!(approx_eq(s.current_x(), x_star, 1e-6), "should stay at x*: {}", s.current_x());
    }
}
