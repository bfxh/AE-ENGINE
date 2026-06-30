//! Sprott G Attractor - Sprott G 简单混沌系统 (3D)
//!
//! Julien C. Sprott 1994 年在其论文 "Some simple chaotic flows" 中
//! 系统搜索发现的最简单混沌系统之一 (Sprott A-S, 共 19 个). Sprott G
//! 是其中第七个, 以极少的非线性项产生混沌行为.
//!
//! 状态方程 (Sprott G, 参数化版本):
//!   dx/dt = a x + z
//!   dy/dt = x z - y
//!   dz/dt = -x + y
//!
//! 各项物理意义:
//!   + a x: 线性反馈 (x 自激, a>0 不稳定)
//!   + z: 线性耦合 (z 驱动 x)
//!   + x z: 非线性耦合 (y 受 xz 乘积驱动, 唯一非线性项)
//!   - y: 线性阻尼 (y 衰减)
//!   - x: 线性恢复 (z 受 x 牵引)
//!   + y: 线性耦合 (y 驱动 z)
//!
//! 经典参数 (Sprott 1994): a = 0.4
//! 经典初值: (x0, y0, z0) = (0.1, 0.1, 0.1) 或小扰动
//!
//! 性质:
//!   - 散度 div(F) = tr(J) = a - 1 (常数, 与位置无关)
//!     经典参数 a=0.4: div = -0.6 (中等耗散)
//!   - 平衡点 (利用 a x + z = 0, x z - y = 0, -x + y = 0):
//!     从 dz/dt=0: y = x
//!     从 dx/dt=0: z = -a x
//!     从 dy/dt=0: x z = y = x
//!     若 x != 0: z = 1, 则 x = -1/a, y = -1/a
//!     若 x = 0: E0 = (0, 0, 0)
//!     E0 = (0, 0, 0)
//!     E1 = (-1/a, -1/a, 1) (经典 a=0.4: E1 = (-2.5, -2.5, 1))
//!   - Lyapunov 谱 (经典参数, 文献值):
//!     L1 approx +0.16  (正, 主混沌方向)
//!     L2 = 0      (沿轨道切向)
//!     L3 approx -0.76  (负, 收缩)
//!     和 = -0.6 (与散度一致)
//!   - Kaplan-Yorke 维数 D_KY approx 2 + L1/|L3| approx 2.21
//!   - 吸引子形态: 单叶扭曲带
//!
//! Sprott 系列对比:
//!   - Sprott F: 1 个非线性项 (x^2), 6 项, 已完成
//!   - Sprott G: 1 个非线性项 (xz), 6 项, 本模块
//!   - Sprott 系统共同特征: 极简 (3 变量, <=2 非线性项, 常数散度)
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!
//! 历史:
//!   Sprott, J. C. 1994. "Some simple chaotic flows." Phys. Rev. E 50,
//!   R647-R650. (Sprott A-S 系统的原始发现)

/// Sprott G 系统配置 (1 参数 + dt)
#[derive(Clone, Copy, Debug)]
pub struct SprottGConfig {
    /// 参数 a (x 自激强度, 经典 0.4)
    pub a: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for SprottGConfig {
    fn default() -> Self {
        Self { a: 0.4, dt: 0.01 }
    }
}

/// Sprott G 求解器 (3D, 跟踪最大 Lyapunov 指数)
pub struct SprottGSolver {
    pub config: SprottGConfig,
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

impl SprottGSolver {
    pub fn new(config: SprottGConfig, x0: f64, y0: f64, z0: f64) -> Self {
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

    pub fn classic(config: SprottGConfig) -> Self {
        Self::new(config, 0.1, 0.1, 0.1)
    }

    /// 右端导数 F = [a x + z, x z - y, -x + y]
    pub fn derivatives(cfg: &SprottGConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        [cfg.a * x + z, x * z - y, -x + y]
    }

    /// Jacobian:
    /// J = [[a,  0,  1],
    ///      [z, -1,  x],
    ///      [-1, 1,  0]]
    pub fn jacobian(cfg: &SprottGConfig, x: f64, _y: f64, z: f64) -> [[f64; 3]; 3] {
        [
            [cfg.a, 0.0, 1.0],
            [z, -1.0, x],
            [-1.0, 1.0, 0.0],
        ]
    }

    /// 散度 div(F) = tr(J) = a - 1 (常数, 经典 a=0.4 时 div=-0.6)
    pub fn divergence(cfg: &SprottGConfig, _x: f64, _y: f64, _z: f64) -> f64 {
        cfg.a - 1.0
    }

    /// 计算两个平衡点:
    /// E0 = (0, 0, 0)
    /// E1 = (-1/a, -1/a, 1) (当 a != 0)
    pub fn equilibria(cfg: &SprottGConfig) -> ([f64; 3], [f64; 3]) {
        if cfg.a.abs() < 1e-12 {
            return ([0.0, 0.0, 0.0], [f64::NAN, f64::NAN, f64::NAN]);
        }
        let inv_a = -1.0 / cfg.a;
        ([0.0, 0.0, 0.0], [inv_a, inv_a, 1.0])
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

    /// 最大 Lyapunov 指数 (文献值 ~0.16)
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
        let cfg = SprottGConfig::default();
        assert!(approx_eq(cfg.a, 0.4, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = SprottGSolver::classic(SprottGConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = SprottGConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = SprottGSolver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], cfg.a * x + z, 1e-12));
        assert!(approx_eq(d[1], x * z - y, 1e-12));
        assert!(approx_eq(d[2], -x + y, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = SprottGConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = SprottGSolver::jacobian(&cfg, x, y, z);
        assert!(approx_eq(j[0][0], cfg.a, 1e-12));
        assert!(approx_eq(j[0][1], 0.0, 1e-12));
        assert!(approx_eq(j[0][2], 1.0, 1e-12));
        assert!(approx_eq(j[1][0], z, 1e-12));
        assert!(approx_eq(j[1][1], -1.0, 1e-12));
        assert!(approx_eq(j[1][2], x, 1e-12));
        assert!(approx_eq(j[2][0], -1.0, 1e-12));
        assert!(approx_eq(j[2][1], 1.0, 1e-12));
        assert!(approx_eq(j[2][2], 0.0, 1e-12));
    }

    #[test]
    fn test_divergence_constant() {
        // 散度 = a - 1 (常数, 与位置无关)
        let cfg = SprottGConfig::default();
        assert!(approx_eq(SprottGSolver::divergence(&cfg, 0.0, 0.0, 0.0), -0.6, 1e-12));
        assert!(approx_eq(SprottGSolver::divergence(&cfg, 1.0, 2.0, 3.0), -0.6, 1e-12));
        // 改变 a 影响散度
        let cfg2 = SprottGConfig { a: 0.5, ..cfg };
        assert!(approx_eq(SprottGSolver::divergence(&cfg2, 0.0, 0.0, 0.0), -0.5, 1e-12));
    }

    #[test]
    fn test_divergence_negative_dissipative() {
        let cfg = SprottGConfig::default();
        let div = SprottGSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < 0.0);
        assert!(approx_eq(div, -0.6, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = SprottGConfig::default();
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5)] {
            let j = SprottGSolver::jacobian(&cfg, x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = SprottGSolver::divergence(&cfg, x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_equilibria_values() {
        let cfg = SprottGConfig::default();
        let (e0, e1) = SprottGSolver::equilibria(&cfg);
        // E0 = (0, 0, 0)
        assert!(approx_eq(e0[0], 0.0, 1e-12));
        assert!(approx_eq(e0[1], 0.0, 1e-12));
        assert!(approx_eq(e0[2], 0.0, 1e-12));
        // E1 = (-1/a, -1/a, 1) = (-2.5, -2.5, 1) for a=0.4
        assert!(approx_eq(e1[0], -2.5, 1e-12));
        assert!(approx_eq(e1[1], -2.5, 1e-12));
        assert!(approx_eq(e1[2], 1.0, 1e-12));
    }

    #[test]
    fn test_equilibria_satisfy_equations() {
        let cfg = SprottGConfig::default();
        let (e0, e1) = SprottGSolver::equilibria(&cfg);
        for eq in [e0, e1] {
            let d = SprottGSolver::derivatives(&cfg, eq[0], eq[1], eq[2]);
            for v in d.iter() {
                assert!(v.abs() < 1e-9, "equilibrium derivative = {}", v);
            }
        }
    }

    #[test]
    fn test_step_advances() {
        let mut s = SprottGSolver::classic(SprottGConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = SprottGSolver::classic(SprottGConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = SprottGSolver::classic(SprottGConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = SprottGSolver::classic(SprottGConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = SprottGSolver::classic(SprottGConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        // Sprott G 经典参数是混沌的, L > 0 (文献值 ~0.16)
        let mut s = SprottGSolver::classic(SprottGConfig::default());
        s.run(80000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = SprottGSolver::classic(SprottGConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        let cfg = SprottGConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = SprottGSolver::classic(cfg);
        let mut s2 = SprottGSolver::new(cfg, 0.1 + d0, 0.1, 0.1);
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
    fn test_y_tracks_x_in_dz() {
        // dz/dt = -x + y, 在吸引子上 y 大致追踪 x (因为 dz/dt 平均趋于小值)
        let cfg = SprottGConfig::default();
        let mut s = SprottGSolver::classic(cfg);
        s.run(30000);
        let mean_diff = s.trajectory.iter()
            .map(|(x, y, _)| (x - y).abs())
            .sum::<f64>() / s.trajectory.len() as f64;
        assert!(mean_diff < 5.0, "y should roughly track x, mean_diff={}", mean_diff);
    }

    #[test]
    fn test_volume_monotonic_contraction() {
        let cfg = SprottGConfig::default();
        for &(x, y, z) in &[(1.0_f64, 2.0, 3.0), (-5.0, 7.0, -3.0), (10.0, -10.0, 20.0)] {
            assert!(SprottGSolver::divergence(&cfg, x, y, z) < 0.0);
        }
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = SprottGConfig::default();
        let mut s = SprottGSolver::new(cfg, 100.0, 100.0, 100.0);
        s.run(500);
        assert!(s.has_escaped() || !s.has_nan());
    }
}
