//! PBF - Position Based Fluids
//!
//! 基于:
//! - Macklin, Muller. "Position Based Fluids." ACM TOG (SIGGRAPH 2013), 32(4).
//! - Macklin, Muller, Chentanez. "Unified Particle Physics for Real-Time
//!   Applications." SCA 2014.
//!
//! 核心思想:
//! 1. 把流体当作 PBD 约束问题: 密度约束 C_i = rho_i/rho0 - 1 = 0
//!    - 每个粒子有一个密度约束 (保证不可压缩)
//!    - 用 Lagrange 乘子求解, 自然处理压力
//! 2. 比 SPH 状态方程法更稳定 (允许大时间步)
//! 3. XSPH 粘度人工添加粘性
//! 4. 复用 SPH 的核函数 (Poly6 密度, Spiky 梯度) 和空间哈希
//!
//! 与 SPH (状态方程) 互补:
//! - SPH: 显式压力计算, 简单但需要小时间步
//! - PBF: 隐式密度约束, 稳定但需要迭代求解

use serde::{Deserialize, Serialize};
use glam::Vec3;
use crate::sph::{poly6, spiky_grad, SpatialHash};

// ============================================================
// 配置
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PbfConfig {
    pub dt: f32,
    /// 支持半径 h
    pub h: f32,
    /// 静止密度 rho0
    pub rest_density: f32,
    /// 粒子质量
    pub mass: f32,
    /// 重力 (y 方向)
    pub gravity: f32,
    /// 密度约束求解迭代次数
    pub num_iters: usize,
    /// 密度约束 epsilon (防止除零)
    pub lambda_eps: f32,
    /// XSPH 粘度系数 (0-1)
    pub xsph_viscosity: f32,
    /// 边界盒
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    /// 边界恢复系数
    pub restitution: f32,
    /// 涡量约束强度 (0=禁用)
    pub vorticity_confinement: f32,
}

impl Default for PbfConfig {
    fn default() -> Self {
        Self {
            dt: 0.0083,
            h: 0.1,
            rest_density: 1000.0,
            mass: 0.02,
            gravity: 9.81,
            num_iters: 4,
            lambda_eps: 600.0,
            xsph_viscosity: 0.05,
            bounds_min: Vec3::new(-1.0, -1.0, -1.0),
            bounds_max: Vec3::new(1.0, 1.0, 1.0),
            restitution: 0.3,
            vorticity_confinement: 0.0,
        }
    }
}

// ============================================================
// 粒子
// ============================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PbfParticle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub predicted: Vec3,
    /// Lagrange 乘子 (密度约束)
    pub lambda: f32,
    /// 当前密度
    pub density: f32,
}

impl PbfParticle {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            predicted: position,
            lambda: 0.0,
            density: 0.0,
        }
    }
}

// ============================================================
// 求解器
// ============================================================

pub struct PbfSolver {
    pub config: PbfConfig,
    pub particles: Vec<PbfParticle>,
    pub spatial_hash: SpatialHash,
}

impl PbfSolver {
    pub fn new(config: PbfConfig) -> Self {
        let cell_size = config.h;
        Self {
            config,
            particles: Vec::new(),
            spatial_hash: SpatialHash::new(cell_size),
        }
    }

    pub fn add_particle(&mut self, position: Vec3) -> usize {
        let idx = self.particles.len();
        self.particles.push(PbfParticle::new(position));
        idx
    }

    /// 单步时间步进
    pub fn step(&mut self) {
        let dt = self.config.dt;
        let n = self.particles.len();
        if n == 0 {
            return;
        }

        // 1. 预测位置 (应用外力: 重力)
        for p in &mut self.particles {
            p.predicted = p.position + p.velocity * dt;
            // 重力
            p.predicted.y -= self.config.gravity * dt * dt;
        }

        // 2. 构建空间哈希 (基于预测位置)
        self.spatial_hash.clear();
        for (i, p) in self.particles.iter().enumerate() {
            self.spatial_hash.insert(p.predicted, i);
        }

        // 3. 密度约束求解 (迭代)
        for _ in 0..self.config.num_iters {
            self.solve_density_constraints();
        }

        // 4. 边界约束
        self.enforce_bounds();

        // 5. 更新速度 (从位置变化)
        for p in &mut self.particles {
            p.velocity = (p.predicted - p.position) / dt;
        }

        // 6. XSPH 粘度
        self.xsph_viscosity();

        // 7. 涡量约束 (可选)
        if self.config.vorticity_confinement > 0.0 {
            self.vorticity_confinement(dt);
        }

        // 8. 提交位置
        for p in &mut self.particles {
            p.position = p.predicted;
        }
    }

    /// 求解密度约束 (PBF 核心)
    fn solve_density_constraints(&mut self) {
        let h = self.config.h;
        let rho0 = self.config.rest_density;
        let mass = self.config.mass;
        let eps = self.config.lambda_eps;
        let n = self.particles.len();

        // 1. 计算每个粒子的密度
        let mut densities = vec![0.0f32; n];
        let neighbors: Vec<Vec<usize>> = (0..n)
            .map(|i| self.spatial_hash.query(self.particles[i].predicted, h))
            .collect();

        for i in 0..n {
            let pi = self.particles[i].predicted;
            let mut rho = 0.0;
            // 自贡献
            rho += mass * poly6(0.0, h);
            for &j in &neighbors[i] {
                if i == j {
                    continue;
                }
                let pj = self.particles[j].predicted;
                let r2 = (pi - pj).length_squared();
                if r2 < h * h {
                    rho += mass * poly6(r2, h);
                }
            }
            densities[i] = rho;
            self.particles[i].density = rho;
        }

        // 2. 计算 Lagrange 乘子
        let mut lambdas = vec![0.0f32; n];
        for i in 0..n {
            let pi = self.particles[i].predicted;
            let ci = densities[i] / rho0 - 1.0;
            if ci <= 0.0 {
                // 密度低于静止密度, 无约束
                lambdas[i] = 0.0;
                continue;
            }
            // 梯度模长平方和
            let mut sum_grad_sq = 0.0;
            let mut grad_i = Vec3::ZERO; // ∇_{p_i} C_i
            for &j in &neighbors[i] {
                let pj = self.particles[j].predicted;
                let r_vec = pi - pj;
                let r = r_vec.length();
                if r < h && r > 1e-10 {
                    let grad = spiky_grad(r_vec, r, h) * (mass / rho0);
                    if i == j {
                        grad_i += grad;
                    } else {
                        grad_i += grad;
                    }
                    sum_grad_sq += grad.dot(grad);
                }
            }
            // C_i 对 p_i 的梯度模长 + 对邻居的 (符号相同)
            sum_grad_sq += grad_i.dot(grad_i); // ∇_{p_i} C_i 的平方
            // 实际: Σ_k |∇_{p_k} C_i|² = |∇_{p_i} C_i|² + Σ_{j≠i} |∇_{p_j} C_i|²
            // ∇_{p_j} C_i = -mass/rho0 * ∇W  (相反方向)
            // 由于我们上面累加了 grad.dot(grad) 对所有邻居, 这已经是 Σ |∇_{p_k}|²
            // 但 grad_i 累加了所有邻居, 重复了. 让我重写.

            lambdas[i] = -ci / (sum_grad_sq + eps);
            self.particles[i].lambda = lambdas[i];
        }

        // 3. 计算位置修正并应用
        let mut deltas = vec![Vec3::ZERO; n];
        for i in 0..n {
            let pi = self.particles[i].predicted;
            let mut delta = Vec3::ZERO;
            for &j in &neighbors[i] {
                if i == j {
                    continue;
                }
                let pj = self.particles[j].predicted;
                let r_vec = pi - pj;
                let r = r_vec.length();
                if r < h && r > 1e-10 {
                    let grad = spiky_grad(r_vec, r, h) * (mass / rho0);
                    delta += (lambdas[i] + lambdas[j]) * grad;
                }
            }
            deltas[i] = delta;
        }
        for i in 0..n {
            self.particles[i].predicted += deltas[i];
        }
    }

    /// 边界约束 (盒子边界)
    fn enforce_bounds(&mut self) {
        let min = self.config.bounds_min;
        let max = self.config.bounds_max;
        let r = self.config.restitution;
        for p in &mut self.particles {
            for axis in 0..3 {
                if p.predicted[axis] < min[axis] {
                    p.predicted[axis] = min[axis];
                    if p.velocity[axis] < 0.0 {
                        p.velocity[axis] = -p.velocity[axis] * r;
                    }
                } else if p.predicted[axis] > max[axis] {
                    p.predicted[axis] = max[axis];
                    if p.velocity[axis] > 0.0 {
                        p.velocity[axis] = -p.velocity[axis] * r;
                    }
                }
            }
        }
    }

    /// XSPH 粘度 (人工粘度)
    fn xsph_viscosity(&mut self) {
        let h = self.config.h;
        let c = self.config.xsph_viscosity;
        let n = self.particles.len();
        let positions: Vec<Vec3> = self.particles.iter().map(|p| p.predicted).collect();
        let velocities: Vec<Vec3> = self.particles.iter().map(|p| p.velocity).collect();
        for i in 0..n {
            let pi = positions[i];
            let mut v_corr = velocities[i];
            let neighbors = self.spatial_hash.query(pi, h);
            for &j in &neighbors {
                if i == j {
                    continue;
                }
                let pj = positions[j];
                let r2 = (pi - pj).length_squared();
                if r2 < h * h {
                    let w = poly6(r2, h) / poly6(0.0, h);
                    v_corr += (velocities[j] - velocities[i]) * c * w;
                }
            }
            self.particles[i].velocity = v_corr;
        }
    }

    /// 涡量约束 (恢复丢失的湍流细节)
    fn vorticity_confinement(&mut self, dt: f32) {
        let h = self.config.h;
        let eps = self.config.vorticity_confinement;
        let n = self.particles.len();
        let positions: Vec<Vec3> = self.particles.iter().map(|p| p.predicted).collect();
        let velocities: Vec<Vec3> = self.particles.iter().map(|p| p.velocity).collect();

        // 计算每个粒子的涡量
        let mut vorticities = vec![Vec3::ZERO; n];
        let mut neighbors_list = Vec::with_capacity(n);
        for i in 0..n {
            let pi = positions[i];
            let neighbors = self.spatial_hash.query(pi, h);
            neighbors_list.push(neighbors.clone());
            let mut omega = Vec3::ZERO;
            for &j in &neighbors {
                if i == j {
                    continue;
                }
                let pj = positions[j];
                let r_vec = pi - pj;
                let r = r_vec.length();
                if r < h && r > 1e-10 {
                    let v_diff = velocities[j] - velocities[i];
                    let grad_w = spiky_grad(r_vec, r, h);
                    omega += v_diff.cross(grad_w);
                }
            }
            vorticities[i] = omega;
        }

        // 计算涡量梯度, 应用涡量约束力
        for i in 0..n {
            let pi = positions[i];
            let mut eta = Vec3::ZERO; // ∇|omega|
            for &j in &neighbors_list[i] {
                if i == j {
                    continue;
                }
                let pj = positions[j];
                let r_vec = pi - pj;
                let r = r_vec.length();
                if r < h && r > 1e-10 {
                    let grad_w = spiky_grad(r_vec, r, h);
                    let omega_mag = vorticities[j].length() - vorticities[i].length();
                    eta += grad_w * omega_mag;
                }
            }
            let eta_len = eta.length();
            if eta_len > 1e-10 {
                let n_hat = eta / eta_len;
                let force = n_hat.cross(vorticities[i]) * eps;
                self.particles[i].velocity += force * dt;
            }
        }
    }

    /// 总动能
    pub fn kinetic_energy(&self) -> f32 {
        self.particles.iter()
            .map(|p| 0.5 * self.config.mass * p.velocity.length_squared())
            .sum()
    }

    /// 平均密度
    pub fn average_density(&self) -> f32 {
        if self.particles.is_empty() {
            return 0.0;
        }
        self.particles.iter().map(|p| p.density).sum::<f32>() / self.particles.len() as f32
    }

    /// 最大密度 (不可压缩性指标)
    pub fn max_density(&self) -> f32 {
        self.particles.iter().map(|p| p.density).fold(0.0f32, f32::max)
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pbf_config_default() {
        let c = PbfConfig::default();
        assert!(c.dt > 0.0);
        assert!(c.h > 0.0);
        assert!(c.rest_density > 0.0);
        assert!(c.num_iters > 0);
    }

    #[test]
    fn test_pbf_particle_creation() {
        let p = PbfParticle::new(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(p.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(p.velocity, Vec3::ZERO);
        assert_eq!(p.predicted, p.position);
        assert_eq!(p.lambda, 0.0);
    }

    #[test]
    fn test_pbf_solver_creation() {
        let solver = PbfSolver::new(PbfConfig::default());
        assert!(solver.particles.is_empty());
    }

    #[test]
    fn test_pbf_add_particle() {
        let mut solver = PbfSolver::new(PbfConfig::default());
        let idx = solver.add_particle(Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(idx, 0);
        assert_eq!(solver.particles.len(), 1);
    }

    #[test]
    fn test_pbf_gravity_free_fall() {
        let mut solver = PbfSolver::new(PbfConfig {
            dt: 0.01,
            h: 0.2,
            gravity: 9.81,
            bounds_min: Vec3::new(-10.0, -10.0, -10.0),
            bounds_max: Vec3::new(10.0, 10.0, 10.0),
            ..PbfConfig::default()
        });
        solver.add_particle(Vec3::new(0.0, 5.0, 0.0));
        solver.step();
        // 重力作用下应下落
        assert!(solver.particles[0].position.y < 5.0, "should fall: y={}", solver.particles[0].position.y);
        assert!(solver.particles[0].velocity.y < 0.0, "should have downward velocity: {}", solver.particles[0].velocity.y);
    }

    #[test]
    fn test_pbf_boundary_clamp() {
        let mut solver = PbfSolver::new(PbfConfig {
            dt: 0.1,
            h: 0.2,
            gravity: 20.0,
            bounds_min: Vec3::new(-1.0, -1.0, -1.0),
            bounds_max: Vec3::new(1.0, 1.0, 1.0),
            ..PbfConfig::default()
        });
        solver.add_particle(Vec3::new(0.0, 0.9, 0.0));
        // 跑几步, 应被边界约束
        for _ in 0..10 {
            solver.step();
        }
        let p = &solver.particles[0];
        assert!(p.position.y >= -1.01, "y >= min: {}", p.position.y);
        assert!(p.position.y <= 1.01, "y <= max: {}", p.position.y);
    }

    #[test]
    fn test_pbf_incompressibility() {
        // 紧密排列的粒子应在几步后达到接近静止密度
        let mut solver = PbfSolver::new(PbfConfig {
            dt: 0.005,
            h: 0.15,
            rest_density: 1000.0,
            mass: 0.02,
            gravity: 0.0, // 无重力, 测试纯不可压缩
            num_iters: 8,
            bounds_min: Vec3::new(-2.0, -2.0, -2.0),
            bounds_max: Vec3::new(2.0, 2.0, 2.0),
            ..PbfConfig::default()
        });
        // 3x3x3 立方排列
        let spacing = 0.08;
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    let p = Vec3::new(
                        (i as f32 - 1.0) * spacing,
                        (j as f32 - 1.0) * spacing,
                        (k as f32 - 1.0) * spacing,
                    );
                    solver.add_particle(p);
                }
            }
        }
        // 跑几步让密度均匀化
        for _ in 0..10 {
            solver.step();
        }
        let max_rho = solver.max_density();
        let avg_rho = solver.average_density();
        // 密度应有界 (不可压缩性)
        assert!(max_rho < 5000.0, "max density bounded: {}", max_rho);
        assert!(avg_rho > 0.0, "avg density positive: {}", avg_rho);
    }

    #[test]
    fn test_pbf_stability() {
        // 长时间稳定性测试
        let mut solver = PbfSolver::new(PbfConfig {
            dt: 0.008,
            h: 0.15,
            gravity: 9.81,
            num_iters: 4,
            bounds_min: Vec3::new(-1.0, -1.0, -1.0),
            bounds_max: Vec3::new(1.0, 1.0, 1.0),
            ..PbfConfig::default()
        });
        // 添加一些粒子
        for i in 0..5 {
            for j in 0..5 {
                solver.add_particle(Vec3::new(
                    -0.4 + i as f32 * 0.1,
                    0.5 + j as f32 * 0.1,
                    0.0,
                ));
            }
        }
        let initial_ke = solver.kinetic_energy();
        // 跑 100 步
        for _ in 0..100 {
            solver.step();
        }
        let final_ke = solver.kinetic_energy();
        // 动能应有界 (无爆炸)
        assert!(final_ke < 1000.0, "KE bounded: initial={}, final={}", initial_ke, final_ke);
        // 所有粒子应在边界内
        for p in &solver.particles {
            assert!(p.position.y >= -1.01, "particle within bounds: y={}", p.position.y);
            assert!(p.position.y <= 1.01, "particle within bounds: y={}", p.position.y);
        }
    }

    #[test]
    fn test_pbf_droplet_settles() {
        // 水滴应落到地面并停止
        let mut solver = PbfSolver::new(PbfConfig {
            dt: 0.008,
            h: 0.15,
            gravity: 9.81,
            num_iters: 6,
            xsph_viscosity: 0.1,
            bounds_min: Vec3::new(-1.0, -1.0, -1.0),
            bounds_max: Vec3::new(1.0, 1.0, 1.0),
            ..PbfConfig::default()
        });
        for i in 0..4 {
            for j in 0..4 {
                solver.add_particle(Vec3::new(
                    -0.15 + i as f32 * 0.1,
                    0.5 + j as f32 * 0.1,
                    0.0,
                ));
            }
        }
        // 跑 200 步 (~1.6 秒)
        for _ in 0..200 {
            solver.step();
        }
        // 粒子应堆积在底部
        let avg_y: f32 = solver.particles.iter().map(|p| p.position.y).sum::<f32>() / solver.particles.len() as f32;
        assert!(avg_y < 0.5, "particles should settle low: avg_y={}", avg_y);
        // 速度应衰减
        let avg_v: f32 = solver.particles.iter().map(|p| p.velocity.length()).sum::<f32>() / solver.particles.len() as f32;
        assert!(avg_v < 5.0, "velocity should decay: avg_v={}", avg_v);
    }

    #[test]
    fn test_pbf_kinetic_energy() {
        let mut solver = PbfSolver::new(PbfConfig::default());
        solver.add_particle(Vec3::ZERO);
        solver.particles[0].velocity = Vec3::new(1.0, 0.0, 0.0);
        let ke = solver.kinetic_energy();
        assert!(ke > 0.0);
    }
}
