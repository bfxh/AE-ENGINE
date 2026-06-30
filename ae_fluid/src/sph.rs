//! SPH — Smoothed Particle Hydrodynamics 无网格粒子流体
//!
//! 基于:
//! - Müller, Charypar, Gross. "Particle-Based Fluid Simulation for Interactive
//!   Applications." SCA 2003.
//! - Monaghan. "Smoothed Particle Hydrodynamics." Rep. Prog. Phys. 2005.
//!
//! 核心思想:
//! 1. 连续场用核函数 W(r,h) 离散到粒子上
//! 2. 密度: rho_i = sum_j m_j W(|x_i-x_j|, h)  (Poly6 核)
//! 3. 压力: p_i = k * (rho_i - rho0)  (线性状态方程)
//! 4. 力: F_pressure = -sum m_j (p_i+p_j)/(2 rho_j) grad W  (Spiky 核)
//!        F_viscosity = mu sum m_j (v_j-v_i)/rho_j lap W  (Viscosity 核)
//! 5. 空间哈希加速邻居搜索 O(n) 而非 O(n^2)
//!
//! 与 LFM (网格流体) 互补: SPH 适合自由表面液体 (水花、喷溅),
//! LFM 适合大尺度烟雾/火焰。

use serde::{Deserialize, Serialize};
use glam::Vec3;
use hashbrown::HashMap;

// ============================================================
// 配置
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SphConfig {
    pub dt: f32,
    /// 支持半径 h (核函数作用范围)
    pub h: f32,
    /// 静止密度 rho0
    pub rest_density: f32,
    /// 压力刚度 k
    pub pressure_k: f32,
    /// 动力学粘度 mu
    pub viscosity: f32,
    /// 重力加速度 (y 方向, 向下为负)
    pub gravity: f32,
    /// 粒子质量
    pub mass: f32,
    /// 边界盒 (min, max)
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    /// 边界恢复系数 (0=完全非弹性, 1=完全弹性)
    pub restitution: f32,
}

impl Default for SphConfig {
    fn default() -> Self {
        Self {
            dt: 0.005,
            h: 0.1,
            rest_density: 1000.0,
            pressure_k: 500.0,
            viscosity: 0.1,
            gravity: 9.81,
            mass: 0.02,
            bounds_min: Vec3::new(-1.0, -1.0, -1.0),
            bounds_max: Vec3::new(1.0, 1.0, 1.0),
            restitution: 0.3,
        }
    }
}

// ============================================================
// 粒子
// ============================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SphParticle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub density: f32,
    pub pressure: f32,
    pub force: Vec3,
}

impl SphParticle {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            density: 0.0,
            pressure: 0.0,
            force: Vec3::ZERO,
        }
    }
}

// ============================================================
// 核函数 (Muller 2003, 3D)
// ============================================================

const PI: f32 = std::f32::consts::PI;

/// Poly6 核 W(r^2, h) — 密度计算
#[inline]
pub fn poly6(r2: f32, h: f32) -> f32 {
    let h2 = h * h;
    if r2 >= h2 { return 0.0; }
    let h9 = h.powi(9);
    315.0 / (64.0 * PI * h9) * (h2 - r2).powi(3)
}

/// Spiky 核梯度 grad W(r, h) — 压力梯度
/// 返回向量 (指向 r_vec 方向)
#[inline]
pub fn spiky_grad(r_vec: Vec3, r: f32, h: f32) -> Vec3 {
    if r >= h || r < 1e-10 { return Vec3::ZERO; }
    let h6 = h.powi(6);
    let coeff = -45.0 / (PI * h6) * (h - r).powi(2);
    coeff * r_vec / r
}

/// Viscosity 核拉普拉斯 lap W(r, h) — 粘度
#[inline]
pub fn viscosity_laplacian(r: f32, h: f32) -> f32 {
    if r >= h { return 0.0; }
    let h6 = h.powi(6);
    45.0 / (PI * h6) * (h - r)
}

// ============================================================
// 空间哈希 (邻居搜索加速)
// ============================================================

#[derive(Debug, Clone)]
pub struct SpatialHash {
    pub cell_size: f32,
    pub map: HashMap<(i32, i32, i32), Vec<usize>>,
}

impl SpatialHash {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            map: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    #[inline]
    pub fn cell_coord(pos: Vec3, cell_size: f32) -> (i32, i32, i32) {
        (
            (pos.x / cell_size).floor() as i32,
            (pos.y / cell_size).floor() as i32,
            (pos.z / cell_size).floor() as i32,
        )
    }

    pub fn insert(&mut self, pos: Vec3, idx: usize) {
        let cell = Self::cell_coord(pos, self.cell_size);
        self.map.entry(cell).or_insert_with(Vec::new).push(idx);
    }

    /// 查询 pos 周围半径 h 内的粒子索引
    pub fn query(&self, pos: Vec3, h: f32) -> Vec<usize> {
        let mut result = Vec::new();
        let cell = Self::cell_coord(pos, self.cell_size);
        let span = (h / self.cell_size).ceil() as i32;
        for dx in -span..=span {
            for dy in -span..=span {
                for dz in -span..=span {
                    let key = (cell.0 + dx, cell.1 + dy, cell.2 + dz);
                    if let Some(indices) = self.map.get(&key) {
                        result.extend(indices.iter().copied());
                    }
                }
            }
        }
        result
    }
}

// ============================================================
// 求解器
// ============================================================

#[derive(Debug, Clone)]
pub struct SphSolver {
    pub config: SphConfig,
    pub particles: Vec<SphParticle>,
    pub spatial_hash: SpatialHash,
    pub time: f32,
    pub step_count: usize,
}

impl SphSolver {
    pub fn new(config: SphConfig) -> Self {
        let h = config.h;
        Self {
            config,
            particles: Vec::new(),
            spatial_hash: SpatialHash::new(h),
            time: 0.0,
            step_count: 0,
        }
    }

    pub fn add_particle(&mut self, position: Vec3) -> usize {
        let idx = self.particles.len();
        self.particles.push(SphParticle::new(position));
        idx
    }

    /// 单步时间步进
    pub fn step(&mut self) {
        let dt = self.config.dt;
        // 1. 重建空间哈希
        self.build_spatial_hash();
        // 2. 计算密度和压力
        self.compute_densities_and_pressures();
        // 3. 计算力
        self.compute_forces();
        // 4. 积分 (半隐式 Euler)
        self.integrate(dt);
        // 5. 边界处理
        self.handle_boundaries();
        self.time += dt;
        self.step_count += 1;
    }

    fn build_spatial_hash(&mut self) {
        self.spatial_hash.clear();
        for (i, p) in self.particles.iter().enumerate() {
            self.spatial_hash.insert(p.position, i);
        }
    }

    fn compute_densities_and_pressures(&mut self) {
        let h = self.config.h;
        let mass = self.config.mass;
        let rho0 = self.config.rest_density;
        let k = self.config.pressure_k;
        let positions: Vec<Vec3> = self.particles.iter().map(|p| p.position).collect();
        for i in 0..self.particles.len() {
            let pos_i = positions[i];
            let neighbors = self.spatial_hash.query(pos_i, h);
            let mut density = 0.0;
            for &j in &neighbors {
                let r_vec = pos_i - positions[j];
                let r2 = r_vec.length_squared();
                density += mass * poly6(r2, h);
            }
            // 自身贡献
            density += mass * poly6(0.0, h);
            let pressure = k * (density - rho0).max(0.0);
            self.particles[i].density = density;
            self.particles[i].pressure = pressure;
        }
    }

    fn compute_forces(&mut self) {
        let h = self.config.h;
        let mass = self.config.mass;
        let mu = self.config.viscosity;
        let g = self.config.gravity;
        // 缓存当前状态
        let states: Vec<(Vec3, Vec3, f32, f32)> = self.particles.iter()
            .map(|p| (p.position, p.velocity, p.density, p.pressure))
            .collect();
        for i in 0..self.particles.len() {
            let (pos_i, vel_i, rho_i, p_i) = states[i];
            let neighbors = self.spatial_hash.query(pos_i, h);
            let mut f_pressure = Vec3::ZERO;
            let mut f_viscosity = Vec3::ZERO;
            for &j in &neighbors {
                if i == j { continue; }
                let (pos_j, vel_j, rho_j, p_j) = states[j];
                let r_vec = pos_i - pos_j;
                let r = r_vec.length();
                if r < 1e-10 || r >= h { continue; }
                // 压力力: -sum m_j (p_i+p_j)/(2 rho_j) grad W
                let grad = spiky_grad(r_vec, r, h);
                f_pressure -= (mass * (p_i + p_j) / (2.0 * rho_j)) * grad;
                // 粘度力: mu sum m_j (v_j-v_i)/rho_j lap W
                let lap = viscosity_laplacian(r, h);
                f_viscosity += (mu * mass / rho_j) * (vel_j - vel_i) * lap;
            }
            // 重力
            let f_gravity = Vec3::new(0.0, -g, 0.0) * rho_i;
            self.particles[i].force = f_pressure + f_viscosity + f_gravity;
        }
    }

    fn integrate(&mut self, dt: f32) {
        for p in &mut self.particles {
            // a = F / rho
            let rho = p.density.max(1.0);
            let accel = p.force / rho;
            p.velocity += accel * dt;
            p.position += p.velocity * dt;
        }
    }

    fn handle_boundaries(&mut self) {
        let min = self.config.bounds_min;
        let max = self.config.bounds_max;
        let e = self.config.restitution;
        for p in &mut self.particles {
            // x
            if p.position.x < min.x {
                p.position.x = min.x;
                p.velocity.x = -p.velocity.x * e;
            } else if p.position.x > max.x {
                p.position.x = max.x;
                p.velocity.x = -p.velocity.x * e;
            }
            // y
            if p.position.y < min.y {
                p.position.y = min.y;
                p.velocity.y = -p.velocity.y * e;
            } else if p.position.y > max.y {
                p.position.y = max.y;
                p.velocity.y = -p.velocity.y * e;
            }
            // z
            if p.position.z < min.z {
                p.position.z = min.z;
                p.velocity.z = -p.velocity.z * e;
            } else if p.position.z > max.z {
                p.position.z = max.z;
                p.velocity.z = -p.velocity.z * e;
            }
        }
    }

    /// 总动能
    pub fn kinetic_energy(&self) -> f32 {
        let m = self.config.mass;
        self.particles.iter()
            .map(|p| 0.5 * m * p.velocity.length_squared())
            .sum()
    }

    /// 平均密度
    pub fn average_density(&self) -> f32 {
        if self.particles.is_empty() { return 0.0; }
        self.particles.iter().map(|p| p.density).sum::<f32>() / self.particles.len() as f32
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sph_config_default() {
        let c = SphConfig::default();
        assert!((c.h - 0.1).abs() < 1e-6);
        assert_eq!(c.rest_density, 1000.0);
        assert!(c.pressure_k > 0.0);
    }

    #[test]
    fn test_poly6_kernel() {
        let h = 0.1f32;
        // 中心值最大
        let w0 = poly6(0.0, h);
        let w1 = poly6(h * h * 0.25, h);
        assert!(w0 > w1);
        // 超出范围为 0
        let w_far = poly6(h * h * 1.1, h);
        assert_eq!(w_far, 0.0);
    }

    #[test]
    fn test_spiky_grad() {
        let h = 0.1f32;
        let r_vec = Vec3::new(0.05, 0.0, 0.0);
        let grad = spiky_grad(r_vec, 0.05, h);
        // 梯度应指向 r_vec 方向 (负，因为 coeff 为负)
        assert!(grad.x.abs() > 0.0);
        // r >= h 时为 0
        let grad_far = spiky_grad(Vec3::new(0.2, 0.0, 0.0), 0.2, h);
        assert_eq!(grad_far, Vec3::ZERO);
    }

    #[test]
    fn test_viscosity_laplacian() {
        let h = 0.1f32;
        let lap = viscosity_laplacian(0.05, h);
        assert!(lap > 0.0);
        let lap_far = viscosity_laplacian(0.2, h);
        assert_eq!(lap_far, 0.0);
    }

    #[test]
    fn test_spatial_hash() {
        let mut hash = SpatialHash::new(0.1);
        hash.insert(Vec3::new(0.05, 0.05, 0.05), 0);
        hash.insert(Vec3::new(0.15, 0.05, 0.05), 1);
        hash.insert(Vec3::new(1.0, 1.0, 1.0), 2);
        let neighbors = hash.query(Vec3::new(0.1, 0.05, 0.05), 0.1);
        assert!(neighbors.contains(&0));
        assert!(neighbors.contains(&1));
        assert!(!neighbors.contains(&2));
    }

    #[test]
    fn test_sph_free_fall() {
        let mut solver = SphSolver::new(SphConfig {
            dt: 0.005, h: 0.1, rest_density: 1000.0,
            pressure_k: 0.0, viscosity: 0.0, gravity: 9.81,
            mass: 0.02,
            bounds_min: Vec3::new(-1.0, -1.0, -1.0),
            bounds_max: Vec3::new(1.0, 1.0, 1.0),
            restitution: 0.0,
        });
        solver.add_particle(Vec3::new(0.0, 0.5, 0.0));
        let y0 = solver.particles[0].position.y;
        for _ in 0..40 {
            solver.step();
        }
        let y1 = solver.particles[0].position.y;
        assert!(y1 < y0, "should fall: y0={} y1={}", y0, y1);
    }

    #[test]
    fn test_sph_density_computation() {
        let mut solver = SphSolver::new(SphConfig::default());
        // 两个靠近的粒子
        solver.add_particle(Vec3::new(0.0, 0.0, 0.0));
        solver.add_particle(Vec3::new(0.05, 0.0, 0.0));
        solver.build_spatial_hash();
        solver.compute_densities_and_pressures();
        // 密度应该 > 0 (有邻居贡献)
        assert!(solver.particles[0].density > 0.0);
        assert!(solver.particles[1].density > 0.0);
    }

    #[test]
    fn test_sph_boundary_collision() {
        let mut solver = SphSolver::new(SphConfig {
            dt: 0.005, h: 0.1, rest_density: 1000.0,
            pressure_k: 0.0, viscosity: 0.0, gravity: 9.81,
            mass: 0.02,
            bounds_min: Vec3::new(-0.5, -0.5, -0.5),
            bounds_max: Vec3::new(0.5, 0.5, 0.5),
            restitution: 0.5,
        });
        solver.add_particle(Vec3::new(0.0, 0.4, 0.0));
        for _ in 0..200 {
            solver.step();
        }
        // 粒子应该在边界内
        let p = &solver.particles[0];
        assert!(p.position.y >= -0.5 - 1e-3, "particle below floor: y={}", p.position.y);
        assert!(p.position.y <= 0.5 + 1e-3);
    }

    #[test]
    fn test_sph_stability() {
        let mut solver = SphSolver::new(SphConfig {
            dt: 0.003, h: 0.1, rest_density: 1000.0,
            pressure_k: 200.0, viscosity: 0.1, gravity: 9.81,
            mass: 0.02,
            bounds_min: Vec3::new(-0.5, -0.5, -0.5),
            bounds_max: Vec3::new(0.5, 0.5, 0.5),
            restitution: 0.2,
        });
        // 粒子网格
        for i in 0..5 {
            for j in 0..5 {
                for k in 0..5 {
                    solver.add_particle(Vec3::new(i as f32 * 0.04, j as f32 * 0.04, k as f32 * 0.04));
                }
            }
        }
        let mut max_ke = 0.0f32;
        for _ in 0..100 {
            solver.step();
            max_ke = max_ke.max(solver.kinetic_energy());
        }
        assert!(max_ke < 20.0, "energy should stay bounded: max_ke={}", max_ke);
    }

    #[test]
    fn test_sph_water_settles() {
        let mut solver = SphSolver::new(SphConfig {
            dt: 0.003, h: 0.1, rest_density: 1000.0,
            pressure_k: 300.0, viscosity: 0.3, gravity: 9.81,
            mass: 0.02,
            bounds_min: Vec3::new(-0.5, -0.5, -0.5),
            bounds_max: Vec3::new(0.5, 0.5, 0.5),
            restitution: 0.1,
        });
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    solver.add_particle(Vec3::new(i as f32 * 0.04, j as f32 * 0.04, k as f32 * 0.04));
                }
            }
        }
        for _ in 0..300 {
            solver.step();
        }
        // 水应该沉到底部
        let avg_y = solver.particles.iter().map(|p| p.position.y).sum::<f32>() / solver.particles.len() as f32;
        assert!(avg_y < 0.0, "water should settle at bottom: avg_y={}", avg_y);
    }
}
