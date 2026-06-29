//! Fast Mass Spring - 快速弹簧系统求解
//!
//! 基于:
//! - Liu, Chen, Arya, Kryo, Grinspun. "Fast Simulation of Mass-Spring
//!   Systems." ACM TOG (SIGGRAPH 2013), 32(6).
//!
//! 核心思想:
//! 1. 弹簧能量 E(x) = Σ (k/2)(|x_i - x_j| - L_0)² 是非线性的 (因 |.|)
//! 2. FMS 关键洞察: 固定弹簧方向 d = (x_i - x_j)/|x_i - x_j|
//!    固定 d 后, E 变成二次型: E ≈ Σ (k/2)(d·(x_i - x_j) - L_0)²
//! 3. 系统矩阵 A = M + h²L (L 固定方向后线性, 不再依赖 x)
//!    - A 在整个模拟中不变 (如果方向固定), 可预分解
//! 4. 块坐标下降: 交替更新方向 d 和位置 x
//!    - 每步: 更新 d (用预测位置), 求解 A x = b
//! 5. 复杂度: 预处理 O(n) + 每步 O(n) (vs 牛顿法 O(n³) 或 CG O(n²))
//!
//! 与 Projective Dynamics 的关系:
//! - PD 是 FMS 的推广 (用投影代替方向固定)
//! - FMS 的弹簧约束是 PD 弹簧约束的特例

use serde::{Deserialize, Serialize};
use glam::Vec3;

// ============================================================
// 配置
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FmsConfig {
    pub dt: f32,
    pub gravity: Vec3,
    pub damping: f32,
    /// 块坐标下降迭代次数 (每步)
    pub num_iters: usize,
    /// Jacobi 求解迭代次数 (每次块坐标)
    pub jacobi_iters: usize,
}

impl Default for FmsConfig {
    fn default() -> Self {
        Self {
            dt: 1.0 / 60.0,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            damping: 0.99,
            num_iters: 4,
            jacobi_iters: 10,
        }
    }
}

// ============================================================
// 粒子和弹簧
// ============================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FmsParticle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub inv_mass: f32,
    pub pinned: bool,
}

impl FmsParticle {
    pub fn new(position: Vec3, mass: f32) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            inv_mass: if mass > 0.0 { 1.0 / mass } else { 0.0 },
            pinned: false,
        }
    }

    pub fn pinned(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            inv_mass: 0.0,
            pinned: true,
        }
    }

    #[inline]
    pub fn is_dynamic(&self) -> bool {
        self.inv_mass > 0.0 && !self.pinned
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FmsSpring {
    pub p0: usize,
    pub p1: usize,
    pub rest_length: f32,
    pub stiffness: f32,
}

impl FmsSpring {
    pub fn new(p0: usize, p1: usize, rest_length: f32, stiffness: f32) -> Self {
        Self { p0, p1, rest_length, stiffness }
    }
}

// ============================================================
// 求解器
// ============================================================

pub struct FmsSolver {
    pub config: FmsConfig,
    pub particles: Vec<FmsParticle>,
    pub springs: Vec<FmsSpring>,
    /// 预计算的 A 矩阵对角块 (每个粒子的 3x3 对角块)
    /// A_ii = M_ii + h² * Σ_j k_ij * d_ij d_ij^T
    /// 由于 d 依赖位置, 我们在每次 step 时重新计算 (简化版)
    /// 完整 FMS 会预分解, 但需要固定方向
    a_diag: Vec<glam::Mat3>,
    /// 当前弹簧方向 (每次更新)
    directions: Vec<Vec3>,
}

impl FmsSolver {
    pub fn new(config: FmsConfig) -> Self {
        Self {
            config,
            particles: Vec::new(),
            springs: Vec::new(),
            a_diag: Vec::new(),
            directions: Vec::new(),
        }
    }

    pub fn add_particle(&mut self, p: FmsParticle) -> usize {
        let idx = self.particles.len();
        self.particles.push(p);
        idx
    }

    pub fn add_spring(&mut self, s: FmsSpring) {
        self.springs.push(s);
        self.directions.push(Vec3::new(1.0, 0.0, 0.0));
    }

    /// 添加弹簧 (自动计算 rest_length)
    pub fn connect(&mut self, p0: usize, p1: usize, stiffness: f32) {
        let rest = (self.particles[p0].position - self.particles[p1].position).length();
        self.add_spring(FmsSpring::new(p0, p1, rest, stiffness));
    }

    /// 单步时间步进
    pub fn step(&mut self) {
        let dt = self.config.dt;
        let n = self.particles.len();
        if n == 0 {
            return;
        }

        // 1. 预测位置 (外力 + 速度)
        let mut predicted = vec![Vec3::ZERO; n];
        for (i, p) in self.particles.iter().enumerate() {
            if p.is_dynamic() {
                predicted[i] = p.position + p.velocity * dt * self.config.damping
                    + self.config.gravity * dt * dt;
            } else {
                predicted[i] = p.position;
            }
        }

        // 2. 初始化 A 矩阵对角 (M + h²L 的对角部分)
        self.a_diag = (0..n)
            .map(|i| {
                let mass = if self.particles[i].is_dynamic() {
                    1.0 / self.particles[i].inv_mass
                } else {
                    1e10 // 固定粒子大质量
                };
                glam::Mat3::IDENTITY * mass
            })
            .collect();

        // 3. 块坐标下降: 交替更新方向和求解位置
        let mut x = predicted.clone();
        for _ in 0..self.config.num_iters {
            // 3a. 更新弹簧方向 (用当前位置)
            for (s, dir) in self.springs.iter().zip(self.directions.iter_mut()) {
                let d = x[s.p1] - x[s.p0];
                let len = d.length().max(1e-10);
                *dir = d / len;
            }

            // 3b. 更新 A 对角 (A_ii += h² * Σ k * d d^T)
            let h2 = dt * dt;
            self.a_diag = (0..n)
                .map(|i| {
                    let mass = if self.particles[i].is_dynamic() {
                        1.0 / self.particles[i].inv_mass
                    } else {
                        1e10
                    };
                    let mut a_ii = glam::Mat3::IDENTITY * mass;
                    for (s, d) in self.springs.iter().zip(self.directions.iter()) {
                        if s.p0 == i || s.p1 == i {
                            // d d^T 贡献 (符号: p0 是 -, p1 是 +, 但 d d^T 相同)
                            let ddt = glam::Mat3::from_cols(
                                Vec3::new(d.x * d.x, d.x * d.y, d.x * d.z),
                                Vec3::new(d.y * d.x, d.y * d.y, d.y * d.z),
                                Vec3::new(d.z * d.x, d.z * d.y, d.z * d.z),
                            );
                            a_ii += ddt * (h2 * s.stiffness);
                        }
                    }
                    a_ii
                })
                .collect();

            // 3c. 构建右侧 b = M*y + h² * Σ k * L_0 * d * (sign)
            // 对弹簧 (i,j): 贡献 +k*L_0*d 到 b_i, -k*L_0*d 到 b_j
            let mut b = vec![Vec3::ZERO; n];
            for i in 0..n {
                let mass = if self.particles[i].is_dynamic() {
                    1.0 / self.particles[i].inv_mass
                } else {
                    0.0
                };
                b[i] = predicted[i] * mass;
            }
            for (s, d) in self.springs.iter().zip(self.directions.iter()) {
                let force = d * (s.stiffness * s.rest_length);
                b[s.p0] += force * h2;
                b[s.p1] -= force * h2;
            }

            // 3d. Jacobi 迭代求解 A x = b
            // x_i = (b_i + Σ_{j neighbor} h²*k*d*d*x_j) / A_ii
            // 由于 A 是块对角 + 非对角, 完整 Jacobi 需要非对角项
            // 简化: 用对角 A (忽略非对角), 多次迭代
            for _ in 0..self.config.jacobi_iters {
                let mut x_new = vec![Vec3::ZERO; n];
                for i in 0..n {
                    if !self.particles[i].is_dynamic() {
                        x_new[i] = self.particles[i].position;
                        continue;
                    }
                    // 非对角贡献: Σ h²*k*d*(d·x_j) (从 i 到 j 的弹簧)
                    let mut off_diag = Vec3::ZERO;
                    for (s, d) in self.springs.iter().zip(self.directions.iter()) {
                        if s.p0 == i {
                            // 弹簧 (i, j=p1): 贡献 h²*k*d*(d·x_j)
                            off_diag += d * (d.dot(x[s.p1])) * (h2 * s.stiffness);
                        } else if s.p1 == i {
                            // 弹簧 (j=p0, i): 贡献 h²*k*d*(d·x_j) (符号相同, d d^T)
                            off_diag += d * (d.dot(x[s.p0])) * (h2 * s.stiffness);
                        }
                    }
                    // x_i = A_ii^-1 * (b_i + off_diag)
                    let rhs = b[i] + off_diag;
                    let a_inv = inverse_mat3(self.a_diag[i]);
                    x_new[i] = a_inv * rhs;
                }
                x = x_new;
            }
        }

        // 4. 更新速度和位置
        let dt_inv = 1.0 / dt.max(1e-10);
        for (i, p) in self.particles.iter_mut().enumerate() {
            if p.is_dynamic() {
                p.velocity = (x[i] - p.position) * dt_inv;
                p.position = x[i];
            }
        }
    }

    /// 总动能
    pub fn kinetic_energy(&self) -> f32 {
        let mut ke = 0.0;
        for p in &self.particles {
            if p.is_dynamic() {
                let m = 1.0 / p.inv_mass;
                ke += 0.5 * m * p.velocity.length_squared();
            }
        }
        ke
    }

    /// 总弹簧势能
    pub fn spring_energy(&self) -> f32 {
        let mut pe = 0.0;
        for s in &self.springs {
            let d = self.particles[s.p1].position - self.particles[s.p0].position;
            let len = d.length();
            let stretch = len - s.rest_length;
            pe += 0.5 * s.stiffness * stretch * stretch;
        }
        pe
    }

    /// 总能量 (动能 + 势能)
    pub fn total_energy(&self) -> f32 {
        self.kinetic_energy() + self.spring_energy()
    }

    /// 最大速度 (稳定性监测)
    pub fn max_velocity(&self) -> f32 {
        self.particles.iter()
            .filter(|p| p.is_dynamic())
            .map(|p| p.velocity.length())
            .fold(0.0f32, f32::max)
    }
}

/// 3x3 矩阵求逆
fn inverse_mat3(m: glam::Mat3) -> glam::Mat3 {
    let det = m.determinant();
    if det.abs() < 1e-10 {
        return glam::Mat3::IDENTITY;
    }
    m.inverse()
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fms_config_default() {
        let c = FmsConfig::default();
        assert!(c.dt > 0.0);
        assert!(c.num_iters > 0);
        assert!(c.jacobi_iters > 0);
    }

    #[test]
    fn test_fms_particle_creation() {
        let p = FmsParticle::new(Vec3::new(1.0, 2.0, 3.0), 2.0);
        assert_eq!(p.position, Vec3::new(1.0, 2.0, 3.0));
        assert!((p.inv_mass - 0.5).abs() < 1e-6);
        assert!(p.is_dynamic());

        let pinned = FmsParticle::pinned(Vec3::ZERO);
        assert!(!pinned.is_dynamic());
    }

    #[test]
    fn test_fms_spring_creation() {
        let s = FmsSpring::new(0, 1, 1.0, 100.0);
        assert_eq!(s.p0, 0);
        assert_eq!(s.p1, 1);
        assert!((s.rest_length - 1.0).abs() < 1e-6);
        assert!((s.stiffness - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_fms_solver_creation() {
        let solver = FmsSolver::new(FmsConfig::default());
        assert!(solver.particles.is_empty());
        assert!(solver.springs.is_empty());
    }

    #[test]
    fn test_fms_free_fall() {
        // 单粒子自由落体 (无弹簧)
        let mut solver = FmsSolver::new(FmsConfig {
            dt: 0.01,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            num_iters: 1,
            ..FmsConfig::default()
        });
        solver.add_particle(FmsParticle::new(Vec3::new(0.0, 10.0, 0.0), 1.0));
        solver.step();
        // 应下落
        assert!(solver.particles[0].position.y < 10.0, "should fall: y={}", solver.particles[0].position.y);
        assert!(solver.particles[0].velocity.y < 0.0, "downward velocity: {}", solver.particles[0].velocity.y);
    }

    #[test]
    fn test_fms_pinned_particle() {
        // 固定粒子不动
        let mut solver = FmsSolver::new(FmsConfig {
            dt: 0.01,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            num_iters: 1,
            ..FmsConfig::default()
        });
        let p0 = solver.add_particle(FmsParticle::pinned(Vec3::new(0.0, 5.0, 0.0)));
        let p1 = solver.add_particle(FmsParticle::new(Vec3::new(0.0, 4.0, 0.0), 1.0));
        solver.connect(p0, p1, 100.0);
        solver.step();
        // p0 应保持位置
        assert!((solver.particles[p0].position - Vec3::new(0.0, 5.0, 0.0)).length() < 1e-4, "pinned stays");
    }

    #[test]
    fn test_fms_spring_rest_length() {
        // 弹簧在静止长度时无变形
        let mut solver = FmsSolver::new(FmsConfig {
            dt: 0.01,
            gravity: Vec3::ZERO, // 无重力
            num_iters: 10,
            jacobi_iters: 30,
            ..FmsConfig::default()
        });
        let p0 = solver.add_particle(FmsParticle::pinned(Vec3::new(0.0, 0.0, 0.0)));
        let p1 = solver.add_particle(FmsParticle::new(Vec3::new(1.0, 0.0, 0.0), 1.0));
        solver.connect(p0, p1, 100.0);
        // 静止长度 = 1, 无外力, 应保持
        for _ in 0..5 {
            solver.step();
        }
        let d = solver.particles[p1].position - solver.particles[p0].position;
        assert!(d.length() < 1.05, "spring at rest: len={}", d.length());
    }

    #[test]
    fn test_fms_spring_stretch() {
        // 弹簧被拉伸后应回缩
        let mut solver = FmsSolver::new(FmsConfig {
            dt: 0.001,
            gravity: Vec3::ZERO,
            num_iters: 20,
            jacobi_iters: 50,
            ..FmsConfig::default()
        });
        let p0 = solver.add_particle(FmsParticle::pinned(Vec3::new(0.0, 0.0, 0.0)));
        let p1 = solver.add_particle(FmsParticle::new(Vec3::new(2.0, 0.0, 0.0), 1.0));
        // rest_length = 2 (初始), 但我们手动设置 rest = 1
        solver.add_spring(FmsSpring::new(p0, p1, 1.0, 500.0));
        // 初始拉伸 (长度 2, rest 1), 应回缩
        for _ in 0..50 {
            solver.step();
        }
        let d = solver.particles[p1].position - solver.particles[p0].position;
        // 应回缩到接近 1 (rest length)
        assert!(d.length() < 1.5, "spring should contract: len={}", d.length());
    }

    #[test]
    fn test_fms_pendulum() {
        // 单摆: 一端固定, 另一端自由摆动
        let mut solver = FmsSolver::new(FmsConfig {
            dt: 0.005,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            num_iters: 10,
            jacobi_iters: 30,
            ..FmsConfig::default()
        });
        let p0 = solver.add_particle(FmsParticle::pinned(Vec3::new(0.0, 5.0, 0.0)));
        let p1 = solver.add_particle(FmsParticle::new(Vec3::new(1.0, 5.0, 0.0), 1.0));
        solver.connect(p0, p1, 200.0);
        // 跑 100 步, 应摆动
        let mut max_v: f32 = 0.0;
        for _ in 0..100 {
            solver.step();
            max_v = max_v.max(solver.particles[p1].velocity.length());
        }
        // 应有运动
        assert!(max_v > 0.1, "pendulum should move: max_v={}", max_v);
        // 应保持接近绳长 (1.0)
        let d = solver.particles[p1].position - solver.particles[p0].position;
        assert!(d.length() < 1.2, "pendulum length bounded: {}", d.length());
    }

    #[test]
    fn test_fms_energy_positive() {
        let mut solver = FmsSolver::new(FmsConfig {
            dt: 0.01,
            gravity: Vec3::ZERO,
            num_iters: 5,
            ..FmsConfig::default()
        });
        let p0 = solver.add_particle(FmsParticle::pinned(Vec3::ZERO));
        let p1 = solver.add_particle(FmsParticle::new(Vec3::new(1.5, 0.0, 0.0), 1.0));
        solver.add_spring(FmsSpring::new(p0, p1, 1.0, 100.0));
        let pe = solver.spring_energy();
        assert!(pe > 0.0, "stretched spring has PE: {}", pe);
    }

    #[test]
    fn test_fms_chain_stability() {
        // 链式弹簧: 多个粒子连成链, 应稳定
        let mut solver = FmsSolver::new(FmsConfig {
            dt: 0.005,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            num_iters: 8,
            jacobi_iters: 20,
            ..FmsConfig::default()
        });
        // 5 个粒子的链, 第一个固定
        let mut prev = solver.add_particle(FmsParticle::pinned(Vec3::new(0.0, 5.0, 0.0)));
        for i in 1..5 {
            let p = solver.add_particle(FmsParticle::new(
                Vec3::new(i as f32 * 0.5, 5.0, 0.0),
                1.0,
            ));
            solver.connect(prev, p, 500.0);
            prev = p;
        }
        // 跑 200 步, 不应爆炸
        for _ in 0..200 {
            solver.step();
        }
        let max_v = solver.max_velocity();
        assert!(max_v < 50.0, "chain stable: max_v={}", max_v);
        // 所有粒子应在合理范围内
        for p in &solver.particles {
            assert!(p.position.y > -10.0, "particle y bounded: {}", p.position.y);
            assert!(p.position.y < 10.0, "particle y not flying: {}", p.position.y);
        }
    }

    #[test]
    fn test_fms_mesh_2d() {
        // 2D 网格弹簧布 (简化), 一角固定
        let mut solver = FmsSolver::new(FmsConfig {
            dt: 0.005,
            gravity: Vec3::new(0.0, -5.0, 0.0),
            num_iters: 8,
            jacobi_iters: 20,
            ..FmsConfig::default()
        });
        let nx = 4;
        let ny = 4;
        let spacing = 0.2;
        let mut idx = vec![0usize; nx * ny];
        for j in 0..ny {
            for i in 0..nx {
                let p = if i == 0 && j == 0 {
                    FmsParticle::pinned(Vec3::new(i as f32 * spacing, 2.0 - j as f32 * spacing, 0.0))
                } else {
                    FmsParticle::new(Vec3::new(i as f32 * spacing, 2.0 - j as f32 * spacing, 0.0), 0.1)
                };
                idx[j * nx + i] = solver.add_particle(p);
            }
        }
        // 水平弹簧
        for j in 0..ny {
            for i in 0..nx-1 {
                solver.connect(idx[j*nx+i], idx[j*nx+i+1], 200.0);
            }
        }
        // 垂直弹簧
        for j in 0..ny-1 {
            for i in 0..nx {
                solver.connect(idx[j*nx+i], idx[(j+1)*nx+i], 200.0);
            }
        }
        // 跑 100 步
        for _ in 0..100 {
            solver.step();
        }
        // 不应爆炸
        let max_v = solver.max_velocity();
        assert!(max_v < 30.0, "cloth stable: max_v={}", max_v);
        // 固定点不动
        let pinned = solver.particles[idx[0]].position;
        assert!((pinned - Vec3::ZERO - Vec3::new(0.0, 2.0, 0.0)).length() < 1e-3, "pinned stays");
    }
}
