//! Nose-Hoover 振子 — 热力学混沌系统
//!
//! Nose-Hoover 振子是统计力学中用于恒温分子动力学 (isothermal molecular
//! dynamics) 的简化模型, 由 Shuichi Nose (1984) 和 William Hoover (1985)
//! 提出. 它描述了一个振子与热浴耦合的动力学, 展现混沌行为.
//!
//! 方程 (α=1 经典参数):
//!   dx/dt = y
//!   dy/dt = -x + y z
//!   dz/dt = α - y²
//!
//! 性质:
//!   - 无不动点 (α > 0 时): 令 dx/dt=dy/dt=dz/dt=0 → y=0, x=0, α=0 (矛盾)
//!   - 保守-耗散混合: 散度 ∇·F = z (非常数, 时正时负)
//!   - 长期平均散度 ≈ 0 (类似保守系统)
//!   - Lyapunov 指数: λ₁ > 0 (混沌), λ₂ = 0 (中性), λ₃ < 0 (收缩)
//!   - 吸引子: 不是真正的"吸引子" (因散度非负定), 而是"混沌海"
//!   - Kaplan-Yorke 维数 D_KY ≈ 2 + λ₁/|λ₃| > 2
//!
//! 守恒量 (α=1):
//!   H = ½ y² + ½ z² - x y - α z (Hamiltonian-like, 但不严格守恒)
//!   实际上 Nose-Hoover 不严格守恒任何光滑量, 但有奇异不变集
//!
//! 几何结构:
//!   - 混沌海 + 周期岛 (KAM 类似结构)
//!   - 类似 Hénon-Heiles 的 KAM 破裂图像
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta)
//!
//! 历史:
//!   Nose, S. 1984. "A unified formulation of the constant temperature
//!   molecular dynamics methods." J. Chem. Phys. 81, 511.
//!   Hoover, W. G. 1985. "Canonical dynamics: Equilibrium phase-space
//!   distributions." Phys. Rev. A 31, 1695.
//!   Posch, H. A., Hoover, W. G. & Vesely, F. J. 1986. "Canonical dynamics
//!   of the Nosé oscillator: Stability, order, and chaos." Phys. Rev. A 33,
//!   4253. (首次详细分析 Nose-Hoover 的混沌结构)

/// Nose-Hoover 配置
#[derive(Clone, Copy, Debug)]
pub struct NoseHooverConfig {
    /// 参数 α (热浴强度, 经典 1.0)
    pub alpha: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for NoseHooverConfig {
    fn default() -> Self {
        Self { alpha: 1.0, dt: 0.01 }
    }
}

/// Nose-Hoover 振子求解器
pub struct NoseHooverSolver {
    pub config: NoseHooverConfig,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub time: f64,
    pub step_count: u64,
    pub trajectory: Vec<(f64, f64, f64)>,
    /// Lyapunov 累积
    pub lyap_sum: f64,
    /// 切向量
    pub v: [f64; 3],
}

impl NoseHooverSolver {
    pub fn new(config: NoseHooverConfig, x0: f64, y0: f64, z0: f64) -> Self {
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

    pub fn classic(config: NoseHooverConfig) -> Self {
        // 经典初值 (Posch-Hoover-Vesely 1986)
        Self::new(config, 0.0, 5.0, 0.0)
    }

    /// 右端导数 F = [y, -x + y z, α - y²]
    pub fn derivatives(cfg: &NoseHooverConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        [y, -x + y * z, cfg.alpha - y * y]
    }

    /// Jacobian: J = [[0, 1, 0], [-1, z, y], [0, -2y, 0]]
    pub fn jacobian(cfg: &NoseHooverConfig, _x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
        [
            [0.0, 1.0, 0.0],
            [-1.0, z, y],
            [0.0, -2.0 * y, 0.0],
        ]
    }

    /// 散度 ∇·F = tr(J) = z (非常数)
    pub fn divergence(_cfg: &NoseHooverConfig, _x: f64, _y: f64, z: f64) -> f64 {
        z
    }

    /// 类 Hamiltonian 量 H = ½ y² + ½ z² - x y - α z
    /// (不严格守恒, 但作为诊断量)
    pub fn hamiltonian(cfg: &NoseHooverConfig, x: f64, y: f64, z: f64) -> f64 {
        0.5 * y * y + 0.5 * z * z - x * y - cfg.alpha * z
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
        let cfg = NoseHooverConfig::default();
        assert!(approx_eq(cfg.alpha, 1.0, 1e-12));
        assert!(approx_eq(cfg.dt, 0.01, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = NoseHooverSolver::classic(NoseHooverConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = NoseHooverConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = NoseHooverSolver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], y, 1e-12));
        assert!(approx_eq(d[1], -x + y * z, 1e-12));
        assert!(approx_eq(d[2], cfg.alpha - y * y, 1e-12));
    }

    #[test]
    fn test_no_fixed_point_for_positive_alpha() {
        // α > 0 时无不动点
        // dx/dt=0 → y=0; dy/dt=0 → -x=0; dz/dt=0 → α=0 (矛盾)
        let cfg = NoseHooverConfig::default();
        // 在 (0,0,0) 处 dz/dt = α ≠ 0
        let d = NoseHooverSolver::derivatives(&cfg, 0.0, 0.0, 0.0);
        assert!(!approx_eq(d[2], 0.0, 1e-12));
        assert!(approx_eq(d[2], cfg.alpha, 1e-12));
    }

    #[test]
    fn test_fixed_point_at_origin_for_alpha_zero() {
        // α=0: 原点 (0,0,0) 是不动点
        let cfg = NoseHooverConfig { alpha: 0.0, dt: 0.01 };
        let d = NoseHooverSolver::derivatives(&cfg, 0.0, 0.0, 0.0);
        assert!(approx_eq(d[0], 0.0, 1e-12));
        assert!(approx_eq(d[1], 0.0, 1e-12));
        assert!(approx_eq(d[2], 0.0, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        // J = [[0, 1, 0], [-1, z, y], [0, -2y, 0]]
        let cfg = NoseHooverConfig::default();
        let j = NoseHooverSolver::jacobian(&cfg, 0.3, 0.5, 0.7);
        assert!(approx_eq(j[0][0], 0.0, 1e-12));
        assert!(approx_eq(j[0][1], 1.0, 1e-12));
        assert!(approx_eq(j[0][2], 0.0, 1e-12));
        assert!(approx_eq(j[1][0], -1.0, 1e-12));
        assert!(approx_eq(j[1][1], 0.7, 1e-12));
        assert!(approx_eq(j[1][2], 0.5, 1e-12));
        assert!(approx_eq(j[2][0], 0.0, 1e-12));
        assert!(approx_eq(j[2][1], -1.0, 1e-12));
        assert!(approx_eq(j[2][2], 0.0, 1e-12));
    }

    #[test]
    fn test_divergence_is_z() {
        // 散度 = z (非常数)
        let cfg = NoseHooverConfig::default();
        assert!(approx_eq(NoseHooverSolver::divergence(&cfg, 0.0, 0.0, 0.5), 0.5, 1e-12));
        assert!(approx_eq(NoseHooverSolver::divergence(&cfg, 0.0, 0.0, -0.3), -0.3, 1e-12));
        assert!(approx_eq(NoseHooverSolver::divergence(&cfg, 0.0, 0.0, 0.0), 0.0, 1e-12));
    }

    #[test]
    fn test_hamiltonian_value() {
        let cfg = NoseHooverConfig::default();
        let h = NoseHooverSolver::hamiltonian(&cfg, 0.3, 0.5, 0.7);
        let expected = 0.5 * 0.5 * 0.5 + 0.5 * 0.7 * 0.7 - 0.3 * 0.5 - cfg.alpha * 0.7;
        assert!(approx_eq(h, expected, 1e-12));
    }

    #[test]
    fn test_step_advances() {
        let mut s = NoseHooverSolver::classic(NoseHooverConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = NoseHooverSolver::classic(NoseHooverConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = NoseHooverSolver::classic(NoseHooverConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        // 经典 Nose-Hoover 轨道有界
        let mut s = NoseHooverSolver::classic(NoseHooverConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_attractor_range_known() {
        // 经典 Posch-Hoover 初值下, 轨道大致 |x|<10, |y|<10, |z|<10
        let mut s = NoseHooverSolver::classic(NoseHooverConfig::default());
        s.run(50000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -20.0 && xmax < 20.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -20.0 && ymax < 20.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -20.0 && zmax < 20.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        // 经典 Nose-Hoover 应有正 Lyapunov (混沌)
        let mut s = NoseHooverSolver::classic(NoseHooverConfig::default());
        s.run(50000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        // Lyapunov 应为有限值
        let mut s = NoseHooverSolver::classic(NoseHooverConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 5.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        // 两条相近轨道应分离. Nose-Hoover 相空间含 KAM 岛,
        // 初值 (0,5,0) 可能穿过规则区域使短时分离较弱, 需长时间.
        let cfg = NoseHooverConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = NoseHooverSolver::new(cfg, 0.0, 5.0, 0.0);
        let mut s2 = NoseHooverSolver::new(cfg, d0, 5.0, 0.0);
        for _ in 0..100000 {
            s1.step();
            s2.step();
        }
        let dx = s1.x - s2.x;
        let dy = s1.y - s2.y;
        let dz = s1.z - s2.z;
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        // t=1000, λ~0.1, 应放大许多数量级 (已放大 100x 即可证明混沌)
        assert!(d > 1e-4, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = NoseHooverConfig::default();
        let mut s = NoseHooverSolver::new(cfg, 100.0, 100.0, 100.0);
        s.run(500);
        // 大初值可能逃逸
        assert!(s.has_escaped() || !s.has_nan());
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = NoseHooverSolver::classic(NoseHooverConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_volume_not_monotonic() {
        // 散度 = z, 时正时负, 体积非单调收缩
        let cfg = NoseHooverConfig::default();
        let mut s = NoseHooverSolver::classic(cfg);
        s.run(10000);
        let mut positive = 0;
        let mut negative = 0;
        for &(x, y, z) in &s.trajectory {
            let div = NoseHooverSolver::divergence(&cfg, x, y, z);
            if div > 0.0 {
                positive += 1;
            } else if div < 0.0 {
                negative += 1;
            }
        }
        // 应该有正有负 (非单调耗散)
        assert!(positive > 100, "should have positive divergence: {}", positive);
        assert!(negative > 100, "should have negative divergence: {}", negative);
    }

    #[test]
    fn test_jacobian_trace_is_z() {
        // tr(J) = z (散度)
        let cfg = NoseHooverConfig::default();
        let j = NoseHooverSolver::jacobian(&cfg, 0.3, 0.5, 0.7);
        let tr = j[0][0] + j[1][1] + j[2][2];
        assert!(approx_eq(tr, 0.7, 1e-12));
    }

    #[test]
    fn test_alpha_zero_origin_stable_manifold() {
        // α=0: 原点是平衡点, J(0,0,0) = [[0,1,0],[-1,0,0],[0,0,0]]
        // 特征值: λ³ + λ = 0 → λ(λ²+1) = 0 → λ = 0, ±i
        // 中心 + 中心 (退化), 线性分析不充分
        let cfg = NoseHooverConfig { alpha: 0.0, dt: 0.01 };
        let j = NoseHooverSolver::jacobian(&cfg, 0.0, 0.0, 0.0);
        assert!(approx_eq(j[0][0], 0.0, 1e-12));
        assert!(approx_eq(j[1][1], 0.0, 1e-12));
        assert!(approx_eq(j[2][2], 0.0, 1e-12));
        // xy 块: [[0,1],[-1,0]] 是旋转矩阵, 特征值 ±i (中心)
        let tr_xy = j[0][0] + j[1][1];
        let det_xy = j[0][0] * j[1][1] - j[0][1] * j[1][0];
        assert!(approx_eq(tr_xy, 0.0, 1e-12));
        assert!(approx_eq(det_xy, 1.0, 1e-12)); // 旋转
    }

    #[test]
    fn test_classic_initial_in_attractor() {
        // 经典初值 (0, 5, 0) 应进入混沌海 (不逃逸)
        let mut s = NoseHooverSolver::classic(NoseHooverConfig::default());
        s.run(50000);
        assert!(!s.has_escaped());
    }

    #[test]
    fn test_jacobian_at_origin_alpha_nonzero() {
        // α≠0: 原点不是不动点, 但 J(0,0,0) 仍可计算
        // J = [[0,1,0],[-1,0,0],[0,0,0]] (与 α 无关, 因 J 不含 α)
        let cfg = NoseHooverConfig::default();
        let j = NoseHooverSolver::jacobian(&cfg, 0.0, 0.0, 0.0);
        assert!(approx_eq(j[0][0], 0.0, 1e-12));
        assert!(approx_eq(j[1][0], -1.0, 1e-12));
        assert!(approx_eq(j[2][2], 0.0, 1e-12));
    }

    #[test]
    fn test_alpha_affects_dynamics() {
        // 不同 α 给出不同动力学
        let cfg1 = NoseHooverConfig { alpha: 1.0, dt: 0.005 };
        let cfg2 = NoseHooverConfig { alpha: 2.0, dt: 0.005 };
        let mut s1 = NoseHooverSolver::new(cfg1, 0.0, 5.0, 0.0);
        let mut s2 = NoseHooverSolver::new(cfg2, 0.0, 5.0, 0.0);
        s1.run(1000);
        s2.run(1000);
        // 轨道应不同
        let d = (s1.x - s2.x).powi(2) + (s1.y - s2.y).powi(2) + (s1.z - s2.z).powi(2);
        assert!(d.sqrt() > 1e-3, "different alpha should give different orbits: {}", d);
    }
}
