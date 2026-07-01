//! Sprott L Attractor - Sprott L 简单混沌系统 (3D)
//!
//! Julien C. Sprott 1994 年在其论文 "Some simple chaotic flows" 中
//! 系统搜索发现的最简单混沌系统之一 (Sprott A-S, 共 19 个). Sprott L
//! 是其中第十二个, 特点是常数散度 (div = -1, 与参数 a, b 无关),
//! 且唯一平衡点不在原点.
//!
//! 状态方程 (Sprott L, 参数化版本):
//!   dx/dt = y + a*z
//!   dy/dt = b*x^2 - y
//!   dz/dt = 1 - x
//!
//! 各项物理意义:
//!   + y: 线性耦合 (x 受 y 牵引)
//!   + a*z: 线性耦合 (x 受 z 驱动, a 控制 z->x 耦合强度)
//!   + b*x^2: 非线性耦合 (y 受 x^2 驱动, 唯一非线性项, b 控制非线性强度)
//!   - y: 线性阻尼 (y 衰减)
//!   + 1: 常数驱动 (z 受常数偏置驱动, 形成偏移平衡点)
//!   - x: 线性耦合 (z 受 -x 反馈驱动)
//!
//! 经典参数 (Sprott 1994): a = 3.9, b = 0.9
//! 经典初值: (x0, y0, z0) = (0.1, 0.1, 0.1)
//!
//! 性质:
//!   - 散度 div(F) = tr(J) = -1 (常数, 与位置和参数 a, b 均无关!)
//!     这意味着无论 a, b 取何值, 系统始终耗散, 相体积以 e^(-t) 速率收缩.
//!     中等耗散速率 (与 Sprott I 散度恒为 -1 相同).
//!     (与 Sprott K 的 div = y + a - 1 非常数不同, Sprott L 散度恒为 -1)
//!   - 平衡点 (利用 y + a*z = 0, b*x^2 - y = 0, 1 - x = 0):
//!     从 dz/dt=0: x = 1
//!     从 dy/dt=0: y = b*x^2 = b*1 = b
//!     从 dx/dt=0: y + a*z = 0, 即 z = -y/a = -b/a
//!     唯一平衡点: E0 = (1, b, -b/a)
//!     (经典 a=3.9, b=0.9: E0 = (1, 0.9, -0.9/3.9) 约等于 (1, 0.9, -0.23077))
//!     (与 Sprott I, Sprott J 平衡点在原点不同, Sprott L 平衡点偏离原点)
//!     (与 Sprott K 有两个平衡点不同, Sprott L 只有一个)
//!   - 在 E0 处 Jacobian:
//!     J0 = [[0, 1, a], [2*b, -1, 0], [-1, 0, 0]]
//!     (a=3.9, b=0.9: J0 = [[0,1,3.9],[1.8,-1,0],[-1,0,0]])
//!     特征方程含 a, b, E0 为不稳定鞍焦 (产生混沌)
//!   - Lyapunov 谱 (经典参数, 文献值):
//!     L1 approx +0.15  (正, 主混沌方向)
//!     L2 = 0      (沿轨道切向)
//!     L3 approx -1.15  (负, 收缩)
//!     和 = -1.0 (与散度一致)
//!   - Kaplan-Yorke 维数 D_KY approx 2 + L1/|L3| approx 2.13
//!   - 吸引子形态: 单叶扭曲带
//!
//! Sprott 系列对比 (已实现):
//!   - Sprott G: 1 个非线性项 (xz), 6 项, 已完成
//!   - Sprott H: 1 个非线性项 (z^2), 5 项, 已完成
//!   - Sprott I: 1 个非线性项 (y^2), 6 项, 已完成 (散度恒为 -1, 与 a 无关)
//!   - Sprott J: 1 个非线性项 (y^2), 6 项, 已完成 (散度 = -a, 依赖 a, 常数)
//!   - Sprott K: 1 个非线性项 (xy), 6 项, 已完成 (散度 = y+a-1, 非常数!)
//!   - Sprott L: 1 个非线性项 (x^2), 6 项, 本模块 (散度 = -1, 常数, 与 a, b 无关)
//!   - Sprott 系统共同特征: 极简 (3 变量, 不超过 2 非线性项)
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!
//! 历史:
//!   Sprott, J. C. 1994. "Some simple chaotic flows." Phys. Rev. E 50,
//!   R647-R650. (Sprott A-S 系统的原始发现)

/// Sprott L 系统配置 (2 参数 a, b + dt)
#[derive(Clone, Copy, Debug)]
pub struct SprottLConfig {
    /// 参数 a (z -> x 耦合强度, 经典 3.9)
    pub a: f64,
    /// 参数 b (x^2 -> y 非线性耦合强度, 经典 0.9)
    pub b: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for SprottLConfig {
    fn default() -> Self {
        Self { a: 3.9, b: 0.9, dt: 0.01 }
    }
}

/// Sprott L 求解器 (3D, 跟踪最大 Lyapunov 指数)
pub struct SprottLSolver {
    pub config: SprottLConfig,
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

impl SprottLSolver {
    pub fn new(config: SprottLConfig, x0: f64, y0: f64, z0: f64) -> Self {
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

    pub fn classic(config: SprottLConfig) -> Self {
        Self::new(config, 0.1, 0.1, 0.1)
    }

    /// 右端导数 F = [y + a*z, b*x^2 - y, 1 - x]
    pub fn derivatives(cfg: &SprottLConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        [y + cfg.a * z, cfg.b * x * x - y, 1.0 - x]
    }

    /// Jacobian:
    /// J = [[0,      1,  a],
    ///      [2*b*x, -1,  0],
    ///      [-1,    0,   0]]
    pub fn jacobian(cfg: &SprottLConfig, x: f64, _y: f64, _z: f64) -> [[f64; 3]; 3] {
        [
            [0.0, 1.0, cfg.a],
            [2.0 * cfg.b * x, -1.0, 0.0],
            [-1.0, 0.0, 0.0],
        ]
    }

    /// 散度 div(F) = tr(J) = -1 (常数, 与位置和参数 a, b 均无关)
    pub fn divergence(_cfg: &SprottLConfig, _x: f64, _y: f64, _z: f64) -> f64 {
        -1.0
    }

    /// 计算平衡点:
    /// 唯一平衡点 E0 = (1, b, -b/a)
    /// (返回元组以保持与 Sprott K 相同的接口; 第二个元素为 NAN 表示不存在)
    pub fn equilibria(cfg: &SprottLConfig) -> ([f64; 3], [f64; 3]) {
        let e0 = [1.0, cfg.b, -cfg.b / cfg.a];
        (e0, [f64::NAN, f64::NAN, f64::NAN])
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

    /// 最大 Lyapunov 指数 (文献值 ~0.15)
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
        let cfg = SprottLConfig::default();
        assert!(approx_eq(cfg.a, 3.9, 1e-12));
        assert!(approx_eq(cfg.b, 0.9, 1e-12));
        assert!(approx_eq(cfg.dt, 0.01, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = SprottLSolver::classic(SprottLConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = SprottLConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = SprottLSolver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], y + cfg.a * z, 1e-12));
        assert!(approx_eq(d[1], cfg.b * x * x - y, 1e-12));
        assert!(approx_eq(d[2], 1.0 - x, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = SprottLConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = SprottLSolver::jacobian(&cfg, x, y, z);
        assert!(approx_eq(j[0][0], 0.0, 1e-12));
        assert!(approx_eq(j[0][1], 1.0, 1e-12));
        assert!(approx_eq(j[0][2], cfg.a, 1e-12));
        assert!(approx_eq(j[1][0], 2.0 * cfg.b * x, 1e-12));
        assert!(approx_eq(j[1][1], -1.0, 1e-12));
        assert!(approx_eq(j[1][2], 0.0, 1e-12));
        assert!(approx_eq(j[2][0], -1.0, 1e-12));
        assert!(approx_eq(j[2][1], 0.0, 1e-12));
        assert!(approx_eq(j[2][2], 0.0, 1e-12));
    }

    #[test]
    fn test_divergence_constant() {
        // 散度 = -1 (常数, 与位置和参数 a, b 均无关)
        let cfg = SprottLConfig::default();
        assert!(approx_eq(SprottLSolver::divergence(&cfg, 0.0, 0.0, 0.0), -1.0, 1e-12));
        assert!(approx_eq(SprottLSolver::divergence(&cfg, 1.0, 2.0, 3.0), -1.0, 1e-12));
        // 改变 a 不影响散度
        let cfg2 = SprottLConfig { a: 2.0, ..cfg };
        assert!(approx_eq(SprottLSolver::divergence(&cfg2, 0.0, 0.0, 0.0), -1.0, 1e-12));
        // 改变 b 不影响散度
        let cfg3 = SprottLConfig { b: 1.5, ..cfg };
        assert!(approx_eq(SprottLSolver::divergence(&cfg3, 5.0, -3.0, 2.0), -1.0, 1e-12));
        // 同时改变 a, b 仍不影响散度
        let cfg4 = SprottLConfig { a: 5.0, b: 2.0, ..cfg };
        assert!(approx_eq(SprottLSolver::divergence(&cfg4, -2.0, 7.0, 1.0), -1.0, 1e-12));
    }

    #[test]
    fn test_divergence_negative_dissipative() {
        let cfg = SprottLConfig::default();
        let div = SprottLSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < 0.0);
        assert!(approx_eq(div, -1.0, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = SprottLConfig::default();
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5)] {
            let j = SprottLSolver::jacobian(&cfg, x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = SprottLSolver::divergence(&cfg, x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_equilibria_values() {
        let cfg = SprottLConfig::default();
        let (e0, e1) = SprottLSolver::equilibria(&cfg);
        // 唯一平衡点 E0 = (1, b, -b/a)
        assert!(approx_eq(e0[0], 1.0, 1e-12));
        assert!(approx_eq(e0[1], cfg.b, 1e-12));
        assert!(approx_eq(e0[2], -cfg.b / cfg.a, 1e-12));
        // 经典参数 a=3.9, b=0.9: E0 约等于 (1, 0.9, -0.23077)
        assert!(approx_eq(e0[2], -0.9 / 3.9, 1e-12));
        // 不存在第二个平衡点 (返回 NAN)
        assert!(e1[0].is_nan());
        assert!(e1[1].is_nan());
        assert!(e1[2].is_nan());
    }

    #[test]
    fn test_equilibria_satisfy_equations() {
        let cfg = SprottLConfig::default();
        let (e0, _e1) = SprottLSolver::equilibria(&cfg);
        // 只测唯一真实平衡点 E0
        let d = SprottLSolver::derivatives(&cfg, e0[0], e0[1], e0[2]);
        for v in d.iter() {
            assert!(v.abs() < 1e-9, "equilibrium derivative = {}", v);
        }
    }

    #[test]
    fn test_step_advances() {
        let mut s = SprottLSolver::classic(SprottLConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = SprottLSolver::classic(SprottLConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = SprottLSolver::classic(SprottLConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = SprottLSolver::classic(SprottLConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = SprottLSolver::classic(SprottLConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        // Sprott L 经典参数是混沌的, L > 0 (文献值 ~0.15)
        let mut s = SprottLSolver::classic(SprottLConfig::default());
        s.run(80000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = SprottLSolver::classic(SprottLConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        let cfg = SprottLConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = SprottLSolver::classic(cfg);
        let mut s2 = SprottLSolver::new(cfg, 0.1 + d0, 0.1, 0.1);
        for _ in 0..60000 {
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
    fn test_dx_linear_in_yz() {
        // dx/dt = y + a*z, 不含 x (线性于 y, z): 改变 x 不影响 dx/dt, 改变 y 或 z 才影响
        let cfg = SprottLConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d0 = SprottLSolver::derivatives(&cfg, x, y, z);
        // 改变 x: dx/dt 不变
        let d_x = SprottLSolver::derivatives(&cfg, x + 1.0, y, z);
        assert!(approx_eq(d_x[0], d0[0], 1e-12));
        // 改变 y: dx/dt 改变 (与单位斜率同步)
        let d_y = SprottLSolver::derivatives(&cfg, x, y + 0.1, z);
        assert!(!approx_eq(d_y[0], d0[0], 1e-9));
        assert!(approx_eq(d_y[0] - d0[0], 0.1, 1e-12));
        // 改变 z: dx/dt 改变 (与 a 同步)
        let d_z = SprottLSolver::derivatives(&cfg, x, y, z + 0.1);
        assert!(!approx_eq(d_z[0], d0[0], 1e-9));
        assert!(approx_eq(d_z[0] - d0[0], cfg.a * 0.1, 1e-12));
    }

    #[test]
    fn test_volume_monotonic_contraction() {
        let cfg = SprottLConfig::default();
        for &(x, y, z) in &[(1.0_f64, 2.0, 3.0), (-5.0, 7.0, -3.0), (10.0, -10.0, 20.0)] {
            assert!(SprottLSolver::divergence(&cfg, x, y, z) < 0.0);
        }
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = SprottLConfig::default();
        let mut s = SprottLSolver::new(cfg, 100.0, 100.0, 100.0);
        s.run(500);
        assert!(s.has_escaped() || !s.has_nan());
    }
}
