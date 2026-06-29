use rand::Rng;
use serde::{Deserialize, Serialize};

// 20层基因: 敏捷5层 + 力量5层 + 智力5层 + 体质5层
const AGILITY_OFFSET: usize = 0;
const STRENGTH_OFFSET: usize = 5;
const INTELLIGENCE_OFFSET: usize = 10;
const CONSTITUTION_OFFSET: usize = 15;
const TOTAL_LAYERS: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genome {
    pub layers: [u8; TOTAL_LAYERS],
    pub traits: Traits,
    pub mutations: Vec<MutationRecord>,
    pub origin: GenomeOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Traits {
    pub radiation_resistant: bool,
    pub night_vision: bool,
    pub regeneration: bool,
    pub toxin_resistant: bool,
    pub heat_resistant: bool,
    pub cold_resistant: bool,
    pub enhanced_senses: bool,
    pub quick_learner: bool,
    pub natural_armor: bool,
    pub photosynthetic: bool,
    pub carnivorous: bool,
    pub herbivorous: bool,
    pub aquatic: bool,
    pub aerial: bool,
    pub burrowing: bool,
    pub social: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationRecord {
    pub layer: usize,
    pub previous_value: u8,
    pub new_value: u8,
    pub mutation_type: MutationType,
    pub frame: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationType {
    Random,
    Radiation,
    Chemical,
    Inherited,
    Edited,
    Stress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenomeOrigin {
    Preset(u64),
    Inherited(u64, u64),
    Edited(u64),
    Mutated(u64),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Attributes {
    pub speed: f32,
    pub crit_chance: f32,
    pub accuracy: f32,
    pub attack_speed: f32,
    pub dodge_chance: f32,
    pub base_damage: f32,
    pub attack_power: f32,
    pub crit_multiplier: f32,
    pub armor_penetration: f32,
    pub carry_weight: f32,
    pub perception_range: f32,
    pub learning_rate: f32,
    pub hack_success_rate: f32,
    pub research_speed: f32,
    pub mental_resistance: f32,
    pub max_health: f32,
    pub natural_recovery: f32,
    pub radiation_resistance: f32,
    pub toxin_resistance: f32,
    pub aging_rate: f32,
}

impl Genome {
    pub fn new(origin: GenomeOrigin) -> Self {
        let mut rng = rand::thread_rng();
        let mut layers = [50u8; TOTAL_LAYERS];

        match origin {
            GenomeOrigin::Preset(id) => {
                Self::load_preset(id, &mut layers);
            },
            GenomeOrigin::Inherited(_, _) => {
                for layer in layers.iter_mut() {
                    *layer = rng.gen_range(30..=70);
                }
            },
            GenomeOrigin::Edited(_) => {
                for layer in layers.iter_mut() {
                    *layer = rng.gen_range(40..=80);
                }
            },
            GenomeOrigin::Mutated(_) => {
                for layer in layers.iter_mut() {
                    *layer = rng.gen_range(20..=90);
                }
            },
        }

        Self { layers, traits: Traits::default(), mutations: Vec::new(), origin }
    }

    fn load_preset(id: u64, layers: &mut [u8; TOTAL_LAYERS]) {
        match id {
            0 => {
                // 废土客: 高体质, 中等力量
                for i in 0..5 {
                    layers[AGILITY_OFFSET + i] = 45;
                    layers[STRENGTH_OFFSET + i] = 55;
                    layers[INTELLIGENCE_OFFSET + i] = 40;
                    layers[CONSTITUTION_OFFSET + i] = 65;
                }
            },
            1 => {
                // 虫族: 高敏捷, 高智力
                for i in 0..5 {
                    layers[AGILITY_OFFSET + i] = 70;
                    layers[STRENGTH_OFFSET + i] = 40;
                    layers[INTELLIGENCE_OFFSET + i] = 65;
                    layers[CONSTITUTION_OFFSET + i] = 45;
                }
            },
            2 => {
                // 克隆人: 均衡, 认知滤网加成
                for i in 0..5 {
                    layers[AGILITY_OFFSET + i] = 55;
                    layers[STRENGTH_OFFSET + i] = 55;
                    layers[INTELLIGENCE_OFFSET + i] = 60;
                    layers[CONSTITUTION_OFFSET + i] = 55;
                }
            },
            _ => {
                for layer in layers.iter_mut() {
                    *layer = 50;
                }
            },
        }
    }

    pub fn agility(&self, layer: usize) -> u8 {
        self.layers[AGILITY_OFFSET + layer.min(4)]
    }

    pub fn strength(&self, layer: usize) -> u8 {
        self.layers[STRENGTH_OFFSET + layer.min(4)]
    }

    pub fn intelligence(&self, layer: usize) -> u8 {
        self.layers[INTELLIGENCE_OFFSET + layer.min(4)]
    }

    pub fn constitution(&self, layer: usize) -> u8 {
        self.layers[CONSTITUTION_OFFSET + layer.min(4)]
    }

    pub fn apply_mutation(
        &mut self,
        layer: usize,
        delta: i8,
        mutation_type: MutationType,
        frame: u64,
    ) {
        let old = self.layers[layer];
        let new = (old as i16 + delta as i16).clamp(0, 100) as u8;
        self.layers[layer] = new;

        self.mutations.push(MutationRecord {
            layer,
            previous_value: old,
            new_value: new,
            mutation_type,
            frame,
        });
    }

    pub fn combine(parent_a: &Genome, parent_b: &Genome, _frame: u64) -> Self {
        let mut rng = rand::thread_rng();
        let mut layers = [0u8; TOTAL_LAYERS];

        for (i, layer) in layers.iter_mut().enumerate() {
            let (base, _variant) = if rng.gen_bool(0.5) {
                (parent_a.layers[i], parent_b.layers[i])
            } else {
                (parent_b.layers[i], parent_a.layers[i])
            };

            let mut value = base;
            if rng.gen_bool(0.05) {
                let delta: i8 = rng.gen_range(-10..=10);
                value = (value as i16 + delta as i16).clamp(0, 100) as u8;
            }

            *layer = value;
        }

        let mut traits = Traits::default();

        traits.radiation_resistant =
            parent_a.traits.radiation_resistant || parent_b.traits.radiation_resistant;
        traits.night_vision = parent_a.traits.night_vision || parent_b.traits.night_vision;
        traits.regeneration = parent_a.traits.regeneration && parent_b.traits.regeneration;
        traits.toxin_resistant = parent_a.traits.toxin_resistant || parent_b.traits.toxin_resistant;

        if rng.gen_bool(0.05) {
            let random_trait = rng.gen_range(0..16);
            match random_trait {
                0 => traits.radiation_resistant = true,
                1 => traits.night_vision = true,
                2 => traits.regeneration = true,
                3 => traits.toxin_resistant = true,
                _ => {},
            }
        }

        Self {
            layers,
            traits,
            mutations: Vec::new(),
            origin: GenomeOrigin::Inherited(
                match parent_a.origin {
                    GenomeOrigin::Preset(id) => id,
                    _ => 0,
                },
                match parent_b.origin {
                    GenomeOrigin::Preset(id) => id,
                    _ => 0,
                },
            ),
        }
    }

    pub fn calculate_attributes(&self) -> Attributes {
        let a1 = self.agility(0) as f32;
        let a2 = self.agility(1) as f32;
        let a3 = self.agility(2) as f32;
        let a4 = self.agility(3) as f32;
        let a5 = self.agility(4) as f32;

        let s1 = self.strength(0) as f32;
        let s2 = self.strength(1) as f32;
        let s3 = self.strength(2) as f32;
        let s4 = self.strength(3) as f32;
        let s5 = self.strength(4) as f32;

        let i1 = self.intelligence(0) as f32;
        let i2 = self.intelligence(1) as f32;
        let i3 = self.intelligence(2) as f32;
        let i4 = self.intelligence(3) as f32;
        let i5 = self.intelligence(4) as f32;

        let c1 = self.constitution(0) as f32;
        let c2 = self.constitution(1) as f32;
        let c3 = self.constitution(2) as f32;
        let c4 = self.constitution(3) as f32;
        let c5 = self.constitution(4) as f32;

        Attributes {
            speed: 5.0 * (1.0 + a1 * 0.02),
            crit_chance: (0.05 + a2 * 0.01).min(0.95),
            accuracy: (0.8 + a3 * 0.005).min(1.0),
            attack_speed: 1.0 * (1.0 + a4 * 0.015),
            dodge_chance: (0.05 + a5 * 0.01).min(0.75),
            base_damage: 10.0 * (1.0 + s1 * 0.03),
            attack_power: s2 * 2.0,
            crit_multiplier: 1.5 + s3 * 0.02,
            armor_penetration: s4 * 0.5,
            carry_weight: 50.0 + s5 * 5.0,
            perception_range: 20.0 + i1 * 1.0,
            learning_rate: 1.0 + i2 * 0.05,
            hack_success_rate: (0.3 + i3 * 0.01).min(1.0),
            research_speed: 1.0 + i4 * 0.08,
            mental_resistance: i5 * 0.02,
            max_health: 100.0 + c1 * 20.0,
            natural_recovery: 1.0 + c2 * 0.5,
            radiation_resistance: c3 * 0.02,
            toxin_resistance: c4 * 0.02,
            aging_rate: (1.0 - c5 * 0.008).max(0.2),
        }
    }

    pub fn has_trait(&self, trait_name: &str) -> bool {
        match trait_name {
            "radiation_resistant" => self.traits.radiation_resistant,
            "night_vision" => self.traits.night_vision,
            "regeneration" => self.traits.regeneration,
            "toxin_resistant" => self.traits.toxin_resistant,
            "heat_resistant" => self.traits.heat_resistant,
            "cold_resistant" => self.traits.cold_resistant,
            "enhanced_senses" => self.traits.enhanced_senses,
            "quick_learner" => self.traits.quick_learner,
            "natural_armor" => self.traits.natural_armor,
            "photosynthetic" => self.traits.photosynthetic,
            "carnivorous" => self.traits.carnivorous,
            "herbivorous" => self.traits.herbivorous,
            "aquatic" => self.traits.aquatic,
            "aerial" => self.traits.aerial,
            "burrowing" => self.traits.burrowing,
            "social" => self.traits.social,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_genomes() {
        let wastelander = Genome::new(GenomeOrigin::Preset(0));
        let attr = wastelander.calculate_attributes();
        assert!(attr.max_health > 100.0);
        assert!(attr.radiation_resistance > 0.0);

        let insectoid = Genome::new(GenomeOrigin::Preset(1));
        let attr2 = insectoid.calculate_attributes();
        assert!(attr2.speed > 5.0);
        assert!(attr2.learning_rate > 1.0);

        let clone = Genome::new(GenomeOrigin::Preset(2));
        let attr3 = clone.calculate_attributes();
        assert!((attr3.max_health - 100.0).abs() > 0.0);
    }

    #[test]
    fn test_combine_genomes() {
        let parent_a = Genome::new(GenomeOrigin::Preset(0));
        let parent_b = Genome::new(GenomeOrigin::Preset(2));
        let child = Genome::combine(&parent_a, &parent_b, 0);

        for i in 0..TOTAL_LAYERS {
            assert!(child.layers[i] <= 100);
        }
    }

    #[test]
    fn test_mutation() {
        let mut genome = Genome::new(GenomeOrigin::Preset(0));
        let old = genome.layers[0];
        genome.apply_mutation(0, 10, MutationType::Radiation, 100);
        assert!(genome.layers[0] >= old);
        assert_eq!(genome.mutations.len(), 1);
    }
}
