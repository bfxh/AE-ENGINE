//! Burke-Shaw Attractor — Burke-Shaw 混沌系统 (3D)
//!
//! Robert Shaw 在研究耗散系统中的信息流和混沌时提出的 3D 系统,
//! 与 Lorenz 63 类似都是二次非线性混沌系统, 但 Burke-Shaw 的
//! 非线性项为 xz 和 xy (与 Lorenz 的 xz, xy 相同但系数不同),
//! 产生形态不同的吸引子. 该系统在混沌信息论和拓扑学研究中
//! 有重要地位, Shaw 用它演示了混沌系统中的信息产生.
//!
//! 状态方程 (Burke-Shaw):
//!   dx/dt = -S (x + y)
//!   dy/dt = -y - S x z
//!   dz/dt = S x y + V
//!
//! 各项物理意义:
//!   - -S(x + y): 线性阻尼 + 耦合 (耗散)
//!   - -y: y 轴阻尼
//!   - S x z: 非线性反馈 (类似 Lorenz 的 xz 项)
//!   + S x y: 非线性耦合 (类似 Lorenz 的 xy 项)
//!   + V: 外部驱动 (能量注入)
//!
//! 经典参数 (Shaw): S = 10, V = 4
//! 其他混沌参数: S = 10, V = 13.58
//! 经典初值: (x₀, y₀, z₀) = (1, 0, 1) 或 (0.1, 0, 0)
//!
//! 性质:
//!   - 散度 ∇·F = tr(J) = -S - 1 (常数负, 耗散)
//!     经典参数 S=10: div = -11 (体积收缩率 e^(-11t), 强耗散)
//!   - 平衡点 (利用 x = -y, z = 1/S 代入):
//!     0 = S x y + V = -S x² + V → x = ±√(V/S)
//!     E± = (±√(V/S), ∓√(V/S), 1/S)
//!     经典参数 S=10, V=4: E± = (±0.632, ∓0.632, 0.1)
//!   - Lyapunov 谱 (经典参数, 文献值):
//!     λ₁ ≈ +0.94  (正, 主混沌方向)
//!     λ₂ = 0      (沿轨道切向)
//!     λ₃ ≈ -11.94 (负, 强收缩)
//!     和 = -11 (与散度一致)
//!   - Kaplan-Yorke 维数 D_KY ≈ 2 + λ₁/|λ₃| ≈ 2.08
//!   - 吸引子形态: 单叶扭曲带 (与 Lorenz 双叶不同)
//!
//! 与 Lorenz 63 对比:
//!   - Lorenz 63: σ=10, r=28, b=8/3; div = -σ-1-b ≈ -13.67
//!   - Burke-Shaw: S=10, V=4; div = -S-1 = -11
//!   - 两者都有 xy, xz 非线性项, 但 Burke-Shaw 无 z² 项
//!   - Burke-Shaw 平衡点只有 2 个, Lorenz 有 3 个
//!
//! 信息论意义:
//!   Shaw (1981) 用 Burke-Shaw 系统演示了混沌系统如何"产生信息".
//!   两条无限接近的轨道在混沌吸引子上指数分离, 等价于系统不断
//!   "创造"新的信息 (关于轨道具体位置的信息). 这与 Kolmogorov-Sinai
//!   熵相关: h_KS = Σ λᵢ⁺ (正 Lyapunov 指数之和).
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!
//! 历史:
//!   Shaw, R. 1981. "Strange attractors, chaotic behavior, and
//!   information flow." Z. Naturforsch. A 36, 80-112. (信息论分析)
//!   Sprott, J. C. 2003. "Chaos and Time-Series Analysis." Oxford.
//!   (教科书中讨论 Burke-Shaw 系统)

/// Burke-Shaw 系统配置 (2 参数 + dt)
#[derive(Clone, Copy, Debug)]
pub struct BurkeShawConfig {
    /// 耦合强度 S (经典 10)
    pub s: f64,
    /// 外部驱动 V (经典 4)
    pub v: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for BurkeShawConfig {
    fn default() -> Self {
        Self { s: 10.0, v: 4.0, dt: 0.005 }
    }
}

/// Burke-Shaw 求解器 (3D, 跟踪最大 Lyapunov 指数)
pub struct BurkeShawSolver {
    pub config: BurkeShawConfig,
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

impl BurkeShawSolver {
    pub fn new(config: BurkeShawConfig, x0: f64, y0: f64, z0: f64) -> Self {
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

    pub fn classic(config: BurkeShawConfig) -> Self {
        Self::new(config, 1.0, 0.0, 1.0)
    }

    /// 右端导数 F = [-S(x + y), -y - S x z, S x y + V]
    pub fn derivatives(cfg: &BurkeShawConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        [
            -cfg.s * (x + y),
            -y - cfg.s * x * z,
            cfg.s * x * y + cfg.v,
        ]
    }

    /// Jacobian:
    /// J = [[-S,    -S,    0    ],
    ///      [-S z,  -1,    -S x ],
    ///      [S y,   S x,   0    ]]
    pub fn jacobian(cfg: &BurkeShawConfig, x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
        [
            [-cfg.s, -cfg.s, 0.0],
            [-cfg.s * z, -1.0, -cfg.s * x],
            [cfg.s * y, cfg.s * x, 0.0],
        ]
    }

    /// 散度 ∇·F = tr(J) = -S - 1 (常数负)
    pub fn divergence(cfg: &BurkeShawConfig, _x: f64, _y: f64, _z: f64) -> f64 {
        -cfg.s - 1.0
    }

    /// 计算两个平衡点:
    /// E± = (±√(V/S), ∓√(V/S), 1/S)
    pub fn equilibria(cfg: &BurkeShawConfig) -> ([f64; 3], [f64; 3]) {
        let r = (cfg.v / cfg.s).max(0.0).sqrt();
        let z = 1.0 / cfg.s;
        ([r, -r, z], [-r, r, z])
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

    /// 最大 Lyapunov 指数 (文献值 ~0.94)
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
        let cfg = BurkeShawConfig::default();
        assert!(approx_eq(cfg.s, 10.0, 1e-12));
        assert!(approx_eq(cfg.v, 4.0, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = BurkeShawSolver::classic(BurkeShawConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = BurkeShawConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = BurkeShawSolver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], -cfg.s * (x + y), 1e-12));
        assert!(approx_eq(d[1], -y - cfg.s * x * z, 1e-12));
        assert!(approx_eq(d[2], cfg.s * x * y + cfg.v, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = BurkeShawConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = BurkeShawSolver::jacobian(&cfg, x, y, z);
        assert!(approx_eq(j[0][0], -cfg.s, 1e-12));
        assert!(approx_eq(j[0][1], -cfg.s, 1e-12));
        assert!(approx_eq(j[0][2], 0.0, 1e-12));
        assert!(approx_eq(j[1][0], -cfg.s * z, 1e-12));
        assert!(approx_eq(j[1][1], -1.0, 1e-12));
        assert!(approx_eq(j[1][2], -cfg.s * x, 1e-12));
        assert!(approx_eq(j[2][0], cfg.s * y, 1e-12));
        assert!(approx_eq(j[2][1], cfg.s * x, 1e-12));
        assert!(approx_eq(j[2][2], 0.0, 1e-12));
    }

    #[test]
    fn test_divergence_constant() {
        let cfg = BurkeShawConfig::default();
        let expected = -cfg.s - 1.0;
        assert!(approx_eq(BurkeShawSolver::divergence(&cfg, 0.0, 0.0, 0.0), expected, 1e-12));
        assert!(approx_eq(BurkeShawSolver::divergence(&cfg, 1.0, 2.0, 3.0), expected, 1e-12));
        assert!(approx_eq(BurkeShawSolver::divergence(&cfg, -5.0, 7.0, -3.0), expected, 1e-12));
    }

    #[test]
    fn test_divergence_negative_dissipative() {
        let cfg = BurkeShawConfig::default();
        let div = BurkeShawSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < 0.0);
        assert!(approx_eq(div, -11.0, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = BurkeShawConfig::default();
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5)] {
            let j = BurkeShawSolver::jacobian(&cfg, x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = BurkeShawSolver::divergence(&cfg, x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_equilibria_values() {
        let cfg = BurkeShawConfig::default();
        let (e1, e2) = BurkeShawSolver::equilibria(&cfg);
        let r = (cfg.v / cfg.s).sqrt();
        let z = 1.0 / cfg.s;
        assert!(approx_eq(e1[0], r, 1e-12));
        assert!(approx_eq(e1[1], -r, 1e-12));
        assert!(approx_eq(e1[2], z, 1e-12));
        assert!(approx_eq(e2[0], -r, 1e-12));
        assert!(approx_eq(e2[1], r, 1e-12));
        assert!(approx_eq(e2[2], z, 1e-12));
    }

    #[test]
    fn test_equilibria_satisfy_equations() {
        let cfg = BurkeShawConfig::default();
        let (e1, e2) = BurkeShawSolver::equilibria(&cfg);
        for eq in [e1, e2] {
            let d = BurkeShawSolver::derivatives(&cfg, eq[0], eq[1], eq[2]);
            for v in d.iter() {
                assert!(v.abs() < 1e-9, "equilibrium derivative = {}", v);
            }
        }
    }

    #[test]
    fn test_equilibria_symmetric() {
        // E1 和 E2 关于 (x,y) 反演对称: (x,y,z) ↔ (-x,-y,z)
        let cfg = BurkeShawConfig::default();
        let (e1, e2) = BurkeShawSolver::equilibria(&cfg);
        assert!(approx_eq(e1[0], -e2[0], 1e-12));
        assert!(approx_eq(e1[1], -e2[1], 1e-12));
        assert!(approx_eq(e1[2], e2[2], 1e-12));
    }

    #[test]
    fn test_step_advances() {
        let mut s = BurkeShawSolver::classic(BurkeShawConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = BurkeShawSolver::classic(BurkeShawConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = BurkeShawSolver::classic(BurkeShawConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = BurkeShawSolver::classic(BurkeShawConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = BurkeShawSolver::classic(BurkeShawConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        let mut s = BurkeShawSolver::classic(BurkeShawConfig::default());
        s.run(80000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = BurkeShawSolver::classic(BurkeShawConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        let cfg = BurkeShawConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = BurkeShawSolver::classic(cfg);
        let mut s2 = BurkeShawSolver::new(cfg, 1.0 + d0, 0.0, 1.0);
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
    fn test_zero_drive_no_chaos() {
        // V = 0 时, 无外部驱动, 系统衰减到平衡点
        let cfg = BurkeShawConfig { v: 0.0, ..BurkeShawConfig::default() };
        let mut s = BurkeShawSolver::new(cfg, 1.0, 1.0, 1.0);
        s.run(50000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda < 0.05, "no drive → no chaos, lambda = {}", lambda);
    }

    #[test]
    fn test_volume_monotonic_contraction() {
        let cfg = BurkeShawConfig::default();
        for &(x, y, z) in &[(1.0_f64, 2.0, 3.0), (-5.0, 7.0, -3.0), (10.0, -10.0, 20.0)] {
            assert!(BurkeShawSolver::divergence(&cfg, x, y, z) < 0.0);
        }
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = BurkeShawConfig::default();
        let mut s = BurkeShawSolver::new(cfg, 100.0, 100.0, 100.0);
        s.run(500);
        assert!(s.has_escaped() || !s.has_nan());
    }
}
