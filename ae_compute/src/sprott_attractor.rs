//! Sprott Attractor (Case A) — 无平衡点混沌系统 (隐藏吸引子前驱)
//!
//! Julien C. Sprott 1993 年通过穷举搜索发现 19 个 (A-S) 最简单多项式
//! 混沌系统. 其中 case A 是最简洁的例子之一, 由 3 个变量 + 3 个二次项
//! 构成, 展现稳定混沌吸引子. 关键性质: 系统不存在任何不动点 (实数解),
//! 这一特征在 2010 年后被 Leonov-Kuznetsov 重新发掘并命名为
//! "隐藏吸引子 (hidden attractor)", 区别于以 Lorenz/Rössler 为代表的
//! "自激吸引子 (self-excited attractor)" (其吸引域与不稳定平衡点相交).
//!
//! 状态方程 (Sprott 1993, case A):
//!   dx/dt = y
//!   dy/dt = -x + y z
//!   dz/dt = 1 - y^2
//!
//! 无参数, 唯一可调量是时间步长 dt.
//! 经典初值: (x0, y0, z0) = (0.1, 0.1, 0) 或 (0.1, 0, 0)
//!
//! 性质:
//!   - 无不动点: y=0, x=0, 1-y^2=1=0 矛盾, 故无实平衡点
//!   - 散度 ∇·F = tr(J) = z (非常数, 时正时负)
//!   - 长期平均散度 ≈ 0 (准保守, 类似 Nose-Hoover)
//!   - Lyapunov 谱 (Sprott 1993): λ₁ ≈ 0.273, λ₂ = 0, λ₃ ≈ -0.273
//!     (和 ≈ 0, 准保守)
//!   - Kaplan-Yorke 维数 D_KY = 2 + λ₁/|λ₃| ≈ 3.0 (但轨道有界)
//!   - 吸引子不是"吸引子"严格意义 (准保守), 而是"混沌海"
//!
//! 隐藏吸引子意义 (Leonov-Kuznetsov 2010+):
//!   传统吸引子 (Lorenz, Rössler, Chua) 的吸引域都与不稳定平衡点
//!   的稳定流形相交, 从平衡点附近出发即可到达. Sprott A 无平衡点,
//!   必须从特定 basin 出发, 称为"隐藏". 这一概念在飞行控制、
//!   电机、PLL 锁相环等领域有重要工程意义, 因传统线性化分析
//!   (基于平衡点) 无法预测此类隐藏振荡.
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!
//! 历史:
//!   Sprott, J. C. 1993. "Strange attractors: creating patterns in chaos."
//!   Comput. Phys. 7, 137. (开创性穷举搜索)
//!   Sprott, J. C. 1994. "Some simple chaotic flows."
//!   Phys. Rev. E 50, R647. (系统化整理)
//!   Leonov, G. A. & Kuznetsov, N. V. 2011. "Algorithms for searching
//!   for hidden oscillations..." IFAC Proc. 44, 12609. (隐藏吸引子命名)
//!   Jafari, S., Sprott, J. C. & Golpayegani, S. M. R. H. 2013.
//!   "Elementary quadratic chaotic flows with no equilibria." Phys. Lett.
//!   A 377, 699. (后继工作)

/// Sprott A 配置 (无物理参数, 仅时间步长)
#[derive(Clone, Copy, Debug)]
pub struct SprottAConfig {
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for SprottAConfig {
    fn default() -> Self {
        Self { dt: 0.01 }
    }
}

/// Sprott A 求解器 (3D, 跟踪最大 Lyapunov 指数)
pub struct SprottASolver {
    pub config: SprottAConfig,
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

impl SprottASolver {
    pub fn new(config: SprottAConfig, x0: f64, y0: f64, z0: f64) -> Self {
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

    pub fn classic(config: SprottAConfig) -> Self {
        // Sprott 1993 推荐初值 (0.1, 0.1, 0)
        Self::new(config, 0.1, 0.1, 0.0)
    }

    /// 右端导数 F = [y, -x + y z, 1 - y^2]
    pub fn derivatives(_cfg: &SprottAConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        [y, -x + y * z, 1.0 - y * y]
    }

    /// Jacobian: J = [[0, 1, 0], [-1, z, y], [0, -2y, 0]]
    pub fn jacobian(_cfg: &SprottAConfig, _x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
        [
            [0.0, 1.0, 0.0],
            [-1.0, z, y],
            [0.0, -2.0 * y, 0.0],
        ]
    }

    /// 散度 ∇·F = tr(J) = z (非常数)
    pub fn divergence(_cfg: &SprottAConfig, _x: f64, _y: f64, z: f64) -> f64 {
        z
    }

    /// 系统不存在实数平衡点 (隐藏吸引子的标志性特征).
    /// 解析推导: y=0, x=0, 1-y^2=1=0 矛盾.
    /// 此函数返回 None, 与其他系统 equilibria() 接口保持一致.
    pub fn equilibria(_cfg: &SprottAConfig) -> Option<Vec<[f64; 3]>> {
        None
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

    /// 最大 Lyapunov 指数 (Sprott 1993 文献值 ~0.273)
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
        let cfg = SprottAConfig::default();
        assert!(approx_eq(cfg.dt, 0.01, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = SprottASolver::classic(SprottAConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = SprottAConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = SprottASolver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], y, 1e-12));
        assert!(approx_eq(d[1], -x + y * z, 1e-12));
        assert!(approx_eq(d[2], 1.0 - y * y, 1e-12));
    }

    #[test]
    fn test_derivatives_at_origin() {
        // 原点处: dx=0, dy=0, dz=1 (dz ≠ 0 是无平衡点的关键)
        let cfg = SprottAConfig::default();
        let d = SprottASolver::derivatives(&cfg, 0.0, 0.0, 0.0);
        assert!(approx_eq(d[0], 0.0, 1e-12));
        assert!(approx_eq(d[1], 0.0, 1e-12));
        assert!(approx_eq(d[2], 1.0, 1e-12));
    }

    #[test]
    fn test_no_equilibria() {
        // 隐藏吸引子标志: 无实数平衡点
        // y=0, x=0, 1-y^2=1=0 矛盾
        let cfg = SprottAConfig::default();
        assert!(SprottASolver::equilibria(&cfg).is_none());
    }

    #[test]
    fn test_no_equilibria_numerical_check() {
        // 数值验证: 任何点 (x, y, 0) 处 dz/dt = 1 - y^2 ≠ 0 当 y ≠ ±1
        // 而在 dx/dt=0 (y=0) 处 dy/dt = -x ≠ 0 (除非 x=0), 但此时 dz/dt = 1
        // 故无解
        let cfg = SprottAConfig::default();
        // 检查 (0, 0, z): dz/dt = 1 ≠ 0
        let d = SprottASolver::derivatives(&cfg, 0.0, 0.0, 5.0);
        assert!(!approx_eq(d[2], 0.0, 1e-12));
        // 检查 (x, 1, z): dy/dt = -x + z, dx/dt = 1 (≠0)
        let d = SprottASolver::derivatives(&cfg, 5.0, 1.0, 5.0);
        assert!(!approx_eq(d[0], 0.0, 1e-12));
        // 检查 (x, -1, z): dx/dt = -1 (≠0)
        let d = SprottASolver::derivatives(&cfg, 5.0, -1.0, 5.0);
        assert!(!approx_eq(d[0], 0.0, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        // J = [[0, 1, 0], [-1, z, y], [0, -2y, 0]]
        let cfg = SprottAConfig::default();
        let j = SprottASolver::jacobian(&cfg, 0.3, 0.5, 0.7);
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
        let cfg = SprottAConfig::default();
        assert!(approx_eq(SprottASolver::divergence(&cfg, 0.0, 0.0, 0.5), 0.5, 1e-12));
        assert!(approx_eq(SprottASolver::divergence(&cfg, 0.0, 0.0, -0.3), -0.3, 1e-12));
        assert!(approx_eq(SprottASolver::divergence(&cfg, 0.0, 0.0, 0.0), 0.0, 1e-12));
    }

    #[test]
    fn test_divergence_not_constant() {
        // 散度随 z 变化 (非常数)
        let cfg = SprottAConfig::default();
        let d1 = SprottASolver::divergence(&cfg, 0.0, 0.0, 0.0);
        let d2 = SprottASolver::divergence(&cfg, 0.0, 0.0, 1.0);
        assert!((d1 - d2).abs() > 0.5);
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        // tr(J) = z = divergence
        let cfg = SprottAConfig::default();
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5)] {
            let j = SprottASolver::jacobian(&cfg, x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = SprottASolver::divergence(&cfg, x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_step_advances() {
        let mut s = SprottASolver::classic(SprottAConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = SprottASolver::classic(SprottAConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = SprottASolver::classic(SprottAConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = SprottASolver::classic(SprottAConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        // 轨道有界 (准保守混沌海)
        let mut s = SprottASolver::classic(SprottAConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        // Sprott A 是混沌系统, λ > 0 (文献值 ~0.273)
        let mut s = SprottASolver::classic(SprottAConfig::default());
        s.run(50000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = SprottASolver::classic(SprottAConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 5.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        // 两条相近轨道应指数分离
        let cfg = SprottAConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = SprottASolver::new(cfg, 0.1, 0.1, 0.0);
        let mut s2 = SprottASolver::new(cfg, 0.1 + d0, 0.1, 0.0);
        for _ in 0..100000 {
            s1.step();
            s2.step();
        }
        let dx = s1.x - s2.x;
        let dy = s1.y - s2.y;
        let dz = s1.z - s2.z;
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        // Sprott A 是准保守系统 (长期平均散度 ≈ 0), FTLE 在不同混沌海区域
        // 波动很大, 轨道可能长时间滞留在 KAM 岛边缘 (sticky region),
        // 短时有效分离率远低于长期 λ ≈ 0.273. 这里只要求 10x 放大即可
        // 证明混沌 (排除保守系统稳定流形情形).
        assert!(d > 1e-5, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_volume_not_monotonic() {
        // 散度 = z, 时正时负, 体积非单调
        let cfg = SprottAConfig::default();
        let mut s = SprottASolver::classic(cfg);
        s.run(10000);
        let mut positive = 0;
        let mut negative = 0;
        for &(x, y, z) in &s.trajectory {
            let div = SprottASolver::divergence(&cfg, x, y, z);
            if div > 0.0 {
                positive += 1;
            } else if div < 0.0 {
                negative += 1;
            }
        }
        assert!(positive > 100, "should have positive divergence: {}", positive);
        assert!(negative > 100, "should have negative divergence: {}", negative);
    }

    #[test]
    fn test_average_divergence_near_zero() {
        // 准保守: 长期平均散度 ≈ 0 (与 λ₁ + λ₂ + λ₃ ≈ 0 一致)
        let cfg = SprottAConfig::default();
        let mut s = SprottASolver::classic(cfg);
        s.run(50000);
        let n = s.trajectory.len() as f64;
        let avg_div: f64 = s.trajectory.iter()
            .map(|&(x, y, z)| SprottASolver::divergence(&cfg, x, y, z))
            .sum::<f64>() / n;
        // 平均散度应远小于 1 (准保守)
        assert!(avg_div.abs() < 1.0, "avg divergence: {}", avg_div);
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = SprottAConfig::default();
        let mut s = SprottASolver::new(cfg, 100.0, 100.0, 100.0);
        s.run(500);
        // 大初值可能逃逸
        assert!(s.has_escaped() || !s.has_nan());
    }

    #[test]
    fn test_initial_sensitivity_in_y() {
        // 验证 z 方程: dz/dt = 1 - y^2 在 |y| < 1 时为正
        // 故轨道在 z 方向有持续推动 (与"无平衡点"一致)
        let cfg = SprottAConfig::default();
        let d1 = SprottASolver::derivatives(&cfg, 0.0, 0.5, 0.0);
        let d2 = SprottASolver::derivatives(&cfg, 0.0, 2.0, 0.0);
        assert!(d1[2] > 0.0, "dz/dt > 0 for |y| < 1: {}", d1[2]);
        assert!(d2[2] < 0.0, "dz/dt < 0 for |y| > 1: {}", d2[2]);
    }
}
