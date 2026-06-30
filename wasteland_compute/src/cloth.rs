//! Cloth Simulation — XPBD 布料求解器
//!
//! 基于:
//! - Macklin, Muller, Chentanez. "XPBD: Position-Based Simulation of
//!   Compliant Constrained Dynamics." MIG 2016.
//! - Muller, Macklin, Chentanez, Kim, Miles. "Detailed Rigid Body Simulation
//!   with Extended Position Based Dynamics." VRIPHYS 2020.
//! - Bridson, Marino, Fedkiw. "Simulation of Clothing with Folds and Wrinkles."
//!   SCA 2003.
//!
//! 核心思想:
//! 1. 布料 = 三角网格 + 粒子 (每个顶点)
//! 2. XPBD 约束 (compliance α = 1/k):
//!    - 拉伸 (stretch): 每条边恢复静止长度 ||x_b - x_a|| = rest_len
//!    - 剪切 (shear): 对角边约束 (跨三角形)
//!    - 弯曲 (bend): skip-one-vertex 距离约束 (稳定, 适用于任意网格)
//! 3. XPBD 更新 (标准符号):
//!    C = dist - rest_len  (拉伸时 C > 0)
//!    ∇C_a = -dir, ∇C_b = +dir  (dir = (x_b - x_a)/|x_b - x_a|)
//!    Δλ = (-C - α̃·λ_old) / (w_a + w_b + α̃),  α̃ = α/h²
//!    Δx_a = -w_a · dir · Δλ   (C>0 时 Δλ<0, a 朝 b 移动)
//!    Δx_b = +w_b · dir · Δλ   (C>0 时 Δλ<0, b 朝 a 移动)
//! 4. 时间积分: 预测位置 -> XPBD 迭代 -> 速度+位置更新

use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClothConfig {
    pub dt: f32,
    pub gravity: Vec3,
    pub damping: f32,
    pub stretch_stiffness: f32,
    pub shear_stiffness: f32,
    pub bend_stiffness: f32,
    pub iterations: usize,
    pub wind: Vec3,
    pub air_drag: f32,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    pub restitution: f32,
}

impl Default for ClothConfig {
    fn default() -> Self {
        Self {
            dt: 1.0 / 60.0,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            damping: 0.99,
            stretch_stiffness: 1e4,
            shear_stiffness: 1e3,
            bend_stiffness: 1e2,
            iterations: 10,
            wind: Vec3::ZERO,
            air_drag: 0.02,
            bounds_min: Vec3::new(-10.0, -10.0, -10.0),
            bounds_max: Vec3::new(10.0, 10.0, 10.0),
            restitution: 0.3,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ClothParticle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub inv_mass: f32,
    pub predicted: Vec3,
    pub pinned: bool,
}

impl ClothParticle {
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

/// 距离约束 (边) — XPBD
/// C(x) = |x_b - x_a| - rest_length
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DistanceConstraint {
    pub a: usize,
    pub b: usize,
    pub rest_length: f32,
    pub stiffness: f32,
    pub lambda: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClothMesh {
    pub particles: Vec<ClothParticle>,
    pub stretch_constraints: Vec<DistanceConstraint>,
    pub shear_constraints: Vec<DistanceConstraint>,
    pub bend_constraints: Vec<DistanceConstraint>,
}

impl ClothMesh {
    pub fn plane(
        origin: Vec3,
        u_dir: Vec3,
        v_dir: Vec3,
        n_u: usize,
        n_v: usize,
        mass_per_particle: f32,
    ) -> Self {
        let mut particles = Vec::with_capacity(n_u * n_v);
        let mut stretch = Vec::new();
        let mut shear = Vec::new();
        let mut bend = Vec::new();

        let n_u_m1 = n_u.saturating_sub(1).max(1);
        let n_v_m1 = n_v.saturating_sub(1).max(1);

        for j in 0..n_v {
            for i in 0..n_u {
                let p = origin
                    + u_dir * (i as f32 / n_u_m1 as f32)
                    + v_dir * (j as f32 / n_v_m1 as f32);
                particles.push(ClothParticle::new(p, mass_per_particle));
            }
        }

        let seg_u = u_dir.length() / n_u_m1 as f32;
        let seg_v = v_dir.length() / n_v_m1 as f32;
        for j in 0..n_v {
            for i in 0..n_u {
                let idx = i + j * n_u;
                if i + 1 < n_u {
                    stretch.push(DistanceConstraint {
                        a: idx,
                        b: idx + 1,
                        rest_length: seg_u,
                        stiffness: 1.0,
                        lambda: 0.0,
                    });
                }
                if j + 1 < n_v {
                    stretch.push(DistanceConstraint {
                        a: idx,
                        b: idx + n_u,
                        rest_length: seg_v,
                        stiffness: 1.0,
                        lambda: 0.0,
                    });
                }
            }
        }

        let diag = (seg_u * seg_u + seg_v * seg_v).sqrt();
        for j in 0..(n_v.saturating_sub(1)) {
            for i in 0..(n_u.saturating_sub(1)) {
                let idx = i + j * n_u;
                shear.push(DistanceConstraint {
                    a: idx,
                    b: idx + n_u + 1,
                    rest_length: diag,
                    stiffness: 1.0,
                    lambda: 0.0,
                });
                shear.push(DistanceConstraint {
                    a: idx + 1,
                    b: idx + n_u,
                    rest_length: diag,
                    stiffness: 1.0,
                    lambda: 0.0,
                });
            }
        }

        // 弯曲约束 (skip-one-vertex 距离约束, 稳定)
        let bend_h = 2.0 * seg_u;
        for j in 0..n_v {
            for i in 1..n_u.saturating_sub(1) {
                let p_left = (i - 1) + j * n_u;
                let p_right = (i + 1) + j * n_u;
                bend.push(DistanceConstraint {
                    a: p_left,
                    b: p_right,
                    rest_length: bend_h,
                    stiffness: 1.0,
                    lambda: 0.0,
                });
            }
        }
        let bend_v = 2.0 * seg_v;
        for j in 1..n_v.saturating_sub(1) {
            for i in 0..n_u {
                let p_up = i + (j - 1) * n_u;
                let p_down = i + (j + 1) * n_u;
                bend.push(DistanceConstraint {
                    a: p_up,
                    b: p_down,
                    rest_length: bend_v,
                    stiffness: 1.0,
                    lambda: 0.0,
                });
            }
        }

        Self {
            particles,
            stretch_constraints: stretch,
            shear_constraints: shear,
            bend_constraints: bend,
        }
    }

    pub fn set_stiffness(&mut self, stretch: f32, shear: f32, bend: f32) {
        for c in &mut self.stretch_constraints {
            c.stiffness = stretch;
        }
        for c in &mut self.shear_constraints {
            c.stiffness = shear;
        }
        for c in &mut self.bend_constraints {
            c.stiffness = bend;
        }
    }

    pub fn pin(&mut self, idx: usize) {
        if idx < self.particles.len() {
            self.particles[idx].pinned = true;
            self.particles[idx].inv_mass = 0.0;
        }
    }

    pub fn unpin(&mut self, idx: usize, mass: f32) {
        if idx < self.particles.len() {
            self.particles[idx].pinned = false;
            self.particles[idx].inv_mass = if mass > 0.0 { 1.0 / mass } else { 0.0 };
        }
    }
}

pub struct ClothSolver {
    pub config: ClothConfig,
    pub meshes: Vec<ClothMesh>,
}

impl ClothSolver {
    pub fn new(config: ClothConfig) -> Self {
        Self { config, meshes: Vec::new() }
    }

    pub fn add_mesh(&mut self, mesh: ClothMesh) -> usize {
        let idx = self.meshes.len();
        self.meshes.push(mesh);
        idx
    }

    pub fn step(&mut self) {
        let dt = self.config.dt;
        let damping = self.config.damping;
        let gravity = self.config.gravity;
        let wind = self.config.wind;
        let air_drag = self.config.air_drag;
        let accel_ext = gravity + wind;

        // 1. 预测位置
        for mesh in &mut self.meshes {
            for p in &mut mesh.particles {
                if p.is_dynamic() {
                    let v_damped = p.velocity * (1.0 - air_drag);
                    p.predicted = p.position + v_damped * dt * damping + accel_ext * dt * dt;
                } else {
                    p.predicted = p.position;
                }
            }
        }

        // 2. 重置 XPBD 乘子
        for mesh in &mut self.meshes {
            for c in &mut mesh.stretch_constraints {
                c.lambda = 0.0;
            }
            for c in &mut mesh.shear_constraints {
                c.lambda = 0.0;
            }
            for c in &mut mesh.bend_constraints {
                c.lambda = 0.0;
            }
        }

        // 3. XPBD 迭代
        for _ in 0..self.config.iterations {
            self.solve_distance_constraints(ConstraintKind::Stretch);
            self.solve_distance_constraints(ConstraintKind::Shear);
            self.solve_distance_constraints(ConstraintKind::Bend);
        }

        // 4. 边界约束
        for mesh in &mut self.meshes {
            for p in &mut mesh.particles {
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
        }

        // 5. 速度和位置更新
        let dt_inv = 1.0 / dt.max(1e-10);
        for mesh in &mut self.meshes {
            for p in &mut mesh.particles {
                if p.is_dynamic() {
                    p.velocity = (p.predicted - p.position) * dt_inv;
                    if !p.velocity.is_finite() {
                        p.velocity = Vec3::ZERO;
                    }
                    p.position = p.predicted;
                    if !p.position.is_finite() {
                        p.position = Vec3::ZERO;
                    }
                }
            }
        }
    }

    fn solve_distance_constraints(&mut self, kind: ConstraintKind) {
        let dt = self.config.dt;
        let dt2 = dt * dt;
        let k_global = match kind {
            ConstraintKind::Stretch => self.config.stretch_stiffness,
            ConstraintKind::Shear => self.config.shear_stiffness,
            ConstraintKind::Bend => self.config.bend_stiffness,
        };
        let alpha = 1.0 / k_global.max(1e-6);

        for mesh in &mut self.meshes {
            let constraints: Vec<(usize, usize, f32, f32)> = match kind {
                ConstraintKind::Stretch => mesh
                    .stretch_constraints
                    .iter()
                    .map(|c| (c.a, c.b, c.rest_length, c.stiffness))
                    .collect(),
                ConstraintKind::Shear => mesh
                    .shear_constraints
                    .iter()
                    .map(|c| (c.a, c.b, c.rest_length, c.stiffness))
                    .collect(),
                ConstraintKind::Bend => mesh
                    .bend_constraints
                    .iter()
                    .map(|c| (c.a, c.b, c.rest_length, c.stiffness))
                    .collect(),
            };

            for (ci, &(a, b, rest_len, k)) in constraints.iter().enumerate() {
                let pa = mesh.particles[a].predicted;
                let pb = mesh.particles[b].predicted;
                let diff = pb - pa;
                let dist = diff.length();
                if dist < 1e-10 {
                    continue;
                }
                let c_val = dist - rest_len;
                let wa = mesh.particles[a].inv_mass;
                let wb = mesh.particles[b].inv_mass;
                let w_sum = wa + wb;
                if w_sum < 1e-10 {
                    continue;
                }

                let alpha_scaled = alpha / dt2 / k.max(1e-6);
                let lambda_old = match kind {
                    ConstraintKind::Stretch => mesh.stretch_constraints[ci].lambda,
                    ConstraintKind::Shear => mesh.shear_constraints[ci].lambda,
                    ConstraintKind::Bend => mesh.bend_constraints[ci].lambda,
                };
                let d_lambda = (-c_val - alpha_scaled * lambda_old) / (w_sum + alpha_scaled);
                let lambda_new = lambda_old + d_lambda;

                // 标准符号: Δx_a = -w_a·dir·Δλ, Δx_b = +w_b·dir·Δλ
                let dir = diff / dist;
                if mesh.particles[a].is_dynamic() {
                    mesh.particles[a].predicted -= dir * (d_lambda * wa);
                }
                if mesh.particles[b].is_dynamic() {
                    mesh.particles[b].predicted += dir * (d_lambda * wb);
                }

                match kind {
                    ConstraintKind::Stretch => mesh.stretch_constraints[ci].lambda = lambda_new,
                    ConstraintKind::Shear => mesh.shear_constraints[ci].lambda = lambda_new,
                    ConstraintKind::Bend => mesh.bend_constraints[ci].lambda = lambda_new,
                }
            }
        }
    }

    pub fn kinetic_energy(&self) -> f32 {
        let mut ke = 0.0;
        for mesh in &self.meshes {
            for p in &mesh.particles {
                if p.is_dynamic() {
                    let m = 1.0 / p.inv_mass;
                    ke += 0.5 * m * p.velocity.length_squared();
                }
            }
        }
        ke
    }

    pub fn particle_count(&self) -> usize {
        self.meshes.iter().map(|m| m.particles.len()).sum()
    }

    pub fn constraint_count(&self) -> usize {
        self.meshes
            .iter()
            .map(|m| {
                m.stretch_constraints.len() + m.shear_constraints.len() + m.bend_constraints.len()
            })
            .sum()
    }
}

#[derive(Clone, Copy)]
enum ConstraintKind {
    Stretch,
    Shear,
    Bend,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloth_config_default() {
        let c = ClothConfig::default();
        assert!(c.dt > 0.0);
        assert!(c.stretch_stiffness > 0.0);
        assert!(c.iterations > 0);
    }

    #[test]
    fn test_cloth_particle_creation() {
        let p = ClothParticle::new(Vec3::new(1.0, 2.0, 3.0), 2.0);
        assert_eq!(p.position, Vec3::new(1.0, 2.0, 3.0));
        assert!((p.inv_mass - 0.5).abs() < 1e-6);
        assert!(p.is_dynamic());
    }

    #[test]
    fn test_cloth_pinned_particle() {
        let p = ClothParticle::pinned(Vec3::ZERO);
        assert!(!p.is_dynamic());
    }

    #[test]
    fn test_cloth_mesh_plane() {
        let mesh = ClothMesh::plane(
            Vec3::new(-1.0, 0.0, -1.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 2.0),
            5,
            5,
            0.01,
        );
        assert_eq!(mesh.particles.len(), 25);
        assert_eq!(mesh.stretch_constraints.len(), 40);
        assert_eq!(mesh.shear_constraints.len(), 32);
        assert_eq!(mesh.bend_constraints.len(), 30);
    }

    #[test]
    fn test_cloth_mesh_plane_3x3() {
        let mesh = ClothMesh::plane(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            3,
            3,
            0.01,
        );
        assert_eq!(mesh.particles.len(), 9);
        assert_eq!(mesh.stretch_constraints.len(), 12);
        assert_eq!(mesh.shear_constraints.len(), 8);
        assert_eq!(mesh.bend_constraints.len(), 6);
    }

    #[test]
    fn test_cloth_pin_unpin() {
        let mut mesh = ClothMesh::plane(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            3,
            3,
            0.01,
        );
        mesh.pin(0);
        assert!(!mesh.particles[0].is_dynamic());
        mesh.unpin(0, 0.01);
        assert!(mesh.particles[0].is_dynamic());
    }

    #[test]
    fn test_cloth_set_stiffness() {
        let mut mesh = ClothMesh::plane(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            3,
            3,
            0.01,
        );
        mesh.set_stiffness(100.0, 50.0, 10.0);
        assert!((mesh.stretch_constraints[0].stiffness - 100.0).abs() < 1e-4);
        assert!((mesh.shear_constraints[0].stiffness - 50.0).abs() < 1e-4);
        assert!((mesh.bend_constraints[0].stiffness - 10.0).abs() < 1e-4);
    }

    #[test]
    fn test_cloth_solver_creation() {
        let solver = ClothSolver::new(ClothConfig::default());
        assert!(solver.meshes.is_empty());
    }

    #[test]
    fn test_cloth_free_fall() {
        let mut solver = ClothSolver::new(ClothConfig {
            dt: 0.01,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            bounds_min: Vec3::new(-5.0, -5.0, -5.0),
            bounds_max: Vec3::new(5.0, 5.0, 5.0),
            ..ClothConfig::default()
        });
        let mut mesh = ClothMesh::plane(
            Vec3::new(-0.5, 4.0, -0.5),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            3,
            3,
            0.01,
        );
        for i in 0..mesh.particles.len() {
            mesh.unpin(i, 0.01);
        }
        solver.add_mesh(mesh);
        let y0 = solver.meshes[0].particles[0].position.y;
        solver.step();
        let y1 = solver.meshes[0].particles[0].position.y;
        assert!(y1 < y0, "should fall: {} -> {}", y0, y1);
    }

    #[test]
    fn test_cloth_pinned_hangs() {
        let mut solver = ClothSolver::new(ClothConfig {
            dt: 0.005,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            stretch_stiffness: 1e5,
            iterations: 20,
            bounds_min: Vec3::new(-5.0, -5.0, -5.0),
            bounds_max: Vec3::new(5.0, 5.0, 5.0),
            ..ClothConfig::default()
        });
        let mut mesh = ClothMesh::plane(
            Vec3::new(-1.0, 3.0, -1.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 2.0),
            5,
            5,
            0.01,
        );
        mesh.pin(0);
        mesh.pin(4);
        mesh.pin(20);
        mesh.pin(24);
        solver.add_mesh(mesh);
        for _ in 0..60 {
            solver.step();
        }
        let center = solver.meshes[0].particles[12].position.y;
        assert!(center > 0.0, "cloth hangs: center_y={}", center);
    }

    #[test]
    fn test_cloth_stretch_constraint_preserves_length() {
        let mut solver = ClothSolver::new(ClothConfig {
            dt: 0.005,
            gravity: Vec3::ZERO,
            stretch_stiffness: 1e6,
            iterations: 30,
            ..ClothConfig::default()
        });
        let mesh = ClothMesh::plane(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            3,
            3,
            0.01,
        );
        let rest_len = mesh.stretch_constraints[0].rest_length;
        solver.add_mesh(mesh);
        solver.meshes[0].particles[4].position = Vec3::new(1.5, 0.5, 1.5);
        for _ in 0..20 {
            solver.step();
        }
        for c in &solver.meshes[0].stretch_constraints {
            let pa = solver.meshes[0].particles[c.a].position;
            let pb = solver.meshes[0].particles[c.b].position;
            let len = (pb - pa).length();
            assert!(
                (len - rest_len).abs() < 0.1 * rest_len,
                "stretch preserved: rest={}, cur={}",
                rest_len,
                len
            );
        }
    }

    #[test]
    fn test_cloth_no_nan_long_run() {
        let mut solver = ClothSolver::new(ClothConfig {
            dt: 0.005,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            bounds_min: Vec3::new(-2.0, -2.0, -2.0),
            bounds_max: Vec3::new(2.0, 2.0, 2.0),
            ..ClothConfig::default()
        });
        let mut mesh = ClothMesh::plane(
            Vec3::new(-0.5, 1.5, -0.5),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            5,
            5,
            0.01,
        );
        mesh.pin(0);
        mesh.pin(4);
        mesh.pin(20);
        mesh.pin(24);
        solver.add_mesh(mesh);
        for step in 0..200 {
            solver.step();
            if step % 50 == 49 {
                for p in &solver.meshes[0].particles {
                    assert!(p.position.x.is_finite(), "NaN at step {} in position", step);
                    assert!(p.velocity.x.is_finite(), "NaN at step {} in velocity", step);
                }
            }
        }
    }

    #[test]
    fn test_cloth_boundary_collision() {
        let mut solver = ClothSolver::new(ClothConfig {
            dt: 0.01,
            gravity: Vec3::new(0.0, -20.0, 0.0),
            stretch_stiffness: 1e3,
            bounds_min: Vec3::new(-1.0, -1.0, -1.0),
            bounds_max: Vec3::new(1.0, 1.0, 1.0),
            ..ClothConfig::default()
        });
        let mut mesh = ClothMesh::plane(
            Vec3::new(-0.3, 0.8, -0.3),
            Vec3::new(0.6, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.6),
            3,
            3,
            0.01,
        );
        for i in 0..mesh.particles.len() {
            mesh.unpin(i, 0.01);
        }
        solver.add_mesh(mesh);
        for _ in 0..100 {
            solver.step();
        }
        for p in &solver.meshes[0].particles {
            assert!(p.position.y >= -1.01, "boundary: y={}", p.position.y);
        }
    }

    #[test]
    fn test_cloth_wind_affects() {
        let mut solver1 = ClothSolver::new(ClothConfig {
            dt: 0.01,
            gravity: Vec3::ZERO,
            wind: Vec3::ZERO,
            ..ClothConfig::default()
        });
        let mut solver2 = ClothSolver::new(ClothConfig {
            dt: 0.01,
            gravity: Vec3::ZERO,
            wind: Vec3::new(10.0, 0.0, 0.0),
            ..ClothConfig::default()
        });
        let mesh1 = ClothMesh::plane(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            3,
            3,
            0.01,
        );
        let mesh2 = ClothMesh::plane(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            3,
            3,
            0.01,
        );
        solver1.add_mesh(mesh1);
        solver2.add_mesh(mesh2);
        solver1.step();
        solver2.step();
        let x1 = solver1.meshes[0].particles[4].position.x;
        let x2 = solver2.meshes[0].particles[4].position.x;
        assert!(x2 > x1, "wind affects: x1={}, x2={}", x1, x2);
    }

    #[test]
    fn test_cloth_multiple_meshes() {
        let mut solver = ClothSolver::new(ClothConfig::default());
        let m1 = ClothMesh::plane(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            3,
            3,
            0.01,
        );
        let m2 = ClothMesh::plane(
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            4,
            4,
            0.01,
        );
        solver.add_mesh(m1);
        solver.add_mesh(m2);
        assert_eq!(solver.meshes.len(), 2);
        assert_eq!(solver.particle_count(), 9 + 16);
        solver.step();
    }

    #[test]
    fn test_cloth_kinetic_energy() {
        let mut solver =
            ClothSolver::new(ClothConfig { gravity: Vec3::ZERO, ..ClothConfig::default() });
        let mut mesh = ClothMesh::plane(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            2,
            2,
            2.0,
        );
        for i in 0..mesh.particles.len() {
            mesh.unpin(i, 2.0);
        }
        mesh.particles[0].velocity = Vec3::new(1.0, 0.0, 0.0);
        solver.add_mesh(mesh);
        let ke = solver.kinetic_energy();
        assert!((ke - 1.0).abs() < 1e-4, "ke: {}", ke);
    }

    #[test]
    fn test_cloth_constraint_count() {
        let _ = ClothSolver::new(ClothConfig::default());
        let mut s = ClothSolver::new(ClothConfig::default());
        let mesh = ClothMesh::plane(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            4,
            4,
            0.01,
        );
        let expected = mesh.stretch_constraints.len()
            + mesh.shear_constraints.len()
            + mesh.bend_constraints.len();
        s.add_mesh(mesh);
        assert_eq!(s.constraint_count(), expected);
    }
}
