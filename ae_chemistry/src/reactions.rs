use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::elements::Compound;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemicalReaction {
    pub reactants: Vec<(Compound, f32)>,
    pub products: Vec<(Compound, f32)>,
    pub activation_energy: f32,
    pub reaction_enthalpy: f32,
    pub reaction_rate: f32,
    pub catalyst: Option<Compound>,
    pub reaction_type: ReactionType,
    pub conditions: ReactionConditions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReactionType {
    Synthesis,
    Decomposition,
    Combustion,
    Oxidation,
    Reduction,
    AcidBase,
    Precipitation,
    RadioactiveDecay,
    Polymerization,
    Catalysis,
    Explosion,
    Corrosion,
    Biological,
    Photochemical,
    Electrochemical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ReactionConditions {
    pub min_temperature: f32,
    pub max_temperature: f32,
    pub min_pressure: f32,
    pub requires_oxygen: bool,
    pub requires_water: bool,
    pub requires_light: bool,
    pub requires_catalyst: bool,
    pub ph_min: f32,
    pub ph_max: f32,
}

impl Default for ReactionConditions {
    fn default() -> Self {
        Self {
            min_temperature: 0.0,
            max_temperature: f32::MAX,
            min_pressure: 0.0,
            requires_oxygen: false,
            requires_water: false,
            requires_light: false,
            requires_catalyst: false,
            ph_min: 0.0,
            ph_max: 14.0,
        }
    }
}

impl ChemicalReaction {
    pub fn combustion_organic() -> Self {
        Self {
            reactants: vec![(Compound::Methane, 1.0), (Compound::Water, 2.0)],
            products: vec![(Compound::CarbonDioxide, 1.0)],
            activation_energy: 50.0,
            reaction_enthalpy: -890.0,
            reaction_rate: 0.8,
            catalyst: None,
            reaction_type: ReactionType::Combustion,
            conditions: ReactionConditions {
                min_temperature: 500.0,
                requires_oxygen: true,
                ..Default::default()
            },
        }
    }

    pub fn rust_formation() -> Self {
        Self {
            reactants: vec![(Compound::IronOxide, 0.0)],
            products: vec![(Compound::IronOxide, 2.0)],
            activation_energy: 10.0,
            reaction_enthalpy: -100.0,
            reaction_rate: 0.01,
            catalyst: Some(Compound::SodiumChloride),
            reaction_type: ReactionType::Oxidation,
            conditions: ReactionConditions {
                requires_oxygen: true,
                requires_water: true,
                ..Default::default()
            },
        }
    }

    pub fn acid_corrosion() -> Self {
        Self {
            reactants: vec![(Compound::SulfuricAcid, 1.0), (Compound::IronOxide, 1.0)],
            products: vec![(Compound::IronOxide, 1.0), (Compound::Water, 1.0)],
            activation_energy: 5.0,
            reaction_enthalpy: -50.0,
            reaction_rate: 0.1,
            catalyst: None,
            reaction_type: ReactionType::Corrosion,
            conditions: ReactionConditions { ph_max: 4.0, ..Default::default() },
        }
    }

    pub fn explosion_tnt() -> Self {
        Self {
            reactants: vec![(Compound::TNT, 1.0)],
            products: vec![
                (Compound::CarbonDioxide, 7.0),
                (Compound::Water, 2.5),
                (Compound::Ammonia, 1.5),
            ],
            activation_energy: 100.0,
            reaction_enthalpy: -3400.0,
            reaction_rate: 100.0,
            catalyst: None,
            reaction_type: ReactionType::Explosion,
            conditions: ReactionConditions { min_temperature: 300.0, ..Default::default() },
        }
    }

    pub fn radioactive_decay() -> Self {
        Self {
            reactants: vec![(Compound::RadioactiveIsotope, 1.0)],
            products: vec![(Compound::LeadSulfide, 1.0)],
            activation_energy: 0.0,
            reaction_enthalpy: 1000.0,
            reaction_rate: 0.0000001,
            catalyst: None,
            reaction_type: ReactionType::RadioactiveDecay,
            conditions: ReactionConditions::default(),
        }
    }

    pub fn photosynthesis() -> Self {
        Self {
            reactants: vec![(Compound::CarbonDioxide, 6.0), (Compound::Water, 6.0)],
            products: vec![(Compound::Glucose, 1.0)],
            activation_energy: 30.0,
            reaction_enthalpy: 2800.0,
            reaction_rate: 0.001,
            catalyst: None,
            reaction_type: ReactionType::Photochemical,
            conditions: ReactionConditions {
                requires_light: true,
                requires_water: true,
                ..Default::default()
            },
        }
    }

    pub fn fermentation() -> Self {
        Self {
            reactants: vec![(Compound::Glucose, 1.0)],
            products: vec![(Compound::Ethanol, 2.0), (Compound::CarbonDioxide, 2.0)],
            activation_energy: 20.0,
            reaction_enthalpy: -100.0,
            reaction_rate: 0.01,
            catalyst: None,
            reaction_type: ReactionType::Biological,
            conditions: ReactionConditions { ..Default::default() },
        }
    }

    pub fn thermite_reaction() -> Self {
        Self {
            reactants: vec![(Compound::Thermite, 1.0)],
            products: vec![(Compound::IronOxide, 2.0)],
            activation_energy: 1200.0,
            reaction_enthalpy: -850.0,
            reaction_rate: 50.0,
            catalyst: None,
            reaction_type: ReactionType::Combustion,
            conditions: ReactionConditions { min_temperature: 1200.0, ..Default::default() },
        }
    }

    pub fn mutagen_synthesis() -> Self {
        Self {
            reactants: vec![(Compound::BiologicalToxin, 1.0), (Compound::RadioactiveIsotope, 0.1)],
            products: vec![(Compound::MutagenCompound, 0.5)],
            activation_energy: 200.0,
            reaction_enthalpy: 500.0,
            reaction_rate: 0.001,
            catalyst: None,
            reaction_type: ReactionType::Biological,
            conditions: ReactionConditions { min_temperature: 300.0, ..Default::default() },
        }
    }

    pub fn acid_rain_formation() -> Self {
        Self {
            reactants: vec![(Compound::Water, 1.0), (Compound::SulfuricAcid, 0.1)],
            products: vec![(Compound::AcidRain, 1.0)],
            activation_energy: 5.0,
            reaction_enthalpy: -20.0,
            reaction_rate: 0.5,
            catalyst: None,
            reaction_type: ReactionType::Synthesis,
            conditions: ReactionConditions {
                ph_max: 5.0,
                requires_water: true,
                ..Default::default()
            },
        }
    }

    pub fn biofuel_combustion() -> Self {
        Self {
            reactants: vec![(Compound::Biofuel, 1.0)],
            products: vec![(Compound::CarbonDioxide, 2.0), (Compound::Water, 3.0)],
            activation_energy: 30.0,
            reaction_enthalpy: -1200.0,
            reaction_rate: 0.6,
            catalyst: None,
            reaction_type: ReactionType::Combustion,
            conditions: ReactionConditions {
                min_temperature: 250.0,
                requires_oxygen: true,
                ..Default::default()
            },
        }
    }

    pub fn cryo_agent_expansion() -> Self {
        Self {
            reactants: vec![(Compound::CryoAgent, 1.0)],
            products: vec![(Compound::Ammonia, 2.0), (Compound::CarbonDioxide, 2.0)],
            activation_energy: 20.0,
            reaction_enthalpy: 200.0,
            reaction_rate: 20.0,
            catalyst: None,
            reaction_type: ReactionType::Decomposition,
            conditions: ReactionConditions { max_temperature: 250.0, ..Default::default() },
        }
    }

    pub fn rust_inhibition() -> Self {
        Self {
            reactants: vec![(Compound::RustInhibitor, 1.0), (Compound::IronOxide, 1.0)],
            products: vec![(Compound::SiliconDioxide, 0.5), (Compound::Water, 0.5)],
            activation_energy: 15.0,
            reaction_enthalpy: -30.0,
            reaction_rate: 0.05,
            catalyst: None,
            reaction_type: ReactionType::Reduction,
            conditions: ReactionConditions { requires_water: true, ..Default::default() },
        }
    }

    pub fn neurotoxin_dispersion() -> Self {
        Self {
            reactants: vec![(Compound::Neurotoxin, 1.0)],
            products: vec![(Compound::BiologicalToxin, 0.3)],
            activation_energy: 10.0,
            reaction_enthalpy: -50.0,
            reaction_rate: 0.8,
            catalyst: None,
            reaction_type: ReactionType::Decomposition,
            conditions: ReactionConditions { min_temperature: 280.0, ..Default::default() },
        }
    }

    pub fn can_proceed(
        &self,
        temperature: f32,
        pressure: f32,
        oxygen_present: bool,
        water_present: bool,
        ph: f32,
    ) -> bool {
        if temperature < self.conditions.min_temperature
            || temperature > self.conditions.max_temperature
        {
            return false;
        }
        if pressure < self.conditions.min_pressure {
            return false;
        }
        if self.conditions.requires_oxygen && !oxygen_present {
            return false;
        }
        if self.conditions.requires_water && !water_present {
            return false;
        }
        if ph < self.conditions.ph_min || ph > self.conditions.ph_max {
            return false;
        }
        true
    }

    pub fn progress(&self, temperature: f32, dt: f32) -> f32 {
        const R: f32 = 8.314;
        let temp_k = temperature.max(1.0);
        let ea_joules = self.activation_energy * 1000.0;
        let arrhenius = (-ea_joules / (R * temp_k)).exp();
        let reference = (-50000.0 / (R * 298.0)).exp();
        let normalized = (arrhenius / reference).min(100.0);
        self.reaction_rate * normalized * dt
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionSystem {
    pub active_reactions: Vec<ActiveReaction>,
    pub completed_reactions: Vec<ReactionResult>,
    pub temperature: f32,
    pub pressure: f32,
    pub oxygen_level: f32,
    pub humidity: f32,
    pub ph: f32,
    pub catalyst_concentration: f32,
    pub radiation_level: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveReaction {
    pub reaction: ChemicalReaction,
    pub progress: f32,
    pub position: Vec3,
    pub intensity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionResult {
    pub reaction: ChemicalReaction,
    pub position: Vec3,
    pub energy_released: f32,
    pub products_generated: Vec<(Compound, f32)>,
    pub byproducts: Vec<ReactionByproduct>,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionByproduct {
    pub compound: Compound,
    pub amount: f32,
    pub spread_radius: f32,
    pub duration: f32,
    pub hazard: HazardType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HazardType {
    ToxicFumes,
    Radiation,
    Fire,
    AcidSpray,
    ExplosiveResidue,
    CorrosivePuddle,
    BiologicalContamination,
    OxygenDepletion,
}

impl Default for ReactionSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl ReactionSystem {
    pub fn new() -> Self {
        Self {
            active_reactions: Vec::new(),
            completed_reactions: Vec::new(),
            temperature: 293.0,
            pressure: 101.325,
            oxygen_level: 0.21,
            humidity: 0.5,
            ph: 7.0,
            catalyst_concentration: 0.0,
            radiation_level: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32, time: f64) {
        let mut completed = Vec::new();
        let mut new_reactions = Vec::new();

        for (i, active) in self.active_reactions.iter_mut().enumerate() {
            let reaction_progress = active.reaction.progress(self.temperature, dt);
            active.progress += reaction_progress * active.intensity;

            if active.progress >= 1.0 {
                completed.push(i);
            }
        }

        for i in completed.iter().rev() {
            let active = self.active_reactions.remove(*i);
            let energy = active.reaction.reaction_enthalpy.abs() * active.intensity;

            let result = ReactionResult {
                reaction: active.reaction.clone(),
                position: active.position,
                energy_released: energy,
                products_generated: active.reaction.products.clone(),
                byproducts: self.generate_byproducts(&active.reaction),
                timestamp: time,
            };

            self.temperature += energy * 0.001;
            self.completed_reactions.push(result);
        }

        self.active_reactions.append(&mut new_reactions);

        if self.completed_reactions.len() > 1000 {
            self.completed_reactions.drain(0..500);
        }
    }

    fn generate_byproducts(&self, reaction: &ChemicalReaction) -> Vec<ReactionByproduct> {
        let mut byproducts = Vec::new();
        match reaction.reaction_type {
            ReactionType::Combustion => {
                byproducts.push(ReactionByproduct {
                    compound: Compound::CarbonDioxide,
                    amount: 1.0,
                    spread_radius: 5.0,
                    duration: 10.0,
                    hazard: HazardType::OxygenDepletion,
                });
            },
            ReactionType::Explosion => {
                byproducts.push(ReactionByproduct {
                    compound: Compound::CarbonDioxide,
                    amount: 2.0,
                    spread_radius: 20.0,
                    duration: 1.0,
                    hazard: HazardType::ExplosiveResidue,
                });
            },
            ReactionType::RadioactiveDecay => {
                byproducts.push(ReactionByproduct {
                    compound: Compound::RadioactiveIsotope,
                    amount: 0.5,
                    spread_radius: 50.0,
                    duration: 3.15e7,
                    hazard: HazardType::Radiation,
                });
            },
            ReactionType::Corrosion => {
                byproducts.push(ReactionByproduct {
                    compound: Compound::CorrosiveAgent,
                    amount: 0.3,
                    spread_radius: 2.0,
                    duration: 60.0,
                    hazard: HazardType::CorrosivePuddle,
                });
            },
            _ => {},
        }
        byproducts
    }

    pub fn trigger_reaction(&mut self, reaction: ChemicalReaction, position: Vec3, intensity: f32) {
        let oxygen_present = self.oxygen_level > 0.01;
        let water_present = self.humidity > 0.01;

        if reaction.can_proceed(
            self.temperature,
            self.pressure,
            oxygen_present,
            water_present,
            self.ph,
        ) {
            self.active_reactions.push(ActiveReaction {
                reaction,
                progress: 0.0,
                position,
                intensity,
            });
        }
    }
}
