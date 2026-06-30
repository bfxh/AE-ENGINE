//! Particle-in-Cell (PIC) Plasma Solver
//!
//! 等离子体物理模拟. Boris pusher (1970) 推进带电粒子在电磁场中,
//! 双线性电荷沉积到网格, FFT 解泊松方程得到电场.
//!
//! 算法:
//!   1. 电荷沉积: rho(x_grid) = sum q * W(x_p - x_grid)  (双线性)
//!   2. FFT 泊松: nabla^2 phi = -rho/eps0  ->  phi_k = rho_k / (k^2 eps0)
//!   3. 电场: E = -grad(phi)  (中心差分, 周期边界)
//!   4. Boris pusher:
//!      v_minus = v + (q/m)*E*(dt/2)
//!      t = (q/m)*B*(dt/2),  s = 2t/(1+|t|^2)
//!      v_prime = v_minus + v_minus x t
//!      v_plus  = v_minus + v_prime x s
//!      v_new   = v_plus + (q/m)*E*(dt/2)
//!      x_new   = x + v_new*dt  (周期包裹)
//!
//! 等离子体频率: omega_p = sqrt(n * q^2 / (eps0 * m))
//! 等离子体振荡周期: T_p = 2*pi / omega_p
//! Boris 稳定性: omega_c * dt < 2  (回旋频率)
//!
//! 应用: 等离子体振荡 (Langmuir), 朗道阻尼, 双流不稳定性, 磁约束.
//!
//! 基于 Boris 1970, Birdsall & Langdon 1991 (PIC).

use serde::{Deserialize, Serialize};

/// 真空介电常数 (约化单位 eps0 = 1)
pub const EPS0: f32 = 1.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PoissonMethod {
    /// radix-2 FFT (要求 nx, ny 是 2 的幂)
    Fft,
    /// Jacobi 迭代 (周期边界)
    Jacobi { max_iter: usize, tol: f32 },
}

impl Default for PoissonMethod {
    fn default() -> Self {
        PoissonMethod::Fft
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PicConfig {
    pub nx: usize,
    pub ny: usize,
    pub dx: f32,
    pub dt: f32,
    pub n_particles: usize,
    pub charge: f32,
    pub mass: f32,
    /// 外部均匀磁场 (Boris 旋转用, 可为 0)
    pub b_field: [f32; 3],
    pub method: PoissonMethod,
}

impl Default for PicConfig {
    fn default() -> Self {
        PicConfig {
            nx: 32,
            ny: 32,
            dx: 1.0,
            dt: 0.1,
            n_particles: 1024,
            charge: 1.0,
            mass: 1.0,
            b_field: [0.0; 3],
            method: PoissonMethod::Fft,
        }
    }
}

impl PicConfig {
    pub fn n_cells(&self) -> usize {
        self.nx * self.ny
    }
    pub fn area(&self) -> f32 {
        (self.nx as f32) * self.dx * (self.ny as f32) * self.dx
    }
    pub fn density(&self) -> f32 {
        self.n_particles as f32 / self.area()
    }
    /// 等离子体频率 omega_p = sqrt(n * q^2 / (eps0 * m))
    pub fn plasma_frequency(&self) -> f32 {
        (self.density() * self.charge * self.charge / (EPS0 * self.mass)).sqrt()
    }
    pub fn plasma_period(&self) -> f32 {
        2.0 * std::f32::consts::PI / self.plasma_frequency()
    }
    /// Boris 算法稳定性: omega_c * dt < 2 (回旋频率)
    pub fn is_stable(&self) -> bool {
        let b_mag = (self.b_field[0].powi(2) + self.b_field[1].powi(2) + self.b_field[2].powi(2)).sqrt();
        let omega_c = self.charge * b_mag / self.mass;
        omega_c * self.dt < 2.0
    }
}

/// 简单复数 (避免引入 num-complex 依赖)
#[derive(Debug, Clone, Copy)]
pub struct Complex {
    pub re: f32,
    pub im: f32,
}

impl Complex {
    pub fn new(re: f32, im: f32) -> Self { Complex { re, im } }
    pub fn zero() -> Self { Complex { re: 0.0, im: 0.0 } }
    pub fn scale(self, s: f32) -> Self { Complex { re: self.re * s, im: self.im * s } }
}

impl std::ops::Add for Complex {
    type Output = Complex;
    fn add(self, other: Complex) -> Complex {
        Complex { re: self.re + other.re, im: self.im + other.im }
    }
}

impl std::ops::Sub for Complex {
    type Output = Complex;
    fn sub(self, other: Complex) -> Complex {
        Complex { re: self.re - other.re, im: self.im - other.im }
    }
}

impl std::ops::Mul for Complex {
    type Output = Complex;
    fn mul(self, other: Complex) -> Complex {
        Complex {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }
}

pub struct PicSolver {
    pub config: PicConfig,
    pub positions: Vec<[f32; 2]>,
    pub velocities: Vec<[f32; 2]>,
    pub rho: Vec<f32>,
    pub phi: Vec<f32>,
    pub ex: Vec<f32>,
    pub ey: Vec<f32>,
    pub time: f32,
    pub steps: usize,
    rng_state: u64,
}

impl PicSolver {
    pub fn new(config: PicConfig) -> Self {
        let n = config.n_particles;
        let nc = config.n_cells();
        PicSolver {
            config,
            positions: vec![[0.0; 2]; n],
            velocities: vec![[0.0; 2]; n],
            rho: vec![0.0; nc],
            phi: vec![0.0; nc],
            ex: vec![0.0; nc],
            ey: vec![0.0; nc],
            time: 0.0,
            steps: 0,
            rng_state: 0x1234_5678_9ABC_DEF0,
        }
    }

    fn rand(&mut self) -> f32 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        (x >> 11) as f32 / (1u64 << 53) as f32
    }

    fn rand_normal(&mut self) -> f32 {
        let u1 = self.rand().max(1e-10);
        let u2 = self.rand();
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f32::consts::PI * u2;
        r * theta.cos()
    }

    pub fn idx(&self, i: usize, j: usize) -> usize {
        i + self.config.nx * j
    }

    fn wrap(i: i32, n: usize) -> usize {
        let m = n as i32;
        (((i % m) + m) % m) as usize
    }

    /// 随机初始化粒子位置 (均匀分布)
    pub fn initialize_uniform(&mut self, seed: u64) {
        self.rng_state = if seed == 0 { 0x1234_5678_9ABC_DEF0 } else { seed };
        let lx = (self.config.nx as f32) * self.config.dx;
        let ly = (self.config.ny as f32) * self.config.dx;
        for i in 0..self.config.n_particles {
            self.positions[i][0] = self.rand() * lx;
            self.positions[i][1] = self.rand() * ly;
        }
    }

    /// 初始化速度 (Maxwellian, 去质心)
    pub fn initialize_velocities(&mut self, temperature: f32) {
        let n = self.config.n_particles;
        for i in 0..n {
            self.velocities[i][0] = self.rand_normal() * temperature.sqrt();
            self.velocities[i][1] = self.rand_normal() * temperature.sqrt();
        }
        let mut vcm = [0.0f32; 2];
        for i in 0..n {
            vcm[0] += self.velocities[i][0];
            vcm[1] += self.velocities[i][1];
        }
        vcm[0] /= n as f32;
        vcm[1] /= n as f32;
        for i in 0..n {
            self.velocities[i][0] -= vcm[0];
            self.velocities[i][1] -= vcm[1];
        }
    }

    /// 双线性电荷沉积到网格
    pub fn deposit_charge(&mut self) {
        for r in self.rho.iter_mut() { *r = 0.0; }
        let q = self.config.charge;
        let dx = self.config.dx;
        let nx = self.config.nx;
        let ny = self.config.ny;
        let lx = (nx as f32) * dx;
        let ly = (ny as f32) * dx;
        for p in 0..self.config.n_particles {
            let xp = ((self.positions[p][0] % lx) + lx) % lx;
            let yp = ((self.positions[p][1] % ly) + ly) % ly;
            let fi = xp / dx;
            let fj = yp / dx;
            let i = fi.floor() as i32 % nx as i32;
            let j = fj.floor() as i32 % ny as i32;
            let wx = fi - fi.floor();
            let wy = fj - fj.floor();
            let i0 = Self::wrap(i, nx);
            let i1 = Self::wrap(i + 1, nx);
            let j0 = Self::wrap(j, ny);
            let j1 = Self::wrap(j + 1, ny);
            self.rho[i0 + nx * j0] += q * (1.0 - wx) * (1.0 - wy);
            self.rho[i1 + nx * j0] += q * wx * (1.0 - wy);
            self.rho[i0 + nx * j1] += q * (1.0 - wx) * wy;
            self.rho[i1 + nx * j1] += q * wx * wy;
        }
        let cell_area = dx * dx;
        for r in self.rho.iter_mut() { *r /= cell_area; }
    }

    /// FFT 求解泊松方程 nabla^2 phi = -rho/eps0 (周期边界)
    /// 要求 nx, ny 是 2 的幂
    pub fn solve_poisson_fft(&mut self) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        assert!(nx.is_power_of_two() && ny.is_power_of_two(),
            "FFT requires nx, ny to be powers of 2");
        let dx = self.config.dx;

        // 1. 沿 x 方向 FFT 每一行
        let mut work = vec![Complex::zero(); nx];
        let mut rho_k = vec![Complex::zero(); nx * ny];
        for j in 0..ny {
            for i in 0..nx {
                work[i] = Complex::new(self.rho[self.idx(i, j)] / EPS0, 0.0);
            }
            Self::fft_in_place(&mut work, false);
            for i in 0..nx {
                rho_k[self.idx(i, j)] = work[i];
            }
        }
        // 2. 沿 y 方向 FFT 每一列
        let mut work_y = vec![Complex::zero(); ny];
        for i in 0..nx {
            for j in 0..ny {
                work_y[j] = rho_k[self.idx(i, j)];
            }
            Self::fft_in_place(&mut work_y, false);
            for j in 0..ny {
                rho_k[self.idx(i, j)] = work_y[j];
            }
        }

        // 3. 频域求解 phi_k = rho_k / k^2 (k=0 处 phi=0, 零平均)
        let lx = (nx as f32) * dx;
        let ly = (ny as f32) * dx;
        let two_pi = 2.0 * std::f32::consts::PI;
        let mut phi_k = vec![Complex::zero(); nx * ny];
        for j in 0..ny {
            let ky = two_pi * (j as f32) / ly;
            for i in 0..nx {
                let kx = two_pi * (i as f32) / lx;
                let k2 = kx * kx + ky * ky;
                let idx = self.idx(i, j);
                if k2 > 1e-12 {
                    // nabla^2 phi = -rho/eps0  ->  -k^2 phi_k = -rho_k  ->  phi_k = rho_k / k^2
                    phi_k[idx] = rho_k[idx].scale(1.0 / k2);
                } else {
                    phi_k[idx] = Complex::zero();
                }
            }
        }

        // 4. IFFT 沿 y 方向
        for i in 0..nx {
            for j in 0..ny {
                work_y[j] = phi_k[self.idx(i, j)];
            }
            Self::fft_in_place(&mut work_y, true);
            for j in 0..ny {
                phi_k[self.idx(i, j)] = work_y[j];
            }
        }
        // 5. IFFT 沿 x 方向
        for j in 0..ny {
            for i in 0..nx {
                work[i] = phi_k[self.idx(i, j)];
            }
            Self::fft_in_place(&mut work, true);
            for i in 0..nx {
                self.phi[i + nx * j] = work[i].re;
            }
        }
    }

    /// Radix-2 迭代 FFT (Cooley-Tukey)
    /// inverse=true 时做 IFFT (含 1/N 归一化)
    fn fft_in_place(data: &mut [Complex], inverse: bool) {
        let n = data.len();
        assert!(n.is_power_of_two(), "FFT length must be power of 2");
        if n == 1 { return; }
        // 位反转重排
        let mut j = 0usize;
        for i in 1..n {
            let mut bit = n >> 1;
            while (j & bit) != 0 {
                j ^= bit;
                bit >>= 1;
            }
            j ^= bit;
            if i < j {
                data.swap(i, j);
            }
        }
        // 蝶形运算
        let two_pi = 2.0 * std::f32::consts::PI;
        let sign = if inverse { 1.0 } else { -1.0 };
        let mut len = 2;
        while len <= n {
            let ang = sign * two_pi / (len as f32);
            let wlen = Complex::new(ang.cos(), ang.sin());
            for i in (0..n).step_by(len) {
                let mut w = Complex::new(1.0, 0.0);
                for k in 0..(len / 2) {
                    let u = data[i + k];
                    let v = data[i + k + len / 2] * w;
                    data[i + k] = u + v;
                    data[i + k + len / 2] = u - v;
                    w = w * wlen;
                }
            }
            len <<= 1;
        }
        if inverse {
            let inv_n = 1.0 / (n as f32);
            for c in data.iter_mut() {
                *c = c.scale(inv_n);
            }
        }
    }

    /// Jacobi 迭代求解泊松方程 (周期边界)
    pub fn solve_poisson_jacobi(&mut self, max_iter: usize, tol: f32) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let dx2 = self.config.dx * self.config.dx;
        let rhs_factor = 1.0 / EPS0;
        for r in self.phi.iter_mut() { *r = 0.0; }
        let mut new = self.phi.clone();
        for _iter in 0..max_iter {
            let mut max_diff = 0.0f32;
            for j in 0..ny {
                for i in 0..nx {
                    let ip = Self::wrap((i + 1) as i32, nx);
                    let im = Self::wrap((i as i32) - 1, nx);
                    let jp = Self::wrap((j + 1) as i32, ny);
                    let jm = Self::wrap((j as i32) - 1, ny);
                    let rhs = -self.rho[self.idx(i, j)] * rhs_factor * dx2;
                    let val = 0.25 * (self.phi[self.idx(ip, j)]
                        + self.phi[self.idx(im, j)]
                        + self.phi[self.idx(i, jp)]
                        + self.phi[self.idx(i, jm)]
                        + rhs);
                    let diff = (val - self.phi[self.idx(i, j)]).abs();
                    if diff > max_diff { max_diff = diff; }
                    new[self.idx(i, j)] = val;
                }
            }
            std::mem::swap(&mut self.phi, &mut new);
            if max_diff < tol { break; }
        }
    }

    /// 计算电场 E = -grad(phi), 中心差分, 周期边界
    pub fn compute_e_field(&mut self) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let inv_2dx = 1.0 / (2.0 * self.config.dx);
        for j in 0..ny {
            for i in 0..nx {
                let ip = Self::wrap((i + 1) as i32, nx);
                let im = Self::wrap((i as i32) - 1, nx);
                let jp = Self::wrap((j + 1) as i32, ny);
                let jm = Self::wrap((j as i32) - 1, ny);
                let p_ip = self.phi[ip + nx * j];
                let p_im = self.phi[im + nx * j];
                let p_jp = self.phi[i + nx * jp];
                let p_jm = self.phi[i + nx * jm];
                self.ex[i + nx * j] = -(p_ip - p_im) * inv_2dx;
                self.ey[i + nx * j] = -(p_jp - p_jm) * inv_2dx;
            }
        }
    }

    /// 双线性插值电场到粒子位置
    pub fn gather_e(&self, xp: f32, yp: f32) -> [f32; 2] {
        let dx = self.config.dx;
        let nx = self.config.nx;
        let ny = self.config.ny;
        let lx = (nx as f32) * dx;
        let ly = (ny as f32) * dx;
        let xp = ((xp % lx) + lx) % lx;
        let yp = ((yp % ly) + ly) % ly;
        let fi = xp / dx;
        let fj = yp / dx;
        let i = fi.floor() as i32 % nx as i32;
        let j = fj.floor() as i32 % ny as i32;
        let wx = fi - fi.floor();
        let wy = fj - fj.floor();
        let i0 = Self::wrap(i, nx);
        let i1 = Self::wrap(i + 1, nx);
        let j0 = Self::wrap(j, ny);
        let j1 = Self::wrap(j + 1, ny);
        let ex = self.ex[self.idx(i0, j0)] * (1.0 - wx) * (1.0 - wy)
            + self.ex[self.idx(i1, j0)] * wx * (1.0 - wy)
            + self.ex[self.idx(i0, j1)] * (1.0 - wx) * wy
            + self.ex[self.idx(i1, j1)] * wx * wy;
        let ey = self.ey[self.idx(i0, j0)] * (1.0 - wx) * (1.0 - wy)
            + self.ey[self.idx(i1, j0)] * wx * (1.0 - wy)
            + self.ey[self.idx(i0, j1)] * (1.0 - wx) * wy
            + self.ey[self.idx(i1, j1)] * wx * wy;
        [ex, ey]
    }

    /// Boris pusher 一步
    pub fn boris_push(&mut self) {
        let dt = self.config.dt;
        let qm = self.config.charge / self.config.mass;
        let half_dt = 0.5 * dt;
        let b = self.config.b_field;
        let t = [qm * b[0] * half_dt, qm * b[1] * half_dt, qm * b[2] * half_dt];
        let t2 = t[0] * t[0] + t[1] * t[1] + t[2] * t[2];
        let s = [2.0 * t[0] / (1.0 + t2), 2.0 * t[1] / (1.0 + t2), 2.0 * t[2] / (1.0 + t2)];
        let lx = (self.config.nx as f32) * self.config.dx;
        let ly = (self.config.ny as f32) * self.config.dx;

        for p in 0..self.config.n_particles {
            let e = self.gather_e(self.positions[p][0], self.positions[p][1]);
            // 2D: v = (vx, vy, 0)
            // 半步 E 加速
            let vm_x = self.velocities[p][0] + qm * e[0] * half_dt;
            let vm_y = self.velocities[p][1] + qm * e[1] * half_dt;
            let vm_z = 0.0;
            // 磁场旋转: v' = v_minus + v_minus x t
            let vp_x = vm_x + (vm_y * t[2] - vm_z * t[1]);
            let vp_y = vm_y + (vm_z * t[0] - vm_x * t[2]);
            let vp_z = vm_z + (vm_x * t[1] - vm_y * t[0]);
            // v_plus = v_minus + v' x s
            let vpl_x = vm_x + (vp_y * s[2] - vp_z * s[1]);
            let vpl_y = vm_y + (vp_z * s[0] - vp_x * s[2]);
            let _ = vp_z; // 2D 不用 vz
            // 半步 E 加速
            self.velocities[p][0] = vpl_x + qm * e[0] * half_dt;
            self.velocities[p][1] = vpl_y + qm * e[1] * half_dt;
            // 位置更新 + 周期包裹
            self.positions[p][0] = ((self.positions[p][0] + self.velocities[p][0] * dt) % lx + lx) % lx;
            self.positions[p][1] = ((self.positions[p][1] + self.velocities[p][1] * dt) % ly + ly) % ly;
        }
    }

    /// 一步: 沉积电荷 -> 解泊松 -> 算 E -> Boris push
    pub fn step(&mut self) {
        self.deposit_charge();
        match self.config.method {
            PoissonMethod::Fft => self.solve_poisson_fft(),
            PoissonMethod::Jacobi { max_iter, tol } => self.solve_poisson_jacobi(max_iter, tol),
        }
        self.compute_e_field();
        self.boris_push();
        self.time += self.config.dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n { self.step(); }
    }

    /// 动能 (1/2 m v^2 之和)
    pub fn kinetic_energy(&self) -> f32 {
        let mut ke = 0.0;
        let m = self.config.mass;
        for v in self.velocities.iter() {
            ke += 0.5 * m * (v[0] * v[0] + v[1] * v[1]);
        }
        ke
    }

    /// 电场能量 (1/2 eps0 * |E|^2 * cell_area 之和)
    pub fn field_energy(&self) -> f32 {
        let cell_area = self.config.dx * self.config.dx;
        let mut fe = 0.0;
        for i in 0..self.config.n_cells() {
            fe += 0.5 * EPS0 * (self.ex[i] * self.ex[i] + self.ey[i] * self.ey[i]) * cell_area;
        }
        fe
    }

    pub fn total_energy(&self) -> f32 {
        self.kinetic_energy() + self.field_energy()
    }

    pub fn reset(&mut self) {
        for p in self.positions.iter_mut() { *p = [0.0; 2]; }
        for v in self.velocities.iter_mut() { *v = [0.0; 2]; }
        for r in self.rho.iter_mut() { *r = 0.0; }
        for p in self.phi.iter_mut() { *p = 0.0; }
        for e in self.ex.iter_mut() { *e = 0.0; }
        for e in self.ey.iter_mut() { *e = 0.0; }
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
    fn test_eps0() {
        assert_eq!(EPS0, 1.0);
    }

    #[test]
    fn test_config_default() {
        let c = PicConfig::default();
        assert_eq!(c.nx, 32);
        assert_eq!(c.ny, 32);
        assert_eq!(c.n_particles, 1024);
        assert_eq!(c.charge, 1.0);
        assert_eq!(c.mass, 1.0);
        assert_eq!(c.b_field, [0.0; 3]);
    }

    #[test]
    fn test_n_cells() {
        let c = PicConfig { nx: 16, ny: 8, ..Default::default() };
        assert_eq!(c.n_cells(), 128);
    }

    #[test]
    fn test_density() {
        let c = PicConfig { nx: 32, ny: 32, dx: 1.0, n_particles: 1024, ..Default::default() };
        assert!(approx_eq(c.density(), 1.0, 1e-6));
    }

    #[test]
    fn test_plasma_frequency() {
        let c = PicConfig::default();
        // omega_p = sqrt(n*q^2/(eps0*m)) = sqrt(1*1/(1*1)) = 1
        assert!(approx_eq(c.plasma_frequency(), 1.0, 1e-6));
    }

    #[test]
    fn test_plasma_period() {
        let c = PicConfig::default();
        assert!(approx_eq(c.plasma_period(), 2.0 * std::f32::consts::PI, 1e-5));
    }

    #[test]
    fn test_stable_no_b() {
        let c = PicConfig::default();
        assert!(c.is_stable());
    }

    #[test]
    fn test_stable_with_b() {
        let c = PicConfig { b_field: [0.0, 0.0, 1.0], dt: 1.0, ..Default::default() };
        assert!(c.is_stable());
    }

    #[test]
    fn test_unstable_with_b() {
        let c = PicConfig { b_field: [0.0, 0.0, 3.0], dt: 1.0, ..Default::default() };
        assert!(!c.is_stable());
    }

    #[test]
    fn test_solver_new() {
        let s = PicSolver::new(PicConfig::default());
        assert_eq!(s.positions.len(), 1024);
        assert_eq!(s.velocities.len(), 1024);
        assert_eq!(s.rho.len(), 32 * 32);
        assert_eq!(s.ex.len(), 32 * 32);
        assert_eq!(s.steps, 0);
    }

    #[test]
    fn test_initialize_uniform() {
        let mut s = PicSolver::new(PicConfig::default());
        s.initialize_uniform(42);
        let lx = 32.0;
        let ly = 32.0;
        for p in s.positions.iter() {
            assert!(p[0] >= 0.0 && p[0] < lx);
            assert!(p[1] >= 0.0 && p[1] < ly);
        }
    }

    #[test]
    fn test_deposit_charge_total() {
        let mut s = PicSolver::new(PicConfig { n_particles: 100, ..Default::default() });
        s.initialize_uniform(42);
        s.deposit_charge();
        // rho 是密度, 积分 = sum(rho * cell_area) = N * q
        let total: f32 = s.rho.iter().sum();
        let cell_area = 1.0;
        let integrated = total * cell_area;
        assert!(approx_eq(integrated, 100.0, 1e-3),
            "expected 100, got {}", integrated);
    }

    #[test]
    fn test_fft_simple() {
        // cos(2*pi*k*n/N) 的 FFT 应在 bin k 和 bin N-k 有峰值
        let n = 8;
        let k = 1;
        let mut data: Vec<Complex> = (0..n).map(|i| {
            let ang = 2.0 * std::f32::consts::PI * (k as f32) * (i as f32) / (n as f32);
            Complex::new(ang.cos(), 0.0)
        }).collect();
        PicSolver::fft_in_place(&mut data, false);
        // bin k 应该有非零实部 (= N/2 = 4)
        assert!(data[k].re.abs() > (n as f32) * 0.4, "bin {} re = {}", k, data[k].re);
        // 其他 bin 应接近 0
        for i in 0..n {
            if i != k && i != n - k {
                assert!(data[i].re.abs() < 1e-3, "bin {} re = {}", i, data[i].re);
                assert!(data[i].im.abs() < 1e-3, "bin {} im = {}", i, data[i].im);
            }
        }
    }

    #[test]
    fn test_fft_ifft_roundtrip() {
        let n = 16;
        let original: Vec<Complex> = (0..n).map(|i| {
            Complex::new((i as f32) * 0.1, (i as f32) * 0.05 - 0.3)
        }).collect();
        let mut data = original.clone();
        PicSolver::fft_in_place(&mut data, false);
        PicSolver::fft_in_place(&mut data, true);
        for i in 0..n {
            assert!(approx_eq(data[i].re, original[i].re, 1e-4));
            assert!(approx_eq(data[i].im, original[i].im, 1e-4));
        }
    }

    #[test]
    fn test_poisson_fft_constant_rho() {
        // 常数电荷密度 → phi = 0 (零模去除)
        let mut s = PicSolver::new(PicConfig::default());
        for r in s.rho.iter_mut() { *r = 1.0; }
        s.solve_poisson_fft();
        for p in s.phi.iter() {
            assert!(p.abs() < 1e-3, "phi = {}", p);
        }
    }

    #[test]
    fn test_poisson_fft_dipole() {
        // 左半 +1, 右半 -1, sum rho = 0, 解存在
        let mut s = PicSolver::new(PicConfig { nx: 16, ny: 16, ..Default::default() });
        let nx = s.config.nx;
        for j in 0..16 {
            for i in 0..16 {
                s.rho[i + nx * j] = if i < 8 { 1.0 } else { -1.0 };
            }
        }
        s.solve_poisson_fft();
        // sum phi 应接近 0 (零模)
        let sum_phi: f32 = s.phi.iter().sum();
        assert!(sum_phi.abs() < 1e-3, "sum phi = {}", sum_phi);
        // phi 应有非零幅度
        let max_phi = s.phi.iter().map(|p| p.abs()).fold(0.0f32, f32::max);
        assert!(max_phi > 0.1, "max phi = {}", max_phi);
    }

    #[test]
    fn test_compute_e_field() {
        let mut s = PicSolver::new(PicConfig { nx: 16, ny: 16, ..Default::default() });
        let lx = 16.0;
        let dx = 1.0;
        let nx = s.config.nx;
        for j in 0..16 {
            for i in 0..16 {
                let x = (i as f32) * dx;
                s.phi[i + nx * j] = (2.0 * std::f32::consts::PI * x / lx).sin();
            }
        }
        s.compute_e_field();
        // Ex 在 i=0 处 = -(phi[1] - phi[15])/(2*dx) (中心差分, 周期)
        let expected = -(s.phi[s.idx(1, 0)] - s.phi[s.idx(15, 0)]) / 2.0;
        assert!(approx_eq(s.ex[s.idx(0, 0)], expected, 1e-5));
    }

    #[test]
    fn test_gather_e_zero_field() {
        let s = PicSolver::new(PicConfig::default());
        let e = s.gather_e(5.5, 7.3);
        assert_eq!(e[0], 0.0);
        assert_eq!(e[1], 0.0);
    }

    #[test]
    fn test_gather_e_interpolation() {
        let mut s = PicSolver::new(PicConfig { nx: 4, ny: 4, ..Default::default() });
        let nx = s.config.nx;
        s.ex[1 + nx] = 1.0;
        // 粒子在 (1.5, 1.5) — 4 个角各贡献 0.25
        let e = s.gather_e(1.5, 1.5);
        assert!(approx_eq(e[0], 0.25, 1e-5));
    }

    #[test]
    fn test_boris_push_no_field() {
        // 无 E, 无 B → 匀速直线
        let mut s = PicSolver::new(PicConfig {
            nx: 32, ny: 32, dx: 1.0, dt: 0.5, n_particles: 1, ..Default::default()
        });
        s.positions[0] = [5.0, 5.0];
        s.velocities[0] = [2.0, 1.0];
        s.boris_push();
        assert!(approx_eq(s.positions[0][0], 6.0, 1e-5));
        assert!(approx_eq(s.positions[0][1], 5.5, 1e-5));
        assert!(approx_eq(s.velocities[0][0], 2.0, 1e-5));
        assert!(approx_eq(s.velocities[0][1], 1.0, 1e-5));
    }

    #[test]
    fn test_boris_push_periodic_wrap() {
        let mut s = PicSolver::new(PicConfig {
            nx: 8, ny: 8, dx: 1.0, dt: 1.0, n_particles: 1, ..Default::default()
        });
        s.positions[0] = [7.5, 7.5];
        s.velocities[0] = [1.0, 1.0];
        s.boris_push();
        // 7.5 + 1 = 8.5, wrap to 0.5
        assert!(approx_eq(s.positions[0][0], 0.5, 1e-5));
        assert!(approx_eq(s.positions[0][1], 0.5, 1e-5));
    }

    #[test]
    fn test_boris_push_b_field_rotation() {
        // 纯 B 场 (z 方向), 粒子做圆周运动
        // omega_c = qB/m = 1, 周期 T = 2*pi
        let dt = std::f32::consts::PI / 50.0; // 100 步一圈
        let mut s = PicSolver::new(PicConfig {
            nx: 64, ny: 64, dx: 1.0, dt, n_particles: 1,
            b_field: [0.0, 0.0, 1.0], ..Default::default()
        });
        s.positions[0] = [32.0, 32.0];
        s.velocities[0] = [1.0, 0.0];
        for _ in 0..100 {
            s.boris_push();
        }
        // 粒子应回到接近起点
        let dx = (s.positions[0][0] - 32.0).abs();
        let dy = (s.positions[0][1] - 32.0).abs();
        assert!(dx < 0.1, "dx = {}", dx);
        assert!(dy < 0.1, "dy = {}", dy);
        // 速度大小不变 (磁力不做功)
        let v2 = s.velocities[0][0].powi(2) + s.velocities[0][1].powi(2);
        assert!(approx_eq(v2, 1.0, 1e-3));
    }

    #[test]
    fn test_kinetic_energy() {
        let mut s = PicSolver::new(PicConfig { n_particles: 2, mass: 2.0, ..Default::default() });
        s.velocities[0] = [3.0, 0.0];
        s.velocities[1] = [0.0, 4.0];
        // KE = 1/2 * 2 * (9 + 16) = 25
        assert!(approx_eq(s.kinetic_energy(), 25.0, 1e-5));
    }

    #[test]
    fn test_field_energy() {
        let mut s = PicSolver::new(PicConfig { nx: 2, ny: 2, dx: 1.0, ..Default::default() });
        s.ex[0] = 1.0;
        // FE = 0.5 * 1 * 1 * 1 = 0.5 (一个单元)
        assert!(approx_eq(s.field_energy(), 0.5, 1e-5));
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = PicSolver::new(PicConfig { n_particles: 4, ..Default::default() });
        s.initialize_uniform(42);
        s.initialize_velocities(0.01);
        let t0 = s.time;
        s.step();
        assert!(s.time > t0);
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_step_n() {
        let mut s = PicSolver::new(PicConfig { n_particles: 4, ..Default::default() });
        s.initialize_uniform(42);
        s.initialize_velocities(0.01);
        s.step_n(5);
        assert_eq!(s.steps, 5);
    }

    #[test]
    fn test_energy_conservation_no_b() {
        // 无 B 场, 等离子体应近似能量守恒
        let mut s = PicSolver::new(PicConfig {
            nx: 16, ny: 16, dx: 1.0, dt: 0.05, n_particles: 64,
            ..Default::default()
        });
        s.initialize_uniform(42);
        s.initialize_velocities(0.5);
        s.step();
        let e0 = s.total_energy();
        s.step_n(20);
        let e1 = s.total_energy();
        let drift = ((e1 - e0) / e0).abs();
        assert!(drift < 0.20, "energy drift = {} (e0={}, e1={})", drift, e0, e1);
    }

    #[test]
    fn test_plasma_oscillation_frequency() {
        // 均匀等离子体 + 小扰动 → 以 omega_p 振荡
        let mut s = PicSolver::new(PicConfig {
            nx: 16, ny: 4, dx: 1.0, dt: 0.1, n_particles: 128,
            charge: 1.0, mass: 1.0, ..Default::default()
        });
        s.initialize_uniform(42);
        for i in 0..s.config.n_particles {
            s.positions[i][0] += 0.1 * (2.0 * std::f32::consts::PI * s.positions[i][0] / 16.0).sin();
        }
        s.initialize_velocities(0.0);
        let omega_p = s.config.plasma_frequency();
        assert!(omega_p > 0.5);
        let mut energies = Vec::new();
        for _ in 0..30 {
            s.step();
            energies.push(s.field_energy());
        }
        let max_e = energies.iter().fold(0.0f32, |a, &b| a.max(b));
        let min_e = energies.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        // 场能应该有振荡
        assert!(max_e > min_e * 1.05, "no oscillation: max={}, min={}", max_e, min_e);
    }

    #[test]
    fn test_poisson_jacobi_converges() {
        let mut s = PicSolver::new(PicConfig {
            nx: 16, ny: 16, dx: 1.0,
            method: PoissonMethod::Jacobi { max_iter: 2000, tol: 1e-6 },
            ..Default::default()
        });
        let nx = s.config.nx;
        for j in 0..16 {
            for i in 0..16 {
                s.rho[i + nx * j] = if i < 8 { 1.0 } else { -1.0 };
            }
        }
        s.solve_poisson_jacobi(2000, 1e-6);
        let max_phi = s.phi.iter().map(|p| p.abs()).fold(0.0f32, f32::max);
        assert!(max_phi > 0.5, "max phi = {}", max_phi);
        // 反对称: rho(x) = -rho(L-x) → phi(x) = -phi(L-x)
        let mid_left = s.phi[s.idx(4, 8)];
        let mid_right = s.phi[s.idx(12, 8)];
        assert!((mid_left + mid_right).abs() < 0.1,
            "asymmetry: {} + {} = {}", mid_left, mid_right, mid_left + mid_right);
    }

    #[test]
    fn test_reset() {
        let mut s = PicSolver::new(PicConfig::default());
        s.initialize_uniform(42);
        s.initialize_velocities(1.0);
        s.step();
        assert!(s.steps > 0);
        s.reset();
        assert_eq!(s.steps, 0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.positions[0], [0.0, 0.0]);
    }

    #[test]
    fn test_complex_arithmetic() {
        let a = Complex::new(1.0, 2.0);
        let b = Complex::new(3.0, -1.0);
        let sum = a + b;
        assert!(approx_eq(sum.re, 4.0, 1e-6));
        assert!(approx_eq(sum.im, 1.0, 1e-6));
        let prod = a * b;
        // (1+2i)(3-i) = 3 - i + 6i - 2i^2 = 5 + 5i
        assert!(approx_eq(prod.re, 5.0, 1e-6));
        assert!(approx_eq(prod.im, 5.0, 1e-6));
    }
}
