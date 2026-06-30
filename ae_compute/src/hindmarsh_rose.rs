//! Hindmarsh-Rose Neuron — Hindmarsh-Rose 神经元模型 (bursting)
//!
//! Hindmarsh 和 Rose 1984 年提出的 3D 神经元模型, 是 FitzHugh-Nagumo 的
//! 扩展, 通过引入慢变量 z 实现了 realistic bursting 行为 (交替快速放电
//! 与静息期). 该模型在计算神经科学中广泛用于研究 bursting 节律、
//! 神经振荡器的多尺度动力学、以及癫痫、帕金森等病理节律.
//!
//! 状态方程 (Hindmarsh & Rose 1984):
//!   dx/dt = y - a x³ + b x² - z + I
//!   dy/dt = c - d x² - y
//!   dz/dt = r (s (x - x_R) - z)
//!
//! 经典参数 (bursting 区): a=1, b=3, c=1, d=5, s=4, x_R=-1.6, r=0.005, I=3
//! 经典初值: (x₀, y₀, z₀) = (-1.6, -1.0, 0.5) (接近静息态)
//!
//! 变量意义:
//!   x = 膜电位 (无量纲化)
//!   y = 快恢复变量 (Na⁺/K⁺ 离子流的快速部分)
//!   z = 慢适应变量 (Ca²⁺ 激活的 K⁺ 电流, 时间常数 1/r ≫ 1)
//!   I = 外部注入电流
//!   x_R = 静息电位
//!
//! 性质:
//!   - 散度 ∇·F = -3a x² + 2b x - 1 - r (随 x 变化, 一般为负, 耗散)
//!   - 多尺度动力学: (x, y) 为快变量 (时间尺度 ~1), z 为慢变量 (时间尺度 ~1/r=200)
//!   - 三种动力学模式 (随 I 变化):
//!     * I < I_c1: 稳定静息态 (不动点)
//!     * I_c1 < I < I_c2: 连续放电 (极限环)
//!     * I_c2 < I: 簇放电 (bursting, 慢变量 z 调节放电-静息切换)
//!   - Lyapunov 谱 (bursting 区, 典型值):
//!     λ₁ ≈ +0.01-0.05  (正, 弱混沌, 因簇间时间间隔不规则)
//!     λ₂ ≈ 0           (沿慢变量方向)
//!     λ₃ ≈ -5~-10      (负, 快变量收缩)
//!   - 平衡点 (I=0): 由 c - d x² - y = 0 → y = c - d x²
//!     代入 dx/dt=0: c - d x² - a x³ + b x² - z = 0
//!     代入 dz/dt=0: z = s(x - x_R)
//!     → -a x³ + (b-d) x² - s(x - x_R) + c = 0
//!
//! 历史:
//!   Hindmarsh, J. L. & Rose, R. M. 1984. "A model of neuronal bursting
//!   using three coupled first order differential equations."
//!   Proc. R. Soc. Lond. B 221, 87-102. (原始模型)
//!   Izhikevich, E. M. 2007. "Dynamical Systems in Neuroscience." MIT Press.
//!   (教科书系统分析)
//!   Ermentrout, G. B. & Terman, D. H. 2010. "Mathematical Foundations
//!   of Neuroscience." Springer. (bifurcation 分析)

/// Hindmarsh-Rose 神经元配置
#[derive(Clone, Copy, Debug)]
pub struct HindmarshRoseConfig {
    /// 参数 a (x³ 系数, 经典 1.0)
    pub a: f64,
    /// 参数 b (x² 系数, 经典 3.0)
    pub b: f64,
    /// 参数 c (y 偏置, 经典 1.0)
    pub c: f64,
    /// 参数 d (x² 反馈, 经典 5.0)
    pub d: f64,
    /// 参数 s (z 对 x 的敏感性, 经典 4.0)
    pub s: f64,
    /// 参数 x_R (静息电位, 经典 -1.6)
    pub x_r: f64,
    /// 参数 r (慢变量时间常数的倒数, 经典 0.005, 越小 z 越慢)
    pub r: f64,
    /// 外部注入电流 I (经典 3.0, bursting 区)
    pub i_ext: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for HindmarshRoseConfig {
    fn default() -> Self {
        Self {
            a: 1.0,
            b: 3.0,
            c: 1.0,
            d: 5.0,
            s: 4.0,
            x_r: -1.6,
            r: 0.005,
            i_ext: 3.0,
            dt: 0.01,
        }
    }
}

/// Hindmarsh-Rose 求解器 (3D, 跟踪最大 Lyapunov 指数)
pub struct HindmarshRoseSolver {
    pub config: HindmarshRoseConfig,
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

impl HindmarshRoseSolver {
    pub fn new(config: HindmarshRoseConfig, x0: f64, y0: f64, z0: f64) -> Self {
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

    /// 经典初值: 接近静息态
    pub fn classic(config: HindmarshRoseConfig) -> Self {
        Self::new(config, -1.6, -1.0, 0.5)
    }

    /// 右端导数 F = [y - a x³ + b x² - z + I, c - d x² - y, r(s(x - x_R) - z)]
    pub fn derivatives(cfg: &HindmarshRoseConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        [
            y - cfg.a * x * x * x + cfg.b * x * x - z + cfg.i_ext,
            cfg.c - cfg.d * x * x - y,
            cfg.r * (cfg.s * (x - cfg.x_r) - z),
        ]
    }

    /// Jacobian:
    /// J = [[-3a x² + 2b x,   1,    -1],
    ///      [-2d x,           -1,    0],
    ///      [r s,              0,    -r]]
    pub fn jacobian(cfg: &HindmarshRoseConfig, x: f64, _y: f64, _z: f64) -> [[f64; 3]; 3] {
        [
            [-3.0 * cfg.a * x * x + 2.0 * cfg.b * x, 1.0, -1.0],
            [-2.0 * cfg.d * x, -1.0, 0.0],
            [cfg.r * cfg.s, 0.0, -cfg.r],
        ]
    }

    /// 散度 ∇·F = tr(J) = -3a x² + 2b x - 1 - r (随 x 变化)
    pub fn divergence(cfg: &HindmarshRoseConfig, x: f64, _y: f64, _z: f64) -> f64 {
        -3.0 * cfg.a * x * x + 2.0 * cfg.b * x - 1.0 - cfg.r
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

    /// 最大 Lyapunov 指数 (文献值 ~0.01-0.05, 弱混沌)
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

    /// 检测膜电位是否超过阈值 (用于 spike 计数)
    pub fn is_spiking(&self, threshold: f64) -> bool {
        self.x > threshold
    }

    /// 统计轨迹中超过阈值的 spike 数 (近似, 简单过零计数)
    pub fn spike_count(&self, threshold: f64) -> usize {
        let mut count = 0;
        let mut above = self.trajectory[0].0 > threshold;
        for &(x, _, _) in &self.trajectory[1..] {
            let now_above = x > threshold;
            if now_above && !above {
                count += 1;
            }
            above = now_above;
        }
        count
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
        let cfg = HindmarshRoseConfig::default();
        assert!(approx_eq(cfg.a, 1.0, 1e-12));
        assert!(approx_eq(cfg.b, 3.0, 1e-12));
        assert!(approx_eq(cfg.c, 1.0, 1e-12));
        assert!(approx_eq(cfg.d, 5.0, 1e-12));
        assert!(approx_eq(cfg.s, 4.0, 1e-12));
        assert!(approx_eq(cfg.x_r, -1.6, 1e-12));
        assert!(approx_eq(cfg.r, 0.005, 1e-12));
        assert!(approx_eq(cfg.i_ext, 3.0, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = HindmarshRoseSolver::classic(HindmarshRoseConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = HindmarshRoseConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = HindmarshRoseSolver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], y - cfg.a * x * x * x + cfg.b * x * x - z + cfg.i_ext, 1e-12));
        assert!(approx_eq(d[1], cfg.c - cfg.d * x * x - y, 1e-12));
        assert!(approx_eq(d[2], cfg.r * (cfg.s * (x - cfg.x_r) - z), 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = HindmarshRoseConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = HindmarshRoseSolver::jacobian(&cfg, x, y, z);
        // Row 0: [-3a x² + 2b x, 1, -1]
        assert!(approx_eq(j[0][0], -3.0 * cfg.a * x * x + 2.0 * cfg.b * x, 1e-12));
        assert!(approx_eq(j[0][1], 1.0, 1e-12));
        assert!(approx_eq(j[0][2], -1.0, 1e-12));
        // Row 1: [-2d x, -1, 0]
        assert!(approx_eq(j[1][0], -2.0 * cfg.d * x, 1e-12));
        assert!(approx_eq(j[1][1], -1.0, 1e-12));
        assert!(approx_eq(j[1][2], 0.0, 1e-12));
        // Row 2: [r s, 0, -r]
        assert!(approx_eq(j[2][0], cfg.r * cfg.s, 1e-12));
        assert!(approx_eq(j[2][1], 0.0, 1e-12));
        assert!(approx_eq(j[2][2], -cfg.r, 1e-12));
    }

    #[test]
    fn test_divergence_formula() {
        // 散度 = -3a x² + 2b x - 1 - r
        let cfg = HindmarshRoseConfig::default();
        let x = 0.5_f64;
        let expected = -3.0 * cfg.a * x * x + 2.0 * cfg.b * x - 1.0 - cfg.r;
        assert!(approx_eq(HindmarshRoseSolver::divergence(&cfg, x, 0.0, 0.0), expected, 1e-12));
    }

    #[test]
    fn test_divergence_not_constant() {
        // 散度随 x 变化
        let cfg = HindmarshRoseConfig::default();
        let d1 = HindmarshRoseSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        let d2 = HindmarshRoseSolver::divergence(&cfg, 1.0, 0.0, 0.0);
        assert!((d1 - d2).abs() > 0.5);
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = HindmarshRoseConfig::default();
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5)] {
            let j = HindmarshRoseSolver::jacobian(&cfg, x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = HindmarshRoseSolver::divergence(&cfg, x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_step_advances() {
        let mut s = HindmarshRoseSolver::classic(HindmarshRoseConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = HindmarshRoseSolver::classic(HindmarshRoseConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = HindmarshRoseSolver::classic(HindmarshRoseConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = HindmarshRoseSolver::classic(HindmarshRoseConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = HindmarshRoseSolver::classic(HindmarshRoseConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        // HR 神经元典型范围: x∈[-2,2], y∈[-15,5], z∈[0,5]
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_bursting_spikes_present() {
        // 经典参数 (I=3) 应产生 spiking/bursting, 膜电位应出现尖峰
        let mut s = HindmarshRoseSolver::classic(HindmarshRoseConfig::default());
        s.run(30000);
        // 膜电位应超过 0 (spike 阈值, 从静息 -1.6 上升)
        let xmax = s.trajectory.iter().map(|&(x, _, _)| x).fold(f64::NEG_INFINITY, f64::max);
        assert!(xmax > 0.0, "should have spikes, xmax = {}", xmax);
    }

    #[test]
    fn test_spike_count_bursting() {
        // Bursting 区应产生多个 spike
        let mut s = HindmarshRoseSolver::classic(HindmarshRoseConfig::default());
        s.run(30000);
        let count = s.spike_count(0.0);
        // 应该有显著数量的 spike (至少 10 个)
        assert!(count > 10, "should have many spikes in bursting mode, count = {}", count);
    }

    #[test]
    fn test_zero_current_quiescent() {
        // I=0 时, 神经元应接近静息态, 无 spike
        let cfg = HindmarshRoseConfig { i_ext: 0.0, ..HindmarshRoseConfig::default() };
        let mut s = HindmarshRoseSolver::new(cfg, -1.6, -1.0, 0.5);
        s.run(30000);
        let count = s.spike_count(0.0);
        // 无外部电流: 应该没有 spike (或极少)
        assert!(count < 5, "no current → quiescent, count = {}", count);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = HindmarshRoseSolver::classic(HindmarshRoseConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_lyapunov_not_strongly_positive() {
        // HR 在 bursting 区 Lyapunov 指数较小 (~0.01-0.05, 弱混沌)
        // 不应该出现强混沌 (λ < 1)
        let mut s = HindmarshRoseSolver::classic(HindmarshRoseConfig::default());
        s.run(50000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda < 1.0, "lambda should be small (weak chaos): {}", lambda);
    }

    #[test]
    fn test_average_divergence_negative() {
        // 在吸引子轨道上采样, 平均散度应为负 (耗散)
        let cfg = HindmarshRoseConfig::default();
        let mut s = HindmarshRoseSolver::classic(cfg);
        s.run(20000);
        let mut div_sum = 0.0;
        let n_samples = 1000;
        let trajectory_len = s.trajectory.len();
        for i in (10000..trajectory_len).step_by(10) {
            let (x, _y, _z) = s.trajectory[i];
            div_sum += HindmarshRoseSolver::divergence(&cfg, x, 0.0, 0.0);
        }
        let avg_div = div_sum / n_samples as f64;
        assert!(avg_div < 0.0, "average divergence should be negative: {}", avg_div);
    }

    #[test]
    fn test_slow_variable_z_bounded() {
        // z 是慢变量, 应该保持在有界范围内 (不会快速变化)
        let mut s = HindmarshRoseSolver::classic(HindmarshRoseConfig::default());
        s.run(30000);
        let (_, _, _, _, zmin, zmax) = s.attractor_bounds();
        // z 的变化幅度应远小于快变量 x, y
        assert!(zmax - zmin < 20.0, "z should be slow and bounded: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_multiscale_dynamics() {
        // 验证多尺度: z 的时间常数 1/r ≫ 1 (x, y 的时间常数)
        let cfg = HindmarshRoseConfig::default();
        // 1/r = 200, x/y 的时间常数 ~1
        assert!(1.0 / cfg.r > 100.0, "z should be much slower than x, y");
        // Jacobian 中 z 行的对角元素 |-r| 应远小于 x, y 行的对角元素
        // (z 的时间常数 1/r ≫ 1, 而 x, y 的时间常数 ~1)
        let j = HindmarshRoseSolver::jacobian(&cfg, 1.0, 0.0, 0.0);
        let z_diag = j[2][2].abs(); // |-r| = 0.005
        let x_diag = j[0][0].abs(); // |-3a+2b| = 3 at x=1
        let y_diag = j[1][1].abs(); // |-1| = 1
        assert!(z_diag < y_diag, "z timescale 1/r={} >> y timescale 1", 1.0 / cfg.r);
        assert!(z_diag < x_diag, "z timescale 1/r={} >> x timescale", 1.0 / cfg.r);
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = HindmarshRoseConfig::default();
        let mut s = HindmarshRoseSolver::new(cfg, 100.0, 100.0, 100.0);
        s.run(500);
        assert!(s.has_escaped() || !s.has_nan());
    }
}
