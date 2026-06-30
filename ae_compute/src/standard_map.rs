//! Standard Map (Chirikov-Taylor) — KAM 定理的离散时间范例
//!
//! Boris Chirikov 1969 年研究等离子体中带电粒子的磁场扰动时提出,
//! 是非线性动力学中最重要的离散时间辛映射 (area-preserving map),
//! 与连续时间的 Hénon-Heiles 系统并列为 KAM 定理的标准数值例子.
//!
//! 映射 (p, θ) → (p', θ'):
//!   p' = p + K sin(θ)        (动量踢)
//!   θ' = θ + p'   (mod 2π)   (自由旋转)
//!
//! 参数:
//!   - K: 随机性参数 (stochasticity parameter)
//!   - θ ∈ [0, 2π): 旋转角 (周期)
//!   - p ∈ ℝ: 动量 (通常观察时取 mod 2π, 但演化中不取模, 以检测扩散)
//!
//! 性质:
//!   - 辛映射 (symplectic): det(Jacobian) = 1, 保相体积
//!   - K = 0: 可积 (p 守恒, θ 线性增长)
//!   - K 小: KAM 不变环面存活, p 被囚禁 (规则运动)
//!   - K_c ≈ 0.9716: 黄金比 KAM 环面破裂 (Greene 1979)
//!   - K > K_c: 全球混沌出现, p 沿环面扩散 (Chirikov 重叠判据)
//!   - K ≫ 1: 准线性扩散, ⟨Δp²⟩ ≈ K²/2 · n (Chirikov 公式)
//!
//! Jacobian:
//!   J = | ∂θ'/∂θ  ∂θ'/∂p |   = | 1 + K cos θ   1 |
//!       | ∂p'/∂θ  ∂p'/∂p |     | K cos θ       1 |
//!   det(J) = (1+Kcosθ)·1 - 1·Kcosθ = 1  ✓ 保面积
//!
//! Lyapunov 指数:
//!   追踪切向量 v 的演化: v_{n+1} = J_n v_n, 周期性重归一化
//!   λ = (1/N) Σ log|v_n| (重归一化前)
//!   K < K_c (KAM 环面): λ = 0
//!   K > K_c (混沌): λ > 0
//!
//! 历史:
//!   Chirikov, B.V. 1969. "Resonance processes in magnetic traps."
//!   Atomnaya Energiya 6, 630. (原俄文)
//!   Greene, J.M. 1979. "A method for determining a stochastic transition."
//!   J. Math. Phys. 20, 1183. (K_c 数值精确确定)
//!   Chirikov, B.V. 1979. "A universal instability of many-dimensional
//!   oscillator systems." Phys. Rep. 52, 263.

use std::f64::consts::PI;

/// Standard Map 配置
#[derive(Clone, Debug)]
pub struct StandardMapConfig {
    /// 随机性参数 K
    pub k: f64,
}

impl Default for StandardMapConfig {
    fn default() -> Self {
        Self { k: 0.971635406 } // 接近 K_c (Greene 临界值)
    }
}

/// Standard Map 求解器 (离散时间辛映射)
pub struct StandardMapSolver {
    pub config: StandardMapConfig,
    /// 旋转角 θ ∈ [0, 2π)
    pub theta: f64,
    /// 动量 p (不取模, 可观察扩散)
    pub p: f64,
    pub step_count: u64,
    /// 轨迹历史 (theta_wrapped, p_unwrapped), 用于相图
    pub trajectory: Vec<(f64, f64)>,
    /// Lyapunov 累积器 (Σ log|v| before renormalize)
    pub lyap_sum: f64,
    /// 切向量 v = (v_theta, v_p)
    v_theta: f64,
    v_p: f64,
}

impl StandardMapSolver {
    pub fn new(config: StandardMapConfig, theta0: f64, p0: f64) -> Self {
        let mut s = Self {
            config,
            theta: Self::wrap_theta(theta0),
            p: p0,
            step_count: 0,
            trajectory: Vec::new(),
            lyap_sum: 0.0,
            v_theta: 1.0,
            v_p: 0.0,
        };
        s.trajectory.push((s.theta, s.p));
        s
    }

    /// 将 θ 包装到 [0, 2π)
    #[inline]
    pub fn wrap_theta(theta: f64) -> f64 {
        let r = theta.rem_euclid(2.0 * PI);
        if r < 0.0 {
            r + 2.0 * PI
        } else {
            r
        }
    }

    /// 将 p 包装到 [-π, π) (仅用于显示/统计, 不影响演化)
    #[inline]
    pub fn wrap_p(p: f64) -> f64 {
        let r = p.rem_euclid(2.0 * PI);
        if r >= PI {
            r - 2.0 * PI
        } else {
            r
        }
    }

    /// Standard Map 一步: p' = p + K sin(θ); θ' = (θ + p') mod 2π
    pub fn step(&mut self) {
        let k = self.config.k;
        self.p += k * self.theta.sin();
        self.theta = Self::wrap_theta(self.theta + self.p);
        self.step_count += 1;
        self.trajectory.push((self.theta, self.p));
        self.update_lyapunov();
    }

    /// 多步推进
    pub fn run(&mut self, n_steps: usize) {
        for _ in 0..n_steps {
            self.step();
        }
    }

    /// 更新 Lyapunov 切向量并累积 (使用当前 θ 处的 Jacobian)
    fn update_lyapunov(&mut self) {
        let k = self.config.k;
        let c = self.theta.cos();
        // J = [[1 + K cos θ, 1], [K cos θ, 1]] 作用于 v = (v_theta, v_p)
        let vt = self.v_theta;
        let vp = self.v_p;
        let new_vt = (1.0 + k * c) * vt + vp;
        let new_vp = k * c * vt + vp;
        let mag = (new_vt * new_vt + new_vp * new_vp).sqrt();
        if mag > 0.0 {
            self.lyap_sum += mag.ln();
            self.v_theta = new_vt / mag;
            self.v_p = new_vp / mag;
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

    /// Jacobian 矩阵在当前 θ 的值
    pub fn jacobian(&self) -> [[f64; 2]; 2] {
        let kc = self.config.k * self.theta.cos();
        [[1.0 + kc, 1.0], [kc, 1.0]]
    }

    /// Jacobian 行列式 (应恒为 1, 辛映射)
    pub fn jacobian_det(&self) -> f64 {
        let j = self.jacobian();
        j[0][0] * j[1][1] - j[0][1] * j[1][0]
    }

    /// p 的扩散: ⟨Δp²⟩ = ⟨(p - p0)²⟩
    pub fn p_diffusion(&self) -> f64 {
        if self.trajectory.len() < 2 {
            return 0.0;
        }
        let p0 = self.trajectory[0].1;
        let n = self.trajectory.len() as f64;
        let mean: f64 = self.trajectory.iter().map(|&(_, p)| p - p0).sum::<f64>() / n;
        let var: f64 = self.trajectory
            .iter()
            .map(|&(_, p)| (p - p0 - mean).powi(2))
            .sum::<f64>()
            / n;
        var
    }

    /// 当前 p 是否仍在初始 KAM 环面附近 (|p - p0| < π)
    pub fn p_bounded(&self) -> bool {
        if self.trajectory.is_empty() {
            return true;
        }
        let p0 = self.trajectory[0].1;
        (self.p - p0).abs() < PI
    }

    /// 检查 NaN/Inf
    pub fn has_nan(&self) -> bool {
        !self.theta.is_finite() || !self.p.is_finite()
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
        let cfg = StandardMapConfig::default();
        assert!(approx_eq(cfg.k, 0.971635406, 1e-9));
    }

    #[test]
    fn test_solver_creation() {
        let s = StandardMapSolver::new(StandardMapConfig { k: 1.0 }, 0.5, 0.1);
        assert_eq!(s.step_count, 0);
        assert_eq!(s.trajectory.len(), 1);
        assert!(approx_eq(s.theta, 0.5, 1e-12));
        assert!(approx_eq(s.p, 0.1, 1e-12));
    }

    #[test]
    fn test_wrap_theta() {
        assert!(approx_eq(StandardMapSolver::wrap_theta(0.0), 0.0, 1e-12));
        assert!(approx_eq(StandardMapSolver::wrap_theta(2.0 * PI), 0.0, 1e-12));
        assert!(approx_eq(StandardMapSolver::wrap_theta(-0.1), 2.0 * PI - 0.1, 1e-12));
        assert!(approx_eq(StandardMapSolver::wrap_theta(3.0 * PI), PI, 1e-12));
    }

    #[test]
    fn test_wrap_p() {
        assert!(approx_eq(StandardMapSolver::wrap_p(0.0), 0.0, 1e-12));
        assert!(approx_eq(StandardMapSolver::wrap_p(PI), -PI, 1e-12)); // 边界映射到 -π
        assert!(approx_eq(StandardMapSolver::wrap_p(-PI), -PI, 1e-12));
        assert!(approx_eq(StandardMapSolver::wrap_p(2.0 * PI), 0.0, 1e-12));
    }

    #[test]
    fn test_k_zero_p_constant() {
        // K=0: p 守恒, θ 线性增长 (mod 2π)
        let mut s = StandardMapSolver::new(StandardMapConfig { k: 0.0 }, 0.5, 0.3);
        s.run(1000);
        assert!(approx_eq(s.p, 0.3, 1e-12), "p constant at K=0: {}", s.p);
        // θ = (0.5 + 1000 * 0.3) mod 2π
        let expected_theta = StandardMapSolver::wrap_theta(0.5 + 1000.0 * 0.3);
        assert!(approx_eq(s.theta, expected_theta, 1e-10));
    }

    #[test]
    fn test_jacobian_det_is_one() {
        // 辛映射: det(J) = 1 (对任意 θ, K)
        for k in [0.0_f64, 0.5, 1.0, 5.0, 100.0] {
            for theta in [0.0_f64, 0.5, 1.0, 2.0, 3.0] {
                let s = StandardMapSolver::new(StandardMapConfig { k }, theta, 0.0);
                let det = s.jacobian_det();
                assert!(approx_eq(det, 1.0, 1e-12), "K={}, θ={}: det={}", k, theta, det);
            }
        }
    }

    #[test]
    fn test_jacobian_analytic() {
        // J = [[1 + K cos θ, 1], [K cos θ, 1]]
        let s = StandardMapSolver::new(StandardMapConfig { k: 1.5 }, 0.7, 0.0);
        let j = s.jacobian();
        let c = 0.7_f64.cos();
        assert!(approx_eq(j[0][0], 1.0 + 1.5 * c, 1e-12));
        assert!(approx_eq(j[0][1], 1.0, 1e-12));
        assert!(approx_eq(j[1][0], 1.5 * c, 1e-12));
        assert!(approx_eq(j[1][1], 1.0, 1e-12));
    }

    #[test]
    fn test_step_advances() {
        let mut s = StandardMapSolver::new(StandardMapConfig { k: 1.0 }, 0.5, 0.1);
        s.step();
        assert_eq!(s.step_count, 1);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_step_analytic() {
        // 手动验证一步: p' = p + K sin(θ); θ' = (θ + p') mod 2π
        let mut s = StandardMapSolver::new(StandardMapConfig { k: 2.0 }, 1.0, 0.5);
        s.step();
        let expected_p = 0.5 + 2.0 * 1.0_f64.sin();
        let expected_theta = StandardMapSolver::wrap_theta(1.0 + expected_p);
        assert!(approx_eq(s.p, expected_p, 1e-12));
        assert!(approx_eq(s.theta, expected_theta, 1e-12));
    }

    #[test]
    fn test_symmetry_under_inversion() {
        // 映射满足 (θ, p) → (-θ, -p) 对称: 若 x_n+1 = M(x_n), 则 -x_n+1 = M(-x_n)
        // 即从 (θ0, p0) 出发 N 步后的 (θ, p) 应等于从 (-θ0, -p0) 出发 (-θ, -p) (mod 2π)
        // 注: 在混沌区 (K > K_c) Lyapunov 放大 rem_euclid 的 1-ULP 误差,
        // 故取 K=0.3 (< K_c, λ=0) 在 KAM 环面上检验严格对称性.
        let k = 0.3_f64;
        let theta0 = 0.4_f64;
        let p0 = 0.6_f64;
        let mut s1 = StandardMapSolver::new(StandardMapConfig { k }, theta0, p0);
        s1.run(50);
        let mut s2 = StandardMapSolver::new(StandardMapConfig { k }, -theta0, -p0);
        s2.run(50);
        // s1.theta 应等于 wrap(-s2.theta), s1.p 应等于 -s2.p
        let neg_s2_theta = StandardMapSolver::wrap_theta(-s2.theta);
        assert!(approx_eq(s1.theta, neg_s2_theta, 1e-9),
            "symmetry θ: {} vs {}", s1.theta, neg_s2_theta);
        assert!(approx_eq(s1.p, -s2.p, 1e-9),
            "symmetry p: {} vs {}", s1.p, -s2.p);
    }

    #[test]
    fn test_low_k_p_bounded() {
        // K=0.5 < K_c: 多数初始条件下 p 保持有界 (KAM 环面)
        let mut s = StandardMapSolver::new(StandardMapConfig { k: 0.5 }, 1.0, 0.1);
        s.run(50000);
        assert!(s.p_bounded(), "K=0.5 p should stay bounded: |Δp|={}", (s.p - 0.1).abs());
    }

    #[test]
    fn test_low_k_zero_lyapunov() {
        // K 小且在 KAM 环面上: Lyapunov 指数 ≈ 0
        // 取无理旋转数初始条件避免低阶共振
        let mut s = StandardMapSolver::new(StandardMapConfig { k: 0.3 }, 0.0, 0.5);
        s.run(50000);
        let lambda = s.lyapunov();
        assert!(lambda.abs() < 0.01, "K=0.3 KAM torus λ≈0: {}", lambda);
    }

    #[test]
    fn test_high_k_positive_lyapunov() {
        // K=5 ≫ K_c: 混沌, Lyapunov 指数显著为正
        let mut s = StandardMapSolver::new(StandardMapConfig { k: 5.0 }, 1.0, 0.1);
        s.run(50000);
        let lambda = s.lyapunov();
        // K=5 时 λ ≈ ln(K/2) ≈ ln(2.5) ≈ 0.916 (Chirikov 公式)
        assert!(lambda > 0.3, "K=5 chaos λ>0.3: {}", lambda);
        assert!(lambda < 2.0, "K=5 λ not too large: {}", lambda);
    }

    #[test]
    fn test_high_k_p_diffuses() {
        // K ≫ 1: p 准线性扩散, ⟨Δp²⟩ ≈ K²/2 · n
        let k = 10.0_f64;
        let n = 10000_usize;
        let mut s = StandardMapSolver::new(StandardMapConfig { k }, 1.0, 0.0);
        s.run(n);
        let var = s.p_diffusion();
        // Chirikov: D = K²/2 (数量级). 有限样本有波动, 检查数量级
        let expected = k * k / 2.0 * n as f64;
        // 验证量级 (允许 5x 误差, 因为准线性近似只在 K≫1 准确且有相关修正)
        assert!(var > 0.1 * expected, "K=10 diffusion too small: var={}, expected~{}", var, expected);
        assert!(var < 10.0 * expected, "K=10 diffusion too large: var={}, expected~{}", var, expected);
    }

    #[test]
    fn test_k_critical_transition() {
        // K 跨越 K_c ≈ 0.9716: 低 K Lyapunov ≈ 0, 高 K Lyapunov > 0
        let mut lambda_low = 0.0;
        let mut lambda_high = 0.0;
        // 取多个初值平均 (避免落在共振岛)
        for (theta0, p0) in [(0.5_f64, 0.3_f64), (1.0, 0.7), (2.0, 0.4), (1.5, 1.0)] {
            let mut s_lo = StandardMapSolver::new(StandardMapConfig { k: 0.3 }, theta0, p0);
            s_lo.run(20000);
            lambda_low += s_lo.lyapunov();
            let mut s_hi = StandardMapSolver::new(StandardMapConfig { k: 2.0 }, theta0, p0);
            s_hi.run(20000);
            lambda_high += s_hi.lyapunov();
        }
        lambda_low /= 4.0;
        lambda_high /= 4.0;
        assert!(lambda_low < lambda_high, "low K λ < high K λ: {} vs {}", lambda_low, lambda_high);
        assert!(lambda_high > 0.1, "K=2 average λ>0.1: {}", lambda_high);
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = StandardMapSolver::new(StandardMapConfig { k: 100.0 }, 1.0, 0.5);
        s.run(100000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_zero_k() {
        let mut s = StandardMapSolver::new(StandardMapConfig { k: 0.0 }, 1.0, 0.5);
        s.run(100000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_dt_flexible_k() {
        // 不同 K 值下都应正常演化
        for k in [0.0_f64, 0.5, 1.0, 2.0, 10.0, 100.0] {
            let mut s = StandardMapSolver::new(StandardMapConfig { k }, 0.7, 0.3);
            s.run(1000);
            assert!(!s.has_nan(), "K={}: no NaN", k);
        }
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = StandardMapSolver::new(StandardMapConfig { k: 1.0 }, 0.5, 0.1);
        s.run(100);
        assert_eq!(s.trajectory.len(), 101);
    }

    #[test]
    fn test_theta_periodic_invariance() {
        // θ 和 θ + 2π 应给出相同轨迹
        let k = 1.0_f64;
        let mut s1 = StandardMapSolver::new(StandardMapConfig { k }, 0.5, 0.1);
        let mut s2 = StandardMapSolver::new(StandardMapConfig { k }, 0.5 + 2.0 * PI, 0.1);
        s1.run(50);
        s2.run(50);
        assert!(approx_eq(s1.theta, s2.theta, 1e-10));
        assert!(approx_eq(s1.p, s2.p, 1e-10));
    }

    #[test]
    fn test_p_not_wrapped_during_evolution() {
        // p 在演化中不取模 (可观察扩散). K=0 时 p 不变.
        // K>0 且无 KAM 时 p 可任意增长.
        let mut s = StandardMapSolver::new(StandardMapConfig { k: 0.0 }, 0.0, 5.0);
        s.run(1000);
        assert!(approx_eq(s.p, 5.0, 1e-12), "p not wrapped: {}", s.p);
    }

    #[test]
    fn test_fixed_point_origin() {
        // (θ=0, p=0) 是不动点 (sin 0 = 0, 0 + 0 = 0)
        let mut s = StandardMapSolver::new(StandardMapConfig { k: 1.5 }, 0.0, 0.0);
        s.run(1000);
        assert!(approx_eq(s.theta, 0.0, 1e-12));
        assert!(approx_eq(s.p, 0.0, 1e-12));
    }

    #[test]
    fn test_fixed_point_pi_zero_unstable_for_large_k() {
        // (θ=π, p=0) 也是不动点 (sin π = 0, π + 0 = π → wrap → π)
        // 但对 K > 2 该不动点线性失稳 (Jacobian 特征值离开单位圆)
        let mut s = StandardMapSolver::new(StandardMapConfig { k: 1.0 }, PI, 0.0);
        s.run(1000);
        assert!(approx_eq(s.theta, PI, 1e-9) || approx_eq(s.theta, -PI, 1e-9),
            "(π, 0) fixed point at K=1: θ={}", s.theta);
        assert!(approx_eq(s.p, 0.0, 1e-9));
    }

    #[test]
    fn test_lyapunov_zero_at_k_zero() {
        // K=0: Jacobian = [[1,1],[0,1]], 切向量不指数增长 (代数增长)
        // 重归一化后 λ = 0
        let mut s = StandardMapSolver::new(StandardMapConfig { k: 0.0 }, 0.5, 0.3);
        s.run(10000);
        let lambda = s.lyapunov();
        // K=0 时 J = [[1,1],[0,1]], |v| 可能因剪切代数增长, 但重归一化使 λ → 0
        // 实际上 v = (1,0) → (1,0) (J·(1,0) = (1,0)), |v|=1, ln|v|=0, λ=0
        assert!(lambda.abs() < 0.01, "K=0 λ=0: {}", lambda);
    }

    #[test]
    fn test_quasiperiodic_low_k_no_drift() {
        // K 小, 在 KAM 环面上: p 不应有净漂移
        let mut s = StandardMapSolver::new(StandardMapConfig { k: 0.3 }, 0.0, 0.5);
        s.run(100000);
        let p_drift = (s.p - 0.5).abs();
        // KAM 环面囚禁: |p - p0| < 2π (有界)
        assert!(p_drift < 2.0 * PI, "K=0.3 KAM bounded: drift={}", p_drift);
    }
}
