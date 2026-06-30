use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::particles::{ElementType, Particle, Phase};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiologicalEmergence {
    pub organisms: Vec<EmergentOrganism>,
    pub behavior_patterns: Vec<BehaviorPattern>,
    pub environment_temperature: f32,
    pub environment_radiation: f32,
    pub nutrient_density: f32,
    pub toxin_density: f32,
    pub tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergentOrganism {
    pub id: Uuid,
    pub particle_ids: Vec<Uuid>,
    pub center: Vec3,
    pub organism_type: OrganismType,
    pub health: f32,
    pub energy: f32,
    pub age: f32,
    pub generation: u32,
    pub active_behavior: BehaviorType,
    pub chemical_signals: Vec<ChemicalSignal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrganismType {
    SingleCell,
    Multicellular,
    Colony,
    MycelialNetwork,
    SymbioticCluster,
    MineralNucleus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BehaviorType {
    Idle,
    Feeding,
    Reproduction,
    Migration,
    Defense,
    Construction,
    Communication,
    Metamorphosis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorPattern {
    pub name: String,
    pub behavior: BehaviorType,
    pub trigger_conditions: Vec<TriggerCondition>,
    pub energy_cost: f32,
    pub duration: f32,
    pub cooldown: f32,
    pub last_activated: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerCondition {
    TemperatureAbove(f32),
    TemperatureBelow(f32),
    NutrientDensityAbove(f32),
    ToxinDensityAbove(f32),
    NearbyOrganismCount(u32),
    EnergyAbove(f32),
    EnergyBelow(f32),
    RadiationAbove(f32),
    ParticleDensityAbove(f32),
    CrystalStructureDetected,
    ChemicalSignalReceived { signal_type: SignalType },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalType {
    Danger,
    Food,
    Mate,
    Aggregation,
    Dispersal,
    Morphogenesis,
    PhaseShift,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemicalSignal {
    pub signal_type: SignalType,
    pub position: Vec3,
    pub intensity: f32,
    pub decay_rate: f32,
    pub source_organism: Uuid,
}

impl BiologicalEmergence {
    pub fn new() -> Self {
        Self {
            organisms: Vec::new(),
            behavior_patterns: Vec::new(),
            environment_temperature: 293.0,
            environment_radiation: 0.0,
            nutrient_density: 0.5,
            toxin_density: 0.0,
            tick: 0,
        }
    }

    pub fn initialize_default_behaviors(&mut self) {
        self.behavior_patterns = vec![
            BehaviorPattern {
                name: "feeding".into(),
                behavior: BehaviorType::Feeding,
                trigger_conditions: vec![
                    TriggerCondition::EnergyBelow(0.3),
                    TriggerCondition::NutrientDensityAbove(0.1),
                ],
                energy_cost: 0.05,
                duration: 10.0,
                cooldown: 5.0,
                last_activated: 0.0,
            },
            BehaviorPattern {
                name: "reproduction".into(),
                behavior: BehaviorType::Reproduction,
                trigger_conditions: vec![
                    TriggerCondition::EnergyAbove(0.7),
                    TriggerCondition::NearbyOrganismCount(2),
                ],
                energy_cost: 0.4,
                duration: 20.0,
                cooldown: 30.0,
                last_activated: 0.0,
            },
            BehaviorPattern {
                name: "migration".into(),
                behavior: BehaviorType::Migration,
                trigger_conditions: vec![
                    TriggerCondition::NutrientDensityAbove(0.05),
                    TriggerCondition::ToxinDensityAbove(0.3),
                ],
                energy_cost: 0.1,
                duration: 15.0,
                cooldown: 10.0,
                last_activated: 0.0,
            },
            BehaviorPattern {
                name: "defense".into(),
                behavior: BehaviorType::Defense,
                trigger_conditions: vec![TriggerCondition::ToxinDensityAbove(0.5)],
                energy_cost: 0.15,
                duration: 5.0,
                cooldown: 3.0,
                last_activated: 0.0,
            },
            BehaviorPattern {
                name: "construction".into(),
                behavior: BehaviorType::Construction,
                trigger_conditions: vec![
                    TriggerCondition::EnergyAbove(0.5),
                    TriggerCondition::ParticleDensityAbove(0.3),
                ],
                energy_cost: 0.2,
                duration: 30.0,
                cooldown: 20.0,
                last_activated: 0.0,
            },
            BehaviorPattern {
                name: "metamorphosis".into(),
                behavior: BehaviorType::Metamorphosis,
                trigger_conditions: vec![
                    TriggerCondition::TemperatureAbove(350.0),
                    TriggerCondition::EnergyAbove(0.6),
                ],
                energy_cost: 0.5,
                duration: 50.0,
                cooldown: 100.0,
                last_activated: 0.0,
            },
            BehaviorPattern {
                name: "phase_shift_communication".into(),
                behavior: BehaviorType::Communication,
                trigger_conditions: vec![TriggerCondition::ChemicalSignalReceived {
                    signal_type: SignalType::PhaseShift,
                }],
                energy_cost: 0.02,
                duration: 2.0,
                cooldown: 1.0,
                last_activated: 0.0,
            },
        ];
    }

    pub fn detect_organisms(
        &mut self,
        particles: &[Particle],
        cluster_threshold: f32,
        min_cluster_size: usize,
    ) {
        self.organisms.clear();

        let n = particles.len();
        let mut visited = vec![false; n];

        for i in 0..n {
            if !particles[i].active || visited[i] {
                continue;
            }

            let mut cluster = Vec::new();
            let mut stack = vec![i];
            visited[i] = true;

            while let Some(idx) = stack.pop() {
                cluster.push(idx);

                for j in 0..n {
                    if !particles[j].active || visited[j] {
                        continue;
                    }
                    let dist = (particles[idx].position - particles[j].position).length();
                    if dist < cluster_threshold {
                        visited[j] = true;
                        stack.push(j);
                    }
                }
            }

            if cluster.len() >= min_cluster_size {
                let center: Vec3 = cluster.iter().map(|&idx| particles[idx].position).sum::<Vec3>()
                    / cluster.len() as f32;

                let has_carbon =
                    cluster.iter().any(|&idx| particles[idx].element_type == ElementType::Carbon);
                let has_iron =
                    cluster.iter().any(|&idx| particles[idx].element_type == ElementType::Iron);
                let has_silicon =
                    cluster.iter().any(|&idx| particles[idx].element_type == ElementType::Silicon);

                let organism_type = if has_carbon && has_iron {
                    OrganismType::MycelialNetwork
                } else if has_carbon {
                    OrganismType::Multicellular
                } else if has_silicon && has_iron {
                    OrganismType::MineralNucleus
                } else if cluster.len() > 20 {
                    OrganismType::Colony
                } else {
                    OrganismType::SingleCell
                };

                let particle_ids: Vec<Uuid> =
                    cluster.iter().map(|&idx| particles[idx].id).collect();

                self.organisms.push(EmergentOrganism {
                    id: Uuid::new_v4(),
                    particle_ids,
                    center,
                    organism_type,
                    health: 1.0,
                    energy: 0.5,
                    age: 0.0,
                    generation: 1,
                    active_behavior: BehaviorType::Idle,
                    chemical_signals: Vec::new(),
                });
            }
        }
    }

    pub fn step(&mut self, particles: &mut [Particle], dt: f32) {
        self.tick += 1;

        let mut new_organisms = Vec::new();
        let nutrient_density = self.nutrient_density;
        let tick = self.tick;
        let env_temp = self.environment_temperature;

        let organism_count = self.organisms.len();
        for oi in 0..organism_count {
            if oi >= self.organisms.len() {
                break;
            }

            self.organisms[oi].age += dt;
            self.organisms[oi].energy -= 0.001 * dt;

            if self.organisms[oi].energy <= 0.0 {
                for pid in &self.organisms[oi].particle_ids {
                    if let Some(p) = particles.iter_mut().find(|p| &p.id == pid) {
                        p.active = false;
                    }
                }
                continue;
            }

            let (best_behavior, _best_priority) = {
                let mut best = BehaviorType::Idle;
                let mut best_p = -1.0f32;
                for pattern in &mut self.behavior_patterns {
                    if pattern.last_activated + pattern.cooldown > tick as f32 {
                        continue;
                    }
                    let priority = evaluate_triggers_static(
                        &pattern.trigger_conditions,
                        &self.organisms[oi],
                        env_temp,
                        nutrient_density,
                        self.environment_radiation,
                        self.toxin_density,
                        &self.organisms,
                    );
                    if priority > best_p {
                        best_p = priority;
                        best = pattern.behavior;
                        pattern.last_activated = tick as f32;
                    }
                }
                (best, best_p)
            };

            self.organisms[oi].active_behavior = best_behavior;

            match best_behavior {
                BehaviorType::Feeding => {
                    self.organisms[oi].energy += nutrient_density * 0.1 * dt;
                    if self.organisms[oi].energy > 1.0 {
                        self.organisms[oi].energy = 1.0;
                    }
                },
                BehaviorType::Reproduction => {
                    if self.organisms[oi].energy > 0.4 && self.organisms[oi].particle_ids.len() >= 4
                    {
                        self.organisms[oi].energy -= 0.3;
                        let split_point = self.organisms[oi].particle_ids.len() / 2;
                        let new_particle_ids =
                            self.organisms[oi].particle_ids.split_off(split_point - 1);

                        let mut child = EmergentOrganism {
                            id: Uuid::new_v4(),
                            particle_ids: new_particle_ids,
                            center: self.organisms[oi].center + Vec3::new(0.5, 0.0, 0.0),
                            organism_type: self.organisms[oi].organism_type,
                            health: 0.8,
                            energy: 0.3,
                            age: 0.0,
                            generation: self.organisms[oi].generation + 1,
                            active_behavior: BehaviorType::Idle,
                            chemical_signals: Vec::new(),
                        };
                        child.chemical_signals.push(ChemicalSignal {
                            signal_type: SignalType::Morphogenesis,
                            position: child.center,
                            intensity: 0.5,
                            decay_rate: 0.01,
                            source_organism: child.id,
                        });
                        new_organisms.push(child);
                    }
                },
                BehaviorType::Migration => {
                    let direction =
                        Vec3::new((tick as f32 * 0.1).sin(), 0.0, (tick as f32 * 0.1).cos());
                    let speed = 0.5 * dt;
                    self.organisms[oi].center += direction * speed;
                    for pid in &self.organisms[oi].particle_ids {
                        if let Some(p) = particles.iter_mut().find(|p| &p.id == pid) {
                            p.position += direction * speed;
                        }
                    }
                },
                BehaviorType::Defense => {
                    self.organisms[oi].energy -= 0.02 * dt;
                    let org_center = self.organisms[oi].center;
                    let org_id = self.organisms[oi].id;
                    self.organisms[oi].chemical_signals.push(ChemicalSignal {
                        signal_type: SignalType::Danger,
                        position: org_center,
                        intensity: 0.8,
                        decay_rate: 0.05,
                        source_organism: org_id,
                    });
                },
                BehaviorType::Construction => {
                    self.organisms[oi].energy -= 0.05 * dt;
                    for pid in &self.organisms[oi].particle_ids {
                        if let Some(p) = particles.iter_mut().find(|p| &p.id == pid) {
                            if p.phase == Phase::Granular {
                                p.phase = Phase::CrystalLattice { spacing: 0.3 };
                            }
                        }
                    }
                },
                BehaviorType::Communication => {
                    let org_center = self.organisms[oi].center;
                    let org_id = self.organisms[oi].id;
                    self.organisms[oi].chemical_signals.push(ChemicalSignal {
                        signal_type: SignalType::PhaseShift,
                        position: org_center,
                        intensity: 0.6,
                        decay_rate: 0.02,
                        source_organism: org_id,
                    });
                },
                BehaviorType::Metamorphosis => {
                    trigger_phase_shift_static(&mut self.organisms[oi], particles);
                },
                BehaviorType::Idle => {},
            }

            for signal in &mut self.organisms[oi].chemical_signals {
                signal.intensity *= (1.0 - signal.decay_rate * dt).max(0.0);
            }
            self.organisms[oi].chemical_signals.retain(|s| s.intensity > 0.01);
        }

        self.organisms.extend(new_organisms);
        self.organisms.retain(|o| o.energy > 0.0 && o.health > 0.0);

        self.propagate_signals();

        if self.tick.is_multiple_of(50) {
            handle_metamorphosis_static(&mut self.organisms, particles, env_temp, nutrient_density);
        }
    }
}

fn evaluate_triggers_static(
    conditions: &[TriggerCondition],
    organism: &EmergentOrganism,
    env_temp: f32,
    nutrient_density: f32,
    radiation: f32,
    toxin_density: f32,
    all_organisms: &[EmergentOrganism],
) -> f32 {
    let mut score = 0.0f32;
    let mut matched = 0;

    for condition in conditions {
        let matches = match condition {
            TriggerCondition::TemperatureAbove(t) => env_temp > *t,
            TriggerCondition::TemperatureBelow(t) => env_temp < *t,
            TriggerCondition::NutrientDensityAbove(d) => nutrient_density > *d,
            TriggerCondition::ToxinDensityAbove(d) => toxin_density > *d,
            TriggerCondition::NearbyOrganismCount(c) => {
                all_organisms
                    .iter()
                    .filter(|o| o.id != organism.id && (o.center - organism.center).length() < 10.0)
                    .count()
                    >= *c as usize
            },
            TriggerCondition::EnergyAbove(e) => organism.energy > *e,
            TriggerCondition::EnergyBelow(e) => organism.energy < *e,
            TriggerCondition::RadiationAbove(r) => radiation > *r,
            TriggerCondition::ParticleDensityAbove(_d) => organism.particle_ids.len() > 5,
            TriggerCondition::CrystalStructureDetected => {
                organism.organism_type == OrganismType::MineralNucleus
            },
            TriggerCondition::ChemicalSignalReceived { signal_type } => {
                organism.chemical_signals.iter().any(|s| s.signal_type == *signal_type)
            },
        };

        if matches {
            matched += 1;
            score += 1.0;
        }
    }

    if matched == 0 { -1.0 } else { score / conditions.len() as f32 }
}

fn trigger_phase_shift_static(organism: &mut EmergentOrganism, particles: &mut [Particle]) {
    for pid in &organism.particle_ids {
        if let Some(p) = particles.iter_mut().find(|p| &p.id == pid) {
            match p.phase {
                Phase::Solid => p.phase = Phase::Granular,
                Phase::Granular => p.phase = Phase::Solid,
                Phase::CrystalLattice { .. } => p.phase = Phase::Amorphous,
                Phase::Amorphous => p.phase = Phase::CrystalLattice { spacing: 0.3 },
                _ => {},
            }
        }
    }

    organism.chemical_signals.push(ChemicalSignal {
        signal_type: SignalType::PhaseShift,
        position: organism.center,
        intensity: 1.0,
        decay_rate: 0.03,
        source_organism: organism.id,
    });
}

fn handle_metamorphosis_static(
    organisms: &mut [EmergentOrganism],
    particles: &mut [Particle],
    env_temp: f32,
    nutrient_density: f32,
) {
    for organism in organisms.iter_mut() {
        if organism.organism_type == OrganismType::MineralNucleus
            && organism.energy > 0.6
            && env_temp > 350.0
        {
            organism.active_behavior = BehaviorType::Metamorphosis;
            trigger_phase_shift_static(organism, particles);
        }

        if organism.organism_type == OrganismType::MycelialNetwork
            && organism.energy > 0.7
            && nutrient_density > 0.5
        {
            for pid in &organism.particle_ids {
                if let Some(p) = particles.iter_mut().find(|p| &p.id == pid) {
                    if p.element_type == ElementType::Iron {
                        p.phase = Phase::CrystalLattice { spacing: 0.25 };
                    }
                }
            }
        }
    }
}

impl BiologicalEmergence {
    fn propagate_signals(&mut self) {
        let signals: Vec<ChemicalSignal> =
            self.organisms.iter().flat_map(|o| o.chemical_signals.clone()).collect();

        for signal in &signals {
            if signal.intensity < 0.1 {
                continue;
            }

            for organism in &mut self.organisms {
                if organism.id == signal.source_organism {
                    continue;
                }

                let dist = (organism.center - signal.position).length();
                let received_intensity = signal.intensity * (-dist * 0.1).exp();

                if received_intensity > 0.05
                    && !organism.chemical_signals.iter().any(|s| {
                        s.signal_type == signal.signal_type
                            && s.source_organism == signal.source_organism
                    })
                {
                    organism.chemical_signals.push(ChemicalSignal {
                        signal_type: signal.signal_type,
                        position: signal.position,
                        intensity: received_intensity,
                        decay_rate: signal.decay_rate,
                        source_organism: signal.source_organism,
                    });
                }
            }
        }
    }

    pub fn get_organism_stats(&self) -> OrganismStats {
        let total = self.organisms.len();
        let mycelial = self
            .organisms
            .iter()
            .filter(|o| o.organism_type == OrganismType::MycelialNetwork)
            .count();
        let mineral = self
            .organisms
            .iter()
            .filter(|o| o.organism_type == OrganismType::MineralNucleus)
            .count();
        let avg_energy = if total > 0 {
            self.organisms.iter().map(|o| o.energy).sum::<f32>() / total as f32
        } else {
            0.0
        };

        OrganismStats {
            total_organisms: total,
            mycelial_networks: mycelial,
            mineral_nuclei: mineral,
            average_energy: avg_energy,
            total_signals: self.organisms.iter().map(|o| o.chemical_signals.len()).sum(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganismStats {
    pub total_organisms: usize,
    pub mycelial_networks: usize,
    pub mineral_nuclei: usize,
    pub average_energy: f32,
    pub total_signals: usize,
}

impl Default for BiologicalEmergence {
    fn default() -> Self {
        let mut be = Self::new();
        be.initialize_default_behaviors();
        be
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particles::Phase;
    use glam::Vec3;

    #[test]
    fn test_new() {
        let be = BiologicalEmergence::new();
        assert!(be.organisms.is_empty());
        assert!(be.behavior_patterns.is_empty());
        assert!((be.environment_temperature - 293.0).abs() < 0.001);
        assert_eq!(be.environment_radiation, 0.0);
        assert!((be.nutrient_density - 0.5).abs() < 0.001);
        assert_eq!(be.toxin_density, 0.0);
        assert_eq!(be.tick, 0);
    }

    #[test]
    fn test_initialize_default_behaviors() {
        let mut be = BiologicalEmergence::new();
        be.initialize_default_behaviors();
        assert_eq!(be.behavior_patterns.len(), 7);
        assert!(be.behavior_patterns.iter().any(|b| b.name == "feeding"));
        assert!(be.behavior_patterns.iter().any(|b| b.name == "reproduction"));
    }

    #[test]
    fn test_detect_organisms_single_cell() {
        let mut be = BiologicalEmergence::new();
        let particles = vec![
            Particle::new(ElementType::Carbon, Vec3::new(0.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Carbon, Vec3::new(0.5, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Carbon, Vec3::new(0.0, 0.5, 0.0), Phase::Solid),
        ];
        be.detect_organisms(&particles, 2.0, 3);
        assert_eq!(be.organisms.len(), 1);
        assert_eq!(be.organisms[0].organism_type, OrganismType::Multicellular);
    }

    #[test]
    fn test_detect_organisms_mycelial_network() {
        let mut be = BiologicalEmergence::new();
        let particles = vec![
            Particle::new(ElementType::Carbon, Vec3::new(0.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Iron, Vec3::new(0.5, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Carbon, Vec3::new(0.0, 0.5, 0.0), Phase::Solid),
        ];
        be.detect_organisms(&particles, 2.0, 3);
        assert_eq!(be.organisms.len(), 1);
        assert_eq!(be.organisms[0].organism_type, OrganismType::MycelialNetwork);
    }

    #[test]
    fn test_detect_organisms_mineral_nucleus() {
        let mut be = BiologicalEmergence::new();
        let particles = vec![
            Particle::new(ElementType::Silicon, Vec3::new(0.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Iron, Vec3::new(0.5, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Silicon, Vec3::new(0.0, 0.5, 0.0), Phase::Solid),
        ];
        be.detect_organisms(&particles, 2.0, 3);
        assert_eq!(be.organisms.len(), 1);
        assert_eq!(be.organisms[0].organism_type, OrganismType::MineralNucleus);
    }

    #[test]
    fn test_detect_organisms_below_min_size() {
        let mut be = BiologicalEmergence::new();
        let particles =
            vec![Particle::new(ElementType::Carbon, Vec3::new(0.0, 0.0, 0.0), Phase::Solid)];
        be.detect_organisms(&particles, 2.0, 3);
        assert!(be.organisms.is_empty());
    }

    #[test]
    fn test_step_no_organisms() {
        let mut be = BiologicalEmergence::new();
        be.initialize_default_behaviors();
        let mut particles: Vec<Particle> = vec![];
        be.step(&mut particles, 0.016);
        assert_eq!(be.tick, 1);
    }

    #[test]
    fn test_step_energy_depletion() {
        let mut be = BiologicalEmergence::new();
        be.initialize_default_behaviors();
        let mut particles = vec![
            Particle::new(ElementType::Carbon, Vec3::new(0.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Carbon, Vec3::new(0.1, 0.0, 0.0), Phase::Solid),
        ];
        be.organisms.push(EmergentOrganism {
            id: Uuid::new_v4(),
            particle_ids: particles.iter().map(|p| p.id).collect(),
            center: Vec3::ZERO,
            organism_type: OrganismType::SingleCell,
            health: 1.0,
            energy: 0.0,
            age: 0.0,
            generation: 1,
            active_behavior: BehaviorType::Idle,
            chemical_signals: Vec::new(),
        });
        be.step(&mut particles, 0.016);
        assert!(be.organisms.is_empty());
    }

    #[test]
    fn test_get_organism_stats_empty() {
        let be = BiologicalEmergence::new();
        let stats = be.get_organism_stats();
        assert_eq!(stats.total_organisms, 0);
        assert_eq!(stats.mycelial_networks, 0);
        assert_eq!(stats.mineral_nuclei, 0);
        assert_eq!(stats.average_energy, 0.0);
        assert_eq!(stats.total_signals, 0);
    }

    #[test]
    fn test_get_organism_stats_with_organisms() {
        let mut be = BiologicalEmergence::new();
        be.organisms.push(EmergentOrganism {
            id: Uuid::new_v4(),
            particle_ids: vec![],
            center: Vec3::ZERO,
            organism_type: OrganismType::MycelialNetwork,
            health: 1.0,
            energy: 0.8,
            age: 0.0,
            generation: 1,
            active_behavior: BehaviorType::Idle,
            chemical_signals: vec![ChemicalSignal {
                signal_type: SignalType::Danger,
                position: Vec3::ZERO,
                intensity: 0.5,
                decay_rate: 0.01,
                source_organism: Uuid::new_v4(),
            }],
        });
        be.organisms.push(EmergentOrganism {
            id: Uuid::new_v4(),
            particle_ids: vec![],
            center: Vec3::new(1.0, 0.0, 0.0),
            organism_type: OrganismType::MineralNucleus,
            health: 1.0,
            energy: 0.4,
            age: 0.0,
            generation: 1,
            active_behavior: BehaviorType::Idle,
            chemical_signals: vec![],
        });
        let stats = be.get_organism_stats();
        assert_eq!(stats.total_organisms, 2);
        assert_eq!(stats.mycelial_networks, 1);
        assert_eq!(stats.mineral_nuclei, 1);
        assert!((stats.average_energy - 0.6).abs() < 0.001);
        assert_eq!(stats.total_signals, 1);
    }

    #[test]
    fn test_step_feeding() {
        let mut be = BiologicalEmergence::new();
        be.initialize_default_behaviors();
        for pattern in &mut be.behavior_patterns {
            pattern.last_activated = -100.0;
        }
        be.nutrient_density = 0.5;
        let mut particles = vec![
            Particle::new(ElementType::Carbon, Vec3::new(0.0, 0.0, 0.0), Phase::Solid),
            Particle::new(ElementType::Carbon, Vec3::new(0.1, 0.0, 0.0), Phase::Solid),
        ];
        be.organisms.push(EmergentOrganism {
            id: Uuid::new_v4(),
            particle_ids: particles.iter().map(|p| p.id).collect(),
            center: Vec3::ZERO,
            organism_type: OrganismType::SingleCell,
            health: 1.0,
            energy: 0.2,
            age: 0.0,
            generation: 1,
            active_behavior: BehaviorType::Idle,
            chemical_signals: Vec::new(),
        });
        be.step(&mut particles, 0.016);
        assert!(be.organisms[0].energy > 0.2);
    }

    #[test]
    fn test_step_defense_emits_signal() {
        let mut be = BiologicalEmergence::new();
        be.initialize_default_behaviors();
        for pattern in &mut be.behavior_patterns {
            pattern.last_activated = -100.0;
        }
        be.toxin_density = 0.6;
        be.nutrient_density = 0.0;
        let mut particles =
            vec![Particle::new(ElementType::Carbon, Vec3::new(0.0, 0.0, 0.0), Phase::Solid)];
        be.organisms.push(EmergentOrganism {
            id: Uuid::new_v4(),
            particle_ids: particles.iter().map(|p| p.id).collect(),
            center: Vec3::ZERO,
            organism_type: OrganismType::SingleCell,
            health: 1.0,
            energy: 0.5,
            age: 0.0,
            generation: 1,
            active_behavior: BehaviorType::Idle,
            chemical_signals: Vec::new(),
        });
        be.step(&mut particles, 0.016);
        assert!(!be.organisms[0].chemical_signals.is_empty());
        assert_eq!(be.organisms[0].chemical_signals[0].signal_type, SignalType::Danger);
    }

    #[test]
    fn test_step_construction_phase_change() {
        let mut be = BiologicalEmergence::new();
        be.initialize_default_behaviors();
        let mut particles = vec![
            Particle::new(ElementType::Carbon, Vec3::new(0.0, 0.0, 0.0), Phase::Granular),
            Particle::new(ElementType::Carbon, Vec3::new(0.1, 0.0, 0.0), Phase::Granular),
            Particle::new(ElementType::Carbon, Vec3::new(0.2, 0.0, 0.0), Phase::Granular),
            Particle::new(ElementType::Carbon, Vec3::new(0.3, 0.0, 0.0), Phase::Granular),
            Particle::new(ElementType::Carbon, Vec3::new(0.4, 0.0, 0.0), Phase::Granular),
            Particle::new(ElementType::Carbon, Vec3::new(0.5, 0.0, 0.0), Phase::Granular),
        ];
        be.organisms.push(EmergentOrganism {
            id: Uuid::new_v4(),
            particle_ids: particles.iter().map(|p| p.id).collect(),
            center: Vec3::ZERO,
            organism_type: OrganismType::SingleCell,
            health: 1.0,
            energy: 0.8,
            age: 0.0,
            generation: 1,
            active_behavior: BehaviorType::Idle,
            chemical_signals: Vec::new(),
        });
        be.step(&mut particles, 0.016);
    }
}
