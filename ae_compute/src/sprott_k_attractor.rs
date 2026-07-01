//! Sprott K Attractor - Sprott K 简单混沌系统 (3D)
//!
//! Julien C. Sprott 1994 年在其论文 "Some simple chaotic flows" 中
//! 系统搜索发现的最简单混沌系统之一 (Sprott A-S, 共 19 个). Sprott K
//! 是其中第十一个, 特点是散度非常数 (位置依赖耗散), 且有两个平衡点.
//!
//! 状态方程 (Sprott K, 参数化版本):
//!   dx/dt = x*y - z
//!   dy/dt = x - y
//!   dz/dt = x + a*z
//!
//! 各项物理意义:
//!   + x*y: 非线性耦合 (x 受 xy 乘积驱动, 唯一非线性项)
//!   - z: 线性耦合 (x 受 -z 驱动, 回归反馈)
//!   + x: 线性耦合 (y 受 x 驱动)
//!   - y: 线性阻尼 (y 衰减)
//!   + x: 线性耦合 (z 受 x 驱动)
//!   + a*z: 线性反馈 (z 受自身调制, a 控制反馈率)
//!
//! 经典参数 (Sprott 1994): a = 0.3
//! 经典初值: (x0, y0, z0) = (0.1, 0.1, 0.1)
//!
//! 性质:
//!   - 散度 div(F) = tr(J) = y + a - 1 (非常数! 依赖 y 位置)
//!     * 在原点: div(0,0,0) = a - 1 = -0.7 (经典 a=0.3, 耗散)
//!     * y 大于 1-a = 0.7: div 大于 0 (局部体积膨胀)
//!     * y 小于 1-a = 0.7: div 小于 0 (局部体积收缩)
//!     * 平均散度 小于 0 (整体耗散, 吸引子存在)
//!     * (与 Sprott J 散度恒为 -a 常数不同, Sprott K 散度非常数, 类似 Sprott D)
//!   - 平衡点 (利用 xy-z=0, x-y=0, x+az=0):
//!     从 dy/dt=0: x = y
//!     从 dx/dt=0: z = x*y = x^2
//!     从 dz/dt=0: x = -a*z = -a*x^2
//!     x(1 + a*x) = 0 等价于 x = 0 或 x = -1/a
//!     两个平衡点:
//!       E0 = (0, 0, 0)
//!       E1 = (-1/a, -1/a, 1/a^2)
//!     (经典 a=0.3: E1 约等于 (-3.333, -3.333, 11.111))
//!     (与 Sprott J 只有一个平衡点不同, Sprott K 有两个)
//!   - Lyapunov 谱 (经典参数, 文献值):
//!     L1 约等于 +0.14  (正, 主混沌方向)
//!     L2 = 0      (沿轨道切向)
//!     L3 约等于 负值   (收缩方向, 和为平均散度)
//!   - Kaplan-Yorke 维数 D_KY 约等于 2 + L1/|L3|
//!   - 吸引子形态: 单叶扭曲带
//!
//! Sprott 系列对比 (已实现):
//!   - Sprott G: 1 个非线性项 (xz), 6 项, 已完成
//!   - Sprott H: 1 个非线性项 (z^2), 5 项, 已完成
//!   - Sprott I: 1 个非线性项 (y^2), 6 项, 已完成 (散度恒为 -1, 与 a 无关)
//!   - Sprott J: 1 个非线性项 (y^2), 6 项, 已完成 (散度 = -a, 依赖 a, 常数)
//!   - Sprott K: 1 个非线性项 (xy), 6 项, 本模块 (散度 = y+a-1, 非常数!)
//!   - Sprott 系统共同特征: 极简 (3 变量, 不超过 2 非线性项)
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!
//! 历史:
//!   Sprott, J. C. 1994. "Some simple chaotic flows." Phys. Rev. E 50,
//!   R647-R650. (Sprott A-S 系统的原始发现)

/// Sprott K 系统配置 (1 参数 + dt)
#[derive(Clone, Copy, Debug)]
pub struct SprottKConfig {
    /// 参数 a (z 反馈率, 经典 0.3)
    pub a: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for SprottKConfig {
    fn default() -> Self {
        Self { a: 0.3, dt: 0.01 }
    }
}

/// Sprott K 求解器 (3D, 跟踪最大 Lyapunov 指数)
pub struct SprottKSolver {
    pub config: SprottKConfig,
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

impl SprottKSolver {
    pub fn new(config: SprottKConfig, x0: f64, y0: f64, z0: f64) -> Self {
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

    pub fn classic(config: SprottKConfig) -> Self {
        Self::new(config, 0.1, 0.1, 0.1)
    }

    /// 右端导数 F = [x*y - z, x - y, x + a*z]
    pub fn derivatives(cfg: &SprottKConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        [x * y - z, x - y, x + cfg.a * z]
    }

    /// Jacobian:
    /// J = [[y,   x,  -1],
    ///      [1,  -1,   0],
    ///      [1,   0,   a]]
    pub fn jacobian(cfg: &SprottKConfig, x: f64, y: f64, _z: f64) -> [[f64; 3]; 3] {
        [
            [y, x, -1.0],
            [1.0, -1.0, 0.0],
            [1.0, 0.0, cfg.a],
        ]
    }

    /// 散度 div(F) = tr(J) = y + a - 1 (非常数! 依赖 y 位置)
    pub fn divergence(cfg: &SprottKConfig, _x: f64, y: f64, _z: f64) -> f64 {
        y + cfg.a - 1.0
    }

    /// 计算平衡点:
    /// E0 = (0, 0, 0)
    /// E1 = (-1/a, -1/a, 1/a^2)
    /// (当 a = 0 时不存在 E1, 返回 NAN)
    pub fn equilibria(cfg: &SprottKConfig) -> ([f64; 3], [f64; 3]) {
        let a = cfg.a;
        if a.abs() < 1e-12 {
            ([0.0, 0.0, 0.0], [f64::NAN, f64::NAN, f64::NAN])
        } else {
            let inv_a = -1.0 / a;
            let z1 = 1.0 / (a * a);
            ([0.0, 0.0, 0.0], [inv_a, inv_a, z1])
        }
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

    /// 最大 Lyapunov 指数 (文献值 ~0.14)
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
        let cfg = SprottKConfig::default();
        assert!(approx_eq(cfg.a, 0.3, 1e-12));
        assert!(approx_eq(cfg.dt, 0.01, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = SprottKSolver::classic(SprottKConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = SprottKConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = SprottKSolver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], x * y - z, 1e-12));
        assert!(approx_eq(d[1], x - y, 1e-12));
        assert!(approx_eq(d[2], x + cfg.a * z, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = SprottKConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = SprottKSolver::jacobian(&cfg, x, y, z);
        // J = [[y, x, -1], [1, -1, 0], [1, 0, a]]
        assert!(approx_eq(j[0][0], y, 1e-12));
        assert!(approx_eq(j[0][1], x, 1e-12));
        assert!(approx_eq(j[0][2], -1.0, 1e-12));
        assert!(approx_eq(j[1][0], 1.0, 1e-12));
        assert!(approx_eq(j[1][1], -1.0, 1e-12));
        assert!(approx_eq(j[1][2], 0.0, 1e-12));
        assert!(approx_eq(j[2][0], 1.0, 1e-12));
        assert!(approx_eq(j[2][1], 0.0, 1e-12));
        assert!(approx_eq(j[2][2], cfg.a, 1e-12));
    }

    #[test]
    fn test_divergence_at_origin() {
        // div(0,0,0) = 0 + a - 1 = a - 1 = -0.7 (经典 a=0.3)
        let cfg = SprottKConfig::default();
        let div = SprottKSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(approx_eq(div, cfg.a - 1.0, 1e-12));
        assert!(approx_eq(div, -0.7, 1e-12));
    }

    #[test]
    fn test_divergence_formula() {
        // div(x,y,z) = y + a - 1 (非常数, 依赖 y)
        let cfg = SprottKConfig::default();
        assert!(approx_eq(SprottKSolver::divergence(&cfg, 0.3, 0.5, 0.7), 0.5 + cfg.a - 1.0, 1e-12));
        assert!(approx_eq(SprottKSolver::divergence(&cfg, 1.0, 2.0, 3.0), 2.0 + cfg.a - 1.0, 1e-12));
        assert!(approx_eq(SprottKSolver::divergence(&cfg, -1.0, -2.0, 0.5), -2.0 + cfg.a - 1.0, 1e-12));
    }

    #[test]
    fn test_divergence_depends_on_y() {
        // 散度仅依赖于 y (不依赖 x, z)
        let cfg = SprottKConfig::default();
        let div1 = SprottKSolver::divergence(&cfg, 1.0, 2.0, 0.5);
        let div2 = SprottKSolver::divergence(&cfg, 1.0, 2.0, 7.0);
        assert!(approx_eq(div1, div2, 1e-12));
        let div3 = SprottKSolver::divergence(&cfg, -5.0, 2.0, 0.5);
        assert!(approx_eq(div1, div3, 1e-12));
        // 不同 y 不同散度
        let div4 = SprottKSolver::divergence(&cfg, 1.0, 3.0, 0.5);
        assert!(!approx_eq(div1, div4, 1e-9));
    }

    #[test]
    fn test_divergence_negative_dissipative_at_origin() {
        // 在原点 div = a - 1 = -0.7 小于 0 (耗散)
        let cfg = SprottKConfig::default();
        let div = SprottKSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < 0.0);
        assert!(approx_eq(div, -0.7, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = SprottKConfig::default();
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5), (2.0, -1.0, 0.3)] {
            let j = SprottKSolver::jacobian(&cfg, x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = SprottKSolver::divergence(&cfg, x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_equilibria_values() {
        let cfg = SprottKConfig::default();
        let (e0, e1) = SprottKSolver::equilibria(&cfg);
        // E0 = (0, 0, 0)
        assert!(approx_eq(e0[0], 0.0, 1e-12));
        assert!(approx_eq(e0[1], 0.0, 1e-12));
        assert!(approx_eq(e0[2], 0.0, 1e-12));
        // E1 = (-1/a, -1/a, 1/a^2)
        let a = cfg.a;
        assert!(approx_eq(e1[0], -1.0 / a, 1e-12));
        assert!(approx_eq(e1[1], -1.0 / a, 1e-12));
        assert!(approx_eq(e1[2], 1.0 / (a * a), 1e-12));
        // 经典 a=0.3: E1 约等于 (-3.333, -3.333, 11.111)
        assert!(approx_eq(e1[0], -3.3333333333333335, 1e-10));
        assert!(approx_eq(e1[2], 11.11111111111111, 1e-10));
    }

    #[test]
    fn test_equilibria_satisfy_equations() {
        let cfg = SprottKConfig::default();
        let (e0, e1) = SprottKSolver::equilibria(&cfg);
        // E0
        let d0 = SprottKSolver::derivatives(&cfg, e0[0], e0[1], e0[2]);
        for v in d0.iter() {
            assert!(v.abs() < 1e-12, "E0 derivative = {}", v);
        }
        // E1
        let d1 = SprottKSolver::derivatives(&cfg, e1[0], e1[1], e1[2]);
        for v in d1.iter() {
            assert!(v.abs() < 1e-12, "E1 derivative = {}", v);
        }
    }

    #[test]
    fn test_step_advances() {
        let mut s = SprottKSolver::classic(SprottKConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = SprottKSolver::classic(SprottKConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = SprottKSolver::classic(SprottKConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = SprottKSolver::classic(SprottKConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = SprottKSolver::classic(SprottKConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        // Sprott K 经典参数是混沌的, L 大于 0 (文献值 ~0.14)
        let mut s = SprottKSolver::classic(SprottKConfig::default());
        s.run(100000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = SprottKSolver::classic(SprottKConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        let cfg = SprottKConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = SprottKSolver::classic(cfg);
        let mut s2 = SprottKSolver::new(cfg, 0.1 + d0, 0.1, 0.1);
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
    fn test_escape_for_large_initial() {
        let cfg = SprottKConfig::default();
        let mut s = SprottKSolver::new(cfg, 100.0, 100.0, 100.0);
        s.run(500);
        assert!(s.has_escaped(), "should escape: x={} y={} z={}", s.x, s.y, s.z);
    }
}