use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::interactions::InteractionRule;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Particle {
    pub id: Uuid,
    pub position: Vec3,
    pub velocity: Vec3,
    pub mass: f32,
    pub radius: f32,
    pub element_type: ElementType,
    pub phase: Phase,
    pub temperature: f32,
    pub charge: f32,
    pub bonds: Vec<ParticleBond>,
    pub lifetime: f32,
    pub age: f32,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ElementType {
    Iron,
    Carbon,
    Silicon,
    Oxygen,
    Hydrogen,
    Nitrogen,
    Calcium,
    Phosphorus,
    Sulfur,
    Sodium,
    Potassium,
    Magnesium,
    Copper,
    Zinc,
    Lead,
    Uranium,
    Custom(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Phase {
    Solid,
    Liquid,
    Gas,
    Plasma,
    Granular,
    CrystalLattice { spacing: f32 },
    Amorphous,
}

impl Eq for Phase {}

impl PartialOrd for Phase {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Phase {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        fn phase_discriminant(p: &Phase) -> u8 {
            match p {
                Phase::Solid => 0,
                Phase::Liquid => 1,
                Phase::Gas => 2,
                Phase::Plasma => 3,
                Phase::Granular => 4,
                Phase::CrystalLattice { .. } => 5,
                Phase::Amorphous => 6,
            }
        }
        let d = phase_discriminant(self).cmp(&phase_discriminant(other));
        if d != std::cmp::Ordering::Equal {
            return d;
        }
        if let (Phase::CrystalLattice { spacing: a }, Phase::CrystalLattice { spacing: b }) = (self, other) {
            a.total_cmp(b)
        } else {
            std::cmp::Ordering::Equal
        }
    }
}

impl std::hash::Hash for Phase {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        if let Phase::CrystalLattice { spacing } = self {
            spacing.to_bits().hash(state);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleBond {
    pub target_id: Uuid,
    pub bond_type: BondType,
    pub strength: f32,
    pub equilibrium_distance: f32,
    pub max_distance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BondType {
    Covalent,
    Ionic,
    Metallic,
    VanDerWaals,
    HydrogenBond,
    Mechanical,
    Magnetic,
}

impl Particle {
    pub fn new(element: ElementType, position: Vec3, phase: Phase) -> Self {
        let (mass, radius) = element.default_properties();
        Self {
            id: Uuid::new_v4(),
            position,
            velocity: Vec3::ZERO,
            mass,
            radius,
            element_type: element,
            phase,
            temperature: 293.0,
            charge: 0.0,
            bonds: Vec::new(),
            lifetime: f32::MAX,
            age: 0.0,
            active: true,
        }
    }

    pub fn kinetic_energy(&self) -> f32 {
        0.5 * self.mass * self.velocity.length_squared()
    }

    pub fn potential_energy(&self, others: &[Particle], rules: &[InteractionRule]) -> f32 {
        let mut pe = 0.0f32;
        for other in others {
            if other.id == self.id || !other.active {
                continue;
            }
            let dist = (self.position - other.position).length();
            if dist > 0.0 {
                for rule in rules {
                    if rule.applies_to(self.element_type, other.element_type) {
                        pe += rule.potential_energy(dist, self.mass, other.mass);
                    }
                }
            }
        }
        pe
    }

    pub fn bond_energy(&self) -> f32 {
        self.bonds.iter().map(|b| b.strength * 0.1).sum()
    }

    pub fn total_energy(&self, others: &[Particle], rules: &[InteractionRule]) -> f32 {
        self.kinetic_energy() + self.potential_energy(others, rules) + self.bond_energy()
    }

    pub fn apply_force(&mut self, force: Vec3, dt: f32) {
        let acceleration = force / self.mass;
        self.velocity += acceleration * dt;
        self.position += self.velocity * dt;
        self.age += dt;
    }

    pub fn dampen(&mut self, factor: f32) {
        self.velocity *= 1.0 - factor;
    }
}

impl ElementType {
    pub fn default_properties(&self) -> (f32, f32) {
        match self {
            ElementType::Iron => (55.845, 0.126),
            ElementType::Carbon => (12.011, 0.07),
            ElementType::Silicon => (28.085, 0.111),
            ElementType::Oxygen => (15.999, 0.066),
            ElementType::Hydrogen => (1.008, 0.053),
            ElementType::Nitrogen => (14.007, 0.065),
            ElementType::Calcium => (40.078, 0.194),
            ElementType::Phosphorus => (30.974, 0.098),
            ElementType::Sulfur => (32.065, 0.088),
            ElementType::Sodium => (22.990, 0.186),
            ElementType::Potassium => (39.098, 0.227),
            ElementType::Magnesium => (24.305, 0.160),
            ElementType::Copper => (63.546, 0.128),
            ElementType::Zinc => (65.380, 0.134),
            ElementType::Lead => (207.200, 0.175),
            ElementType::Uranium => (238.029, 0.156),
            ElementType::Custom(id) => (10.0 + *id as f32 * 0.5, 0.1),
        }
    }

    pub fn electronegativity(&self) -> f32 {
        match self {
            ElementType::Iron => 1.83,
            ElementType::Carbon => 2.55,
            ElementType::Silicon => 1.90,
            ElementType::Oxygen => 3.44,
            ElementType::Hydrogen => 2.20,
            ElementType::Nitrogen => 3.04,
            ElementType::Calcium => 1.00,
            ElementType::Phosphorus => 2.19,
            ElementType::Sulfur => 2.58,
            ElementType::Sodium => 0.93,
            ElementType::Potassium => 0.82,
            ElementType::Magnesium => 1.31,
            ElementType::Copper => 1.90,
            ElementType::Zinc => 1.65,
            ElementType::Lead => 2.33,
            ElementType::Uranium => 1.38,
            ElementType::Custom(_) => 1.5,
        }
    }

    pub fn melting_point(&self) -> f32 {
        match self {
            ElementType::Iron => 1811.0,
            ElementType::Carbon => 3823.0,
            ElementType::Silicon => 1687.0,
            ElementType::Oxygen => 54.0,
            ElementType::Hydrogen => 14.0,
            ElementType::Nitrogen => 63.0,
            ElementType::Calcium => 1115.0,
            ElementType::Copper => 1358.0,
            ElementType::Zinc => 693.0,
            ElementType::Lead => 601.0,
            ElementType::Uranium => 1405.0,
            _ => 1000.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleSystem {
    pub particles: Vec<Particle>,
    pub interaction_rules: Vec<InteractionRule>,
    pub bounds: ParticleBounds,
    pub gravity: Vec3,
    pub global_temperature: f32,
    pub damping: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ParticleBounds {
    pub min: Vec3,
    pub max: Vec3,
    pub boundary_type: ParticleBoundaryType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticleBoundaryType {
    Absorbing,
    Reflecting,
    Periodic,
    None,
}

impl ParticleSystem {
    pub fn new(bounds: ParticleBounds) -> Self {
        Self {
            particles: Vec::new(),
            interaction_rules: Vec::new(),
            bounds,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            global_temperature: 293.0,
            damping: 0.001,
        }
    }

    pub fn add_particle(&mut self, particle: Particle) -> Uuid {
        let id = particle.id;
        self.particles.push(particle);
        id
    }

    pub fn spawn_crystal_lattice(
        &mut self,
        element: ElementType,
        origin: Vec3,
        spacing: f32,
        dims: [u32; 3],
    ) -> Vec<Uuid> {
        let mut ids = Vec::new();
        for z in 0..dims[2] {
            for y in 0..dims[1] {
                for x in 0..dims[0] {
                    let offset =
                        if (x + y + z) % 2 == 0 { Vec3::ZERO } else { Vec3::splat(spacing * 0.5) };
                    let pos = origin
                        + Vec3::new(x as f32 * spacing, y as f32 * spacing, z as f32 * spacing)
                        + offset;
                    let mut particle =
                        Particle::new(element, pos, Phase::CrystalLattice { spacing });
                    particle.mass = element.default_properties().0;
                    particle.radius = spacing * 0.4;
                    ids.push(self.add_particle(particle));
                }
            }
        }
        ids
    }

    pub fn spawn_granular(
        &mut self,
        element: ElementType,
        region_center: Vec3,
        region_size: Vec3,
        count: usize,
    ) -> Vec<Uuid> {
        let mut ids = Vec::new();
        for _ in 0..count {
            let pos = region_center
                + Vec3::new(
                    (rand::random::<f32>() - 0.5) * region_size.x,
                    (rand::random::<f32>() - 0.5) * region_size.y,
                    (rand::random::<f32>() - 0.5) * region_size.z,
                );
            let mut particle = Particle::new(element, pos, Phase::Granular);
            particle.radius = element.default_properties().1 * 2.0;
            particle.velocity = Vec3::new(
                (rand::random::<f32>() - 0.5) * 2.0,
                (rand::random::<f32>() - 0.5) * 2.0,
                (rand::random::<f32>() - 0.5) * 2.0,
            );
            ids.push(self.add_particle(particle));
        }
        ids
    }

    pub fn step(&mut self, dt: f32) {
        let rule_count = self.interaction_rules.len();

        for i in 0..self.particles.len() {
            if !self.particles[i].active {
                continue;
            }

            let mut total_force = self.gravity * self.particles[i].mass;

            for j in 0..self.particles.len() {
                if i == j || !self.particles[j].active {
                    continue;
                }

                let delta = self.particles[j].position - self.particles[i].position;
                let dist = delta.length();
                if dist < 1e-6 {
                    continue;
                }

                for r in 0..rule_count {
                    if self.interaction_rules[r]
                        .applies_to(self.particles[i].element_type, self.particles[j].element_type)
                    {
                        let force = self.interaction_rules[r].compute_force(
                            &self.particles[i],
                            &self.particles[j],
                            dist,
                            &delta,
                        );
                        total_force += force;
                    }
                }
            }

            self.particles[i].apply_force(total_force, dt);
            self.particles[i].dampen(self.damping);

            self.apply_boundary(i);
        }

        for i in 0..self.particles.len() {
            // age is already incremented in apply_force() above (line 172).
            // Only check lifetime expiry here.
            if self.particles[i].age > self.particles[i].lifetime {
                self.particles[i].active = false;
            }
        }
    }

    fn apply_boundary(&mut self, i: usize) {
        let p = &mut self.particles[i];
        match self.bounds.boundary_type {
            ParticleBoundaryType::Reflecting => {
                if p.position.x < self.bounds.min.x {
                    p.position.x = self.bounds.min.x;
                    p.velocity.x = p.velocity.x.abs();
                }
                if p.position.x > self.bounds.max.x {
                    p.position.x = self.bounds.max.x;
                    p.velocity.x = -p.velocity.x.abs();
                }
                if p.position.y < self.bounds.min.y {
                    p.position.y = self.bounds.min.y;
                    p.velocity.y = p.velocity.y.abs();
                }
                if p.position.y > self.bounds.max.y {
                    p.position.y = self.bounds.max.y;
                    p.velocity.y = -p.velocity.y.abs();
                }
                if p.position.z < self.bounds.min.z {
                    p.position.z = self.bounds.min.z;
                    p.velocity.z = p.velocity.z.abs();
                }
                if p.position.z > self.bounds.max.z {
                    p.position.z = self.bounds.max.z;
                    p.velocity.z = -p.velocity.z.abs();
                }
            },
            ParticleBoundaryType::Absorbing => {
                if p.position.x < self.bounds.min.x
                    || p.position.x > self.bounds.max.x
                    || p.position.y < self.bounds.min.y
                    || p.position.y > self.bounds.max.y
                    || p.position.z < self.bounds.min.z
                    || p.position.z > self.bounds.max.z
                {
                    p.active = false;
                }
            },
            ParticleBoundaryType::Periodic => {
                if p.position.x < self.bounds.min.x {
                    p.position.x += self.bounds.max.x - self.bounds.min.x;
                }
                if p.position.x > self.bounds.max.x {
                    p.position.x -= self.bounds.max.x - self.bounds.min.x;
                }
                if p.position.y < self.bounds.min.y {
                    p.position.y += self.bounds.max.y - self.bounds.min.y;
                }
                if p.position.y > self.bounds.max.y {
                    p.position.y -= self.bounds.max.y - self.bounds.min.y;
                }
                if p.position.z < self.bounds.min.z {
                    p.position.z += self.bounds.max.z - self.bounds.min.z;
                }
                if p.position.z > self.bounds.max.z {
                    p.position.z -= self.bounds.max.z - self.bounds.min.z;
                }
            },
            ParticleBoundaryType::None => {},
        }
    }

    pub fn active_count(&self) -> usize {
        self.particles.iter().filter(|p| p.active).count()
    }

    pub fn get_particles_by_element(&self, element: ElementType) -> Vec<&Particle> {
        self.particles.iter().filter(|p| p.active && p.element_type == element).collect()
    }

    pub fn compute_center_of_mass(&self, element: ElementType) -> Option<Vec3> {
        let particles = self.get_particles_by_element(element);
        if particles.is_empty() {
            return None;
        }
        let total_mass: f32 = particles.iter().map(|p| p.mass).sum();
        let com: Vec3 = particles.iter().map(|p| p.position * p.mass).sum();
        Some(com / total_mass)
    }

    pub fn compute_average_temperature(&self) -> f32 {
        let active: Vec<&Particle> = self.particles.iter().filter(|p| p.active).collect();
        if active.is_empty() {
            return self.global_temperature;
        }
        active.iter().map(|p| p.temperature).sum::<f32>() / active.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn make_particle() -> Particle {
        Particle::new(ElementType::Iron, Vec3::new(0.0, 0.0, 0.0), Phase::Solid)
    }

    #[test]
    fn test_particle_new_defaults() {
        let p = make_particle();
        assert!(p.active);
        assert_eq!(p.velocity, Vec3::ZERO);
        assert_eq!(p.temperature, 293.0);
        assert_eq!(p.charge, 0.0);
        assert!(p.bonds.is_empty());
        assert_eq!(p.lifetime, f32::MAX);
        assert_eq!(p.age, 0.0);
        let (mass, radius) = ElementType::Iron.default_properties();
        assert!((p.mass - mass).abs() < 0.001);
        assert!((p.radius - radius).abs() < 0.001);
    }

    #[test]
    fn test_particle_kinetic_energy() {
        let mut p = make_particle();
        assert_eq!(p.kinetic_energy(), 0.0);
        p.velocity = Vec3::new(2.0, 0.0, 0.0);
        let expected = 0.5 * p.mass * 4.0;
        assert!((p.kinetic_energy() - expected).abs() < 0.001);
    }

    #[test]
    fn test_particle_apply_force() {
        let mut p = make_particle();
        let force = Vec3::new(10.0, 0.0, 0.0);
        let dt = 0.1;
        p.apply_force(force, dt);
        let expected_acc = force / p.mass;
        let expected_vel = expected_acc * dt;
        assert!((p.velocity.x - expected_vel.x).abs() < 0.001);
        assert!((p.position.x - expected_vel.x * dt).abs() < 0.001);
        assert!((p.age - dt).abs() < 0.001);
    }

    #[test]
    fn test_particle_dampen() {
        let mut p = make_particle();
        p.velocity = Vec3::new(10.0, 0.0, 0.0);
        p.dampen(0.5);
        assert!((p.velocity.x - 5.0).abs() < 0.001);
        p.dampen(0.0);
        assert!((p.velocity.x - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_particle_bond_energy() {
        let mut p = make_particle();
        assert_eq!(p.bond_energy(), 0.0);
        let other = Particle::new(ElementType::Carbon, Vec3::new(1.0, 0.0, 0.0), Phase::Solid);
        p.bonds.push(ParticleBond {
            target_id: other.id,
            bond_type: BondType::Covalent,
            strength: 10.0,
            equilibrium_distance: 0.5,
            max_distance: 1.0,
        });
        assert!((p.bond_energy() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_element_default_properties() {
        let (mass, radius) = ElementType::Hydrogen.default_properties();
        assert!((mass - 1.008).abs() < 0.001);
        assert!((radius - 0.053).abs() < 0.001);
        let (mass, radius) = ElementType::Custom(10).default_properties();
        assert!((mass - 15.0).abs() < 0.001);
        assert!((radius - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_element_electronegativity() {
        assert!((ElementType::Oxygen.electronegativity() - 3.44).abs() < 0.001);
        assert!((ElementType::Iron.electronegativity() - 1.83).abs() < 0.001);
        assert!((ElementType::Custom(0).electronegativity() - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_element_melting_point() {
        assert!((ElementType::Iron.melting_point() - 1811.0).abs() < 0.001);
        assert!((ElementType::Carbon.melting_point() - 3823.0).abs() < 0.001);
        assert!((ElementType::Sulfur.melting_point() - 1000.0).abs() < 0.001);
    }

    #[test]
    fn test_system_new() {
        let bounds = ParticleBounds {
            min: Vec3::new(-10.0, -10.0, -10.0),
            max: Vec3::new(10.0, 10.0, 10.0),
            boundary_type: ParticleBoundaryType::Reflecting,
        };
        let system = ParticleSystem::new(bounds);
        assert!(system.particles.is_empty());
        assert!(system.interaction_rules.is_empty());
        assert_eq!(system.gravity, Vec3::new(0.0, -9.81, 0.0));
        assert_eq!(system.global_temperature, 293.0);
    }

    #[test]
    fn test_system_add_particle() {
        let bounds = ParticleBounds {
            min: Vec3::new(-10.0, -10.0, -10.0),
            max: Vec3::new(10.0, 10.0, 10.0),
            boundary_type: ParticleBoundaryType::None,
        };
        let mut system = ParticleSystem::new(bounds);
        let p = make_particle();
        let id = system.add_particle(p);
        assert_eq!(system.particles.len(), 1);
        assert_eq!(system.particles[0].id, id);
    }

    #[test]
    fn test_spawn_crystal_lattice() {
        let bounds = ParticleBounds {
            min: Vec3::new(-10.0, -10.0, -10.0),
            max: Vec3::new(10.0, 10.0, 10.0),
            boundary_type: ParticleBoundaryType::None,
        };
        let mut system = ParticleSystem::new(bounds);
        let ids = system.spawn_crystal_lattice(ElementType::Iron, Vec3::ZERO, 1.0, [2, 2, 2]);
        assert_eq!(ids.len(), 8);
        for id in &ids {
            let p = system.particles.iter().find(|p| &p.id == id).unwrap();
            assert_eq!(p.element_type, ElementType::Iron);
            assert!(matches!(p.phase, Phase::CrystalLattice { .. }));
        }
    }

    #[test]
    fn test_spawn_granular() {
        let bounds = ParticleBounds {
            min: Vec3::new(-10.0, -10.0, -10.0),
            max: Vec3::new(10.0, 10.0, 10.0),
            boundary_type: ParticleBoundaryType::None,
        };
        let mut system = ParticleSystem::new(bounds);
        let ids =
            system.spawn_granular(ElementType::Silicon, Vec3::ZERO, Vec3::new(5.0, 5.0, 5.0), 10);
        assert_eq!(ids.len(), 10);
        for id in &ids {
            let p = system.particles.iter().find(|p| &p.id == id).unwrap();
            assert_eq!(p.element_type, ElementType::Silicon);
            assert_eq!(p.phase, Phase::Granular);
        }
    }

    #[test]
    fn test_active_count() {
        let bounds = ParticleBounds {
            min: Vec3::new(-10.0, -10.0, -10.0),
            max: Vec3::new(10.0, 10.0, 10.0),
            boundary_type: ParticleBoundaryType::None,
        };
        let mut system = ParticleSystem::new(bounds);
        let p1 = make_particle();
        let mut p2 = Particle::new(ElementType::Carbon, Vec3::new(1.0, 0.0, 0.0), Phase::Solid);
        p2.active = false;
        system.add_particle(p1);
        system.add_particle(p2);
        assert_eq!(system.active_count(), 1);
    }

    #[test]
    fn test_get_particles_by_element() {
        let bounds = ParticleBounds {
            min: Vec3::new(-10.0, -10.0, -10.0),
            max: Vec3::new(10.0, 10.0, 10.0),
            boundary_type: ParticleBoundaryType::None,
        };
        let mut system = ParticleSystem::new(bounds);
        system.add_particle(make_particle());
        system.add_particle(Particle::new(
            ElementType::Carbon,
            Vec3::new(1.0, 0.0, 0.0),
            Phase::Solid,
        ));
        let iron = system.get_particles_by_element(ElementType::Iron);
        assert_eq!(iron.len(), 1);
        let carbon = system.get_particles_by_element(ElementType::Carbon);
        assert_eq!(carbon.len(), 1);
        let none = system.get_particles_by_element(ElementType::Copper);
        assert!(none.is_empty());
    }

    #[test]
    fn test_compute_center_of_mass() {
        let bounds = ParticleBounds {
            min: Vec3::new(-10.0, -10.0, -10.0),
            max: Vec3::new(10.0, 10.0, 10.0),
            boundary_type: ParticleBoundaryType::None,
        };
        let mut system = ParticleSystem::new(bounds);
        system.add_particle(make_particle());
        let mut p2 = Particle::new(ElementType::Iron, Vec3::new(2.0, 0.0, 0.0), Phase::Solid);
        p2.mass = 1.0;
        system.add_particle(p2);
        let com = system.compute_center_of_mass(ElementType::Iron).unwrap();
        let m1 = ElementType::Iron.default_properties().0;
        let expected_x = (0.0 * m1 + 2.0 * 1.0) / (m1 + 1.0);
        assert!((com.x - expected_x).abs() < 0.001);
    }

    #[test]
    fn test_compute_average_temperature() {
        let bounds = ParticleBounds {
            min: Vec3::new(-10.0, -10.0, -10.0),
            max: Vec3::new(10.0, 10.0, 10.0),
            boundary_type: ParticleBoundaryType::None,
        };
        let mut system = ParticleSystem::new(bounds);
        system.add_particle(make_particle());
        let mut p2 = Particle::new(ElementType::Carbon, Vec3::new(1.0, 0.0, 0.0), Phase::Solid);
        p2.temperature = 393.0;
        system.add_particle(p2);
        let avg = system.compute_average_temperature();
        assert!((avg - (293.0 + 393.0) / 2.0).abs() < 0.001);
    }

    #[test]
    fn test_reflecting_boundary() {
        let bounds = ParticleBounds {
            min: Vec3::new(-5.0, -5.0, -5.0),
            max: Vec3::new(5.0, 5.0, 5.0),
            boundary_type: ParticleBoundaryType::Reflecting,
        };
        let mut system = ParticleSystem::new(bounds);
        let mut p = make_particle();
        p.position = Vec3::new(6.0, 0.0, 0.0);
        p.velocity = Vec3::new(3.0, 0.0, 0.0);
        system.add_particle(p);
        system.step(0.016);
        assert!(system.particles[0].position.x <= 5.0);
        assert!(system.particles[0].active);
    }

    #[test]
    fn test_absorbing_boundary() {
        let bounds = ParticleBounds {
            min: Vec3::new(-5.0, -5.0, -5.0),
            max: Vec3::new(5.0, 5.0, 5.0),
            boundary_type: ParticleBoundaryType::Absorbing,
        };
        let mut system = ParticleSystem::new(bounds);
        let mut p = make_particle();
        p.position = Vec3::new(6.0, 0.0, 0.0);
        p.velocity = Vec3::ZERO;
        system.add_particle(p);
        system.step(0.016);
        assert!(!system.particles[0].active);
    }

    #[test]
    fn test_periodic_boundary() {
        let bounds = ParticleBounds {
            min: Vec3::new(-5.0, -5.0, -5.0),
            max: Vec3::new(5.0, 5.0, 5.0),
            boundary_type: ParticleBoundaryType::Periodic,
        };
        let mut system = ParticleSystem::new(bounds);
        let mut p = make_particle();
        p.position = Vec3::new(6.0, 0.0, 0.0);
        p.velocity = Vec3::ZERO;
        system.add_particle(p);
        system.step(0.016);
        assert!(system.particles[0].position.x < 5.0);
        assert!(system.particles[0].position.x >= -5.0);
    }

    #[test]
    fn test_lifetime_expiry() {
        let bounds = ParticleBounds {
            min: Vec3::new(-10.0, -10.0, -10.0),
            max: Vec3::new(10.0, 10.0, 10.0),
            boundary_type: ParticleBoundaryType::None,
        };
        let mut system = ParticleSystem::new(bounds);
        let mut p = make_particle();
        p.lifetime = 0.01;
        system.add_particle(p);
        system.step(0.1);
        assert!(!system.particles[0].active);
    }
}
