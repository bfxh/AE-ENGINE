//! Aizawa 吸引子 — 3D 奇异吸引子
//!
//! Aizawa 吸引子是一个 3D 耗散混沌系统, 与 Lorenz/Rössler 拓扑结构
//! 截然不同. 它呈现独特的"双漏斗"或"圆环+螺旋"形态, 是研究 3D 奇异
//! 吸引子多样性的经典例子.
//!
//! 方程:
//!   dx/dt = (z - b) x - d y
//!   dy/dt = d x + (z - b) y
//!   dz/dt = c + a z - z³/3 - (x² + y²)(1 + e z) + f z x³
//!
//! 经典参数: a = 0.95, b = 0.7, c = 0.6, d = 3.5, e = 0.25, f = 0.1
//!
//! 性质:
//!   - 耗散系统: 流的散度 ∇·F = (z-b) + (z-b) + (a - z² - e(x²+y²) + 3 f x²) < 0
//!     长期演化收缩到吸引子 (体积元 → 0)
//!   - 吸引子有界: x,y ∈ [-2, 2], z ∈ [-2, 2] 大致
//!   - Lyapunov 指数: λ₁ > 0 (混沌), λ₂ ≈ 0 (中性), λ₃ < 0 (收缩)
//!   - 吸引子维数 (Kaplan-Yorke): D_KY = 2 + (λ₁+λ₂)/|λ₃| > 2
//!
//! 对称性:
//!   dx/dt, dy/dt 在 (x, y) → (-x, -y) 下反号 (z 不变), 即绕 z 轴
//!   旋转 π 对称. 但 dz/dt 含 f z x³ 项, 在 x → -x 下变号, 故 dz
//!   不具此对称性 (除非 f=0).
//!
//! 平衡点:
//!   令 dx/dt = dy/dt = 0 → (z-b)(x,y) 与 (-d y, d x) 同时为 0
//!   → 要么 d=0 (否), 要么 (x,y)=(0,0)
//!   平衡点在 z 轴上: dz/dt = c + a z - z³/3 = 0
//!   → z³/3 - a z - c = 0, 即 z³ - 3 a z - 3 c = 0
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta), 足够短中期精度.
//!
//! 历史:
//!   Aizawa 吸引子是日本学者研究混沌系统时提出的典型例子, 常见于
//!   混沌系统可视化与分形几何教材.

/// Aizawa 吸引子配置
#[derive(Clone, Copy, Debug)]
pub struct AizawaConfig {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for AizawaConfig {
    fn default() -> Self {
        Self {
            a: 0.95,
            b: 0.7,
            c: 0.6,
            d: 3.5,
            e: 0.25,
            f: 0.1,
            dt: 0.01,
        }
    }
}

/// Aizawa 吸引子求解器
pub struct AizawaSolver {
    pub config: AizawaConfig,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub time: f64,
    pub step_count: u64,
    pub trajectory: Vec<(f64, f64, f64)>,
    /// Lyapunov 累积 (切向量长度)
    pub lyap_sum: f64,
    /// 切向量 (3D)
    pub v: [f64; 3],
}

impl AizawaSolver {
    pub fn new(config: AizawaConfig, x0: f64, y0: f64, z0: f64) -> Self {
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

    pub fn classic(config: AizawaConfig) -> Self {
        Self::new(config, 0.1, 0.0, 0.0)
    }

    /// 右端导数 F(t, [x, y, z])
    pub fn derivatives(cfg: &AizawaConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        let zb = z - cfg.b;
        let r2 = x * x + y * y;
        let dx = zb * x - cfg.d * y;
        let dy = cfg.d * x + zb * y;
        let dz = cfg.c + cfg.a * z - z.powi(3) / 3.0 - r2 * (1.0 + cfg.e * z) + cfg.f * z * x.powi(3);
        [dx, dy, dz]
    }

    /// Jacobian 矩阵 (用于 Lyapunov 与平衡点稳定性分析)
    /// J = [[z-b, -d, x],
    ///      [d, z-b, y],
    ///      [-2x(1+ez)+3fzx², -2y(1+ez), a - z² - e·r² + f·x³]]
    pub fn jacobian(cfg: &AizawaConfig, x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
        let zb = z - cfg.b;
        let r2 = x * x + y * y;
        let df_dx = -2.0 * x * (1.0 + cfg.e * z) + 3.0 * cfg.f * z * x * x;
        let df_dy = -2.0 * y * (1.0 + cfg.e * z);
        let df_dz = cfg.a - z * z - cfg.e * r2 + cfg.f * x.powi(3);
        [
            [zb, -cfg.d, x],
            [cfg.d, zb, y],
            [df_dx, df_dy, df_dz],
        ]
    }

    /// 散度 ∇·F = tr(J) = 2(z-b) + (a - z² - e r² + f x³)
    /// 耗散系统: 散度 < 0 (吸引子存在)
    pub fn divergence(cfg: &AizawaConfig, x: f64, y: f64, z: f64) -> f64 {
        let zb = z - cfg.b;
        let r2 = x * x + y * y;
        2.0 * zb + (cfg.a - z * z - cfg.e * r2 + cfg.f * x.powi(3))
    }

    /// z 轴上的平衡点: z³ - 3a z - 3c = 0
    /// 返回所有实根 (1 个或 3 个)
    pub fn z_axis_equilibria(cfg: &AizawaConfig) -> Vec<f64> {
        // 解 z³ - 3a z - 3c = 0
        let a = cfg.a;
        let c = cfg.c;
        // 判别式 Δ = (q/2)² + (p/3)³, 其中 p = -3a, q = -3c
        let p = -3.0 * a;
        let q = -3.0 * c;
        let disc = (q / 2.0).powi(2) + (p / 3.0).powi(3);
        if disc > 0.0 {
            // 一个实根
            let s = disc.sqrt();
            let u = (-q / 2.0 + s).cbrt();
            let v = (-q / 2.0 - s).cbrt();
            vec![u + v]
        } else if disc.abs() < 1e-12 {
            // 三重根或两重根
            if p.abs() < 1e-12 {
                vec![0.0]
            } else {
                let r1 = 3.0 * q / p;
                let r2 = -3.0 * q / (2.0 * p);
                vec![r1, r2]
            }
        } else {
            // 三个实根 (三角形式)
            let r = (-p / 3.0).powi(3).sqrt();
            let phi = (-q / (2.0 * r)).acos();
            let r3 = r.cbrt();
            let r1 = 2.0 * r3 * (phi / 3.0).cos();
            let r2 = 2.0 * r3 * ((phi + 2.0 * std::f64::consts::PI) / 3.0).cos();
            let r3 = 2.0 * r3 * ((phi + 4.0 * std::f64::consts::PI) / 3.0).cos();
            vec![r1, r2, r3]
        }
    }

    /// 单步 RK4 推进
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

        // Lyapunov (连续系统): 变分方程 dv/dt = J v, 前向欧拉离散
        // v_{n+1} = v_n + dt J v_n = (I + dt J) v_n, 归一化, 累积 ln|v_{n+1}|/|v_n|
        // λ = (1/T) Σ ln|v_{n+1}|/|v_n|, T 为总时间
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

    /// 多步推进
    pub fn run(&mut self, n_steps: usize) {
        for _ in 0..n_steps {
            self.step();
        }
    }

    /// 主 Lyapunov 指数 λ₁
    pub fn lyapunov_exponent(&self) -> f64 {
        if self.step_count == 0 {
            return 0.0;
        }
        self.lyap_sum / (self.time.max(1e-12))
    }

    /// 检查 NaN/Inf
    pub fn has_nan(&self) -> bool {
        !self.x.is_finite() || !self.y.is_finite() || !self.z.is_finite()
    }

    /// 吸引子近似边界 (基于轨迹 min/max)
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

    /// 检查是否逃逸 (远离吸引子)
    pub fn has_escaped(&self) -> bool {
        self.x.abs() > 50.0 || self.y.abs() > 50.0 || self.z.abs() > 50.0 || self.has_nan()
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
        let cfg = AizawaConfig::default();
        assert!(approx_eq(cfg.a, 0.95, 1e-12));
        assert!(approx_eq(cfg.b, 0.7, 1e-12));
        assert!(approx_eq(cfg.c, 0.6, 1e-12));
        assert!(approx_eq(cfg.d, 3.5, 1e-12));
        assert!(approx_eq(cfg.e, 0.25, 1e-12));
        assert!(approx_eq(cfg.f, 0.1, 1e-12));
        assert!(approx_eq(cfg.dt, 0.01, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = AizawaSolver::classic(AizawaConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
        assert!(approx_eq(s.x, 0.1, 1e-12));
    }

    #[test]
    fn test_derivatives_origin() {
        // 在原点 (0,0,0): dx = (0-b)*0 - d*0 = 0; dy = d*0 + (0-b)*0 = 0
        // dz = c + a*0 - 0 - 0*(1+0) + 0 = c
        let cfg = AizawaConfig::default();
        let d = AizawaSolver::derivatives(&cfg, 0.0, 0.0, 0.0);
        assert!(approx_eq(d[0], 0.0, 1e-12));
        assert!(approx_eq(d[1], 0.0, 1e-12));
        assert!(approx_eq(d[2], cfg.c, 1e-12));
    }

    #[test]
    fn test_derivatives_z_axis() {
        // z 轴上 (0,0,z): dx = 0, dy = 0, dz = c + a z - z³/3
        let cfg = AizawaConfig::default();
        let z = 0.5_f64;
        let d = AizawaSolver::derivatives(&cfg, 0.0, 0.0, z);
        assert!(approx_eq(d[0], 0.0, 1e-12));
        assert!(approx_eq(d[1], 0.0, 1e-12));
        let expected = cfg.c + cfg.a * z - z.powi(3) / 3.0;
        assert!(approx_eq(d[2], expected, 1e-12));
    }

    #[test]
    fn test_z_axis_rotation_symmetry() {
        // dx, dy 在 (x,y) → (-x,-y) 下反号 (z 不变)
        // dz 因含 f z x³ 项, 在 x → -x 下变号, 不具此对称
        let cfg = AizawaConfig::default();
        let (x, y, z) = (0.5_f64, 0.3_f64, 0.4_f64);
        let d1 = AizawaSolver::derivatives(&cfg, x, y, z);
        let d2 = AizawaSolver::derivatives(&cfg, -x, -y, z);
        assert!(approx_eq(d1[0], -d2[0], 1e-12), "dx: {} vs {}", d1[0], d2[0]);
        assert!(approx_eq(d1[1], -d2[1], 1e-12), "dy: {} vs {}", d1[1], d2[1]);
        // dz 不对称 (f z x³ 项): d1[2] - d2[2] = 2 f z x³
        let expected_diff = 2.0 * cfg.f * z * x.powi(3);
        assert!(approx_eq(d1[2] - d2[2], expected_diff, 1e-12));
    }

    #[test]
    fn test_step_advances() {
        let mut s = AizawaSolver::classic(AizawaConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = AizawaSolver::classic(AizawaConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = AizawaSolver::classic(AizawaConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = AizawaSolver::classic(AizawaConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        // 经典 Aizawa 吸引子大致范围
        assert!(xmin > -5.0 && xmax < 5.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -5.0 && ymax < 5.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -5.0 && zmax < 5.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_dissipation_negative_divergence() {
        // 耗散系统: 平均散度应 < 0
        let cfg = AizawaConfig::default();
        let mut s = AizawaSolver::classic(cfg);
        s.run(20000);
        let mut sum = 0.0;
        let n = s.trajectory.len();
        for &(x, y, z) in &s.trajectory {
            sum += AizawaSolver::divergence(&cfg, x, y, z);
        }
        let avg = sum / n as f64;
        assert!(avg < 0.0, "average divergence should be negative: {}", avg);
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = AizawaConfig::default();
        let j = AizawaSolver::jacobian(&cfg, 0.3, 0.4, 0.5);
        // 检查 J[0] = [z-b, -d, x]
        assert!(approx_eq(j[0][0], 0.5 - cfg.b, 1e-12));
        assert!(approx_eq(j[0][1], -cfg.d, 1e-12));
        assert!(approx_eq(j[0][2], 0.3, 1e-12));
        // J[1] = [d, z-b, y]
        assert!(approx_eq(j[1][0], cfg.d, 1e-12));
        assert!(approx_eq(j[1][1], 0.5 - cfg.b, 1e-12));
        assert!(approx_eq(j[1][2], 0.4, 1e-12));
    }

    #[test]
    fn test_z_axis_equilibria_satisfy_equation() {
        // z 轴平衡点: z³ - 3 a z - 3 c = 0
        let cfg = AizawaConfig::default();
        let eqs = AizawaSolver::z_axis_equilibria(&cfg);
        assert!(!eqs.is_empty());
        for &z in &eqs {
            let residual = z.powi(3) - 3.0 * cfg.a * z - 3.0 * cfg.c;
            assert!(approx_eq(residual, 0.0, 1e-8), "z={}, residual={}", z, residual);
        }
    }

    #[test]
    fn test_z_axis_equilibrium_is_equilibrium() {
        // 在 (0, 0, z*) 处导数应为 0
        let cfg = AizawaConfig::default();
        let eqs = AizawaSolver::z_axis_equilibria(&cfg);
        for &z in &eqs {
            let d = AizawaSolver::derivatives(&cfg, 0.0, 0.0, z);
            let mag = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            assert!(mag < 1e-6, "at z={}, |F|={}", z, mag);
        }
    }

    #[test]
    fn test_lyapunov_positive() {
        // 经典 Aizawa 应有正 Lyapunov 指数 (混沌)
        let mut s = AizawaSolver::classic(AizawaConfig::default());
        s.run(50000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        // 两条相近轨道应分离. Aizawa Lyapunov 较小 (~0.1-0.3),
        // 需要足够长时间让 Lyapunov 放大显现
        let cfg = AizawaConfig::default();
        let mut s1 = AizawaSolver::new(cfg, 0.1, 0.0, 0.0);
        let mut s2 = AizawaSolver::new(cfg, 0.1 + 1e-10, 0.0, 0.0);
        for _ in 0..30000 {
            s1.step();
            s2.step();
        }
        let dx = s1.x - s2.x;
        let dy = s1.y - s2.y;
        let dz = s1.z - s2.z;
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        // t=300, λ≈0.15, 放大 e^45 ≈ 3e19 → 饱和, d 应明显放大
        assert!(d > 1e-3, "should be amplified: {}", d);
    }

    #[test]
    fn test_escape_for_large_initial() {
        let cfg = AizawaConfig::default();
        let mut s = AizawaSolver::new(cfg, 100.0, 100.0, 100.0);
        s.run(500);
        assert!(s.has_escaped() || s.has_nan(), "should escape from large initial");
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = AizawaSolver::classic(AizawaConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_divergence_at_origin() {
        // 原点处散度 = 2(0-b) + (a - 0 - 0 + 0) = -2b + a
        let cfg = AizawaConfig::default();
        let div = AizawaSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        let expected = -2.0 * cfg.b + cfg.a;
        assert!(approx_eq(div, expected, 1e-12));
    }

    #[test]
    fn test_divergence_negative_at_origin() {
        // 经典参数下 -2b + a = -1.4 + 0.95 = -0.45 < 0
        let cfg = AizawaConfig::default();
        let div = AizawaSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < 0.0, "divergence at origin: {}", div);
    }

    #[test]
    fn test_attractor_range_known() {
        // 经典参数下吸引子大致 x,y ∈ [-1.5, 1.5], z ∈ [-1.5, 1.5]
        let mut s = AizawaSolver::classic(AizawaConfig::default());
        s.run(50000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -3.0, "xmin: {}", xmin);
        assert!(xmax < 3.0, "xmax: {}", xmax);
        assert!(ymin > -3.0, "ymin: {}", ymin);
        assert!(ymax < 3.0, "ymax: {}", ymax);
        assert!(zmin > -3.0, "zmin: {}", zmin);
        assert!(zmax < 3.0, "zmax: {}", zmax);
    }

    #[test]
    fn test_volume_contraction() {
        // 耗散: 长期演化后轨迹不应发散
        let mut s = AizawaSolver::classic(AizawaConfig::default());
        s.run(10000);
        let bounds1 = s.attractor_bounds();
        s.run(20000);
        let bounds2 = s.attractor_bounds();
        // 边界不应无限增长 (耗散系统)
        let r1 = (
            bounds1.1 - bounds1.0,
            bounds1.3 - bounds1.2,
            bounds1.5 - bounds1.4,
        );
        let r2 = (
            bounds2.1 - bounds2.0,
            bounds2.3 - bounds2.2,
            bounds2.5 - bounds2.4,
        );
        // 范围不应显著扩大
        assert!(r2.0 < r1.0 * 2.0, "x range grew: {} -> {}", r1.0, r2.0);
        assert!(r2.1 < r1.1 * 2.0, "y range grew: {} -> {}", r1.1, r2.1);
        assert!(r2.2 < r1.2 * 2.0, "z range grew: {} -> {}", r1.2, r2.2);
    }

    #[test]
    fn test_classic_initial_conds_in_attractor() {
        // 经典初值 (0.1, 0, 0) 应能进入吸引子 (不逃逸)
        let mut s = AizawaSolver::classic(AizawaConfig::default());
        s.run(50000);
        assert!(!s.has_escaped());
    }

    #[test]
    fn test_lyapunov_finite_value() {
        // Lyapunov 应为有限值
        let mut s = AizawaSolver::classic(AizawaConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_step_time_advances() {
        let mut s = AizawaSolver::classic(AizawaConfig::default());
        let dt = s.config.dt;
        s.step();
        assert!(approx_eq(s.time, dt, 1e-12));
        s.step();
        assert!(approx_eq(s.time, 2.0 * dt, 1e-12));
    }

    #[test]
    fn test_jacobian_at_origin() {
        // 原点 J = [[-b, -d, 0], [d, -b, 0], [0, 0, a]]
        let cfg = AizawaConfig::default();
        let j = AizawaSolver::jacobian(&cfg, 0.0, 0.0, 0.0);
        assert!(approx_eq(j[0][0], -cfg.b, 1e-12));
        assert!(approx_eq(j[0][1], -cfg.d, 1e-12));
        assert!(approx_eq(j[0][2], 0.0, 1e-12));
        assert!(approx_eq(j[1][0], cfg.d, 1e-12));
        assert!(approx_eq(j[1][1], -cfg.b, 1e-12));
        assert!(approx_eq(j[1][2], 0.0, 1e-12));
        assert!(approx_eq(j[2][0], 0.0, 1e-12));
        assert!(approx_eq(j[2][1], 0.0, 1e-12));
        assert!(approx_eq(j[2][2], cfg.a, 1e-12));
    }

    #[test]
    fn test_origin_jacobian_eigenvalues() {
        // 原点 J 的特征值: xy 块 [-b, -d; d, -b] 特征值 -b ± i d
        // z 方向特征值 a
        let cfg = AizawaConfig::default();
        let j = AizawaSolver::jacobian(&cfg, 0.0, 0.0, 0.0);
        // xy 块迹 = -2b, det = b² + d²
        let tr_xy = j[0][0] + j[1][1];
        let det_xy = j[0][0] * j[1][1] - j[0][1] * j[1][0];
        assert!(approx_eq(tr_xy, -2.0 * cfg.b, 1e-12));
        assert!(approx_eq(det_xy, cfg.b * cfg.b + cfg.d * cfg.d, 1e-12));
        // z 方向 a > 0 (不稳定方向)
        assert!(j[2][2] > 0.0, "z direction should be unstable at origin");
    }
}
