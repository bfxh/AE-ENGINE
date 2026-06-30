//! Sprott E Attractor — Sprott E 简单混沌系统 (3D)
//!
//! Julien C. Sprott 1994 年在其论文 "Some simple chaotic flows" 中
//! 系统搜索发现的最简单混沌系统之一 (Sprott A-S, 共 19 个). Sprott E
//! 是其中第五个, 特点是唯一平衡点和线性 z 方程.
//!
//! 状态方程 (Sprott E 1994):
//!   dx/dt = y·z
//!   dy/dt = x² - y
//!   dz/dt = 1 - 4·x
//!
//! 经典参数 (Sprott 1994): 无参数 (系数固定为 1, 1, 1, 1, 4)
//! 经典初值: (x₀, y₀, z₀) = (0.05, 0.05, 0.05)
//!
//! 性质:
//!   - 散度 ∇·F = tr(J) = -1 (常数负, 耗散)
//!   - 平衡点: E0 = (1/4, 1/16, 0) (唯一)
//!   - Lyapunov 谱: λ₁ ≈ +0.20, λ₂ = 0, λ₃ ≈ -1.20
//!   - Kaplan-Yorke 维数 D_KY ≈ 2.17
//!
//! Sprott 系列对比:
//!   - Sprott B/C/E 都是常数散度 -1, 但非线性项组合不同
//!   - Sprott C: yz 在 dx/dt, x² 在 dz/dt
//!   - Sprott E: yz 在 dx/dt, x² 在 dy/dt
//!
//! 历史:
//!   Sprott, J. C. 1994. "Some simple chaotic flows." Phys. Rev. E 50, R647-R650.

/// Sprott E 求解器 (3D, 跟踪最大 Lyapunov 指数, 无参数)
pub struct SprottESolver {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub time: f64,
    pub step_count: u64,
    pub trajectory: Vec<(f64, f64, f64)>,
    pub lyap_sum: f64,
    pub v: [f64; 3],
    pub dt: f64,
}

impl SprottESolver {
    pub fn new(dt: f64, x0: f64, y0: f64, z0: f64) -> Self {
        Self {
            x: x0,
            y: y0,
            z: z0,
            time: 0.0,
            step_count: 0,
            trajectory: vec![(x0, y0, z0)],
            lyap_sum: 0.0,
            v: [1.0, 0.0, 0.0],
            dt,
        }
    }

    pub fn classic(dt: f64) -> Self {
        Self::new(dt, 0.05, 0.05, 0.05)
    }

    /// 右端导数 F = [y·z, x² - y, 1 - 4·x]
    pub fn derivatives(x: f64, y: f64, z: f64) -> [f64; 3] {
        [y * z, x * x - y, 1.0 - 4.0 * x]
    }

    /// Jacobian:
    /// J = [[0,   z,  y],
    ///      [2x, -1,  0],
    ///      [-4,  0,  0]]
    pub fn jacobian(x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
        [
            [0.0, z, y],
            [2.0 * x, -1.0, 0.0],
            [-4.0, 0.0, 0.0],
        ]
    }

    /// 散度 ∇·F = tr(J) = -1 (常数负)
    pub fn divergence(_x: f64, _y: f64, _z: f64) -> f64 {
        -1.0
    }

    /// 唯一平衡点 E0 = (1/4, 1/16, 0)
    pub fn equilibria() -> [f64; 3] {
        [0.25, 0.0625, 0.0]
    }

    /// 单步 RK4 推进 + 变分方程 Lyapunov
    pub fn step(&mut self) {
        let dt = self.dt;
        let (x, y, z) = (self.x, self.y, self.z);

        let k1 = Self::derivatives(x, y, z);
        let k2 = Self::derivatives(x + 0.5 * dt * k1[0], y + 0.5 * dt * k1[1], z + 0.5 * dt * k1[2]);
        let k3 = Self::derivatives(x + 0.5 * dt * k2[0], y + 0.5 * dt * k2[1], z + 0.5 * dt * k2[2]);
        let k4 = Self::derivatives(x + dt * k3[0], y + dt * k3[1], z + dt * k3[2]);

        self.x = x + dt / 6.0 * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]);
        self.y = y + dt / 6.0 * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]);
        self.z = z + dt / 6.0 * (k1[2] + 2.0 * k2[2] + 2.0 * k3[2] + k4[2]);

        self.step_count += 1;
        self.time += dt;
        self.trajectory.push((self.x, self.y, self.z));

        let j = Self::jacobian(self.x, self.y, self.z);
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

    /// 最大 Lyapunov 指数 (文献值 ~0.20)
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
    fn test_solver_creation() {
        let s = SprottESolver::classic(0.01);
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
        assert!(approx_eq(s.dt, 0.01, 1e-12));
    }

    #[test]
    fn test_derivatives_analytic() {
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = SprottESolver::derivatives(x, y, z);
        assert!(approx_eq(d[0], y * z, 1e-12));
        assert!(approx_eq(d[1], x * x - y, 1e-12));
        assert!(approx_eq(d[2], 1.0 - 4.0 * x, 1e-12));
    }

    #[test]
    fn test_derivatives_at_equilibrium() {
        let eq = SprottESolver::equilibria();
        let d = SprottESolver::derivatives(eq[0], eq[1], eq[2]);
        for v in d.iter() {
            assert!(v.abs() < 1e-12, "equilibrium derivative = {}", v);
        }
    }

    #[test]
    fn test_jacobian_shape() {
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = SprottESolver::jacobian(x, y, z);
        assert!(approx_eq(j[0][0], 0.0, 1e-12));
        assert!(approx_eq(j[0][1], z, 1e-12));
        assert!(approx_eq(j[0][2], y, 1e-12));
        assert!(approx_eq(j[1][0], 2.0 * x, 1e-12));
        assert!(approx_eq(j[1][1], -1.0, 1e-12));
        assert!(approx_eq(j[1][2], 0.0, 1e-12));
        assert!(approx_eq(j[2][0], -4.0, 1e-12));
        assert!(approx_eq(j[2][1], 0.0, 1e-12));
        assert!(approx_eq(j[2][2], 0.0, 1e-12));
    }

    #[test]
    fn test_divergence_constant() {
        assert!(approx_eq(SprottESolver::divergence(0.0, 0.0, 0.0), -1.0, 1e-12));
        assert!(approx_eq(SprottESolver::divergence(1.0, 2.0, 3.0), -1.0, 1e-12));
        assert!(approx_eq(SprottESolver::divergence(-5.0, 7.0, -3.0), -1.0, 1e-12));
    }

    #[test]
    fn test_divergence_negative_dissipative() {
        let div = SprottESolver::divergence(0.0, 0.0, 0.0);
        assert!(div < 0.0);
        assert!(approx_eq(div, -1.0, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5), (2.0, -1.0, 0.3)] {
            let j = SprottESolver::jacobian(x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = SprottESolver::divergence(x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_equilibrium_values() {
        let e0 = SprottESolver::equilibria();
        assert!(approx_eq(e0[0], 0.25, 1e-12));
        assert!(approx_eq(e0[1], 0.0625, 1e-12));
        assert!(approx_eq(e0[2], 0.0, 1e-12));
    }

    #[test]
    fn test_equilibrium_satisfies_equations() {
        let eq = SprottESolver::equilibria();
        let d = SprottESolver::derivatives(eq[0], eq[1], eq[2]);
        for v in d.iter() {
            assert!(v.abs() < 1e-12, "equilibrium derivative = {}", v);
        }
    }

    #[test]
    fn test_z_linear_in_x() {
        // dz/dt = 1 - 4x 不含 z, z 无耗散
        let d1 = SprottESolver::derivatives(0.5, 0.5, 0.1);
        let d2 = SprottESolver::derivatives(0.5, 0.5, 5.0);
        assert!(approx_eq(d1[2], d2[2], 1e-12));
    }

    #[test]
    fn test_step_advances() {
        let mut s = SprottESolver::classic(0.01);
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = SprottESolver::classic(0.01);
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = SprottESolver::classic(0.01);
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = SprottESolver::classic(0.01);
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = SprottESolver::classic(0.01);
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        let mut s = SprottESolver::classic(0.01);
        s.run(100000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = SprottESolver::classic(0.01);
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        let d0 = 1e-6_f64;
        let mut s1 = SprottESolver::classic(0.01);
        let mut s2 = SprottESolver::new(0.01, 0.05 + d0, 0.05, 0.05);
        s1.run(50000);
        s2.run(50000);
        let d = ((s1.x - s2.x).powi(2) + (s1.y - s2.y).powi(2) + (s1.z - s2.z).powi(2)).sqrt();
        assert!(d > 1e-3, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_escape_for_large_initial() {
        let mut s = SprottESolver::new(0.01, 1000.0, 1000.0, 1000.0);
        s.run(1000);
        assert!(s.has_escaped(), "should escape: x={} y={} z={}", s.x, s.y, s.z);
    }
}
