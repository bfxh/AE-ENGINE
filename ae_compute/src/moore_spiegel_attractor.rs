//! Moore-Spiegel Attractor — Moore-Spiegel 恒星对流混沌系统 (3D)
//!
//! R. Moore 和 E. A. Spiegel 1966 年为模拟恒星外壳对流不稳定性而提出
//! 的 3D 自治混沌系统. 该系统是天体物理学中最早的混沌模型之一,
//! 与 Lorenz 63 (大气对流) 并列为"对流混沌"的两大经典例子.
//!
//! 物理背景: 恒星外壳对流
//!   恒星外壳存在气体对流区 (如太阳的对流区), 气体在浮力驱动下
//!   上升冷却, 形成对流单元. Moore 和 Spiegel 用三阶 ODE 描述此
//!   过程, 发现特定参数下出现非周期振荡, 这是恒星变星的可能机制.
//!
//! 状态方程 (Moore & Spiegel 1966):
//!   令 y = x', z = x'', 则:
//!   dx/dt = y
//!   dy/dt = z
//!   dz/dt = -z - (T + R(x² - 1))y - R·x
//!
//! 等价的三阶 ODE 形式:
//!   x''' + x'' + (T + R(x² - 1))x' + R·x = 0
//!
//! 各项物理意义:
//!   - y = x': 速度 (位移的导数)
//!   - z = x'': 加速度 (速度的导数)
//!   - -z: 粘性阻尼 (与加速度成正比, 三阶耗散)
//!   - -(T + R(x²-1))y: 非线性阻尼 (依赖于位移的"恢复+阻尼"项)
//!     · -T·y: 线性阻尼 (常数 T)
//!     · -R(x²-1)·y: 非线性阻尼 (位移大时增强, 位移小时反向)
//!   - R·x: 线性恢复力 (弹簧, R 为刚度)
//!
//! 经典参数 (Moore & Spiegel 1966): T = 6, R = 20
//! 经典初值: (x₀, y₀, z₀) = (0.1, 0, 0) 或 (1, 0, 0)
//!
//! 性质:
//!   - 散度 ∇·F = tr(J) = -1 (常数负, 耗散)
//!     体积收缩率 e^(-t), 中等耗散
//!   - 平衡点 (利用 y=0, z=0, R·x=0):
//!     x = 0 → E0 = (0, 0, 0) (唯一平衡点)
//!   - Lyapunov 谱 (经典参数, 文献值):
//!     λ₁ ≈ +0.07  (正, 主混沌方向, 弱混沌)
//!     λ₂ = 0      (沿轨道切向)
//!     λ₃ ≈ -1.07  (负, 收缩)
//!     和 = -1 (与散度一致)
//!   - Kaplan-Yorke 维数 D_KY ≈ 2 + λ₁/|λ₃| ≈ 2.065
//!   - 吸引子形态: 螺旋型 (spiral), 围绕原点的不规则振荡
//!
//! 与 Lorenz 63 对比:
//!   - Lorenz 63: 大气对流, 一阶方程组, 3 个平衡点, λ₁ ≈ 0.91
//!   - Moore-Spiegel: 恒星对流, 三阶 ODE 自治化, 1 个平衡点, λ₁ ≈ 0.07
//!   - 两者都是 Rayleigh-Bénard 对流的简化, 但物理对象不同
//!   - Moore-Spiegel 混沌较弱, 是"边缘混沌"的典型例子
//!
//! 与 Arneodo 对比 (均为三阶 ODE 自治化):
//!   - Arneodo: dz/dt = -μ z - y - x + x³ (Duffing 型恢复力)
//!   - Moore-Spiegel: dz/dt = -z - (T+R(x²-1))y - Rx (非线性阻尼)
//!   - Arneodo 是非线性恢复力 (势能), Moore-Spiegel 是非线性阻尼 (耗散)
//!
//! 天体物理意义:
//!   恒星变星 (如造父变星) 的光变曲线呈周期或非周期振荡. Moore-Spiegel
//!   系统证明, 恒星外壳对流的不稳定性可以导致非周期光变, 是某些不规则
//!   变星的可能机制. 该系统启发了后来关于恒星脉动与混沌的研究.
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!
//! 历史:
//!   Moore, R. K. & Spiegel, E. A. 1966. "A magnetically driven
//!   stellar oscillator." Astrophys. J. 143, 871-887. (原始论文)
//!   Spiegel, E. A. 1985. "Cosmic arrhythmias." In Chaos in
//!   Astrophysics. (综述, 混沌在天体物理的应用)
//!   Sprott, J. C. 2003. "Chaos and Time-Series Analysis." Oxford.

/// Moore-Spiegel 系统配置 (2 参数 + dt)
#[derive(Clone, Copy, Debug)]
pub struct MooreSpiegelConfig {
    /// 线性阻尼参数 T (经典 6)
    pub t_param: f64,
    /// 恢复力刚度 R (经典 20)
    pub r_param: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for MooreSpiegelConfig {
    fn default() -> Self {
        Self {
            t_param: 6.0,
            r_param: 20.0,
            dt: 0.01,
        }
    }
}

/// Moore-Spiegel 求解器 (3D, 跟踪最大 Lyapunov 指数)
pub struct MooreSpiegelSolver {
    pub config: MooreSpiegelConfig,
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

impl MooreSpiegelSolver {
    pub fn new(config: MooreSpiegelConfig, x0: f64, y0: f64, z0: f64) -> Self {
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

    pub fn classic(config: MooreSpiegelConfig) -> Self {
        Self::new(config, 0.1, 0.0, 0.0)
    }

    /// 右端导数 F = [y, z, -z - (T + R(x²-1))y - R·x]
    pub fn derivatives(cfg: &MooreSpiegelConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        let nonlinear_damping = cfg.t_param + cfg.r_param * (x * x - 1.0);
        [y, z, -z - nonlinear_damping * y - cfg.r_param * x]
    }

    /// Jacobian:
    /// J = [[0,                          1,  0],
    ///      [0,                          0,  1],
    ///      [-2Rxy - R,  -(T + R(x²-1)), -1]]
    pub fn jacobian(cfg: &MooreSpiegelConfig, x: f64, y: f64, _z: f64) -> [[f64; 3]; 3] {
        let nonlinear_damping = cfg.t_param + cfg.r_param * (x * x - 1.0);
        [
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [-2.0 * cfg.r_param * x * y - cfg.r_param, -nonlinear_damping, -1.0],
        ]
    }

    /// 散度 ∇·F = tr(J) = -1 (常数负)
    pub fn divergence(_cfg: &MooreSpiegelConfig, _x: f64, _y: f64, _z: f64) -> f64 {
        -1.0
    }

    /// 唯一平衡点 E0 = (0, 0, 0)
    pub fn equilibria() -> [f64; 3] {
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

    /// 最大 Lyapunov 指数 (文献值 ~0.07, 弱混沌)
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
        let cfg = MooreSpiegelConfig::default();
        assert!(approx_eq(cfg.t_param, 6.0, 1e-12));
        assert!(approx_eq(cfg.r_param, 20.0, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = MooreSpiegelSolver::classic(MooreSpiegelConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = MooreSpiegelConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = MooreSpiegelSolver::derivatives(&cfg, x, y, z);
        let nonlinear_damping = cfg.t_param + cfg.r_param * (x * x - 1.0);
        assert!(approx_eq(d[0], y, 1e-12));
        assert!(approx_eq(d[1], z, 1e-12));
        assert!(approx_eq(d[2], -z - nonlinear_damping * y - cfg.r_param * x, 1e-12));
    }

    #[test]
    fn test_derivatives_origin_zero() {
        // 在原点 (0,0,0): dx/dt=0, dy/dt=0, dz/dt=0 (E0 是平衡点)
        let cfg = MooreSpiegelConfig::default();
        let d = MooreSpiegelSolver::derivatives(&cfg, 0.0, 0.0, 0.0);
        for v in d.iter() {
            assert!(v.abs() < 1e-12, "origin derivative = {}", v);
        }
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = MooreSpiegelConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = MooreSpiegelSolver::jacobian(&cfg, x, y, z);
        let nonlinear_damping = cfg.t_param + cfg.r_param * (x * x - 1.0);
        // Row 0: [0, 1, 0]
        assert!(approx_eq(j[0][0], 0.0, 1e-12));
        assert!(approx_eq(j[0][1], 1.0, 1e-12));
        assert!(approx_eq(j[0][2], 0.0, 1e-12));
        // Row 1: [0, 0, 1]
        assert!(approx_eq(j[1][0], 0.0, 1e-12));
        assert!(approx_eq(j[1][1], 0.0, 1e-12));
        assert!(approx_eq(j[1][2], 1.0, 1e-12));
        // Row 2: [-2Rxy - R, -(T+R(x²-1)), -1]
        assert!(approx_eq(j[2][0], -2.0 * cfg.r_param * x * y - cfg.r_param, 1e-12));
        assert!(approx_eq(j[2][1], -nonlinear_damping, 1e-12));
        assert!(approx_eq(j[2][2], -1.0, 1e-12));
    }

    #[test]
    fn test_divergence_constant() {
        // 散度 = -1 (常数, 与参数和位置无关)
        let cfg = MooreSpiegelConfig::default();
        assert!(approx_eq(MooreSpiegelSolver::divergence(&cfg, 0.0, 0.0, 0.0), -1.0, 1e-12));
        assert!(approx_eq(MooreSpiegelSolver::divergence(&cfg, 1.0, 2.0, 3.0), -1.0, 1e-12));
        assert!(approx_eq(MooreSpiegelSolver::divergence(&cfg, -5.0, 7.0, -3.0), -1.0, 1e-12));
        // 改参数也应不变
        let cfg2 = MooreSpiegelConfig { t_param: 10.0, r_param: 30.0, dt: 0.01 };
        assert!(approx_eq(MooreSpiegelSolver::divergence(&cfg2, 0.0, 0.0, 0.0), -1.0, 1e-12));
    }

    #[test]
    fn test_divergence_negative_dissipative() {
        let cfg = MooreSpiegelConfig::default();
        let div = MooreSpiegelSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < 0.0);
        assert!(approx_eq(div, -1.0, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = MooreSpiegelConfig::default();
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5), (2.0, -1.0, 0.3)] {
            let j = MooreSpiegelSolver::jacobian(&cfg, x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = MooreSpiegelSolver::divergence(&cfg, x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_equilibrium_values() {
        let e0 = MooreSpiegelSolver::equilibria();
        assert!(approx_eq(e0[0], 0.0, 1e-12));
        assert!(approx_eq(e0[1], 0.0, 1e-12));
        assert!(approx_eq(e0[2], 0.0, 1e-12));
    }

    #[test]
    fn test_equilibrium_satisfies_equations() {
        let cfg = MooreSpiegelConfig::default();
        let eq = MooreSpiegelSolver::equilibria();
        let d = MooreSpiegelSolver::derivatives(&cfg, eq[0], eq[1], eq[2]);
        for v in d.iter() {
            assert!(v.abs() < 1e-12, "equilibrium derivative = {}", v);
        }
    }

    #[test]
    fn test_step_advances() {
        let mut s = MooreSpiegelSolver::classic(MooreSpiegelConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = MooreSpiegelSolver::classic(MooreSpiegelConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = MooreSpiegelSolver::classic(MooreSpiegelConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = MooreSpiegelSolver::classic(MooreSpiegelConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = MooreSpiegelSolver::classic(MooreSpiegelConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -200.0 && zmax < 200.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = MooreSpiegelSolver::classic(MooreSpiegelConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_lyapunov_positive() {
        // Moore-Spiegel 经典参数是混沌的 (弱混沌, λ ≈ 0.07)
        let mut s = MooreSpiegelSolver::classic(MooreSpiegelConfig::default());
        s.run(200000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        // 弱混沌: 扰动放大较慢, 但仍应放大
        let cfg = MooreSpiegelConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = MooreSpiegelSolver::classic(cfg);
        let mut s2 = MooreSpiegelSolver::new(cfg, 0.1 + d0, 0.0, 0.0);
        s1.run(50000);
        s2.run(50000);
        let d = ((s1.x - s2.x).powi(2) + (s1.y - s2.y).powi(2) + (s1.z - s2.z).powi(2)).sqrt();
        // 弱混沌 λ~0.07, 500 时间单位后 e^35 远超 1
        // 但实际饱和于吸引子尺寸 ~O(1)
        assert!(d > 1e-5, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_volume_monotonic_contraction() {
        // 散度为常数 -1, 体积单调收缩
        let cfg = MooreSpiegelConfig::default();
        let div = MooreSpiegelSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < 0.0);
    }
}
