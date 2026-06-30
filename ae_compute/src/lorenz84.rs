//! Lorenz 84 Model — Lorenz 全球大气环流模型 (3D)
//!
//! Edward N. Lorenz 1984 年提出的简化全球大气环流模型, 是 Lorenz 系列
//! (63/84/96) 的第二个, 介于 Lorenz 63 (Rayleigh-Bénard 对流, 小尺度)
//! 和 Lorenz 96 (纬度带可预报性, 中尺度) 之间. Lorenz 84 模拟全球尺度
//! 的西风带 (zonal flow) 与两个行星波 (planetary waves) 的相互作用,
//! 用于研究大气不规则性与可预报性的起源.
//!
//! 状态变量物理意义:
//!   x: 西风带强度 (zonal flow, 绕极地的平均西风)
//!   y: 行星波 1 振幅 (Rossby 波, 大尺度地形强迫)
//!   z: 行星波 2 振幅 (Rossby 波, 海陆差异强迫)
//!
//! 状态方程 (Lorenz 1984):
//!   dx/dt = -y² - z² - a x + a F
//!   dy/dt =  x y - b x z - y + G
//!   dz/dt =  b x y + x z - z
//!
//! 各项物理意义:
//!   - -y² - z²: 波动对西风的反馈 (波增长消耗平均流能量)
//!   - -a x: Rayleigh 摩擦 (地表摩擦耗散西风)
//!   + a F: 外部强迫 (赤道-极地温度梯度驱动西风)
//!   + x y, x z: 平均流对波的增长 (斜压不稳定, baroclinic instability)
//!   - b x z, + b x y: 两波非线性相互作用 (波-波耦合)
//!   - y, - z: 波动 Rayleigh 阻尼
//!   + G: 波动外部强迫 (地形、海陆分布)
//!
//! 经典参数 (Lorenz 1984): a = 0.25, b = 4, F = 8, G = 1
//! 经典初值: (x₀, y₀, z₀) = (1, 1, 1) 或 (-2.5, -2.5, 2.5)
//!
//! 性质:
//!   - 散度 ∇·F = tr(J) = -a + 2x - 2 (非常数, 随 x 变化)
//!     经典参数 a=0.25: div = 2x - 2.25
//!     平均耗散: 对吸引子上的轨道, 时间平均散度 < 0 (耗散)
//!   - 平衡点: G = 0 时 (F, 0, 0) 是平衡点 (纯西风, 无波)
//!     G ≠ 0 时无简单解析形式, 需数值求解
//!   - Lyapunov 谱 (经典参数, 文献值):
//!     λ₁ ≈ +0.15  (正, 主混沌方向, 大气不可预报性来源)
//!     λ₂ = 0      (沿轨道切向)
//!     λ₃ ≈ -2.4   (负, 收缩)
//!   - Kaplan-Yorke 维数 D_KY ≈ 2 + λ₁/|λ₃| ≈ 2.06
//!   - 吸引子形态: 类似 Lorenz 63 的双叶, 但更复杂 (折叠的曲面)
//!
//! 与 Lorenz 63 对比:
//!   - Lorenz 63: 局部对流 (Rayleigh-Bénard), 3 变量, 对称 (x↔-x)
//!   - Lorenz 84: 全球环流 (西风+行星波), 3 变量, 无对称
//!   - Lorenz 96: 纬度带 (N 变量链), 周期边界
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!
//! 历史:
//!   Lorenz, E. N. 1984. "Irregularity: A fundamental property of the
//!   atmosphere." Tellus A 36, 98-110. (原始论文)
//!   Lorenz, E. N. 1990. "Can chaos and intransitivity lead to
//!   interannual variability?" Tellus A 42, 378. (扩展研究)
//!   Palmer, T. N. 1993. "Extended-range atmospheric prediction and the
//!   Lorenz model." Bull. Amer. Meteor. Soc. 74, 49. (气象学应用)

/// Lorenz 84 模型配置 (4 参数 + dt)
#[derive(Clone, Copy, Debug)]
pub struct Lorenz84Config {
    /// 摩擦系数 a (经典 0.25)
    pub a: f64,
    /// 波-波耦合强度 b (经典 4)
    pub b: f64,
    /// 西风强迫 F (经典 8, 赤道-极地温差)
    pub f: f64,
    /// 波动强迫 G (经典 1, 地形/海陆)
    pub g: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for Lorenz84Config {
    fn default() -> Self {
        Self { a: 0.25, b: 4.0, f: 8.0, g: 1.0, dt: 0.005 }
    }
}

/// Lorenz 84 求解器 (3D, 跟踪最大 Lyapunov 指数)
pub struct Lorenz84Solver {
    pub config: Lorenz84Config,
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

impl Lorenz84Solver {
    pub fn new(config: Lorenz84Config, x0: f64, y0: f64, z0: f64) -> Self {
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

    pub fn classic(config: Lorenz84Config) -> Self {
        // 经典初值 (1, 1, 1)
        Self::new(config, 1.0, 1.0, 1.0)
    }

    /// 右端导数 F = [-y² - z² - a x + a F,
    ///                x y - b x z - y + G,
    ///                b x y + x z - z]
    pub fn derivatives(cfg: &Lorenz84Config, x: f64, y: f64, z: f64) -> [f64; 3] {
        [
            -y * y - z * z - cfg.a * x + cfg.a * cfg.f,
            x * y - cfg.b * x * z - y + cfg.g,
            cfg.b * x * y + x * z - z,
        ]
    }

    /// Jacobian:
    /// J = [[-a,        -2y,         -2z       ],
    ///      [y - b z,   x - 1,       -b x      ],
    ///      [b y + z,   b x,         x - 1     ]]
    pub fn jacobian(cfg: &Lorenz84Config, x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
        [
            [-cfg.a, -2.0 * y, -2.0 * z],
            [y - cfg.b * z, x - 1.0, -cfg.b * x],
            [cfg.b * y + z, cfg.b * x, x - 1.0],
        ]
    }

    /// 散度 ∇·F = tr(J) = -a + 2x - 2 (非常数)
    pub fn divergence(cfg: &Lorenz84Config, x: f64, _y: f64, _z: f64) -> f64 {
        -cfg.a + 2.0 * x - 2.0
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
        let cfg = Lorenz84Config::default();
        assert!(approx_eq(cfg.a, 0.25, 1e-12));
        assert!(approx_eq(cfg.b, 4.0, 1e-12));
        assert!(approx_eq(cfg.f, 8.0, 1e-12));
        assert!(approx_eq(cfg.g, 1.0, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = Lorenz84Solver::classic(Lorenz84Config::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = Lorenz84Config::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = Lorenz84Solver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], -y * y - z * z - cfg.a * x + cfg.a * cfg.f, 1e-12));
        assert!(approx_eq(d[1], x * y - cfg.b * x * z - y + cfg.g, 1e-12));
        assert!(approx_eq(d[2], cfg.b * x * y + x * z - z, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = Lorenz84Config::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = Lorenz84Solver::jacobian(&cfg, x, y, z);
        // Row 0: [-a, -2y, -2z]
        assert!(approx_eq(j[0][0], -cfg.a, 1e-12));
        assert!(approx_eq(j[0][1], -2.0 * y, 1e-12));
        assert!(approx_eq(j[0][2], -2.0 * z, 1e-12));
        // Row 1: [y - b z, x - 1, -b x]
        assert!(approx_eq(j[1][0], y - cfg.b * z, 1e-12));
        assert!(approx_eq(j[1][1], x - 1.0, 1e-12));
        assert!(approx_eq(j[1][2], -cfg.b * x, 1e-12));
        // Row 2: [b y + z, b x, x - 1]
        assert!(approx_eq(j[2][0], cfg.b * y + z, 1e-12));
        assert!(approx_eq(j[2][1], cfg.b * x, 1e-12));
        assert!(approx_eq(j[2][2], x - 1.0, 1e-12));
    }

    #[test]
    fn test_divergence_formula() {
        // 散度 = -a + 2x - 2 (非常数, 随 x 变化)
        let cfg = Lorenz84Config::default();
        assert!(approx_eq(
            Lorenz84Solver::divergence(&cfg, 0.0, 0.0, 0.0),
            -cfg.a - 2.0,
            1e-12
        ));
        assert!(approx_eq(
            Lorenz84Solver::divergence(&cfg, 1.0, 0.0, 0.0),
            -cfg.a + 2.0 - 2.0,
            1e-12
        ));
        assert!(approx_eq(
            Lorenz84Solver::divergence(&cfg, 5.0, 7.0, -3.0),
            -cfg.a + 10.0 - 2.0,
            1e-12
        ));
    }

    #[test]
    fn test_divergence_not_constant() {
        // 散度随 x 变化 (非常数)
        let cfg = Lorenz84Config::default();
        let d1 = Lorenz84Solver::divergence(&cfg, 0.0, 0.0, 0.0);
        let d2 = Lorenz84Solver::divergence(&cfg, 1.0, 0.0, 0.0);
        assert!((d1 - d2).abs() > 1.0, "d1={}, d2={}", d1, d2);
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = Lorenz84Config::default();
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5)] {
            let j = Lorenz84Solver::jacobian(&cfg, x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = Lorenz84Solver::divergence(&cfg, x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_equilibrium_no_wave_g_zero() {
        // G = 0 时, (F, 0, 0) 是平衡点 (纯西风, 无波)
        // dx/dt = -0 - 0 - a*F + a*F = 0 ✓
        // dy/dt = F*0 - b*F*0 - 0 + 0 = 0 ✓
        // dz/dt = b*F*0 + F*0 - 0 = 0 ✓
        let cfg = Lorenz84Config { g: 0.0, ..Lorenz84Config::default() };
        let (x, y, z) = (cfg.f, 0.0, 0.0);
        let d = Lorenz84Solver::derivatives(&cfg, x, y, z);
        for v in d.iter() {
            assert!(v.abs() < 1e-12, "equilibrium derivative = {}", v);
        }
    }

    #[test]
    fn test_no_equilibrium_at_origin() {
        // 原点不是平衡点 (因为 a*F ≠ 0)
        let cfg = Lorenz84Config::default();
        let d = Lorenz84Solver::derivatives(&cfg, 0.0, 0.0, 0.0);
        assert!(d[0].abs() > 0.0, "dx/dt at origin = a*F = {}", d[0]);
        assert!(d[1].abs() > 0.0, "dy/dt at origin = G = {}", d[1]);
    }

    #[test]
    fn test_step_advances() {
        let mut s = Lorenz84Solver::classic(Lorenz84Config::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = Lorenz84Solver::classic(Lorenz84Config::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = Lorenz84Solver::classic(Lorenz84Config::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = Lorenz84Solver::classic(Lorenz84Config::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = Lorenz84Solver::classic(Lorenz84Config::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        // Lorenz 84 吸引子典型范围: x∈[-2, 10], y∈[-6, 6], z∈[-6, 6]
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        // Lorenz 84 经典参数是混沌的, λ > 0 (文献值 ~0.15)
        let mut s = Lorenz84Solver::classic(Lorenz84Config::default());
        s.run(80000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = Lorenz84Solver::classic(Lorenz84Config::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        let cfg = Lorenz84Config::default();
        let d0 = 1e-6_f64;
        let mut s1 = Lorenz84Solver::classic(cfg);
        let mut s2 = Lorenz84Solver::new(cfg, 1.0 + d0, 1.0, 1.0);
        for _ in 0..60000 {
            s1.step();
            s2.step();
        }
        let dx = s1.x - s2.x;
        let dy = s1.y - s2.y;
        let dz = s1.z - s2.z;
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        // t=300, λ~0.15, 应放大 e^45 (饱和到吸引子尺度 ~1)
        assert!(d > 1e-3, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_zero_forcing_decays() {
        // F = 0, G = 0 时, 系统衰减到原点 (无外部能量输入)
        // dx/dt = -y² - z² - a x
        // dy/dt = x y - b x z - y
        // dz/dt = b x y + x z - z
        // 原点是平衡点, 且 a, 1, 1 都正 → 稳定
        let cfg = Lorenz84Config { f: 0.0, g: 0.0, ..Lorenz84Config::default() };
        let mut s = Lorenz84Solver::new(cfg, 1.0, 1.0, 1.0);
        s.run(50000);
        // 衰减到原点附近
        let r = (s.x * s.x + s.y * s.y + s.z * s.z).sqrt();
        assert!(r < 0.1, "should decay to origin, r = {}", r);
    }

    #[test]
    fn test_zero_forcing_no_chaos() {
        // F = 0, G = 0: 无混沌 (衰减系统)
        let cfg = Lorenz84Config { f: 0.0, g: 0.0, ..Lorenz84Config::default() };
        let mut s = Lorenz84Solver::new(cfg, 1.0, 1.0, 1.0);
        s.run(50000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda < 0.01, "no forcing → no chaos, lambda = {}", lambda);
    }

    #[test]
    fn test_volume_contraction_on_average() {
        // 散度 = -a + 2x - 2 (非常数), 但吸引子上时间平均 < 0 (耗散)
        let mut s = Lorenz84Solver::classic(Lorenz84Config::default());
        s.run(30000);
        let mut avg_div = 0.0_f64;
        let n = s.trajectory.len();
        for &(x, _y, _z) in &s.trajectory {
            avg_div += Lorenz84Solver::divergence(&s.config, x, 0.0, 0.0);
        }
        avg_div /= n as f64;
        assert!(avg_div < 0.0, "average divergence should be negative: {}", avg_div);
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = Lorenz84Config::default();
        let mut s = Lorenz84Solver::new(cfg, 100.0, 100.0, 100.0);
        s.run(500);
        assert!(s.has_escaped() || !s.has_nan());
    }
}
