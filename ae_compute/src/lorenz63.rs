//! Lorenz 63 Attractor — Lorenz 1963 大气对流混沌系统 (3D)
//!
//! Edward N. Lorenz 1963 年在研究大气对流时发现的三变量自治混沌系统,
//! 是混沌理论的奠基性例子, 也是"蝴蝶效应"一词的起源. Lorenz 63 是
//! Saltzman 1962 年 Bénard 对流方程的 3 模截断, 展示了确定性系统中
//! 的非周期运动和对初始条件的敏感依赖.
//!
//! 物理背景: Rayleigh-Bénard 对流
//!   流体夹在两平板之间, 下板加热上板冷却. 当温度梯度超过临界值时,
//!   流体开始对流. Saltzman 1962 年用偏微分方程描述此过程, Lorenz
//!   用 Galerkin 投影保留 3 个模态, 得到 3 变量 ODE:
//!     x: 对流强度 (速度模态)
//!     y: 水平温度梯度 (温度模态 1)
//!     z: 垂直温度梯度偏差 (温度模态 2)
//!
//! 状态方程 (Lorenz 1963):
//!   dx/dt = σ(y - x)
//!   dy/dt = x(ρ - z) - y
//!   dz/dt = xy - βz
//!
//! 各项物理意义:
//!   - σ(y - x): 速度场由温度梯度驱动, Prandtl 数 σ 调节惯性/粘性比
//!   + x(ρ - z): 温度模态 y 受对流和浮力驱动, Rayleigh 数 ρ 控制不稳定性
//!   - y: 温度模态 y 的耗散
//!   + xy: 非线性对流项 (速度场平流温度梯度)
//!   - βz: 温度模态 z 的耗散 (垂直分层松散)
//!
//! 经典参数 (Lorenz 1963): σ = 10, ρ = 28, β = 8/3
//! 经典初值: (x₀, y₀, z₀) = (1, 1, 1) 或 (0, 1, 0)
//!
//! 性质:
//!   - 散度 ∇·F = tr(J) = -σ - 1 - β (常数负, 耗散)
//!     经典参数: div = -10 - 1 - 8/3 = -41/3 ≈ -13.667
//!     体积收缩率 e^(-13.667 t), 强耗散
//!   - 平衡点 (利用 y=x, z=x²/β):
//!     0 = x(ρ - z - 1) → x = 0 或 z = ρ - 1
//!     E0 = (0, 0, 0)                          (无对流, 静止)
//!     E+ = (+√(β(ρ-1)), +√(β(ρ-1)), ρ-1)     (顺时针对流)
//!     E- = (-√(β(ρ-1)), -√(β(ρ-1)), ρ-1)     (逆时针对流)
//!     经典参数: β(ρ-1) = (8/3)·27 = 72
//!     E± = (±√72, ±√72, 27) ≈ (±8.485, ±8.485, 27)
//!   - Lyapunov 谱 (经典参数, 文献值):
//!     λ₁ ≈ +0.906  (正, 主混沌方向, 蝴蝶效应根源)
//!     λ₂ = 0       (沿轨道切向)
//!     λ₃ ≈ -14.572 (负, 体积收缩)
//!     和 ≈ -13.666 (与散度一致)
//!   - Kaplan-Yorke 维数 D_KY ≈ 2 + λ₁/|λ₃| ≈ 2.062
//!   - 吸引子形态: 双叶"蝴蝶" (butterfly), 围绕 E+ 和 E- 螺旋
//!   - 蝴蝶效应: 初始小扰动以 e^(λ₁ t) 指数放大
//!
//! 分岔行为:
//!   - ρ < 1: E0 稳定 (无对流)
//!   - ρ = 1: 鞍结分岔, E0 失稳, E± 出现
//!   - 1 < ρ < ρ_H ≈ 24.74: E± 稳定 (稳态对流)
//!   - ρ > ρ_H: E± 失稳, 产生奇怪吸引子 (混沌)
//!   - ρ = 28: 经典混沌参数
//!
//! 历史:
//!   Lorenz, E. N. 1963. "Deterministic nonperiodic flow."
//!   J. Atmos. Sci. 20, 130-141. (混沌理论奠基论文)
//!   Saltzman, B. 1962. "Finite amplitude free convection as an
//!   initial value problem-I." J. Atmos. Sci. 19, 329-341. (原始对流模型)
//!   Tucker, W. 2002. "A rigorous ODE solver and Smale's 14th problem."
//!   Found. Comput. Math. 2, 53-117. (证明 Lorenz 吸引子确实存在)
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v

/// Lorenz 63 系统配置 (3 参数 + dt)
#[derive(Clone, Copy, Debug)]
pub struct Lorenz63Config {
    /// Prandtl 数 σ (经典 10)
    pub sigma: f64,
    /// Rayleigh 数 ρ (经典 28, 混沌阈值 ~24.74)
    pub rho: f64,
    /// 几何参数 β (经典 8/3)
    pub beta: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for Lorenz63Config {
    fn default() -> Self {
        Self {
            sigma: 10.0,
            rho: 28.0,
            beta: 8.0 / 3.0,
            dt: 0.005,
        }
    }
}

/// Lorenz 63 求解器 (3D, 跟踪最大 Lyapunov 指数)
pub struct Lorenz63Solver {
    pub config: Lorenz63Config,
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

impl Lorenz63Solver {
    pub fn new(config: Lorenz63Config, x0: f64, y0: f64, z0: f64) -> Self {
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

    /// 经典初值 (1, 1, 1)
    pub fn classic(config: Lorenz63Config) -> Self {
        Self::new(config, 1.0, 1.0, 1.0)
    }

    /// Lorenz 原论文初值 (0, 1, 0)
    pub fn lorenz_original(config: Lorenz63Config) -> Self {
        Self::new(config, 0.0, 1.0, 0.0)
    }

    /// 右端导数 F = [σ(y-x), x(ρ-z)-y, xy-βz]
    pub fn derivatives(cfg: &Lorenz63Config, x: f64, y: f64, z: f64) -> [f64; 3] {
        [
            cfg.sigma * (y - x),
            x * (cfg.rho - z) - y,
            x * y - cfg.beta * z,
        ]
    }

    /// Jacobian:
    /// J = [[-σ,      σ,    0 ],
    ///      [ρ - z,  -1,   -x ],
    ///      [y,       x,   -β ]]
    pub fn jacobian(cfg: &Lorenz63Config, x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
        [
            [-cfg.sigma, cfg.sigma, 0.0],
            [cfg.rho - z, -1.0, -x],
            [y, x, -cfg.beta],
        ]
    }

    /// 散度 ∇·F = tr(J) = -σ - 1 - β (常数负)
    pub fn divergence(cfg: &Lorenz63Config, _x: f64, _y: f64, _z: f64) -> f64 {
        -cfg.sigma - 1.0 - cfg.beta
    }

    /// 计算三个平衡点:
    /// E0 = (0, 0, 0)
    /// E+ = (+√(β(ρ-1)), +√(β(ρ-1)), ρ-1)  当 ρ > 1
    /// E- = (-√(β(ρ-1)), -√(β(ρ-1)), ρ-1)  当 ρ > 1
    pub fn equilibria(cfg: &Lorenz63Config) -> ([f64; 3], [f64; 3], [f64; 3]) {
        let arg = cfg.beta * (cfg.rho - 1.0);
        let r = arg.max(0.0).sqrt();
        (
            [0.0, 0.0, 0.0],
            [r, r, cfg.rho - 1.0],
            [-r, -r, cfg.rho - 1.0],
        )
    }

    /// Hopf 分岔临界值 ρ_H = σ(σ+β+3)/(σ-β-1), 当 σ-β-1 > 0
    /// 经典参数 σ=10, β=8/3: ρ_H = 10·(10+8/3+3)/(10-8/3-1) = 470/19 ≈ 24.74
    pub fn hopf_bifurcation_rho(cfg: &Lorenz63Config) -> f64 {
        let denom = cfg.sigma - cfg.beta - 1.0;
        if denom > 0.0 {
            cfg.sigma * (cfg.sigma + cfg.beta + 3.0) / denom
        } else {
            f64::NAN
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

    /// 最大 Lyapunov 指数 (文献值 ~0.906)
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
        let cfg = Lorenz63Config::default();
        assert!(approx_eq(cfg.sigma, 10.0, 1e-12));
        assert!(approx_eq(cfg.rho, 28.0, 1e-12));
        assert!(approx_eq(cfg.beta, 8.0 / 3.0, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = Lorenz63Solver::classic(Lorenz63Config::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_two_initial_conditions() {
        let s1 = Lorenz63Solver::classic(Lorenz63Config::default());
        let s2 = Lorenz63Solver::lorenz_original(Lorenz63Config::default());
        assert!((s1.x - s2.x).abs() > 0.5);
        assert!((s1.z - s2.z).abs() > 0.5);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = Lorenz63Config::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = Lorenz63Solver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], cfg.sigma * (y - x), 1e-12));
        assert!(approx_eq(d[1], x * (cfg.rho - z) - y, 1e-12));
        assert!(approx_eq(d[2], x * y - cfg.beta * z, 1e-12));
    }

    #[test]
    fn test_derivatives_origin_zero() {
        // 在原点 (0,0,0), 所有导数 = 0
        let cfg = Lorenz63Config::default();
        let d = Lorenz63Solver::derivatives(&cfg, 0.0, 0.0, 0.0);
        for v in d.iter() {
            assert!(v.abs() < 1e-12);
        }
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = Lorenz63Config::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = Lorenz63Solver::jacobian(&cfg, x, y, z);
        // Row 0: [-σ, σ, 0]
        assert!(approx_eq(j[0][0], -cfg.sigma, 1e-12));
        assert!(approx_eq(j[0][1], cfg.sigma, 1e-12));
        assert!(approx_eq(j[0][2], 0.0, 1e-12));
        // Row 1: [ρ-z, -1, -x]
        assert!(approx_eq(j[1][0], cfg.rho - z, 1e-12));
        assert!(approx_eq(j[1][1], -1.0, 1e-12));
        assert!(approx_eq(j[1][2], -x, 1e-12));
        // Row 2: [y, x, -β]
        assert!(approx_eq(j[2][0], y, 1e-12));
        assert!(approx_eq(j[2][1], x, 1e-12));
        assert!(approx_eq(j[2][2], -cfg.beta, 1e-12));
    }

    #[test]
    fn test_divergence_constant() {
        // 散度 = -σ - 1 - β (常数)
        let cfg = Lorenz63Config::default();
        let expected = -cfg.sigma - 1.0 - cfg.beta;
        assert!(approx_eq(Lorenz63Solver::divergence(&cfg, 0.0, 0.0, 0.0), expected, 1e-12));
        assert!(approx_eq(Lorenz63Solver::divergence(&cfg, 1.0, 2.0, 3.0), expected, 1e-12));
        assert!(approx_eq(Lorenz63Solver::divergence(&cfg, -5.0, 7.0, -3.0), expected, 1e-12));
    }

    #[test]
    fn test_divergence_negative_dissipative() {
        let cfg = Lorenz63Config::default();
        let div = Lorenz63Solver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < 0.0, "divergence should be negative: {}", div);
        // 经典参数: -10 - 1 - 8/3 = -41/3 ≈ -13.667
        assert!(approx_eq(div, -41.0 / 3.0, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = Lorenz63Config::default();
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5), (5.0, -3.0, 20.0)] {
            let j = Lorenz63Solver::jacobian(&cfg, x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = Lorenz63Solver::divergence(&cfg, x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_equilibria_values() {
        let cfg = Lorenz63Config::default();
        let (e0, e1, e2) = Lorenz63Solver::equilibria(&cfg);
        // E0 = (0, 0, 0)
        assert!(approx_eq(e0[0], 0.0, 1e-12));
        assert!(approx_eq(e0[1], 0.0, 1e-12));
        assert!(approx_eq(e0[2], 0.0, 1e-12));
        // E1, E2: x = y, z = ρ-1
        assert!(approx_eq(e1[0], e1[1], 1e-12));
        assert!(approx_eq(e2[0], e2[1], 1e-12));
        assert!(approx_eq(e1[2], cfg.rho - 1.0, 1e-12));
        assert!(approx_eq(e2[2], cfg.rho - 1.0, 1e-12));
        // E1 = -E2 (反演对称)
        assert!(approx_eq(e1[0], -e2[0], 1e-12));
        // x² = β(ρ-1)
        let r2 = cfg.beta * (cfg.rho - 1.0);
        assert!(approx_eq(e1[0] * e1[0], r2, 1e-9));
    }

    #[test]
    fn test_equilibria_satisfy_equations() {
        let cfg = Lorenz63Config::default();
        let (e0, e1, e2) = Lorenz63Solver::equilibria(&cfg);
        for eq in [e0, e1, e2] {
            let d = Lorenz63Solver::derivatives(&cfg, eq[0], eq[1], eq[2]);
            for v in d.iter() {
                assert!(v.abs() < 1e-9, "equilibrium derivative = {}", v);
            }
        }
    }

    #[test]
    fn test_hopf_bifurcation_value() {
        // 经典参数下 ρ_H ≈ 470/19 ≈ 24.74
        let cfg = Lorenz63Config::default();
        let rho_h = Lorenz63Solver::hopf_bifurcation_rho(&cfg);
        assert!(approx_eq(rho_h, 470.0 / 19.0, 1e-9));
        assert!(rho_h < cfg.rho, "rho={} should be above Hopf threshold={}", cfg.rho, rho_h);
    }

    #[test]
    fn test_step_advances() {
        let mut s = Lorenz63Solver::classic(Lorenz63Config::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = Lorenz63Solver::classic(Lorenz63Config::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = Lorenz63Solver::classic(Lorenz63Config::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = Lorenz63Solver::classic(Lorenz63Config::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        // Lorenz 63 吸引子典型范围: |x|, |y| < 25, z ∈ [0, 50]
        let mut s = Lorenz63Solver::classic(Lorenz63Config::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -10.0 && zmax < 100.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        // Lorenz 63 经典参数是混沌的, λ > 0 (文献值 ~0.906)
        let mut s = Lorenz63Solver::classic(Lorenz63Config::default());
        s.run(100000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = Lorenz63Solver::classic(Lorenz63Config::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        // 蝴蝶效应: 微小扰动指数放大
        let cfg = Lorenz63Config::default();
        let d0 = 1e-6_f64;
        let mut s1 = Lorenz63Solver::classic(cfg);
        let mut s2 = Lorenz63Solver::new(cfg, 1.0 + d0, 1.0, 1.0);
        s1.run(20000);
        s2.run(20000);
        let d = ((s1.x - s2.x).powi(2) + (s1.y - s2.y).powi(2) + (s1.z - s2.z).powi(2)).sqrt();
        // 混沌放大: Lorenz 63 Lyapunov ~0.9, 100 时间单位后 e^90 远超 1
        // 但有限精度 + 切空间饱和, 实际 d ~ O(1)
        assert!(d > 1e-3, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_volume_monotonic_contraction() {
        // 散度为常数负, 体积单调收缩
        let cfg = Lorenz63Config::default();
        let div = Lorenz63Solver::divergence(&cfg, 0.0, 0.0, 0.0);
        // 散度 < 0 (耗散)
        assert!(div < 0.0);
        // 体积变化率 dV/dt = div · V, div < 0 → V 单调减
    }

    #[test]
    fn test_escape_for_large_initial() {
        // 远离吸引子的初值会发散
        let mut s = Lorenz63Solver::new(Lorenz63Config::default(), 1000.0, 1000.0, 1000.0);
        s.run(1000);
        assert!(s.has_escaped(), "should escape: x={} y={} z={}", s.x, s.y, s.z);
    }
}
