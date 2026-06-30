//! Sprott F Attractor - Sprott F 简单混沌系统 (3D)
//! Sprott 1994 年发现的最简单混沌系统之一. 6 项 1 非线性项 (x^2).

#[derive(Clone, Copy, Debug)]
pub struct SprottFConfig {
    pub a: f64,
    pub dt: f64,
}

impl Default for SprottFConfig {
    fn default() -> Self {
        Self { a: 0.5, dt: 0.01 }
    }
}

pub struct SprottFSolver {
    pub config: SprottFConfig,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub time: f64,
    pub step_count: u64,
    pub trajectory: Vec<(f64, f64, f64)>,
    pub lyap_sum: f64,
    pub v: [f64; 3],
}

impl SprottFSolver {
    pub fn new(config: SprottFConfig, x0: f64, y0: f64, z0: f64) -> Self {
        Self {
            config, x: x0, y: y0, z: z0,
            time: 0.0, step_count: 0,
            trajectory: vec![(x0, y0, z0)],
            lyap_sum: 0.0, v: [1.0, 0.0, 0.0],
        }
    }

    pub fn classic(config: SprottFConfig) -> Self {
        Self::new(config, 0.05, 0.05, 0.05)
    }

    pub fn derivatives(cfg: &SprottFConfig, x: f64, y: f64, z: f64) -> [f64; 3] {
        [y + z, -x + cfg.a * y, x * x - z]
    }

    pub fn jacobian(cfg: &SprottFConfig, x: f64, _y: f64, _z: f64) -> [[f64; 3]; 3] {
        [
            [0.0, 1.0, 1.0],
            [-1.0, cfg.a, 0.0],
            [2.0 * x, 0.0, -1.0],
        ]
    }

    pub fn divergence(cfg: &SprottFConfig, _x: f64, _y: f64, _z: f64) -> f64 {
        cfg.a - 1.0
    }

    pub fn equilibria(cfg: &SprottFConfig) -> ([f64; 3], [f64; 3]) {
        if cfg.a.abs() < 1e-12 {
            return ([0.0, 0.0, 0.0], [f64::NAN, f64::NAN, f64::NAN]);
        }
        let x = -1.0 / cfg.a;
        let z = 1.0 / (cfg.a * cfg.a);
        ([0.0, 0.0, 0.0], [x, -z, z])
    }

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
        for _ in 0..n_steps { self.step(); }
    }

    pub fn lyapunov_exponent(&self) -> f64 {
        if self.step_count == 0 { return 0.0; }
        self.lyap_sum / self.time.max(1e-12)
    }

    pub fn has_nan(&self) -> bool {
        !self.x.is_finite() || !self.y.is_finite() || !self.z.is_finite()
    }

    pub fn has_escaped(&self) -> bool {
        self.x.abs() > 100.0 || self.y.abs() > 100.0 || self.z.abs() > 100.0 || self.has_nan()
    }

    pub fn attractor_bounds(&self) -> (f64, f64, f64, f64, f64, f64) {
        let mut xmin = f64::INFINITY; let mut xmax = f64::NEG_INFINITY;
        let mut ymin = f64::INFINITY; let mut ymax = f64::NEG_INFINITY;
        let mut zmin = f64::INFINITY; let mut zmax = f64::NEG_INFINITY;
        for &(x, y, z) in &self.trajectory {
            if x < xmin { xmin = x; } if x > xmax { xmax = x; }
            if y < ymin { ymin = y; } if y > ymax { ymax = y; }
            if z < zmin { zmin = z; } if z > zmax { zmax = z; }
        }
        (xmin, xmax, ymin, ymax, zmin, zmax)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool { (a - b).abs() < tol }

    #[test]
    fn test_default_config() {
        let cfg = SprottFConfig::default();
        assert!(approx_eq(cfg.a, 0.5, 1e-12));
    }

    #[test]
    fn test_solver_creation() {
        let s = SprottFSolver::classic(SprottFConfig::default());
        assert_eq!(s.step_count, 0);
        assert_eq!(s.trajectory.len(), 1);
    }

    #[test]
    fn test_derivatives_analytic() {
        let cfg = SprottFConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let d = SprottFSolver::derivatives(&cfg, x, y, z);
        assert!(approx_eq(d[0], y + z, 1e-12));
        assert!(approx_eq(d[1], -x + cfg.a * y, 1e-12));
        assert!(approx_eq(d[2], x * x - z, 1e-12));
    }

    #[test]
    fn test_derivatives_origin_zero() {
        let cfg = SprottFConfig::default();
        let d = SprottFSolver::derivatives(&cfg, 0.0, 0.0, 0.0);
        for v in d.iter() { assert!(v.abs() < 1e-12); }
    }

    #[test]
    fn test_jacobian_shape() {
        let cfg = SprottFConfig::default();
        let (x, y, z) = (0.3_f64, 0.5_f64, 0.7_f64);
        let j = SprottFSolver::jacobian(&cfg, x, y, z);
        assert!(approx_eq(j[0][0], 0.0, 1e-12));
        assert!(approx_eq(j[0][1], 1.0, 1e-12));
        assert!(approx_eq(j[0][2], 1.0, 1e-12));
        assert!(approx_eq(j[1][0], -1.0, 1e-12));
        assert!(approx_eq(j[1][1], cfg.a, 1e-12));
        assert!(approx_eq(j[1][2], 0.0, 1e-12));
        assert!(approx_eq(j[2][0], 2.0 * x, 1e-12));
        assert!(approx_eq(j[2][1], 0.0, 1e-12));
        assert!(approx_eq(j[2][2], -1.0, 1e-12));
    }

    #[test]
    fn test_divergence_constant() {
        let cfg = SprottFConfig::default();
        let expected = cfg.a - 1.0;
        assert!(approx_eq(SprottFSolver::divergence(&cfg, 0.0, 0.0, 0.0), expected, 1e-12));
        assert!(approx_eq(SprottFSolver::divergence(&cfg, 1.0, 2.0, 3.0), expected, 1e-12));
    }

    #[test]
    fn test_divergence_negative_dissipative() {
        let cfg = SprottFConfig::default();
        let div = SprottFSolver::divergence(&cfg, 0.0, 0.0, 0.0);
        assert!(div < 0.0);
        assert!(approx_eq(div, -0.5, 1e-12));
    }

    #[test]
    fn test_jacobian_trace_is_divergence() {
        let cfg = SprottFConfig::default();
        for &(x, y, z) in &[(0.3_f64, 0.5, 0.7), (-1.0, 2.0, 0.5)] {
            let j = SprottFSolver::jacobian(&cfg, x, y, z);
            let tr = j[0][0] + j[1][1] + j[2][2];
            let div = SprottFSolver::divergence(&cfg, x, y, z);
            assert!(approx_eq(tr, div, 1e-12));
        }
    }

    #[test]
    fn test_equilibria_values() {
        let cfg = SprottFConfig::default();
        let (e0, e1) = SprottFSolver::equilibria(&cfg);
        assert!(approx_eq(e0[0], 0.0, 1e-12));
        assert!(approx_eq(e1[0], -2.0, 1e-12));
        assert!(approx_eq(e1[1], -4.0, 1e-12));
        assert!(approx_eq(e1[2], 4.0, 1e-12));
    }

    #[test]
    fn test_equilibria_satisfy_equations() {
        let cfg = SprottFConfig::default();
        let (e0, e1) = SprottFSolver::equilibria(&cfg);
        for eq in [e0, e1] {
            let d = SprottFSolver::derivatives(&cfg, eq[0], eq[1], eq[2]);
            for v in d.iter() { assert!(v.abs() < 1e-9); }
        }
    }

    #[test]
    fn test_step_advances() {
        let mut s = SprottFSolver::classic(SprottFConfig::default());
        s.step();
        assert_eq!(s.step_count, 1);
        assert_eq!(s.trajectory.len(), 2);
    }

    #[test]
    fn test_trajectory_grows() {
        let mut s = SprottFSolver::classic(SprottFConfig::default());
        s.run(1000);
        assert_eq!(s.trajectory.len(), 1001);
    }

    #[test]
    fn test_no_nan_short() {
        let mut s = SprottFSolver::classic(SprottFConfig::default());
        s.run(10000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_no_nan_long() {
        let mut s = SprottFSolver::classic(SprottFConfig::default());
        s.run(50000);
        assert!(!s.has_nan());
    }

    #[test]
    fn test_attractor_bounded() {
        let mut s = SprottFSolver::classic(SprottFConfig::default());
        s.run(30000);
        let (xmin, xmax, ymin, ymax, zmin, zmax) = s.attractor_bounds();
        assert!(xmin > -50.0 && xmax < 50.0, "x: [{}, {}]", xmin, xmax);
        assert!(ymin > -50.0 && ymax < 50.0, "y: [{}, {}]", ymin, ymax);
        assert!(zmin > -50.0 && zmax < 50.0, "z: [{}, {}]", zmin, zmax);
    }

    #[test]
    fn test_lyapunov_positive() {
        let mut s = SprottFSolver::classic(SprottFConfig::default());
        s.run(100000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda > 0.0, "lambda should be positive: {}", lambda);
    }

    #[test]
    fn test_lyapunov_finite_value() {
        let mut s = SprottFSolver::classic(SprottFConfig::default());
        s.run(20000);
        let lambda = s.lyapunov_exponent();
        assert!(lambda.is_finite());
        assert!(lambda < 10.0);
    }

    #[test]
    fn test_chaos_sensitivity() {
        let cfg = SprottFConfig::default();
        let d0 = 1e-6_f64;
        let mut s1 = SprottFSolver::classic(cfg);
        let mut s2 = SprottFSolver::new(cfg, 0.05 + d0, 0.05, 0.05);
        s1.run(50000);
        s2.run(50000);
        let d = ((s1.x - s2.x).powi(2) + (s1.y - s2.y).powi(2) + (s1.z - s2.z).powi(2)).sqrt();
        assert!(d > 1e-3, "should be amplified: d0={} d={}", d0, d);
    }

    #[test]
    fn test_escape_for_large_initial() {
        let mut s = SprottFSolver::new(SprottFConfig::default(), 1000.0, 1000.0, 1000.0);
        s.run(1000);
        assert!(s.has_escaped());
    }
}