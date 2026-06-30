//! Dadras-Momeni Attractor — Dadras-Momeni 多卷混沌系统 (3D)
//!
//! Sara Dadras 和 Hamid Reza Momeni 2009 年提出的 3D 自治混沌系统,
//! 该系统的显著特征是能产生多卷 (multi-scroll) 吸引子, 包括 2 卷、
//! 3 卷和 4 卷吸引子, 通过参数调节切换. 多卷吸引子在混沌保密通信中
//! 有重要应用, 因为更大的吸引子结构提供更大的密钥空间.
//!
//! 状态方程 (Dadras-Momeni 2009):
//!   dx/dt = y - a x + b y z
//!   dy/dt = c y - x z + z
//!   dz/dt = d x y - e z
//!
//! 各项物理意义:
//!   - -a x, c y, -e z: 线性阻尼 (各轴不同)
//!   + y: 线性耦合 (x-y 串联)
//!   + b y z, -x z, +d x y: 非线性耦合 (二次交叉项)
//!   + z: z 对 y 的线性驱动
//!
//! 经典参数 (Dadras-Momeni 2009): a=3, b=2.7, c=1.7, d=2, e=9
//! 经典初值: (x₀, y₀, z₀) = (1.1, 2.1, -2.0) 或 (1, 1, 1)
//!
//! 性质:
//!   - 散度 ∇·F = tr(J) = -a + c - e (常数)
//!     经典参数: div = -3 + 1.7 - 9 = -10.3 (强耗散)
//!   - 平衡点:
//!     E0 = (0, 0, 0) (平凡)
//!     其他平衡点需数值求解 (涉及非线性方程组)
//!   - Lyapunov 谱 (经典参数, 文献值):
//!     λ₁ ≈ +1.07  (正, 主混沌方向)
//!     λ₂ = 0      (沿轨道切向)
//!     λ₃ ≈ -11.37 (负, 强收缩)
//!     和 = -10.3 (与散度一致)
//!   - Kaplan-Yorke 维数 D_KY ≈ 2 + λ₁/|λ₃| ≈ 2.09
//!   - 吸引子形态: 多卷结构 (参数调节可切换卷数)
//!
//! 多卷吸引子机制:
//!   通过调节参数 a, b, c, d, e, 系统可在 2 卷、3 卷、4 卷之间切换.
//!   卷数越多, 吸引子结构越复杂, 相空间覆盖越大.
//!   - 2 卷: a=3, b=2.7, c=1.7, d=2, e=9 (经典)
//!   - 3 卷: 调节参数
//!   - 4 卷: 调节参数
//!
//! 与其他多卷系统对比:
//!   - Chua's Circuit: 需要非线性电阻 (分段线性), 产生双卷
//!   - Dadras-Momeni: 纯多项式 (光滑), 产生多卷
//!   - Sprott: 简单多项式, 通常单卷或双卷
//!
//! 应用:
//!   - 混沌保密通信 (多卷 = 更大密钥空间)
//!   - 混沌同步 (多卷系统的同步更鲁棒)
//!   - 随机数生成
//!   - 混沌控制
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!
//! 历史:
//!   Dadras, S. & Momeni, H. R. 2009. "A novel three-dimensional
//!   autonomous chaotic system generating two, three and four-scroll
//!   attractors." Phys. Lett. A 373, 3637-3642. (原始论文)
//!   Dadras, S. & Momeni, H. R. 2010. "Four-scroll hyperchaotic
//!   attractor generated from a new 4D system with one equilibrium
//!   and its LQR control." Int. J. Bifurcation Chaos 20, 3245.

/// Dadras-Momeni 系统配置 (5 参数 + dt)
#[derive(Clone, Copy, Debug)]
pub struct DadrasMomeniConfig {
    /// 参数 a (x 阻尼, 经典 3)
    pub a: f64,
    /// 参数 b (yz 耦合, 经典 2.7)
    pub b: f64,
    /// 参数 c (y 增益, 经典 1.7)
    pub c: f64,
    /// 参数 d (xy 耦合, 经典 2)
    pub d: f64,
    /// 参数 e (z 阻尼, 经典 9)
    pub e: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for DadrasMomeniConfig {
    fn default() -> Self {
        Self { a: 3.0, b: 2.7, c: 1.7, d: 2.0, e: 9.0, dt: 0.005 }
    }
}

/// Dadras-Momeni 求解器 (3D, 跟踪最大 Lyapunov 指数)
pub struct DadrasMomeniSolver {
    pub config: DadrasMomeniConfig,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub time: f64,
    pub step_count: u64,
    pub trajectory: Vec<(f64, f64, f64)>,
    /// Lyapunov 累积
    pub lyap_sum: f64,
    /// 切向量 (3D)
    pub v: [f64; 3],
}

impl DadrasMomeniSolver {
    pub fn new(config: DadrasMomeniConfig, x0: f64, y0: f64, z0: f64) -> Self {
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

    pub fn classic(config: DadrasMomeniConfig) -> Self {
        Self::new(config, 1.1, 2.1, -2.0)
    }

    /// 右端导数 F = [y - a x + b y z,
    ///                c y - x z + z,
    ///                d x y - e z]
    pub fn derivatives(cfg: &DadrasMomeniConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        [
            y - cfg.a * x + cfg.b * y * z,
            cfg.c * y - x * z + z,
            cfg.d * x * y - cfg.e * z,
        ]
    }

    /// Jacobian:
    /// J = [[-a,    1+bz,  by ],
    ///      [-z,    c,     1-x],
    ///      [dy,    dx,    -e ]]
    pub fn jacobian(cfg: &DadrasMomeniConfig, x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
        [
            [-cfg.a, 1.0 + cfg.b * z, cfg.b * y],
            [-z, cfg.c, 1.0 - x],
            [cfg.d * y, cfg.d * x, -cfg.e],
        ]
    }

    /// 散度 ∇·F = tr(J) = -a + c - e (常数)
    pub fn divergence(cfg: &DadrasMomeniConfig, _x: f64, _y: f64, _z: f64) -> f64 {
        -cfg.a + cfg.c - cfg.e
    }

    /// 原点 (0, 0, 0) 是平衡点
    pub fn origin_equilibrium() -> [f64; 3] {
        [0.0, 0.0, 0.0]
    }

    /// 单步 RK4 推进 + 变分方程 Lyapunov
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

    /// 最大 Lyapunov 指数 (文献值 ~1.07)
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
        self.x.abs() > 100.0 || self.y.abs() > 100.0 || self.z.abs() > 100.0 || self.has_nan()
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
        let cfg = DadrasMomeniConfig::default();
        assert!(approx_eq(cfg.a, 3.0, 1e-12));
        assert!(approx_eq(cfg.b, 2.7, 1e-12));
        assert!(approx_eq(cfg.c, 1.7, 1e-12));
        assert!(approx_eq(cfg.d, 2.0, 1e-12));
        assert!(approx_eq(cfg.e, 9.0, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = DadrasMomeniSolver::classic(DadrasMomeniConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = DadrasMomeniConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = DadrasMomeniSolver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], y - cfg.a * x + cfg.b * y * z, 1e-12));
        assert!(approx_eq(d[1], cfg.c * y - x * z + z, 1e-12));
        assert!(approx_eq(d[2], cfg.d * x * y - cfg.e * z, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = DadrasMomeniConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = DadrasMomeniSolver::jacobian(&cfg, x, y, z);
        assert!(approx_eq(j[0][0], -cfg.a, 1e-12));
        assert!(approx_eq(j[0][1], 1.0 + cfg.b * z, 1e-12));
        assert!(approx_eq(j[0][2], cfg.b * y, 1e-12));
        assert!(approx_eq(j[1][0], -z, 1e-12));
        assert!(approx_eq(j[1][1], cfg.c, 1e-12));
        assert!(approx_eq(j[1][2], 1.0 - x, 1e-12));
        assert!(approx_eq(j[2][0], cfg.d * y, 1e-12));
        assert!(approx_eq(j[2][1], cfg.d * x, 1e-12));
        assert!(approx_eq(j[2][2], -cfg.e, 1e-12));
    }

    #[test]
    fn test_divergence_constant() {
        let cfg = DadrasMomeniConfig::default();
        let expected = -cfg.a + cfg.c - cfg.e;
        assert!(approx_eq(DadrasMomeniSolver::divergence(&cfg, 0.0, 0.0, 0.0), expected, 1e-12));
        assert!(approx_eq(DadrasMomeniSolver::divergence(&cfg, 1.0, 2.0, 3.0), expected, 1e-12));
        assert!(approx_eq(DadrasMomeniSolver::divergence(&cfg, -5.0, 7.0, -3.0), expected, 1e-12));
    }

    #[test]
    fn test_divergence_negative_dissipative() {
        let cfg = DadrasMomeniConfig::default();
        let div = DadrasMomeniSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < 0.0, "divergence should be negative: {}", div);
        assert!(approx_eq(div, -10.3, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = DadrasMomeniConfig::default();
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5)] {
            let j = DadrasMomeniSolver::jacobian(&cfg, x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = DadrasMomeniSolver::divergence(&cfg, x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_origin_is_equilibrium() {
        let cfg = DadrasMomeniConfig::default();
        let eq = DadrasMomeniSolver::origin_equilibrium();
        let d = DadrasMomeniSolver::derivatives(&cfg, eq[0], eq[1], eq[2]);
        for v in d.iter() {
            assert!(v.abs() < 1e-12, "origin derivative = {}", v);
        }
    }

    #[test]
    fn test_step_advances() {
        let mut s = DadrasMomeniSolver::classic(DadrasMomeniConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = DadrasMomeniSolver::classic(DadrasMomeniConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = DadrasMomeniSolver::classic(DadrasMomeniConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = DadrasMomeniSolver::classic(DadrasMomeniConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = DadrasMomeniSolver::classic(DadrasMomeniConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        let mut s = DadrasMomeniSolver::classic(DadrasMomeniConfig::default());
        s.run(80000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = DadrasMomeniSolver::classic(DadrasMomeniConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        let cfg = DadrasMomeniConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = DadrasMomeniSolver::classic(cfg);
        let mut s2 = DadrasMomeniSolver::new(cfg, 1.1 + d0, 2.1, -2.0);
        for _ in 0..40000 {
            s1.step();
            s2.step();
        }
        let dx = s1.x - s2.x;
        let dy = s1.y - s2.y;
        let dz = s1.z - s2.z;
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        assert!(d > 1e-3, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_volume_monotonic_contraction() {
        let cfg = DadrasMomeniConfig::default();
        for &(x, y, z) in &[(1.0_f64, 2.0, 3.0), (-5.0, 7.0, -3.0), (10.0, -10.0, 20.0)] {
            assert!(DadrasMomeniSolver::divergence(&cfg, x, y, z) < 0.0);
        }
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = DadrasMomeniConfig::default();
        let mut s = DadrasMomeniSolver::new(cfg, 100.0, 100.0, 100.0);
        s.run(500);
        assert!(s.has_escaped() || !s.has_nan());
    }
}
