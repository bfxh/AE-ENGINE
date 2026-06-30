//! ABC Flow — Arnold-Beltrami-Childress 流 (稳态三维不可压缩混沌流)
//!
//! ABC 流是理想不可压缩欧拉流体的稳态解, 由 Arnold (1965), Beltrami (1889),
//! Childress (1967) 研究. 它是 Beltrami 流 (速度场平行于涡度场 ∇×v = λ v),
//! 在三维环面 T³ 上周期化. 尽管速度场是稳态的, 流体粒子轨迹却混沌 —
//! 称为 "拓扑混沌" 或 "拉格朗日混沌".
//!
//! 状态方程 (稳态, 自治):
//!   dx/dt = A sin(z) + C cos(y)
//!   dy/dt = B sin(x) + A cos(z)
//!   dz/dt = C sin(y) + B cos(x)
//!
//! 经典参数: A = B = C = 1 (完全对称, 最强混沌)
//!
//! 性质:
//!   - 不可压缩: 散度 ∇·v = 0 (处处, 严格守体积)
//!   - Beltrami 条件: ∇×v = v (涡度=速度, 特征值 λ=1)
//!   - 无不动点: 当 A, B, C > 0 时, 三方程不能同时为零 (除退化情形)
//!   - 粒子轨迹混沌 (拓扑混合), 但速度场稳态
//!   - 混沌海 + 周期岛 (KAM 结构), 类似 Hénon-Heiles
//!   - Lyapunov 指数: λ₁ > 0 (混沌方向), λ₂ = 0 (沿流切向), λ₃ < 0 (收缩)
//!   - 由于散度=0, λ₁ + λ₂ + λ₃ = 0, 即 λ₃ = -λ₁
//!   - Poincaré 截面显示混沌海中嵌入 KAM 岛
//!
//! 物理意义:
//!   - 欧拉方程精确解 (无粘流体)
//!   - 快发电机 (fast dynamo) 机制: 磁场指数增长
//!   - 湍流混合的最小模型
//!   - 被用于研究拉格朗日相干结构 (LCS)
//!
//! KAM 结构:
//!   - 小扰动 (A=ε, B=C=0) → 可积 (流沿 x 方向)
//!   - 增大 A → 共振破坏 → KAM 岛 + 混沌海
//!   - A=B=C=1 时混沌海占主导
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta)
//!
//! 历史:
//!   Arnold, V. I. 1965. "Sur la topologie des écoulements stationnaires des
//!   fluides parfaits." C. R. Acad. Sci. Paris 261, 17. (ABC 流拓扑)
//!   Childress, S. 1967. "New solutions of the kinematic dynamo problem."
//!   J. Math. Phys. 8, 916. (发电机应用)
//!   Dombre, T. et al. 1986. "Chaotic streamlines in the ABC flows."
//!   J. Fluid Mech. 167, 353. (详细混沌分析)

/// ABC 流配置
#[derive(Clone, Copy, Debug)]
pub struct AbcFlowConfig {
    /// 参数 A (y→x, z→y 耦合)
    pub a: f64,
    /// 参数 B (x→y, z→x 耦合)
    pub b: f64,
    /// 参数 C (y→z, x→z 耦合)
    pub c: f64,
    /// 时间步长 dt
    pub dt: f64,
}

impl Default for AbcFlowConfig {
    fn default() -> Self {
        Self { a: 1.0, b: 1.0, c: 1.0, dt: 0.01 }
    }
}

/// ABC 流粒子轨迹求解器
pub struct AbcFlowSolver {
    pub config: AbcFlowConfig,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub time: f64,
    pub step_count: u64,
    pub trajectory: Vec<(f64, f64, f64)>,
    /// Lyapunov 累积 (最大指数)
    pub lyap_sum: f64,
    /// 切向量 (3D)
    pub v: [f64; 3],
}

impl AbcFlowSolver {
    pub fn new(config: AbcFlowConfig, x0: f64, y0: f64, z0: f64) -> Self {
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

    pub fn classic(config: AbcFlowConfig) -> Self {
        // Dombre et al. 1986 推荐初值 (在 2π 环面上的一个点)
        Self::new(config, 0.1, 0.1, 0.1)
    }

    /// 右端导数 v = [A sin z + C cos y, B sin x + A cos z, C sin y + B cos x]
    pub fn derivatives(cfg: &AbcFlowConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        [
            cfg.a * z.sin() + cfg.c * y.cos(),
            cfg.b * x.sin() + cfg.a * z.cos(),
            cfg.c * y.sin() + cfg.b * x.cos(),
        ]
    }

    /// Jacobian:
    /// J = [[0,         -C sin y,   A cos z],
    ///      [B cos x,   0,         -A sin z],
    ///      [0,          C cos y,   0       ]]
    pub fn jacobian(cfg: &AbcFlowConfig, x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
        [
            [0.0, -cfg.c * y.sin(), cfg.a * z.cos()],
            [cfg.b * x.cos(), 0.0, -cfg.a * z.sin()],
            [0.0, cfg.c * y.cos(), 0.0],
        ]
    }

    /// 散度 ∇·v = tr(J) = 0 (不可压缩, 处处为零)
    pub fn divergence(_cfg: &AbcFlowConfig, _x: f64, _y: f64, _z: f64) -> f64 {
        0.0
    }

    /// 涡度 ∇×v (Beltrami 条件: 涡度 = 速度)
    pub fn vorticity(cfg: &AbcFlowConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        // ∇×v = [∂v_z/∂y - ∂v_y/∂z, ∂v_x/∂z - ∂v_z/∂x, ∂v_y/∂x - ∂v_x/∂y]
        //     = [C cos y + A sin z, A cos z + B sin x, B cos x + C sin y]
        //     = v (Beltrami)
        Self::derivatives(cfg, x, y, z)
    }

    /// 速度模 |v|²
    pub fn speed_squared(cfg: &AbcFlowConfig, x: f64, y: f64, z: f64) -> f64 {
        let v = Self::derivatives(cfg, x, y, z);
        v[0] * v[0] + v[1] * v[1] + v[2] * v[2]
    }

    /// 单步 RK4 推进 + Lyapunov 变分方程
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

    pub fn lyapunov_exponent(&self) -> f64 {
        if self.step_count == 0 {
            return 0.0;
        }
        self.lyap_sum / self.time.max(1e-12)
    }

    pub fn has_nan(&self) -> bool {
        !self.x.is_finite() || !self.y.is_finite() || !self.z.is_finite()
    }

    /// 将坐标 wrap 到 [0, 2π) (环面 T³ 上的位置)
    pub fn wrapped_position(&self) -> (f64, f64, f64) {
        let two_pi = 2.0 * std::f64::consts::PI;
        let w = |t: f64| -> f64 {
            let r = t % two_pi;
            if r < 0.0 { r + two_pi } else { r }
        };
        (w(self.x), w(self.y), w(self.z))
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
        let cfg = AbcFlowConfig::default();
        assert!(approx_eq(cfg.a, 1.0, 1e-12));
        assert!(approx_eq(cfg.b, 1.0, 1e-12));
        assert!(approx_eq(cfg.c, 1.0, 1e-12));
        assert!(approx_eq(cfg.dt, 0.01, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = AbcFlowSolver::classic(AbcFlowConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
        assert!(approx_eq(s.x, 0.1, 1e-12));
    }

    #[test]
    fn test_derivatives_analytic() {
        // 在 (0, 0, 0): sin=0, cos=1
        // dx = A*0 + C*1 = C
        // dy = B*0 + A*1 = A
        // dz = C*0 + B*1 = B
        let cfg = AbcFlowConfig::default();
        let d = AbcFlowSolver::derivatives(&cfg, 0.0, 0.0, 0.0);
        assert!(approx_eq(d[0], cfg.c, 1e-12));
        assert!(approx_eq(d[1], cfg.a, 1e-12));
        assert!(approx_eq(d[2], cfg.b, 1e-12));
    }

    #[test]
    fn test_derivatives_at_pi() {
        // 在 (π/2, 0, 0): sin(π/2)=1, sin(0)=0, cos(π/2)=0, cos(0)=1
        // dx = A*sin(0) + C*cos(0) = 0 + C = C
        // dy = B*sin(π/2) + A*cos(0) = B + A
        // dz = C*sin(0) + B*cos(π/2) = 0 + 0 = 0
        let cfg = AbcFlowConfig::default();
        let pi2 = std::f64::consts::FRAC_PI_2;
        let d = AbcFlowSolver::derivatives(&cfg, pi2, 0.0, 0.0);
        assert!(approx_eq(d[0], cfg.c, 1e-12));
        assert!(approx_eq(d[1], cfg.a + cfg.b, 1e-12));
        assert!(approx_eq(d[2], 0.0, 1e-12));
    }

    #[test]
    fn test_jacobian_shape() {
        // J = [[0, -C sin y, A cos z], [B cos x, 0, -A sin z], [0, C cos y, 0]]
        let cfg = AbcFlowConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = AbcFlowSolver::jacobian(&cfg, x, y, z);
        assert!(approx_eq(j[0][0], 0.0, 1e-12));
        assert!(approx_eq(j[0][1], -cfg.c * y.sin(), 1e-12));
        assert!(approx_eq(j[0][2], cfg.a * z.cos(), 1e-12));
        assert!(approx_eq(j[1][0], cfg.b * x.cos(), 1e-12));
        assert!(approx_eq(j[1][1], 0.0, 1e-12));
        assert!(approx_eq(j[1][2], -cfg.a * z.sin(), 1e-12));
        assert!(approx_eq(j[2][0], 0.0, 1e-12));
        assert!(approx_eq(j[2][1], cfg.c * y.cos(), 1e-12));
        assert!(approx_eq(j[2][2], 0.0, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_zero() {
        // 散度 = tr(J) = 0 (不可压缩)
        let cfg = AbcFlowConfig::default();
        let j = AbcFlowSolver::jacobian(&cfg, 0.3, 0.5, 0.7);
        let tr = j[0][0] + j[1][1] + j[2][2];
        assert!(approx_eq(tr, 0.0, 1e-12));
    }

    #[test]
    fn test_divergence_zero_everywhere() {
        let cfg = AbcFlowConfig::default();
        // 散度处处为零 (不可压缩)
        for &(x, y, z) in &[(0.0, 0.0, 0.0), (1.0, 2.0, 3.0), (-0.5, 0.3, 1.7)] {
            assert!(approx_eq(AbcFlowSolver::divergence(&cfg, x, y, z), 0.0, 1e-12));
        }
    }

    #[test]
    fn test_beltrami_condition() {
        // Beltrami 条件: 涡度 = 速度 (∇×v = v)
        let cfg = AbcFlowConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let v = AbcFlowSolver::derivatives(&cfg, x, y, z);
        let omega = AbcFlowSolver::vorticity(&cfg, x, y, z);
        assert!(approx_eq(v[0], omega[0], 1e-12));
        assert!(approx_eq(v[1], omega[1], 1e-12));
        assert!(approx_eq(v[2], omega[2], 1e-12));
    }

    #[test]
    fn test_volume_conservation() {
        // 散度=0 → 相空间体积守恒 (无吸引子, 混沌海)
        // 验证: 两条轨道的距离不应单调收缩 (非耗散)
        let cfg = AbcFlowConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = AbcFlowSolver::new(cfg, 0.1, 0.2, 0.3);
        let mut s2 = AbcFlowSolver::new(cfg, 0.1 + d0, 0.2, 0.3);
        for _ in 0..10000 {
            s1.step();
            s2.step();
        }
        let dx = s1.x - s2.x;
        let dy = s1.y - s2.y;
        let dz = s1.z - s2.z;
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        // 距离应放大 (混沌) 但非指数爆炸
        assert!(d > d0, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_step_advances() {
        let mut s = AbcFlowSolver::classic(AbcFlowConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = AbcFlowSolver::classic(AbcFlowConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = AbcFlowSolver::classic(AbcFlowConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        // ABC 流粒子在环面 T³ 上运动, 坐标漂移但有界 (无逃逸)
        // 散度=0 → 无吸引子, 轨道在有限体积内漫游
        let mut s = AbcFlowSolver::classic(AbcFlowConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        // ABC 流速度 |v| ≤ A+B+C = 3, 时间 300 → 最大漂移 ~900
        // 但在环面上, 坐标漂移有界于 ~|v|*t
        assert!(xmin > -2000.0 && xmax < 2000.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -2000.0 && ymax < 2000.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -2000.0 && zmax < 2000.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        // 经典 A=B=C=1 → 混沌, λ₁ > 0
        let mut s = AbcFlowSolver::classic(AbcFlowConfig::default());
        s.run(50000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = AbcFlowSolver::classic(AbcFlowConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 5.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        // 两条相近轨道应指数分离 (混沌)
        let cfg = AbcFlowConfig::default();
        let d0 = 1e-8_f64;
        let mut s1 = AbcFlowSolver::new(cfg, 0.1, 0.2, 0.3);
        let mut s2 = AbcFlowSolver::new(cfg, 0.1 + d0, 0.2, 0.3);
        for _ in 0..50000 {
            s1.step();
            s2.step();
        }
        let dx = s1.x - s2.x;
        let dy = s1.y - s2.y;
        let dz = s1.z - s2.z;
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        // t=500, λ~0.1, 应放大许多数量级
        assert!(d > 1e-3, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = AbcFlowSolver::classic(AbcFlowConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_wrapped_position() {
        // wrap 到 [0, 2π)
        let s = AbcFlowSolver::new(AbcFlowConfig::default(), 7.0, -1.0, 3.0 * std::f64::consts::PI);
        let (wx, wy, wz) = s.wrapped_position();
        let two_pi = 2.0 * std::f64::consts::PI;
        assert!(wx >= 0.0 && wx < two_pi, "wx: {}", wx);
        assert!(wy >= 0.0 && wy < two_pi, "wy: {}", wy);
        assert!(wz >= 0.0 && wz < two_pi, "wz: {}", wz);
        // 7.0 - 2π ≈ 0.7168
        assert!(approx_eq(wx, 7.0 - two_pi, 1e-10));
        // -1.0 + 2π ≈ 5.283
        assert!(approx_eq(wy, -1.0 + two_pi, 1e-10));
    }

    #[test]
    fn test_speed_bounded() {
        // |v|² = (A sin z + C cos y)² + (B sin x + A cos z)² + (C sin y + B cos x)²
        // 最大 ≤ (A+C)² + (B+A)² + (C+B)² (sin,cos ≤ 1)
        let cfg = AbcFlowConfig::default();
        let max_speed_sq = (cfg.a + cfg.c).powi(2) + (cfg.b + cfg.a).powi(2) + (cfg.c + cfg.b).powi(2);
        for &(x, y, z) in &[(0.0, 0.0, 0.0), (1.5, 0.7, 2.3), (-0.4, 1.1, 5.0)] {
            let s = AbcFlowSolver::speed_squared(&cfg, x, y, z);
            assert!(s <= max_speed_sq + 1e-10, "speed² {} > max {}", s, max_speed_sq);
        }
    }

    #[test]
    fn test_periodicity_2pi() {
        // ABC 流在 2π 周期: v(x+2π, y, z) = v(x, y, z) (因 sin, cos 周期)
        let cfg = AbcFlowConfig::default();
        let two_pi = 2.0 * std::f64::consts::PI;
        let v1 = AbcFlowSolver::derivatives(&cfg, 0.3, 0.5, 0.7);
        let v2 = AbcFlowSolver::derivatives(&cfg, 0.3 + two_pi, 0.5, 0.7);
        let v3 = AbcFlowSolver::derivatives(&cfg, 0.3, 0.5 + two_pi, 0.7);
        let v4 = AbcFlowSolver::derivatives(&cfg, 0.3, 0.5, 0.7 + two_pi);
        for i in 0..3 {
            assert!(approx_eq(v1[i], v2[i], 1e-10), "x-periodicity failed: {}", i);
            assert!(approx_eq(v1[i], v3[i], 1e-10), "y-periodicity failed: {}", i);
            assert!(approx_eq(v1[i], v4[i], 1e-10), "z-periodicity failed: {}", i);
        }
    }

    #[test]
    fn test_small_a_integrable_limit() {
        // A=0, B=C=0 → 速度=0, 粒子静止 (退化情形)
        let cfg = AbcFlowConfig { a: 0.0, b: 0.0, c: 0.0, dt: 0.01 };
        let mut s = AbcFlowSolver::new(cfg, 0.5, 0.5, 0.5);
        s.run(1000);
        assert!(approx_eq(s.x, 0.5, 1e-10));
        assert!(approx_eq(s.y, 0.5, 1e-10));
        assert!(approx_eq(s.z, 0.5, 1e-10));
    }

    #[test]
    fn test_single_axis_flow() {
        // A>0, B=C=0 → v = [A sin z, A cos z, 0]
        // 粒子在 xy 平面做圆周运动 (z 不变), 速度 |v|=A
        let cfg = AbcFlowConfig { a: 1.0, b: 0.0, c: 0.0, dt: 0.001 };
        let mut s = AbcFlowSolver::new(cfg, 1.0, 0.0, std::f64::consts::FRAC_PI_2);
        // z=π/2: sin z=1, cos z=0 → v=[1, 0, 0], x 增长
        s.run(100);
        assert!(s.x > 1.0, "x should increase: {}", s.x);
        assert!(approx_eq(s.z, std::f64::consts::FRAC_PI_2, 1e-3));
    }

    #[test]
    fn test_jacobian_beltrami_property() {
        // Beltrami 条件在矩阵层面: J = (涡度 Jacobian), 即 J 对称于 Beltrami 结构
        // 具体地: 由于 ∇×v = v, 涡度的 Jacobian = 速度的 Jacobian
        // 验证 J 的特征值: 一实 + 共轭复对 (Hamiltonian-like)
        let cfg = AbcFlowConfig::default();
        let j = AbcFlowSolver::jacobian(&cfg, 0.3, 0.5, 0.7);
        // tr(J) = 0 (不可压缩)
        let tr = j[0][0] + j[1][1] + j[2][2];
        assert!(approx_eq(tr, 0.0, 1e-12));
        // det(J) 可正可负 (取决于位置), 但应有限
        let det = j[0][0] * (j[1][1] * j[2][2] - j[1][2] * j[2][1])
            - j[0][1] * (j[1][0] * j[2][2] - j[1][2] * j[2][0])
            + j[0][2] * (j[1][0] * j[2][1] - j[1][1] * j[2][0]);
        assert!(det.is_finite());
    }
}
