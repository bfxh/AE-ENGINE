use serde::{Deserialize, Serialize};

use crate::elements::Compound;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatterState {
    Solid,
    Liquid,
    Gas,
    Plasma,
    BoseEinsteinCondensate,
    Supercritical,
    Degenerate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Substance {
    pub compound: Compound,
    pub state: MatterState,
    pub temperature: f32,
    pub pressure: f32,
    pub mass: f32,
    pub volume: f32,
    pub concentration: f32,
    pub purity: f32,
    pub contamination: Vec<Contaminant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contaminant {
    pub compound: Compound,
    pub concentration: f32,
    pub hazard: super::reactions::HazardType,
}

impl Substance {
    pub fn new(compound: Compound, mass: f32, temperature: f32) -> Self {
        let density = compound.molar_mass() * 0.01;
        let volume = mass / density;

        Self {
            compound,
            state: MatterState::Solid,
            temperature,
            pressure: 101.325,
            mass,
            volume,
            concentration: 1.0,
            purity: 1.0,
            contamination: Vec::new(),
        }
    }

    pub fn density(&self) -> f32 {
        if self.volume < f32::EPSILON {
            return 0.0;
        }
        self.mass / self.volume
    }

    pub fn update_state(&mut self) {
        let melting = self.compound_melting_point();
        let boiling = self.compound_boiling_point();

        self.state = if self.temperature >= boiling {
            MatterState::Gas
        } else if self.temperature >= melting {
            MatterState::Liquid
        } else {
            MatterState::Solid
        };
    }

    pub fn heat(&mut self, energy: f32) {
        let specific_heat = self.compound.molar_mass() * 0.001;
        let delta_t = energy / (self.mass * specific_heat);
        self.temperature += delta_t;
        self.update_state();
    }

    pub fn mix(&mut self, other: &Substance, ratio: f32) {
        let total_mass = self.mass + other.mass * ratio;
        let weighted_temp =
            (self.temperature * self.mass + other.temperature * other.mass * ratio) / total_mass;
        self.temperature = weighted_temp;
        self.mass = total_mass;
        self.volume = self.mass / self.density();
        self.purity = (self.purity * self.mass + other.purity * other.mass * ratio) / total_mass;
        self.purity = self.purity.min(1.0);
        self.contamination.extend(other.contamination.clone());
    }

    fn compound_melting_point(&self) -> f32 {
        match self.compound {
            Compound::Water => 273.15,
            Compound::IronOxide => 1811.0,
            Compound::SodiumChloride => 1074.0,
            Compound::Glucose => 419.0,
            Compound::Ethanol => 159.0,
            Compound::AceticAcid => 290.0,
            Compound::SulfuricAcid => 283.0,
            Compound::NitricAcid => 231.0,
            Compound::LeadSulfide => 1387.0,
            Compound::UraniumOxide => 3138.0,
            Compound::SiliconDioxide => 1986.0,
            _ => 300.0,
        }
    }

    fn compound_boiling_point(&self) -> f32 {
        match self.compound {
            Compound::Water => 373.15,
            Compound::Ethanol => 351.0,
            Compound::AceticAcid => 391.0,
            Compound::SulfuricAcid => 610.0,
            Compound::NitricAcid => 356.0,
            Compound::Ammonia => 240.0,
            Compound::Methane => 112.0,
            Compound::CarbonDioxide => 195.0,
            _ => 500.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemicalEnvironment {
    pub substances: Vec<Substance>,
    pub temperature: f32,
    pub pressure: f32,
    pub humidity: f32,
    pub ph: f32,
    pub oxygen_level: f32,
    pub radiation_level: f32,
    pub volume: f32,
}

impl ChemicalEnvironment {
    pub fn new(volume: f32) -> Self {
        Self {
            substances: Vec::new(),
            temperature: 293.0,
            pressure: 101.325,
            humidity: 0.5,
            ph: 7.0,
            oxygen_level: 0.21,
            radiation_level: 0.0,
            volume,
        }
    }

    pub fn add_substance(&mut self, substance: Substance) {
        self.substances.push(substance);
    }

    pub fn update(&mut self, dt: f32) {
        let total_heat_capacity: f32 =
            self.substances.iter().map(|s| s.mass * s.compound.molar_mass() * 0.001).sum();
        let total_heat: f32 = self
            .substances
            .iter()
            .map(|s| s.temperature * s.mass * s.compound.molar_mass() * 0.001)
            .sum();

        if total_heat_capacity > 0.0 {
            let equilibrium_temp = total_heat / total_heat_capacity;
            let thermal_diffusion = 0.1 * dt;
            for substance in &mut self.substances {
                substance.temperature +=
                    (equilibrium_temp - substance.temperature) * thermal_diffusion;
                substance.update_state();
            }
        }

        self.temperature = if self.substances.is_empty() {
            293.0
        } else {
            self.substances.iter().map(|s| s.temperature).sum::<f32>()
                / self.substances.len() as f32
        };
    }

    pub fn get_ph(&self) -> f32 {
        let mut acid_concentration = 0.0;
        let mut base_concentration = 0.0;

        for substance in &self.substances {
            match substance.compound.category() {
                crate::elements::CompoundCategory::Acid => {
                    acid_concentration += substance.concentration * substance.volume / self.volume;
                },
                crate::elements::CompoundCategory::Base => {
                    base_concentration += substance.concentration * substance.volume / self.volume;
                },
                _ => {},
            }
        }

        if acid_concentration > 0.0 {
            7.0 - (acid_concentration * 10.0).min(7.0)
        } else if base_concentration > 0.0 {
            7.0 + (base_concentration * 10.0).min(7.0)
        } else {
            7.0
        }
    }
}
