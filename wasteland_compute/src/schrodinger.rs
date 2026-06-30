//! Time-dependent Schrödinger Equation (TDSE) FDTD Solver
//!
//! 量子力学波函数演化, 含时 Schrödinger 方程的有限差分时域求解.
//!
//! 方程 (原子单位 hbar = m_e = e = 1):
//!   i d psi/dt = H psi = -(1/2m) nabla^2 psi + V(x) psi
//!
//! 分离实虚部 psi = u + iv:
//!   du/dt = H v = -(1/2m) nabla^2 v + V v
//!   dv/dt = -H u = (1/2m) nabla^2 u - V u
//!
//! Leapfrog 时间积分 (u 整步, v 半步超前):
//!   v^{n+1/2} = v^{n-1/2} - dt * H * u^n
//!   u^{n+1} = u^n + dt * H * v^{n+1/2}
//!
//! 稳定性: dt < m * dx^2 / d  (d = 维度)
//!
//! 边界:
//!   HardWall  - psi = 0 (无限深势阱, 反射)
//!   Periodic  - 周期性
//!   Absorbing - sponge 衰减层 (近似开放边界, 波泄漏)
//!
//! 应用: 量子隧穿, 干涉, 衍射, 量子谐振子, 势阱束缚态, 波包演化.
//!
//! 基于 Schrödinger 1926, Yee 1966 (FDTD), Sullivan 2000.

use serde::{Deserialize, Serialize};

/// 约化普朗克常数 (原子单位)
pub const HBAR: f32 = 1.0;
/// 电子质量 (原子单位)
pub const M_E: f32 = 1.0;
/// 玻尔半径 (原子单位)
pub const A0: f32 = 1.0;
/// 哈特里能量 (原子单位)
pub const E_H: f32 = 1.0;

// SI 转换常数
pub const HBAR_SI: f32 = 1.054571817e-34;
pub const M_E_SI: f32 = 9.1093837015e-31;
pub const A0_SI: f32 = 5.29177210903e-11;
pub const E_H_SI: f32 = 4.3597447222071e-18;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum QmBoundary {
    HardWall,
    Periodic,
    Absorbing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchrodingerConfig {
    pub nx: usize,
    pub ny: usize,
    pub dx: f32,
    pub dt: f32,
    pub mass: f32,
    pub boundary: QmBoundary,
}

impl Default for SchrodingerConfig {
    fn default() -> Self {
        SchrodingerConfig {
            nx: 128,
            ny: 1,
            dx: 0.1,
            dt: 0.005,
            mass: 1.0,
            boundary: QmBoundary::HardWall,
        }
    }
}

impl SchrodingerConfig {
    pub fn dims(&self) -> usize {
        if self.ny <= 1 { 1 } else { 2 }
    }
    pub fn cell_volume(&self) -> f32 {
        if self.dims() == 1 { self.dx } else { self.dx * self.dx }
    }
    pub fn stability_dt(&self) -> f32 {
        let d = self.dims() as f32;
        self.mass * self.dx * self.dx / d
    }
    pub fn is_stable(&self) -> bool {
        self.dt < self.stability_dt()
    }
}

#[derive(Debug, Clone)]
pub enum PotentialPreset {
    Free,
    SquareWell { center: f32, half_width: f32, depth: f32 },
    Barrier { center: f32, half_width: f32, height: f32 },
    Harmonic { omega: f32, center: f32 },
    DoubleWell { separation: f32, height: f32, center: f32 },
    Step { position: f32, height: f32 },
}

pub struct SchrodingerSolver {
    pub config: SchrodingerConfig,
    pub psi_re: Vec<f32>,
    pub psi_im: Vec<f32>,
    pub potential: Vec<f32>,
    pub time: f32,
    pub steps: usize,
}

impl SchrodingerSolver {
    pub fn new(config: SchrodingerConfig) -> Self {
        let n = config.nx * config.ny;
        SchrodingerSolver {
            config,
            psi_re: vec![0.0; n],
            psi_im: vec![0.0; n],
            potential: vec![0.0; n],
            time: 0.0,
            steps: 0,
        }
    }

    pub fn idx(&self, i: usize, j: usize) -> usize {
        i + self.config.nx * j
    }

    pub fn x_at(&self, i: usize) -> f32 {
        (i as f32) * self.config.dx
    }

    pub fn y_at(&self, j: usize) -> f32 {
        (j as f32) * self.config.dx
    }

    pub fn set_potential(&mut self, preset: &PotentialPreset) {
        match preset {
            PotentialPreset::Free => {
                for v in self.potential.iter_mut() { *v = 0.0; }
            }
            PotentialPreset::SquareWell { center, half_width, depth } => {
                for i in 0..self.config.nx {
                    let x = self.x_at(i);
                    let v = if (x - center).abs() < *half_width { 0.0 } else { *depth };
                    for j in 0..self.config.ny {
                        let idx = self.idx(i, j);
                        self.potential[idx] = v;
                    }
                }
            }
            PotentialPreset::Barrier { center, half_width, height } => {
                for i in 0..self.config.nx {
                    let x = self.x_at(i);
                    let v = if (x - center).abs() < *half_width { *height } else { 0.0 };
                    for j in 0..self.config.ny {
                        let idx = self.idx(i, j);
                        self.potential[idx] = v;
                    }
                }
            }
            PotentialPreset::Harmonic { omega, center } => {
                let w2 = omega * omega;
                for i in 0..self.config.nx {
                    let x = self.x_at(i) - center;
                    for j in 0..self.config.ny {
                        let y = self.y_at(j);
                        let idx = self.idx(i, j);
                        self.potential[idx] = 0.5 * w2 * (x * x + y * y);
                    }
                }
            }
            PotentialPreset::DoubleWell { separation, height, center } => {
                for i in 0..self.config.nx {
                    let x = self.x_at(i) - center;
                    let xs = x / separation;
                    let v = height * (xs * xs - 1.0).powi(2);
                    for j in 0..self.config.ny {
                        let idx = self.idx(i, j);
                        self.potential[idx] = v;
                    }
                }
            }
            PotentialPreset::Step { position, height } => {
                for i in 0..self.config.nx {
                    let x = self.x_at(i);
                    let v = if x > *position { *height } else { 0.0 };
                    for j in 0..self.config.ny {
                        let idx = self.idx(i, j);
                        self.potential[idx] = v;
                    }
                }
            }
        }
    }

    pub fn initialize_gaussian_packet(&mut self, center: [f32; 2], width: f32, momentum: [f32; 2]) {
        let inv_w2 = 1.0 / (width * width);
        for j in 0..self.config.ny {
            for i in 0..self.config.nx {
                let x = self.x_at(i);
                let y = self.y_at(j);
                let dx = x - center[0];
                let dy = y - center[1];
                let r2 = dx * dx + dy * dy;
                let phase = momentum[0] * x + momentum[1] * y;
                let amp = (-0.5 * r2 * inv_w2).exp();
                let idx = self.idx(i, j);
                self.psi_re[idx] = amp * phase.cos();
                self.psi_im[idx] = amp * phase.sin();
            }
        }
        self.normalize();
        self.kick_im_half();
    }

    pub fn initialize_plane_wave(&mut self, momentum: [f32; 2]) {
        for j in 0..self.config.ny {
            for i in 0..self.config.nx {
                let x = self.x_at(i);
                let y = self.y_at(j);
                let phase = momentum[0] * x + momentum[1] * y;
                let idx = self.idx(i, j);
                self.psi_re[idx] = phase.cos();
                self.psi_im[idx] = phase.sin();
            }
        }
        self.normalize();
        self.kick_im_half();
    }

    fn kick_im_half(&mut self) {
        let dt = self.config.dt;
        let n = self.psi_re.len();
        let mut h_u = vec![0.0; n];
        let snapshot = self.psi_re.clone();
        self.hamiltonian_apply(&snapshot, &mut h_u);
        for i in 0..n {
            self.psi_im[i] -= 0.5 * dt * h_u[i];
        }
    }

    fn hamiltonian_apply(&self, field: &[f32], result: &mut [f32]) {
        let dx2 = self.config.dx * self.config.dx;
        let inv_2m = 0.5 / self.config.mass;
        let nx = self.config.nx;
        let ny = self.config.ny;
        let two_d = ny > 1;
        match self.config.boundary {
            QmBoundary::Periodic => {
                for j in 0..ny {
                    for i in 0..nx {
                        let ip = if i + 1 < nx { i + 1 } else { 0 };
                        let im = if i > 0 { i - 1 } else { nx - 1 };
                        let idx = self.idx(i, j);
                        let lap = if two_d {
                            let jp = if j + 1 < ny { j + 1 } else { 0 };
                            let jm = if j > 0 { j - 1 } else { ny - 1 };
                            (field[self.idx(ip, j)] + field[self.idx(im, j)]
                                + field[self.idx(i, jp)] + field[self.idx(i, jm)]
                                - 4.0 * field[idx]) / dx2
                        } else {
                            (field[self.idx(ip, j)] + field[self.idx(im, j)] - 2.0 * field[idx]) / dx2
                        };
                        result[idx] = -inv_2m * lap + self.potential[idx] * field[idx];
                    }
                }
            }
            _ => {
                let hardwall = self.config.boundary == QmBoundary::HardWall;
                for j in 0..ny {
                    for i in 0..nx {
                        let idx = self.idx(i, j);
                        let is_boundary = i == 0 || i == nx - 1 || (two_d && (j == 0 || j == ny - 1));
                        if is_boundary && hardwall {
                            result[idx] = 0.0;
                            continue;
                        }
                        let f_ip = if i + 1 < nx { field[self.idx(i + 1, j)] } else { 0.0 };
                        let f_im = if i > 0 { field[self.idx(i - 1, j)] } else { 0.0 };
                        let lap = if two_d {
                            let f_jp = if j + 1 < ny { field[self.idx(i, j + 1)] } else { 0.0 };
                            let f_jm = if j > 0 { field[self.idx(i, j - 1)] } else { 0.0 };
                            (f_ip + f_im + f_jp + f_jm - 4.0 * field[idx]) / dx2
                        } else {
                            (f_ip + f_im - 2.0 * field[idx]) / dx2
                        };
                        result[idx] = -inv_2m * lap + self.potential[idx] * field[idx];
                    }
                }
            }
        }
    }

    fn apply_absorbing(&mut self) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let width = (nx / 8).max(1).min(20);
        let damp = 0.05;
        for i in 0..nx {
            for j in 0..ny {
                let mut dist = i.min(nx - 1 - i);
                if ny > 1 {
                    dist = dist.min(j.min(ny - 1 - j));
                }
                if dist < width {
                    let factor = 1.0 - damp * (width - dist) as f32 / width as f32;
                    let idx = self.idx(i, j);
                    self.psi_re[idx] *= factor;
                    self.psi_im[idx] *= factor;
                }
            }
        }
    }

    pub fn step(&mut self) {
        let dt = self.config.dt;
        let n = self.psi_re.len();
        let mut h_u = vec![0.0; n];
        let snapshot = self.psi_re.clone();
        self.hamiltonian_apply(&snapshot, &mut h_u);
        for i in 0..n {
            self.psi_im[i] -= dt * h_u[i];
        }
        let mut h_v = vec![0.0; n];
        let snapshot2 = self.psi_im.clone();
        self.hamiltonian_apply(&snapshot2, &mut h_v);
        for i in 0..n {
            self.psi_re[i] += dt * h_v[i];
        }
        if self.config.boundary == QmBoundary::Absorbing {
            self.apply_absorbing();
        }
        self.time += dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    pub fn probability_density(&self) -> Vec<f32> {
        self.psi_re.iter().zip(self.psi_im.iter())
            .map(|(r, i)| r * r + i * i)
            .collect()
    }

    pub fn total_probability(&self) -> f32 {
        let vol = self.config.cell_volume();
        let sum: f32 = self.psi_re.iter().zip(self.psi_im.iter())
            .map(|(r, i)| r * r + i * i)
            .sum();
        sum * vol
    }

    pub fn norm(&self) -> f32 {
        self.total_probability().sqrt()
    }

    pub fn normalize(&mut self) {
        let n = self.norm();
        if n > 1e-12 {
            let inv = 1.0 / n;
            for i in 0..self.psi_re.len() {
                self.psi_re[i] *= inv;
                self.psi_im[i] *= inv;
            }
        }
    }

    pub fn energy(&self) -> f32 {
        let vol = self.config.cell_volume();
        let n = self.psi_re.len();
        let mut h_re = vec![0.0; n];
        let mut h_im = vec![0.0; n];
        let snap_re = self.psi_re.clone();
        let snap_im = self.psi_im.clone();
        self.hamiltonian_apply(&snap_re, &mut h_re);
        self.hamiltonian_apply(&snap_im, &mut h_im);
        let mut e = 0.0;
        for i in 0..n {
            e += self.psi_re[i] * h_re[i] + self.psi_im[i] * h_im[i];
        }
        e * vol
    }

    pub fn potential_energy(&self) -> f32 {
        let vol = self.config.cell_volume();
        let mut e = 0.0;
        for i in 0..self.psi_re.len() {
            let prob = self.psi_re[i] * self.psi_re[i] + self.psi_im[i] * self.psi_im[i];
            e += self.potential[i] * prob;
        }
        e * vol
    }

    pub fn kinetic_energy(&self) -> f32 {
        self.energy() - self.potential_energy()
    }

    pub fn expected_position(&self) -> [f32; 2] {
        let vol = self.config.cell_volume();
        let mut sx = 0.0;
        let mut sy = 0.0;
        for j in 0..self.config.ny {
            for i in 0..self.config.nx {
                let idx = self.idx(i, j);
                let prob = self.psi_re[idx] * self.psi_re[idx] + self.psi_im[idx] * self.psi_im[idx];
                sx += self.x_at(i) * prob;
                sy += self.y_at(j) * prob;
            }
        }
        [sx * vol, sy * vol]
    }

    pub fn expected_momentum(&self) -> [f32; 2] {
        let vol = self.config.cell_volume();
        let inv_2dx = 1.0 / (2.0 * self.config.dx);
        let mut px = 0.0;
        let mut py = 0.0;
        let nx = self.config.nx;
        let ny = self.config.ny;
        for j in 0..ny {
            for i in 0..nx {
                if i == 0 || i == nx - 1 {
                    continue;
                }
                let idx = self.idx(i, j);
                let idx_ip = self.idx(i + 1, j);
                let idx_im = self.idx(i - 1, j);
                let du_dx = (self.psi_re[idx_ip] - self.psi_re[idx_im]) * inv_2dx;
                let dv_dx = (self.psi_im[idx_ip] - self.psi_im[idx_im]) * inv_2dx;
                px += self.psi_re[idx] * dv_dx - self.psi_im[idx] * du_dx;
                if ny > 1 && j > 0 && j < ny - 1 {
                    let idx_jp = self.idx(i, j + 1);
                    let idx_jm = self.idx(i, j - 1);
                    let du_dy = (self.psi_re[idx_jp] - self.psi_re[idx_jm]) * inv_2dx;
                    let dv_dy = (self.psi_im[idx_jp] - self.psi_im[idx_jm]) * inv_2dx;
                    py += self.psi_re[idx] * dv_dy - self.psi_im[idx] * du_dy;
                }
            }
        }
        [px * vol, py * vol]
    }

    pub fn position_variance(&self) -> f32 {
        let vol = self.config.cell_volume();
        let [mx, _] = self.expected_position();
        let mut sx2 = 0.0;
        for j in 0..self.config.ny {
            for i in 0..self.config.nx {
                let idx = self.idx(i, j);
                let prob = self.psi_re[idx] * self.psi_re[idx] + self.psi_im[idx] * self.psi_im[idx];
                let dx = self.x_at(i) - mx;
                sx2 += dx * dx * prob;
            }
        }
        sx2 * vol
    }

    pub fn reset(&mut self) {
        for v in self.psi_re.iter_mut() { *v = 0.0; }
        for v in self.psi_im.iter_mut() { *v = 0.0; }
        for v in self.potential.iter_mut() { *v = 0.0; }
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

    fn default_1d(nx: usize) -> SchrodingerConfig {
        SchrodingerConfig {
            nx,
            ny: 1,
            dx: 0.1,
            dt: 0.005,
            mass: 1.0,
            boundary: QmBoundary::HardWall,
        }
    }

    // ============ 常数测试 ============

    #[test]
    fn test_atomic_units() {
        assert!(approx_eq(HBAR, 1.0, 1e-6));
        assert!(approx_eq(M_E, 1.0, 1e-6));
        assert!(approx_eq(A0, 1.0, 1e-6));
        assert!(approx_eq(E_H, 1.0, 1e-6));
    }

    #[test]
    fn test_si_constants_finite() {
        assert!(HBAR_SI > 0.0 && HBAR_SI.is_finite());
        assert!(M_E_SI > 0.0 && M_E_SI.is_finite());
        assert!(A0_SI > 0.0 && A0_SI.is_finite());
        assert!(E_H_SI > 0.0 && E_H_SI.is_finite());
    }

    #[test]
    fn test_si_constants_values() {
        assert!(approx_eq(HBAR_SI, 1.054571817e-34, 1e-40));
        assert!(approx_eq(M_E_SI, 9.1093837015e-31, 1e-37));
    }

    // ============ QmBoundary 测试 ============

    #[test]
    fn test_qm_boundary_equality() {
        assert_eq!(QmBoundary::HardWall, QmBoundary::HardWall);
        assert_eq!(QmBoundary::Periodic, QmBoundary::Periodic);
        assert_eq!(QmBoundary::Absorbing, QmBoundary::Absorbing);
        assert_ne!(QmBoundary::HardWall, QmBoundary::Periodic);
    }

    // ============ SchrodingerConfig 测试 ============

    #[test]
    fn test_config_default() {
        let c = SchrodingerConfig::default();
        assert_eq!(c.nx, 128);
        assert_eq!(c.ny, 1);
        assert!(approx_eq(c.dx, 0.1, 1e-6));
        assert!(approx_eq(c.dt, 0.005, 1e-6));
        assert!(approx_eq(c.mass, 1.0, 1e-6));
        assert_eq!(c.boundary, QmBoundary::HardWall);
    }

    #[test]
    fn test_config_dims() {
        let c1 = SchrodingerConfig { nx: 10, ny: 1, ..Default::default() };
        assert_eq!(c1.dims(), 1);
        let c2 = SchrodingerConfig { nx: 10, ny: 10, ..Default::default() };
        assert_eq!(c2.dims(), 2);
    }

    #[test]
    fn test_config_cell_volume() {
        let c1 = SchrodingerConfig { nx: 10, ny: 1, dx: 0.2, ..Default::default() };
        assert!(approx_eq(c1.cell_volume(), 0.2, 1e-6));
        let c2 = SchrodingerConfig { nx: 10, ny: 10, dx: 0.2, ..Default::default() };
        assert!(approx_eq(c2.cell_volume(), 0.04, 1e-6));
    }

    #[test]
    fn test_config_stability_dt() {
        let c1 = SchrodingerConfig { nx: 10, ny: 1, dx: 0.1, mass: 1.0, ..Default::default() };
        assert!(approx_eq(c1.stability_dt(), 0.01, 1e-6));
        let c2 = SchrodingerConfig { nx: 10, ny: 10, dx: 0.1, mass: 1.0, ..Default::default() };
        assert!(approx_eq(c2.stability_dt(), 0.005, 1e-6));
    }

    #[test]
    fn test_config_is_stable() {
        let c = SchrodingerConfig::default();
        assert!(c.is_stable());
        let c2 = SchrodingerConfig { dt: 0.02, ..Default::default() };
        assert!(!c2.is_stable());
    }

    // ============ SchrodingerSolver 创建测试 ============

    #[test]
    fn test_solver_new() {
        let s = SchrodingerSolver::new(default_1d(64));
        assert_eq!(s.psi_re.len(), 64);
        assert_eq!(s.psi_im.len(), 64);
        assert_eq!(s.potential.len(), 64);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
        for v in &s.psi_re { assert_eq!(*v, 0.0); }
        for v in &s.potential { assert_eq!(*v, 0.0); }
    }

    #[test]
    fn test_solver_idx() {
        let s = SchrodingerSolver::new(SchrodingerConfig { nx: 10, ny: 5, ..Default::default() });
        assert_eq!(s.idx(0, 0), 0);
        assert_eq!(s.idx(9, 0), 9);
        assert_eq!(s.idx(0, 1), 10);
        assert_eq!(s.idx(3, 2), 23);
    }

    #[test]
    fn test_solver_x_at() {
        let s = SchrodingerSolver::new(default_1d(64));
        assert!(approx_eq(s.x_at(0), 0.0, 1e-6));
        assert!(approx_eq(s.x_at(1), 0.1, 1e-6));
        assert!(approx_eq(s.x_at(10), 1.0, 1e-6));
    }

    // ============ 势能预设测试 ============

    #[test]
    fn test_potential_free() {
        let mut s = SchrodingerSolver::new(default_1d(32));
        s.set_potential(&PotentialPreset::Free);
        for v in &s.potential {
            assert_eq!(*v, 0.0);
        }
    }

    #[test]
    fn test_potential_barrier() {
        let mut s = SchrodingerSolver::new(default_1d(64));
        s.set_potential(&PotentialPreset::Barrier { center: 3.2, half_width: 0.5, height: 10.0 });
        let center_idx = 32;
        assert!(approx_eq(s.potential[center_idx], 10.0, 1e-5));
        assert!(approx_eq(s.potential[0], 0.0, 1e-5));
    }

    #[test]
    fn test_potential_step() {
        let mut s = SchrodingerSolver::new(default_1d(64));
        s.set_potential(&PotentialPreset::Step { position: 3.2, height: 5.0 });
        assert!(approx_eq(s.potential[40], 5.0, 1e-5));
        assert!(approx_eq(s.potential[10], 0.0, 1e-5));
    }

    #[test]
    fn test_potential_harmonic() {
        let mut s = SchrodingerSolver::new(default_1d(64));
        s.set_potential(&PotentialPreset::Harmonic { omega: 1.0, center: 3.2 });
        let idx_center = 32;
        assert!(approx_eq(s.potential[idx_center], 0.0, 1e-5));
        let idx_off = 42;
        assert!(approx_eq(s.potential[idx_off], 0.5, 1e-5));
    }

    #[test]
    fn test_potential_square_well() {
        let mut s = SchrodingerSolver::new(default_1d(64));
        s.set_potential(&PotentialPreset::SquareWell { center: 3.2, half_width: 1.0, depth: 100.0 });
        let idx_center = 32;
        assert!(approx_eq(s.potential[idx_center], 0.0, 1e-5));
        assert!(approx_eq(s.potential[0], 100.0, 1e-5));
    }

    // ============ 初始化测试 ============

    #[test]
    fn test_initialize_gaussian_packet_normalized() {
        let mut s = SchrodingerSolver::new(default_1d(128));
        s.initialize_gaussian_packet([6.4, 0.0], 1.0, [0.0, 0.0]);
        let prob = s.total_probability();
        assert!(approx_eq(prob, 1.0, 1e-3), "total_probability = {}", prob);
    }

    #[test]
    fn test_initialize_gaussian_packet_center() {
        let mut s = SchrodingerSolver::new(default_1d(128));
        s.initialize_gaussian_packet([6.4, 0.0], 1.0, [0.0, 0.0]);
        let [mx, _] = s.expected_position();
        assert!(approx_eq(mx, 6.4, 0.1), "expected_position = {}", mx);
    }

    #[test]
    fn test_initialize_gaussian_packet_momentum() {
        let mut s = SchrodingerSolver::new(default_1d(256));
        s.initialize_gaussian_packet([12.8, 0.0], 1.0, [0.5, 0.0]);
        let [px, _] = s.expected_momentum();
        assert!(approx_eq(px, 0.5, 0.05), "expected_momentum = {}", px);
    }

    #[test]
    fn test_initialize_plane_wave_normalized() {
        let mut s = SchrodingerSolver::new(default_1d(128));
        s.initialize_plane_wave([0.5, 0.0]);
        let prob = s.total_probability();
        assert!(approx_eq(prob, 1.0, 1e-3), "plane wave prob = {}", prob);
    }

    // ============ 观测量测试 ============

    #[test]
    fn test_total_probability() {
        let mut s = SchrodingerSolver::new(default_1d(128));
        s.initialize_gaussian_packet([6.4, 0.0], 1.0, [0.0, 0.0]);
        assert!(approx_eq(s.total_probability(), 1.0, 1e-3));
    }

    #[test]
    fn test_probability_density() {
        let mut s = SchrodingerSolver::new(default_1d(128));
        s.initialize_gaussian_packet([6.4, 0.0], 1.0, [0.0, 0.0]);
        let pd = s.probability_density();
        assert_eq!(pd.len(), 128);
        let center = 64;
        let mut max_idx = 0;
        let mut max_val = 0.0;
        for i in 0..128 {
            if pd[i] > max_val {
                max_val = pd[i];
                max_idx = i;
            }
        }
        assert!((max_idx as i32 - center as i32).abs() <= 2,
            "max at {}, expected near {}", max_idx, center);
    }

    #[test]
    fn test_position_variance() {
        let mut s = SchrodingerSolver::new(default_1d(256));
        s.initialize_gaussian_packet([12.8, 0.0], 1.0, [0.0, 0.0]);
        let var = s.position_variance();
        assert!(approx_eq(var, 0.5, 0.05), "variance = {}", var);
    }

    #[test]
    fn test_energy_zero_potential() {
        let mut s = SchrodingerSolver::new(default_1d(256));
        s.set_potential(&PotentialPreset::Free);
        s.initialize_gaussian_packet([12.8, 0.0], 1.0, [0.0, 0.0]);
        let ke = s.kinetic_energy();
        let pe = s.potential_energy();
        assert!(approx_eq(pe, 0.0, 1e-6));
        assert!(approx_eq(ke, 0.25, 0.02), "kinetic_energy = {}", ke);
    }

    // ============ 演化测试 ============

    #[test]
    fn test_step_progress() {
        let mut s = SchrodingerSolver::new(default_1d(64));
        s.initialize_gaussian_packet([3.2, 0.0], 1.0, [0.0, 0.0]);
        s.step();
        assert_eq!(s.steps, 1);
        assert!(approx_eq(s.time, 0.005, 1e-6));
    }

    #[test]
    fn test_free_evolution_probability_conservation() {
        let mut s = SchrodingerSolver::new(default_1d(256));
        s.set_potential(&PotentialPreset::Free);
        s.initialize_gaussian_packet([12.8, 0.0], 1.0, [0.0, 0.0]);
        let prob_before = s.total_probability();
        s.step_n(100);
        let prob_after = s.total_probability();
        assert!(approx_eq(prob_after, prob_before, 1e-3),
            "prob drift: {} -> {}", prob_before, prob_after);
    }

    #[test]
    fn test_free_evolution_energy_conservation() {
        let mut s = SchrodingerSolver::new(default_1d(256));
        s.set_potential(&PotentialPreset::Free);
        s.initialize_gaussian_packet([12.8, 0.0], 1.0, [0.0, 0.0]);
        let e_before = s.energy();
        s.step_n(100);
        let e_after = s.energy();
        assert!((e_after - e_before).abs() / e_before.abs() < 0.02,
            "energy drift: {} -> {}", e_before, e_after);
    }

    #[test]
    fn test_free_evolution_momentum_conservation() {
        let mut s = SchrodingerSolver::new(default_1d(256));
        s.set_potential(&PotentialPreset::Free);
        s.initialize_gaussian_packet([12.8, 0.0], 1.0, [0.5, 0.0]);
        let [px_before, _] = s.expected_momentum();
        s.step_n(50);
        let [px_after, _] = s.expected_momentum();
        assert!((px_after - px_before).abs() < 0.05,
            "momentum drift: {} -> {}", px_before, px_after);
    }

    #[test]
    fn test_gaussian_spreads() {
        let mut s = SchrodingerSolver::new(default_1d(256));
        s.set_potential(&PotentialPreset::Free);
        s.initialize_gaussian_packet([12.8, 0.0], 1.0, [0.0, 0.0]);
        let var_before = s.position_variance();
        s.step_n(100);
        let var_after = s.position_variance();
        assert!(var_after > var_before,
            "variance should increase: {} -> {}", var_before, var_after);
    }

    // ============ 物理场景测试 ============

    #[test]
    fn test_harmonic_ground_state_energy() {
        let cfg = SchrodingerConfig {
            nx: 256,
            ny: 1,
            dx: 0.1,
            dt: 0.002,
            mass: 1.0,
            boundary: QmBoundary::HardWall,
        };
        let mut s = SchrodingerSolver::new(cfg);
        let omega = 1.0_f32;
        s.set_potential(&PotentialPreset::Harmonic { omega, center: 12.8 });
        let width = 1.0 / omega.sqrt();
        s.initialize_gaussian_packet([12.8, 0.0], width, [0.0, 0.0]);
        let e = s.energy();
        assert!(approx_eq(e, omega / 2.0, 0.05),
            "ground state energy = {}, expected {}", e, omega / 2.0);
    }

    #[test]
    fn test_harmonic_ground_state_stability() {
        let cfg = SchrodingerConfig {
            nx: 256,
            ny: 1,
            dx: 0.1,
            dt: 0.002,
            mass: 1.0,
            boundary: QmBoundary::HardWall,
        };
        let mut s = SchrodingerSolver::new(cfg);
        let omega = 1.0_f32;
        s.set_potential(&PotentialPreset::Harmonic { omega, center: 12.8 });
        let width = 1.0 / omega.sqrt();
        s.initialize_gaussian_packet([12.8, 0.0], width, [0.0, 0.0]);
        let var_before = s.position_variance();
        let e_before = s.energy();
        s.step_n(200);
        let var_after = s.position_variance();
        let e_after = s.energy();
        assert!((var_after - var_before).abs() / var_before < 0.05,
            "variance drift: {} -> {}", var_before, var_after);
        assert!((e_after - e_before).abs() / e_before.abs() < 0.02,
            "energy drift: {} -> {}", e_before, e_after);
    }

    #[test]
    fn test_hardwall_reflection_probability() {
        let mut s = SchrodingerSolver::new(default_1d(128));
        s.set_potential(&PotentialPreset::Free);
        s.initialize_gaussian_packet([6.4, 0.0], 0.8, [0.0, 0.0]);
        let prob_before = s.total_probability();
        s.step_n(200);
        let prob_after = s.total_probability();
        assert!(approx_eq(prob_after, prob_before, 1e-3),
            "prob: {} -> {}", prob_before, prob_after);
    }

    #[test]
    fn test_periodic_probability_conservation() {
        let cfg = SchrodingerConfig {
            nx: 128,
            ny: 1,
            dx: 0.1,
            dt: 0.005,
            mass: 1.0,
            boundary: QmBoundary::Periodic,
        };
        let mut s = SchrodingerSolver::new(cfg);
        s.set_potential(&PotentialPreset::Free);
        s.initialize_gaussian_packet([6.4, 0.0], 1.0, [0.0, 0.0]);
        let prob_before = s.total_probability();
        s.step_n(100);
        let prob_after = s.total_probability();
        assert!(approx_eq(prob_after, prob_before, 1e-3),
            "periodic prob: {} -> {}", prob_before, prob_after);
    }

    #[test]
    fn test_absorbing_reduces_probability() {
        let cfg = SchrodingerConfig {
            nx: 128,
            ny: 1,
            dx: 0.1,
            dt: 0.005,
            mass: 1.0,
            boundary: QmBoundary::Absorbing,
        };
        let mut s = SchrodingerSolver::new(cfg);
        s.set_potential(&PotentialPreset::Free);
        s.initialize_gaussian_packet([6.4, 0.0], 1.0, [2.0, 0.0]);
        let prob_before = s.total_probability();
        s.step_n(300);
        let prob_after = s.total_probability();
        assert!(prob_after < prob_before,
            "absorbing should reduce prob: {} -> {}", prob_before, prob_after);
    }

    #[test]
    fn test_barrier_transmission() {
        let mut s = SchrodingerSolver::new(default_1d(256));
        s.set_potential(&PotentialPreset::Barrier { center: 12.0, half_width: 0.3, height: 1.0 });
        s.initialize_gaussian_packet([8.0, 0.0], 1.0, [2.0, 0.0]);
        s.step_n(500);
        let pd = s.probability_density();
        let mut right_prob = 0.0;
        for i in 130..256 {
            right_prob += pd[i] * 0.1;
        }
        assert!(right_prob > 0.1, "right_prob = {}", right_prob);
    }

    #[test]
    fn test_long_run_stability() {
        let mut s = SchrodingerSolver::new(default_1d(128));
        s.set_potential(&PotentialPreset::Free);
        s.initialize_gaussian_packet([6.4, 0.0], 1.0, [0.0, 0.0]);
        s.step_n(200);
        for v in &s.psi_re { assert!(v.is_finite(), "psi_re = {}", v); }
        for v in &s.psi_im { assert!(v.is_finite(), "psi_im = {}", v); }
    }

    #[test]
    fn test_reset() {
        let mut s = SchrodingerSolver::new(default_1d(64));
        s.set_potential(&PotentialPreset::Barrier { center: 3.2, half_width: 0.5, height: 10.0 });
        s.initialize_gaussian_packet([3.2, 0.0], 1.0, [0.0, 0.0]);
        s.step_n(10);
        s.reset();
        assert_eq!(s.steps, 0);
        assert!(approx_eq(s.time, 0.0, 1e-6));
        for v in &s.psi_re { assert_eq!(*v, 0.0); }
        for v in &s.psi_im { assert_eq!(*v, 0.0); }
        for v in &s.potential { assert_eq!(*v, 0.0); }
    }

    #[test]
    fn test_2d_solver() {
        let cfg = SchrodingerConfig {
            nx: 32,
            ny: 32,
            dx: 0.2,
            dt: 0.002,
            mass: 1.0,
            boundary: QmBoundary::HardWall,
        };
        let mut s = SchrodingerSolver::new(cfg);
        assert_eq!(s.psi_re.len(), 32 * 32);
        s.set_potential(&PotentialPreset::Free);
        s.initialize_gaussian_packet([3.2, 3.2], 1.0, [0.0, 0.0]);
        let prob = s.total_probability();
        assert!(approx_eq(prob, 1.0, 1e-2), "2D prob = {}", prob);
        s.step_n(10);
        for v in &s.psi_re { assert!(v.is_finite()); }
    }
}
