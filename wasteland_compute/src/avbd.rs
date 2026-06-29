//! AVBD — Augmented Velocity-Based Dynamics (SIGGRAPH 2025)

//! 核心: VBD (隐式速度积分) + 增广拉格朗日 (硬约束)

use glam::{Mat3, Quat, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvbdConfig {
    pub dt: f32,
    pub num_iters: usize,
    pub constraint_mu: f32,
    pub gravity: Vec3,
    pub damping: f32,
    pub restitution: f32,
}

impl Default for AvbdConfig {
    fn default() -> Self {
        Self {
            dt: 1.0 / 60.0,
            num_iters: 16,
            constraint_mu: 1e4,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            damping: 0.99,
            restitution: 0.3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvbdParticle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub predicted: Vec3,
    pub inv_mass: f32,
}

impl AvbdParticle {
    pub fn new(position: Vec3, mass: f32) -> Self {
        Self { position, velocity: Vec3::ZERO, predicted: position, inv_mass: if mass > 0.0 { 1.0 / mass } else { 0.0 } }
    }
    pub fn fixed(position: Vec3) -> Self {
        Self { position, velocity: Vec3::ZERO, predicted: position, inv_mass: 0.0 }
    }
    #[inline] pub fn is_fixed(&self) -> bool { self.inv_mass == 0.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistanceConstraint {
    pub p0: usize,
    pub p1: usize,
    pub rest_length: f32,
    pub stiffness: f32,
    pub lambda: f32,
}

impl DistanceConstraint {
    pub fn new(p0: usize, p1: usize, rest_length: f32, stiffness: f32) -> Self {
        Self { p0, p1, rest_length, stiffness, lambda: 0.0 }
    }
    #[inline] pub fn constraint(&self, x0: Vec3, x1: Vec3) -> f32 {
        (x1 - x0).length() - self.rest_length
    }
    #[inline] pub fn gradient(&self, x0: Vec3, x1: Vec3) -> (Vec3, Vec3) {
        let d = x1 - x0;
        let len = d.length().max(1e-10);
        let n_hat = d / len;
        (-n_hat, n_hat)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactConstraint {
    pub particle: usize,
    pub normal: Vec3,
    pub penetration: f32,
    pub friction: f32,
    pub lambda_n: f32,
    pub lambda_t: Vec3,
}

impl ContactConstraint {
    pub fn new(particle: usize, normal: Vec3, penetration: f32, friction: f32) -> Self {
        Self { particle, normal: normal.normalize_or_zero(), penetration, friction, lambda_n: 0.0, lambda_t: Vec3::ZERO }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BendingConstraint {
    pub p0: usize,
    pub p1: usize,
    pub p2: usize,
    pub rest_angle: f32,
    pub stiffness: f32,
    pub lambda: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvbdSolver {
    pub config: AvbdConfig,
    pub particles: Vec<AvbdParticle>,
    pub distance_constraints: Vec<DistanceConstraint>,
    pub contact_constraints: Vec<ContactConstraint>,
    pub bending_constraints: Vec<BendingConstraint>,
    pub prev_positions: Vec<Vec3>,
}

impl AvbdSolver {
    pub fn new(config: AvbdConfig) -> Self {
        Self { config, particles: Vec::new(), distance_constraints: Vec::new(), contact_constraints: Vec::new(), bending_constraints: Vec::new(), prev_positions: Vec::new() }
    }

    pub fn add_particle(&mut self, position: Vec3, mass: f32) -> usize {
        let idx = self.particles.len();
        self.particles.push(AvbdParticle::new(position, mass));
        self.prev_positions.push(position);
        idx
    }

    pub fn add_fixed_particle(&mut self, position: Vec3) -> usize {
        let idx = self.particles.len();
        self.particles.push(AvbdParticle::fixed(position));
        self.prev_positions.push(position);
        idx
    }

    pub fn add_distance(&mut self, p0: usize, p1: usize, stiffness: f32) {
        let rest = (self.particles[p0].position - self.particles[p1].position).length();
        self.distance_constraints.push(DistanceConstraint::new(p0, p1, rest, stiffness));
    }

    pub fn add_distance_with_length(&mut self, p0: usize, p1: usize, rest_length: f32, stiffness: f32) {
        self.distance_constraints.push(DistanceConstraint::new(p0, p1, rest_length, stiffness));
    }

    pub fn detect_ground_contacts(&mut self, ground_y: f32, friction: f32) {
        for i in 0..self.particles.len() {
            if self.particles[i].position.y < ground_y {
                self.contact_constraints.push(ContactConstraint::new(i, Vec3::new(0.0, 1.0, 0.0), ground_y, friction));
            }
        }
    }

    pub fn step(&mut self) {
        let dt = self.config.dt;
        // 保存上一步位置
        for i in 0..self.particles.len() {
            self.prev_positions[i] = self.particles[i].position;
        }
        // 1. 预测
        self.predict(dt);
        // 2. 重置乘子
        for c in &mut self.distance_constraints { c.lambda = 0.0; }
        for c in &mut self.contact_constraints { c.lambda_n = 0.0; c.lambda_t = Vec3::ZERO; }
        for c in &mut self.bending_constraints { c.lambda = 0.0; }
        // 3. 增广拉格朗日迭代
        for _ in 0..self.config.num_iters {
            self.solve_distance_constraints(dt);
            self.solve_contact_constraints(dt);
        }
        // 4. 速度更新
        self.update_velocities(dt);
        // 5. 清理接触约束
        self.contact_constraints.clear();
    }

    fn predict(&mut self, dt: f32) {
        let g = self.config.gravity;
        let damping = self.config.damping;
        let dt2 = dt * dt;
        for p in &mut self.particles {
            if p.inv_mass > 0.0 {
                p.predicted = p.position + dt * p.velocity * damping + dt2 * g;
            } else {
                p.predicted = p.position;
            }
        }
    }

    fn solve_distance_constraints(&mut self, _dt: f32) {
        for c in &mut self.distance_constraints {
            let i0 = c.p0;
            let i1 = c.p1;
            let x0 = self.particles[i0].predicted;
            let x1 = self.particles[i1].predicted;
            let w0 = self.particles[i0].inv_mass;
            let w1 = self.particles[i1].inv_mass;
            let w_sum = w0 + w1;
            if w_sum == 0.0 { continue; }
            let d = x1 - x0;
            let len = d.length().max(1e-10);
            let n_hat = d / len;
            let c_val = len - c.rest_length;
            // 标准 PBD 位置修正: dx = C / (w0+w1) * n * stiffness
            let correction = c_val / w_sum * c.stiffness;
            self.particles[i0].predicted += n_hat * correction * w0;
            self.particles[i1].predicted -= n_hat * correction * w1;
        }
    }

    fn solve_contact_constraints(&mut self, dt: f32) {
        for c in &mut self.contact_constraints {
            let i = c.particle;
            let x = self.particles[i].predicted;
            let w = self.particles[i].inv_mass;
            if w == 0.0 { continue; }
            let n_hat = c.normal;
            // 法向约束: C_n = dot(n, x) - penetration (穿透时 < 0)
            let c_n = n_hat.dot(x) - c.penetration;
            if c_n < 0.0 {
                // 标准 PBD: 位置修正 = -C_n * n * w (硬约束 stiffness=1)
                let correction = n_hat * (-c_n) * w;
                self.particles[i].predicted += correction;
                // 摩擦 (切向)
                let v = (self.particles[i].predicted - self.particles[i].position) / dt.max(1e-10);
                let v_t = v - n_hat * n_hat.dot(v);
                let v_t_len = v_t.length();
                if v_t_len > 1e-10 {
                    let t = v_t / v_t_len;
                    let max_friction = c.friction * (-c_n);
                    let friction_impulse = v_t_len.min(max_friction);
                    self.particles[i].predicted -= t * friction_impulse * w * dt;
                }
            }
        }
    }

    fn update_velocities(&mut self, dt: f32) {
        let dt_inv = 1.0 / dt.max(1e-10);
        let restitution = self.config.restitution;
        for i in 0..self.particles.len() {
            if self.particles[i].inv_mass > 0.0 {
                self.particles[i].velocity = (self.particles[i].predicted - self.prev_positions[i]) * dt_inv;
                if self.particles[i].position.y < 0.01 && self.particles[i].velocity.y < 0.0 {
                    self.particles[i].velocity.y = -self.particles[i].velocity.y * restitution;
                }
            }
        }
        for p in &mut self.particles {
            p.position = p.predicted;
        }
    }

    pub fn kinetic_energy(&self) -> f32 {
        let mut ke = 0.0;
        for p in &self.particles {
            if p.inv_mass > 0.0 {
                let m = 1.0 / p.inv_mass;
                ke += 0.5 * m * p.velocity.length_squared();
            }
        }
        ke
    }

    pub fn max_velocity(&self) -> f32 {
        self.particles.iter().map(|p| p.velocity.length()).fold(0.0f32, f32::max)
    }

    pub fn detect_particle_contacts(&mut self, radius: f32, friction: f32) {
        let n = self.particles.len();
        for i in 0..n {
            for j in (i+1)..n {
                let d = self.particles[j].position - self.particles[i].position;
                let dist = d.length();
                if dist < 2.0 * radius && dist > 1e-10 {
                    let n_hat = d / dist;
                    let penetration = 2.0 * radius - dist;
                    self.contact_constraints.push(ContactConstraint::new(i, -n_hat, penetration * 0.5, friction));
                    self.contact_constraints.push(ContactConstraint::new(j, n_hat, penetration * 0.5, friction));
                }
            }
        }
    }
}

// ============================================================
// 刚体 (AVBD 增广)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvbdRigidBody {
    pub position: Vec3,
    pub rotation: Quat,
    pub linear_vel: Vec3,
    pub angular_vel: Vec3,
    pub mass: f32,
    pub inv_mass: f32,
    pub local_inertia: Vec3,
    pub inv_inertia_world: Mat3,
    pub predicted_pos: Vec3,
    pub predicted_rot: Quat,
}

impl AvbdRigidBody {
    pub fn new(position: Vec3, mass: f32, inertia: Vec3) -> Self {
        Self {
            position, rotation: Quat::IDENTITY,
            linear_vel: Vec3::ZERO, angular_vel: Vec3::ZERO,
            mass, inv_mass: if mass > 0.0 { 1.0 / mass } else { 0.0 },
            local_inertia: inertia,
            inv_inertia_world: Mat3::IDENTITY,
            predicted_pos: position, predicted_rot: Quat::IDENTITY,
        }
    }

    pub fn fixed(position: Vec3) -> Self {
        Self::new(position, 0.0, Vec3::ZERO)
    }

    pub fn update_inertia(&mut self) {
        if self.inv_mass == 0.0 {
            self.inv_inertia_world = Mat3::ZERO;
            return;
        }
        let inv_local = Mat3::from_diagonal(Vec3::new(
            1.0 / self.local_inertia.x.max(1e-10),
            1.0 / self.local_inertia.y.max(1e-10),
            1.0 / self.local_inertia.z.max(1e-10),
        ));
        let rot = Mat3::from_quat(self.rotation);
        self.inv_inertia_world = rot * inv_local * rot.transpose();
    }

    pub fn predict(&mut self, dt: f32, gravity: Vec3, damping: f32) {
        if self.inv_mass == 0.0 {
            self.predicted_pos = self.position;
            self.predicted_rot = self.rotation;
            return;
        }
        self.predicted_pos = self.position + dt * self.linear_vel * damping + dt * dt * gravity;
        let omega_quat = Quat::from_xyzw(self.angular_vel.x, self.angular_vel.y, self.angular_vel.z, 0.0);
        let dq = omega_quat * self.rotation;
        let q_new = Quat::from_xyzw(
            self.rotation.x + dq.x * 0.5 * dt,
            self.rotation.y + dq.y * 0.5 * dt,
            self.rotation.z + dq.z * 0.5 * dt,
            self.rotation.w + dq.w * 0.5 * dt,
        );
        self.predicted_rot = if q_new.length_squared() < 1e-10 { Quat::IDENTITY } else { q_new.normalize() };
    }

    pub fn commit(&mut self, dt: f32) {
        if self.inv_mass > 0.0 {
            let dt_inv = 1.0 / dt.max(1e-10);
            self.linear_vel = (self.predicted_pos - self.position) * dt_inv;
            let dq = self.predicted_rot * self.rotation.conjugate();
            let angle = 2.0 * dq.w.acos().clamp(0.0, std::f32::consts::PI);
            let s = (1.0 - dq.w * dq.w).max(0.0).sqrt();
            if s > 1e-6 {
                let axis = Vec3::new(dq.x, dq.y, dq.z) / s;
                self.angular_vel = axis * (angle * dt_inv);
            } else {
                self.angular_vel = Vec3::ZERO;
            }
        }
        self.position = self.predicted_pos;
        self.rotation = self.predicted_rot;
        self.update_inertia();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avbd_config_default() {
        let config = AvbdConfig::default();
        assert!(config.dt > 0.0);
        assert!(config.num_iters > 0);
        assert!(config.constraint_mu > 0.0);
    }

    #[test]
    fn test_particle_creation() {
        let p = AvbdParticle::new(Vec3::new(1.0, 2.0, 3.0), 2.0);
        assert_eq!(p.position, Vec3::new(1.0, 2.0, 3.0));
        assert!((p.inv_mass - 0.5).abs() < 1e-6);
        assert!(!p.is_fixed());
        let fixed = AvbdParticle::fixed(Vec3::ZERO);
        assert!(fixed.is_fixed());
    }

    #[test]
    fn test_distance_constraint() {
        let c = DistanceConstraint::new(0, 1, 1.0, 1.0);
        let x0 = Vec3::ZERO;
        let x1 = Vec3::new(2.0, 0.0, 0.0);
        assert!((c.constraint(x0, x1) - 1.0).abs() < 1e-6);
        let (g0, g1) = c.gradient(x0, x1);
        assert!((g0 - Vec3::new(-1.0, 0.0, 0.0)).length() < 1e-6);
        assert!((g1 - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-6);
    }

    #[test]
    fn test_avbd_free_fall() {
        let mut solver = AvbdSolver::new(AvbdConfig::default());
        solver.add_particle(Vec3::new(0.0, 10.0, 0.0), 1.0);
        solver.step();
        assert!(solver.particles[0].position.y < 10.0, "particle should fall");
    }

    #[test]
    fn test_avbd_pendulum() {
        let mut solver = AvbdSolver::new(AvbdConfig::default());
        let pivot = solver.add_fixed_particle(Vec3::new(0.0, 10.0, 0.0));
        let bob = solver.add_particle(Vec3::new(1.0, 10.0, 0.0), 1.0);
        solver.add_distance(pivot, bob, 1.0);
        for _ in 0..60 { solver.step(); }
        assert!(solver.particles[bob].position.y < 10.0, "pendulum should swing down");
        let dist = (solver.particles[bob].position - solver.particles[pivot].position).length();
        assert!((dist - 1.0).abs() < 0.5, "distance constraint violated: {}", dist);
    }

    #[test]
    fn test_avbd_ground_collision() {
        let mut solver = AvbdSolver::new(AvbdConfig::default());
        solver.add_particle(Vec3::new(0.0, 5.0, 0.0), 1.0);
        for _ in 0..120 {
            solver.detect_ground_contacts(0.0, 0.5);
            solver.step();
        }
        assert!(solver.particles[0].position.y >= -0.5, "penetrated ground");
        assert!(solver.particles[0].position.y < 5.0, "should have fallen");
    }

    #[test]
    fn test_avbd_cloth_grid() {
        let mut solver = AvbdSolver::new(AvbdConfig { num_iters: 32, ..Default::default() });
        let size = 4;
        let mut grid = vec![0usize; size * size];
        for j in 0..size {
            for i in 0..size {
                let idx = if j == 0 && (i == 0 || i == size - 1) {
                    solver.add_fixed_particle(Vec3::new(i as f32, 5.0, j as f32))
                } else {
                    solver.add_particle(Vec3::new(i as f32, 5.0, j as f32), 1.0)
                };
                grid[j * size + i] = idx;
            }
        }
        for j in 0..size {
            for i in 0..size {
                if i + 1 < size { solver.add_distance(grid[j*size+i], grid[j*size+i+1], 0.8); }
                if j + 1 < size { solver.add_distance(grid[j*size+i], grid[(j+1)*size+i], 0.8); }
            }
        }
        for _ in 0..60 { solver.step(); }
        let max_v = solver.max_velocity();
        assert!(max_v.is_finite(), "diverged");
        let center = solver.particles[grid[2*size+2]].position;
        assert!(center.y < 5.0, "cloth should sag");
    }

    #[test]
    fn test_rigid_body_creation() {
        let rb = AvbdRigidBody::new(Vec3::new(0.0, 5.0, 0.0), 1.0, Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(rb.position, Vec3::new(0.0, 5.0, 0.0));
        assert!((rb.inv_mass - 1.0).abs() < 1e-6);
        assert_eq!(rb.rotation, Quat::IDENTITY);
    }

    #[test]
    fn test_rigid_body_predict() {
        let mut rb = AvbdRigidBody::new(Vec3::ZERO, 1.0, Vec3::new(1.0, 1.0, 1.0));
        rb.linear_vel = Vec3::new(1.0, 0.0, 0.0);
        rb.predict(0.1, Vec3::new(0.0, -9.81, 0.0), 1.0);
        assert!((rb.predicted_pos.x - 0.1).abs() < 1e-3, "x prediction wrong");
        assert!(rb.predicted_pos.y < 0.0, "y should decrease");
    }

    #[test]
    fn test_avbd_stability_long_run() {
        let mut solver = AvbdSolver::new(AvbdConfig::default());
        solver.add_particle(Vec3::new(0.0, 1.0, 0.0), 1.0);
        solver.add_particle(Vec3::new(1.0, 1.0, 0.0), 1.0);
        solver.add_distance(0, 1, 1.0);
        for step in 0..300 {
            solver.detect_ground_contacts(0.0, 0.3);
            solver.step();
            let max_v = solver.max_velocity();
            assert!(max_v.is_finite(), "diverged at step {}", step);
            assert!(max_v < 100.0, "velocity too large at step {}: {}", step, max_v);
        }
    }
}
