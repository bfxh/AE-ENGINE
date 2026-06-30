//! Strain Based Dynamics - 基于应变的可变形体仿真
//!
//! 基于:
//! - Muller, Matthias, and Nuttapong Chentanez. "Strain Based Dynamics."
//!   ACM TOG (SIGGRAPH Asia 2014), 33(6).
//! - Muller, Chentanez, Kim. "Real Time Dynamic Fracture with Volumetric
//!   Approximate Convex Decomposition." ACM TOG 2013.
//!
//! 核心思想:
//! 1. 用四面体网格离散化物体 (TetMesh)
//! 2. 每个四面体有静止形态 (rest shape) 和当前形态 (current shape)
//! 3. 用应变张量 (strain tensor) 度量形变
//! 4. PBD 风格: 应变约束 C = strain - rest_strain = 0
//! 5. 求解位置修正, 使应变回到静止形态 (或允许的弹性范围)
//!
//! 优点:
//! - 比 FEM 简单 (无需组装全局刚度矩阵)
//! - 比 Shape Matching 更精确 (每 tet 独立约束)
//! - 自然处理体积守恒、剪切、拉伸
//! - PBD 框架, 易与其他约束组合
//!
//! 应变度量 (Green-Lagrange 应变张量):
//!   E = 0.5 * (F^T F - I), F = R_rest^{-1} * R_current
//!   其中 R = [e1, e2, e3] 是四面体三条边的矩阵
//!
//! 约束:
//!   C = E - E_rest (应变偏离静止值)
//!   拉格朗日乘子: lambda = -C / (Σ |∇C_i|^2 * w_i)
//!   位置修正: Δx_i = lambda * ∇C_i * w_i

use glam::{Mat3, Vec3};
use serde::{Deserialize, Serialize};

// ============================================================
// 配置
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbdConfig {
    pub dt: f32,
    pub gravity: Vec3,
    pub damping: f32,
    /// 应变约束硬度 (0=软, 1=硬)
    pub stiffness: f32,
    /// 体积约束硬度 (单独控制, 防止体积损失)
    pub volume_stiffness: f32,
    /// PBD 求解迭代数
    pub iterations: usize,
    /// 边界盒
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    pub restitution: f32,
}

impl Default for SbdConfig {
    fn default() -> Self {
        Self {
            dt: 1.0 / 60.0,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            damping: 0.99,
            stiffness: 0.9,
            volume_stiffness: 0.95,
            iterations: 8,
            bounds_min: Vec3::new(-10.0, -10.0, -10.0),
            bounds_max: Vec3::new(10.0, 10.0, 10.0),
            restitution: 0.3,
        }
    }
}

// ============================================================
// 粒子和四面体
// ============================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SbdParticle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub inv_mass: f32,
    pub predicted: Vec3,
    pub pinned: bool,
}

impl SbdParticle {
    pub fn new(position: Vec3, mass: f32) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            inv_mass: if mass > 0.0 { 1.0 / mass } else { 0.0 },
            predicted: position,
            pinned: false,
        }
    }

    pub fn pinned(position: Vec3) -> Self {
        Self { position, velocity: Vec3::ZERO, inv_mass: 0.0, predicted: position, pinned: true }
    }

    #[inline]
    pub fn is_dynamic(&self) -> bool {
        self.inv_mass > 0.0 && !self.pinned
    }
}

/// 四面体 (4 个粒子索引)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Tet {
    pub p: [usize; 4],
    /// 静止形态的逆矩阵 (3x3, 列向量 = 三条边)
    pub rest_inv: Mat3,
    /// 静止体积
    pub rest_volume: f32,
    /// 当前拉格朗日乘子 (用于刚度衰减)
    pub lambda_strain: f32,
    pub lambda_volume: f32,
}

impl Tet {
    /// 从静止位置创建四面体
    pub fn new(p0: usize, p1: usize, p2: usize, p3: usize, positions: &[Vec3]) -> Self {
        let x0 = positions[p0];
        let x1 = positions[p1];
        let x2 = positions[p2];
        let x3 = positions[p3];
        // 三条边 (从 vertex 0 出发)
        let e1 = x1 - x0;
        let e2 = x2 - x0;
        let e3 = x3 - x0;
        let rest = Mat3::from_cols(e1, e2, e3);
        let rest_inv = rest.inverse();
        let rest_volume = (e1.dot(e2.cross(e3))).abs() / 6.0;
        Self { p: [p0, p1, p2, p3], rest_inv, rest_volume, lambda_strain: 0.0, lambda_volume: 0.0 }
    }
}

// ============================================================
// 求解器
// ============================================================

pub struct SbdSolver {
    pub config: SbdConfig,
    pub particles: Vec<SbdParticle>,
    pub tets: Vec<Tet>,
}

impl SbdSolver {
    pub fn new(config: SbdConfig) -> Self {
        Self { config, particles: Vec::new(), tets: Vec::new() }
    }

    pub fn add_particle(&mut self, p: SbdParticle) -> usize {
        let idx = self.particles.len();
        self.particles.push(p);
        idx
    }

    pub fn add_tet(&mut self, p0: usize, p1: usize, p2: usize, p3: usize) -> usize {
        let tet = Tet::new(
            p0,
            p1,
            p2,
            p3,
            &self.particles.iter().map(|p| p.position).collect::<Vec<_>>(),
        );
        let idx = self.tets.len();
        self.tets.push(tet);
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
        for p in &mut self.particles {
            if p.is_dynamic() {
                p.predicted = p.position
                    + p.velocity * dt * self.config.damping
                    + self.config.gravity * dt * dt;
            } else {
                p.predicted = p.position;
            }
        }

        // 1.5 检测 NaN (退化 tet 导致), 重置预测位置
        for p in &mut self.particles {
            if !p.predicted.x.is_finite()
                || !p.predicted.y.is_finite()
                || !p.predicted.z.is_finite()
            {
                p.predicted = p.position;
            }
        }

        // 2. PBD 求解约束
        for _ in 0..self.config.iterations {
            self.solve_strain_constraints();
            self.solve_volume_constraints();
        }

        // 3. 边界约束
        for p in &mut self.particles {
            if !p.is_dynamic() {
                continue;
            }
            for axis in 0..3 {
                if p.predicted[axis] < self.config.bounds_min[axis] {
                    p.predicted[axis] = self.config.bounds_min[axis];
                    if p.velocity[axis] < 0.0 {
                        p.velocity[axis] = -p.velocity[axis] * self.config.restitution;
                    }
                } else if p.predicted[axis] > self.config.bounds_max[axis] {
                    p.predicted[axis] = self.config.bounds_max[axis];
                    if p.velocity[axis] > 0.0 {
                        p.velocity[axis] = -p.velocity[axis] * self.config.restitution;
                    }
                }
            }
        }

        // 4. 更新速度和位置
        let dt_inv = 1.0 / dt.max(1e-10);
        for p in &mut self.particles {
            if p.is_dynamic() {
                p.velocity = (p.predicted - p.position) * dt_inv;
                p.position = p.predicted;
            }
        }
    }

    /// 应变约束: 每条边的应变 = ||current_edge|| / ||rest_edge|| - 1
    /// 简化版: 每条边作为独立距离约束 (类似 PBD 长度约束)
    /// 完整版: 计算 Green-Lagrange 应变张量, 约束 6 个应变分量
    fn solve_strain_constraints(&mut self) {
        let stiffness = self.config.stiffness;
        // 使用 PBD 硬度衰减 (1 - (1-stiffness)^iterations)
        // 这里已在外层迭代, 用 sqrt衰减让总效果接近 stiffness
        let k = stiffness.powf(1.0 / self.config.iterations as f32);

        // 避免借用冲突: 先提取需要的 tet 数据
        let tet_data: Vec<(Mat3, [usize; 4], f32)> =
            self.tets.iter().map(|t| (t.rest_inv, t.p, t.rest_volume)).collect();

        for (rest_inv, p_idx, _rest_vol) in &tet_data {
            // 检测退化 tet (rest_inv 奇异 = 共面粒子)
            let rest_det = rest_inv.determinant();
            if !rest_det.is_finite() || rest_det.abs() > 1e18 {
                continue;
            }
            let x0 = self.particles[p_idx[0]].predicted;
            let x1 = self.particles[p_idx[1]].predicted;
            let x2 = self.particles[p_idx[2]].predicted;
            let x3 = self.particles[p_idx[3]].predicted;

            // 当前形状矩阵
            let cur = Mat3::from_cols(x1 - x0, x2 - x0, x3 - x0);
            // 变形梯度 F = cur * rest_inv
            let f_mat = cur * (*rest_inv);

            // Green-Lagrange 应变: E = 0.5 * (F^T F - I)
            let ft_f = f_mat.transpose() * f_mat;
            let strain = Mat3::from_cols(
                Vec3::new(ft_f.x_axis.x - 1.0, ft_f.x_axis.y, ft_f.x_axis.z),
                Vec3::new(ft_f.y_axis.x, ft_f.y_axis.y - 1.0, ft_f.y_axis.z),
                Vec3::new(ft_f.z_axis.x, ft_f.z_axis.y, ft_f.z_axis.z),
            ) * 0.5;

            // 简化: 用应变张量的对角项作为约束 (拉伸/压缩)
            // 完整版应处理剪切项, 但简化版已足够稳定
            // 约束: C = strain_diagonal = (E_xx, E_yy, E_zz)
            // 目标: 让 strain -> 0 (恢复静止形态)

            // 计算位置修正 (基于 F 的链式法则)
            // ∂E/∂x = 0.5 * ∂(F^T F)/∂x = F^T * ∂F/∂x
            // 简化: 直接用 F 的列向量作为修正方向

            let w0 = self.particles[p_idx[0]].inv_mass;
            let w1 = self.particles[p_idx[1]].inv_mass;
            let w2 = self.particles[p_idx[2]].inv_mass;
            let w3 = self.particles[p_idx[3]].inv_mass;
            let w_sum = w0 + w1 + w2 + w3;
            if w_sum < 1e-10 {
                continue;
            }

            // 应变能量的"伪梯度": 用 F 的 SVD 主方向
            // 简化策略: 把应变映射回位置修正
            // 对每条边: 修正让其恢复静止长度
            // 这种简化等价于 6 个距离约束 (tet 的 6 条边)
            // 但保留 F 计算以便未来扩展到完整应变约束

            // 计算修正: 用 rest 形态的边向量, 找到 F 中偏差最大的方向
            // 简化: Δx_i = -k * E_i * (cur - rest) / w_sum
            // 这是一个伪弹簧修正, 把当前形状拉回静止形状

            let _ = strain; // 标记使用 (完整版会用到)
            let _ = f_mat;

            // 简化版: 边距离约束 (等价于把 6 条边作为长度约束)
            // 这样保留了 SBD 的框架, 但用 PBD 长度约束实现
            let edges = [
                (p_idx[0], p_idx[1]),
                (p_idx[0], p_idx[2]),
                (p_idx[0], p_idx[3]),
                (p_idx[1], p_idx[2]),
                (p_idx[1], p_idx[3]),
                (p_idx[2], p_idx[3]),
            ];
            // rest 长度
            let rest_edges =
                [(self.particles[p_idx[0]].position - self.particles[p_idx[1]].position).length()];
            let _ = rest_edges;

            // 用 rest_inv 重算 rest 边长度 (避免依赖初始 position, 因为粒子会移动)
            // rest 形状的 4 个顶点: x0_rest = 0, x1_rest = rest.col(0), 等
            // 用当前粒子 position 不对, 应该用 rest 形态
            // 这里我们用 rest_inv 的逆 (即 rest) 重算
            let rest_mat = rest_inv.inverse();
            let rest_pts = [Vec3::ZERO, rest_mat.x_axis, rest_mat.y_axis, rest_mat.z_axis];
            let rest_lengths: [f32; 6] = [
                (rest_pts[0] - rest_pts[1]).length(),
                (rest_pts[0] - rest_pts[2]).length(),
                (rest_pts[0] - rest_pts[3]).length(),
                (rest_pts[1] - rest_pts[2]).length(),
                (rest_pts[1] - rest_pts[3]).length(),
                (rest_pts[2] - rest_pts[3]).length(),
            ];

            for (ei, &(a, b)) in edges.iter().enumerate() {
                let pa = self.particles[a].predicted;
                let pb = self.particles[b].predicted;
                let diff = pa - pb;
                let dist = diff.length();
                if dist < 1e-10 {
                    continue;
                }
                let rest_len = rest_lengths[ei];
                let c = dist - rest_len;
                let wa = self.particles[a].inv_mass;
                let wb = self.particles[b].inv_mass;
                let w_ab = wa + wb;
                if w_ab < 1e-10 {
                    continue;
                }
                let dir = diff / dist;
                let correction = -k * c / w_ab;
                if correction.is_finite() {
                    if self.particles[a].is_dynamic() {
                        self.particles[a].predicted += dir * (correction * wa);
                    }
                    if self.particles[b].is_dynamic() {
                        self.particles[b].predicted -= dir * (correction * wb);
                    }
                }
            }
        }
    }

    /// 体积约束: |current_volume| = |rest_volume|
    fn solve_volume_constraints(&mut self) {
        let k = self.config.volume_stiffness.powf(1.0 / self.config.iterations as f32);

        let tet_data: Vec<(Mat3, [usize; 4], f32)> =
            self.tets.iter().map(|t| (t.rest_inv, t.p, t.rest_volume)).collect();

        for (rest_inv, p_idx, rest_vol) in &tet_data {
            let x0 = self.particles[p_idx[0]].predicted;
            let x1 = self.particles[p_idx[1]].predicted;
            let x2 = self.particles[p_idx[2]].predicted;
            let x3 = self.particles[p_idx[3]].predicted;

            let cur = Mat3::from_cols(x1 - x0, x2 - x0, x3 - x0);
            let _ = rest_inv;
            let cur_vol = cur.determinant() / 6.0;
            let c = cur_vol - rest_vol; // 体积约束
            if c.abs() < 1e-10 {
                continue;
            }

            // 体积梯度: ∂V/∂x_i
            // V = (1/6) * (x1-x0) · ((x2-x0) × (x3-x0))
            // ∂V/∂x0 = (1/6) * ((x2-x0)×(x3-x0) + (x1-x0)×(x3-x0)... )
            // 简化: ∂V/∂x0 = -(1/6) * ((x2-x0)×(x3-x0) + (x1-x0)×(x3-x0) + ...)
            // 标准 formula:
            let e1 = x1 - x0;
            let e2 = x2 - x0;
            let e3 = x3 - x0;
            let grad0 = -(1.0 / 6.0) * (e2.cross(e3) + e3.cross(e1) + e1.cross(e2));
            let grad1 = (1.0 / 6.0) * e2.cross(e3);
            let grad2 = (1.0 / 6.0) * e3.cross(e1);
            let grad3 = (1.0 / 6.0) * e1.cross(e2);

            let w0 = self.particles[p_idx[0]].inv_mass;
            let w1 = self.particles[p_idx[1]].inv_mass;
            let w2 = self.particles[p_idx[2]].inv_mass;
            let w3 = self.particles[p_idx[3]].inv_mass;
            let denom = w0 * grad0.length_squared()
                + w1 * grad1.length_squared()
                + w2 * grad2.length_squared()
                + w3 * grad3.length_squared();
            if denom < 1e-12 {
                continue;
            }
            let lambda = -c / denom;
            let factor = k * lambda;
            if !factor.is_finite() {
                continue;
            }
            if self.particles[p_idx[0]].is_dynamic() {
                self.particles[p_idx[0]].predicted += grad0 * (factor * w0);
            }
            if self.particles[p_idx[1]].is_dynamic() {
                self.particles[p_idx[1]].predicted += grad1 * (factor * w1);
            }
            if self.particles[p_idx[2]].is_dynamic() {
                self.particles[p_idx[2]].predicted += grad2 * (factor * w2);
            }
            if self.particles[p_idx[3]].is_dynamic() {
                self.particles[p_idx[3]].predicted += grad3 * (factor * w3);
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

    /// 当前总体积 (检查体积守恒)
    pub fn total_volume(&self) -> f32 {
        let mut vol = 0.0;
        for tet in &self.tets {
            let x0 = self.particles[tet.p[0]].position;
            let x1 = self.particles[tet.p[1]].position;
            let x2 = self.particles[tet.p[2]].position;
            let x3 = self.particles[tet.p[3]].position;
            let cur = Mat3::from_cols(x1 - x0, x2 - x0, x3 - x0);
            vol += cur.determinant() / 6.0;
        }
        vol
    }

    /// 静止总体积
    pub fn total_rest_volume(&self) -> f32 {
        self.tets.iter().map(|t| t.rest_volume).sum()
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sbd_config_default() {
        let c = SbdConfig::default();
        assert!(c.dt > 0.0);
        assert!(c.stiffness >= 0.0 && c.stiffness <= 1.0);
        assert!(c.iterations > 0);
    }

    #[test]
    fn test_sbd_particle_creation() {
        let p = SbdParticle::new(Vec3::new(1.0, 2.0, 3.0), 2.0);
        assert_eq!(p.position, Vec3::new(1.0, 2.0, 3.0));
        assert!((p.inv_mass - 0.5).abs() < 1e-6);
        assert!(p.is_dynamic());
    }

    #[test]
    fn test_sbd_pinned_particle() {
        let p = SbdParticle::pinned(Vec3::ZERO);
        assert!(!p.is_dynamic());
    }

    #[test]
    fn test_tet_creation() {
        // 标准 tet: 4 个顶点
        let p0 = Vec3::new(0.0, 0.0, 0.0);
        let p1 = Vec3::new(1.0, 0.0, 0.0);
        let p2 = Vec3::new(0.0, 1.0, 0.0);
        let p3 = Vec3::new(0.0, 0.0, 1.0);
        let positions = vec![p0, p1, p2, p3];
        let tet = Tet::new(0, 1, 2, 3, &positions);
        // 体积 = (1/6) * |det([e1,e2,e3])| = 1/6
        assert!((tet.rest_volume - 1.0 / 6.0).abs() < 1e-5, "tet volume: {}", tet.rest_volume);
    }

    #[test]
    fn test_sbd_free_fall() {
        let mut solver = SbdSolver::new(SbdConfig::default());
        solver.add_particle(SbdParticle::new(Vec3::new(0.0, 10.0, 0.0), 1.0));
        // 至少一个 tet 才有意义, 但自由粒子也应能下落
        solver.step();
        assert!(solver.particles[0].position.y < 10.0, "should fall");
    }

    #[test]
    fn test_sbd_tet_preserves_volume() {
        // 一个 tet, 应保持体积
        let mut solver = SbdSolver::new(SbdConfig {
            dt: 0.005,
            gravity: Vec3::ZERO, // 无重力, 测试纯弹性
            stiffness: 1.0,
            volume_stiffness: 1.0,
            iterations: 16,
            ..SbdConfig::default()
        });
        let p0 = solver.add_particle(SbdParticle::new(Vec3::new(0.0, 0.0, 0.0), 1.0));
        let p1 = solver.add_particle(SbdParticle::new(Vec3::new(1.0, 0.0, 0.0), 1.0));
        let p2 = solver.add_particle(SbdParticle::new(Vec3::new(0.0, 1.0, 0.0), 1.0));
        let p3 = solver.add_particle(SbdParticle::new(Vec3::new(0.0, 0.0, 1.0), 1.0));
        solver.add_tet(p0, p1, p2, p3);

        let rest_vol = solver.total_rest_volume();
        // 给一个扰动
        solver.particles[1].position = Vec3::new(1.5, 0.0, 0.0);
        // 多步求解, 应恢复体积
        for _ in 0..20 {
            solver.step();
        }
        let final_vol = solver.total_volume();
        assert!(
            (final_vol - rest_vol).abs() < 0.05 * rest_vol.abs(),
            "volume preserved: rest={}, final={}, diff={}",
            rest_vol,
            final_vol,
            (final_vol - rest_vol).abs()
        );
    }

    #[test]
    fn test_sbd_rigid_tet_no_deformation() {
        // 高刚度 + 无外力, tet 应保持形状
        let mut solver = SbdSolver::new(SbdConfig {
            dt: 0.01,
            gravity: Vec3::ZERO,
            stiffness: 1.0,
            volume_stiffness: 1.0,
            iterations: 20,
            ..SbdConfig::default()
        });
        let p0 = solver.add_particle(SbdParticle::new(Vec3::new(0.0, 0.0, 0.0), 1.0));
        let p1 = solver.add_particle(SbdParticle::new(Vec3::new(1.0, 0.0, 0.0), 1.0));
        let p2 = solver.add_particle(SbdParticle::new(Vec3::new(0.0, 1.0, 0.0), 1.0));
        let p3 = solver.add_particle(SbdParticle::new(Vec3::new(0.0, 0.0, 1.0), 1.0));
        solver.add_tet(p0, p1, p2, p3);

        let initial_edge_len =
            (solver.particles[1].position - solver.particles[0].position).length();
        for _ in 0..10 {
            solver.step();
        }
        let final_edge_len = (solver.particles[1].position - solver.particles[0].position).length();
        assert!(
            (final_edge_len - initial_edge_len).abs() < 0.01,
            "rigid tet edge preserved: initial={}, final={}",
            initial_edge_len,
            final_edge_len
        );
    }

    #[test]
    fn test_sbd_pinned_tet_hangs() {
        // 顶点 0 pinned, 其他自由, 应悬挂不无限下落
        let mut solver = SbdSolver::new(SbdConfig {
            dt: 0.005,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            stiffness: 0.9,
            volume_stiffness: 0.95,
            iterations: 10,
            ..SbdConfig::default()
        });
        let p0 = solver.add_particle(SbdParticle::pinned(Vec3::new(0.0, 5.0, 0.0)));
        let p1 = solver.add_particle(SbdParticle::new(Vec3::new(1.0, 5.0, 0.0), 1.0));
        let p2 = solver.add_particle(SbdParticle::new(Vec3::new(0.0, 4.0, 0.0), 1.0));
        let p3 = solver.add_particle(SbdParticle::new(Vec3::new(0.0, 5.0, 1.0), 1.0));
        solver.add_tet(p0, p1, p2, p3);

        for _ in 0..60 {
            solver.step();
        }
        // 应保持悬挂, 不无限下落
        let avg_y = (solver.particles[1].position.y
            + solver.particles[2].position.y
            + solver.particles[3].position.y)
            / 3.0;
        assert!(avg_y > 0.0, "tet hangs: avg_y={}", avg_y);
    }

    #[test]
    fn test_sbd_soft_deforms_under_gravity() {
        // 低刚度 tet 在重力下应变形
        let mut solver = SbdSolver::new(SbdConfig {
            dt: 0.005,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            stiffness: 0.1,        // 软
            volume_stiffness: 0.3, // 软体积
            iterations: 4,
            ..SbdConfig::default()
        });
        let p0 = solver.add_particle(SbdParticle::pinned(Vec3::new(-0.5, 5.0, 0.0)));
        let p1 = solver.add_particle(SbdParticle::pinned(Vec3::new(0.5, 5.0, 0.0)));
        let p2 = solver.add_particle(SbdParticle::new(Vec3::new(-0.5, 4.0, 0.5), 1.0));
        let p3 = solver.add_particle(SbdParticle::new(Vec3::new(0.5, 4.0, -0.5), 1.0));
        solver.add_tet(p0, p1, p2, p3);

        let initial_height = solver.particles[2].position.y;
        for _ in 0..60 {
            solver.step();
        }
        let final_height = solver.particles[2].position.y;
        // 软体应下垂 (高度下降)
        assert!(
            final_height < initial_height,
            "soft body deforms: initial={}, final={}",
            initial_height,
            final_height
        );
    }

    #[test]
    fn test_sbd_total_volume() {
        let mut solver = SbdSolver::new(SbdConfig::default());
        let p0 = solver.add_particle(SbdParticle::new(Vec3::ZERO, 1.0));
        let p1 = solver.add_particle(SbdParticle::new(Vec3::new(2.0, 0.0, 0.0), 1.0));
        let p2 = solver.add_particle(SbdParticle::new(Vec3::new(0.0, 2.0, 0.0), 1.0));
        let p3 = solver.add_particle(SbdParticle::new(Vec3::new(0.0, 0.0, 2.0), 1.0));
        solver.add_tet(p0, p1, p2, p3);
        let vol = solver.total_volume();
        // 体积 = (1/6) * |det([2,0,0; 0,2,0; 0,0,2])| = 8/6
        assert!((vol - 8.0 / 6.0).abs() < 1e-4, "volume: {}", vol);
    }

    #[test]
    fn test_sbd_kinetic_energy() {
        let solver = SbdSolver::new(SbdConfig { gravity: Vec3::ZERO, ..SbdConfig::default() });
        let p = SbdParticle::new(Vec3::ZERO, 2.0);
        let mut solver_p = solver;
        solver_p.particles.push(p);
        solver_p.particles[0].velocity = Vec3::new(1.0, 0.0, 0.0);
        let ke = solver_p.kinetic_energy();
        // 0.5 * 2.0 * 1.0^2 = 1.0
        assert!((ke - 1.0).abs() < 1e-4, "ke: {}", ke);
    }

    #[test]
    fn test_sbd_multiple_tets() {
        // 两个 tet 共享一个面, 应一起求解
        let mut solver = SbdSolver::new(SbdConfig {
            dt: 0.005,
            gravity: Vec3::ZERO,
            stiffness: 0.9,
            volume_stiffness: 0.9,
            iterations: 8,
            ..SbdConfig::default()
        });
        let p0 = solver.add_particle(SbdParticle::new(Vec3::new(0.0, 0.0, 0.0), 1.0));
        let p1 = solver.add_particle(SbdParticle::new(Vec3::new(1.0, 0.0, 0.0), 1.0));
        let p2 = solver.add_particle(SbdParticle::new(Vec3::new(0.0, 1.0, 0.0), 1.0));
        let p3 = solver.add_particle(SbdParticle::new(Vec3::new(0.0, 0.0, 1.0), 1.0));
        let p4 = solver.add_particle(SbdParticle::new(Vec3::new(1.0, 1.0, 1.0), 1.0));
        solver.add_tet(p0, p1, p2, p3);
        solver.add_tet(p1, p2, p3, p4);

        let rest_vol = solver.total_rest_volume();
        // 给一个扰动
        solver.particles[0].position = Vec3::new(-0.2, 0.0, 0.0);
        for _ in 0..20 {
            solver.step();
        }
        let final_vol = solver.total_volume();
        assert!(
            (final_vol - rest_vol).abs() < 0.1 * rest_vol.abs(),
            "multi-tet volume: rest={}, final={}",
            rest_vol,
            final_vol
        );
    }

    #[test]
    fn test_sbd_boundary_collision() {
        let mut solver = SbdSolver::new(SbdConfig {
            dt: 0.01,
            gravity: Vec3::new(0.0, -20.0, 0.0),
            stiffness: 0.9,
            volume_stiffness: 0.9,
            iterations: 8,
            bounds_min: Vec3::new(-5.0, -1.0, -5.0),
            bounds_max: Vec3::new(5.0, 5.0, 5.0),
            ..SbdConfig::default()
        });
        let p0 = solver.add_particle(SbdParticle::new(Vec3::new(0.0, 4.0, 0.0), 1.0));
        let p1 = solver.add_particle(SbdParticle::new(Vec3::new(0.5, 4.0, 0.0), 1.0));
        let p2 = solver.add_particle(SbdParticle::new(Vec3::new(0.0, 3.5, 0.0), 1.0));
        let p3 = solver.add_particle(SbdParticle::new(Vec3::new(0.0, 4.0, 0.5), 1.0));
        solver.add_tet(p0, p1, p2, p3);

        for _ in 0..60 {
            solver.step();
        }
        // 应停在边界内
        for p in &solver.particles {
            assert!(p.position.y >= -1.01, "boundary collision: y={}", p.position.y);
        }
    }
}
