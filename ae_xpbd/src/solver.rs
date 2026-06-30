use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct XpbdParticle {
    pub position: Vec3,
    pub prev_position: Vec3,
    pub velocity: Vec3,
    pub mass: f32,
    pub inv_mass: f32,
    pub radius: f32,
    pub flags: u32,
}

impl XpbdParticle {
    pub fn new(position: Vec3, mass: f32) -> Self {
        let inv_mass = if mass <= 0.0 || mass.is_infinite() { 0.0 } else { 1.0 / mass };
        Self {
            position,
            prev_position: position,
            velocity: Vec3::ZERO,
            mass,
            inv_mass,
            radius: 0.1,
            flags: 0,
        }
    }

    pub fn is_static(&self) -> bool {
        self.inv_mass == 0.0
    }

    pub fn set_mass(&mut self, mass: f32) {
        self.mass = mass;
        self.inv_mass = if mass <= 0.0 || mass.is_infinite() { 0.0 } else { 1.0 / mass };
    }
}

#[derive(Debug, Clone)]
pub struct XpbdConfig {
    pub substeps: u32,
    pub gravity: Vec3,
    pub damping: f32,
    pub max_velocity: f32,
    pub relaxation: f32,
}

impl Default for XpbdConfig {
    fn default() -> Self {
        Self {
            substeps: 8,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            damping: 0.98,
            max_velocity: 100.0,
            relaxation: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct XpbdSolver {
    pub particles: Vec<XpbdParticle>,
    pub config: XpbdConfig,
    velocity_damping: f32,
}

impl XpbdSolver {
    pub fn new(config: XpbdConfig) -> Self {
        Self { particles: Vec::new(), config, velocity_damping: 0.0 }
    }

    pub fn add_particle(&mut self, particle: XpbdParticle) -> usize {
        let idx = self.particles.len();
        self.particles.push(particle);
        idx
    }

    pub fn step(&mut self, dt: f32, constraints: &mut [Box<dyn XpbdConstraint>]) {
        let sub_dt = dt / self.config.substeps as f32;
        self.velocity_damping = (1.0 - self.config.damping).powf(sub_dt);

        for _ in 0..self.config.substeps {
            self.integrate(sub_dt);
            self.solve_constraints(constraints, sub_dt);
            self.update_velocities(sub_dt);
            self.clamp_velocities();
        }
    }

    fn integrate(&mut self, dt: f32) {
        for p in &mut self.particles {
            if p.is_static() {
                continue;
            }
            p.velocity += self.config.gravity * dt;
            p.velocity *= self.velocity_damping;
            p.prev_position = p.position;
            p.position += p.velocity * dt;
        }
    }

    fn solve_constraints(&mut self, constraints: &mut [Box<dyn XpbdConstraint>], dt: f32) {
        for c in constraints.iter_mut() {
            c.solve(self, dt, self.config.relaxation);
        }
    }

    fn update_velocities(&mut self, dt: f32) {
        let inv_dt = 1.0 / dt;
        for p in &mut self.particles {
            if p.is_static() {
                continue;
            }
            p.velocity = (p.position - p.prev_position) * inv_dt;
        }
    }

    fn clamp_velocities(&mut self) {
        let max_v_sq = self.config.max_velocity * self.config.max_velocity;
        for p in &mut self.particles {
            if p.is_static() {
                continue;
            }
            if p.velocity.length_squared() > max_v_sq {
                p.velocity = p.velocity.normalize() * self.config.max_velocity;
            }
        }
    }

    pub fn get_particle(&self, index: usize) -> Option<&XpbdParticle> {
        self.particles.get(index)
    }

    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }
}

pub trait XpbdConstraint {
    fn solve(&mut self, solver: &mut XpbdSolver, dt: f32, relaxation: f32);
    fn compliance(&self) -> f32;
    fn set_compliance(&mut self, compliance: f32);
}

pub fn compute_lagrange_multiplier(
    compliance: f32,
    constraint: f32,
    dt: f32,
    total_inv_mass: f32,
) -> f32 {
    let alpha = compliance / (dt * dt);
    if total_inv_mass + alpha < 1e-12 {
        return 0.0;
    }
    -constraint / (total_inv_mass + alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_new() {
        let p = XpbdParticle::new(Vec3::new(1.0, 2.0, 3.0), 2.0);
        assert_eq!(p.mass, 2.0);
        assert!((p.inv_mass - 0.5).abs() < 0.001);
        assert_eq!(p.position, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_static_particle() {
        let mut p = XpbdParticle::new(Vec3::ZERO, 0.0);
        assert!(p.is_static());
        p.set_mass(f32::INFINITY);
        assert!(p.is_static());
    }

    #[test]
    fn test_lagrange_multiplier() {
        let lm = compute_lagrange_multiplier(0.0, -1.0, 0.016, 0.5);
        assert!(lm > 0.0);
    }

    #[test]
    fn test_solver_gravity() {
        let config = XpbdConfig::default();
        let mut solver = XpbdSolver::new(config);
        solver.add_particle(XpbdParticle::new(Vec3::ZERO, 1.0));

        let mut constraints: Vec<Box<dyn XpbdConstraint>> = Vec::new();
        solver.step(0.016, &mut constraints);

        let p = &solver.particles[0];
        assert!(p.velocity.y < 0.0);
    }

    #[test]
    fn test_static_particle_no_move() {
        let config = XpbdConfig::default();
        let mut solver = XpbdSolver::new(config);
        solver.add_particle(XpbdParticle::new(Vec3::ZERO, 0.0));

        let mut constraints: Vec<Box<dyn XpbdConstraint>> = Vec::new();
        let pos_before = solver.particles[0].position;
        solver.step(0.016, &mut constraints);
        let pos_after = solver.particles[0].position;

        assert!((pos_after - pos_before).length() < 0.001);
    }

    #[test]
    fn test_velocity_clamp() {
        let config = XpbdConfig {
            max_velocity: 5.0,
            gravity: Vec3::new(0.0, -1000.0, 0.0),
            ..Default::default()
        };
        let mut solver = XpbdSolver::new(config);
        solver.add_particle(XpbdParticle::new(Vec3::ZERO, 1.0));

        let mut constraints: Vec<Box<dyn XpbdConstraint>> = Vec::new();
        solver.step(0.016, &mut constraints);

        let p = &solver.particles[0];
        assert!(p.velocity.length() <= 5.0 + 0.01);
    }
}
