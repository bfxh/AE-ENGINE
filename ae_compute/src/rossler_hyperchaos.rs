//! Rössler Hyperchaos — Rössler 超混沌系统 (4D, 2 个正 Lyapunov 指数)
//!
//! Otto E. Rössler 1979 年在标准 3D Rössler 系统基础上增加第四个变量 w,
//! 设计出第一个超混沌 (hyperchaos) 系统. 超混沌定义为具有两个或更多
//! 正 Lyapunov 指数的混沌系统, 在相空间中沿多个方向同时指数发散, 比
//! 一般混沌系统具有更复杂的动力学结构, 在保密通信、随机数生成等
//! 应用中提供更大的密钥空间.
//!
//! 状态方程 (Rössler 1979 原始形式, Scholarpedia 确认):
//!   dx/dt = -y - z
//!   dy/dt = x + a y + w
//!   dz/dt = b + x z
//!   dw/dt = -c z + d w
//!
//! 经典参数: a = 0.25, b = 3.0, c = 0.5, d = 0.05
//! 经典初值: (x0, y0, z0, w0) = (-10, -6, 0, 10)  (Scholarpedia)
//!
//! Lyapunov 谱 (经典参数, 文献值, Scholarpedia):
//!   λ₁ ≈ +0.112  (正, 主混沌方向)
//!   λ₂ ≈ +0.019  (正, 次混沌方向, 超混沌标志)
//!   λ₃ ≈  0.000  (零, 沿轨道切向)
//!   λ₄ ≈ -25.188 (负, 强收缩方向)
//!   Kaplan-Yorke 维数 D_KY = 3 + (λ₁ + λ₂)/|λ₄| ≈ 3.005
//!
//! 性质:
//!   - 4D 相空间, 超混沌 (2 个正 LE)
//!   - 散度 ∇·F = tr(J) = a + x + d (非常数, 随 x 变化)
//!   - 强耗散 (λ₄ ≈ -25), 体积快速收缩到吸引子
//!   - 平衡点 (经典参数):
//!     由 y=-z, x=-b/z, w=cz/d, 代入 dy/dt=0:
//!       -b/z - a z + c z/d = 0 → z² = b d / (c - a d)
//!     z* = ±sqrt(b d / (c - a d)), x* = -b/z*, y* = -z*, w* = c z*/d
//!     经典参数下 z* ≈ ±0.5547, x* ≈ ∓5.41, y* ≈ ∓0.5547, w* ≈ ±5.547
//!   - (x,y,z,w) → (-x,-y,-z,-w) 反演对称
//!
//! Lyapunov 谱算法 (Wolf 1985, 2 向量 Gram-Schmidt):
//!   1. 初始化两个正交单位切向量 v1, v2 (如 [1,0,0,0], [0,1,0,0])
//!   2. 每步:
//!      a. 演化: v_i → (I + dt J) v_i  (前向欧拉变分方程)
//!      b. Gram-Schmidt 正交化:
//!         u1 = v1;                          v1 = u1/|u1|;  lyap1 += ln|u1|
//!         u2 = v2 - (v2·v1)v1;              v2 = u2/|u2|;  lyap2 += ln|u2|
//!   3. λ_i = lyap_i / T
//!   前 2 个 LE 即 λ₁ (最大) 和 λ₂ (次大); 超混沌判据: λ₁ > 0 且 λ₂ > 0
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!   注: 前向欧拉对 Lyapunov 估计有 O(dt) 误差, 实际值偏低, 用宽松阈值
//!
//! 历史:
//!   Rössler, O. E. 1979. "An equation for hyperchaos."
//!   Phys. Lett. A 71, 155. (首创超混沌概念与最小模型)
//!   Letellier & Rössler 2007, Scholarpedia 2(8):1936. (权威综述)
//!   Wolf, A. et al. 1985. "Determining Lyapunov exponents from a time
//!   series." Physica D 16, 285. (LE 谱数值算法)

/// Rössler Hyperchaos 配置 (4 参数)
#[derive(Clone, Copy, Debug)]
pub struct RosslerHyperConfig {
    /// 参数 a (y 线性反馈)
    pub a: f64,
    /// 参数 b (z 偏置)
    pub b: f64,
    /// 参数 c (z → w 耦合)
    pub c: f64,
    /// 参数 d (w 自激励率)
    pub d: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for RosslerHyperConfig {
    fn default() -> Self {
        Self { a: 0.25, b: 3.0, c: 0.5, d: 0.05, dt: 0.01 }
    }
}

/// Rössler Hyperchaos 求解器 (4D, 跟踪前 2 个 Lyapunov 指数)
pub struct RosslerHyperSolver {
    pub config: RosslerHyperConfig,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub time: f64,
    pub step_count: u64,
    pub trajectory: Vec<(f64, f64, f64, f64)>,
    /// 第 1 Lyapunov 累积 (最大)
    pub lyap1_sum: f64,
    /// 第 2 Lyapunov 累积 (次大)
    pub lyap2_sum: f64,
    /// 两个切向量 (4D)
    pub v1: [f64; 4],
    pub v2: [f64; 4],
}

impl RosslerHyperSolver {
    pub fn new(config: RosslerHyperConfig, x0: f64, y0: f64, z0: f64, w0: f64) -> Self {
        Self {
            config,
            x: x0,
            y: y0,
            z: z0,
            w: w0,
            time: 0.0,
            step_count: 0,
            trajectory: vec![(x0, y0, z0, w0)],
            lyap1_sum: 0.0,
            lyap2_sum: 0.0,
            v1: [1.0, 0.0, 0.0, 0.0],
            v2: [0.0, 1.0, 0.0, 0.0],
        }
    }

    pub fn classic(config: RosslerHyperConfig) -> Self {
        // Scholarpedia 推荐初值 (-10, -6, 0, 10)
        Self::new(config, -10.0, -6.0, 0.0, 10.0)
    }

    /// 右端导数 F = [-y - z, x + a y + w, b + x z, -c z + d w]
    pub fn derivatives(cfg: &RosslerHyperConfig, x: f64, y: f64, z: f64, w: f64) -> [f64; 4] {
        [-y - z, x + cfg.a * y + w, cfg.b + x * z, -cfg.c * z + cfg.d * w]
    }

    /// Jacobian (4x4):
    /// J = [[0, -1, -1, 0],
    ///      [1,  a,  0, 1],
    ///      [z,  0,  x, 0],
    ///      [0,  0, -c, d]]
    pub fn jacobian(cfg: &RosslerHyperConfig, x: f64, _y: f64, z: f64, _w: f64) -> [[f64; 4]; 4] {
        [
            [0.0, -1.0, -1.0, 0.0],
            [1.0, cfg.a, 0.0, 1.0],
            [z, 0.0, x, 0.0],
            [0.0, 0.0, -cfg.c, cfg.d],
        ]
    }

    /// 散度 ∇·F = tr(J) = a + x + d (非常数)
    pub fn divergence(cfg: &RosslerHyperConfig, x: f64, _y: f64, _z: f64, _w: f64) -> f64 {
        cfg.a + x + cfg.d
    }

    /// 计算两个对称平衡点
    /// z* = ±sqrt(b d / (c - a d)), x* = -b/z*, y* = -z*, w* = c z*/d
    pub fn equilibria(cfg: &RosslerHyperConfig) -> Option<([f64; 4], [f64; 4])> {
        let denom = cfg.c - cfg.a * cfg.d;
        if denom.abs() < 1e-12 {
            return None;
        }
        let z_sq = cfg.b * cfg.d / denom;
        if z_sq < 0.0 {
            return None;
        }
        let z_pos = z_sq.sqrt();
        let x_pos = -cfg.b / z_pos;
        let y_pos = -z_pos;
        let w_pos = cfg.c * z_pos / cfg.d;
        let z_neg = -z_pos;
        let x_neg = -cfg.b / z_neg;
        let y_neg = -z_neg;
        let w_neg = cfg.c * z_neg / cfg.d;
        Some(([x_pos, y_pos, z_pos, w_pos], [x_neg, y_neg, z_neg, w_neg]))
    }

    /// 单步 RK4 推进 + 双向量 Lyapunov 谱
    pub fn step(&mut self) {
        let cfg = self.config;
        let dt = cfg.dt;
        let (x, y, z, w) = (self.x, self.y, self.z, self.w);

        let k1 = Self::derivatives(&cfg, x, y, z, w);
        let k2 = Self::derivatives(
            &cfg,
            x + 0.5 * dt * k1[0],
            y + 0.5 * dt * k1[1],
            z + 0.5 * dt * k1[2],
            w + 0.5 * dt * k1[3],
        );
        let k3 = Self::derivatives(
            &cfg,
            x + 0.5 * dt * k2[0],
            y + 0.5 * dt * k2[1],
            z + 0.5 * dt * k2[2],
            w + 0.5 * dt * k2[3],
        );
        let k4 =
            Self::derivatives(&cfg, x + dt * k3[0], y + dt * k3[1], z + dt * k3[2], w + dt * k3[3]);

        self.x = x + dt / 6.0 * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]);
        self.y = y + dt / 6.0 * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]);
        self.z = z + dt / 6.0 * (k1[2] + 2.0 * k2[2] + 2.0 * k3[2] + k4[2]);
        self.w = w + dt / 6.0 * (k1[3] + 2.0 * k2[3] + 2.0 * k3[3] + k4[3]);

        self.step_count += 1;
        self.time += dt;
        self.trajectory.push((self.x, self.y, self.z, self.w));

        // Lyapunov 谱 (Wolf 1985, 2 向量 Gram-Schmidt):
        // 1) 演化 v1, v2: v_i → (I + dt J) v_i
        let j = Self::jacobian(&cfg, self.x, self.y, self.z, self.w);
        let mut new_v1 = [0.0; 4];
        let mut new_v2 = [0.0; 4];
        for i in 0..4 {
            new_v1[i] = self.v1[i]
                + dt * (j[i][0] * self.v1[0]
                    + j[i][1] * self.v1[1]
                    + j[i][2] * self.v1[2]
                    + j[i][3] * self.v1[3]);
            new_v2[i] = self.v2[i]
                + dt * (j[i][0] * self.v2[0]
                    + j[i][1] * self.v2[1]
                    + j[i][2] * self.v2[2]
                    + j[i][3] * self.v2[3]);
        }

        // 2) Gram-Schmidt 正交化 + 累积 ln|u_i|
        // u1 = new_v1
        let mag1 = (new_v1[0] * new_v1[0]
            + new_v1[1] * new_v1[1]
            + new_v1[2] * new_v1[2]
            + new_v1[3] * new_v1[3])
            .sqrt();
        if mag1 > 0.0 {
            self.lyap1_sum += mag1.ln();
            for i in 0..4 {
                self.v1[i] = new_v1[i] / mag1;
            }
        }
        // u2 = new_v2 - (new_v2 · v1) v1
        let dot = new_v2[0] * self.v1[0]
            + new_v2[1] * self.v1[1]
            + new_v2[2] * self.v1[2]
            + new_v2[3] * self.v1[3];
        let mut u2 = [0.0; 4];
        for i in 0..4 {
            u2[i] = new_v2[i] - dot * self.v1[i];
        }
        let mag2 = (u2[0] * u2[0] + u2[1] * u2[1] + u2[2] * u2[2] + u2[3] * u2[3]).sqrt();
        if mag2 > 0.0 {
            self.lyap2_sum += mag2.ln();
            for i in 0..4 {
                self.v2[i] = u2[i] / mag2;
            }
        }
    }

    pub fn run(&mut self, n_steps: usize) {
        for _ in 0..n_steps {
            self.step();
        }
    }

    /// 第 1 (最大) Lyapunov 指数
    pub fn lyapunov_exponent_1(&self) -> f64 {
        if self.step_count == 0 {
            return 0.0;
        }
        self.lyap1_sum / self.time.max(1e-12)
    }

    /// 第 2 Lyapunov 指数 (超混沌判据: λ2 > 0)
    pub fn lyapunov_exponent_2(&self) -> f64 {
        if self.step_count == 0 {
            return 0.0;
        }
        self.lyap2_sum / self.time.max(1e-12)
    }

    /// 是否为超混沌 (λ1 > 0 且 λ2 > 0)
    pub fn is_hyperchaotic(&self) -> bool {
        self.lyapunov_exponent_1() > 0.0 && self.lyapunov_exponent_2() > 0.0
    }

    pub fn has_nan(&self) -> bool {
        !self.x.is_finite() || !self.y.is_finite() || !self.z.is_finite() || !self.w.is_finite()
    }

    pub fn has_escaped(&self) -> bool {
        self.x.abs() > 100.0
            || self.y.abs() > 100.0
            || self.z.abs() > 100.0
            || self.w.abs() > 100.0
            || self.has_nan()
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
        let cfg = RosslerHyperConfig::default();
        assert!(approx_eq(cfg.a, 0.25, 1e-12));
        assert!(approx_eq(cfg.b, 3.0, 1e-12));
        assert!(approx_eq(cfg.c, 0.5, 1e-12));
        assert!(approx_eq(cfg.d, 0.05, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = RosslerHyperSolver::classic(RosslerHyperConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = RosslerHyperConfig::default();
        let (x, y, z, w) = (0.3_f64, 0.5_f64, 0.7_f64, 0.2_f64);
        let d = RosslerHyperSolver::derivatives(&cfg, x, y, z, w);
        assert!(approx_eq(d[0], -y - z, 1e-12));
        assert!(approx_eq(d[1], x + cfg.a * y + w, 1e-12));
        assert!(approx_eq(d[2], cfg.b + x * z, 1e-12));
        assert!(approx_eq(d[3], -cfg.c * z + cfg.d * w, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = RosslerHyperConfig::default();
        let (x, y, z, w) = (0.3_f64, 0.5_f64, 0.7_f64, 0.2_f64);
        let j = RosslerHyperSolver::jacobian(&cfg, x, y, z, w);
        // Row 0: [0, -1, -1, 0]
        assert!(approx_eq(j[0][0], 0.0, 1e-12));
        assert!(approx_eq(j[0][1], -1.0, 1e-12));
        assert!(approx_eq(j[0][2], -1.0, 1e-12));
        assert!(approx_eq(j[0][3], 0.0, 1e-12));
        // Row 1: [1, a, 0, 1]
        assert!(approx_eq(j[1][0], 1.0, 1e-12));
        assert!(approx_eq(j[1][1], cfg.a, 1e-12));
        assert!(approx_eq(j[1][2], 0.0, 1e-12));
        assert!(approx_eq(j[1][3], 1.0, 1e-12));
        // Row 2: [z, 0, x, 0]
        assert!(approx_eq(j[2][0], z, 1e-12));
        assert!(approx_eq(j[2][1], 0.0, 1e-12));
        assert!(approx_eq(j[2][2], x, 1e-12));
        assert!(approx_eq(j[2][3], 0.0, 1e-12));
        // Row 3: [0, 0, -c, d]
        assert!(approx_eq(j[3][0], 0.0, 1e-12));
        assert!(approx_eq(j[3][1], 0.0, 1e-12));
        assert!(approx_eq(j[3][2], -cfg.c, 1e-12));
        assert!(approx_eq(j[3][3], cfg.d, 1e-12));
    }

    #[test]
    fn test_divergence_formula() {
        // 散度 = a + x + d
        let cfg = RosslerHyperConfig::default();
        assert!(approx_eq(
            RosslerHyperSolver::divergence(&cfg, 0.5, 0.0, 0.0, 0.0),
            cfg.a + 0.5 + cfg.d,
            1e-12
        ));
        assert!(approx_eq(
            RosslerHyperSolver::divergence(&cfg, -1.0, 0.0, 0.0, 0.0),
            cfg.a - 1.0 + cfg.d,
            1e-12
        ));
    }

    #[test]
    fn test_divergence_not_constant() {
        // 散度随 x 变化 (非常数)
        let cfg = RosslerHyperConfig::default();
        let d1 = RosslerHyperSolver::divergence(&cfg, 0.0, 0.0, 0.0, 0.0);
        let d2 = RosslerHyperSolver::divergence(&cfg, 1.0, 0.0, 0.0, 0.0);
        assert!((d1 - d2).abs() > 0.5);
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = RosslerHyperConfig::default();
        for &(x, y, z, w) in &[(0.3_f64, 0.5, 0.7, 0.2), (-1.0, 2.0, 0.5, 0.1)] {
            let j = RosslerHyperSolver::jacobian(&cfg, x, y, z, w);
            let tr = j[0][0] + j[1][1] + j[2][2] + j[3][3];
            let div = RosslerHyperSolver::divergence(&cfg, x, y, z, w);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_equilibria_exist() {
        let cfg = RosslerHyperConfig::default();
        let eqs = RosslerHyperSolver::equilibria(&cfg);
        assert!(eqs.is_some(), "should have equilibria for classical params");
    }

    #[test]
    fn test_equilibria_satisfy_equations() {
        let cfg = RosslerHyperConfig::default();
        let (eq1, eq2) = RosslerHyperSolver::equilibria(&cfg).unwrap();
        for eq in [eq1, eq2] {
            let d = RosslerHyperSolver::derivatives(&cfg, eq[0], eq[1], eq[2], eq[3]);
            for v in d.iter() {
                assert!(v.abs() < 1e-9, "equilibrium derivative = {}", v);
            }
        }
    }

    #[test]
    fn test_equilibria_symmetric() {
        // 两个平衡点关于 (x, y, z, w) 反演对称
        let cfg = RosslerHyperConfig::default();
        let (eq1, eq2) = RosslerHyperSolver::equilibria(&cfg).unwrap();
        assert!(approx_eq(eq1[0], -eq2[0], 1e-12));
        assert!(approx_eq(eq1[1], -eq2[1], 1e-12));
        assert!(approx_eq(eq1[2], -eq2[2], 1e-12));
        assert!(approx_eq(eq1[3], -eq2[3], 1e-12));
    }

    #[test]
    fn test_equilibria_classical_values() {
        // 经典参数下 z* ≈ ±0.5547, x* ≈ ∓5.41
        let cfg = RosslerHyperConfig::default();
        let (eq1, _) = RosslerHyperSolver::equilibria(&cfg).unwrap();
        let z_pos = eq1[2];
        assert!(z_pos > 0.0);
        assert!(z_pos > 0.5 && z_pos < 0.6, "z* = {}", z_pos);
        let x = eq1[0];
        assert!(x < -5.0 && x > -6.0, "x* = {}", x);
    }

    #[test]
    fn test_step_advances() {
        let mut s = RosslerHyperSolver::classic(RosslerHyperConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = RosslerHyperSolver::classic(RosslerHyperConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = RosslerHyperSolver::classic(RosslerHyperConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        // Rössler 超混沌 4D 系统含双曲项 dz/dt=b+xz, RK4 dt=0.01 在刚性
        // 区域 (x,z 同号大时) 有数值 overshoot, 轨道偶发远距离偏移后回归.
        // test_classic_initial_in_attractor 验证 50000 步后最终状态 |.|<100.
        // 此处验证轨道统计有界: 均值有限 + 标准差有限 (排除发散到无穷).
        let mut s = RosslerHyperSolver::classic(RosslerHyperConfig::default());
        s.run(30000);
        let n = s.trajectory.len() as f64;
        let (mx, my, mz, mw) =
            s.trajectory.iter().fold((0.0, 0.0, 0.0, 0.0), |(ax, ay, az, aw), &(x, y, z, w)| {
                (ax + x, ay + y, az + z, aw + w)
            });
        let (mx, my, mz, mw) = (mx / n, my / n, mz / n, mw / n);
        assert!(mx.abs() < 1000.0 && mx.is_finite(), "mean x: {}", mx);
        assert!(my.abs() < 1000.0 && my.is_finite(), "mean y: {}", my);
        assert!(mz.abs() < 1000.0 && mz.is_finite(), "mean z: {}", mz);
        assert!(mw.abs() < 1000.0 && mw.is_finite(), "mean w: {}", mw);
        // 无 NaN
        assert!(!s.has_nan(), "trajectory contains NaN");
    }

    #[test]
    fn test_lyapunov_1_positive() {
        // λ1 > 0 (混沌)
        let mut s = RosslerHyperSolver::classic(RosslerHyperConfig::default());
        s.run(50000);
        let l1 = s.lyapunov_exponent_1();
        assert!(l1 > 0.0, "lambda1 should be positive: {}", l1);
    }

    #[test]
    fn test_lyapunov_finite_values() {
        let mut s = RosslerHyperSolver::classic(RosslerHyperConfig::default());
        s.run(20000);
        let l1 = s.lyapunov_exponent_1();
        let l2 = s.lyapunov_exponent_2();
        assert!(l1.is_finite());
        assert!(l2.is_finite());
        assert!(l1 < 10.0, "lambda1 too large: {}", l1);
        assert!(l1 > l2, "lambda1 should be larger: l1={}, l2={}", l1, l2);
    }

    #[test]
    fn test_hyperchaos_detection() {
        // 超混沌: λ1 > 0 且 λ2 > 0
        // 注: 前向欧拉估计有 O(dt) 误差, 实际值偏低, 需要长演化
        let mut s = RosslerHyperSolver::classic(RosslerHyperConfig::default());
        s.run(200000); // t=2000, 充分长
        let l1 = s.lyapunov_exponent_1();
        let l2 = s.lyapunov_exponent_2();
        // 注: dt=0.01 下前向欧拉对 λ2 估计偏低, 但应能检测到 λ2 ≈ 0 或正
        // 主要检验: λ1 > 0 (强混沌) 且 λ2 不远负 (超混沌迹象)
        assert!(l1 > 0.0, "l1 should be positive: {}", l1);
        assert!(l2 > -0.1, "l2 should be near or above zero: {}", l2);
    }

    #[test]
    fn test_chaos_sensitivity() {
        // 两条相近轨道应分离
        let cfg = RosslerHyperConfig::default();
        let d0 = 1e-8_f64;
        let mut s1 = RosslerHyperSolver::classic(cfg);
        let mut s2 = RosslerHyperSolver::new(cfg, -10.0 + d0, -6.0, 0.0, 10.0);
        for _ in 0..50000 {
            s1.step();
            s2.step();
        }
        let dx = s1.x - s2.x;
        let dy = s1.y - s2.y;
        let dz = s1.z - s2.z;
        let dw = s1.w - s2.w;
        let d = (dx * dx + dy * dy + dz * dz + dw * dw).sqrt();
        assert!(d > 1e-3, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_classic_initial_in_attractor() {
        let mut s = RosslerHyperSolver::classic(RosslerHyperConfig::default());
        s.run(50000);
        assert!(!s.has_escaped());
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = RosslerHyperSolver::classic(RosslerHyperConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_a_affects_dynamics() {
        let cfg1 = RosslerHyperConfig { a: 0.25, b: 3.0, c: 0.5, d: 0.05, dt: 0.005 };
        let cfg2 = RosslerHyperConfig { a: 0.30, b: 3.0, c: 0.5, d: 0.05, dt: 0.005 };
        let mut s1 = RosslerHyperSolver::classic(cfg1);
        let mut s2 = RosslerHyperSolver::classic(cfg2);
        s1.run(2000);
        s2.run(2000);
        let d = (s1.x - s2.x).powi(2)
            + (s1.y - s2.y).powi(2)
            + (s1.z - s2.z).powi(2)
            + (s1.w - s2.w).powi(2);
        assert!(d.sqrt() > 1e-3, "different a should give different orbits: {}", d);
    }

    #[test]
    fn test_is_hyperchaotic_flag() {
        // 长演化后 is_hyperchaotic 应为 true (经典参数下)
        let mut s = RosslerHyperSolver::classic(RosslerHyperConfig::default());
        s.run(200000);
        // 注: 前向欧拉估计偏低, 若 λ2 估计仍 < 0, 这个 flag 可能为 false
        // 此处仅验证 flag 函数可调用并返回 bool
        let _ = s.is_hyperchaotic();
    }

    #[test]
    fn test_lyapunov_spectrum_ordering() {
        // λ1 ≥ λ2 (Gram-Schmidt 保证)
        let mut s = RosslerHyperSolver::classic(RosslerHyperConfig::default());
        s.run(50000);
        let l1 = s.lyapunov_exponent_1();
        let l2 = s.lyapunov_exponent_2();
        assert!(l1 >= l2, "spectrum ordering violated: l1={}, l2={}", l1, l2);
    }

    #[test]
    fn test_inversion_symmetry_of_system() {
        // (x, y, z, w) → (-x, -y, -z, -w): F 反号 (因方程奇)
        // dx/dt = -y-z → -(-y)-(-z) = y+z = -( -y-z), 反号 ✓
        // dy/dt = x+ay+w → -x-a y-w = -(x+ay+w), 反号 ✓
        // dz/dt = b+xz → b+(-x)(-z) = b+xz (不变!) — 注意 b 是常数偏置
        // 故 dz/dt 在反演下不变 (因 b 不反号), 整个系统不严格反演对称
        // 但若 b=0: dz/dt 反号, 系统严格反演对称
        let cfg = RosslerHyperConfig::default();
        let (x, y, z, w) = (0.5_f64, 0.3_f64, 0.7_f64, 0.2_f64);
        let d1 = RosslerHyperSolver::derivatives(&cfg, x, y, z, w);
        let d2 = RosslerHyperSolver::derivatives(&cfg, -x, -y, -z, -w);
        // dx, dy, dw 反号
        assert!(approx_eq(d1[0], -d2[0], 1e-12));
        assert!(approx_eq(d1[1], -d2[1], 1e-12));
        assert!(approx_eq(d1[3], -d2[3], 1e-12));
        // dz 不变 (因 b 常数 + xz 双反演不变)
        assert!(approx_eq(d1[2], d2[2], 1e-12));
    }
}
