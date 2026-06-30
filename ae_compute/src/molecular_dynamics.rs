//! Molecular Dynamics (Lennard-Jones, Velocity Verlet)
//!
//! 微观尺度物理模拟. 基于牛顿运动方程数值积分粒子轨迹.
//!
//! 势能: Lennard-Jones
//!   U(r) = 4e[(s/r)^12 - (s/r)^6]
//!   力 F(r) = 24e/r * [2(s/r)^12 - (s/r)^6]
//!   截断半径 r_c (通常 2.5s), 平移使 U(r_c) = 0
//!
//! 积分: Velocity Verlet (二阶辛积分, 能量长期稳定)
//!   v(t+dt/2) = v(t) + a(t)*dt/2
//!   x(t+dt)   = x(t) + v(t+dt/2)*dt
//!   a(t+dt)   = F(x(t+dt))/m
//!   v(t+dt)   = v(t+dt/2) + a(t+dt)*dt/2
//!
//! 边界: 周期性 (最小镜像约定)
//!   dr_ij = r_j - r_i - L*round((r_j - r_i)/L)
//!
//! 邻居: Verlet list (skin 距离)
//!   r_list = r_cutoff + r_skin
//!   重建条件: 最大位移 > r_skin / 2
//!
//! 系综:
//!   NVE — 微正则 (能量守恒)
//!   NVT Berendsen — v *= sqrt(1 + dt/tau*(T0/T - 1))
//!   NVT Langevin — 随机力 + 阻尼
//!   NPT Berendsen — 温度 + 压力耦合 (坐标与盒子缩放)
//!
//! 观测量:
//!   动能 KE, 势能 PE, 总能量 E
//!   温度 T = 2*KE / (d*N*k_B)
//!   压力 P = (N*k_B*T + virial/d) / V
//!   径向分布 g(r), 均方位移 MSD, 速度自相关 C_v(t)
//!
//! 基于 Verlet 1967, Allen & Tildesley 2017, Frenkel & Smit 2001.

use serde::{Deserialize, Serialize};

/// 玻尔兹曼常数 (约化单位 k_B = 1)
pub const K_B: f32 = 1.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Ensemble {
    /// 微正则系综 (能量守恒)
    NVE,
    /// 正则系综 (Berendsen 恒温器)
    NVTBerendsen { temperature: f32, tau: f32 },
    /// 正则系综 (Langevin 随机动力学)
    NVTLangevin { temperature: f32, damping: f32, seed: u64 },
    /// 等温等压系综 (Berendsen 恒温+恒压)
    NPTBerendsen {
        temperature: f32,
        pressure: f32,
        tau_t: f32,
        tau_p: f32,
        compressibility: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdConfig {
    pub n_particles: usize,
    /// 维度 (2 或 3)
    pub dims: u8,
    /// 模拟盒子尺寸 [lx, ly, lz]
    pub box_size: [f32; 3],
    /// 时间步长
    pub dt: f32,
    /// LJ epsilon
    pub epsilon: f32,
    /// LJ sigma
    pub sigma: f32,
    /// 截断半径 (通常 2.5*sigma)
    pub cutoff: f32,
    /// Verlet 表 skin 距离
    pub skin: f32,
    /// 粒子质量
    pub mass: f32,
    pub ensemble: Ensemble,
}

impl Default for MdConfig {
    fn default() -> Self {
        MdConfig {
            n_particles: 64,
            dims: 3,
            box_size: [4.0, 4.0, 4.0],
            dt: 0.005,
            epsilon: 1.0,
            sigma: 1.0,
            cutoff: 2.5,
            skin: 0.3,
            mass: 1.0,
            ensemble: Ensemble::NVE,
        }
    }
}

impl MdConfig {
    pub fn volume(&self) -> f32 {
        match self.dims {
            2 => self.box_size[0] * self.box_size[1],
            _ => self.box_size[0] * self.box_size[1] * self.box_size[2],
        }
    }
    pub fn density(&self) -> f32 {
        self.n_particles as f32 / self.volume()
    }
    /// LJ 势能 (截断 + 平移使 U(r_c) = 0)
    pub fn lj_potential(&self, r: f32) -> f32 {
        if r >= self.cutoff || r < 1e-10 {
            return 0.0;
        }
        let s = self.sigma / r;
        let s6 = s.powi(6);
        let s12 = s6 * s6;
        let u = 4.0 * self.epsilon * (s12 - s6);
        let sc = self.sigma / self.cutoff;
        let sc6 = sc.powi(6);
        let sc12 = sc6 * sc6;
        let uc = 4.0 * self.epsilon * (sc12 - sc6);
        u - uc
    }
    /// LJ 力大小 (正 = 排斥). 平移不影响.
    pub fn lj_force_magnitude(&self, r: f32) -> f32 {
        if r >= self.cutoff || r < 1e-10 {
            return 0.0;
        }
        let s = self.sigma / r;
        let s6 = s.powi(6);
        let s12 = s6 * s6;
        24.0 * self.epsilon / r * (2.0 * s12 - s6)
    }
}

pub struct MdSolver {
    pub config: MdConfig,
    pub positions: Vec<[f32; 3]>,
    pub velocities: Vec<[f32; 3]>,
    pub forces: Vec<[f32; 3]>,
    pub time: f32,
    pub steps: usize,
    /// Verlet 邻居表 (扁平)
    neighbors: Vec<usize>,
    neighbor_offsets: Vec<usize>,
    rebuild_positions: Vec<[f32; 3]>,
    last_rebuild_step: usize,
    /// 初始位置 (用于 MSD)
    initial_positions: Vec<[f32; 3]>,
    /// RNG 状态 (xorshift64)
    rng_state: u64,
    pub kinetic_energy: f32,
    pub potential_energy: f32,
    pub virial: f32,
}

impl MdSolver {
    pub fn new(config: MdConfig) -> Self {
        let n = config.n_particles;
        MdSolver {
            config,
            positions: vec![[0.0; 3]; n],
            velocities: vec![[0.0; 3]; n],
            forces: vec![[0.0; 3]; n],
            time: 0.0,
            steps: 0,
            neighbors: Vec::new(),
            neighbor_offsets: vec![0; n + 1],
            rebuild_positions: vec![[0.0; 3]; n],
            last_rebuild_step: 0,
            initial_positions: vec![[0.0; 3]; n],
            rng_state: 0x1234_5678_9ABC_DEF0,
            kinetic_energy: 0.0,
            potential_energy: 0.0,
            virial: 0.0,
        }
    }

    /// 最小镜像约定: 将 delta 映射到 [-L/2, L/2]
    pub fn minimum_image(delta: f32, box_size: f32) -> f32 {
        delta - box_size * ((delta / box_size) + 0.5).floor()
    }

    /// xorshift64 均匀随机 [0, 1)
    fn rand(&mut self) -> f32 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        (x >> 11) as f32 / (1u64 << 53) as f32
    }

    /// Box-Muller 正态分布 (均值 0, 方差 1)
    fn rand_normal(&mut self) -> f32 {
        let u1 = self.rand().max(1e-10);
        let u2 = self.rand();
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f32::consts::PI * u2;
        r * theta.cos()
    }

    /// 在 FCC 晶格上初始化位置
    pub fn initialize_fcc(&mut self, cells_per_side: usize) {
        let n = self.config.n_particles;
        let dims = self.config.dims as usize;
        let a = self.config.box_size[0] / cells_per_side as f32;
        let basis3 = [
            [0.0, 0.0, 0.0],
            [0.5, 0.5, 0.0],
            [0.5, 0.0, 0.5],
            [0.0, 0.5, 0.5],
        ];
        let basis2 = [[0.0, 0.0, 0.0], [0.5, 0.5, 0.0]];
        let nbasis = if dims == 3 { 4 } else { 2 };
        let mut idx = 0;
        'outer: for k in 0..cells_per_side {
            for j in 0..cells_per_side {
                for i in 0..cells_per_side {
                    for b in 0..nbasis {
                        if idx >= n {
                            break 'outer;
                        }
                        let bref = if dims == 3 { &basis3[b] } else { &basis2[b] };
                        self.positions[idx] = [
                            (i as f32 + bref[0]) * a,
                            (j as f32 + bref[1]) * a,
                            (k as f32 + bref[2]) * a,
                        ];
                        idx += 1;
                    }
                }
            }
        }
        self.initial_positions.copy_from_slice(&self.positions);
        self.rebuild_positions.copy_from_slice(&self.positions);
    }

    /// 随机初始化位置
    pub fn initialize_random(&mut self, seed: u64) {
        self.rng_state = if seed == 0 { 0x1234_5678_9ABC_DEF0 } else { seed };
        for i in 0..self.config.n_particles {
            for d in 0..3 {
                self.positions[i][d] = self.rand() * self.config.box_size[d];
            }
        }
        self.initial_positions.copy_from_slice(&self.positions);
        self.rebuild_positions.copy_from_slice(&self.positions);
    }

    /// Maxwell-Boltzmann 速度分布初始化, 去除质心运动, 缩放到目标温度
    pub fn initialize_velocities(&mut self, temperature: f32) {
        let n = self.config.n_particles;
        let dims = self.config.dims as usize;
        for i in 0..n {
            for d in 0..dims {
                self.velocities[i][d] = self.rand_normal() * (temperature / self.config.mass).sqrt();
            }
        }
        let mut vcm = [0.0f32; 3];
        for i in 0..n {
            for d in 0..dims {
                vcm[d] += self.velocities[i][d];
            }
        }
        for d in 0..dims {
            vcm[d] /= n as f32;
        }
        for i in 0..n {
            for d in 0..dims {
                self.velocities[i][d] -= vcm[d];
            }
        }
        let ke = self.kinetic_energy_raw();
        let t_cur = 2.0 * ke / (dims * n) as f32;
        if t_cur > 1e-10 {
            let scale = (temperature / t_cur).sqrt();
            for i in 0..n {
                for d in 0..dims {
                    self.velocities[i][d] *= scale;
                }
            }
        }
    }

    pub fn kinetic_energy_raw(&self) -> f32 {
        let n = self.config.n_particles;
        let dims = self.config.dims as usize;
        let mut ke = 0.0;
        for i in 0..n {
            let mut v2 = 0.0;
            for d in 0..dims {
                v2 += self.velocities[i][d] * self.velocities[i][d];
            }
            ke += 0.5 * self.config.mass * v2;
        }
        ke
    }

    /// 构建 Verlet 邻居表 (i < j)
    pub fn build_neighbor_list(&mut self) {
        let n = self.config.n_particles;
        let r_list = self.config.cutoff + self.config.skin;
        let r_list2 = r_list * r_list;
        let dims = self.config.dims as usize;
        let box_size = self.config.box_size;

        self.neighbors.clear();
        self.neighbor_offsets[0] = 0;
        for i in 0..n {
            for j in (i + 1)..n {
                let mut dr2 = 0.0;
                for d in 0..dims {
                    let delta = Self::minimum_image(
                        self.positions[j][d] - self.positions[i][d],
                        box_size[d],
                    );
                    dr2 += delta * delta;
                }
                if dr2 < r_list2 {
                    self.neighbors.push(j);
                }
            }
            self.neighbor_offsets[i + 1] = self.neighbors.len();
        }
        self.last_rebuild_step = self.steps;
        self.rebuild_positions.copy_from_slice(&self.positions);
    }

    /// 检查最大位移是否超过 skin/2, 需要重建
    fn needs_rebuild(&self) -> bool {
        if self.neighbors.is_empty() {
            return true;
        }
        let threshold = self.config.skin * 0.5;
        let threshold2 = threshold * threshold;
        let dims = self.config.dims as usize;
        let box_size = self.config.box_size;
        for i in 0..self.config.n_particles {
            let mut disp2 = 0.0;
            for d in 0..dims {
                let delta =
                    Self::minimum_image(self.positions[i][d] - self.rebuild_positions[i][d], box_size[d]);
                disp2 += delta * delta;
            }
            if disp2 > threshold2 {
                return true;
            }
        }
        false
    }

    /// 计算力 (LJ) + 势能 + virial
    pub fn compute_forces(&mut self) {
        let n = self.config.n_particles;
        let dims = self.config.dims as usize;
        let box_size = self.config.box_size;
        let cutoff2 = self.config.cutoff * self.config.cutoff;

        for f in self.forces.iter_mut() {
            *f = [0.0; 3];
        }
        let mut pe = 0.0;
        let mut virial = 0.0;

        for i in 0..n {
            let start = self.neighbor_offsets[i];
            let end = self.neighbor_offsets[i + 1];
            for &j in &self.neighbors[start..end] {
                let mut dr = [0.0f32; 3];
                let mut dr2 = 0.0;
                for d in 0..dims {
                    dr[d] = Self::minimum_image(
                        self.positions[i][d] - self.positions[j][d],
                        box_size[d],
                    );
                    dr2 += dr[d] * dr[d];
                }
                if dr2 < cutoff2 && dr2 > 1e-20 {
                    let r = dr2.sqrt();
                    let f_mag = self.config.lj_force_magnitude(r);
                    pe += self.config.lj_potential(r);
                    let inv_r = 1.0 / r;
                    for d in 0..dims {
                        let fcomp = f_mag * dr[d] * inv_r;
                        self.forces[i][d] += fcomp;
                        self.forces[j][d] -= fcomp;
                        virial += dr[d] * fcomp;
                    }
                }
            }
        }
        self.potential_energy = pe;
        self.virial = virial;
    }

    /// Velocity Verlet 一步 (二阶辛积分)
    pub fn velocity_verlet_step(&mut self) {
        let n = self.config.n_particles;
        let dims = self.config.dims as usize;
        let dt = self.config.dt;
        let half_dt = 0.5 * dt;
        let inv_m = 1.0 / self.config.mass;

        // v(t + dt/2) = v(t) + a(t)*dt/2
        for i in 0..n {
            for d in 0..dims {
                self.velocities[i][d] += half_dt * self.forces[i][d] * inv_m;
            }
        }
        // x(t + dt) = x(t) + v(t+dt/2)*dt, 周期包裹
        for i in 0..n {
            for d in 0..dims {
                self.positions[i][d] += dt * self.velocities[i][d];
                let b = self.config.box_size[d];
                self.positions[i][d] = ((self.positions[i][d] % b) + b) % b;
            }
        }
        // 重建邻居表 (如需要) + 新力
        if self.needs_rebuild() {
            self.build_neighbor_list();
        }
        self.compute_forces();
        // v(t + dt) = v(t+dt/2) + a(t+dt)*dt/2
        for i in 0..n {
            for d in 0..dims {
                self.velocities[i][d] += half_dt * self.forces[i][d] * inv_m;
            }
        }
    }

    fn apply_thermostat(&mut self) {
        let n = self.config.n_particles;
        let dims = self.config.dims as usize;
        match self.config.ensemble {
            Ensemble::NVE => {}
            Ensemble::NVTBerendsen { temperature: t0, tau } => {
                let ke = self.kinetic_energy_raw();
                let t_cur = 2.0 * ke / (dims * n) as f32;
                if t_cur > 1e-10 {
                    let dt = self.config.dt;
                    let factor = (1.0 + dt / tau * (t0 / t_cur - 1.0)).max(0.0).sqrt();
                    for i in 0..n {
                        for d in 0..dims {
                            self.velocities[i][d] *= factor;
                        }
                    }
                }
            }
            Ensemble::NVTLangevin { temperature: t0, damping, .. } => {
                let dt = self.config.dt;
                let m = self.config.mass;
                let c1 = (1.0 - damping * dt / m).max(0.0);
                let sigma = (2.0 * damping * t0 * dt / m).sqrt();
                for i in 0..n {
                    for d in 0..dims {
                        let eta = self.rand_normal();
                        self.velocities[i][d] = self.velocities[i][d] * c1 + sigma * eta;
                    }
                }
            }
            Ensemble::NPTBerendsen { temperature: t0, tau_t, .. } => {
                let ke = self.kinetic_energy_raw();
                let t_cur = 2.0 * ke / (dims * n) as f32;
                if t_cur > 1e-10 {
                    let dt = self.config.dt;
                    let factor = (1.0 + dt / tau_t * (t0 / t_cur - 1.0)).max(0.0).sqrt();
                    for i in 0..n {
                        for d in 0..dims {
                            self.velocities[i][d] *= factor;
                        }
                    }
                }
            }
        }
    }

    fn apply_barostat(&mut self) {
        if let Ensemble::NPTBerendsen {
            pressure: p_target,
            tau_p,
            compressibility,
            ..
        } = self.config.ensemble
        {
            let dims = self.config.dims as usize;
            let n = self.config.n_particles;
            let v = self.config.volume();
            let ke = self.kinetic_energy_raw();
            let t = 2.0 * ke / (dims * n) as f32;
            let p_cur = (n as f32 * K_B * t + self.virial / dims as f32) / v;
            let dt = self.config.dt;
            let mu = 1.0 + dt / tau_p * compressibility * (p_target - p_cur);
            let mu = mu.max(0.5).min(2.0);
            let scale = mu.powf(1.0 / dims as f32);
            for i in 0..n {
                for d in 0..dims {
                    self.positions[i][d] *= scale;
                }
            }
            for d in 0..dims {
                self.config.box_size[d] *= scale;
            }
            self.build_neighbor_list();
        }
    }

    /// 一步演化 (Velocity Verlet + 恒温器 + 恒压器)
    pub fn step(&mut self) {
        self.velocity_verlet_step();
        self.apply_thermostat();
        self.apply_barostat();
        self.kinetic_energy = self.kinetic_energy_raw();
        self.time += self.config.dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    // ============ 观测量 ============

    pub fn kinetic_energy(&self) -> f32 {
        self.kinetic_energy_raw()
    }
    pub fn potential_energy(&self) -> f32 {
        self.potential_energy
    }
    pub fn total_energy(&self) -> f32 {
        self.kinetic_energy_raw() + self.potential_energy
    }
    pub fn temperature(&self) -> f32 {
        let dims = self.config.dims as usize;
        let n = self.config.n_particles;
        2.0 * self.kinetic_energy_raw() / (dims * n) as f32
    }
    pub fn pressure(&self) -> f32 {
        let dims = self.config.dims as usize;
        let n = self.config.n_particles;
        let v = self.config.volume();
        let t = self.temperature();
        (n as f32 * K_B * t + self.virial / dims as f32) / v
    }

    /// 径向分布函数 g(r)
    pub fn radial_distribution(&self, bins: usize, r_max: f32) -> Vec<f32> {
        let n = self.config.n_particles;
        let dims = self.config.dims as usize;
        let box_size = self.config.box_size;
        let mut hist = vec![0u32; bins];
        let dr = r_max / bins as f32;

        for i in 0..n {
            for j in (i + 1)..n {
                let mut dr2 = 0.0;
                for d in 0..dims {
                    let delta = Self::minimum_image(
                        self.positions[i][d] - self.positions[j][d],
                        box_size[d],
                    );
                    dr2 += delta * delta;
                }
                let r = dr2.sqrt();
                if r < r_max {
                    let b = (r / dr) as usize;
                    if b < bins {
                        hist[b] += 2;
                    }
                }
            }
        }

        let rho = n as f32 / self.config.volume();
        let mut g = vec![0.0; bins];
        for k in 0..bins {
            let r_k = (k as f32 + 0.5) * dr;
            let r_in = r_k - 0.5 * dr;
            let r_out = r_k + 0.5 * dr;
            let shell = match dims {
                2 => std::f32::consts::PI * (r_out * r_out - r_in * r_in),
                _ => (4.0 / 3.0) * std::f32::consts::PI * (r_out.powi(3) - r_in.powi(3)),
            };
            let norm = n as f32 * rho * shell;
            g[k] = if norm > 1e-12 {
                hist[k] as f32 / norm
            } else {
                0.0
            };
        }
        g
    }

    /// 均方位移 MSD
    pub fn mean_square_displacement(&self) -> f32 {
        let n = self.config.n_particles;
        let dims = self.config.dims as usize;
        let box_size = self.config.box_size;
        let mut msd = 0.0;
        for i in 0..n {
            for d in 0..dims {
                let delta = self.positions[i][d] - self.initial_positions[i][d];
                let wrapped = Self::minimum_image(delta, box_size[d]);
                msd += wrapped * wrapped;
            }
        }
        msd / n as f32
    }

    /// 速度自相关 (相对于参考速度 v0)
    pub fn velocity_autocorrelation(&self, v0: &[[f32; 3]]) -> f32 {
        let n = self.config.n_particles;
        let dims = self.config.dims as usize;
        let mut sum = 0.0;
        let mut norm = 0.0;
        for i in 0..n {
            let mut dot = 0.0;
            let mut v0_sq = 0.0;
            let mut v_sq = 0.0;
            for d in 0..dims {
                dot += v0[i][d] * self.velocities[i][d];
                v0_sq += v0[i][d] * v0[i][d];
                v_sq += self.velocities[i][d] * self.velocities[i][d];
            }
            sum += dot;
            norm += (v0_sq * v_sq).sqrt();
        }
        if norm > 1e-12 {
            sum / norm
        } else {
            0.0
        }
    }

    pub fn snapshot_velocities(&self) -> Vec<[f32; 3]> {
        self.velocities.clone()
    }

    pub fn reset(&mut self) {
        for p in self.positions.iter_mut() {
            *p = [0.0; 3];
        }
        for v in self.velocities.iter_mut() {
            *v = [0.0; 3];
        }
        for f in self.forces.iter_mut() {
            *f = [0.0; 3];
        }
        self.time = 0.0;
        self.steps = 0;
        self.neighbors.clear();
        self.kinetic_energy = 0.0;
        self.potential_energy = 0.0;
        self.virial = 0.0;
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    // ============ 常数/枚举 ============

    #[test]
    fn test_kb() {
        assert_eq!(K_B, 1.0);
    }

    #[test]
    fn test_ensemble_equality() {
        assert_eq!(Ensemble::NVE, Ensemble::NVE);
        assert_eq!(
            Ensemble::NVTBerendsen { temperature: 1.0, tau: 0.5 },
            Ensemble::NVTBerendsen { temperature: 1.0, tau: 0.5 }
        );
        assert_ne!(
            Ensemble::NVE,
            Ensemble::NVTBerendsen { temperature: 1.0, tau: 0.5 }
        );
    }

    // ============ MdConfig ============

    #[test]
    fn test_config_default() {
        let c = MdConfig::default();
        assert_eq!(c.n_particles, 64);
        assert_eq!(c.dims, 3);
        assert_eq!(c.dt, 0.005);
        assert_eq!(c.epsilon, 1.0);
        assert_eq!(c.sigma, 1.0);
        assert_eq!(c.cutoff, 2.5);
        assert_eq!(c.ensemble, Ensemble::NVE);
    }

    #[test]
    fn test_config_volume() {
        let mut c = MdConfig::default();
        c.dims = 3;
        c.box_size = [2.0, 3.0, 4.0];
        assert!(approx_eq(c.volume(), 24.0, 1e-6));
        c.dims = 2;
        assert!(approx_eq(c.volume(), 6.0, 1e-6));
    }

    #[test]
    fn test_config_density() {
        let mut c = MdConfig::default();
        c.n_particles = 100;
        c.box_size = [5.0, 5.0, 5.0];
        c.dims = 3;
        assert!(approx_eq(c.density(), 0.8, 1e-6));
    }

    #[test]
    fn test_lj_potential_cutoff_zero() {
        let c = MdConfig::default();
        assert!(approx_eq(c.lj_potential(3.0), 0.0, 1e-6));
    }

    #[test]
    fn test_lj_potential_at_sigma() {
        let c = MdConfig::default();
        // U(sigma) = 4*(1-1) = 0, 平移后 U(sigma) = -U(r_c) = +0.0163
        let u = c.lj_potential(1.0);
        assert!(u > 0.0, "U(sigma) should be positive after shift, got {}", u);
        assert!(approx_eq(u, 0.0163, 0.001), "U(sigma) = {}, expected ~0.0163", u);
    }

    #[test]
    fn test_lj_potential_at_equilibrium() {
        let c = MdConfig::default();
        let r_eq = 2.0f32.powf(1.0 / 6.0);
        let u = c.lj_potential(r_eq);
        assert!(u < 0.0, "U(r_eq) should be negative, got {}", u);
        assert!(approx_eq(u, -0.9837, 0.01), "U(r_eq) = {}, expected ~-0.984", u);
    }

    #[test]
    fn test_lj_force_zero_at_equilibrium() {
        let c = MdConfig::default();
        let r_eq = 2.0f32.powf(1.0 / 6.0);
        let f = c.lj_force_magnitude(r_eq);
        assert!(f.abs() < 1e-3, "Force at r_eq should be ~0, got {}", f);
    }

    #[test]
    fn test_lj_force_repulsive() {
        let c = MdConfig::default();
        assert!(c.lj_force_magnitude(0.9) > 0.0);
    }

    #[test]
    fn test_lj_force_attractive() {
        let c = MdConfig::default();
        assert!(c.lj_force_magnitude(1.5) < 0.0);
    }

    // ============ MdSolver ============

    #[test]
    fn test_solver_new() {
        let s = MdSolver::new(MdConfig::default());
        assert_eq!(s.positions.len(), 64);
        assert_eq!(s.velocities.len(), 64);
        assert_eq!(s.forces.len(), 64);
        assert_eq!(s.steps, 0);
    }

    #[test]
    fn test_minimum_image() {
        assert!(approx_eq(MdSolver::minimum_image(0.3, 10.0), 0.3, 1e-6));
        assert!(approx_eq(MdSolver::minimum_image(6.0, 10.0), -4.0, 1e-6));
        assert!(approx_eq(MdSolver::minimum_image(-6.0, 10.0), 4.0, 1e-6));
        assert!(approx_eq(MdSolver::minimum_image(10.0, 10.0), 0.0, 1e-6));
    }

    #[test]
    fn test_initialize_fcc() {
        let mut c = MdConfig::default();
        c.n_particles = 32;
        c.dims = 3;
        c.box_size = [4.0, 4.0, 4.0];
        let mut s = MdSolver::new(c);
        s.initialize_fcc(2);
        assert!(approx_eq(s.positions[0][0], 0.0, 1e-6));
        assert!(approx_eq(s.positions[1][0], 1.0, 1e-6));
        assert!(approx_eq(s.positions[1][1], 1.0, 1e-6));
        for p in s.positions.iter() {
            assert!(p[0] >= 0.0 && p[0] < 4.0);
            assert!(p[1] >= 0.0 && p[1] < 4.0);
            assert!(p[2] >= 0.0 && p[2] < 4.0);
        }
    }

    #[test]
    fn test_initialize_velocities() {
        let mut c = MdConfig::default();
        c.n_particles = 32;
        c.dims = 3;
        let mut s = MdSolver::new(c);
        s.initialize_velocities(1.0);
        let mut vcm = [0.0f32; 3];
        for v in s.velocities.iter() {
            vcm[0] += v[0];
            vcm[1] += v[1];
            vcm[2] += v[2];
        }
        let n = s.velocities.len() as f32;
        assert!(vcm[0].abs() / n < 1e-3);
        assert!(vcm[1].abs() / n < 1e-3);
        assert!(vcm[2].abs() / n < 1e-3);
        assert!(approx_eq(s.temperature(), 1.0, 0.05));
    }

    #[test]
    fn test_build_neighbor_list() {
        let mut c = MdConfig::default();
        c.n_particles = 8;
        c.dims = 3;
        c.box_size = [3.0, 3.0, 3.0];
        c.cutoff = 1.5;
        c.skin = 0.5;
        let mut s = MdSolver::new(c);
        s.initialize_fcc(2);
        s.build_neighbor_list();
        assert!(!s.neighbors.is_empty());
    }

    #[test]
    fn test_compute_forces_finite() {
        let mut c = MdConfig::default();
        c.n_particles = 4;
        c.dims = 3;
        c.box_size = [2.0, 2.0, 2.0];
        c.cutoff = 1.5;
        c.skin = 0.3;
        let mut s = MdSolver::new(c);
        s.positions[0] = [0.0, 0.0, 0.0];
        s.positions[1] = [1.0, 0.0, 0.0];
        s.positions[2] = [0.0, 1.0, 0.0];
        s.positions[3] = [0.0, 0.0, 1.0];
        s.build_neighbor_list();
        s.compute_forces();
        for f in s.forces.iter() {
            assert!(f[0].is_finite());
            assert!(f[1].is_finite());
            assert!(f[2].is_finite());
        }
        // 牛顿第三定律: 总力 ≈ 0
        let mut ftot = [0.0f32; 3];
        for f in s.forces.iter() {
            ftot[0] += f[0];
            ftot[1] += f[1];
            ftot[2] += f[2];
        }
        assert!(ftot[0].abs() < 1e-3);
        assert!(ftot[1].abs() < 1e-3);
        assert!(ftot[2].abs() < 1e-3);
    }

    // ============ NVE 演化 ============

    #[test]
    fn test_step_progress() {
        let mut c = MdConfig::default();
        c.n_particles = 8;
        c.dims = 3;
        c.box_size = [3.0, 3.0, 3.0];
        c.dt = 0.001;
        let mut s = MdSolver::new(c);
        s.initialize_fcc(2);
        s.initialize_velocities(0.5);
        s.build_neighbor_list();
        s.compute_forces();
        let t0 = s.time;
        s.step();
        assert!(s.time > t0);
        assert_eq!(s.steps, 1);
    }

    #[test]
    fn test_nve_energy_conservation() {
        let mut c = MdConfig::default();
        c.n_particles = 8;
        c.dims = 3;
        c.box_size = [3.0, 3.0, 3.0];
        c.dt = 0.001;
        c.ensemble = Ensemble::NVE;
        let mut s = MdSolver::new(c);
        s.initialize_fcc(2);
        s.initialize_velocities(0.3);
        s.build_neighbor_list();
        s.compute_forces();
        let e0 = s.total_energy();
        s.step_n(50);
        let e1 = s.total_energy();
        let rel = (e1 - e0).abs() / e0.abs().max(1e-10);
        assert!(rel < 0.05, "Energy drift {:.2}%: {} -> {}", rel * 100.0, e0, e1);
    }

    #[test]
    fn test_nve_momentum_conservation() {
        let mut c = MdConfig::default();
        c.n_particles = 8;
        c.dims = 3;
        c.box_size = [3.0, 3.0, 3.0];
        c.dt = 0.001;
        c.ensemble = Ensemble::NVE;
        let mut s = MdSolver::new(c);
        s.initialize_fcc(2);
        s.initialize_velocities(0.3);
        s.build_neighbor_list();
        s.compute_forces();
        s.step_n(20);
        let mut p = [0.0f32; 3];
        for v in s.velocities.iter() {
            p[0] += v[0];
            p[1] += v[1];
            p[2] += v[2];
        }
        let n = s.velocities.len() as f32;
        assert!(p[0].abs() / n < 0.01);
        assert!(p[1].abs() / n < 0.01);
        assert!(p[2].abs() / n < 0.01);
    }

    // ============ NVT / NPT ============

    #[test]
    fn test_nvt_berendsen() {
        let mut c = MdConfig::default();
        c.n_particles = 16;
        c.dims = 3;
        c.box_size = [3.0, 3.0, 3.0];
        c.dt = 0.002;
        c.ensemble = Ensemble::NVTBerendsen { temperature: 1.0, tau: 0.1 };
        let mut s = MdSolver::new(c);
        s.initialize_fcc(2);
        s.initialize_velocities(0.5);
        s.build_neighbor_list();
        s.compute_forces();
        s.step_n(200);
        let t = s.temperature();
        assert!((t - 1.0).abs() < 0.5, "T should approach 1.0, got {}", t);
    }

    #[test]
    fn test_langevin_thermalization() {
        let mut c = MdConfig::default();
        c.n_particles = 16;
        c.dims = 3;
        c.box_size = [3.0, 3.0, 3.0];
        c.dt = 0.002;
        c.ensemble = Ensemble::NVTLangevin { temperature: 1.5, damping: 1.0, seed: 42 };
        let mut s = MdSolver::new(c);
        s.initialize_fcc(2);
        s.initialize_velocities(0.5);
        s.build_neighbor_list();
        s.compute_forces();
        s.step_n(500);
        let t = s.temperature();
        assert!((t - 1.5).abs() < 0.8, "T should approach 1.5, got {}", t);
    }

    // ============ 观测量 ============

    #[test]
    fn test_temperature() {
        let mut c = MdConfig::default();
        c.n_particles = 8;
        c.dims = 3;
        c.mass = 1.0;
        let mut s = MdSolver::new(c);
        for v in s.velocities.iter_mut() {
            *v = [1.0, 1.0, 1.0];
        }
        // KE = 0.5*8*3 = 12, T = 2*12/(3*8) = 1.0
        assert!(approx_eq(s.temperature(), 1.0, 1e-6));
    }

    #[test]
    fn test_pressure() {
        let mut c = MdConfig::default();
        c.n_particles = 8;
        c.dims = 3;
        c.box_size = [2.0, 2.0, 2.0];
        c.mass = 1.0;
        let mut s = MdSolver::new(c);
        for v in s.velocities.iter_mut() {
            *v = [1.0, 1.0, 1.0];
        }
        s.virial = 0.0;
        // P = N*k_B*T/V = 8*1*1/8 = 1.0
        assert!(approx_eq(s.pressure(), 1.0, 1e-3));
    }

    #[test]
    fn test_radial_distribution() {
        let mut c = MdConfig::default();
        c.n_particles = 8;
        c.dims = 3;
        c.box_size = [3.0, 3.0, 3.0];
        let mut s = MdSolver::new(c);
        s.initialize_fcc(2);
        let g = s.radial_distribution(20, 1.5);
        assert_eq!(g.len(), 20);
        // FCC 最近邻 a/sqrt(2) = 1.5/1.414 = 1.06
        let peak = g.iter().enumerate().max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());
        if let Some((i, _)) = peak {
            let r = (i as f32 + 0.5) * (1.5 / 20.0);
            assert!(r > 0.8 && r < 1.5, "Peak at r={}", r);
        }
    }

    #[test]
    fn test_msd_zero_initially() {
        let mut c = MdConfig::default();
        c.n_particles = 8;
        c.dims = 3;
        let mut s = MdSolver::new(c);
        s.initialize_fcc(2);
        assert!(approx_eq(s.mean_square_displacement(), 0.0, 1e-10));
    }

    #[test]
    fn test_msd_increases() {
        let mut c = MdConfig::default();
        c.n_particles = 8;
        c.dims = 3;
        c.box_size = [3.0, 3.0, 3.0];
        c.dt = 0.005;
        c.ensemble = Ensemble::NVTBerendsen { temperature: 1.0, tau: 0.5 };
        let mut s = MdSolver::new(c);
        s.initialize_fcc(2);
        s.initialize_velocities(1.0);
        s.build_neighbor_list();
        s.compute_forces();
        let msd0 = s.mean_square_displacement();
        s.step_n(50);
        let msd1 = s.mean_square_displacement();
        assert!(msd1 > msd0, "MSD should increase: {} -> {}", msd0, msd1);
    }

    #[test]
    fn test_velocity_autocorrelation_initial() {
        let mut c = MdConfig::default();
        c.n_particles = 8;
        c.dims = 3;
        let mut s = MdSolver::new(c);
        s.initialize_velocities(1.0);
        let v0 = s.snapshot_velocities();
        assert!(approx_eq(s.velocity_autocorrelation(&v0), 1.0, 0.01));
    }

    // ============ 其他 ============

    #[test]
    fn test_reset() {
        let mut c = MdConfig::default();
        c.n_particles = 8;
        let mut s = MdSolver::new(c);
        s.initialize_fcc(2);
        s.initialize_velocities(1.0);
        s.build_neighbor_list();
        s.compute_forces();
        s.step_n(5);
        s.reset();
        assert_eq!(s.steps, 0);
        assert_eq!(s.time, 0.0);
        assert!(s.neighbors.is_empty());
    }

    #[test]
    fn test_2d_solver() {
        let mut c = MdConfig::default();
        c.n_particles = 4;
        c.dims = 2;
        c.box_size = [3.0, 3.0, 0.0];
        c.dt = 0.001;
        let mut s = MdSolver::new(c);
        s.initialize_fcc(2);
        s.initialize_velocities(0.5);
        s.build_neighbor_list();
        s.compute_forces();
        s.step_n(10);
        assert_eq!(s.steps, 10);
        for p in s.positions.iter() {
            assert!(approx_eq(p[2], 0.0, 1e-6));
        }
    }
}

