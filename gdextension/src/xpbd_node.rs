use godot::prelude::*;

use wasteland_xpbd::constraints::DistanceConstraint;
use wasteland_xpbd::solver::{XpbdConfig, XpbdParticle, XpbdSolver};

#[derive(GodotClass)]
#[class(base=Node)]
pub(crate) struct WastelandXPBD {
    #[var]
    substeps: i64,
    #[var]
    gravity_x: f32,
    #[var]
    gravity_y: f32,
    #[var]
    gravity_z: f32,
    #[var]
    damping: f32,
    #[var]
    max_velocity: f32,
    #[var]
    relaxation: f32,

    solver: XpbdSolver,
    constraints: Vec<Box<dyn wasteland_xpbd::solver::XpbdConstraint>>,
    particle_count: i64,
    constraint_count: i64,
    total_time: f32,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandXPBD {
    fn init(base: Base<Node>) -> Self {
        let config = XpbdConfig {
            substeps: 8,
            gravity: glam::Vec3::new(0.0, -9.81, 0.0),
            damping: 0.98,
            max_velocity: 100.0,
            relaxation: 1.0,
        };
        Self {
            substeps: 8,
            gravity_x: 0.0,
            gravity_y: -9.81,
            gravity_z: 0.0,
            damping: 0.98,
            max_velocity: 100.0,
            relaxation: 1.0,
            solver: XpbdSolver::new(config),
            constraints: Vec::new(),
            particle_count: 0,
            constraint_count: 0,
            total_time: 0.0,
            base,
        }
    }
}

#[godot_api]
impl WastelandXPBD {
    #[func]
    fn add_particle(&mut self, x: f32, y: f32, z: f32, mass: f32) -> i64 {
        let particle = XpbdParticle::new(glam::Vec3::new(x, y, z), mass);
        let id = self.solver.add_particle(particle);
        self.particle_count += 1;
        id as i64
    }

    #[func]
    fn add_distance_constraint(
        &mut self,
        particle_a: i64,
        particle_b: i64,
        rest_length: f32,
        compliance: f32,
    ) -> i64 {
        let constraint = DistanceConstraint::new(
            particle_a as usize,
            particle_b as usize,
            rest_length,
            compliance,
        );
        let id = self.constraints.len();
        self.constraints.push(Box::new(constraint));
        self.constraint_count += 1;
        id as i64
    }

    #[func]
    fn set_particle_mass(&mut self, particle_id: i64, mass: f32) {
        if let Some(p) = self.solver.particles.get_mut(particle_id as usize) {
            p.set_mass(mass);
        }
    }

    #[func]
    fn set_particle_velocity(&mut self, particle_id: i64, vx: f32, vy: f32, vz: f32) {
        if let Some(p) = self.solver.particles.get_mut(particle_id as usize) {
            p.velocity = glam::Vec3::new(vx, vy, vz);
        }
    }

    #[func]
    fn get_particle_position(&self, particle_id: i64) -> Vector3 {
        if let Some(p) = self.solver.get_particle(particle_id as usize) {
            Vector3::new(p.position.x, p.position.y, p.position.z)
        } else {
            Vector3::ZERO
        }
    }

    #[func]
    fn get_particle_velocity(&self, particle_id: i64) -> Vector3 {
        if let Some(p) = self.solver.get_particle(particle_id as usize) {
            Vector3::new(p.velocity.x, p.velocity.y, p.velocity.z)
        } else {
            Vector3::ZERO
        }
    }

    #[func]
    fn step(&mut self, delta_time: f32) {
        self.total_time += delta_time;
        self.solver.config.substeps = self.substeps as u32;
        self.solver.config.gravity =
            glam::Vec3::new(self.gravity_x, self.gravity_y, self.gravity_z);
        self.solver.config.damping = self.damping;
        self.solver.config.max_velocity = self.max_velocity;
        self.solver.config.relaxation = self.relaxation;
        self.solver.step(delta_time, &mut self.constraints);
    }

    #[func]
    fn remove_particle(&mut self, particle_id: i64) {
        let idx = particle_id as usize;
        if idx < self.solver.particles.len() {
            self.solver.particles.remove(idx);
            self.constraints.retain(|_c| true);
            self.particle_count = (self.particle_count - 1).max(0);
        }
    }

    #[func]
    fn remove_constraint(&mut self, constraint_id: i64) {
        let idx = constraint_id as usize;
        if idx < self.constraints.len() {
            self.constraints.remove(idx);
            self.constraint_count = (self.constraint_count - 1).max(0);
        }
    }

    #[func]
    fn clear(&mut self) {
        self.solver.particles.clear();
        self.constraints.clear();
        self.particle_count = 0;
        self.constraint_count = 0;
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        dict! {
            "particle_count" => self.particle_count,
            "constraint_count" => self.constraint_count,
            "substeps" => self.substeps,
            "damping" => self.damping,
            "total_time" => self.total_time,
        }
    }
}
