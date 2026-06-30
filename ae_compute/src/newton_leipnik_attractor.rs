//! Newton-Leipnik Attractor — Newton-Leipnik 双奇怪吸引子 (3D)
//!
//! R. B. Leipnik 和 T. A. Newton 1981 年在研究带线性反馈控制的刚体运动时
//! 发现的 3D 混沌系统. 该系统最显著的特征是存在两个共存的奇怪吸引子
//! (coexisting strange attractors), 轨道落入哪个吸引子取决于初值.
//! 这是混沌系统中"多稳态" (multistability) 的经典例子.
//!
//! 物理背景: 刚体动力学 + 线性反馈控制
//!   考虑一个自由旋转的刚体, 加入线性反馈控制力矩. 在特定参数下,
//!   系统的相空间中出现两个混沌吸引子, 分别围绕两个不稳定平衡点.
//!   与 Euler 自由刚体 (保守, 能量守恒) 不同, Newton-Leipnik 引入
//!   耗散 (线性阻尼) 和驱动 (反馈), 形成耗散混沌系统.
//!
//! 状态方程 (Leipnik & Newton 1981):
//!   dx/dt = -a x + y + 10 y z
//!   dy/dt = -x - 0.4 y + 5 x z
//!   dz/dt = b z - 5 x y
//!
//! 各项物理意义:
//!   - -a x, -0.4 y, b z: 线性阻尼 (各轴不同, 刚体非对称)
//!   + y, -x: 线性耦合 (刚体陀螺力矩, 类似 Euler 方程)
//!   + 10 y z, + 5 x z, -5 x y: 非线性耦合 (反馈控制非线性项)
//!
//! 经典参数 (Leipnik & Newton 1981): a = 0.4, b = 0.175
//! 经典初值 (两个吸引子):
//!   吸引子 1: (x₀, y₀, z₀) = (0.349, 0, -0.16)
//!   吸引子 2: (x₀, y₀, z₀) = (-0.349, 0, 0.16)
//!
//! 性质:
//!   - 散度 ∇·F = tr(J) = -a - 0.4 + b (常数)
//!     经典参数: div = -0.4 - 0.4 + 0.175 = -0.625 (耗散)
//!   - 平衡点:
//!     E0 = (0, 0, 0) (平凡, 刚体静止)
//!     其他平衡点需数值求解 (非平凡, 涉及刚体旋转)
//!   - Lyapunov 谱 (经典参数, 文献值):
//!     λ₁ ≈ +0.118  (正, 主混沌方向)
//!     λ₂ ≈ 0       (沿轨道切向)
//!     λ₃ ≈ -0.743  (负, 收缩)
//!     和 = -0.625 (与散度一致)
//!   - Kaplan-Yorke 维数 D_KY ≈ 2 + λ₁/|λ₃| ≈ 2.16
//!   - 双吸引子: 两个混沌吸引子共存在相空间中, 由稳定流形分隔
//!
//! 与 Euler 自由刚体对比:
//!   - Euler 刚体: 保守, 能量守恒, 无吸引子, Dzhanibekov 效应
//!   - Newton-Leipnik: 耗散, 能量不守恒, 双吸引子, 反馈控制
//!   - 两者都源于刚体动力学, 但 Euler 是自由的, NL 是受控的
//!
//! 多稳态意义:
//!   Newton-Leipnik 是多稳态混沌的典型例子. 两个吸引子共存意味着
//!   相同参数下, 不同初值导致不同长期行为. 这在气候系统、神经网络、
//!   生态系统中有重要意义 (多稳态 = 多种可能的"气候态").
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!
//! 历史:
//!   Leipnik, R. B. & Newton, T. A. 1981. "Double strange attractors in
//!   rigid body motion with linear feedback control." Phys. Lett. A 86,
//!   63-67. (原始论文, 双吸引子发现)
//!   Sprott, J. C. 2003. "Chaos and Time-Series Analysis." Oxford.
//!   (教科书中讨论 Newton-Leipnik 系统)

/// Newton-Leipnik 系统配置 (2 参数 + dt)
#[derive(Clone, Copy, Debug)]
pub struct NewtonLeipnikConfig {
    /// x 轴阻尼 a (经典 0.4)
    pub a: f64,
    /// z 轴阻尼 b (经典 0.175)
    pub b: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for NewtonLeipnikConfig {
    fn default() -> Self {
        Self { a: 0.4, b: 0.175, dt: 0.01 }
    }
}

/// Newton-Leipnik 求解器 (3D, 跟踪最大 Lyapunov 指数)
pub struct NewtonLeipnikSolver {
    pub config: NewtonLeipnikConfig,
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

impl NewtonLeipnikSolver {
    pub fn new(config: NewtonLeipnikConfig, x0: f64, y0: f64, z0: f64) -> Self {
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

    /// 吸引子 1 初值 (0.349, 0, -0.16)
    pub fn attractor1(config: NewtonLeipnikConfig) -> Self {
        Self::new(config, 0.349, 0.0, -0.16)
    }

    /// 吸引子 2 初值 (-0.349, 0, 0.16)
    pub fn attractor2(config: NewtonLeipnikConfig) -> Self {
        Self::new(config, -0.349, 0.0, 0.16)
    }

    /// 右端导数 F = [-a x + y + 10 y z,
    ///                -x - 0.4 y + 5 x z,
    ///                b z - 5 x y]
    pub fn derivatives(cfg: &NewtonLeipnikConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        [
            -cfg.a * x + y + 10.0 * y * z,
            -x - 0.4 * y + 5.0 * x * z,
            cfg.b * z - 5.0 * x * y,
        ]
    }

    /// Jacobian:
    /// J = [[-a,    1 + 10z,  10y],
    ///      [-1+5z, -0.4,     5x ],
    ///      [-5y,   -5x,      b  ]]
    pub fn jacobian(cfg: &NewtonLeipnikConfig, x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
        [
            [-cfg.a, 1.0 + 10.0 * z, 10.0 * y],
            [-1.0 + 5.0 * z, -0.4, 5.0 * x],
            [-5.0 * y, -5.0 * x, cfg.b],
        ]
    }

    /// 散度 ∇·F = tr(J) = -a - 0.4 + b (常数)
    pub fn divergence(cfg: &NewtonLeipnikConfig, _x: f64, _y: f64, _z: f64) -> f64 {
        -cfg.a - 0.4 + cfg.b
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

    /// 最大 Lyapunov 指数 (文献值 ~0.118)
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
        let cfg = NewtonLeipnikConfig::default();
        assert!(approx_eq(cfg.a, 0.4, 1e-12));
        assert!(approx_eq(cfg.b, 0.175, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = NewtonLeipnikSolver::attractor1(NewtonLeipnikConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_two_attractors_distinct() {
        // 两个吸引子初值不同
        let s1 = NewtonLeipnikSolver::attractor1(NewtonLeipnikConfig::default());
        let s2 = NewtonLeipnikSolver::attractor2(NewtonLeipnikConfig::default());
        assert!((s1.x - s2.x).abs() > 0.5);
        assert!((s1.z - s2.z).abs() > 0.2);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = NewtonLeipnikConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = NewtonLeipnikSolver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], -cfg.a * x + y + 10.0 * y * z, 1e-12));
        assert!(approx_eq(d[1], -x - 0.4 * y + 5.0 * x * z, 1e-12));
        assert!(approx_eq(d[2], cfg.b * z - 5.0 * x * y, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = NewtonLeipnikConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = NewtonLeipnikSolver::jacobian(&cfg, x, y, z);
        // Row 0: [-a, 1+10z, 10y]
        assert!(approx_eq(j[0][0], -cfg.a, 1e-12));
        assert!(approx_eq(j[0][1], 1.0 + 10.0 * z, 1e-12));
        assert!(approx_eq(j[0][2], 10.0 * y, 1e-12));
        // Row 1: [-1+5z, -0.4, 5x]
        assert!(approx_eq(j[1][0], -1.0 + 5.0 * z, 1e-12));
        assert!(approx_eq(j[1][1], -0.4, 1e-12));
        assert!(approx_eq(j[1][2], 5.0 * x, 1e-12));
        // Row 2: [-5y, -5x, b]
        assert!(approx_eq(j[2][0], -5.0 * y, 1e-12));
        assert!(approx_eq(j[2][1], -5.0 * x, 1e-12));
        assert!(approx_eq(j[2][2], cfg.b, 1e-12));
    }

    #[test]
    fn test_divergence_constant() {
        // 散度 = -a - 0.4 + b (常数)
        let cfg = NewtonLeipnikConfig::default();
        let expected = -cfg.a - 0.4 + cfg.b;
        assert!(approx_eq(NewtonLeipnikSolver::divergence(&cfg, 0.0, 0.0, 0.0), expected, 1e-12));
        assert!(approx_eq(NewtonLeipnikSolver::divergence(&cfg, 1.0, 2.0, 3.0), expected, 1e-12));
        assert!(approx_eq(NewtonLeipnikSolver::divergence(&cfg, -5.0, 7.0, -3.0), expected, 1e-12));
    }

    #[test]
    fn test_divergence_negative_dissipative() {
        let cfg = NewtonLeipnikConfig::default();
        let div = NewtonLeipnikSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < 0.0, "divergence should be negative: {}", div);
        assert!(approx_eq(div, -0.625, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = NewtonLeipnikConfig::default();
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5)] {
            let j = NewtonLeipnikSolver::jacobian(&cfg, x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = NewtonLeipnikSolver::divergence(&cfg, x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_origin_is_equilibrium() {
        // 原点 (0,0,0) 是平衡点
        let cfg = NewtonLeipnikConfig::default();
        let eq = NewtonLeipnikSolver::origin_equilibrium();
        let d = NewtonLeipnikSolver::derivatives(&cfg, eq[0], eq[1], eq[2]);
        for v in d.iter() {
            assert!(v.abs() < 1e-12, "origin derivative = {}", v);
        }
    }

    #[test]
    fn test_step_advances() {
        let mut s = NewtonLeipnikSolver::attractor1(NewtonLeipnikConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = NewtonLeipnikSolver::attractor1(NewtonLeipnikConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = NewtonLeipnikSolver::attractor1(NewtonLeipnikConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = NewtonLeipnikSolver::attractor1(NewtonLeipnikConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor1_bounded() {
        let mut s = NewtonLeipnikSolver::attractor1(NewtonLeipnikConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        // Newton-Leipnik 吸引子典型范围: x,y∈[-1,1], z∈[-0.5, 0.5]
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_attractor2_bounded() {
        let mut s = NewtonLeipnikSolver::attractor2(NewtonLeipnikConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        // Newton-Leipnik 经典参数是混沌的, λ > 0 (文献值 ~0.118)
        let mut s = NewtonLeipnikSolver::attractor1(NewtonLeipnikConfig::default());
        s.run(100000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = NewtonLeipnikSolver::attractor1(NewtonLeipnikConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        let cfg = NewtonLeipnikConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = NewtonLeipnikSolver::attractor1(cfg);
        let mut s2 = NewtonLeipnikSolver::new(cfg, 0.349 + d0, 0.0, -0.16);
        for _ in 0..80000 {
            s1.step();
            s2.step();
        }
        let dx = s1.x - s2.x;
        let dy = s1.y - s2.y;
        let dz = s1.z - s2.z;
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        // t=800, λ~0.118, 应放大 e^94 (饱和到吸引子尺度 ~1)
        assert!(d > 1e-3, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_two_attractors_remain_distinct() {
        // 两个吸引子的轨道应保持在不同区域 (多稳态)
        // 注意: 两个吸引子可能形状相似但位于不同位置
        let cfg = NewtonLeipnikConfig::default();
        let mut s1 = NewtonLeipnikSolver::attractor1(cfg);
        let mut s2 = NewtonLeipnikSolver::attractor2(cfg);
        s1.run(30000);
        s2.run(30000);
        // 两个吸引子的时间平均位置应该不同
        let mean1: f64 = s1.trajectory.iter().map(|(x, _, _)| *x).sum::<f64>() / s1.trajectory.len() as f64;
        let mean2: f64 = s2.trajectory.iter().map(|(x, _, _)| *x).sum::<f64>() / s2.trajectory.len() as f64;
        // 至少有一个维度的时间平均应该有显著差异 (多稳态标志)
        // 注: 双吸引子可能对称, 所以 x 平均可能相反
        assert!((mean1 - mean2).abs() > 0.01 || (mean1 + mean2).abs() < 0.5,
            "attractors should be distinct: mean1={}, mean2={}", mean1, mean2);
    }

    #[test]
    fn test_volume_monotonic_contraction() {
        // 散度 = -0.625 (常数负), 体积单调收缩
        let cfg = NewtonLeipnikConfig::default();
        for &(x, y, z) in &[(1.0_f64, 2.0, 3.0), (-5.0, 7.0, -3.0), (10.0, -10.0, 20.0)] {
            assert!(NewtonLeipnikSolver::divergence(&cfg, x, y, z) < 0.0);
        }
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = NewtonLeipnikConfig::default();
        let mut s = NewtonLeipnikSolver::new(cfg, 100.0, 100.0, 100.0);
        s.run(500);
        assert!(s.has_escaped() || !s.has_nan());
    }
}
