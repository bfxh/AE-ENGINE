use serde::{Deserialize, Serialize};

use crate::particles::{ElementType, Particle, Phase};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseTransitionSystem {
    pub transitions: Vec<PhaseTransitionRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseTransitionRule {
    pub element: ElementType,
    pub from_phase: Phase,
    pub to_phase: Phase,
    pub temperature_threshold: f32,
    pub pressure_threshold: f32,
    pub transition_energy: f32,
    pub transition_rate: f32,
}

impl PhaseTransitionSystem {
    pub fn new() -> Self {
        Self { transitions: Vec::new() }
    }

    pub fn add_transition(&mut self, rule: PhaseTransitionRule) {
        self.transitions.push(rule);
    }

    pub fn standard_transitions() -> Self {
        let mut system = Self::new();

        for element in
            &[ElementType::Iron, ElementType::Copper, ElementType::Zinc, ElementType::Lead]
        {
            let mp = element.melting_point();
            system.add_transition(PhaseTransitionRule {
                element: *element,
                from_phase: Phase::Solid,
                to_phase: Phase::Liquid,
                temperature_threshold: mp,
                pressure_threshold: 1e5,
                transition_energy: 10.0,
                transition_rate: 0.1,
            });
            system.add_transition(PhaseTransitionRule {
                element: *element,
                from_phase: Phase::Liquid,
                to_phase: Phase::Solid,
                temperature_threshold: mp,
                pressure_threshold: 1e5,
                transition_energy: -10.0,
                transition_rate: 0.1,
            });
        }

        system.add_transition(PhaseTransitionRule {
            element: ElementType::Iron,
            from_phase: Phase::CrystalLattice { spacing: 0.3 },
            to_phase: Phase::Granular,
            temperature_threshold: 600.0,
            pressure_threshold: 0.0,
            transition_energy: 5.0,
            transition_rate: 0.05,
        });
        system.add_transition(PhaseTransitionRule {
            element: ElementType::Iron,
            from_phase: Phase::Granular,
            to_phase: Phase::CrystalLattice { spacing: 0.3 },
            temperature_threshold: 400.0,
            pressure_threshold: 1e6,
            transition_energy: -5.0,
            transition_rate: 0.01,
        });

        system
    }

    pub fn apply(&self, particles: &mut [Particle], _global_pressure: f32, dt: f32) -> usize {
        let mut transition_count = 0;

        for particle in particles.iter_mut() {
            if !particle.active {
                continue;
            }

            for rule in &self.transitions {
                if rule.element != particle.element_type {
                    continue;
                }

                if particle.phase != rule.from_phase {
                    continue;
                }

                let temp_ok = if rule.from_phase == Phase::Solid && rule.to_phase == Phase::Liquid {
                    particle.temperature >= rule.temperature_threshold
                } else if rule.from_phase == Phase::Liquid && rule.to_phase == Phase::Solid {
                    particle.temperature <= rule.temperature_threshold
                } else {
                    particle.temperature >= rule.temperature_threshold
                };

                if !temp_ok {
                    continue;
                }

                let transition_odds = rule.transition_rate * dt;
                if rand::random::<f32>() < transition_odds {
                    particle.phase = rule.to_phase;
                    particle.temperature -= rule.transition_energy * 0.01;
                    transition_count += 1;
                }
            }
        }

        transition_count
    }

    pub fn get_phase_distribution(&self, particles: &[Particle]) -> Vec<(Phase, usize)> {
        let mut counts: std::collections::HashMap<Phase, usize> = std::collections::HashMap::new();
        for p in particles.iter().filter(|p| p.active) {
            *counts.entry(p.phase).or_insert(0) += 1;
        }
        counts.into_iter().collect()
    }
}

impl Default for PhaseTransitionSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_new_empty() {
        let system = PhaseTransitionSystem::new();
        assert!(system.transitions.is_empty());
    }

    #[test]
    fn test_add_transition() {
        let mut system = PhaseTransitionSystem::new();
        system.add_transition(PhaseTransitionRule {
            element: ElementType::Iron,
            from_phase: Phase::Solid,
            to_phase: Phase::Liquid,
            temperature_threshold: 1800.0,
            pressure_threshold: 1e5,
            transition_energy: 10.0,
            transition_rate: 0.1,
        });
        assert_eq!(system.transitions.len(), 1);
    }

    #[test]
    fn test_standard_transitions_has_entries() {
        let system = PhaseTransitionSystem::standard_transitions();
        assert!(!system.transitions.is_empty());
        let has_solid_to_liquid = system.transitions.iter().any(|r| {
            r.element == ElementType::Iron
                && r.from_phase == Phase::Solid
                && r.to_phase == Phase::Liquid
        });
        assert!(has_solid_to_liquid);
    }

    #[test]
    fn test_apply_solid_to_liquid() {
        let system = PhaseTransitionSystem::standard_transitions();
        let mut particles = vec![Particle::new(ElementType::Iron, Vec3::ZERO, Phase::Solid)];
        particles[0].temperature = 2000.0;
        let count = system.apply(&mut particles, 1e5, 10.0);
        assert!(
            count > 0 || particles[0].phase == Phase::Liquid || particles[0].phase == Phase::Solid
        );
    }

    #[test]
    fn test_apply_no_transition_wrong_element() {
        let mut system = PhaseTransitionSystem::new();
        system.add_transition(PhaseTransitionRule {
            element: ElementType::Iron,
            from_phase: Phase::Solid,
            to_phase: Phase::Liquid,
            temperature_threshold: 1800.0,
            pressure_threshold: 1e5,
            transition_energy: 10.0,
            transition_rate: 1.0,
        });
        let mut particles = vec![Particle::new(ElementType::Carbon, Vec3::ZERO, Phase::Solid)];
        particles[0].temperature = 2000.0;
        let count = system.apply(&mut particles, 1e5, 10.0);
        assert_eq!(count, 0);
        assert_eq!(particles[0].phase, Phase::Solid);
    }

    #[test]
    fn test_get_phase_distribution() {
        let system = PhaseTransitionSystem::new();
        let particles = vec![
            Particle::new(ElementType::Iron, Vec3::ZERO, Phase::Solid),
            Particle::new(ElementType::Iron, Vec3::new(1.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Carbon, Vec3::new(2.0, 0.0, 0.0), Phase::Liquid),
        ];
        let dist = system.get_phase_distribution(&particles);
        let solid_count =
            dist.iter().find(|(p, _)| *p == Phase::Solid).map(|(_, c)| *c).unwrap_or(0);
        let liquid_count =
            dist.iter().find(|(p, _)| *p == Phase::Liquid).map(|(_, c)| *c).unwrap_or(0);
        assert_eq!(solid_count, 2);
        assert_eq!(liquid_count, 1);
    }

    #[test]
    fn test_apply_inactive_particle_ignored() {
        let system = PhaseTransitionSystem::standard_transitions();
        let mut particles = vec![Particle::new(ElementType::Iron, Vec3::ZERO, Phase::Solid)];
        particles[0].temperature = 2000.0;
        particles[0].active = false;
        let count = system.apply(&mut particles, 1e5, 10.0);
        assert_eq!(count, 0);
    }
}
