use crate::solver::{XpbdConstraint, XpbdSolver, compute_lagrange_multiplier};
use glam::Vec3;

pub struct DistanceConstraint {
    pub particle_a: usize,
    pub particle_b: usize,
    pub rest_length: f32,
    pub compliance: f32,
}

impl DistanceConstraint {
    pub fn new(particle_a: usize, particle_b: usize, rest_length: f32, compliance: f32) -> Self {
        Self { particle_a, particle_b, rest_length, compliance }
    }
}

impl XpbdConstraint for DistanceConstraint {
    fn solve(&mut self, solver: &mut XpbdSolver, dt: f32, relaxation: f32) {
        let (pa, pb) = get_two_mut(solver, self.particle_a, self.particle_b);
        let w_a = pa.inv_mass;
        let w_b = pb.inv_mass;
        let total_inv_mass = w_a + w_b;

        if total_inv_mass < 1e-12 {
            return;
        }

        let delta = pb.position - pa.position;
        let dist = delta.length();
        if dist < 1e-8 {
            return;
        }

        let n = delta / dist;
        let constraint = dist - self.rest_length;
        let lambda = compute_lagrange_multiplier(self.compliance, constraint, dt, total_inv_mass);
        let correction = lambda * n * relaxation;

        if w_a > 0.0 {
            pa.position -= correction * w_a;
        }
        if w_b > 0.0 {
            pb.position += correction * w_b;
        }
    }

    fn compliance(&self) -> f32 {
        self.compliance
    }

    fn set_compliance(&mut self, compliance: f32) {
        self.compliance = compliance;
    }
}

pub struct ContactConstraint {
    pub particle_a: usize,
    pub particle_b: usize,
    pub normal: Vec3,
    pub penetration: f32,
    pub friction: f32,
    pub compliance: f32,
    #[allow(dead_code)]
    position_a: Vec3,
    #[allow(dead_code)]
    position_b: Vec3,
    #[allow(dead_code)]
    normal_a: Vec3,
    #[allow(dead_code)]
    normal_b: Vec3,
}

impl ContactConstraint {
    pub fn new(
        particle_a: usize,
        particle_b: usize,
        normal: Vec3,
        penetration: f32,
        friction: f32,
        compliance: f32,
    ) -> Self {
        Self {
            particle_a,
            particle_b,
            normal,
            penetration,
            friction,
            compliance,
            position_a: Vec3::ZERO,
            position_b: Vec3::ZERO,
            normal_a: Vec3::ZERO,
            normal_b: Vec3::ZERO,
        }
    }

    pub fn with_anchors(
        particle_a: usize,
        particle_b: usize,
        position_a: Vec3,
        position_b: Vec3,
        normal_a: Vec3,
        normal_b: Vec3,
        friction: f32,
        compliance: f32,
    ) -> Self {
        Self {
            particle_a,
            particle_b,
            normal: Vec3::ZERO,
            penetration: 0.0,
            friction,
            compliance,
            position_a,
            position_b,
            normal_a,
            normal_b,
        }
    }
}

impl XpbdConstraint for ContactConstraint {
    fn solve(&mut self, solver: &mut XpbdSolver, dt: f32, relaxation: f32) {
        let (pa, pb) = get_two_mut(solver, self.particle_a, self.particle_b);
        let w_a = pa.inv_mass;
        let w_b = pb.inv_mass;
        let total_inv_mass = w_a + w_b;

        if total_inv_mass < 1e-12 {
            return;
        }

        let constraint = -self.penetration;
        let lambda = compute_lagrange_multiplier(self.compliance, constraint, dt, total_inv_mass);

        let correction = lambda * self.normal * relaxation;

        if w_a > 0.0 {
            pa.position += correction * w_a;
        }
        if w_b > 0.0 {
            pb.position -= correction * w_b;
        }

        if self.friction > 0.0 && w_a + w_b > 0.0 {
            let (pa2, pb2) = get_two_mut(solver, self.particle_a, self.particle_b);
            let rel_vel = pb2.velocity - pa2.velocity;
            let tangent_vel = rel_vel - self.normal * rel_vel.dot(self.normal);
            let tangent_vel_mag = tangent_vel.length();

            if tangent_vel_mag > 1e-6 {
                let max_friction = lambda.abs() * self.friction;
                let friction_impulse = tangent_vel_mag.min(max_friction * total_inv_mass);
                let friction_dir = -tangent_vel / tangent_vel_mag;

                if w_a > 0.0 {
                    pa2.velocity += friction_dir * friction_impulse * w_a;
                }
                if w_b > 0.0 {
                    pb2.velocity -= friction_dir * friction_impulse * w_b;
                }
            }
        }
    }

    fn compliance(&self) -> f32 {
        self.compliance
    }

    fn set_compliance(&mut self, compliance: f32) {
        self.compliance = compliance;
    }
}

pub struct AngleConstraint {
    pub particle_a: usize,
    pub particle_b: usize,
    pub particle_c: usize,
    pub rest_angle: f32,
    pub compliance: f32,
}

impl AngleConstraint {
    pub fn new(
        particle_a: usize,
        particle_b: usize,
        particle_c: usize,
        rest_angle: f32,
        compliance: f32,
    ) -> Self {
        Self { particle_a, particle_b, particle_c, rest_angle, compliance }
    }
}

impl XpbdConstraint for AngleConstraint {
    fn solve(&mut self, solver: &mut XpbdSolver, _dt: f32, relaxation: f32) {
        let wa = solver.particles[self.particle_a].inv_mass;
        let wb = solver.particles[self.particle_b].inv_mass;
        let wc = solver.particles[self.particle_c].inv_mass;

        let pa = solver.particles[self.particle_a].position;
        let pb = solver.particles[self.particle_b].position;
        let pc = solver.particles[self.particle_c].position;

        let ab = pa - pb;
        let cb = pc - pb;
        let ab_len = ab.length();
        let cb_len = cb.length();

        if ab_len < 1e-8 || cb_len < 1e-8 {
            return;
        }

        let ab_n = ab / ab_len;
        let cb_n = cb / cb_len;
        let cos_angle = ab_n.dot(cb_n).clamp(-1.0, 1.0);
        let angle = cos_angle.acos();
        let delta_angle = angle - self.rest_angle;

        let axis = ab_n.cross(cb_n);
        let axis_len = axis.length();
        if axis_len < 1e-8 {
            return;
        }
        let axis_n = axis / axis_len;

        let grad_a = axis_n.cross(ab_n) / ab_len;
        let grad_c = cb_n.cross(axis_n) / cb_len;
        let grad_b = -(grad_a + grad_c);

        let total_inv = wa * grad_a.length_squared()
            + wb * grad_b.length_squared()
            + wc * grad_c.length_squared();

        if total_inv < 1e-12 {
            return;
        }

        let multiplier = -delta_angle / (total_inv + self.compliance) * relaxation;

        if wa > 0.0 {
            solver.particles[self.particle_a].position += grad_a * multiplier * wa;
        }
        if wb > 0.0 {
            solver.particles[self.particle_b].position += grad_b * multiplier * wb;
        }
        if wc > 0.0 {
            solver.particles[self.particle_c].position += grad_c * multiplier * wc;
        }
    }

    fn compliance(&self) -> f32 {
        self.compliance
    }

    fn set_compliance(&mut self, compliance: f32) {
        self.compliance = compliance;
    }
}

pub struct VolumeConstraint {
    pub particles: Vec<usize>,
    pub rest_volume: f32,
    pub compliance: f32,
}

impl VolumeConstraint {
    pub fn new(particles: Vec<usize>, rest_volume: f32, compliance: f32) -> Self {
        Self { particles, rest_volume, compliance }
    }
}

impl XpbdConstraint for VolumeConstraint {
    fn solve(&mut self, solver: &mut XpbdSolver, _dt: f32, relaxation: f32) {
        if self.particles.len() < 4 {
            return;
        }

        let p0 = solver.particles[self.particles[0]].position;
        let p1 = solver.particles[self.particles[1]].position;
        let p2 = solver.particles[self.particles[2]].position;
        let p3 = solver.particles[self.particles[3]].position;

        let volume = (p1 - p0).cross(p2 - p0).dot(p3 - p0).abs() / 6.0;
        let delta = volume - self.rest_volume;

        let mut total_inv = 0.0f32;
        let mut grads: Vec<Vec3> = Vec::with_capacity(4);

        for i in 0..4 {
            let i0 = i;
            let i1 = (i + 1) % 4;
            let i2 = (i + 2) % 4;

            let pos = [p0, p1, p2, p3];
            let grad = (pos[i1] - pos[i0]).cross(pos[i2] - pos[i0]) / 6.0;
            grads.push(grad);

            let w = solver.particles[self.particles[i]].inv_mass;
            total_inv += w * grad.length_squared();
        }

        if total_inv < 1e-12 {
            return;
        }

        let multiplier = -delta / (total_inv + self.compliance) * relaxation;

        for (i, grad) in grads.iter().enumerate() {
            let w = solver.particles[self.particles[i]].inv_mass;
            if w > 0.0 {
                solver.particles[self.particles[i]].position += *grad * multiplier * w;
            }
        }
    }

    fn compliance(&self) -> f32 {
        self.compliance
    }

    fn set_compliance(&mut self, compliance: f32) {
        self.compliance = compliance;
    }
}

pub struct GroundConstraint {
    pub particle: usize,
    pub height: f32,
}

impl GroundConstraint {
    pub fn new(particle: usize, height: f32) -> Self {
        Self { particle, height }
    }
}

impl XpbdConstraint for GroundConstraint {
    fn solve(&mut self, solver: &mut XpbdSolver, _dt: f32, _relaxation: f32) {
        let p = &mut solver.particles[self.particle];
        if p.position.y < self.height {
            p.position.y = self.height;
            if p.velocity.y < 0.0 {
                p.velocity.y = 0.0;
            }
        }
    }

    fn compliance(&self) -> f32 {
        0.0
    }

    fn set_compliance(&mut self, _compliance: f32) {}
}

pub struct BendConstraint {
    pub particle_a: usize,
    pub particle_b: usize,
    pub particle_c: usize,
    pub particle_d: usize,
    pub rest_angle: f32,
    pub compliance: f32,
}

impl BendConstraint {
    pub fn new(
        particle_a: usize,
        particle_b: usize,
        particle_c: usize,
        particle_d: usize,
        rest_angle: f32,
        compliance: f32,
    ) -> Self {
        Self { particle_a, particle_b, particle_c, particle_d, rest_angle, compliance }
    }
}

impl XpbdConstraint for BendConstraint {
    fn solve(&mut self, solver: &mut XpbdSolver, _dt: f32, relaxation: f32) {
        let wa = solver.particles[self.particle_a].inv_mass;
        let wb = solver.particles[self.particle_b].inv_mass;
        let wc = solver.particles[self.particle_c].inv_mass;
        let wd = solver.particles[self.particle_d].inv_mass;

        let pa = solver.particles[self.particle_a].position;
        let pb = solver.particles[self.particle_b].position;
        let pc = solver.particles[self.particle_c].position;
        let pd = solver.particles[self.particle_d].position;

        let n1 = (pa - pb).cross(pc - pb);
        let n1_len = n1.length();
        let n2 = (pb - pc).cross(pd - pc);
        let n2_len = n2.length();

        if n1_len < 1e-8 || n2_len < 1e-8 {
            return;
        }

        let n1_n = n1 / n1_len;
        let n2_n = n2 / n2_len;
        let cos_angle = n1_n.dot(n2_n).clamp(-1.0, 1.0);
        let angle = cos_angle.acos();
        let delta = angle - self.rest_angle;

        let e = pb - pc;
        let e_len = e.length();
        if e_len < 1e-8 {
            return;
        }

        let grad_a = n1_n * e_len / n1_len;
        let grad_d = n2_n * e_len / n2_len;
        let grad_b =
            (pa - pb).dot(e) / (e_len * n1_len) * n1_n - (pd - pc).dot(e) / (e_len * n2_len) * n2_n;
        let grad_c =
            (pc - pb).dot(e) / (e_len * n1_len) * n1_n - (pc - pd).dot(e) / (e_len * n2_len) * n2_n;

        let total_inv = wa * grad_a.length_squared()
            + wb * grad_b.length_squared()
            + wc * grad_c.length_squared()
            + wd * grad_d.length_squared();

        if total_inv < 1e-12 {
            return;
        }

        let multiplier = -delta / (total_inv + self.compliance) * relaxation;

        if wa > 0.0 {
            solver.particles[self.particle_a].position += grad_a * multiplier * wa;
        }
        if wb > 0.0 {
            solver.particles[self.particle_b].position += grad_b * multiplier * wb;
        }
        if wc > 0.0 {
            solver.particles[self.particle_c].position += grad_c * multiplier * wc;
        }
        if wd > 0.0 {
            solver.particles[self.particle_d].position += grad_d * multiplier * wd;
        }
    }

    fn compliance(&self) -> f32 {
        self.compliance
    }

    fn set_compliance(&mut self, compliance: f32) {
        self.compliance = compliance;
    }
}

pub struct PinConstraint {
    pub particle: usize,
    pub position: Vec3,
    pub compliance: f32,
}

impl PinConstraint {
    pub fn new(particle: usize, position: Vec3, compliance: f32) -> Self {
        Self { particle, position, compliance }
    }
}

impl XpbdConstraint for PinConstraint {
    fn solve(&mut self, solver: &mut XpbdSolver, dt: f32, relaxation: f32) {
        let p = &mut solver.particles[self.particle];
        let w = p.inv_mass;
        if w < 1e-12 {
            return;
        }

        let delta = p.position - self.position;
        let constraint = delta.length();
        if constraint < 1e-8 {
            return;
        }

        let n = delta / constraint;
        let lambda = compute_lagrange_multiplier(self.compliance, constraint, dt, w);
        let correction = lambda * n * relaxation;
        p.position += correction * w;
    }

    fn compliance(&self) -> f32 {
        self.compliance
    }

    fn set_compliance(&mut self, compliance: f32) {
        self.compliance = compliance;
    }
}

pub struct SlideConstraint {
    pub particle: usize,
    pub origin: Vec3,
    pub direction: Vec3,
    pub compliance: f32,
}

impl SlideConstraint {
    pub fn new(particle: usize, origin: Vec3, direction: Vec3, compliance: f32) -> Self {
        let direction = direction.normalize();
        Self { particle, origin, direction, compliance }
    }
}

impl XpbdConstraint for SlideConstraint {
    fn solve(&mut self, solver: &mut XpbdSolver, dt: f32, relaxation: f32) {
        let p = &mut solver.particles[self.particle];
        let w = p.inv_mass;
        if w < 1e-12 {
            return;
        }

        let offset = p.position - self.origin;
        let proj = offset.dot(self.direction) * self.direction;
        let perpendicular = offset - proj;
        let constraint = perpendicular.length();
        if constraint < 1e-8 {
            return;
        }

        let n = perpendicular / constraint;
        let lambda = compute_lagrange_multiplier(self.compliance, constraint, dt, w);
        let correction = lambda * n * relaxation;
        p.position += correction * w;
    }

    fn compliance(&self) -> f32 {
        self.compliance
    }

    fn set_compliance(&mut self, compliance: f32) {
        self.compliance = compliance;
    }
}

pub struct ShapeMatchingConstraint {
    pub particles: Vec<usize>,
    pub rest_positions: Vec<Vec3>,
    pub rest_center: Vec3,
    pub stiffness: f32,
    pub compliance: f32,
}

impl ShapeMatchingConstraint {
    pub fn new(
        particles: Vec<usize>,
        rest_positions: Vec<Vec3>,
        stiffness: f32,
        compliance: f32,
    ) -> Self {
        let rest_center = if rest_positions.is_empty() {
            Vec3::ZERO
        } else {
            rest_positions.iter().sum::<Vec3>() / rest_positions.len() as f32
        };
        Self { particles, rest_positions, rest_center, stiffness, compliance }
    }
}

impl XpbdConstraint for ShapeMatchingConstraint {
    fn solve(&mut self, solver: &mut XpbdSolver, _dt: f32, _relaxation: f32) {
        let n = self.particles.len();
        if n == 0 {
            return;
        }

        let total_mass: f32 = self.particles.iter().map(|&i| solver.particles[i].mass).sum();
        if total_mass < 1e-12 {
            return;
        }

        let mut current_center = Vec3::ZERO;
        for &i in &self.particles {
            current_center += solver.particles[i].position * solver.particles[i].mass;
        }
        current_center /= total_mass;

        let mut apq = glam::Mat3::ZERO;
        for j in 0..n {
            let i = self.particles[j];
            let p = solver.particles[i].position - current_center;
            let q = self.rest_positions[j] - self.rest_center;
            apq.x_axis += q * p.x * solver.particles[i].mass;
            apq.y_axis += q * p.y * solver.particles[i].mass;
            apq.z_axis += q * p.z * solver.particles[i].mass;
        }

        let rotation = extract_rotation(apq);

        for j in 0..n {
            let i = self.particles[j];
            let w = solver.particles[i].inv_mass;
            if w < 1e-12 {
                continue;
            }

            let goal = current_center + rotation * (self.rest_positions[j] - self.rest_center);
            let correction = (goal - solver.particles[i].position) * self.stiffness;
            solver.particles[i].position += correction;
        }
    }

    fn compliance(&self) -> f32 {
        self.compliance
    }

    fn set_compliance(&mut self, compliance: f32) {
        self.compliance = compliance;
    }
}

fn extract_rotation(a: glam::Mat3) -> glam::Quat {
    let det = a.determinant();
    if det.abs() < 1e-12 {
        return glam::Quat::IDENTITY;
    }
    let mut r = a;
    for _ in 0..4 {
        let r_t = r.transpose();
        let r_t_r = r_t * r;
        let det_r = r_t_r.determinant();
        if det_r.abs() < 1e-12 {
            break;
        }
        let inv = r_t_r.inverse();
        r *= glam::Mat3::IDENTITY * 1.5 - inv * r_t_r * 0.5;
    }
    glam::Quat::from_mat3(&r)
}

pub struct SphereCollisionConstraint {
    pub particle_a: usize,
    pub particle_b: usize,
    pub radius_a: f32,
    pub radius_b: f32,
    pub compliance: f32,
}

impl SphereCollisionConstraint {
    pub fn new(
        particle_a: usize,
        particle_b: usize,
        radius_a: f32,
        radius_b: f32,
        compliance: f32,
    ) -> Self {
        Self { particle_a, particle_b, radius_a, radius_b, compliance }
    }
}

impl XpbdConstraint for SphereCollisionConstraint {
    fn solve(&mut self, solver: &mut XpbdSolver, dt: f32, relaxation: f32) {
        let (pa, pb) = get_two_mut(solver, self.particle_a, self.particle_b);
        let w_a = pa.inv_mass;
        let w_b = pb.inv_mass;
        let total_inv_mass = w_a + w_b;

        if total_inv_mass < 1e-12 {
            return;
        }

        let delta = pb.position - pa.position;
        let dist = delta.length();
        let min_dist = self.radius_a + self.radius_b;

        if dist >= min_dist || dist < 1e-8 {
            return;
        }

        let n = delta / dist;
        let constraint = dist - min_dist;
        let lambda = compute_lagrange_multiplier(self.compliance, constraint, dt, total_inv_mass);
        let correction = lambda * n * relaxation;

        if w_a > 0.0 {
            pa.position -= correction * w_a;
        }
        if w_b > 0.0 {
            pb.position += correction * w_b;
        }
    }

    fn compliance(&self) -> f32 {
        self.compliance
    }

    fn set_compliance(&mut self, compliance: f32) {
        self.compliance = compliance;
    }
}

fn get_two_mut(
    solver: &mut XpbdSolver,
    a: usize,
    b: usize,
) -> (&mut crate::solver::XpbdParticle, &mut crate::solver::XpbdParticle) {
    assert_ne!(a, b, "DistanceConstraint: particles must be different");
    if a < b {
        let (left, right) = solver.particles.split_at_mut(b);
        (&mut left[a], &mut right[0])
    } else {
        let (left, right) = solver.particles.split_at_mut(a);
        (&mut right[0], &mut left[b])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::XpbdParticle;
    use crate::solver::XpbdSolver;

    #[test]
    fn test_distance_constraint() {
        let mut solver = XpbdSolver::new(Default::default());
        let a = solver.add_particle(XpbdParticle::new(Vec3::new(0.0, 0.0, 0.0), 1.0));
        let b = solver.add_particle(XpbdParticle::new(Vec3::new(3.0, 0.0, 0.0), 1.0));

        let mut constraints: Vec<Box<dyn XpbdConstraint>> =
            vec![Box::new(DistanceConstraint::new(a, b, 2.0, 0.0))];

        solver.step(0.016, &mut constraints);

        let dist = (solver.particles[a].position - solver.particles[b].position).length();
        assert!((dist - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_contact_constraint_penetration() {
        let mut solver = XpbdSolver::new(Default::default());
        let a = solver.add_particle(XpbdParticle::new(Vec3::new(0.0, 0.0, 0.0), 1.0));
        let b = solver.add_particle(XpbdParticle::new(Vec3::new(0.5, 0.0, 0.0), 1.0));

        let mut constraints: Vec<Box<dyn XpbdConstraint>> =
            vec![Box::new(ContactConstraint::new(a, b, Vec3::new(1.0, 0.0, 0.0), 0.5, 0.3, 0.0))];

        let pos_a_before = solver.particles[a].position;
        let pos_b_before = solver.particles[b].position;
        solver.step(0.016, &mut constraints);
        let dist_after = (solver.particles[a].position - solver.particles[b].position).length();
        let dist_before = (pos_a_before - pos_b_before).length();
        assert!(dist_after > dist_before);
    }

    #[test]
    fn test_ground_constraint() {
        let mut solver = XpbdSolver::new(Default::default());
        let a = solver.add_particle(XpbdParticle::new(Vec3::new(0.0, -5.0, 0.0), 1.0));

        let mut constraints: Vec<Box<dyn XpbdConstraint>> =
            vec![Box::new(GroundConstraint::new(a, 0.0))];

        solver.step(0.016, &mut constraints);
        assert!(solver.particles[a].position.y >= 0.0);
    }

    #[test]
    fn test_angle_constraint() {
        let mut solver = XpbdSolver::new(Default::default());
        let a = solver.add_particle(XpbdParticle::new(Vec3::new(1.0, 0.0, 0.0), 1.0));
        let b = solver.add_particle(XpbdParticle::new(Vec3::new(0.0, 0.0, 0.0), 1.0));
        let c = solver.add_particle(XpbdParticle::new(Vec3::new(0.0, 1.0, 0.0), 1.0));

        let mut constraints: Vec<Box<dyn XpbdConstraint>> =
            vec![Box::new(AngleConstraint::new(a, b, c, std::f32::consts::FRAC_PI_2, 0.0))];

        solver.step(0.016, &mut constraints);
        let pa = solver.particles[a].position;
        let pb = solver.particles[b].position;
        let pc = solver.particles[c].position;
        let ab = (pa - pb).normalize();
        let cb = (pc - pb).normalize();
        let angle = ab.dot(cb).acos();
        assert!((angle - std::f32::consts::FRAC_PI_2).abs() < 0.2);
    }

    #[test]
    fn test_volume_constraint() {
        let mut solver = XpbdSolver::new(Default::default());
        let p0 = solver.add_particle(XpbdParticle::new(Vec3::new(0.0, 0.0, 0.0), 1.0));
        let p1 = solver.add_particle(XpbdParticle::new(Vec3::new(2.0, 0.0, 0.0), 1.0));
        let p2 = solver.add_particle(XpbdParticle::new(Vec3::new(0.0, 2.0, 0.0), 1.0));
        let p3 = solver.add_particle(XpbdParticle::new(Vec3::new(0.0, 0.0, 2.0), 1.0));

        let rest_volume = 8.0 / 6.0;
        let mut constraints: Vec<Box<dyn XpbdConstraint>> =
            vec![Box::new(VolumeConstraint::new(vec![p0, p1, p2, p3], rest_volume, 0.0))];

        solver.step(0.016, &mut constraints);
        let pos0 = solver.particles[p0].position;
        let pos1 = solver.particles[p1].position;
        let pos2 = solver.particles[p2].position;
        let pos3 = solver.particles[p3].position;
        let vol = (pos1 - pos0).cross(pos2 - pos0).dot(pos3 - pos0).abs() / 6.0;
        assert!((vol - rest_volume).abs() < 0.5);
    }

    #[test]
    fn test_bend_constraint() {
        let mut solver = XpbdSolver::new(Default::default());
        let a = solver.add_particle(XpbdParticle::new(Vec3::new(1.0, 0.0, 0.0), 1.0));
        let b = solver.add_particle(XpbdParticle::new(Vec3::new(0.0, 0.0, 0.0), 1.0));
        let c = solver.add_particle(XpbdParticle::new(Vec3::new(0.0, 0.0, 1.0), 1.0));
        let d = solver.add_particle(XpbdParticle::new(Vec3::new(1.0, 0.0, 1.0), 1.0));

        let mut constraints: Vec<Box<dyn XpbdConstraint>> =
            vec![Box::new(BendConstraint::new(a, b, c, d, 0.0, 0.0))];

        solver.step(0.016, &mut constraints);
        let pa = solver.particles[a].position;
        let pb = solver.particles[b].position;
        let pc = solver.particles[c].position;
        let pd = solver.particles[d].position;
        let n1 = (pa - pb).cross(pc - pb);
        let n2 = (pb - pc).cross(pd - pc);
        if n1.length() > 1e-6 && n2.length() > 1e-6 {
            let cos_angle = n1.normalize().dot(n2.normalize()).clamp(-1.0, 1.0);
            assert!(cos_angle > 0.9);
        }
    }

    #[test]
    fn test_pin_constraint() {
        let mut solver = XpbdSolver::new(Default::default());
        let a = solver.add_particle(XpbdParticle::new(Vec3::new(5.0, 0.0, 0.0), 1.0));

        let pin_pos = Vec3::new(0.0, 0.0, 0.0);
        let mut constraints: Vec<Box<dyn XpbdConstraint>> =
            vec![Box::new(PinConstraint::new(a, pin_pos, 0.0))];

        let dist_before = (solver.particles[a].position - pin_pos).length();
        solver.step(0.016, &mut constraints);
        let dist_after = (solver.particles[a].position - pin_pos).length();
        assert!(dist_after < dist_before);
    }

    #[test]
    fn test_slide_constraint() {
        let mut solver = XpbdSolver::new(Default::default());
        let a = solver.add_particle(XpbdParticle::new(Vec3::new(3.0, 5.0, 0.0), 1.0));

        let origin = Vec3::ZERO;
        let direction = Vec3::X;
        let mut constraints: Vec<Box<dyn XpbdConstraint>> =
            vec![Box::new(SlideConstraint::new(a, origin, direction, 0.0))];

        let pos_before = solver.particles[a].position;
        let off_before = pos_before - origin;
        let perp_before = (off_before - off_before.dot(direction) * direction).length();
        solver.step(0.016, &mut constraints);
        let pos_after = solver.particles[a].position;
        let off_after = pos_after - origin;
        let perp_after = (off_after - off_after.dot(direction) * direction).length();
        assert!(perp_after < perp_before);
    }

    #[test]
    fn test_shape_matching_constraint() {
        let mut solver = XpbdSolver::new(Default::default());
        let rest_positions = vec![
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(-1.0, 1.0, 0.0),
        ];
        let particles: Vec<usize> = rest_positions
            .iter()
            .map(|p| {
                solver.add_particle(XpbdParticle::new(*p * 2.0 + Vec3::new(3.0, 0.0, 0.0), 1.0))
            })
            .collect();

        let rest_d01 = (rest_positions[0] - rest_positions[1]).length();
        let d01_before = (solver.particles[particles[0]].position
            - solver.particles[particles[1]].position)
            .length();
        assert!(
            (d01_before - rest_d01).abs() > 0.5,
            "particles should be deformed before constraint"
        );

        let mut constraints: Vec<Box<dyn XpbdConstraint>> = vec![Box::new(
            ShapeMatchingConstraint::new(particles.clone(), rest_positions.clone(), 0.3, 0.0),
        )];

        solver.step(0.016, &mut constraints);

        let d01_after = (solver.particles[particles[0]].position
            - solver.particles[particles[1]].position)
            .length();
        assert!(
            (d01_after - rest_d01).abs() < (d01_before - rest_d01).abs() + 0.01,
            "shape matching should reduce deformation"
        );
    }

    #[test]
    fn test_sphere_collision_constraint() {
        let mut solver = XpbdSolver::new(Default::default());
        let a = solver.add_particle(XpbdParticle::new(Vec3::new(0.0, 0.0, 0.0), 1.0));
        let b = solver.add_particle(XpbdParticle::new(Vec3::new(0.5, 0.0, 0.0), 1.0));

        let mut constraints: Vec<Box<dyn XpbdConstraint>> =
            vec![Box::new(SphereCollisionConstraint::new(a, b, 1.0, 1.0, 0.0))];

        solver.step(0.016, &mut constraints);
        let dist = (solver.particles[a].position - solver.particles[b].position).length();
        assert!(dist >= 1.8);
    }
}
