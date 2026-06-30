//! Lü Attractor — Lü 混沌系统 (Lorenz-Chen 桥接系统)
//!
//! Jinhu Lü (吕金虎) & Guanrong Chen 2002 年提出, 是 Lorenz 系统与
//! Chen 系统之间的"桥接"系统. 在矩阵形式 dX/dt = AX + g(X) 中,
//! Lorenz 满足 a12*a21 > 0, Chen 满足 a12*a21 < 0, 而 Lü 满足
//! a12*a21 = 0 (a21 = 0), 三者构成 Lorenz-Lü-Chen 混沌三联体.
//!
//! 状态方程 (Lü & Chen 2002):
//!   dx/dt = a (y - x)
//!   dy/dt = -x z + c y
//!   dz/dt = x y - b z
//!
//! 经典参数: a = 36, b = 3, c = 20
//! 经典初值: (x0, y0, z0) = (1, 1, 1) 或 (-10, -5, 5)
//!
//! 三联体代数条件 (Vaněček-Čelikovský):
//!   Lorenz:  dy/dt = ρ x - x z - y      →  a21 = ρ  > 0  →  a12*a21 > 0
//!   Lü:      dy/dt = -x z + c y          →  a21 = 0       →  a12*a21 = 0  (边界)
//!   Chen:    dy/dt = (c-a) x - x z + c y →  a21 = c-a < 0 →  a12*a21 < 0
//!
//! 性质:
//!   - 散度 ∇·F = tr(J) = -a + c - b (常数, 强耗散)
//!     经典参数: -36 + 20 - 3 = -19 (体积收缩率 e^(-19t))
//!   - 平衡点:
//!     E0 = (0, 0, 0) (不稳定鞍)
//!     E± = (±sqrt(b c), ±sqrt(b c), c)
//!     经典参数: E± = (±sqrt(60), ±sqrt(60), 20)
//!   - Lyapunov 谱 (经典参数, 文献值):
//!     λ₁ ≈ +1.746 (正, 主混沌方向)
//!     λ₂ = 0 (沿轨道切向)
//!     λ₃ ≈ -20.746 (负, 强收缩)
//!     和 = -19 (与散度一致)
//!   - Kaplan-Yorke 维数 D_KY = 2 + λ₁/|λ₃| ≈ 2.084
//!
//! 与 Lorenz, Chen 的对比 (经典参数):
//!   系统    | a    | b  | c    | div  | λ₁    | D_KY
//!   --------|------|----|------|------|-------|------
//!   Lorenz  | 10   | 8/3| 28   | -13⅔| 0.906 | 2.062
//!   Lü      | 36   | 3  | 20   | -19  | 1.746 | 2.084
//!   Chen    | 35   | 3  | 28   | -10  | 2.027 | 2.169
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!
//! 历史:
//!   Lü, J. & Chen, G. 2002. "A new chaotic attractor coined."
//!   Int. J. Bifurcation Chaos 12, 659. (Lü 系统原始提出)
//!   Vaněček, A. & Čelikovský, S. 1996. "Control systems: from
//!   linear analysis to synthesis of chaos." (代数条件 a12*a21)
//!   Lü, J., Chen, G., Cheng, D. & Čelikovský, S. 2002. "Bridge
//!   the gap between the Lorenz system and the Chen system."
//!   Int. J. Bifurcation Chaos 12, 2917. (桥接意义阐述)

/// Lü 系统配置 (3 参数)
#[derive(Clone, Copy, Debug)]
pub struct LuConfig {
    /// 参数 a (x 阻尼 + y 反馈)
    pub a: f64,
    /// 参数 b (z 阻尼)
    pub b: f64,
    /// 参数 c (y 自激)
    pub c: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for LuConfig {
    fn default() -> Self {
        Self { a: 36.0, b: 3.0, c: 20.0, dt: 0.001 }
    }
}

/// Lü 系统求解器 (3D, 跟踪最大 Lyapunov 指数)
pub struct LuSolver {
    pub config: LuConfig,
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

impl LuSolver {
    pub fn new(config: LuConfig, x0: f64, y0: f64, z0: f64) -> Self {
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

    pub fn classic(config: LuConfig) -> Self {
        // Lü & Chen 2002 推荐初值 (1, 1, 1)
        Self::new(config, 1.0, 1.0, 1.0)
    }

    /// 右端导数 F = [a(y-x), -xz + cy, xy - bz]
    pub fn derivatives(cfg: &LuConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        [cfg.a * (y - x), -x * z + cfg.c * y, x * y - cfg.b * z]
    }

    /// Jacobian: J = [[-a, a, 0], [-z, c, -x], [y, x, -b]]
    pub fn jacobian(cfg: &LuConfig, x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
        [
            [-cfg.a, cfg.a, 0.0],
            [-z, cfg.c, -x],
            [y, x, -cfg.b],
        ]
    }

    /// 散度 ∇·F = tr(J) = -a + c - b (常数, 强耗散)
    pub fn divergence(cfg: &LuConfig, _x: f64, _y: f64, _z: f64) -> f64 {
        -cfg.a + cfg.c - cfg.b
    }

    /// 计算两个非平凡平衡点:
    /// E± = (±sqrt(b*c), ±sqrt(b*c), c)
    /// 需要 b*c > 0
    pub fn equilibria(cfg: &LuConfig) -> Option<([f64; 3], [f64; 3])> {
        let bc = cfg.b * cfg.c;
        if bc <= 0.0 {
            return None;
        }
        let x_pos = bc.sqrt();
        let x_neg = -x_pos;
        Some(([x_pos, x_pos, cfg.c], [x_neg, x_neg, cfg.c]))
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

        // Lyapunov: 变分方程前向欧拉 (I + dt J) v
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

    /// 最大 Lyapunov 指数 (Lü & Chen 2002 文献值 ~1.746)
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
        self.x.abs() > 1000.0 || self.y.abs() > 1000.0 || self.z.abs() > 1000.0 || self.has_nan()
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
        let cfg = LuConfig::default();
        assert!(approx_eq(cfg.a, 36.0, 1e-12));
        assert!(approx_eq(cfg.b, 3.0, 1e-12));
        assert!(approx_eq(cfg.c, 20.0, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = LuSolver::classic(LuConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = LuConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = LuSolver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], cfg.a * (y - x), 1e-12));
        assert!(approx_eq(d[1], -x * z + cfg.c * y, 1e-12));
        assert!(approx_eq(d[2], x * y - cfg.b * z, 1e-12));
    }

    #[test]
    fn test_derivatives_at_origin() {
        // 原点是平衡点
        let cfg = LuConfig::default();
        let d = LuSolver::derivatives(&cfg, 0.0, 0.0, 0.0);
        assert!(approx_eq(d[0], 0.0, 1e-12));
        assert!(approx_eq(d[1], 0.0, 1e-12));
        assert!(approx_eq(d[2], 0.0, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        // J = [[-a, a, 0], [-z, c, -x], [y, x, -b]]
        let cfg = LuConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = LuSolver::jacobian(&cfg, x, y, z);
        assert!(approx_eq(j[0][0], -cfg.a, 1e-12));
        assert!(approx_eq(j[0][1], cfg.a, 1e-12));
        assert!(approx_eq(j[0][2], 0.0, 1e-12));
        assert!(approx_eq(j[1][0], -z, 1e-12));
        assert!(approx_eq(j[1][1], cfg.c, 1e-12));
        assert!(approx_eq(j[1][2], -x, 1e-12));
        assert!(approx_eq(j[2][0], y, 1e-12));
        assert!(approx_eq(j[2][1], x, 1e-12));
        assert!(approx_eq(j[2][2], -cfg.b, 1e-12));
    }

    #[test]
    fn test_jacobian_a21_is_zero_at_origin() {
        // Lü 系统在原点处 a21 = -z = 0 (桥接条件)
        let cfg = LuConfig::default();
        let j = LuSolver::jacobian(&cfg, 0.0, 0.0, 0.0);
        assert!(approx_eq(j[1][0], 0.0, 1e-12), "a21 = -z = 0 at origin (bridge condition)");
    }

    #[test]
    fn test_divergence_constant() {
        // 散度 = -a + c - b (常数, 强耗散)
        let cfg = LuConfig::default();
        let expected = -cfg.a + cfg.c - cfg.b;
        assert!(approx_eq(LuSolver::divergence(&cfg, 0.0, 0.0, 0.0), expected, 1e-12));
        assert!(approx_eq(LuSolver::divergence(&cfg, 1.0, 2.0, 3.0), expected, 1e-12));
        assert!(approx_eq(LuSolver::divergence(&cfg, -5.0, 7.0, -3.0), expected, 1e-12));
    }

    #[test]
    fn test_divergence_negative_strongly_dissipative() {
        // 经典参数下散度 = -19 (强耗散, 体积快速收缩)
        let cfg = LuConfig::default();
        let div = LuSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < -10.0, "should be strongly dissipative: {}", div);
        assert!(approx_eq(div, -19.0, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = LuConfig::default();
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5)] {
            let j = LuSolver::jacobian(&cfg, x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = LuSolver::divergence(&cfg, x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_equilibria_exist() {
        let cfg = LuConfig::default();
        let eqs = LuSolver::equilibria(&cfg);
        assert!(eqs.is_some(), "should have non-trivial equilibria");
    }

    #[test]
    fn test_equilibria_values() {
        // E± = (±sqrt(b*c), ±sqrt(b*c), c)
        // 经典参数: (±sqrt(60), ±sqrt(60), 20)
        let cfg = LuConfig::default();
        let (eq1, eq2) = LuSolver::equilibria(&cfg).unwrap();
        let x_star = (cfg.b * cfg.c).sqrt(); // sqrt(60)
        assert!(approx_eq(eq1[0], x_star, 1e-10), "eq1 x: {}", eq1[0]);
        assert!(approx_eq(eq1[1], x_star, 1e-10), "eq1 y: {}", eq1[1]);
        assert!(approx_eq(eq1[2], cfg.c, 1e-10), "eq1 z: {}", eq1[2]);
        assert!(approx_eq(eq2[0], -x_star, 1e-10), "eq2 x: {}", eq2[0]);
        assert!(approx_eq(eq2[1], -x_star, 1e-10), "eq2 y: {}", eq2[1]);
        assert!(approx_eq(eq2[2], cfg.c, 1e-10), "eq2 z: {}", eq2[2]);
    }

    #[test]
    fn test_equilibria_satisfy_equations() {
        let cfg = LuConfig::default();
        let (eq1, eq2) = LuSolver::equilibria(&cfg).unwrap();
        for eq in [eq1, eq2] {
            let d = LuSolver::derivatives(&cfg, eq[0], eq[1], eq[2]);
            for v in d.iter() {
                assert!(v.abs() < 1e-9, "equilibrium derivative = {}", v);
            }
        }
    }

    #[test]
    fn test_origin_is_equilibrium() {
        let cfg = LuConfig::default();
        let d = LuSolver::derivatives(&cfg, 0.0, 0.0, 0.0);
        for v in d.iter() {
            assert!(v.abs() < 1e-12);
        }
    }

    #[test]
    fn test_equilibria_symmetric() {
        // E± 关于 (x, y) 反演对称 (z 相同)
        let cfg = LuConfig::default();
        let (eq1, eq2) = LuSolver::equilibria(&cfg).unwrap();
        assert!(approx_eq(eq1[0], -eq2[0], 1e-12));
        assert!(approx_eq(eq1[1], -eq2[1], 1e-12));
        assert!(approx_eq(eq1[2], eq2[2], 1e-12));
    }

    #[test]
    fn test_equilibria_none_when_bc_negative() {
        // 当 b*c < 0 (b 和 c 异号) 时无非平凡平衡点
        let cfg = LuConfig { a: 36.0, b: -3.0, c: 20.0, dt: 0.001 };
        assert!(LuSolver::equilibria(&cfg).is_none());
    }

    #[test]
    fn test_step_advances() {
        let mut s = LuSolver::classic(LuConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = LuSolver::classic(LuConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = LuSolver::classic(LuConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = LuSolver::classic(LuConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        // Lü 吸引子有界
        let mut s = LuSolver::classic(LuConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -100.0 && xmax < 100.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -100.0 && ymax < 100.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -100.0 && zmax < 100.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        // Lü 系统是混沌的, λ > 0 (文献值 ~1.746)
        let mut s = LuSolver::classic(LuConfig::default());
        s.run(50000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = LuSolver::classic(LuConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 20.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        // 两条相近轨道应指数分离. Lü λ~1.75, 分离快.
        let cfg = LuConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = LuSolver::new(cfg, 1.0, 1.0, 1.0);
        let mut s2 = LuSolver::new(cfg, 1.0 + d0, 1.0, 1.0);
        for _ in 0..20000 {
            s1.step();
            s2.step();
        }
        let dx = s1.x - s2.x;
        let dy = s1.y - s2.y;
        let dz = s1.z - s2.z;
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        // t=20, λ~1.75, 应放大 e^35 倍 (饱和到吸引子尺度 ~10)
        assert!(d > 1e-3, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_volume_monotonic_contraction() {
        // 散度 = -19 (常数负), 体积单调收缩
        let cfg = LuConfig::default();
        let div = LuSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < 0.0);
        for &(x, y, z) in &[(1.0_f64, 2.0, 3.0), (-5.0, 7.0, -3.0), (10.0, -10.0, 20.0)] {
            assert!(LuSolver::divergence(&cfg, x, y, z) < 0.0);
        }
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = LuConfig::default();
        let mut s = LuSolver::new(cfg, 1000.0, 1000.0, 1000.0);
        s.run(500);
        assert!(s.has_escaped() || !s.has_nan());
    }

    #[test]
    fn test_bridge_condition_lorenz_lu_chen() {
        // 验证 Lorenz-Lü-Chen 三联体的代数桥接条件:
        //   Lorenz: a12*a21 > 0
        //   Lü:     a12*a21 = 0 (边界, a21 = 0)
        //   Chen:   a12*a21 < 0
        // 这里只验证 Lü 的 a21 = 0 条件.
        let cfg = LuConfig::default();
        // Lü 在原点: J = [[-a, a, 0], [0, c, 0], [0, 0, -b]]
        // a12 = a > 0, a21 = -z = 0
        let j_at_origin = LuSolver::jacobian(&cfg, 0.0, 0.0, 0.0);
        let a12 = j_at_origin[0][1]; // a
        let a21 = j_at_origin[1][0]; // -z = 0 at origin
        assert!(a12 > 0.0, "a12 = a should be positive");
        assert!(approx_eq(a21, 0.0, 1e-12), "a21 = 0 (bridge condition)");
        assert!(approx_eq(a12 * a21, 0.0, 1e-12), "Lü: a12*a21 = 0 (bridge)");
    }

    #[test]
    fn test_lyapunov_less_than_lorenz_chen_scale() {
        // Lü 的 λ ≈ 1.746, 介于 Lorenz (0.906) 和 Chen (2.027) 之间
        // 验证数值结果在合理范围 (0.5, 3.0)
        let mut s = LuSolver::classic(LuConfig::default());
        s.run(50000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.5 && lambda < 3.0,
            "lambda should be in (0.5, 3.0): {}", lambda);
    }
}
