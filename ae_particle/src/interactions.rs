use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::particles::{ElementType, Particle};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionRule {
    pub element_a: ElementType,
    pub element_b: ElementType,
    pub force_type: ForceType,
    pub parameters: InteractionParameters,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ForceType {
    LennardJones,
    Coulombic,
    SpringBond,
    HardSphere,
    Yukawa,
    SoftRepulsion,
    Magnetic,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct InteractionParameters {
    pub epsilon: f32,
    pub sigma: f32,
    pub cutoff_distance: f32,
    pub attraction_strength: f32,
    pub repulsion_strength: f32,
}

impl InteractionParameters {
    pub fn van_der_waals(sigma: f32, epsilon: f32) -> Self {
        Self {
            epsilon,
            sigma,
            cutoff_distance: sigma * 3.0,
            attraction_strength: epsilon,
            repulsion_strength: epsilon,
        }
    }

    pub fn metallic_bond(element: ElementType) -> Self {
        let en = element.electronegativity();
        Self {
            epsilon: 2.0 / en,
            sigma: element.default_properties().1 * 2.0,
            cutoff_distance: element.default_properties().1 * 6.0,
            attraction_strength: 1.0 / en,
            repulsion_strength: en * 0.5,
        }
    }

    pub fn granular_interaction() -> Self {
        Self {
            epsilon: 1.0,
            sigma: 0.3,
            cutoff_distance: 0.9,
            attraction_strength: 0.1,
            repulsion_strength: 5.0,
        }
    }
}

impl InteractionRule {
    pub fn applies_to(&self, a: ElementType, b: ElementType) -> bool {
        (self.element_a == a && self.element_b == b) || (self.element_a == b && self.element_b == a)
    }

    pub fn compute_force(&self, a: &Particle, b: &Particle, dist: f32, delta: &Vec3) -> Vec3 {
        if dist > self.parameters.cutoff_distance || dist < 1e-6 {
            return Vec3::ZERO;
        }

        let dir = *delta / dist;

        match self.force_type {
            ForceType::LennardJones => {
                let sr = self.parameters.sigma / dist;
                let sr6 = sr.powi(6);
                let sr12 = sr6 * sr6;
                let force_mag = 24.0 * self.parameters.epsilon * (2.0 * sr12 - sr6) / dist;
                dir * force_mag
            },
            ForceType::Coulombic => {
                let k = 8.987_552e9;
                let force_mag = k * a.charge * b.charge / (dist * dist);
                dir * force_mag * 1e-10
            },
            ForceType::SpringBond => {
                let equilibrium = self.parameters.sigma;
                let force_mag = -self.parameters.epsilon * (dist - equilibrium);
                dir * force_mag
            },
            ForceType::HardSphere => {
                let min_dist = a.radius + b.radius;
                if dist < min_dist {
                    let overlap = min_dist - dist;
                    dir * (-self.parameters.repulsion_strength * overlap)
                } else {
                    Vec3::ZERO
                }
            },
            ForceType::Yukawa => {
                let screening = self.parameters.sigma;
                let force_mag = self.parameters.epsilon * (-dist / screening).exp() / dist;
                dir * force_mag
            },
            ForceType::SoftRepulsion => {
                let force_mag = self.parameters.repulsion_strength / (dist * dist);
                dir * force_mag
            },
            ForceType::Magnetic => {
                let force_mag = self.parameters.attraction_strength / (dist * dist);
                dir * force_mag
            },
        }
    }

    pub fn potential_energy(&self, dist: f32, _mass_a: f32, _mass_b: f32) -> f32 {
        if dist > self.parameters.cutoff_distance || dist < 1e-6 {
            return 0.0;
        }

        match self.force_type {
            ForceType::LennardJones => {
                let sr = self.parameters.sigma / dist;
                let sr6 = sr.powi(6);
                let sr12 = sr6 * sr6;
                4.0 * self.parameters.epsilon * (sr12 - sr6)
            },
            ForceType::Coulombic => {
                let k = 8.987_552e9;
                k * self.parameters.epsilon / dist
            },
            _ => 0.0,
        }
    }
}

impl Default for InteractionParameters {
    fn default() -> Self {
        Self {
            epsilon: 1.0,
            sigma: 1.0,
            cutoff_distance: 3.0,
            attraction_strength: 1.0,
            repulsion_strength: 1.0,
        }
    }
}

pub trait InteractionForce {
    fn compute(&self, a: &Particle, b: &Particle) -> Vec3;
    fn potential(&self, a: &Particle, b: &Particle) -> f32;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particles::Phase;
    use glam::Vec3;

    fn make_particle(element: ElementType, pos: Vec3, charge: f32, radius: f32) -> Particle {
        let mut p = Particle::new(element, pos, Phase::Solid);
        p.charge = charge;
        p.radius = radius;
        p
    }

    #[test]
    fn test_rule_applies_to_symmetric() {
        let rule = InteractionRule {
            element_a: ElementType::Iron,
            element_b: ElementType::Carbon,
            force_type: ForceType::LennardJones,
            parameters: InteractionParameters::default(),
        };
        assert!(rule.applies_to(ElementType::Iron, ElementType::Carbon));
        assert!(rule.applies_to(ElementType::Carbon, ElementType::Iron));
        assert!(!rule.applies_to(ElementType::Iron, ElementType::Copper));
        assert!(!rule.applies_to(ElementType::Copper, ElementType::Carbon));
    }

    #[test]
    fn test_lennard_jones_force() {
        let rule = InteractionRule {
            element_a: ElementType::Iron,
            element_b: ElementType::Iron,
            force_type: ForceType::LennardJones,
            parameters: InteractionParameters {
                epsilon: 1.0,
                sigma: 1.0,
                cutoff_distance: 10.0,
                attraction_strength: 1.0,
                repulsion_strength: 1.0,
            },
        };
        let a = make_particle(ElementType::Iron, Vec3::ZERO, 0.0, 0.5);
        let b = make_particle(ElementType::Iron, Vec3::new(1.0, 0.0, 0.0), 0.0, 0.5);
        let delta = b.position - a.position;
        let dist = delta.length();
        let force = rule.compute_force(&a, &b, dist, &delta);
        assert!(force.length() > 0.0);
    }

    #[test]
    fn test_lennard_jones_cutoff() {
        let rule = InteractionRule {
            element_a: ElementType::Iron,
            element_b: ElementType::Iron,
            force_type: ForceType::LennardJones,
            parameters: InteractionParameters {
                epsilon: 1.0,
                sigma: 1.0,
                cutoff_distance: 0.5,
                attraction_strength: 1.0,
                repulsion_strength: 1.0,
            },
        };
        let a = make_particle(ElementType::Iron, Vec3::ZERO, 0.0, 0.5);
        let b = make_particle(ElementType::Iron, Vec3::new(10.0, 0.0, 0.0), 0.0, 0.5);
        let delta = b.position - a.position;
        let dist = delta.length();
        let force = rule.compute_force(&a, &b, dist, &delta);
        assert_eq!(force, Vec3::ZERO);
    }

    #[test]
    fn test_coulombic_force() {
        let rule = InteractionRule {
            element_a: ElementType::Iron,
            element_b: ElementType::Iron,
            force_type: ForceType::Coulombic,
            parameters: InteractionParameters::default(),
        };
        let a = make_particle(ElementType::Iron, Vec3::ZERO, 1.0, 0.5);
        let b = make_particle(ElementType::Iron, Vec3::new(1.0, 0.0, 0.0), -1.0, 0.5);
        let delta = b.position - a.position;
        let dist = delta.length();
        let force = rule.compute_force(&a, &b, dist, &delta);
        assert!(force.length() > 0.0);
    }

    #[test]
    fn test_coulombic_same_sign_repels() {
        let rule = InteractionRule {
            element_a: ElementType::Iron,
            element_b: ElementType::Iron,
            force_type: ForceType::Coulombic,
            parameters: InteractionParameters::default(),
        };
        let a = make_particle(ElementType::Iron, Vec3::ZERO, 1.0, 0.5);
        let b = make_particle(ElementType::Iron, Vec3::new(1.0, 0.0, 0.0), 1.0, 0.5);
        let delta = b.position - a.position;
        let dist = delta.length();
        let force = rule.compute_force(&a, &b, dist, &delta);
        assert!(force.x > 0.0);
    }

    #[test]
    fn test_spring_bond_force() {
        let rule = InteractionRule {
            element_a: ElementType::Iron,
            element_b: ElementType::Iron,
            force_type: ForceType::SpringBond,
            parameters: InteractionParameters {
                epsilon: 10.0,
                sigma: 1.0,
                cutoff_distance: 10.0,
                attraction_strength: 1.0,
                repulsion_strength: 1.0,
            },
        };
        let a = make_particle(ElementType::Iron, Vec3::ZERO, 0.0, 0.5);
        let b = make_particle(ElementType::Iron, Vec3::new(2.0, 0.0, 0.0), 0.0, 0.5);
        let delta = b.position - a.position;
        let dist = delta.length();
        let force = rule.compute_force(&a, &b, dist, &delta);
        assert!(force.x < 0.0);
    }

    #[test]
    fn test_hard_sphere_force() {
        let rule = InteractionRule {
            element_a: ElementType::Iron,
            element_b: ElementType::Iron,
            force_type: ForceType::HardSphere,
            parameters: InteractionParameters {
                repulsion_strength: 100.0,
                ..InteractionParameters::default()
            },
        };
        let a = make_particle(ElementType::Iron, Vec3::ZERO, 0.0, 1.0);
        let b = make_particle(ElementType::Iron, Vec3::new(0.5, 0.0, 0.0), 0.0, 1.0);
        let delta = b.position - a.position;
        let dist = delta.length();
        let force = rule.compute_force(&a, &b, dist, &delta);
        assert!(force.x < 0.0);
    }

    #[test]
    fn test_hard_sphere_no_overlap() {
        let rule = InteractionRule {
            element_a: ElementType::Iron,
            element_b: ElementType::Iron,
            force_type: ForceType::HardSphere,
            parameters: InteractionParameters::default(),
        };
        let a = make_particle(ElementType::Iron, Vec3::ZERO, 0.0, 0.1);
        let b = make_particle(ElementType::Iron, Vec3::new(10.0, 0.0, 0.0), 0.0, 0.1);
        let delta = b.position - a.position;
        let dist = delta.length();
        let force = rule.compute_force(&a, &b, dist, &delta);
        assert_eq!(force, Vec3::ZERO);
    }

    #[test]
    fn test_yukawa_force() {
        let rule = InteractionRule {
            element_a: ElementType::Iron,
            element_b: ElementType::Iron,
            force_type: ForceType::Yukawa,
            parameters: InteractionParameters {
                epsilon: 1.0,
                sigma: 1.0,
                cutoff_distance: 10.0,
                attraction_strength: 1.0,
                repulsion_strength: 1.0,
            },
        };
        let a = make_particle(ElementType::Iron, Vec3::ZERO, 0.0, 0.5);
        let b = make_particle(ElementType::Iron, Vec3::new(1.0, 0.0, 0.0), 0.0, 0.5);
        let delta = b.position - a.position;
        let dist = delta.length();
        let force = rule.compute_force(&a, &b, dist, &delta);
        assert!(force.length() > 0.0);
    }

    #[test]
    fn test_soft_repulsion_force() {
        let rule = InteractionRule {
            element_a: ElementType::Iron,
            element_b: ElementType::Iron,
            force_type: ForceType::SoftRepulsion,
            parameters: InteractionParameters {
                repulsion_strength: 5.0,
                ..InteractionParameters::default()
            },
        };
        let a = make_particle(ElementType::Iron, Vec3::ZERO, 0.0, 0.5);
        let b = make_particle(ElementType::Iron, Vec3::new(1.0, 0.0, 0.0), 0.0, 0.5);
        let delta = b.position - a.position;
        let dist = delta.length();
        let force = rule.compute_force(&a, &b, dist, &delta);
        assert!(force.length() > 0.0);
    }

    #[test]
    fn test_magnetic_force() {
        let rule = InteractionRule {
            element_a: ElementType::Iron,
            element_b: ElementType::Iron,
            force_type: ForceType::Magnetic,
            parameters: InteractionParameters {
                attraction_strength: 1.0,
                ..InteractionParameters::default()
            },
        };
        let a = make_particle(ElementType::Iron, Vec3::ZERO, 0.0, 0.5);
        let b = make_particle(ElementType::Iron, Vec3::new(1.0, 0.0, 0.0), 0.0, 0.5);
        let delta = b.position - a.position;
        let dist = delta.length();
        let force = rule.compute_force(&a, &b, dist, &delta);
        assert!(force.length() > 0.0);
    }

    #[test]
    fn test_potential_energy_lennard_jones() {
        let rule = InteractionRule {
            element_a: ElementType::Iron,
            element_b: ElementType::Iron,
            force_type: ForceType::LennardJones,
            parameters: InteractionParameters {
                epsilon: 1.0,
                sigma: 1.0,
                cutoff_distance: 10.0,
                attraction_strength: 1.0,
                repulsion_strength: 1.0,
            },
        };
        let pe = rule.potential_energy(1.0, 1.0, 1.0);
        assert!((pe - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_potential_energy_cutoff() {
        let rule = InteractionRule {
            element_a: ElementType::Iron,
            element_b: ElementType::Iron,
            force_type: ForceType::LennardJones,
            parameters: InteractionParameters {
                cutoff_distance: 0.5,
                ..InteractionParameters::default()
            },
        };
        let pe = rule.potential_energy(10.0, 1.0, 1.0);
        assert_eq!(pe, 0.0);
    }

    #[test]
    fn test_van_der_waals_parameters() {
        let params = InteractionParameters::van_der_waals(1.0, 2.0);
        assert!((params.sigma - 1.0).abs() < 0.001);
        assert!((params.epsilon - 2.0).abs() < 0.001);
        assert!((params.cutoff_distance - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_metallic_bond_parameters() {
        let params = InteractionParameters::metallic_bond(ElementType::Iron);
        assert!(params.epsilon > 0.0);
        assert!(params.sigma > 0.0);
        assert!(params.cutoff_distance > 0.0);
    }

    #[test]
    fn test_granular_interaction_parameters() {
        let params = InteractionParameters::granular_interaction();
        assert!((params.epsilon - 1.0).abs() < 0.001);
        assert!((params.sigma - 0.3).abs() < 0.001);
        assert!((params.cutoff_distance - 0.9).abs() < 0.001);
        assert!(params.repulsion_strength > params.attraction_strength);
    }
}
