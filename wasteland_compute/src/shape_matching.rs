//! Shape Matching - 形状匹配可变形体
//!
//! 基于:
//! - Muller, Heidelberger, Teschner, Gross. "Meshless Deformations Based
//!   on Shape Matching." ACM TOG (SIGGRAPH 2005), 24(3).
//! - Muller, Chentanez. "Solid Simulation with Oriented Particles." ACM TOG 2011.
//!
//! 核心思想:
//! 1. 物体由粒子组成, 每个粒子有初始位置 x_i^0 和质量 m_i
//! 2. 每步: 预测粒子位置 x_i* (积分外力)
//! 3. 找最佳刚体变换 (R, c) 匹配预测位置:
//!    min Σ m_i |R q_i + c - p_i|²
//!    其中 q_i = x_i^0 - c^0 (初始相对), p_i = x_i* - c (预测相对)
//! 4. 目标位置: g_i = R q_i + c
//! 5. 位置修正: x_i_new = x_i* + α (g_i - x_i*)
//!    α ∈ [0,1]: 0=不修正(流体), 1=完全刚体
//! 6. 速度更新: v_i = (x_i_new - x_i) / dt
//!
//! 优点:
//! - 自然处理大变形、破碎、流变
//! - 无需网格拓扑 (meshless)
//! - α 可调硬度 (0=流体, 1=刚体, 中间=软体)
//! - 比 FEM 简单, 比 PBD 弹簧稳定

use serde::{Deserialize, Serialize};
use glam::{Mat3, Vec3};

// ============================================================
// 配置
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmConfig {
    pub dt: f32,
    pub gravity: Vec3,
    pub damping: f32,
    /// 形状匹配硬度 (0=流体, 1=刚体)
    pub alpha: f32,
    /// 极分解迭代次数 (3x3 矩阵)
    pub polaris_iters: usize,
    /// 弹性恢复 (当 α<1 时, 长期恢复到初始形状)
    pub beta: f32,
}

impl Default for SmConfig {
    fn default() -> Self {
        Self {
            dt: 1.0 / 60.0,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            damping: 0.99,
            alpha: 0.8,
            polaris_iters: 5,
            beta: 0.0,
        }
    }
}

// ============================================================
// 粒子和簇
// ============================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SmParticle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub inv_mass: f32,
    /// 初始位置 (相对簇质心)
    pub rest_pos: Vec3,
    pub pinned: bool,
}

impl SmParticle {
    pub fn new(position: Vec3, mass: f32) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            inv_mass: if mass > 0.0 { 1.0 / mass } else { 0.0 },
            rest_pos: Vec3::ZERO, // 稍后由 cluster 设置
            pinned: false,
        }
    }

    pub fn pinned(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            inv_mass: 0.0,
            rest_pos: Vec3::ZERO,
            pinned: true,
        }
    }

    #[inline]
    pub fn is_dynamic(&self) -> bool {
        self.inv_mass > 0.0 && !self.pinned
    }
}

/// 形状匹配簇 (一组粒子)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmCluster {
    /// 簇内粒子索引
    pub particle_indices: Vec<usize>,
    /// 初始质心
    pub rest_centroid: Vec3,
    /// 总质量倒数
    pub total_inv_mass: f32,
}

impl SmCluster {
    pub fn new(indices: Vec<usize>) -> Self {
        Self {
            particle_indices: indices,
            rest_centroid: Vec3::ZERO,
            total_inv_mass: 0.0,
        }
    }
}

// ============================================================
// 求解器
// ============================================================

pub struct SmSolver {
    pub config: SmConfig,
    pub particles: Vec<SmParticle>,
    pub clusters: Vec<SmCluster>,
    /// 边界盒
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    pub restitution: f32,
}

impl SmSolver {
    pub fn new(config: SmConfig) -> Self {
        Self {
            config,
            particles: Vec::new(),
            clusters: Vec::new(),
            bounds_min: Vec3::new(-10.0, -10.0, -10.0),
            bounds_max: Vec3::new(10.0, 10.0, 10.0),
            restitution: 0.3,
        }
    }

    pub fn add_particle(&mut self, p: SmParticle) -> usize {
        let idx = self.particles.len();
        self.particles.push(p);
        idx
    }

    pub fn add_cluster(&mut self, mut cluster: SmCluster) -> usize {
        // 计算初始质心和相对位置
        let mut total_mass = 0.0;
        let mut centroid = Vec3::ZERO;
        for &i in &cluster.particle_indices {
            let m = 1.0 / self.particles[i].inv_mass.max(1e-10);
            total_mass += m;
            centroid += self.particles[i].position * m;
        }
        if total_mass > 0.0 {
            centroid /= total_mass;
        }
        cluster.rest_centroid = centroid;
        cluster.total_inv_mass = if total_mass > 0.0 { 1.0 / total_mass } else { 0.0 };
        // 设置每个粒子的 rest_pos (相对质心)
        for &i in &cluster.particle_indices {
            self.particles[i].rest_pos = self.particles[i].position - centroid;
        }
        let idx = self.clusters.len();
        self.clusters.push(cluster);
        idx
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

        // 2. 对每个簇做形状匹配
        let alpha = self.config.alpha;
        let beta = self.config.beta;
        for cluster in &self.clusters {
            if cluster.particle_indices.is_empty() {
                continue;
            }
            // 计算预测质心
            let mut total_mass = 0.0;
            let mut centroid = Vec3::ZERO;
            for &i in &cluster.particle_indices {
                let m = 1.0 / self.particles[i].inv_mass.max(1e-10);
                if self.particles[i].is_dynamic() {
                    total_mass += m;
                    centroid += predicted[i] * m;
                }
            }
            if total_mass < 1e-10 {
                continue;
            }
            centroid /= total_mass;

            // 计算 A = Σ m_i p_i q_i^T (3x3 矩阵)
            let mut a_mat = Mat3::ZERO;
            for &i in &cluster.particle_indices {
                if !self.particles[i].is_dynamic() {
                    continue;
                }
                let m = 1.0 / self.particles[i].inv_mass.max(1e-10);
                let p = predicted[i] - centroid; // 预测相对
                let q = self.particles[i].rest_pos; // 初始相对
                // A += m * p * q^T
                // p * q^T 是 outer product (3x3)
                a_mat += Mat3::from_cols(
                    Vec3::new(p.x * q.x, p.x * q.y, p.x * q.z),
                    Vec3::new(p.y * q.x, p.y * q.y, p.y * q.z),
                    Vec3::new(p.z * q.x, p.z * q.y, p.z * q.z),
                ) * m;
            }

            // 极分解: A = R S, 提取 R (旋转部分)
            let r = polar_decomposition(a_mat, self.config.polaris_iters);

            // 计算目标位置 g_i = R q_i + c, 应用位置修正
            for &i in &cluster.particle_indices {
                if !self.particles[i].is_dynamic() {
                    continue;
                }
                let q = self.particles[i].rest_pos;
                let g = r * q + centroid;
                // 形状匹配修正: x_i_new = x_i* + α (g_i - x_i*)
                let correction = (g - predicted[i]) * alpha;
                predicted[i] += correction;
                // 弹性恢复 (长期恢复到初始形状)
                if beta > 0.0 {
                    let rest_world = r * q + cluster.rest_centroid;
                    let cur = predicted[i];
                    predicted[i] = cur + (rest_world - cur) * beta * dt;
                }
            }
        }

        // 3. 边界约束
        for (i, p) in self.particles.iter_mut().enumerate() {
            if !p.is_dynamic() {
                continue;
            }
            for axis in 0..3 {
                if predicted[i][axis] < self.bounds_min[axis] {
                    predicted[i][axis] = self.bounds_min[axis];
                    if p.velocity[axis] < 0.0 {
                        p.velocity[axis] = -p.velocity[axis] * self.restitution;
                    }
                } else if predicted[i][axis] > self.bounds_max[axis] {
                    predicted[i][axis] = self.bounds_max[axis];
                    if p.velocity[axis] > 0.0 {
                        p.velocity[axis] = -p.velocity[axis] * self.restitution;
                    }
                }
            }
        }

        // 4. 更新速度和位置
        let dt_inv = 1.0 / dt.max(1e-10);
        for (i, p) in self.particles.iter_mut().enumerate() {
            if p.is_dynamic() {
                p.velocity = (predicted[i] - p.position) * dt_inv;
                p.position = predicted[i];
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

    /// 最大速度
    pub fn max_velocity(&self) -> f32 {
        self.particles.iter()
            .filter(|p| p.is_dynamic())
            .map(|p| p.velocity.length())
            .fold(0.0f32, f32::max)
    }
}

/// 3x3 矩阵极分解 (提取旋转部分 R)
/// A = R S, 其中 R 是旋转, S 是对称正定
/// 用 Higham 迭代: R_{k+1} = 0.5 * (R_k + (R_k^T)^{-1})
fn polar_decomposition(a: Mat3, iters: usize) -> Mat3 {
    // 加小扰动避免奇异 (共面/共线粒子情况)
    let a = a + Mat3::IDENTITY * 1e-6;
    let mut r = a;
    for _ in 0..iters {
        let r_t = r.transpose();
        let r_t_inv = match inverse_mat3(r_t) {
            Some(inv) => inv,
            None => break,
        };
        let r_new = (r + r_t_inv) * 0.5;
        let diff = (r_new - r).abs();
        let max_diff = diff.x_axis.length().max(diff.y_axis.length()).max(diff.z_axis.length());
        r = r_new;
        if max_diff < 1e-7 {
            break;
        }
    }
    // 归一化: 确保是旋转 (det = +1)
    let det = r.determinant();
    if det.abs() < 1e-10 {
        return Mat3::IDENTITY;
    }
    if det < 0.0 {
        r = Mat3::from_cols(-r.x_axis, r.y_axis, r.z_axis);
    }
    // 归一化列向量 (确保正交)
    let x = r.x_axis.normalize_or_zero();
    let y = (r.y_axis - x * x.dot(r.y_axis)).normalize_or_zero();
    let z = x.cross(y);
    Mat3::from_cols(x, y, z)
}

/// 3x3 矩阵求逆 (返回 None 如果奇异)
fn inverse_mat3(m: Mat3) -> Option<Mat3> {
    let det = m.determinant();
    if det.abs() < 1e-12 {
        return None;
    }
    Some(m.inverse())
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sm_config_default() {
        let c = SmConfig::default();
        assert!(c.dt > 0.0);
        assert!(c.alpha >= 0.0 && c.alpha <= 1.0);
    }

    #[test]
    fn test_sm_particle_creation() {
        let p = SmParticle::new(Vec3::new(1.0, 2.0, 3.0), 2.0);
        assert_eq!(p.position, Vec3::new(1.0, 2.0, 3.0));
        assert!((p.inv_mass - 0.5).abs() < 1e-6);
        assert!(p.is_dynamic());
    }

    #[test]
    fn test_sm_pinned_particle() {
        let p = SmParticle::pinned(Vec3::ZERO);
        assert!(!p.is_dynamic());
    }

    #[test]
    fn test_polar_decomposition_identity() {
        let r = polar_decomposition(Mat3::IDENTITY, 5);
        assert!((r - Mat3::IDENTITY).abs().x_axis.length() < 1e-4, "identity");
    }

    #[test]
    fn test_polar_decomposition_rotation() {
        // 已知旋转矩阵, 极分解应返回自身
        let rot = Mat3::from_rotation_y(0.5);
        let r = polar_decomposition(rot, 10);
        let diff = (r - rot).abs();
        let max_diff = diff.x_axis.length().max(diff.y_axis.length()).max(diff.z_axis.length());
        assert!(max_diff < 1e-4, "rotation preserved: {}", max_diff);
    }

    #[test]
    fn test_polar_decomposition_scaled() {
        // 旋转 + 缩放, 应只提取旋转
        let rot = Mat3::from_rotation_z(1.0);
        let scaled = rot * Mat3::from_diagonal(Vec3::new(2.0, 2.0, 2.0));
        let r = polar_decomposition(scaled, 10);
        let diff = (r - rot).abs();
        let max_diff = diff.x_axis.length().max(diff.y_axis.length()).max(diff.z_axis.length());
        assert!(max_diff < 1e-3, "extract rotation from scaled: {}", max_diff);
    }

    #[test]
    fn test_sm_free_fall() {
        let mut solver = SmSolver::new(SmConfig {
            dt: 0.01,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            alpha: 0.0, // 无形状匹配 (自由粒子)
            ..SmConfig::default()
        });
        solver.add_particle(SmParticle::new(Vec3::new(0.0, 10.0, 0.0), 1.0));
        solver.step();
        assert!(solver.particles[0].position.y < 10.0, "should fall");
    }

    #[test]
    fn test_sm_rigid_body_preserves_shape() {
        // α=1 (完全刚体), 形状应保持
        let mut solver = SmSolver::new(SmConfig {
            dt: 0.005,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            alpha: 1.0,
            polaris_iters: 10,
            ..SmConfig::default()
        });
        // 4 个粒子组成正方形
        let p0 = solver.add_particle(SmParticle::new(Vec3::new(-1.0, 0.0, 0.0), 1.0));
        let p1 = solver.add_particle(SmParticle::new(Vec3::new(1.0, 0.0, 0.0), 1.0));
        let p2 = solver.add_particle(SmParticle::new(Vec3::new(0.0, 1.0, 0.0), 1.0));
        let p3 = solver.add_particle(SmParticle::new(Vec3::new(0.0, -1.0, 0.0), 1.0));
        solver.add_cluster(SmCluster::new(vec![p0, p1, p2, p3]));
        let initial_d01 = (solver.particles[p0].position - solver.particles[p1].position).length();
        // 跑 50 步
        for _ in 0..50 {
            solver.step();
        }
        let final_d01 = (solver.particles[p0].position - solver.particles[p1].position).length();
        // 刚体应保持距离 (允许小误差)
        assert!((final_d01 - initial_d01).abs() < 0.2, "rigid preserves shape: init={} final={}", initial_d01, final_d01);
    }

    #[test]
    fn test_sm_soft_body_deforms() {
        // α<1 (软体), 形状可变形
        let mut solver = SmSolver::new(SmConfig {
            dt: 0.005,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            alpha: 0.3, // 软
            polaris_iters: 10,
            ..SmConfig::default()
        });
        let p0 = solver.add_particle(SmParticle::pinned(Vec3::new(-1.0, 5.0, 0.0)));
        let p1 = solver.add_particle(SmParticle::new(Vec3::new(1.0, 5.0, 0.0), 1.0));
        let p2 = solver.add_particle(SmParticle::new(Vec3::new(0.0, 6.0, 0.0), 1.0));
        solver.add_cluster(SmCluster::new(vec![p0, p1, p2]));
        // 加边界 (地面) 防止无限下落
        solver.bounds_min = Vec3::new(-5.0, 0.0, -5.0);
        solver.bounds_max = Vec3::new(5.0, 10.0, 5.0);
        let initial_d = (solver.particles[p0].position - solver.particles[p1].position).length();
        // 跑 100 步
        for _ in 0..100 {
            solver.step();
        }
        let final_d = (solver.particles[p0].position - solver.particles[p1].position).length();
        // 软体距离可能变化 (但应有界, 不爆炸)
        assert!(final_d < 10.0, "soft body bounded: {}", final_d);
        assert!((final_d - initial_d).abs() < 8.0, "soft body deformation bounded: init={} final={}", initial_d, final_d);
    }

    #[test]
    fn test_sm_boundary() {
        let mut solver = SmSolver::new(SmConfig {
            dt: 0.05,
            gravity: Vec3::new(0.0, -20.0, 0.0),
            alpha: 0.0,
            ..SmConfig::default()
        });
        solver.bounds_min = Vec3::new(-1.0, -1.0, -1.0);
        solver.bounds_max = Vec3::new(1.0, 1.0, 1.0);
        solver.add_particle(SmParticle::new(Vec3::new(0.0, 0.5, 0.0), 1.0));
        for _ in 0..20 {
            solver.step();
        }
        let p = &solver.particles[0];
        assert!(p.position.y >= -1.01, "y >= min: {}", p.position.y);
        assert!(p.position.y <= 1.01, "y <= max: {}", p.position.y);
    }

    #[test]
    fn test_sm_stability() {
        // 长时间稳定性
        let mut solver = SmSolver::new(SmConfig {
            dt: 0.005,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            alpha: 0.7,
            polaris_iters: 8,
            ..SmConfig::default()
        });
        // 8 个粒子立方体
        let offsets = [
            [-0.5, -0.5, -0.5], [0.5, -0.5, -0.5],
            [-0.5, 0.5, -0.5], [0.5, 0.5, -0.5],
            [-0.5, -0.5, 0.5], [0.5, -0.5, 0.5],
            [-0.5, 0.5, 0.5], [0.5, 0.5, 0.5],
        ];
        let mut indices = Vec::new();
        for o in &offsets {
            let i = solver.add_particle(SmParticle::new(
                Vec3::new(o[0], o[1] + 5.0, o[2]),
                1.0,
            ));
            indices.push(i);
        }
        solver.add_cluster(SmCluster::new(indices));
        solver.bounds_min = Vec3::new(-5.0, -5.0, -5.0);
        solver.bounds_max = Vec3::new(5.0, 5.0, 5.0);
        // 跑 300 步
        for _ in 0..300 {
            solver.step();
        }
        let max_v = solver.max_velocity();
        assert!(max_v < 50.0, "stable: max_v={}", max_v);
        // 粒子应在边界内
        for p in &solver.particles {
            assert!(p.position.y >= -5.01, "particle in bounds: y={}", p.position.y);
            assert!(p.position.y <= 5.01, "particle in bounds: y={}", p.position.y);
        }
    }

    #[test]
    fn test_sm_kinetic_energy() {
        let mut solver = SmSolver::new(SmConfig::default());
        let p = solver.add_particle(SmParticle::new(Vec3::ZERO, 1.0));
        solver.particles[p].velocity = Vec3::new(1.0, 0.0, 0.0);
        let ke = solver.kinetic_energy();
        assert!(ke > 0.0);
    }
}
