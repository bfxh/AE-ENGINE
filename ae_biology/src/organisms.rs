use glam::Vec3;
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::evolution::Genome;
use crate::metabolism::Metabolism;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organism {
    pub id: Uuid,
    pub species: Species,
    pub position: Vec3,
    pub velocity: Vec3,
    pub health: f32,
    pub max_health: f32,
    pub age: f32,
    pub max_age: f32,
    pub size: f32,
    pub mass: f32,
    pub genome: Genome,
    pub metabolism: Metabolism,
    pub state: OrganismState,
    pub radiation_dose: f32,
    pub mutations: Vec<Mutation>,
    pub behavior: Behavior,
    pub faction: Option<String>,
    pub inventory: Vec<OrganismItem>,
    pub sensory: SensoryInput,
    pub reproductive_cooldown: f32,
    pub offspring_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensoryInput {
    pub detection_radius: f32,
    pub nearby_entities: Vec<DetectedEntity>,
    pub nearby_food: Vec<DetectedResource>,
    pub nearby_threats: Vec<DetectedThreat>,
    pub ambient_radiation: f32,
    pub ambient_temperature: f32,
    pub last_scan_time: f64,
}

impl Default for SensoryInput {
    fn default() -> Self {
        Self {
            detection_radius: 50.0,
            nearby_entities: Vec::new(),
            nearby_food: Vec::new(),
            nearby_threats: Vec::new(),
            ambient_radiation: 0.0,
            ambient_temperature: 293.0,
            last_scan_time: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedEntity {
    pub entity_id: Uuid,
    pub species: Species,
    pub position: Vec3,
    pub distance: f32,
    pub threat_level: f32,
    pub is_friendly: bool,
    pub is_prey: bool,
    pub is_predator: bool,
    pub is_mate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedResource {
    pub position: Vec3,
    pub distance: f32,
    pub resource_type: String,
    pub amount: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedThreat {
    pub source_id: Option<Uuid>,
    pub position: Vec3,
    pub distance: f32,
    pub threat_type: ThreatType,
    pub intensity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreatType {
    Predator,
    Radiation,
    Fire,
    Explosion,
    Toxin,
    HostileFaction,
    Environmental,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Species {
    Human,
    MutantHuman,
    Ghoul,
    SuperMutant,
    Radroach,
    Molerat,
    Deathclaw,
    Brahmin,
    Bloatfly,
    Radscorpion,
    YaoGuai,
    MutantHound,
    GiantAnt,
    Cazador,
    Gecko,
    Mantis,
    Custom(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrganismState {
    Alive,
    Sleeping,
    Unconscious,
    Bleeding,
    Poisoned,
    Irradiated,
    Mutating,
    Dead,
    Undead,
    Dismembered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mutation {
    pub name: String,
    pub description: String,
    pub effects: Vec<MutationEffect>,
    pub generation: u32,
    pub stability: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MutationEffect {
    HealthModifier(f32),
    SpeedModifier(f32),
    StrengthModifier(f32),
    RadiationResistance(f32),
    NightVision,
    Regeneration(f32),
    AcidBlood(f32),
    Camouflage(f32),
    ExtraLimbs(u8),
    SizeChange(f32),
    Telepathy(f32),
    FireResistance(f32),
    ToxicImmunity,
    Photosynthesis,
    Bioluminescence,
    Carapace(f32),
    Venomous(f32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Behavior {
    pub state: BehaviorState,
    pub target: Option<Vec3>,
    pub target_entity: Option<Uuid>,
    pub aggression: f32,
    pub fear: f32,
    pub curiosity: f32,
    pub social: f32,
    pub memory: Vec<Memory>,
    pub needs: Needs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BehaviorState {
    Idle,
    Wandering,
    Fleeing,
    Hunting,
    Foraging,
    Patrolling,
    Sleeping,
    Eating,
    Drinking,
    Fighting,
    Mating,
    Building,
    Trading,
    Following,
    Investigating,
    Dying,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub position: Vec3,
    pub entity_id: Option<Uuid>,
    pub memory_type: MemoryType,
    pub importance: f32,
    pub timestamp: f64,
    pub decay_rate: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryType {
    FoodSource,
    WaterSource,
    Danger,
    Shelter,
    Ally,
    Enemy,
    Interest,
    Home,
    Trap,
    RadiationZone,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Needs {
    pub hunger: f32,
    pub thirst: f32,
    pub fatigue: f32,
    pub social_need: f32,
    pub safety_need: f32,
    pub reproduction_urge: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganismItem {
    pub item_type: OrganismItemType,
    pub quantity: u32,
    pub condition: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrganismItemType {
    Meat,
    Hide,
    Bone,
    Venom,
    Gland,
    Blood,
    Egg,
    Milk,
    Fiber,
    Chitin,
    Claw,
    Tooth,
    Organ,
    DNA,
    Feather,
    Scale,
}

impl Species {
    pub fn base_health(&self) -> f32 {
        match self {
            Self::Human => 100.0,
            Self::MutantHuman => 150.0,
            Self::Ghoul => 200.0,
            Self::SuperMutant => 500.0,
            Self::Radroach => 20.0,
            Self::Molerat => 50.0,
            Self::Deathclaw => 800.0,
            Self::Brahmin => 300.0,
            Self::Bloatfly => 15.0,
            Self::Radscorpion => 300.0,
            Self::YaoGuai => 600.0,
            Self::MutantHound => 200.0,
            Self::GiantAnt => 100.0,
            Self::Cazador => 150.0,
            Self::Gecko => 80.0,
            Self::Mantis => 120.0,
            Self::Custom(h) => *h as f32,
        }
    }

    pub fn base_speed(&self) -> f32 {
        match self {
            Self::Human => 3.0,
            Self::MutantHuman => 3.5,
            Self::Ghoul => 4.0,
            Self::SuperMutant => 2.5,
            Self::Radroach => 2.0,
            Self::Molerat => 3.0,
            Self::Deathclaw => 8.0,
            Self::Brahmin => 2.0,
            Self::Bloatfly => 1.5,
            Self::Radscorpion => 5.0,
            Self::YaoGuai => 7.0,
            Self::MutantHound => 6.0,
            Self::GiantAnt => 3.0,
            Self::Cazador => 6.0,
            Self::Gecko => 4.0,
            Self::Mantis => 4.0,
            Self::Custom(_) => 3.0,
        }
    }

    pub fn radiation_resistance(&self) -> f32 {
        match self {
            Self::Human => 0.1,
            Self::MutantHuman => 0.5,
            Self::Ghoul => 1.0,
            Self::SuperMutant => 0.8,
            Self::Radroach => 0.9,
            Self::Molerat => 0.6,
            Self::Deathclaw => 0.7,
            Self::Brahmin => 0.3,
            Self::Bloatfly => 0.8,
            Self::Radscorpion => 0.9,
            Self::YaoGuai => 0.6,
            Self::MutantHound => 0.7,
            Self::GiantAnt => 0.8,
            Self::Cazador => 0.5,
            Self::Gecko => 0.7,
            Self::Mantis => 0.6,
            Self::Custom(_) => 0.5,
        }
    }

    pub fn diet(&self) -> Diet {
        match self {
            Self::Human | Self::Ghoul | Self::SuperMutant | Self::MutantHuman => Diet::Omnivore,
            Self::Radroach | Self::Molerat | Self::Bloatfly | Self::GiantAnt | Self::Mantis => {
                Diet::Herbivore
            },
            Self::Deathclaw
            | Self::YaoGuai
            | Self::MutantHound
            | Self::Radscorpion
            | Self::Cazador => Diet::Carnivore,
            Self::Brahmin | Self::Gecko => Diet::Herbivore,
            Self::Custom(_) => Diet::Omnivore,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Diet {
    Herbivore,
    Carnivore,
    Omnivore,
    Scavenger,
    Detritivore,
    Photosynthetic,
    Chemosynthetic,
    Radiovore,
}

impl Organism {
    pub fn new(species: Species, position: Vec3) -> Self {
        let mut rng = rand::thread_rng();
        let base_health = species.base_health();
        let age = rng.gen_range(0.0..species.base_health() * 0.5);

        Self {
            id: Uuid::new_v4(),
            species,
            position,
            velocity: Vec3::ZERO,
            health: base_health,
            max_health: base_health,
            age,
            max_age: base_health * 10.0,
            size: 1.0,
            mass: 70.0,
            genome: Genome::random(&mut rng),
            metabolism: Metabolism::default(),
            state: OrganismState::Alive,
            radiation_dose: 0.0,
            mutations: Vec::new(),
            behavior: Behavior {
                state: BehaviorState::Idle,
                target: None,
                target_entity: None,
                aggression: rng.gen_range(0.0..1.0),
                fear: rng.gen_range(0.0..1.0),
                curiosity: rng.gen_range(0.0..1.0),
                social: rng.gen_range(0.0..1.0),
                memory: Vec::new(),
                needs: Needs {
                    hunger: 0.0,
                    thirst: 0.0,
                    fatigue: 0.0,
                    social_need: 0.0,
                    safety_need: 0.0,
                    reproduction_urge: 0.0,
                },
            },
            faction: None,
            inventory: Vec::new(),
            sensory: SensoryInput {
                detection_radius: 50.0,
                nearby_entities: Vec::new(),
                nearby_food: Vec::new(),
                nearby_threats: Vec::new(),
                ambient_radiation: 0.0,
                ambient_temperature: 293.0,
                last_scan_time: 0.0,
            },
            reproductive_cooldown: 0.0,
            offspring_count: 0,
        }
    }

    pub fn update(&mut self, dt: f32, time: f64) {
        if self.state == OrganismState::Dead {
            return;
        }

        self.age += dt / 3600.0;
        if self.age >= self.max_age {
            self.state = OrganismState::Dead;
            return;
        }

        self.metabolism.update(dt);
        self.update_needs(dt);
        self.update_radiation(dt);
        self.apply_mutations(dt);
        self.update_behavior(dt, time);
        self.update_position(dt);
        self.update_health(dt);
    }

    fn update_needs(&mut self, dt: f32) {
        let hours = dt / 3600.0;
        self.behavior.needs.hunger = (self.behavior.needs.hunger + hours * 0.1).min(1.0);
        self.behavior.needs.thirst = (self.behavior.needs.thirst + hours * 0.15).min(1.0);
        self.behavior.needs.fatigue = (self.behavior.needs.fatigue + hours * 0.08).min(1.0);
        self.behavior.needs.social_need = (self.behavior.needs.social_need + hours * 0.02).min(1.0);
        self.behavior.needs.safety_need = (self.behavior.needs.safety_need + hours * 0.01).min(1.0);
    }

    fn update_radiation(&mut self, dt: f32) {
        if self.radiation_dose < 0.01 {
            return;
        }
        let resistance = self.species.radiation_resistance();
        let effective_dose = self.radiation_dose * (1.0 - resistance);
        self.health -= effective_dose * 0.1 * dt;

        if self.radiation_dose > 100.0 && resistance < 0.5 {
            let mut rng = rand::thread_rng();
            if rng.gen::<f32>() < 0.01 * dt {
                self.state = OrganismState::Mutating;
                let new_mutation = Mutation::random(&mut rng, self.mutations.len() as u32);
                self.mutations.push(new_mutation);
            }
        }
        self.radiation_dose *= 1.0 - resistance * 0.001 * dt;
    }

    fn apply_mutations(&mut self, dt: f32) {
        for mutation in &self.mutations {
            for effect in &mutation.effects {
                match effect {
                    MutationEffect::HealthModifier(m) => self.max_health *= 1.0 + m * 0.01,
                    MutationEffect::Regeneration(rate) => {
                        self.health = (self.health + rate * dt).min(self.max_health);
                    },
                    MutationEffect::SizeChange(factor) => {
                        self.size *= 1.0 + factor * 0.001 * dt;
                    },
                    _ => {},
                }
            }
        }
    }

    fn update_behavior(&mut self, dt: f32, _time: f64) {
        match self.behavior.state {
            BehaviorState::Idle => {
                if self.behavior.needs.hunger > 0.7 || self.behavior.needs.thirst > 0.7 {
                    self.behavior.state = BehaviorState::Foraging;
                } else if self.behavior.needs.fatigue > 0.8 {
                    self.behavior.state = BehaviorState::Sleeping;
                } else if self.behavior.needs.safety_need > 0.6 {
                    self.behavior.state = BehaviorState::Patrolling;
                } else {
                    let mut rng = rand::thread_rng();
                    if rng.gen::<f32>() < 0.1 * dt {
                        self.behavior.state = BehaviorState::Wandering;
                        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                        let distance = rng.gen_range(5.0..50.0);
                        self.behavior.target = Some(
                            self.position
                                + Vec3::new(angle.cos() * distance, 0.0, angle.sin() * distance),
                        );
                    }
                }
            },
            BehaviorState::Wandering => {
                if let Some(target) = self.behavior.target {
                    let dir = (target - self.position).normalize();
                    self.velocity = dir * self.species.base_speed();
                    if (target - self.position).length() < 1.0 {
                        self.behavior.state = BehaviorState::Idle;
                        self.behavior.target = None;
                    }
                }
            },
            BehaviorState::Sleeping => {
                self.velocity = Vec3::ZERO;
                self.behavior.needs.fatigue =
                    (self.behavior.needs.fatigue - 0.5 * dt / 3600.0).max(0.0);
                if self.behavior.needs.fatigue < 0.1 {
                    self.behavior.state = BehaviorState::Idle;
                }
            },
            BehaviorState::Fleeing => {
                if let Some(target) = self.behavior.target {
                    let dir = (self.position - target).normalize();
                    self.velocity = dir * self.species.base_speed() * 1.5;
                }
            },
            BehaviorState::Dying => {
                self.state = OrganismState::Dead;
            },
            _ => {},
        }
    }

    fn update_position(&mut self, dt: f32) {
        self.position += self.velocity * dt;
        self.velocity *= 0.95;
    }

    fn update_health(&mut self, dt: f32) {
        if self.health <= 0.0 {
            self.state = OrganismState::Dead;
            return;
        }
        if self.behavior.needs.hunger > 0.9 {
            self.health -= 1.0 * dt;
        }
        if self.behavior.needs.thirst > 0.9 {
            self.health -= 2.0 * dt;
        }
    }

    pub fn sensory_scan(&mut self, all_organisms: &[Organism], time: f64) {
        if time - self.sensory.last_scan_time < 0.5 {
            return;
        }
        self.sensory.last_scan_time = time;
        self.sensory.nearby_entities.clear();
        self.sensory.nearby_threats.clear();

        let radius = self.sensory.detection_radius;

        for other in all_organisms {
            if other.id == self.id || other.state == OrganismState::Dead {
                continue;
            }
            let dist = (other.position - self.position).length();
            if dist > radius {
                continue;
            }

            let is_predator = other.species.diet() == Diet::Carnivore
                && (self.species.diet() == Diet::Herbivore
                    || self.species.diet() == Diet::Omnivore);
            let is_prey = self.species.diet() == Diet::Carnivore
                && (other.species.diet() == Diet::Herbivore
                    || other.species.diet() == Diet::Omnivore);
            let is_mate = other.species == self.species
                && other.state == OrganismState::Alive
                && self.reproductive_cooldown <= 0.0;

            let threat_level = if is_predator {
                1.0 - dist / radius
            } else if other.behavior.aggression > 0.7 {
                0.5
            } else {
                0.0
            };

            self.sensory.nearby_entities.push(DetectedEntity {
                entity_id: other.id,
                species: other.species,
                position: other.position,
                distance: dist,
                threat_level,
                is_friendly: other.faction == self.faction && self.faction.is_some(),
                is_prey,
                is_predator,
                is_mate,
            });

            if threat_level > 0.3 {
                self.sensory.nearby_threats.push(DetectedThreat {
                    source_id: Some(other.id),
                    position: other.position,
                    distance: dist,
                    threat_type: ThreatType::Predator,
                    intensity: threat_level,
                });
            }
        }

        if self.sensory.ambient_radiation > 50.0 {
            self.sensory.nearby_threats.push(DetectedThreat {
                source_id: None,
                position: self.position,
                distance: 0.0,
                threat_type: ThreatType::Radiation,
                intensity: (self.sensory.ambient_radiation / 100.0).min(1.0),
            });
        }
    }

    pub fn can_reproduce(&self) -> bool {
        self.state == OrganismState::Alive
            && self.age > self.max_age * 0.15
            && self.age < self.max_age * 0.8
            && self.reproductive_cooldown <= 0.0
            && self.behavior.needs.hunger < 0.5
            && self.behavior.needs.fatigue < 0.5
            && self.health > self.max_health * 0.5
    }

    pub fn create_offspring(&self, partner: &Organism) -> Organism {
        let mut rng = rand::thread_rng();
        let midpoint = (self.position + partner.position) * 0.5;
        let offset = Vec3::new(rng.gen_range(-2.0..2.0), 0.0, rng.gen_range(-2.0..2.0));

        let child_genome = self.genome.reproduce(&partner.genome, &mut rng);

        Organism {
            id: Uuid::new_v4(),
            species: self.species,
            position: midpoint + offset,
            velocity: Vec3::ZERO,
            health: self.species.base_health() * 0.5,
            max_health: self.species.base_health(),
            age: 0.0,
            max_age: self.species.base_health() * 10.0,
            size: (self.size + partner.size) * 0.25,
            mass: (self.mass + partner.mass) * 0.1,
            genome: child_genome,
            metabolism: Metabolism::default(),
            state: OrganismState::Alive,
            radiation_dose: 0.0,
            mutations: Vec::new(),
            behavior: Behavior {
                state: BehaviorState::Idle,
                target: None,
                target_entity: None,
                aggression: (self.behavior.aggression + partner.behavior.aggression) * 0.5,
                fear: (self.behavior.fear + partner.behavior.fear) * 0.5,
                curiosity: (self.behavior.curiosity + partner.behavior.curiosity) * 0.5,
                social: (self.behavior.social + partner.behavior.social) * 0.5,
                memory: Vec::new(),
                needs: Needs {
                    hunger: 0.0,
                    thirst: 0.0,
                    fatigue: 0.0,
                    social_need: 0.0,
                    safety_need: 0.0,
                    reproduction_urge: 0.0,
                },
            },
            faction: self.faction.clone(),
            inventory: Vec::new(),
            sensory: SensoryInput {
                detection_radius: self.sensory.detection_radius * 0.5,
                ..Default::default()
            },
            reproductive_cooldown: self.max_age * 0.15,
            offspring_count: 0,
        }
    }

    pub fn take_damage(&mut self, damage: f32, damage_type: &str) {
        if self.state == OrganismState::Dead {
            return;
        }
        let mut multiplier = 1.0;
        for mutation in &self.mutations {
            for effect in &mutation.effects {
                match (effect, damage_type) {
                    (MutationEffect::FireResistance(_), "fire") => multiplier *= 0.5,
                    (MutationEffect::Carapace(_), "physical") => multiplier *= 0.7,
                    (MutationEffect::ToxicImmunity, "poison") => multiplier = 0.0,
                    _ => {},
                }
            }
        }
        self.health -= damage * multiplier;
        if self.health <= 0.0 {
            self.state = OrganismState::Dead;
        }
    }

    pub fn harvestable_items(&self) -> Vec<OrganismItem> {
        let mut items = Vec::new();
        match self.species {
            Species::Radroach | Species::Mantis | Species::GiantAnt => {
                items.push(OrganismItem {
                    item_type: OrganismItemType::Meat,
                    quantity: 1,
                    condition: 0.5,
                });
                items.push(OrganismItem {
                    item_type: OrganismItemType::Chitin,
                    quantity: 2,
                    condition: 0.7,
                });
            },
            Species::Deathclaw | Species::YaoGuai => {
                items.push(OrganismItem {
                    item_type: OrganismItemType::Meat,
                    quantity: 5,
                    condition: 0.8,
                });
                items.push(OrganismItem {
                    item_type: OrganismItemType::Hide,
                    quantity: 3,
                    condition: 0.6,
                });
                items.push(OrganismItem {
                    item_type: OrganismItemType::Claw,
                    quantity: 2,
                    condition: 0.9,
                });
            },
            Species::Radscorpion | Species::Cazador => {
                items.push(OrganismItem {
                    item_type: OrganismItemType::Venom,
                    quantity: 1,
                    condition: 0.8,
                });
                items.push(OrganismItem {
                    item_type: OrganismItemType::Gland,
                    quantity: 1,
                    condition: 0.6,
                });
            },
            _ => {
                items.push(OrganismItem {
                    item_type: OrganismItemType::Meat,
                    quantity: 1,
                    condition: 0.5,
                });
            },
        }
        if !self.mutations.is_empty() {
            items.push(OrganismItem {
                item_type: OrganismItemType::DNA,
                quantity: 1,
                condition: 0.3,
            });
        }
        items
    }
}

impl Mutation {
    pub fn random(rng: &mut impl Rng, generation: u32) -> Self {
        let effects = match rng.gen_range(0..8) {
            0 => vec![MutationEffect::HealthModifier(rng.gen_range(-20.0..50.0))],
            1 => vec![MutationEffect::SpeedModifier(rng.gen_range(-10.0..30.0))],
            2 => vec![MutationEffect::RadiationResistance(rng.gen_range(0.1..0.8))],
            3 => vec![MutationEffect::Regeneration(rng.gen_range(0.1..2.0))],
            4 => vec![MutationEffect::Carapace(rng.gen_range(0.1..0.5))],
            5 => vec![MutationEffect::AcidBlood(rng.gen_range(0.1..0.5))],
            6 => vec![MutationEffect::SizeChange(rng.gen_range(-0.3..0.5))],
            7 => vec![MutationEffect::Venomous(rng.gen_range(0.1..0.5))],
            _ => vec![MutationEffect::Bioluminescence],
        };

        let names = [
            "Radioactive Adaptation",
            "Forced Evolution",
            "Cellular Regeneration",
            "Chitinous Growth",
            "Metabolic Overdrive",
            "Toxic Secretion",
            "Photosynthetic Skin",
            "Neural Enhancement",
        ];

        Self {
            name: names[rng.gen_range(0..names.len())].to_string(),
            description: String::new(),
            effects,
            generation,
            stability: rng.gen_range(0.3..1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_species_base_health_human() {
        assert_eq!(Species::Human.base_health(), 100.0);
    }

    #[test]
    fn test_species_base_health_deathclaw_strongest() {
        let deathclaw = Species::Deathclaw.base_health();
        for s in [Species::Human, Species::SuperMutant, Species::YaoGuai, Species::Brahmin] {
            assert!(deathclaw > s.base_health());
        }
        assert_eq!(deathclaw, 800.0);
    }

    #[test]
    fn test_species_base_health_bloatfly_weakest() {
        let bloatfly = Species::Bloatfly.base_health();
        assert_eq!(bloatfly, 15.0);
        for s in [Species::Human, Species::Radroach, Species::Molerat] {
            assert!(bloatfly < s.base_health());
        }
    }

    #[test]
    fn test_species_base_health_all_values() {
        assert_eq!(Species::MutantHuman.base_health(), 150.0);
        assert_eq!(Species::Ghoul.base_health(), 200.0);
        assert_eq!(Species::SuperMutant.base_health(), 500.0);
        assert_eq!(Species::Radroach.base_health(), 20.0);
        assert_eq!(Species::Molerat.base_health(), 50.0);
        assert_eq!(Species::Brahmin.base_health(), 300.0);
        assert_eq!(Species::Radscorpion.base_health(), 300.0);
        assert_eq!(Species::YaoGuai.base_health(), 600.0);
        assert_eq!(Species::MutantHound.base_health(), 200.0);
        assert_eq!(Species::GiantAnt.base_health(), 100.0);
        assert_eq!(Species::Cazador.base_health(), 150.0);
        assert_eq!(Species::Gecko.base_health(), 80.0);
        assert_eq!(Species::Mantis.base_health(), 120.0);
    }

    #[test]
    fn test_species_base_speed_deathclaw_fastest() {
        assert_eq!(Species::Deathclaw.base_speed(), 8.0);
        assert!(Species::Deathclaw.base_speed() > Species::YaoGuai.base_speed());
    }

    #[test]
    fn test_species_base_speed_bloatfly_slowest() {
        assert_eq!(Species::Bloatfly.base_speed(), 1.5);
        for s in [Species::Human, Species::Radroach, Species::Gecko] {
            assert!(Species::Bloatfly.base_speed() < s.base_speed());
        }
    }

    #[test]
    fn test_species_radiation_resistance_ghoul_max() {
        assert_eq!(Species::Ghoul.radiation_resistance(), 1.0);
        for s in [Species::Human, Species::MutantHuman, Species::SuperMutant] {
            assert!(Species::Ghoul.radiation_resistance() >= s.radiation_resistance());
        }
    }

    #[test]
    fn test_species_radiation_resistance_human_min() {
        assert_eq!(Species::Human.radiation_resistance(), 0.1);
        for s in [Species::Ghoul, Species::Radroach, Species::SuperMutant] {
            assert!(Species::Human.radiation_resistance() < s.radiation_resistance());
        }
    }

    #[test]
    fn test_species_diet_human_omnivore() {
        assert_eq!(Species::Human.diet(), Diet::Omnivore);
        assert_eq!(Species::Ghoul.diet(), Diet::Omnivore);
        assert_eq!(Species::SuperMutant.diet(), Diet::Omnivore);
        assert_eq!(Species::MutantHuman.diet(), Diet::Omnivore);
    }

    #[test]
    fn test_species_diet_deathclaw_carnivore() {
        assert_eq!(Species::Deathclaw.diet(), Diet::Carnivore);
        assert_eq!(Species::YaoGuai.diet(), Diet::Carnivore);
        assert_eq!(Species::Radscorpion.diet(), Diet::Carnivore);
        assert_eq!(Species::MutantHound.diet(), Diet::Carnivore);
        assert_eq!(Species::Cazador.diet(), Diet::Carnivore);
    }

    #[test]
    fn test_species_diet_brahmin_herbivore() {
        assert_eq!(Species::Brahmin.diet(), Diet::Herbivore);
        assert_eq!(Species::Gecko.diet(), Diet::Herbivore);
        assert_eq!(Species::Radroach.diet(), Diet::Herbivore);
        assert_eq!(Species::Molerat.diet(), Diet::Herbivore);
    }

    #[test]
    fn test_organism_new_health_full() {
        let org = Organism::new(Species::Human, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(org.health, org.max_health);
        assert_eq!(org.health, 100.0);
    }

    #[test]
    fn test_organism_new_state_alive() {
        let org = Organism::new(Species::Radroach, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(org.state, OrganismState::Alive);
    }

    #[test]
    fn test_organism_new_max_age_is_base_health_x10() {
        let org = Organism::new(Species::Human, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(org.max_age, 100.0 * 10.0);
    }

    #[test]
    fn test_organism_new_velocity_zero() {
        let org = Organism::new(Species::Human, Vec3::new(5.0, 0.0, 5.0));
        assert_eq!(org.velocity, Vec3::ZERO);
    }

    #[test]
    fn test_organism_new_position_preserved() {
        let pos = Vec3::new(10.0, 0.0, 20.0);
        let org = Organism::new(Species::Human, pos);
        assert_eq!(org.position, pos);
    }

    #[test]
    fn test_organism_new_no_radiation() {
        let org = Organism::new(Species::Human, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(org.radiation_dose, 0.0);
    }

    #[test]
    fn test_organism_new_no_mutations() {
        let org = Organism::new(Species::Human, Vec3::new(0.0, 0.0, 0.0));
        assert!(org.mutations.is_empty());
    }

    #[test]
    fn test_organism_new_no_offspring() {
        let org = Organism::new(Species::Human, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(org.offspring_count, 0);
    }

    #[test]
    fn test_organism_new_default_size_and_mass() {
        let org = Organism::new(Species::Human, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(org.size, 1.0);
        assert_eq!(org.mass, 70.0);
    }

    #[test]
    fn test_organism_take_damage_reduces_health() {
        let mut org = Organism::new(Species::Human, Vec3::new(0.0, 0.0, 0.0));
        let before = org.health;
        org.take_damage(20.0, "physical");
        assert_eq!(org.health, before - 20.0);
    }

    #[test]
    fn test_organism_take_damage_lethal_marks_dead() {
        let mut org = Organism::new(Species::Bloatfly, Vec3::new(0.0, 0.0, 0.0));
        // Bloatfly 15 HP
        org.take_damage(100.0, "physical");
        assert!(org.health <= 0.0);
        assert_eq!(org.state, OrganismState::Dead);
    }

    #[test]
    fn test_organism_take_damage_dead_no_effect() {
        let mut org = Organism::new(Species::Human, Vec3::new(0.0, 0.0, 0.0));
        org.state = OrganismState::Dead;
        let before = org.health;
        org.take_damage(50.0, "physical");
        assert_eq!(org.health, before);
    }

    #[test]
    fn test_organism_harvestable_items_radroach() {
        let org = Organism::new(Species::Radroach, Vec3::new(0.0, 0.0, 0.0));
        let items = org.harvestable_items();
        assert!(items.iter().any(|i| i.item_type == OrganismItemType::Meat));
        assert!(items.iter().any(|i| i.item_type == OrganismItemType::Chitin));
    }

    #[test]
    fn test_organism_harvestable_items_deathclaw() {
        let org = Organism::new(Species::Deathclaw, Vec3::new(0.0, 0.0, 0.0));
        let items = org.harvestable_items();
        assert!(items.iter().any(|i| i.item_type == OrganismItemType::Meat));
        assert!(items.iter().any(|i| i.item_type == OrganismItemType::Hide));
        assert!(items.iter().any(|i| i.item_type == OrganismItemType::Claw));
    }

    #[test]
    fn test_organism_harvestable_items_radscorpion_venom() {
        let org = Organism::new(Species::Radscorpion, Vec3::new(0.0, 0.0, 0.0));
        let items = org.harvestable_items();
        assert!(items.iter().any(|i| i.item_type == OrganismItemType::Venom));
        assert!(items.iter().any(|i| i.item_type == OrganismItemType::Gland));
    }

    #[test]
    fn test_organism_harvestable_items_human_default_meat() {
        let org = Organism::new(Species::Human, Vec3::new(0.0, 0.0, 0.0));
        let items = org.harvestable_items();
        assert!(items.iter().any(|i| i.item_type == OrganismItemType::Meat));
    }

    #[test]
    fn test_organism_harvestable_items_with_mutation_adds_dna() {
        let mut org = Organism::new(Species::Human, Vec3::new(0.0, 0.0, 0.0));
        org.mutations.push(Mutation {
            name: "test".to_string(),
            description: String::new(),
            effects: vec![],
            generation: 1,
            stability: 1.0,
        });
        let items = org.harvestable_items();
        assert!(items.iter().any(|i| i.item_type == OrganismItemType::DNA));
    }

    #[test]
    fn test_organism_can_reproduce_false_for_dead() {
        let mut org = Organism::new(Species::Human, Vec3::new(0.0, 0.0, 0.0));
        org.state = OrganismState::Dead;
        assert!(!org.can_reproduce());
    }

    #[test]
    fn test_organism_can_reproduce_false_when_hungry() {
        let mut org = Organism::new(Species::Human, Vec3::new(0.0, 0.0, 0.0));
        org.behavior.needs.hunger = 0.8;
        assert!(!org.can_reproduce());
    }

    #[test]
    fn test_organism_can_reproduce_false_when_young() {
        let mut org = Organism::new(Species::Human, Vec3::new(0.0, 0.0, 0.0));
        // 年龄过小（< max_age * 0.15 = 150）
        org.age = 10.0;
        assert!(!org.can_reproduce());
    }

    #[test]
    fn test_organism_can_reproduce_false_when_low_health() {
        let mut org = Organism::new(Species::Human, Vec3::new(0.0, 0.0, 0.0));
        org.age = 200.0; // 成年
        org.health = 10.0; // 低血量（< max_health * 0.5 = 50）
        assert!(!org.can_reproduce());
    }

    #[test]
    fn test_organism_create_offspring_inherits_species() {
        let parent1 = Organism::new(Species::Human, Vec3::new(0.0, 0.0, 0.0));
        let parent2 = Organism::new(Species::Human, Vec3::new(1.0, 0.0, 0.0));
        let child = parent1.create_offspring(&parent2);
        assert_eq!(child.species, Species::Human);
        assert_eq!(child.state, OrganismState::Alive);
    }

    #[test]
    fn test_organism_create_offspring_age_zero() {
        let parent1 = Organism::new(Species::Human, Vec3::new(0.0, 0.0, 0.0));
        let parent2 = Organism::new(Species::Human, Vec3::new(1.0, 0.0, 0.0));
        let child = parent1.create_offspring(&parent2);
        assert_eq!(child.age, 0.0);
    }

    #[test]
    fn test_organism_create_offspring_health_half_base() {
        let parent1 = Organism::new(Species::Human, Vec3::new(0.0, 0.0, 0.0));
        let parent2 = Organism::new(Species::Human, Vec3::new(1.0, 0.0, 0.0));
        let child = parent1.create_offspring(&parent2);
        // 子代初始 health = species.base_health() * 0.5
        assert_eq!(child.health, 50.0);
    }

    #[test]
    fn test_sensory_input_default() {
        let s = SensoryInput::default();
        assert_eq!(s.detection_radius, 50.0);
        assert_eq!(s.ambient_radiation, 0.0);
        assert_eq!(s.ambient_temperature, 293.0);
        assert!(s.nearby_entities.is_empty());
        assert!(s.nearby_food.is_empty());
        assert!(s.nearby_threats.is_empty());
    }

    #[test]
    fn test_mutation_random_generates_effects() {
        let mut rng = rand::thread_rng();
        let m = Mutation::random(&mut rng, 0);
        assert!(!m.effects.is_empty());
        assert_eq!(m.generation, 0);
        assert!(!m.name.is_empty());
    }

    #[test]
    fn test_mutation_random_stability_range() {
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let m = Mutation::random(&mut rng, 0);
            assert!(m.stability >= 0.3 && m.stability <= 1.0);
        }
    }

    #[test]
    fn test_mutation_random_generation_preserved() {
        let mut rng = rand::thread_rng();
        let m = Mutation::random(&mut rng, 42);
        assert_eq!(m.generation, 42);
    }

    #[test]
    fn test_diet_equality() {
        assert_eq!(Diet::Herbivore, Diet::Herbivore);
        assert_ne!(Diet::Carnivore, Diet::Herbivore);
        assert_ne!(Diet::Omnivore, Diet::Scavenger);
    }

    #[test]
    fn test_organism_state_variants() {
        assert_ne!(OrganismState::Alive, OrganismState::Dead);
        assert_ne!(OrganismState::Sleeping, OrganismState::Unconscious);
    }

    #[test]
    fn test_species_equality() {
        assert_eq!(Species::Human, Species::Human);
        assert_ne!(Species::Human, Species::Ghoul);
        assert_eq!(Species::Custom(42), Species::Custom(42));
        assert_ne!(Species::Custom(42), Species::Custom(43));
    }
}
