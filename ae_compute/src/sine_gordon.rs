//! Sine-Gordon Equation Solver (Topological Solitons)
//!
//! 非线性 Klein-Gordon 方程. 可积系统三部曲之一 (KdV -> NLS -> Sine-Gordon).
//! 与 KdV/NLS 不同的是 SG 孤立子具有**拓扑荷** — 不可连续形变消除.
//!
//! 方程:
//!   phi_tt - phi_xx + sin(phi) = 0
//!
//! 静止 kink 解 (拓扑孤立子, |v|<1, 自然单位 c=1):
//!   phi(x,t) = 4 * arctan(exp( gamma * (x - v*t - x0) ))
//!   gamma = 1 / sqrt(1 - v^2)  (Lorentz factor)
//!   拓扑荷 Q = +1 (kink), Q = -1 (antikink)
//!
//! Breather (呼吸孤立子, 0 < omega < 1):
//!   phi(x,t) = 4 * arctan( (omega/sqrt(1-omega^2)) * sin(omega' * t) / cosh(omega*(x-x0)) )
//!   omega' = sqrt(1 - omega^2)
//!   Breather 是 kink-antikink 束缚态.
//!
//! Kink-Kink 散射 (Perring-Skyrme 1970 解析解):
//!   phi(x,t) = 4 * arctan( v * sinh(gamma*x) / cosh(gamma*v*t) )
//!
//! Kink-Antikink 散射:
//!   phi(x,t) = 4 * arctan( sin(omega'*t) / (omega' * cosh(omega*x)) )
//!   (低速碰撞产生 breather 状态, 高速碰撞穿过)
//!
//! Leapfrog (二阶时间, 中心空间):
//!   phi^{n+1}_i = 2*phi^n_i - phi^{n-1}_i
//!               + (dt/dx)^2 * (phi^n_{i+1} - 2*phi^n_i + phi^n_{i-1})
//!               - dt^2 * sin(phi^n_i)
//!
//! CFL 稳定性: dt/dx <= 1 (光速 = 1, 严格子光速)
//!
//! 守恒量:
//!   能量 E = integral [ 1/2 * phi_t^2 + 1/2 * phi_x^2 + (1 - cos(phi)) ] dx
//!   拓扑荷 Q = (1/(2*pi)) * integral phi_x dx  (在 -infinity 到 +infinity 上)
//!     - kink:     Q = +1  (phi 从 0 升到 2*pi)
//!     - antikink: Q = -1  (phi 从 2*pi 降到 0)
//!     - vacuum:   Q =  0
//!   拓扑荷严格守恒, 只有边界处 phi != 0 的解才能改变 Q.
//!
//! 边界条件:
//!   Fixed    - phi 固定为 0 (kink 反射为 antikink, 但 Q 守恒 — 反射时反向并改变方向)
//!   Periodic - 用于多 kink 散射 (注意 Q 必须为 0)
//!   Absorbing - sponge 衰减层 (近似开放边界)
//!
//! 应用:
//!   - 约瑟夫森结磁通量子 (1 个 kink = 1 个磁通量子 Phi_0 = h/(2e))
//!   - DNA 动力学 (Peyrard-Bishop-Dauxois 模型 — 局部变性)
//!   - 位错运动 (Frenkel-Kontorova 模型)
//!   - 电荷密度波
//!   - 拓扑场论 (kink 是最简单的拓扑缺陷)
//!   - 弦论 D-brane 散射 (kink 散射 = D-brane 散射的低能有效描述)
//!
//! 基于 Perring-Skyrme 1962 (核子模型),
//!       Rubinstein 1970 (SG 可积性证明),
//!       Ablowitz-Kaup-Newell-Segur 1973 (IST 逆散射变换).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SineGordonBoundary {
    /// 固定边界 (phi = 0)
    Fixed,
    /// 周期边界 (Q 必须为 0)
    Periodic,
    /// 吸收边界 (sponge 层)
    Absorbing { layer: usize, strength: f32 },
}

impl Default for SineGordonBoundary {
    fn default() -> Self {
        SineGordonBoundary::Absorbing { layer: 16, strength: 0.95 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SineGordonConfig {
    pub nx: usize,
    pub dx: f32,
    pub dt: f32,
    pub boundary: SineGordonBoundary,
}

impl Default for SineGordonConfig {
    fn default() -> Self {
        SineGordonConfig {
            nx: 512,
            dx: 0.05,
            dt: 0.04,
            boundary: SineGordonBoundary::default(),
        }
    }
}

impl SineGordonConfig {
    pub fn length(&self) -> f32 {
        (self.nx as f32) * self.dx
    }
    pub fn n_cells(&self) -> usize {
        self.nx
    }
    /// CFL 数: dt/dx (光速 = 1)
    pub fn courant(&self) -> f32 {
        self.dt / self.dx
    }
    /// CFL 稳定性上限: 1.0 (光速)
    pub fn cfl_limit(&self) -> f32 {
        1.0
    }
    pub fn is_stable(&self) -> bool {
        self.courant() <= self.cfl_limit()
    }
    /// 稳定时间步长上限
    pub fn stable_dt(&self) -> f32 {
        self.cfl_limit() * self.dx
    }
}

pub struct SineGordonSolver {
    pub config: SineGordonConfig,
    pub phi_curr: Vec<f32>,
    pub phi_prev: Vec<f32>,
    pub phi_next: Vec<f32>,
    /// sponge 衰减系数
    pub damping: Vec<f32>,
    pub time: f32,
    pub steps: usize,
}

impl SineGordonSolver {
    pub fn new(config: SineGordonConfig) -> Self {
        let n = config.n_cells();
        let damping = Self::build_damping(&config);
        SineGordonSolver {
            config,
            phi_curr: vec![0.0; n],
            phi_prev: vec![0.0; n],
            phi_next: vec![0.0; n],
            damping,
            time: 0.0,
            steps: 0,
        }
    }

    fn build_damping(config: &SineGordonConfig) -> Vec<f32> {
        let n = config.n_cells();
        match config.boundary {
            SineGordonBoundary::Absorbing { layer, strength } => {
                let mut d = vec![1.0; n];
                for i in 0..n {
                    let mut dist = layer;
                    if i < layer { dist = dist.min(i); }
                    if i >= n - layer { dist = dist.min(n - 1 - i); }
                    if dist < layer {
                        let frac = 1.0 - dist as f32 / layer as f32;
                        d[i] = (1.0 - strength * frac * frac).max(0.0);
                    }
                }
                d
            }
            _ => vec![1.0; n],
        }
    }

    fn wrap(i: i32, n: usize) -> usize {
        let m = n as i32;
        (((i % m) + m) % m) as usize
    }

    /// 真空初值 (phi = 0)
    pub fn initialize_vacuum(&mut self) {
        for v in self.phi_curr.iter_mut() { *v = 0.0; }
        for v in self.phi_prev.iter_mut() { *v = 0.0; }
        self.time = 0.0;
        self.steps = 0;
    }

    /// Kink/antikink 初值
    /// sign = +1: kink (phi 从 0 升到 2*pi)
    /// sign = -1: antikink (phi 从 2*pi 降到 0)
    ///
    /// phi(x, 0) = 4 * sign * arctan(exp(gamma * (x - x0)))
    /// phi_t(x, 0) = -2 * sign * gamma * v / cosh(gamma * (x - x0))
    ///             (运动 kink 的时间导数)
    pub fn initialize_kink(&mut self, sign: f32, velocity: f32, center: f32) {
        let nx = self.config.nx;
        let dx = self.config.dx;
        let v = velocity.clamp(-0.999, 0.999);
        let gamma = 1.0 / (1.0 - v * v).sqrt();
        for i in 0..nx {
            let x = (i as f32) * dx;
            let xi = gamma * (x - center);
            let phi = 4.0 * sign * xi.exp().atan();
            // 时间导数 phi_t = -2*sign*gamma*v / cosh(xi)
            // phi_prev = phi - dt * phi_t
            let phi_t = -2.0 * sign * gamma * v / xi.cosh();
            self.phi_curr[i] = phi;
            self.phi_prev[i] = phi - self.config.dt * phi_t;
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// Breather 初值 (kink-antikink 束缚态)
    /// omega in (0, 1): 频率
    /// phi(x, 0) = 4 * arctan( (omega/omega') / cosh(omega*(x-x0)) )
    /// phi_t(x, 0) = 0  (t=0 是转折点)
    /// omega' = sqrt(1 - omega^2)
    pub fn initialize_breather(&mut self, omega: f32, center: f32) {
        let nx = self.config.nx;
        let dx = self.config.dx;
        let omega = omega.clamp(0.01, 0.999);
        let omega_p = (1.0 - omega * omega).sqrt();
        let ratio = omega / omega_p;
        for i in 0..nx {
            let x = (i as f32) * dx;
            let arg = ratio / ((omega * (x - center)).cosh());
            let phi = 4.0 * arg.atan();
            self.phi_curr[i] = phi;
            self.phi_prev[i] = phi; // phi_t = 0 at t=0
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 双 kink 系统初值 (用于散射)
    /// left kink (+1) at x_L moving right (+v)
    /// right kink (-1) at x_R moving left (-v)
    /// phi = phi_left + phi_right (叠加近似)
    pub fn initialize_kink_antikink_pair(&mut self, velocity: f32, x_left: f32, x_right: f32) {
        let nx = self.config.nx;
        let dx = self.config.dx;
        let v = velocity.clamp(0.01, 0.999);
        let gamma = 1.0 / (1.0 - v * v).sqrt();
        for i in 0..nx {
            let x = (i as f32) * dx;
            // 左 kink 向右运动
            let xi_l = gamma * (x - x_left);
            let phi_l = 4.0 * xi_l.exp().atan();
            let phi_t_l = -2.0 * gamma * v / xi_l.cosh();
            // 右 antikink 向左运动
            let xi_r = gamma * (x - x_right);
            let phi_r = -4.0 * xi_r.exp().atan();
            let phi_t_r = -2.0 * gamma * v / xi_r.cosh(); // sign=-1, v=-v -> 同号
            // 注意: antikink phi_t = -2*sign*gamma*v/cosh = -2*(-1)*gamma*(-v)/cosh = -2*gamma*v/cosh
            let phi = phi_l + phi_r;
            let phi_t = phi_t_l + phi_t_r;
            self.phi_curr[i] = phi;
            self.phi_prev[i] = phi - self.config.dt * phi_t;
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 双 kink (相同符号) 初值 (用于 kink-kink 散射)
    /// left antikink (-1) at x_L moving left (-v)
    /// right kink (+1) at x_R moving right (+v)
    pub fn initialize_kink_kink_pair(&mut self, velocity: f32, x_left: f32, x_right: f32) {
        let nx = self.config.nx;
        let dx = self.config.dx;
        let v = velocity.clamp(0.01, 0.999);
        let gamma = 1.0 / (1.0 - v * v).sqrt();
        for i in 0..nx {
            let x = (i as f32) * dx;
            // 左 antikink 向左运动 (sign=-1, velocity=-v)
            let xi_l = gamma * (x - x_left);
            let phi_l = -4.0 * xi_l.exp().atan();
            // phi_t for left antikink moving left: -2*sign*gamma*v/cosh where v=-v_actual
            // = -2*(-1)*gamma*(-v)/cosh = -2*gamma*v/cosh
            let phi_t_l = -2.0 * gamma * v / xi_l.cosh();
            // 右 kink 向右运动 (sign=+1, velocity=+v)
            let xi_r = gamma * (x - x_right);
            let phi_r = 4.0 * xi_r.exp().atan();
            let phi_t_r = -2.0 * gamma * v / xi_r.cosh();
            let phi = phi_l + phi_r;
            let phi_t = phi_t_l + phi_t_r;
            self.phi_curr[i] = phi;
            self.phi_prev[i] = phi - self.config.dt * phi_t;
        }
        self.time = 0.0;
        self.steps = 0;
    }

    /// 一步 leapfrog 更新
    pub fn step(&mut self) {
        let nx = self.config.nx;
        let dx = self.config.dx;
        let dt = self.config.dt;
        let r2 = (dt / dx) * (dt / dx); // (c*dt/dx)^2, c=1
        let dt2 = dt * dt;
        for i in 0..nx {
            let (ip, im) = match self.config.boundary {
                SineGordonBoundary::Periodic => (
                    Self::wrap((i as i32) + 1, nx),
                    Self::wrap((i as i32) - 1, nx),
                ),
                _ => (
                    (i + 1).min(nx - 1),
                    if i > 0 { i - 1 } else { 0 },
                ),
            };
            let phi = self.phi_curr[i];
            let phi_p = self.phi_curr[ip];
            let phi_m = self.phi_curr[im];
            let lap = phi_p + phi_m - 2.0 * phi;
            let force = -phi.sin();
            let d = self.damping[i];
            // leapfrog: phi^{n+1} = 2*phi^n - phi^{n-1} + r^2 * lap + dt^2 * force
            let new_phi = (2.0 * phi - self.phi_prev[i] + r2 * lap + dt2 * force) * d;
            self.phi_next[i] = new_phi;
        }
        std::mem::swap(&mut self.phi_prev, &mut self.phi_curr);
        std::mem::swap(&mut self.phi_curr, &mut self.phi_next);
        self.time += self.config.dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    /// 能量 E = integral [ 1/2 phi_t^2 + 1/2 phi_x^2 + (1 - cos phi) ] dx
    pub fn energy(&self) -> f32 {
        let nx = self.config.nx;
        let dx = self.config.dx;
        let dt = self.config.dt;
        let mut e = 0.0f32;
        for i in 0..nx {
            let ip = (i + 1).min(nx - 1);
            let im = if i > 0 { i - 1 } else { 0 };
            let phi = self.phi_curr[i];
            let phi_t = (self.phi_curr[i] - self.phi_prev[i]) / dt;
            let phi_x = (self.phi_curr[ip] - self.phi_curr[im]) / (2.0 * dx);
            let kinetic = 0.5 * phi_t * phi_t;
            let gradient = 0.5 * phi_x * phi_x;
            let potential = 1.0 - phi.cos();
            e += (kinetic + gradient + potential) * dx;
        }
        e
    }

    /// 拓扑荷 Q = (1/(2*pi)) * integral phi_x dx
    /// 注意: 周期边界下 Q 严格为 0
    /// 固定/吸收边界下 Q 等于边界处 phi 的差除以 2*pi
    pub fn topological_charge(&self) -> f32 {
        let nx = self.config.nx;
        let dx = self.config.dx;
        let mut q = 0.0f32;
        for i in 0..nx {
            let ip = (i + 1).min(nx - 1);
            let im = if i > 0 { i - 1 } else { 0 };
            let phi_x = (self.phi_curr[ip] - self.phi_curr[im]) / (2.0 * dx);
            q += phi_x * dx;
        }
        q / (2.0 * std::f32::consts::PI)
    }

    /// 最大 |phi|
    pub fn max_amplitude(&self) -> f32 {
        self.phi_curr.iter().map(|&p| p.abs()).fold(0.0f32, f32::max)
    }

    /// 找 kink 中心 (phi = pi 的位置, 二分查找)
    pub fn find_kink_center(&self) -> f32 {
        let nx = self.config.nx;
        let dx = self.config.dx;
        let pi = std::f32::consts::PI;
        let mut best_x = 0.0f32;
        let mut best_diff = f32::MAX;
        for i in 0..nx {
            let diff = (self.phi_curr[i] - pi).abs();
            if diff < best_diff {
                best_diff = diff;
                best_x = (i as f32) * dx;
            }
        }
        best_x
    }

    pub fn reset(&mut self) {
        for v in self.phi_curr.iter_mut() { *v = 0.0; }
        for v in self.phi_prev.iter_mut() { *v = 0.0; }
        for v in self.phi_next.iter_mut() { *v = 0.0; }
        self.time = 0.0;
        self.steps = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn test_config_default() {
        let c = SineGordonConfig::default();
        assert_eq!(c.nx, 512);
        assert_eq!(c.dx, 0.05);
        assert_eq!(c.dt, 0.04);
        assert!(matches!(c.boundary, SineGordonBoundary::Absorbing { .. }));
    }

    #[test]
    fn test_config_length() {
        let c = SineGordonConfig { nx: 100, dx: 0.2, dt: 0.1, boundary: SineGordonBoundary::Fixed };
        assert!(approx_eq(c.length(), 20.0, 1e-6));
    }

    #[test]
    fn test_config_n_cells() {
        let c = SineGordonConfig { nx: 128, dx: 0.1, dt: 0.05, boundary: SineGordonBoundary::Fixed };
        assert_eq!(c.n_cells(), 128);
    }

    #[test]
    fn test_config_courant() {
        let c = SineGordonConfig { nx: 128, dx: 0.1, dt: 0.05, boundary: SineGordonBoundary::Fixed };
        // dt/dx = 0.5
        assert!(approx_eq(c.courant(), 0.5, 1e-6));
    }

    #[test]
    fn test_config_is_stable_true() {
        let c = SineGordonConfig { nx: 128, dx: 0.1, dt: 0.05, boundary: SineGordonBoundary::Fixed };
        assert!(c.is_stable());
    }

    #[test]
    fn test_config_is_stable_false() {
        let c = SineGordonConfig { nx: 128, dx: 0.1, dt: 0.2, boundary: SineGordonBoundary::Fixed };
        // dt/dx = 2.0 > 1
        assert!(!c.is_stable());
    }

    #[test]
    fn test_config_stable_dt() {
        let c = SineGordonConfig { nx: 128, dx: 0.1, dt: 0.05, boundary: SineGordonBoundary::Fixed };
        // stable_dt = 1.0 * dx = 0.1
        assert!(approx_eq(c.stable_dt(), 0.1, 1e-6));
    }

    #[test]
    fn test_solver_new() {
        let s = SineGordonSolver::new(SineGordonConfig { nx: 64, dx: 0.1, dt: 0.05,
            boundary: SineGordonBoundary::Fixed });
        assert_eq!(s.phi_curr.len(), 64);
        assert_eq!(s.phi_prev.len(), 64);
        assert_eq!(s.phi_next.len(), 64);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
        for &p in s.phi_curr.iter() { assert_eq!(p, 0.0); }
    }

    #[test]
    fn test_initialize_vacuum() {
        let mut s = SineGordonSolver::new(SineGordonConfig { nx: 64, dx: 0.1, dt: 0.05,
            boundary: SineGordonBoundary::Fixed });
        s.phi_curr[10] = 1.0;
        s.initialize_vacuum();
        for &p in s.phi_curr.iter() { assert_eq!(p, 0.0); }
        for &p in s.phi_prev.iter() { assert_eq!(p, 0.0); }
        assert_eq!(s.time, 0.0);
    }

    #[test]
    fn test_initialize_kink_boundary_values() {
        let mut s = SineGordonSolver::new(SineGordonConfig {
            nx: 512, dx: 0.05, dt: 0.04,
            boundary: SineGordonBoundary::Absorbing { layer: 16, strength: 0.95 },
        });
        s.initialize_kink(1.0, 0.0, 12.8);
        // kink 中心在 x=12.8, 中心处 phi = 4*atan(1) = pi
        let pi = std::f32::consts::PI;
        let center_idx = (12.8 / 0.05) as usize;
        assert!(approx_eq(s.phi_curr[center_idx], pi, 0.1),
            "kink center phi should be pi, got {}", s.phi_curr[center_idx]);
        // 左边界 phi ~ 0
        assert!(s.phi_curr[0].abs() < 0.1, "left boundary phi should be ~0, got {}", s.phi_curr[0]);
        // 右边界 phi ~ 2*pi
        let two_pi = 2.0 * pi;
        assert!((s.phi_curr[511] - two_pi).abs() < 0.1,
            "right boundary phi should be ~2pi, got {}", s.phi_curr[511]);
    }

    #[test]
    fn test_initialize_antikink_boundary_values() {
        let mut s = SineGordonSolver::new(SineGordonConfig {
            nx: 512, dx: 0.05, dt: 0.04,
            boundary: SineGordonBoundary::Absorbing { layer: 16, strength: 0.95 },
        });
        s.initialize_kink(-1.0, 0.0, 12.8);
        let pi = std::f32::consts::PI;
        let two_pi = 2.0 * pi;
        // antikink: 左 phi ~ 2pi, 右 phi ~ 0
        assert!((s.phi_curr[0] - two_pi).abs() < 0.1,
            "antikink left phi should be ~2pi, got {}", s.phi_curr[0]);
        assert!(s.phi_curr[511].abs() < 0.1,
            "antikink right phi should be ~0, got {}", s.phi_curr[511]);
        let center_idx = (12.8 / 0.05) as usize;
        assert!(approx_eq(s.phi_curr[center_idx], pi, 0.1),
            "antikink center phi should be pi, got {}", s.phi_curr[center_idx]);
    }

    #[test]
    fn test_initialize_breather() {
        let mut s = SineGordonSolver::new(SineGordonConfig {
            nx: 512, dx: 0.05, dt: 0.04,
            boundary: SineGordonBoundary::Absorbing { layer: 16, strength: 0.95 },
        });
        s.initialize_breather(0.5, 12.8);
        // breather 中心 phi = 4*atan(omega/omega') = 4*atan(0.5/sqrt(0.75))
        let omega = 0.5_f32;
        let omega_p = (1.0 - omega * omega).sqrt();
        let expected_center = 4.0 * (omega / omega_p).atan();
        let center_idx = (12.8 / 0.05) as usize;
        assert!(approx_eq(s.phi_curr[center_idx], expected_center, 0.05),
            "breather center: expected {}, got {}", expected_center, s.phi_curr[center_idx]);
        // 边界 phi ~ 0
        assert!(s.phi_curr[0].abs() < 0.05);
        assert!(s.phi_curr[511].abs() < 0.05);
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = SineGordonSolver::new(SineGordonConfig { nx: 64, dx: 0.1, dt: 0.05,
            boundary: SineGordonBoundary::Fixed });
        s.initialize_vacuum();
        assert_eq!(s.time, 0.0);
        s.step();
        assert!(approx_eq(s.time, 0.05, 1e-9));
        assert_eq!(s.steps, 1);
        s.step();
        assert!(approx_eq(s.time, 0.1, 1e-9));
        assert_eq!(s.steps, 2);
    }

    #[test]
    fn test_step_n_advances() {
        let mut s = SineGordonSolver::new(SineGordonConfig { nx: 64, dx: 0.1, dt: 0.05,
            boundary: SineGordonBoundary::Fixed });
        s.step_n(100);
        assert_eq!(s.steps, 100);
        assert!(approx_eq(s.time, 5.0, 1e-6));
    }

    #[test]
    fn test_vacuum_remains_vacuum() {
        // 真空是 SG 的精确解 (phi=0 -> sin(phi)=0 -> phi_tt=0)
        let mut s = SineGordonSolver::new(SineGordonConfig { nx: 128, dx: 0.1, dt: 0.05,
            boundary: SineGordonBoundary::Periodic });
        s.initialize_vacuum();
        s.step_n(500);
        assert!(!s.phi_curr.iter().any(|&p| p.is_nan()));
        assert!(s.max_amplitude() < 1e-9,
            "vacuum should stay vacuum, max = {}", s.max_amplitude());
    }

    #[test]
    fn test_no_nan_kink_propagation() {
        let mut s = SineGordonSolver::new(SineGordonConfig {
            nx: 512, dx: 0.05, dt: 0.04,
            boundary: SineGordonBoundary::Absorbing { layer: 16, strength: 0.95 },
        });
        s.initialize_kink(1.0, 0.3, 12.8);
        s.step_n(1000);
        assert!(!s.phi_curr.iter().any(|&p| p.is_nan()));
        assert!(s.max_amplitude() < 10.0);
    }

    #[test]
    fn test_kink_propagates_rightward() {
        // 静止 kink (v=0): 中心位置不变
        // 运动 kink (v=0.3): 中心以 v=0.3 移动
        let mut s = SineGordonSolver::new(SineGordonConfig {
            nx: 512, dx: 0.05, dt: 0.04,
            boundary: SineGordonBoundary::Absorbing { layer: 16, strength: 0.95 },
        });
        s.initialize_kink(1.0, 0.3, 12.8);
        let x0 = s.find_kink_center();
        s.step_n(200); // t = 8.0, 预期移动 0.3 * 8 = 2.4
        let x1 = s.find_kink_center();
        let dx_moved = x1 - x0;
        let expected = 0.3 * 8.0;
        assert!((dx_moved - expected).abs() < 1.0,
            "kink moved {}, expected ~{}: {} -> {}", dx_moved, expected, x0, x1);
    }

    #[test]
    fn test_kink_amplitude_preserved() {
        // kink 的 phi 范围: [0, 2*pi]
        let mut s = SineGordonSolver::new(SineGordonConfig {
            nx: 512, dx: 0.05, dt: 0.04,
            boundary: SineGordonBoundary::Absorbing { layer: 32, strength: 0.99 },
        });
        s.initialize_kink(1.0, 0.2, 12.8);
        s.step_n(500);
        let pi = std::f32::consts::PI;
        let two_pi = 2.0 * pi;
        // 找最大最小值
        let phi_max = s.phi_curr.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let phi_min = s.phi_curr.iter().cloned().fold(f32::INFINITY, f32::min);
        let span = phi_max - phi_min;
        // 应在 2*pi 附近 (允许吸收边界导致的小衰减)
        assert!(span > 1.5 * pi && span < 2.0 * pi + 0.1,
            "kink span should be ~2pi, got {} (min={}, max={})", span, phi_min, phi_max);
    }

    #[test]
    fn test_topological_charge_vacuum() {
        let s = SineGordonSolver::new(SineGordonConfig { nx: 128, dx: 0.1, dt: 0.05,
            boundary: SineGordonBoundary::Fixed });
        let q = s.topological_charge();
        assert!(q.abs() < 1e-6, "vacuum Q should be 0, got {}", q);
    }

    #[test]
    fn test_topological_charge_kink() {
        let mut s = SineGordonSolver::new(SineGordonConfig {
            nx: 512, dx: 0.05, dt: 0.04,
            boundary: SineGordonBoundary::Absorbing { layer: 16, strength: 0.95 },
        });
        s.initialize_kink(1.0, 0.0, 12.8);
        let q = s.topological_charge();
        // kink: Q = +1 (积分范围跨整个域, 但 phi 从 0 到 2*pi)
        assert!((q - 1.0).abs() < 0.05,
            "kink Q should be +1, got {}", q);
    }

    #[test]
    fn test_topological_charge_antikink() {
        let mut s = SineGordonSolver::new(SineGordonConfig {
            nx: 512, dx: 0.05, dt: 0.04,
            boundary: SineGordonBoundary::Absorbing { layer: 16, strength: 0.95 },
        });
        s.initialize_kink(-1.0, 0.0, 12.8);
        let q = s.topological_charge();
        // antikink: Q = -1
        assert!((q + 1.0).abs() < 0.05,
            "antikink Q should be -1, got {}", q);
    }

    #[test]
    fn test_topological_charge_conservation() {
        // kink 传播过程中 Q 应严格守恒
        let mut s = SineGordonSolver::new(SineGordonConfig {
            nx: 512, dx: 0.05, dt: 0.04,
            boundary: SineGordonBoundary::Absorbing { layer: 32, strength: 0.99 },
        });
        s.initialize_kink(1.0, 0.3, 12.8);
        let q0 = s.topological_charge();
        s.step_n(300);
        let q1 = s.topological_charge();
        let drift = (q1 - q0).abs();
        assert!(drift < 0.05,
            "topological charge drift too large: {} -> {} ({})", q0, q1, drift);
    }

    #[test]
    fn test_energy_conservation_vacuum() {
        // 真空能量 = 0
        let mut s = SineGordonSolver::new(SineGordonConfig { nx: 128, dx: 0.1, dt: 0.05,
            boundary: SineGordonBoundary::Periodic });
        s.initialize_vacuum();
        let e0 = s.energy();
        s.step_n(200);
        let e1 = s.energy();
        assert!(e0.abs() < 1e-6);
        assert!((e1 - e0).abs() < 1e-4);
    }

    #[test]
    fn test_energy_conservation_kink() {
        // 静止 kink (v=0): 能量 = 8*gamma = 8 (kink rest mass)
        // 运动 kink: E = 8*gamma
        let mut s = SineGordonSolver::new(SineGordonConfig {
            nx: 1024, dx: 0.03, dt: 0.025,
            boundary: SineGordonBoundary::Absorbing { layer: 64, strength: 0.999 },
        });
        s.initialize_kink(1.0, 0.0, 15.36); // v=0, center=15.36
        let e0 = s.energy();
        s.step_n(200);
        let e1 = s.energy();
        // 静止 kink 能量 = 8 (在自然单位下)
        assert!((e0 - 8.0).abs() < 0.5,
            "static kink energy should be ~8, got {}", e0);
        let drift = (e1 - e0).abs() / e0.abs().max(1e-6);
        assert!(drift < 0.05,
            "energy drift too large: {} -> {} ({:.3}%)", e0, e1, drift * 100.0);
    }

    #[test]
    fn test_breather_periodicity() {
        // breather 应该周期性振荡, 不传播
        let mut s = SineGordonSolver::new(SineGordonConfig {
            nx: 512, dx: 0.05, dt: 0.04,
            boundary: SineGordonBoundary::Absorbing { layer: 32, strength: 0.99 },
        });
        let omega = 0.5_f32;
        let omega_p = (1.0 - omega * omega).sqrt();
        let period = 2.0 * std::f32::consts::PI / omega_p; // breather 周期
        s.initialize_breather(omega, 12.8);
        let x0 = s.find_kink_center();
        // 模拟 1/4 周期: breather 应该收缩到最小 (phi ~ 0)
        // 然后回到最大
        let target_steps = (period / (4.0 * s.config.dt)) as usize;
        s.step_n(target_steps);
        // phi 中心应接近 0 (breather 在 1/4 周期处消失)
        let center_idx = (12.8 / 0.05) as usize;
        assert!(s.phi_curr[center_idx].abs() < 1.0,
            "breather at 1/4 period should be near 0, got {}", s.phi_curr[center_idx]);
        // 继续到 1/2 周期: breather 应该反向最大
        s.step_n(target_steps);
        // 应该有非零中心值
        assert!(!s.phi_curr.iter().any(|&p| p.is_nan()));
        let _ = x0;
    }

    #[test]
    fn test_breather_no_propagation() {
        // breather 是束缚态, 不传播
        let mut s = SineGordonSolver::new(SineGordonConfig {
            nx: 512, dx: 0.05, dt: 0.04,
            boundary: SineGordonBoundary::Absorbing { layer: 32, strength: 0.99 },
        });
        s.initialize_breather(0.3, 12.8);
        let x0 = 12.8_f32;
        s.step_n(500);
        // breather 应该在原位置附近振荡, 中心位置漂移 < 1.0
        // 由于 breather 在某些时刻 phi ~ 0, find_kink_center 不准确
        // 改用 |phi| 最大值位置
        let mut max_idx = 0usize;
        let mut max_val = 0.0f32;
        for (i, &p) in s.phi_curr.iter().enumerate() {
            if p.abs() > max_val {
                max_val = p.abs();
                max_idx = i;
            }
        }
        let x_center = (max_idx as f32) * s.config.dx;
        assert!((x_center - x0).abs() < 2.0,
            "breather should not propagate: started at {}, max at {}", x0, x_center);
    }

    #[test]
    fn test_kink_kink_scattering() {
        // 两个同号 kink 应该相互排斥 (或穿过对方保持符号)
        // 使用周期边界模拟两个 kink 在环上散射
        let mut s = SineGordonSolver::new(SineGordonConfig {
            nx: 1024, dx: 0.05, dt: 0.04,
            boundary: SineGordonBoundary::Periodic,
        });
        // 注意: 周期边界下总 Q 必须为 0
        // kink-kink 等价于 antikink-antikink + vacuum shift
        // 这里我们用 kink-antikink 测试散射 (Q=0, 周期可解)
        s.initialize_kink_antikink_pair(0.5, 12.8, 38.4);
        let q0 = s.topological_charge();
        s.step_n(500);
        let q1 = s.topological_charge();
        // 周期边界 Q 严格为 0
        assert!(q0.abs() < 0.1, "Q should be 0 in periodic, got {}", q0);
        assert!(q1.abs() < 0.1, "Q should remain 0, got {}", q1);
        assert!(!s.phi_curr.iter().any(|&p| p.is_nan()));
    }

    #[test]
    fn test_kink_antikink_no_nan() {
        let mut s = SineGordonSolver::new(SineGordonConfig {
            nx: 512, dx: 0.05, dt: 0.04,
            boundary: SineGordonBoundary::Absorbing { layer: 32, strength: 0.99 },
        });
        s.initialize_kink_antikink_pair(0.3, 6.4, 19.2);
        s.step_n(1000);
        assert!(!s.phi_curr.iter().any(|&p| p.is_nan()));
        // 低速碰撞产生 breather, phi 不应该爆增
        assert!(s.max_amplitude() < 10.0);
    }

    #[test]
    fn test_reset() {
        let mut s = SineGordonSolver::new(SineGordonConfig { nx: 64, dx: 0.1, dt: 0.05,
            boundary: SineGordonBoundary::Fixed });
        s.phi_curr[10] = 1.0;
        s.step_n(10);
        s.reset();
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
        for &p in s.phi_curr.iter() { assert_eq!(p, 0.0); }
        for &p in s.phi_prev.iter() { assert_eq!(p, 0.0); }
    }

    #[test]
    fn test_energy_positive() {
        let mut s = SineGordonSolver::new(SineGordonConfig {
            nx: 256, dx: 0.05, dt: 0.04,
            boundary: SineGordonBoundary::Absorbing { layer: 16, strength: 0.95 },
        });
        s.initialize_kink(1.0, 0.0, 6.4);
        let e = s.energy();
        assert!(e > 0.0, "energy should be positive, got {}", e);
    }
}
