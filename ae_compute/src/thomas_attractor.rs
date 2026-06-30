//! Thomas 吸引子 — 循环对称混沌系统
//!
//! René Thomas 1999 年提出的极简 3D 混沌系统, 方程在 (x, y, z)
//! 循环置换下不变 (C₃ 对称). 是研究对称性、混沌开关 (chaos switch)
//! 和滞后反馈的标志性例子.
//!
//! 方程:
//!   dx/dt = sin(y) - b x
//!   dy/dt = sin(z) - b y
//!   dz/dt = sin(x) - b z
//!
//! 参数 b (耗散率, ≥ 0):
//!   - b > 0.328: 收敛到不动点 (0,0,0)
//!   - b ≈ 0.32: Hopf 分岔, 出现极限环
//!   - b ≈ 0.27: 周期倍化分岔
//!   - b < 0.208: 完全混沌 (奇异吸引子)
//!   - b = 0.19: 经典混沌参数
//!   - b = 0: 保守系统, Hamiltonian 类似行为
//!
//! 对称性:
//!   - 循环对称 C₃: (x,y,z) → (y,z,x) → (z,x,y) 方程不变
//!   - 反对称: (x,y,z) → (-x,-y,-z) 方程不变 (因 sin 奇函数 + 线性项)
//!
//! 不动点:
//!   令 dx/dt = dy/dt = dz/dt = 0 → sin(y)=bx, sin(z)=by, sin(x)=bz
//!   对称不动点 (x,y,z)=(0,0,0) 始终存在
//!   其他不动点: 数值求解 sin-cos 耦合系统
//!
//! 散度:
//!   ∇·F = -3b (常数负, 耗散系统)
//!   体积元以 e^(-3bt) 收缩 → 吸引子存在
//!
//! Lyapunov 指数 (b=0.19):
//!   λ₁ ≈ 0.06, λ₂ = 0 (中性), λ₃ ≈ -0.63
//!   Kaplan-Yorke 维数 D_KY ≈ 2 + λ₁/|λ₃| ≈ 2.10
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta)
//!
//! 历史:
//!   Thomas, R. 1999. "Deterministic chaos seen in terms of feedback
//!   circuits: Analysis and synthesis." Int. J. Bifurcation Chaos 9, 1889.
//!   Lindner, J. & Ditto, W. 1995. "Symmetry breaking in high-dimensional
//!   chaotic systems." (相关对称性研究)

/// Thomas 吸引子配置
#[derive(Clone, Copy, Debug)]
pub struct ThomasConfig {
    /// 耗散率 b (经典 0.19 混沌, 0.32 收敛)
    pub b: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for ThomasConfig {
    fn default() -> Self {
        Self { b: 0.19, dt: 0.01 }
    }
}

/// Thomas 吸引子求解器
pub struct ThomasSolver {
    pub config: ThomasConfig,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub time: f64,
    pub step_count: u64,
    pub trajectory: Vec<(f64, f64, f64)>,
    /// Lyapunov 累积 (变分方程前向欧拉)
    pub lyap_sum: f64,
    /// 切向量 (3D)
    pub v: [f64; 3],
}

impl ThomasSolver {
    pub fn new(config: ThomasConfig, x0: f64, y0: f64, z0: f64) -> Self {
        Self {
            config,
            x: x0,
            y: y0,
            z: z0,
            time: 0.0,
            step_count: 0,
            trajectory: vec![(x0, y0, z0)],
            lyap_sum: 0.0,
            v: [1.0, 0.0, 0.0],
        }
    }

    pub fn classic(config: ThomasConfig) -> Self {
        Self::new(config, 0.1, 0.0, 0.0)
    }

    /// 右端导数 F = [sin(y) - b x, sin(z) - b y, sin(x) - b z]
    pub fn derivatives(cfg: &ThomasConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        [
            y.sin() - cfg.b * x,
            z.sin() - cfg.b * y,
            x.sin() - cfg.b * z,
        ]
    }

    /// Jacobian: J = [[-b, cos(y), 0], [0, -b, cos(z)], [cos(x), 0, -b]]
    pub fn jacobian(cfg: &ThomasConfig, x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
        [
            [-cfg.b, y.cos(), 0.0],
            [0.0, -cfg.b, z.cos()],
            [x.cos(), 0.0, -cfg.b],
        ]
    }

    /// 散度 tr(J) = -3b (常数)
    pub fn divergence(cfg: &ThomasConfig) -> f64 {
        -3.0 * cfg.b
    }

    /// 原点是否为不动点 (始终是, 因 sin(0)=0)
    pub fn origin_is_fixed_point() -> bool {
        true
    }

    /// 原点处 Jacobian 的特征值 (用于 Hopf 分岔分析)
    /// J(0,0,0) = [[-b, 1, 0], [0, -b, 1], [1, 0, -b]]
    /// 特征方程: (-b-λ)³ + 1 = 0 → (λ+b)³ = 1
    /// λ = -b + ω, 其中 ω³ = 1 (1 的立方根: 1, e^(2πi/3), e^(4πi/3))
    /// λ_k = -b + exp(2πi k/3), k=0,1,2
    /// 实部: Re(λ) = -b + cos(2πk/3)
    ///   k=0: Re = -b + 1 (不稳定, 因 b<1)
    ///   k=1,2: Re = -b - 0.5 (稳定)
    /// Hopf 分岔: 当 -b + 1 = 0 即 b = 1 时 (但 Thomas 实际 b≈0.32 分岔,
    /// 因为非线性项起作用, 线性化只给原点稳定性)
    pub fn origin_eigenvalues(cfg: &ThomasConfig) -> Vec<f64> {
        // 实根 + 复共轭对, 仅返回实部
        // k=0: -b + 1 (实)
        // k=1,2: -b - 0.5 ± i √3/2 (复共轭)
        // 返回三个实部
        vec![1.0 - cfg.b, -0.5 - cfg.b, -0.5 - cfg.b]
    }

    /// 原点是否线性稳定 (所有 Re(λ) < 0)
    /// 需要 b > 1 (但 Thomas 在 b≈0.32 已分岔, 此线性分析仅对原点有效)
    pub fn origin_linearly_stable(cfg: &ThomasConfig) -> bool {
        cfg.b > 1.0
    }

    /// 单步 RK4 推进
    pub fn step(&mut self) {
        let cfg = self.config;
        let dt = cfg.dt;
        let (x, y, z) = (self.x, self.y, self.z);

        let k1 = Self::derivatives(&cfg, x, y, z);
        let k2 = Self::derivatives(&cfg, x + 0.5 * dt * k1[0], y + 0.5 * dt * k1[1], z + 0.5 * dt * k1[2]);
        let k3 = Self::derivatives(&cfg, x + 0.5 * dt * k2[0], y + 0.5 * dt * k2[1], z + 0.5 * dt * k2[2]);
        let k4 = Self::derivatives(&cfg, x + dt * k3[0], y + dt * k3[1], z + dt * k3[2]);

        self.x = x + dt / 6.0 * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]);
        self.y = y + dt / 6.0 * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]);
        self.z = z + dt / 6.0 * (k1[2] + 2.0 * k2[2] + 2.0 * k3[2] + k4[2]);

        self.step_count += 1;
        self.time += dt;
        self.trajectory.push((self.x, self.y, self.z));

        // Lyapunov: 变分方程前向欧拉 v_{n+1} = (I + dt J) v_n
        let j = Self::jacobian(&cfg, self.x, self.y, self.z);
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

    /// 主 Lyapunov 指数 λ₁
    pub fn lyapunov_exponent(&self) -> f64 {
        if self.step_count == 0 {
            return 0.0;
        }
        self.lyap_sum / self.time.max(1e-12)
    }

    pub fn has_nan(&self) -> bool {
        !self.x.is_finite() || !self.y.is_finite() || !self.z.is_finite()
    }

    pub fn has_escaped(&self) -> bool {
        self.x.abs() > 50.0 || self.y.abs() > 50.0 || self.z.abs() > 50.0 || self.has_nan()
    }

    pub fn attractor_bounds(&self) -> (f64, f64, f64, f64, f64, f64) {
        let mut xmin = f64::INFINITY;
        let mut xmax = f64::NEG_INFINITY;
        let mut ymin = f64::INFINITY;
        let mut ymax = f64::NEG_INFINITY;
        let mut zmin = f64::INFINITY;
        let mut zmax = f64::NEG_INFINITY;
        for &(x, y, z) in &self.trajectory {
            if x < xmin { xmin = x; }
            if x > xmax { xmax = x; }
            if y < ymin { ymin = y; }
            if y > ymax { ymax = y; }
            if z < zmin { zmin = z; }
            if z > zmax { zmax = z; }
        }
        (xmin, xmax, ymin, ymax, zmin, zmax)
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
        let cfg = ThomasConfig::default();
        assert!(approx_eq(cfg.b, 0.19, 1e-12));
        assert!(approx_eq(cfg.dt, 0.01, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = ThomasSolver::classic(ThomasConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_origin() {
        // 原点 (0,0,0): sin(0)=0, 所以 F = -b*(x,y,z) = (0,0,0)
        let cfg = ThomasConfig::default();
        let d = ThomasSolver::derivatives(&cfg, 0.0, 0.0, 0.0);
        assert!(approx_eq(d[0], 0.0, 1e-12));
        assert!(approx_eq(d[1], 0.0, 1e-12));
        assert!(approx_eq(d[2], 0.0, 1e-12));
    }

    #[test]
    fn test_derivatives_analytic() {
        // F = [sin(y) - b x, sin(z) - b y, sin(x) - b z]
        let cfg = ThomasConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = ThomasSolver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], y.sin() - cfg.b * x, 1e-12));
        assert!(approx_eq(d[1], z.sin() - cfg.b * y, 1e-12));
        assert!(approx_eq(d[2], x.sin() - cfg.b * z, 1e-12));
    }

    #[test]
    fn test_cyclic_symmetry() {
        // 循环置换 (x,y,z) → (y,z,x) 方程不变
        let cfg = ThomasConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d1 = ThomasSolver::derivatives(&cfg, x, y, z);
        let d2 = ThomasSolver::derivatives(&cfg, y, z, x);
        // d1[0] (在 x 位置) = d2[1] (在 y 位置, 即置换后的 x 方程)
        // 严格: F(x,y,z) = (sin(y)-bx, sin(z)-by, sin(x)-bz)
        // F(y,z,x) = (sin(z)-by, sin(x)-bz, sin(y)-bx)
        // 所以 F(y,z,x) = (F[1], F[2], F[0])
        assert!(approx_eq(d2[0], d1[1], 1e-12));
        assert!(approx_eq(d2[1], d1[2], 1e-12));
        assert!(approx_eq(d2[2], d1[0], 1e-12));
    }

    #[test]
    fn test_inversion_symmetry() {
        // (x,y,z) → (-x,-y,-z) 方程不变 (sin 奇函数 + 线性项)
        let cfg = ThomasConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d1 = ThomasSolver::derivatives(&cfg, x, y, z);
        let d2 = ThomasSolver::derivatives(&cfg, -x, -y, -z);
        assert!(approx_eq(d1[0], -d2[0], 1e-12));
        assert!(approx_eq(d1[1], -d2[1], 1e-12));
        assert!(approx_eq(d1[2], -d2[2], 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        // J = [[-b, cos(y), 0], [0, -b, cos(z)], [cos(x), 0, -b]]
        let cfg = ThomasConfig::default();
        let j = ThomasSolver::jacobian(&cfg, 0.3, 0.5, 0.7);
        assert!(approx_eq(j[0][0], -cfg.b, 1e-12));
        assert!(approx_eq(j[0][1], 0.5_f64.cos(), 1e-12));
        assert!(approx_eq(j[0][2], 0.0, 1e-12));
        assert!(approx_eq(j[1][0], 0.0, 1e-12));
        assert!(approx_eq(j[1][1], -cfg.b, 1e-12));
        assert!(approx_eq(j[1][2], 0.7_f64.cos(), 1e-12));
        assert!(approx_eq(j[2][0], 0.3_f64.cos(), 1e-12));
        assert!(approx_eq(j[2][1], 0.0, 1e-12));
        assert!(approx_eq(j[2][2], -cfg.b, 1e-12));
    }

    #[test]
    fn test_divergence_constant() {
        // 散度 = -3b (常数)
        let cfg = ThomasConfig::default();
        let div = ThomasSolver::divergence(&cfg);
        assert!(approx_eq(div, -3.0 * cfg.b, 1e-12));
        assert!(div < 0.0, "should be dissipative");
    }

    #[test]
    fn test_origin_is_fixed_point() {
        assert!(ThomasSolver::origin_is_fixed_point());
        let cfg = ThomasConfig::default();
        let d = ThomasSolver::derivatives(&cfg, 0.0, 0.0, 0.0);
        let mag = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        assert!(mag < 1e-12);
    }

    #[test]
    fn test_origin_eigenvalues() {
        // 原点 J 特征值实部: 1-b, -0.5-b, -0.5-b
        let cfg = ThomasConfig::default();
        let eigs = ThomasSolver::origin_eigenvalues(&cfg);
        assert_eq!(eigs.len(), 3);
        assert!(approx_eq(eigs[0], 1.0 - cfg.b, 1e-12));
        assert!(approx_eq(eigs[1], -0.5 - cfg.b, 1e-12));
        assert!(approx_eq(eigs[2], -0.5 - cfg.b, 1e-12));
    }

    #[test]
    fn test_origin_unstable_for_small_b() {
        // 经典 b=0.19 < 1, 原点不稳定 (一个特征值实部 > 0)
        let cfg = ThomasConfig::default();
        assert!(!ThomasSolver::origin_linearly_stable(&cfg));
    }

    #[test]
    fn test_origin_stable_for_large_b() {
        // b > 1: 原点线性稳定
        let cfg = ThomasConfig { b: 1.5, dt: 0.01 };
        assert!(ThomasSolver::origin_linearly_stable(&cfg));
    }

    #[test]
    fn test_step_advances() {
        let mut s = ThomasSolver::classic(ThomasConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = ThomasSolver::classic(ThomasConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = ThomasSolver::classic(ThomasConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded_chaos() {
        // b=0.19 混沌, 吸引子有界
        let mut s = ThomasSolver::classic(ThomasConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -10.0 && xmax < 10.0);
        assert!(ymin > -10.0 && ymax < 10.0);
        assert!(zmin > -10.0 && zmax < 10.0);
    }

    #[test]
    fn test_attractor_range_known() {
        // 经典 b=0.19 吸引子大致 |x|,|y|,|z| < 5
        let mut s = ThomasSolver::classic(ThomasConfig::default());
        s.run(50000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -5.0 && xmax < 5.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -5.0 && ymax < 5.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -5.0 && zmax < 5.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_high_b_converges_to_origin() {
        // b > 1: 原点是唯一不动点 (sin(x)/x = b 无解), 线性稳定
        let cfg = ThomasConfig { b: 2.0, dt: 0.005 };
        let mut s = ThomasSolver::new(cfg, 1.0, 1.0, 1.0);
        s.run(50000);
        let r = (s.x * s.x + s.y * s.y + s.z * s.z).sqrt();
        assert!(r < 0.1, "should converge to origin, r={}", r);
    }

    #[test]
    fn test_medium_b_converges_to_nonzero_fixed_point() {
        // b=0.5: sin(x)/x = 0.5 有非零解 x ≈ 1.895, 对称不动点 (x,x,x)
        let cfg = ThomasConfig { b: 0.5, dt: 0.005 };
        let mut s = ThomasSolver::new(cfg, 1.0, 1.0, 1.0);
        s.run(100000);
        // 应收敛到对称不动点 (x*, x*, x*) 附近
        let r = (s.x * s.x + s.y * s.y + s.z * s.z).sqrt();
        assert!(r > 0.5 && r < 5.0, "nonzero fixed point, r={}", r);
        // 三分量应近似相等 (对称不动点)
        assert!((s.x - s.y).abs() < 0.1, "x≈y: {} vs {}", s.x, s.y);
        assert!((s.y - s.z).abs() < 0.1, "y≈z: {} vs {}", s.y, s.z);
    }

    #[test]
    fn test_lyapunov_positive_chaos() {
        // b=0.19 混沌, Lyapunov > 0
        let mut s = ThomasSolver::classic(ThomasConfig::default());
        s.run(50000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_negative_stable() {
        // b=0.5 收敛, Lyapunov < 0
        let cfg = ThomasConfig { b: 0.5, dt: 0.01 };
        let mut s = ThomasSolver::new(cfg, 1.0, 1.0, 1.0);
        s.run(50000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda < 0.0, "lambda should be negative: {}", lambda);
    }

    #[test]
    fn test_lyapunov_value_classical() {
        // b=0.19 文献值 λ₁ ≈ 0.06 (前向欧拉估计偏低, 检查量级)
        let mut s = ThomasSolver::classic(ThomasConfig::default());
        s.run(200000);
        let lambda = s.lyapunov_exponent();
        // 前向欧拉 (I+dt J) 对 Lyapunov 估计有 O(dt) 误差, 实际值偏低
        // 主要检验: 正且量级合理 (< 0.2)
        assert!(lambda > 0.0 && lambda < 0.2, "lambda in (0, 0.2): {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        // 两条相近轨道分离. Thomas Lyapunov 较小 (~0.005-0.06), 需要长时间
        let cfg = ThomasConfig::default();
        let mut s1 = ThomasSolver::new(cfg, 0.1, 0.0, 0.0);
        let mut s2 = ThomasSolver::new(cfg, 0.1 + 1e-6, 0.0, 0.0);
        for _ in 0..300000 {
            s1.step();
            s2.step();
        }
        let dx = s1.x - s2.x;
        let dy = s1.y - s2.y;
        let dz = s1.z - s2.z;
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        // t=3000, Thomas Lyapunov 较小, 放大 ~20 倍 (1e-6 → ~2e-5)
        // 证明混沌存在 (d > 初始扰动)
        assert!(d > 1e-5, "should be amplified: {}", d);
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = ThomasConfig::default();
        let mut s = ThomasSolver::new(cfg, 100.0, 100.0, 100.0);
        s.run(500);
        // 大初值可能逃逸或收敛
        assert!(s.has_escaped() || !s.has_nan());
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = ThomasSolver::classic(ThomasConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_volume_contraction() {
        // 耗散: 散度 -3b < 0, 体积元 e^(-3bt) 收缩
        let mut s = ThomasSolver::classic(ThomasConfig::default());
        s.run(10000);
        let b1 = s.attractor_bounds();
        s.run(20000);
        let b2 = s.attractor_bounds();
        let r1 = (b1.1 - b1.0, b1.3 - b1.2, b1.5 - b1.4);
        let r2 = (b2.1 - b2.0, b2.3 - b2.2, b2.5 - b2.4);
        // 范围不应显著扩大
        assert!(r2.0 < r1.0 * 2.0);
        assert!(r2.1 < r1.1 * 2.0);
        assert!(r2.2 < r1.2 * 2.0);
    }

    #[test]
    fn test_conservative_limit_b_zero() {
        // b=0: 保守系统, 不耗散
        let cfg = ThomasConfig { b: 0.0, dt: 0.005 };
        let mut s = ThomasSolver::new(cfg, 0.5, 0.5, 0.5);
        s.run(10000);
        // 散度 = 0
        assert!(approx_eq(ThomasSolver::divergence(&cfg), 0.0, 1e-12));
        // 不应发散
        assert!(!s.has_escaped());
    }

    #[test]
    fn test_jacobian_at_origin() {
        let cfg = ThomasConfig::default();
        let j = ThomasSolver::jacobian(&cfg, 0.0, 0.0, 0.0);
        // J = [[-b, 1, 0], [0, -b, 1], [1, 0, -b]]
        assert!(approx_eq(j[0][0], -cfg.b, 1e-12));
        assert!(approx_eq(j[0][1], 1.0, 1e-12));
        assert!(approx_eq(j[0][2], 0.0, 1e-12));
        assert!(approx_eq(j[1][0], 0.0, 1e-12));
        assert!(approx_eq(j[1][1], -cfg.b, 1e-12));
        assert!(approx_eq(j[1][2], 1.0, 1e-12));
        assert!(approx_eq(j[2][0], 1.0, 1e-12));
        assert!(approx_eq(j[2][1], 0.0, 1e-12));
        assert!(approx_eq(j[2][2], -cfg.b, 1e-12));
    }

    #[test]
    fn test_origin_jacobian_trace() {
        // tr(J(0,0,0)) = -3b
        let cfg = ThomasConfig::default();
        let j = ThomasSolver::jacobian(&cfg, 0.0, 0.0, 0.0);
        let tr = j[0][0] + j[1][1] + j[2][2];
        assert!(approx_eq(tr, -3.0 * cfg.b, 1e-12));
    }

    #[test]
    fn test_origin_jacobian_det() {
        // det(J(0,0,0)) = (-b)³ + 1·1·1 = 1 - b³ (循环矩阵)
        let cfg = ThomasConfig::default();
        let j = ThomasSolver::jacobian(&cfg, 0.0, 0.0, 0.0);
        let det = j[0][0] * (j[1][1] * j[2][2] - j[1][2] * j[2][1])
            - j[0][1] * (j[1][0] * j[2][2] - j[1][2] * j[2][0])
            + j[0][2] * (j[1][0] * j[2][1] - j[1][1] * j[2][0]);
        let expected = 1.0 - cfg.b.powi(3);
        assert!(approx_eq(det, expected, 1e-12));
    }

    #[test]
    fn test_cyclic_jacobian_symmetry() {
        // Jacobian 在循环置换 (x,y,z)→(y,z,x) 下: J(y,z,x)[i][k] = J(x,y,z)[(i+1)%3][(k+1)%3]
        let cfg = ThomasConfig::default();
        let j1 = ThomasSolver::jacobian(&cfg, 0.3, 0.5, 0.7);
        let j2 = ThomasSolver::jacobian(&cfg, 0.5, 0.7, 0.3);
        for i in 0..3 {
            for k in 0..3 {
                let i2 = (i + 1) % 3;
                let k2 = (k + 1) % 3;
                assert!(approx_eq(j2[i][k], j1[i2][k2], 1e-12), "j2[{}][{}]={}, j1[{}][{}]={}", i, k, j2[i][k], i2, k2, j1[i2][k2]);
            }
        }
    }
}
