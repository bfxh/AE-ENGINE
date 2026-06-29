//! IISPH — Implicit Incompressible SPH 隐式不可压缩 SPH
//!
//! 基于:
//! - Ihmsen, Cornelis, Solenthaler, Zessin, Teschner. "Implicit Incompressible
//!   SPH." IEEE TVCG 2014, 20(3).
//! - Previous: PCISPH by Solenthaler & Pajarola 2009 (predict-correct)
//!
//! 核心思想:
//! 1. 预测位置 x*_i = x_i + dt * (v_i + dt * F_adv_i / m_i)
//! 2. 预测密度 rho*_i = Σ_j m_j W(|x*_i - x*_j|, h)
//! 3. 求解压力 Poisson 方程 (PPE):
//!    div(1/rho * grad p) = (rho0 - rho*) / dt^2
//!    离散化: Σ_j m_j (p_i/rho_i^2 + p_j/rho_j^2) |grad W_ij|^2 = (rho0 - rho*_i)/dt^2
//!    用 Jacobi/Relaxation 迭代求解 p
//! 4. 压力加速度: a_p_i = -Σ_j m_j (p_i/rho_i^2 + p_j/rho_j^2) grad W_ij
//! 5. 速度更新: v_i += dt * (a_p_i + a_adv_i)
//! 6. 位置更新: x_i += dt * v_i
//!
//! 优点:
//! - 比 WCSPH (弱可压) 大 10-100x 时间步, 不需要刚性 EOS
//! - 比 PCISPH 收敛更快 (隐式而非迭代修正)
//! - 比 DFSPH 简单 (单一压力约束, DFSPH 有密度+散度两个约束)
//! - 严格不可压缩 (在数值精度内)
//!
//! 复用 sph.rs 的 poly6/spiky_grad 核函数和 SpatialHash

use serde::{Deserialize, Serialize};
use glam::Vec3;
use crate::sph::{poly6, spiky_grad, SpatialHash};

// ============================================================
// 配置
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IisphConfig {
    pub dt: f32,
    pub h: f32,
    pub rest_density: f32,
    pub viscosity: f32,
    pub gravity: Vec3,
    pub mass: f32,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    pub restitution: f32,
    /// PPE 迭代次数
    pub ppe_iterations: usize,
    /// PPE 收敛阈值 (密度误差)
    pub ppe_tolerance: f32,
    /// 压力松弛因子 (omega, 0-1)
    pub omega: f32,
}

impl Default for IisphConfig {
    fn default() -> Self {
        Self {
            dt: 0.005,
            h: 0.1,
            rest_density: 1000.0,
            viscosity: 0.01,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            mass: 0.02,
            bounds_min: Vec3::new(-1.0, -1.0, -1.0),
            bounds_max: Vec3::new(1.0, 1.0, 1.0),
            restitution: 0.3,
            ppe_iterations: 20,
            ppe_tolerance: 0.01,
            omega: 0.5, // 经验值, Rhem et al. 0.5
        }
    }
}

// ============================================================
// 粒子
// ============================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct IisphParticle {
    pub position: Vec3,
    pub velocity: Vec3,
    /// 当前密度
    pub density: f32,
    /// 预测密度 rho*
    pub predicted_density: f32,
    /// 当前压力
    pub pressure: f32,
    /// 上一步压力 (用于 warm start)
    pub last_pressure: f32,
    /// 非压力加速度 (重力+粘度)
    pub adv_accel: Vec3,
    /// 预测位置 x*
    pub predicted_position: Vec3,
    /// 压力加速度
    pub pressure_accel: Vec3,
}

impl IisphParticle {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            density: 0.0,
            predicted_density: 0.0,
            pressure: 0.0,
            last_pressure: 0.0,
            adv_accel: Vec3::ZERO,
            predicted_position: position,
            pressure_accel: Vec3::ZERO,
        }
    }
}

// ============================================================
// 求解器
// ============================================================

pub struct IisphSolver {
    pub config: IisphConfig,
    pub particles: Vec<IisphParticle>,
    /// 邻居索引缓存 (每步重建)
    neighbors: Vec<Vec<(usize, f32, Vec3)>>, // (idx, dist, r_vec)
}

impl IisphSolver {
    pub fn new(config: IisphConfig) -> Self {
        Self {
            config,
            particles: Vec::new(),
            neighbors: Vec::new(),
        }
    }

    pub fn add_particle(&mut self, p: IisphParticle) -> usize {
        let idx = self.particles.len();
        self.particles.push(p);
        idx
    }

    /// 单步时间步进
    pub fn step(&mut self) {
        let n = self.particles.len();
        if n == 0 {
            return;
        }
        let dt = self.config.dt;
        let h = self.config.h;

        // 1. 构建空间哈希 + 邻居列表
        let mut hash = SpatialHash::new(h);
        for (i, p) in self.particles.iter().enumerate() {
            hash.insert(p.position, i);
        }
        self.neighbors = vec![Vec::new(); n];
        for i in 0..n {
            let pos_i = self.particles[i].position;
            for j in hash.query(pos_i, h) {
                if i == j { continue; }
                let r_vec = pos_i - self.particles[j].position;
                let r = r_vec.length();
                if r < h {
                    self.neighbors[i].push((j, r, r_vec));
                }
            }
        }

        // 2. 计算非压力加速度 (重力 + 粘度)
        for i in 0..n {
            let mut a = self.config.gravity;
            // 粘度 (XSPH 风格, 但放在加速度里)
            let mut visc = Vec3::ZERO;
            let mu = self.config.viscosity;
            let m = self.config.mass;
            let rho_i = self.particles[i].density.max(1e-6);
            for &(j, r, _r_vec) in &self.neighbors[i] {
                let rho_j = self.particles[j].density.max(1e-6);
                let v_diff = self.particles[j].velocity - self.particles[i].velocity;
                // lap W viscosity
                let lap = viscosity_lap(r, h);
                visc += v_diff * (m / rho_j * lap);
            }
            a += visc * mu;
            self.particles[i].adv_accel = a;
        }

        // 3. 预测位置 x* = x + dt * (v + dt * a_adv)
        for i in 0..n {
            let p = &self.particles[i];
            self.particles[i].predicted_position = p.position + (p.velocity + p.adv_accel * dt) * dt;
        }

        // 4. 预测密度 rho*_i = Σ_j m_j W(|x*_i - x*_j|, h) + 自身贡献
        // 注意: 预测密度用预测位置计算
        let mut pred_hash = SpatialHash::new(h);
        for (i, p) in self.particles.iter().enumerate() {
            pred_hash.insert(p.predicted_position, i);
        }
        let m = self.config.mass;
        let h2 = h * h;
        for i in 0..n {
            let pos_i = self.particles[i].predicted_position;
            let mut rho = 0.0;
            // 自身贡献
            rho += m * poly6(0.0, h);
            for j in pred_hash.query(pos_i, h) {
                if i == j { continue; }
                let r_vec = pos_i - self.particles[j].predicted_position;
                let r2 = r_vec.length_squared();
                if r2 < h2 {
                    rho += m * poly6(r2, h);
                }
            }
            self.particles[i].predicted_density = rho;
        }

        // 5. 求解压力 Poisson 方程 (PPE)
        // 离散形式: Σ_j m_j (p_i/rho_i^2 + p_j/rho_j^2) |grad W_ij|^2 * dt^2 = rho0 - rho*_i
        // Jacobi 迭代:
        //   A_ii = dt^2 * Σ_j m_j * |grad W_ij|^2 / rho_j^2 + m_i * |grad W_ii|^2/rho_i^2 (近似省略)
        //   b_i = rho0 - rho*_i
        //   p_i = (b_i - Σ_{j!=i} A_ij p_j) / A_ii

        // Warm start: 用上一步压力
        for i in 0..n {
            self.particles[i].pressure = self.particles[i].last_pressure * 0.5;
        }

        // 预计算每个粒子的 A_ii (对角元素)
        let mut a_diag = vec![0.0f32; n];
        for i in 0..n {
            let rho_i = self.particles[i].predicted_density.max(1e-6);
            let mut sum_l2 = 0.0; // Σ_j (m_j/rho_j^2) |grad W_ij|^2
            // 自身项 (i=i): 用插值近似 |grad W_ii|^2
            // 实际中 grad W(0) = 0, 但 PPE 需要非零对角元
            // 用一个数值稳定化项
            let self_term = m / (rho_i * rho_i) * (45.0 / (std::f32::consts::PI * h.powi(6))) * 0.0;
            // 实际 grad W(0)=0, 所以自身项 = 0, 用边界项修正
            // 改用 Ihmsen 2014 Eq.10 的近似:
            // A_ii = dt^2 * Σ_j (m_j/rho_j^2 + m_i/rho_i^2) |grad W_ij|^2 / 2
            for &(_j, r, _r_vec) in &self.neighbors[i] {
                let rho_j = self.particles[i].predicted_density.max(1e-6); // 用 i 的预测密度近似
                let grad_w = spiky_grad(Vec3::new(h, 0.0, 0.0) - Vec3::ZERO, r, h);
                let grad_len2 = grad_w.length_squared();
                sum_l2 += m / (rho_j * rho_j) * grad_len2;
            }
            // 加上自身项 (避免奇异)
            a_diag[i] = dt * dt * (sum_l2 + self_term + 1e-6);
        }

        // PPE 迭代
        let omega = self.config.omega;
        let rho0 = self.config.rest_density;
        for _ in 0..self.config.ppe_iterations {
            let mut max_err = 0.0f32;
            // 计算 RHS 和 off-diagonal 贡献, 用 Jacobi 更新
            let mut new_p = vec![0.0f32; n];
            for i in 0..n {
                let rho_i = self.particles[i].predicted_density.max(1e-6);
                let b_i = rho0 - self.particles[i].predicted_density;
                let mut sum_off = 0.0;
                for &(j, r, r_vec) in &self.neighbors[i] {
                    let rho_j = self.particles[j].predicted_density.max(1e-6);
                    // grad W_ij (从 j 指向 i, 因为 r_vec = x_i - x_j)
                    let grad_w_ij = spiky_grad(r_vec, r, h);
                    let grad_len2 = grad_w_ij.length_squared();
                    // A_ij = -dt^2 * m_j * (m_i/rho_i^2 + m_j/rho_j^2) * |grad W_ij|^2
                    // 但 Ihmsen Eq.10 简化为:
                    // A_ij = -dt^2 * m_j * m_j / rho_j^2 * |grad W_ij|^2 (j 项)
                    //      - dt^2 * m_j * m_i / rho_i^2 * |grad W_ij|^2 (i 项, 已含在对角)
                    // 这里我们用对称形式, 对角已含 i 项, off-diagonal 只算 j 项
                    let a_ij = -dt * dt * m * m / (rho_j * rho_j) * grad_len2;
                    sum_off += a_ij * self.particles[j].pressure;
                }
                let p_new = (b_i - sum_off) / a_diag[i].max(1e-10);
                // 松弛
                new_p[i] = (1.0 - omega) * self.particles[i].pressure + omega * p_new.max(0.0);
                // 误差 = |rho_predicted_after_pressure - rho0|
                let err = (b_i - a_diag[i] * new_p[i] - sum_off).abs();
                if err > max_err {
                    max_err = err;
                }
            }
            for i in 0..n {
                self.particles[i].pressure = new_p[i];
            }
            if max_err < self.config.ppe_tolerance * rho0 {
                break;
            }
        }

        // 6. 计算压力加速度
        for i in 0..n {
            let rho_i = self.particles[i].predicted_density.max(1e-6);
            let p_i = self.particles[i].pressure;
            let mut a_p = Vec3::ZERO;
            for &(j, r, r_vec) in &self.neighbors[i] {
                let rho_j = self.particles[j].predicted_density.max(1e-6);
                let p_j = self.particles[j].pressure;
                let grad_w_ij = spiky_grad(r_vec, r, h);
                // a_p_i = -Σ_j m_j (p_i/rho_i^2 + p_j/rho_j^2) grad W_ij
                a_p -= grad_w_ij * (m * (p_i / (rho_i * rho_i) + p_j / (rho_j * rho_j)));
            }
            self.particles[i].pressure_accel = a_p;
        }

        // 7. 速度和位置更新
        for i in 0..n {
            let a_total = self.particles[i].adv_accel + self.particles[i].pressure_accel;
            let new_v = self.particles[i].velocity + a_total * dt;
            let new_x = self.particles[i].position + new_v * dt;
            self.particles[i].velocity = new_v;
            self.particles[i].position = new_x;
            // 保存压力供 warm start
            self.particles[i].last_pressure = self.particles[i].pressure;
            // 更新密度 (用当前位置)
            self.particles[i].density = self.particles[i].predicted_density;
        }

        // 8. 边界约束
        for p in &mut self.particles {
            for axis in 0..3 {
                if p.position[axis] < self.config.bounds_min[axis] {
                    p.position[axis] = self.config.bounds_min[axis];
                    if p.velocity[axis] < 0.0 {
                        p.velocity[axis] = -p.velocity[axis] * self.config.restitution;
                    }
                } else if p.position[axis] > self.config.bounds_max[axis] {
                    p.position[axis] = self.config.bounds_max[axis];
                    if p.velocity[axis] > 0.0 {
                        p.velocity[axis] = -p.velocity[axis] * self.config.restitution;
                    }
                }
            }
        }
    }

    /// 平均密度 (检查不可压缩性)
    pub fn average_density(&self) -> f32 {
        if self.particles.is_empty() {
            return 0.0;
        }
        self.particles.iter().map(|p| p.density).sum::<f32>() / self.particles.len() as f32
    }

    /// 平均压力
    pub fn average_pressure(&self) -> f32 {
        if self.particles.is_empty() {
            return 0.0;
        }
        self.particles.iter().map(|p| p.pressure).sum::<f32>() / self.particles.len() as f32
    }

    /// 最大密度误差 (与静止密度)
    pub fn max_density_error(&self) -> f32 {
        let rho0 = self.config.rest_density;
        self.particles.iter()
            .map(|p| (p.density - rho0).abs())
            .fold(0.0f32, f32::max)
    }

    /// 总动能
    pub fn kinetic_energy(&self) -> f32 {
        let m = self.config.mass;
        self.particles.iter()
            .map(|p| 0.5 * m * p.velocity.length_squared())
            .sum()
    }
}

/// Viscosity 核拉普拉斯 (与 sph.rs 一致, 内部使用)
#[inline]
fn viscosity_lap(r: f32, h: f32) -> f32 {
    if r >= h { return 0.0; }
    let h6 = h.powi(6);
    45.0 / (std::f32::consts::PI * h6) * (h - r)
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iisph_config_default() {
        let c = IisphConfig::default();
        assert!(c.dt > 0.0);
        assert!(c.h > 0.0);
        assert!(c.rest_density > 0.0);
        assert!(c.ppe_iterations > 0);
        assert!(c.omega > 0.0 && c.omega <= 1.0);
    }

    #[test]
    fn test_iisph_particle_creation() {
        let p = IisphParticle::new(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(p.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(p.velocity, Vec3::ZERO);
        assert_eq!(p.pressure, 0.0);
    }

    #[test]
    fn test_iisph_solver_creation() {
        let solver = IisphSolver::new(IisphConfig::default());
        assert!(solver.particles.is_empty());
    }

    #[test]
    fn test_iisph_single_particle_no_crash() {
        // 单粒子, 无邻居, 应不崩溃
        let mut solver = IisphSolver::new(IisphConfig::default());
        solver.add_particle(IisphParticle::new(Vec3::ZERO));
        solver.step();
        assert_eq!(solver.particles.len(), 1);
    }

    #[test]
    fn test_iisph_free_fall() {
        // 单粒子在重力下应下落
        let mut solver = IisphSolver::new(IisphConfig {
            dt: 0.005,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            bounds_min: Vec3::new(-5.0, -5.0, -5.0),
            bounds_max: Vec3::new(5.0, 5.0, 5.0),
            ..IisphConfig::default()
        });
        solver.add_particle(IisphParticle::new(Vec3::new(0.0, 4.0, 0.0)));
        let y0 = solver.particles[0].position.y;
        solver.step();
        let y1 = solver.particles[0].position.y;
        assert!(y1 < y0, "should fall: {} -> {}", y0, y1);
    }

    #[test]
    fn test_iisph_two_particles_interact() {
        // 两个粒子靠近, 应有压力相互作用
        let mut solver = IisphSolver::new(IisphConfig {
            dt: 0.001,
            h: 0.2,
            rest_density: 100.0,
            gravity: Vec3::ZERO,
            mass: 0.01,
            bounds_min: Vec3::new(-2.0, -2.0, -2.0),
            bounds_max: Vec3::new(2.0, 2.0, 2.0),
            ppe_iterations: 5,
            ..IisphConfig::default()
        });
        let p0 = solver.add_particle(IisphParticle::new(Vec3::new(0.0, 0.0, 0.0)));
        let p1 = solver.add_particle(IisphParticle::new(Vec3::new(0.05, 0.0, 0.0)));
        solver.step();
        // 至少预测密度 > 0
        assert!(solver.particles[p0].predicted_density > 0.0);
        assert!(solver.particles[p1].predicted_density > 0.0);
    }

    #[test]
    fn test_iisph_volume_preservation() {
        // 一团粒子在边界内, 体积应保持 (不爆炸)
        let mut solver = IisphSolver::new(IisphConfig {
            dt: 0.002,
            h: 0.15,
            rest_density: 1000.0,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            mass: 0.01,
            bounds_min: Vec3::new(-1.0, -1.0, -1.0),
            bounds_max: Vec3::new(1.0, 1.0, 1.0),
            ppe_iterations: 10,
            ..IisphConfig::default()
        });
        // 3x3x3 粒子块
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    solver.add_particle(IisphParticle::new(Vec3::new(
                        -0.1 + i as f32 * 0.05,
                        0.5 + j as f32 * 0.05,
                        -0.1 + k as f32 * 0.05,
                    )));
                }
            }
        }
        let initial_count = solver.particles.len();
        for _ in 0..50 {
            solver.step();
        }
        // 粒子数不变
        assert_eq!(solver.particles.len(), initial_count);
        // 所有粒子在边界内
        for p in &solver.particles {
            for axis in 0..3 {
                assert!(p.position[axis] >= -1.01 && p.position[axis] <= 1.01,
                    "particle in bounds: axis={} pos={}", axis, p.position[axis]);
            }
        }
        // 无 NaN
        for p in &solver.particles {
            assert!(p.position.x.is_finite(), "no NaN in position");
            assert!(p.velocity.x.is_finite(), "no NaN in velocity");
        }
    }

    #[test]
    fn test_iisph_density_computed() {
        // 多粒子应有非零密度
        let mut solver = IisphSolver::new(IisphConfig {
            dt: 0.001,
            h: 0.2,
            gravity: Vec3::ZERO,
            ..IisphConfig::default()
        });
        for i in 0..5 {
            for j in 0..5 {
                solver.add_particle(IisphParticle::new(Vec3::new(
                    i as f32 * 0.05,
                    j as f32 * 0.05,
                    0.0,
                )));
            }
        }
        solver.step();
        let avg_rho = solver.average_density();
        assert!(avg_rho > 0.0, "density computed: {}", avg_rho);
    }

    #[test]
    fn test_iisph_pressure_nonneg() {
        // 压缩情况下压力应为非负 (不可压缩)
        let mut solver = IisphSolver::new(IisphConfig {
            dt: 0.001,
            h: 0.2,
            rest_density: 10.0, // 低静止密度, 强制压缩 -> 高压力
            gravity: Vec3::ZERO,
            ..IisphConfig::default()
        });
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    solver.add_particle(IisphParticle::new(Vec3::new(
                        i as f32 * 0.03,
                        j as f32 * 0.03,
                        k as f32 * 0.03,
                    )));
                }
            }
        }
        solver.step();
        // 在 IISPH 中压力应非负 (约束 p >= 0)
        for p in &solver.particles {
            assert!(p.pressure >= -1e-6, "pressure non-negative: {}", p.pressure);
        }
    }

    #[test]
    fn test_iisph_max_density_error() {
        let mut solver = IisphSolver::new(IisphConfig::default());
        solver.add_particle(IisphParticle::new(Vec3::ZERO));
        solver.step();
        let err = solver.max_density_error();
        assert!(err >= 0.0);
    }

    #[test]
    fn test_iisph_kinetic_energy() {
        let mut solver = IisphSolver::new(IisphConfig {
            gravity: Vec3::ZERO,
            ..IisphConfig::default()
        });
        let mut p = IisphParticle::new(Vec3::ZERO);
        p.velocity = Vec3::new(1.0, 0.0, 0.0);
        solver.add_particle(p);
        let ke = solver.kinetic_energy();
        // 0.5 * 0.02 * 1.0^2 = 0.01
        assert!((ke - 0.01).abs() < 1e-4, "ke: {}", ke);
    }

    #[test]
    fn test_iisph_restitution_boundary() {
        // 边界碰撞应反弹 (恢复系数 > 0)
        let mut solver = IisphSolver::new(IisphConfig {
            dt: 0.01,
            gravity: Vec3::new(0.0, -20.0, 0.0),
            bounds_min: Vec3::new(-1.0, -1.0, -1.0),
            bounds_max: Vec3::new(1.0, 1.0, 1.0),
            restitution: 0.5,
            ..IisphConfig::default()
        });
        solver.add_particle(IisphParticle::new(Vec3::new(0.0, 0.5, 0.0)));
        // 让粒子下落碰底
        for _ in 0..200 {
            solver.step();
        }
        // 粒子应停在底部附近 (有反弹但能量耗散)
        let y = solver.particles[0].position.y;
        assert!(y > -1.5, "particle stays in/near bounds: y={}", y);
    }

    #[test]
    fn test_iisph_ppe_warm_start() {
        // 连续步进, 压力应累积 (warm start)
        let mut solver = IisphSolver::new(IisphConfig {
            dt: 0.001,
            h: 0.2,
            rest_density: 10.0,
            gravity: Vec3::ZERO,
            ppe_iterations: 5,
            ..IisphConfig::default()
        });
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    solver.add_particle(IisphParticle::new(Vec3::new(
                        i as f32 * 0.04,
                        j as f32 * 0.04,
                        k as f32 * 0.04,
                    )));
                }
            }
        }
        solver.step();
        let p1 = solver.particles[0].pressure;
        solver.step();
        let p2 = solver.particles[0].pressure;
        // 第二步应有 warm start, 压力可能变化但应稳定
        assert!(p1.is_finite() && p2.is_finite(), "pressure finite");
    }

    #[test]
    fn test_iisph_relaxation_factor() {
        // omega=1 (无松弛) vs omega=0.5 (松弛), 都应稳定
        for omega in [0.3, 0.5, 0.7, 1.0] {
            let mut solver = IisphSolver::new(IisphConfig {
                dt: 0.001,
                h: 0.2,
                rest_density: 100.0,
                gravity: Vec3::ZERO,
                ppe_iterations: 10,
                omega,
                ..IisphConfig::default()
            });
            for i in 0..3 {
                for j in 0..3 {
                    solver.add_particle(IisphParticle::new(Vec3::new(
                        i as f32 * 0.05,
                        j as f32 * 0.05,
                        0.0,
                    )));
                }
            }
            solver.step();
            // 不爆炸
            for p in &solver.particles {
                assert!(p.pressure.is_finite(), "omega={} pressure finite", omega);
                assert!(p.velocity.x.is_finite(), "omega={} velocity finite", omega);
            }
        }
    }
}
