//! Sprott D Attractor — Sprott D 简单混沌系统 (3D)
//!
//! Julien C. Sprott 1994 年在其论文 "Some simple chaotic flows" 中
//! 系统搜索发现的最简单混沌系统之一 (Sprott A-S, 共 19 个). Sprott D
//! 是其中第四个, 特点是散度非常数 (位置依赖耗散), 且唯一平衡点是原点.
//!
//! 状态方程 (Sprott D 1994):
//!   dx/dt = -y
//!   dy/dt = x + z
//!   dz/dt = x·z + 3·y²
//!
//! 各项物理意义:
//!   - -y: 线性恢复力 (类似谐振子, x 受 y 驱动)
//!   + x: 线性耦合 (y 受 x 驱动)
//!   + z: 线性耦合 (y 受 z 驱动)
//!   + x·z: 非线性反馈 (z 受 xz 乘积调制)
//!   + 3·y²: 非线性反馈 (z 受 y² 调制, 剪切产热)
//!
//! 经典参数 (Sprott 1994): 无参数 (系数固定为 1, 1, 1, 1, 3)
//! 经典初值: (x₀, y₀, z₀) = (0.05, 0.05, 0.05) 或 (1, 0, 0)
//!
//! 性质:
//!   - 散度 ∇·F = tr(J) = x (非常数, 位置依赖)
//!     · x > 0: 散度 > 0 (局部体积膨胀)
//!     · x < 0: 散度 < 0 (局部体积收缩)
//!     · 平均散度 < 0 (整体耗散, 吸引子存在)
//!   - 平衡点 (利用 y=0, z=-x, xz+3y²=0):
//!     y = 0
//!     z = -x
//!     x·(-x) + 0 = 0 → -x² = 0 → x = 0
//!     E0 = (0, 0, 0) (唯一平衡点)
//!   - Lyapunov 谱 (文献值):
//!     λ₁ ≈ +0.19  (正, 主混沌方向)
//!     λ₂ = 0      (沿轨道切向)
//!     λ₃ ≈ -1.19  (负, 收缩)
//!     和 ≈ -1 (与平均散度一致)
//!   - Kaplan-Yorke 维数 D_KY ≈ 2 + λ₁/|λ₃| ≈ 2.16
//!   - 吸引子形态: 双叶结构
//!
//! Sprott 系列对比 (已实现):
//!   - Sprott A (sprott_attractor): 1 个非线性项 (yz)
//!   - Sprott B (sprott_b_attractor): 2 个非线性项 (yz, xy), 散度=-1 常数
//!   - Sprott C (sprott_c_attractor): 2 个非线性项 (yz, x²), 散度=-1 常数
//!   - Sprott D (本模块): 2 个非线性项 (xz, y²), 散度=x 非常数
//!
//! 简约性意义:
//!   Sprott D 展示了非常数散度 (位置依赖耗散) 也能产生混沌吸引子.
//!   与 Sprott B/C (常数散度) 对比, 说明耗散机制的多样性.
//!
//! 数值方法:
//!   RK4 (4 阶 Runge-Kutta); 变分方程前向欧拉 (I + dt J) v
//!
//! 历史:
//!   Sprott, J. C. 1994. "Some simple chaotic flows." Phys. Rev. E 50,
//!   R647-R650. (原始论文, 19 个最简单混沌系统)

/// Sprott D 求解器 (3D, 跟踪最大 Lyapunov 指数, 无参数)
pub struct SprottDSolver {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub time: f64,
    pub step_count: u64,
    pub trajectory: Vec<(f64, f64, f64)>,
    pub lyap_sum: f64,
    pub v: [f64; 3],
    /// 时间步长 dt
    pub dt: f64,
}

impl SprottDSolver {
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

    /// 右端导数 F = [-y, x + z, x·z + 3·y²]
    pub fn derivatives(x: f64, y: f64, z: f64) -> [f64; 3] {
        [-y, x + z, x * z + 3.0 * y * y]
    }

    /// Jacobian:
    /// J = [[0,  -1,  0],
    ///      [1,   0,  1],
    ///      [z,  6y,  x]]
    pub fn jacobian(x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
        [
            [0.0, -1.0, 0.0],
            [1.0, 0.0, 1.0],
            [z, 6.0 * y, x],
        ]
    }

    /// 散度 ∇·F = tr(J) = x (非常数, 位置依赖)
    pub fn divergence(x: f64, _y: f64, _z: f64) -> f64 {
        x
    }

    /// 唯一平衡点 E0 = (0, 0, 0)
    pub fn equilibria() -> [f64; 3] {
        [0.0, 0.0, 0.0]
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

    /// 最大 Lyapunov 指数 (文献值 ~0.19)
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
        let s = SprottDSolver::classic(0.01);
        assert_eq!(s.step_count, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.trajectory.len(), 1);
        assert!(approx_eq(s.dt, 0.01, 1e-12));
    }

    #[test]
    fn test_derivatives_analytic() {
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = SprottDSolver::derivatives(x, y, z);
        assert!(approx_eq(d[0], -y, 1e-12));
        assert!(approx_eq(d[1], x + z, 1e-12));
        assert!(approx_eq(d[2], x * z + 3.0 * y * y, 1e-12));
    }

    #[test]
    fn test_derivatives_origin_zero() {
        // 原点是平衡点
        let d = SprottDSolver::derivatives(0.0, 0.0, 0.0);
        for v in d.iter() {
            assert!(v.abs() < 1e-12);
        }
    }

    #[test]
    fn test_jacobian_shape() {
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = SprottDSolver::jacobian(x, y, z);
        // Row 0: [0, -1, 0]
        assert!(approx_eq(j[0][0], 0.0, 1e-12));
        assert!(approx_eq(j[0][1], -1.0, 1e-12));
        assert!(approx_eq(j[0][2], 0.0, 1e-12));
        // Row 1: [1, 0, 1]
        assert!(approx_eq(j[1][0], 1.0, 1e-12));
        assert!(approx_eq(j[1][1], 0.0, 1e-12));
        assert!(approx_eq(j[1][2], 1.0, 1e-12));
        // Row 2: [z, 6y, x]
        assert!(approx_eq(j[2][0], z, 1e-12));
        assert!(approx_eq(j[2][1], 6.0 * y, 1e-12));
        assert!(approx_eq(j[2][2], x, 1e-12));
    }

    #[test]
    fn test_divergence_nonconstant() {
        // 散度 = x (非常数, 位置依赖)
        assert!(approx_eq(SprottDSolver::divergence(0.0, 0.0, 0.0), 0.0, 1e-12));
        assert!(approx_eq(SprottDSolver::divergence(1.0, 2.0, 3.0), 1.0, 1e-12));
        assert!(approx_eq(SprottDSolver::divergence(-5.0, 7.0, -3.0), -5.0, 1e-12));
    }

    #[test]
    fn test_divergence_depends_on_x() {
        // 散度仅依赖于 x (不依赖 y, z)
        let div1 = SprottDSolver::divergence(1.0, 2.0, 0.5);
        let div2 = SprottDSolver::divergence(1.0, -3.0, 7.0);
        assert!(approx_eq(div1, div2, 1e-12));
        // 不同 x 不同散度
        let div3 = SprottDSolver::divergence(2.0, 2.0, 0.5);
        assert!(!approx_eq(div1, div3, 1e-9));
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5), (2.0, -1.0, 0.3)] {
            let j = SprottDSolver::jacobian(x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = SprottDSolver::divergence(x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_equilibrium_values() {
        let e0 = SprottDSolver::equilibria();
        assert!(approx_eq(e0[0], 0.0, 1e-12));
        assert!(approx_eq(e0[1], 0.0, 1e-12));
        assert!(approx_eq(e0[2], 0.0, 1e-12));
    }

    #[test]
    fn test_equilibrium_satisfies_equations() {
        let eq = SprottDSolver::equilibria();
        let d = SprottDSolver::derivatives(eq[0], eq[1], eq[2]);
        for v in d.iter() {
            assert!(v.abs() < 1e-12, "equilibrium derivative = {}", v);
        }
    }

    #[test]
    fn test_step_advances() {
        let mut s = SprottDSolver::classic(0.01);
        s.step();
        assert_eq!(s.step_count, 1);
        assert!(s.time > 0.0);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = SprottDSolver::classic(0.01);
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
        assert_eq!(s.step_count, 1000);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = SprottDSolver::classic(0.01);
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = SprottDSolver::classic(0.01);
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = SprottDSolver::classic(0.01);
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        // Sprott D 是混沌的, λ > 0 (文献值 ~0.19)
        let mut s = SprottDSolver::classic(0.01);
        s.run(100000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = SprottDSolver::classic(0.01);
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0, "lambda too large: {}", lambda);
    }

    #[test]
    fn test_chaos_sensitivity() {
        // 混沌系统: 微小扰动指数放大
        let d0 = 1e-6_f64;
        let mut s1 = SprottDSolver::classic(0.01);
        let mut s2 = SprottDSolver::new(0.01, 0.05 + d0, 0.05, 0.05);
        s1.run(50000);
        s2.run(50000);
        let d = ((s1.x - s2.x).powi(2) + (s1.y - s2.y).powi(2) + (s1.z - s2.z).powi(2)).sqrt();
        assert!(d > 1e-3, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_escape_for_large_initial() {
        let mut s = SprottDSolver::new(0.01, 1000.0, 1000.0, 1000.0);
        s.run(1000);
        assert!(s.has_escaped(), "should escape: x={} y={} z={}", s.x, s.y, s.z);
    }
}
