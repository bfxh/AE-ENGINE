//! Logistic Map — 1D 离散混沌 (周期倍化分岔与 Feigenbaum 常数)
//!
//! x_{n+1} = r · x_n · (1 - x_n),  x ∈ [0,1],  r ∈ [0,4]
//!
//! May 1976 年 Review of Modern Physics 文章将其推为种群动力学与混沌理论
//! 的桥梁. 是最简单的展示周期倍化分岔、Feigenbaum 普适性、混沌突然出现
//! 的系统.
//!
//! 行为分区间:
//!   - r ∈ [0, 1): x → 0 (灭绝)
//!   - r ∈ [1, 3): 稳定不动点 x* = 1 - 1/r
//!   - r ∈ [3, 1+√6 ≈ 3.449): 周期 2 极限环
//!   - r ∈ [3.449, 3.56995...): 周期倍化 4, 8, 16, ...
//!   - r ∈ (3.56995, 4]: 混沌带 (含周期窗口, 如 r≈3.83 周期 3)
//!   - r = 4: 完全混沌, 不变测度 ρ(x) = 1/(π√(x(1-x)))
//!
//! Feigenbaum 常数:
//!   δ = lim_{k→∞} (r_k - r_{k-1}) / (r_{k+1} - r_k) ≈ 4.6692
//!   普适于所有单峰映射 (任何具有二次极大的一维映射).
//!
//! Lyapunov 指数:
//!   λ = lim (1/N) Σ ln|r - 2r x_n| = ⟨ln|r(1 - 2x_n)|⟩
//!   - r < 3.56995: λ ≤ 0 (规则)
//!   - r = 4: λ = ln 2 ≈ 0.693
//!
//! 精确解 (r = 4):
//!   x_n = sin²(2^n θ_0), 其中 x_0 = sin²(θ_0)
//!
//! 参考:
//!   - May, R.M. 1976. "Simple mathematical models with very complicated
//!     dynamics." Nature 261, 459.
//!   - Feigenbaum, M.J. 1978. "Quantitative universality for a class of
//!     nonlinear transformations." J. Stat. Phys. 19, 25.
//!   - Strogatz, S. "Nonlinear Dynamics and Chaos", §10.

/// Logistic Map 配置
#[derive(Clone, Debug)]
pub struct LogisticMapConfig {
    /// 控制参数 r ∈ [0, 4]
    pub r: f64,
}

impl Default for LogisticMapConfig {
    fn default() -> Self {
        Self { r: 3.9 } // 混沌区
    }
}

/// Logistic Map 求解器 (1D 离散映射)
pub struct LogisticMapSolver {
    pub config: LogisticMapConfig,
    /// 当前状态 x ∈ [0, 1]
    pub x: f64,
    pub step_count: u64,
    /// 轨迹历史 (用于相图/分岔图)
    pub trajectory: Vec<f64>,
    /// Lyapunov 累积器: Σ ln|r - 2r x_n| = Σ ln|r(1 - 2x_n)|
    pub lyap_sum: f64,
}

impl LogisticMapSolver {
    pub fn new(config: LogisticMapConfig, x0: f64) -> Self {
        assert!(config.r >= 0.0 && config.r <= 4.0, "r must be in [0, 4]");
        assert!(x0 >= 0.0 && x0 <= 1.0, "x0 must be in [0, 1]");
        Self {
            config,
            x: x0,
            step_count: 0,
            trajectory: vec![x0],
            lyap_sum: 0.0,
        }
    }

    /// Logistic Map 一步: x_{n+1} = r x_n (1 - x_n)
    /// 返回新的 x
    pub fn step(&mut self) -> f64 {
        let r = self.config.r;
        let x_new = r * self.x * (1.0 - self.x);
        self.x = x_new;
        self.step_count += 1;
        self.trajectory.push(x_new);
        // Lyapunov: dF/dx = r(1 - 2x)
        let deriv = (r * (1.0 - 2.0 * x_new)).abs();
        if deriv > 0.0 {
            self.lyap_sum += deriv.ln();
        }
        // 若 deriv = 0 (临界点), 跳过该项 (中性)
        x_new
    }

    /// 多步推进
    pub fn run(&mut self, n_steps: usize) {
        for _ in 0..n_steps {
            self.step();
        }
    }

    /// 估计 Lyapunov 指数 (每步平均指数增长率)
    pub fn lyapunov(&self) -> f64 {
        if self.step_count == 0 {
            0.0
        } else {
            self.lyap_sum / self.step_count as f64
        }
    }

    /// 不动点: x* = 0 或 x* = 1 - 1/r (r > 1)
    pub fn fixed_points(&self) -> Vec<f64> {
        let r = self.config.r;
        let mut fps = vec![0.0];
        if r >= 1.0 {
            fps.push(1.0 - 1.0 / r);
        }
        fps
    }

    /// 不动点 x* = 1 - 1/r 的稳定性 (|F'(x*)| < 1 则稳定)
    /// F'(x*) = r(1 - 2x*) = r(1 - 2(1 - 1/r)) = r(2/r - 1) = 2 - r
    /// |2 - r| < 1 ⟺ 1 < r < 3
    pub fn fixed_point_stable(&self) -> bool {
        let r = self.config.r;
        r > 1.0 && r < 3.0
    }

    /// 检查 NaN/Inf
    pub fn has_nan(&self) -> bool {
        !self.x.is_finite()
    }

    /// 在 r=4 时的精确解: x_n = sin²(2^n θ_0), 其中 x_0 = sin²(θ_0)
    /// θ_0 = arcsin(√x_0) ∈ [0, π/2]
    pub fn exact_solution_r4(x0: f64, n: usize) -> f64 {
        let theta0 = x0.sqrt().asin();
        let arg = (2_u64.pow(n as u32) as f64) * theta0;
        arg.sin().powi(2)
    }

    /// 估计当前周期 (寻找轨迹中的循环)
    /// 跳过 transient (前 skip 项), 检查最后若干项是否有周期 P 循环
    /// 返回检测到的周期 (1=不动点, 2=周期2, ..., 0=未检测到)
    pub fn detect_period(&self, skip: usize, tol: f64) -> usize {
        if self.trajectory.len() < skip + 20 {
            return 0;
        }
        let tail = &self.trajectory[skip..];
        let n = tail.len();
        // 取最后一个点作为参考, 检查前面 P 步是否回到该值
        let last = tail[n - 1];
        for p in 1..=32 {
            if p + 1 > n {
                break;
            }
            let candidate = tail[n - 1 - p];
            if (candidate - last).abs() < tol {
                // 验证: 中间点也应周期重复
                let mut ok = true;
                for k in 1..p.min(n / 2) {
                    let a = tail[n - 1 - k];
                    let b = tail[n - 1 - k - p];
                    if (a - b).abs() >= tol {
                        ok = false;
                        break;
                    }
                }
                if ok {
                    return p;
                }
            }
        }
        0
    }
}

/// Feigenbaum 分岔点: 检测周期 2^k → 2^{k+1} 的 r 值
/// 返回前几个分岔 r_k (近似)
pub fn feigenbaum_bifurcation_points() -> Vec<f64> {
    // 已知数值 (高精度)
    vec![
        3.0,            // r_1: 周期 1 → 周期 2
        3.449490,       // r_2: 周期 2 → 周期 4
        3.544090,       // r_3: 周期 4 → 周期 8
        3.564407,       // r_4: 周期 8 → 周期 16
        3.568759,       // r_5: 周期 16 → 周期 32
        3.569692,       // r_6: 周期 32 → 周期 64
    ]
}

/// Feigenbaum δ 常数 (理论值)
pub const FEIGENBAUM_DELTA: f64 = 4.669201609102990;

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn test_default_config() {
        let cfg = LogisticMapConfig::default();
        assert!(approx_eq(cfg.r, 3.9, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = LogisticMapSolver::new(LogisticMapConfig { r: 2.0 }, 0.5);
        assert_eq!(s.step_count, 0);
        assert_eq!(s.trajectory.len(), 1);
        assert!(approx_eq(s.x, 0.5, 1e-12));
    }

    #[test]
    #[should_panic(expected = "r must be in [0, 4]")]
    fn test_r_out_of_range_high() {
        let _ = LogisticMapSolver::new(LogisticMapConfig { r: 5.0 }, 0.5);
    }

    #[test]
    #[should_panic(expected = "r must be in [0, 4]")]
    fn test_r_out_of_range_low() {
        let _ = LogisticMapSolver::new(LogisticMapConfig { r: -1.0 }, 0.5);
    }

    #[test]
    #[should_panic(expected = "x0 must be in [0, 1]")]
    fn test_x0_out_of_range() {
        let _ = LogisticMapSolver::new(LogisticMapConfig { r: 2.0 }, 1.5);
    }

    #[test]
    fn test_step_advances() {
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 2.0 }, 0.5);
        s.step();
        assert_eq!(s.step_count, 1);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_step_analytic() {
        // 手动验证: x_{n+1} = r x (1 - x)
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 3.0 }, 0.2);
        s.step();
        let expected = 3.0 * 0.2 * 0.8;
        assert!(approx_eq(s.x, expected, 1e-12));
    }

    #[test]
    fn test_r_zero_goes_to_zero() {
        // r=0: 任何 x 都映射到 0
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 0.0 }, 0.5);
        s.run(10);
        assert!(approx_eq(s.x, 0.0, 1e-12));
    }

    #[test]
    fn test_r_one_goes_to_zero() {
        // r=1: x_{n+1} = x(1-x), 趋向 0
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 1.0 }, 0.5);
        s.run(1000);
        assert!(s.x < 0.01, "r=1 → 0: x={}", s.x);
    }

    #[test]
    fn test_stable_fixed_point_r2() {
        // r=2: 稳定不动点 x* = 1 - 1/2 = 0.5
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 2.0 }, 0.3);
        s.run(1000);
        assert!(approx_eq(s.x, 0.5, 1e-9), "r=2 fixed point: x={}", s.x);
    }

    #[test]
    fn test_fixed_points_analytic() {
        let s = LogisticMapSolver::new(LogisticMapConfig { r: 2.5 }, 0.5);
        let fps = s.fixed_points();
        assert_eq!(fps.len(), 2);
        assert!(approx_eq(fps[0], 0.0, 1e-12));
        assert!(approx_eq(fps[1], 1.0 - 1.0 / 2.5, 1e-12));
    }

    #[test]
    fn test_fixed_point_stable_low_r() {
        // r=2.5 ∈ (1, 3): 稳定
        let s = LogisticMapSolver::new(LogisticMapConfig { r: 2.5 }, 0.5);
        assert!(s.fixed_point_stable());
    }

    #[test]
    fn test_fixed_point_unstable_high_r() {
        // r=3.5 > 3: 不稳定 (周期 4)
        let s = LogisticMapSolver::new(LogisticMapConfig { r: 3.5 }, 0.5);
        assert!(!s.fixed_point_stable());
    }

    #[test]
    fn test_period2_cycle_r32() {
        // r=3.2 ∈ (3, 3.449): 周期 2
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 3.2 }, 0.5);
        s.run(2000); // 收敛到周期 2
        let p = s.detect_period(1900, 1e-6);
        assert_eq!(p, 2, "r=3.2 period: {}", p);
    }

    #[test]
    fn test_period4_cycle_r35() {
        // r=3.5 ∈ (3.449, 3.56995): 周期 4
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 3.5 }, 0.5);
        s.run(5000);
        let p = s.detect_period(4000, 1e-6);
        assert_eq!(p, 4, "r=3.5 period: {}", p);
    }

    #[test]
    fn test_period3_window_r383() {
        // r≈3.83: 周期 3 窗口 (Sarkovskii 定理: 周期 3 意味着所有周期)
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 3.83 }, 0.5);
        s.run(5000);
        let p = s.detect_period(4000, 1e-4);
        assert_eq!(p, 3, "r=3.83 period 3 window: {}", p);
    }

    #[test]
    fn test_chaos_at_r4() {
        // r=4: 混沌, 周期应无法检测 (返回 0 或较大值)
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 4.0 }, 0.123);
        s.run(5000);
        let p = s.detect_period(4000, 1e-9);
        assert_eq!(p, 0, "r=4 chaos: period should be 0, got {}", p);
    }

    #[test]
    fn test_lyapunov_negative_stable() {
        // r=2.5 (< 3): 稳定不动点, λ < 0
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 2.5 }, 0.3);
        s.run(10000);
        let lambda = s.lyapunov();
        assert!(lambda < 0.0, "r=2.5 stable λ<0: {}", lambda);
    }

    #[test]
    fn test_lyapunov_positive_chaos() {
        // r=4: 混沌, λ > 0 (理论上 λ = ln 2 ≈ 0.693)
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 4.0 }, 0.123);
        s.run(50000);
        let lambda = s.lyapunov();
        assert!(lambda > 0.5, "r=4 chaos λ>0.5: {}", lambda);
        assert!(lambda < 0.9, "r=4 λ~ln(2): {}", lambda);
    }

    #[test]
    fn test_lyapunov_zero_at_bifurcation() {
        // r=3: 分岔点 (周期 1 → 2), F'(x*) = 2 - r = -1, |F'| = 1, λ = 0
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 3.0 }, 0.5);
        s.run(10000);
        let lambda = s.lyapunov();
        // 在分岔点附近, λ 应接近 0
        assert!(lambda.abs() < 0.1, "r=3 λ≈0: {}", lambda);
    }

    #[test]
    fn test_exact_solution_r4() {
        // r=4 时精确解: x_n = sin²(2^n θ_0)
        let x0 = 0.3_f64;
        let theta0 = x0.sqrt().asin();
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 4.0 }, x0);
        for n in 1..=20 {
            s.step();
            let exact = ((2_u64.pow(n as u32) as f64) * theta0).sin().powi(2);
            assert!(approx_eq(s.x, exact, 1e-9),
                "r=4 exact at n={}: x={}, expected={}", n, s.x, exact);
        }
    }

    #[test]
    fn test_invariant_density_r4() {
        // r=4: 不变测度 ρ(x) = 1/(π √(x(1-x))), 均值 ⟨x⟩ = 1/2
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 4.0 }, 0.314);
        s.run(100000);
        // 跳过 transient, 计算时间平均
        let tail = &s.trajectory[10000..];
        let mean: f64 = tail.iter().sum::<f64>() / tail.len() as f64;
        assert!((mean - 0.5).abs() < 0.01, "r=4 ⟨x⟩=1/2: {}", mean);
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 4.0 }, 0.5);
        s.run(100000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_r_zero() {
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 0.0 }, 0.5);
        s.run(1000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_dt_flexible_r() {
        // 不同 r 值下都应正常演化
        for r in [0.0_f64, 1.0, 2.0, 3.0, 3.5, 3.83, 4.0] {
            let mut s = LogisticMapSolver::new(LogisticMapConfig { r }, 0.5);
            s.run(1000);
            assert!(!s.has_nan(), "r={}: no NaN", r);
            assert!(s.x >= 0.0 && s.x <= 1.0, "r={}: x in [0,1]", r);
        }
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 3.5 }, 0.5);
        s.run(100);
        assert_eq!(s.trajectory.len(), 101);
    }

    #[test]
    fn test_x_stays_in_unit_interval() {
        // 对 r ∈ [0, 4], 若 x_0 ∈ [0, 1], 则 x_n ∈ [0, 1] (映射保持区间)
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 4.0 }, 0.99);
        s.run(10000);
        assert!(s.x >= 0.0 && s.x <= 1.0, "x stays in [0,1]: {}", s.x);
    }

    #[test]
    fn test_feigenbaum_points_known() {
        // 检查已知分岔点
        let pts = feigenbaum_bifurcation_points();
        assert!(pts.len() >= 4);
        assert!(approx_eq(pts[0], 3.0, 1e-6));
        assert!(approx_eq(pts[1], 3.449490, 1e-5));
        assert!(approx_eq(pts[2], 3.544090, 1e-5));
        // 序列单调递增
        for i in 1..pts.len() {
            assert!(pts[i] > pts[i - 1]);
        }
    }

    #[test]
    fn test_feigenbaum_delta_constant() {
        // δ = lim (r_k - r_{k-1}) / (r_{k+1} - r_k) ≈ 4.6692
        let pts = feigenbaum_bifurcation_points();
        // 使用前 5 个点估计 δ
        let d1 = (pts[2] - pts[1]) / (pts[3] - pts[2]);
        let d2 = (pts[3] - pts[2]) / (pts[4] - pts[3]);
        // 应收敛到 ~4.669
        assert!(d1 > 3.0 && d1 < 6.0, "δ_1 ≈ 4.669: {}", d1);
        assert!(d2 > 4.0 && d2 < 5.5, "δ_2 ≈ 4.669: {}", d2);
        assert!((d2 - FEIGENBAUM_DELTA).abs() < (d1 - FEIGENBAUM_DELTA).abs(),
            "δ should converge: d1={}, d2={}", d1, d2);
    }

    #[test]
    fn test_feigenbaum_delta_value() {
        assert!(approx_eq(FEIGENBAUM_DELTA, 4.669201609102990, 1e-12));
    }

    #[test]
    fn test_sarkovskii_period3_implies_all() {
        // Sarkovskii 定理: 若 f 有周期 3 轨道, 则有所有周期的轨道
        // 这里仅检验 r=3.83 的周期 3 窗口存在
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 3.83 }, 0.5);
        s.run(5000);
        let p = s.detect_period(4000, 1e-4);
        assert_eq!(p, 3, "Sarkovskii period 3: {}", p);
    }

    #[test]
    fn test_chaos_sensitivity_r4() {
        // r=4 混沌: 相近初值指数发散
        // 注: 避免 x0=0.5 (临界点 F'(0.5)=0, 此处 4·0.5·0.5=1.0 是 F 的最大值,
        // 微扰 δ 在 1-4δ² 中被舍入消失, 两轨道完全相同).
        // 取 x0=0.3 远离临界点, 1e-10 扰动能在 f64 下存活并被 Lyapunov 放大.
        let mut s1 = LogisticMapSolver::new(LogisticMapConfig { r: 4.0 }, 0.3);
        let mut s2 = LogisticMapSolver::new(LogisticMapConfig { r: 4.0 }, 0.3 + 1e-10);
        s1.run(100);
        s2.run(100);
        let d = (s1.x - s2.x).abs();
        // Lyapunov λ ≈ ln 2 ≈ 0.693, 100 步后放大 e^69.3 ≈ 1.4e30
        // 但 x ∈ [0,1] 饱和, 应已饱和到 O(0.1)
        assert!(d > 0.01, "chaos divergence: d={}", d);
    }

    #[test]
    fn test_critical_point_deriv_zero() {
        // 临界点 x=0.5: F'(0.5) = r(1 - 2·0.5) = 0
        // 此处 Lyapunov 累积器应跳过 (避免 ln 0)
        let mut s = LogisticMapSolver::new(LogisticMapConfig { r: 4.0 }, 0.5);
        s.step(); // x_1 = 4·0.5·0.5 = 1.0
        s.step(); // x_2 = 4·1·0 = 0
        s.step(); // x_3 = 4·0·1 = 0
        // 不应产生 NaN
        assert!(!s.has_nan());
    }
}
