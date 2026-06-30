use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::interactions::{ForceType, InteractionParameters, InteractionRule};
use crate::particles::{ElementType, Particle, Phase};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergentRuleEngine {
    pub discovered_rules: Vec<DiscoveredRule>,
    pub element_pair_stats: HashMap<(ElementType, ElementType), PairStatistics>,
    pub observation_window: usize,
    pub min_observations: usize,
    pub significance_threshold: f32,
    pub tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredRule {
    pub element_a: ElementType,
    pub element_b: ElementType,
    pub rule_type: EmergentRuleType,
    pub confidence: f32,
    pub discovery_tick: u64,
    pub parameters: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum EmergentRuleType {
    AttractionForce,
    RepulsionForce,
    BondingPair,
    CatalyticActivity,
    GrowthPattern,
    CollectiveMotion,
    PhaseThreshold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairStatistics {
    pub collision_count: u64,
    pub total_distance: f32,
    pub min_distance: f32,
    pub avg_relative_velocity: f32,
    pub bond_count: u64,
    pub temperature_correlation: f32,
    pub phase_cooccurrence: HashMap<(Phase, Phase), u64>,
    pub samples: Vec<f32>,
}

impl Default for PairStatistics {
    fn default() -> Self {
        Self {
            collision_count: 0,
            total_distance: 0.0,
            min_distance: f32::MAX,
            avg_relative_velocity: 0.0,
            bond_count: 0,
            temperature_correlation: 0.0,
            phase_cooccurrence: HashMap::new(),
            samples: Vec::new(),
        }
    }
}

impl EmergentRuleEngine {
    pub fn new() -> Self {
        Self {
            discovered_rules: Vec::new(),
            element_pair_stats: HashMap::new(),
            observation_window: 100,
            min_observations: 50,
            significance_threshold: 0.7,
            tick: 0,
        }
    }

    pub fn observe(&mut self, particles: &[Particle]) {
        let n = particles.len();
        if n < 2 {
            return;
        }

        for i in 0..n {
            if !particles[i].active {
                continue;
            }
            for j in (i + 1)..n {
                if !particles[j].active {
                    continue;
                }

                let key = (
                    particles[i].element_type.min(particles[j].element_type),
                    particles[i].element_type.max(particles[j].element_type),
                );

                let stats = self.element_pair_stats.entry(key).or_default();

                let delta = particles[j].position - particles[i].position;
                let dist = delta.length();

                stats.collision_count += 1;
                stats.total_distance += dist;
                stats.min_distance = stats.min_distance.min(dist);

                let rel_vel = (particles[j].velocity - particles[i].velocity).length();
                stats.avg_relative_velocity =
                    (stats.avg_relative_velocity * (stats.collision_count - 1) as f32 + rel_vel)
                        / stats.collision_count as f32;

                stats.bond_count +=
                    (particles[i].bonds.iter().any(|b| b.target_id == particles[j].id)
                        || particles[j].bonds.iter().any(|b| b.target_id == particles[i].id))
                        as u64;

                let phase_key = (
                    particles[i].phase.min(particles[j].phase),
                    particles[i].phase.max(particles[j].phase),
                );
                *stats.phase_cooccurrence.entry(phase_key).or_insert(0) += 1;

                stats.temperature_correlation = (stats.temperature_correlation
                    * (stats.collision_count - 1) as f32
                    + particles[i].temperature * particles[j].temperature)
                    / stats.collision_count as f32;

                if stats.samples.len() < 1000 {
                    stats.samples.push(dist);
                }
            }
        }

        self.tick += 1;
    }

    pub fn discover_rules(&mut self) -> Vec<DiscoveredRule> {
        let mut new_rules = Vec::new();

        for ((ea, eb), stats) in &self.element_pair_stats {
            if stats.collision_count < self.min_observations as u64 {
                continue;
            }

            let avg_dist = stats.total_distance / stats.collision_count as f32;

            if avg_dist < 2.0 && stats.bond_count > stats.collision_count / 2 {
                let confidence = (stats.bond_count as f32 / stats.collision_count as f32).min(1.0);
                if confidence > self.significance_threshold {
                    new_rules.push(DiscoveredRule {
                        element_a: *ea,
                        element_b: *eb,
                        rule_type: EmergentRuleType::BondingPair,
                        confidence,
                        discovery_tick: self.tick,
                        parameters: vec![avg_dist, stats.avg_relative_velocity],
                    });
                }
            }

            if stats.avg_relative_velocity < 0.1 && avg_dist > 3.0 {
                let confidence = (1.0 - stats.avg_relative_velocity / 0.5).min(1.0);
                if confidence > self.significance_threshold * 0.8 {
                    new_rules.push(DiscoveredRule {
                        element_a: *ea,
                        element_b: *eb,
                        rule_type: EmergentRuleType::CollectiveMotion,
                        confidence,
                        discovery_tick: self.tick,
                        parameters: vec![stats.avg_relative_velocity],
                    });
                }
            }

            if stats.temperature_correlation > 1000.0 {
                let confidence = (stats.temperature_correlation / 5000.0).min(1.0);
                if confidence > self.significance_threshold {
                    new_rules.push(DiscoveredRule {
                        element_a: *ea,
                        element_b: *eb,
                        rule_type: EmergentRuleType::CatalyticActivity,
                        confidence,
                        discovery_tick: self.tick,
                        parameters: vec![stats.temperature_correlation],
                    });
                }
            }
        }

        for rule in &new_rules {
            if !self.discovered_rules.iter().any(|r| {
                r.element_a == rule.element_a
                    && r.element_b == rule.element_b
                    && r.rule_type == rule.rule_type
            }) {
                self.discovered_rules.push(rule.clone());
            }
        }

        new_rules
    }

    pub fn rules_to_interactions(&self) -> Vec<InteractionRule> {
        self.discovered_rules
            .iter()
            .filter_map(|rule| {
                let params = match rule.rule_type {
                    EmergentRuleType::BondingPair => {
                        let avg_dist = rule.parameters.first().copied().unwrap_or(1.0);
                        InteractionParameters {
                            epsilon: rule.confidence * 2.0,
                            sigma: avg_dist,
                            cutoff_distance: avg_dist * 3.0,
                            attraction_strength: rule.confidence * 1.5,
                            repulsion_strength: (1.0 - rule.confidence) * 5.0,
                        }
                    },
                    EmergentRuleType::RepulsionForce => InteractionParameters {
                        epsilon: 0.0,
                        sigma: 1.0,
                        cutoff_distance: 2.0,
                        attraction_strength: 0.0,
                        repulsion_strength: rule.confidence * 5.0,
                    },
                    EmergentRuleType::AttractionForce => InteractionParameters {
                        epsilon: rule.confidence * 2.0,
                        sigma: 1.5,
                        cutoff_distance: 3.0,
                        attraction_strength: rule.confidence,
                        repulsion_strength: 0.5,
                    },
                    _ => return None,
                };

                Some(InteractionRule {
                    element_a: rule.element_a,
                    element_b: rule.element_b,
                    force_type: ForceType::LennardJones,
                    parameters: params,
                })
            })
            .collect()
    }

    pub fn get_emergent_behaviors(&self) -> Vec<EmergentBehavior> {
        self.discovered_rules
            .iter()
            .map(|rule| EmergentBehavior {
                description: match rule.rule_type {
                    EmergentRuleType::BondingPair => format!(
                        "{} and {} form stable bonds",
                        format_element(rule.element_a),
                        format_element(rule.element_b)
                    ),
                    EmergentRuleType::AttractionForce => format!(
                        "{} attracts {}",
                        format_element(rule.element_a),
                        format_element(rule.element_b)
                    ),
                    EmergentRuleType::RepulsionForce => format!(
                        "{} repels {}",
                        format_element(rule.element_a),
                        format_element(rule.element_b)
                    ),
                    EmergentRuleType::CatalyticActivity => format!(
                        "{} catalyzes {} reactions",
                        format_element(rule.element_a),
                        format_element(rule.element_b)
                    ),
                    EmergentRuleType::CollectiveMotion => format!(
                        "{} and {} move collectively",
                        format_element(rule.element_a),
                        format_element(rule.element_b)
                    ),
                    EmergentRuleType::GrowthPattern => format!(
                        "{} induces growth in {}",
                        format_element(rule.element_a),
                        format_element(rule.element_b)
                    ),
                    EmergentRuleType::PhaseThreshold => format!(
                        "{} modifies {} phase threshold",
                        format_element(rule.element_a),
                        format_element(rule.element_b)
                    ),
                },
                confidence: rule.confidence,
                rule_type: rule.rule_type,
            })
            .collect()
    }

    pub fn reset_stats(&mut self) {
        self.element_pair_stats.clear();
        self.tick = 0;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergentBehavior {
    pub description: String,
    pub confidence: f32,
    pub rule_type: EmergentRuleType,
}

fn format_element(e: ElementType) -> String {
    match e {
        ElementType::Iron => "Iron",
        ElementType::Carbon => "Carbon",
        ElementType::Silicon => "Silicon",
        ElementType::Oxygen => "Oxygen",
        ElementType::Hydrogen => "Hydrogen",
        ElementType::Nitrogen => "Nitrogen",
        ElementType::Calcium => "Calcium",
        ElementType::Phosphorus => "Phosphorus",
        ElementType::Sulfur => "Sulfur",
        ElementType::Sodium => "Sodium",
        ElementType::Potassium => "Potassium",
        ElementType::Magnesium => "Magnesium",
        ElementType::Copper => "Copper",
        ElementType::Zinc => "Zinc",
        ElementType::Lead => "Lead",
        ElementType::Uranium => "Uranium",
        ElementType::Custom(id) => return format!("Element-{}", id),
    }
    .to_string()
}

impl Default for EmergentRuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particles::{BondType, ParticleBond, Phase};
    use glam::Vec3;

    #[test]
    fn test_new_defaults() {
        let engine = EmergentRuleEngine::new();
        assert!(engine.discovered_rules.is_empty());
        assert!(engine.element_pair_stats.is_empty());
        assert_eq!(engine.observation_window, 100);
        assert_eq!(engine.min_observations, 50);
        assert!((engine.significance_threshold - 0.7).abs() < 0.001);
        assert_eq!(engine.tick, 0);
    }

    #[test]
    fn test_observe_single_pair() {
        let mut engine = EmergentRuleEngine::new();
        let particles = vec![
            Particle::new(ElementType::Iron, Vec3::new(0.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Carbon, Vec3::new(1.0, 0.0, 0.0), Phase::Solid),
        ];
        engine.observe(&particles);
        assert_eq!(engine.tick, 1);
        let key = (ElementType::Iron, ElementType::Carbon);
        assert!(engine.element_pair_stats.contains_key(&key));
    }

    #[test]
    fn test_observe_multiple_ticks() {
        let mut engine = EmergentRuleEngine::new();
        let particles = vec![
            Particle::new(ElementType::Iron, Vec3::new(0.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Iron, Vec3::new(1.0, 0.0, 0.0), Phase::Solid),
        ];
        for _ in 0..10 {
            engine.observe(&particles);
        }
        assert_eq!(engine.tick, 10);
        let key = (ElementType::Iron, ElementType::Iron);
        let stats = engine.element_pair_stats.get(&key).unwrap();
        assert_eq!(stats.collision_count, 10);
    }

    #[test]
    fn test_observe_skips_inactive() {
        let mut engine = EmergentRuleEngine::new();
        let mut particles = vec![
            Particle::new(ElementType::Iron, Vec3::new(0.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Carbon, Vec3::new(1.0, 0.0, 0.0), Phase::Solid),
        ];
        particles[1].active = false;
        engine.observe(&particles);
        assert!(engine.element_pair_stats.is_empty());
    }

    #[test]
    fn test_observe_bond_count() {
        let mut engine = EmergentRuleEngine::new();
        let mut a = Particle::new(ElementType::Iron, Vec3::new(0.0, 0.0, 0.0), Phase::Solid);
        let b = Particle::new(ElementType::Iron, Vec3::new(0.5, 0.0, 0.0), Phase::Solid);
        a.bonds.push(ParticleBond {
            target_id: b.id,
            bond_type: BondType::Metallic,
            strength: 1.0,
            equilibrium_distance: 0.5,
            max_distance: 1.0,
        });
        let particles = vec![a, b];
        engine.observe(&particles);
        let key = (ElementType::Iron, ElementType::Iron);
        let stats = engine.element_pair_stats.get(&key).unwrap();
        assert_eq!(stats.bond_count, 1);
    }

    #[test]
    fn test_discover_rules_bonding_pair() {
        let mut engine = EmergentRuleEngine {
            min_observations: 0,
            significance_threshold: 0.5,
            ..EmergentRuleEngine::new()
        };
        let mut a = Particle::new(ElementType::Iron, Vec3::new(0.0, 0.0, 0.0), Phase::Solid);
        let b = Particle::new(ElementType::Iron, Vec3::new(0.5, 0.0, 0.0), Phase::Solid);
        a.bonds.push(ParticleBond {
            target_id: b.id,
            bond_type: BondType::Metallic,
            strength: 1.0,
            equilibrium_distance: 0.5,
            max_distance: 1.0,
        });
        let particles = vec![a, b];
        engine.observe(&particles);
        let new_rules = engine.discover_rules();
        assert!(!new_rules.is_empty());
        assert!(new_rules.iter().any(|r| r.rule_type == EmergentRuleType::BondingPair));
    }

    #[test]
    fn test_rules_to_interactions_bonding() {
        let mut engine = EmergentRuleEngine::new();
        engine.discovered_rules.push(DiscoveredRule {
            element_a: ElementType::Iron,
            element_b: ElementType::Carbon,
            rule_type: EmergentRuleType::BondingPair,
            confidence: 0.8,
            discovery_tick: 1,
            parameters: vec![1.5, 0.2],
        });
        let interactions = engine.rules_to_interactions();
        assert_eq!(interactions.len(), 1);
        assert_eq!(interactions[0].element_a, ElementType::Iron);
        assert_eq!(interactions[0].element_b, ElementType::Carbon);
    }

    #[test]
    fn test_rules_to_interactions_skips_unknown() {
        let mut engine = EmergentRuleEngine::new();
        engine.discovered_rules.push(DiscoveredRule {
            element_a: ElementType::Iron,
            element_b: ElementType::Carbon,
            rule_type: EmergentRuleType::CollectiveMotion,
            confidence: 0.8,
            discovery_tick: 1,
            parameters: vec![0.1],
        });
        let interactions = engine.rules_to_interactions();
        assert!(interactions.is_empty());
    }

    #[test]
    fn test_get_emergent_behaviors() {
        let mut engine = EmergentRuleEngine::new();
        engine.discovered_rules.push(DiscoveredRule {
            element_a: ElementType::Iron,
            element_b: ElementType::Oxygen,
            rule_type: EmergentRuleType::BondingPair,
            confidence: 0.9,
            discovery_tick: 1,
            parameters: vec![1.0, 0.1],
        });
        let behaviors = engine.get_emergent_behaviors();
        assert_eq!(behaviors.len(), 1);
        assert!((behaviors[0].confidence - 0.9).abs() < 0.001);
        assert!(behaviors[0].description.contains("Iron"));
        assert!(behaviors[0].description.contains("Oxygen"));
    }

    #[test]
    fn test_reset_stats() {
        let mut engine = EmergentRuleEngine::new();
        let particles = vec![
            Particle::new(ElementType::Iron, Vec3::new(0.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Carbon, Vec3::new(1.0, 0.0, 0.0), Phase::Solid),
        ];
        engine.observe(&particles);
        assert!(!engine.element_pair_stats.is_empty());
        engine.reset_stats();
        assert!(engine.element_pair_stats.is_empty());
        assert_eq!(engine.tick, 0);
    }

    #[test]
    fn test_pair_statistics_default() {
        let stats = PairStatistics::default();
        assert_eq!(stats.collision_count, 0);
        assert_eq!(stats.total_distance, 0.0);
        assert_eq!(stats.min_distance, f32::MAX);
        assert_eq!(stats.avg_relative_velocity, 0.0);
        assert_eq!(stats.bond_count, 0);
    }

    #[test]
    fn test_discover_rules_insufficient_observations() {
        let engine = EmergentRuleEngine { min_observations: 100, ..EmergentRuleEngine::new() };
        let mut e = engine;
        let new_rules = e.discover_rules();
        assert!(new_rules.is_empty());
    }
}
