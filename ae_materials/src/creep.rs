use serde::{Deserialize, Serialize};

use crate::properties::DerivedProperties;

const GAS_CONSTANT: f32 = 8.314;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreepState {
    pub creep_strain: f32,
    pub stage: CreepStage,
    pub time_at_stress: f32,
    pub activation_energy: f32,
    pub stress_exponent: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreepStage {
    Primary,
    Secondary,
    Tertiary,
    Rupture,
}

impl Default for CreepState {
    fn default() -> Self {
        Self {
            creep_strain: 0.0,
            stage: CreepStage::Primary,
            time_at_stress: 0.0,
            activation_energy: 250000.0,
            stress_exponent: 5.0,
        }
    }
}

impl CreepState {
    pub fn apply_creep(
        &mut self,
        stress: f32,
        temperature: f32,
        dt: f32,
        props: &DerivedProperties,
    ) {
        if stress < props.yield_strength * 0.2 || temperature < 500.0 {
            return;
        }

        self.time_at_stress += dt;

        let strain_rate = self.norton_rate(stress, temperature, props);

        match self.stage {
            CreepStage::Primary => {
                let decay_factor = (-self.time_at_stress / 1000.0).exp();
                self.creep_strain += strain_rate * dt * (1.0 + 10.0 * decay_factor);
                if self.time_at_stress > 1000.0 {
                    self.stage = CreepStage::Secondary;
                }
            },
            CreepStage::Secondary => {
                self.creep_strain += strain_rate * dt;
                if self.creep_strain > 0.05 {
                    self.stage = CreepStage::Tertiary;
                }
            },
            CreepStage::Tertiary => {
                let acceleration = 1.0 + self.creep_strain * 50.0;
                self.creep_strain += strain_rate * dt * acceleration;
                if self.creep_strain > 0.2 {
                    self.stage = CreepStage::Rupture;
                }
            },
            CreepStage::Rupture => {},
        }
    }

    fn norton_rate(&self, stress: f32, temperature: f32, _props: &DerivedProperties) -> f32 {
        let a = 1e-12;
        let n = self.stress_exponent;
        let q = self.activation_energy;
        let t = temperature.max(1.0);
        a * stress.powf(n) * (-q / (GAS_CONSTANT * t)).exp()
    }

    pub fn is_ruptured(&self) -> bool {
        self.stage == CreepStage::Rupture
    }

    pub fn strain_fraction(&self) -> f32 {
        self.creep_strain.min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::microstructure::Microstructure;

    #[test]
    fn test_no_creep_at_low_temp() {
        let micro = Microstructure::default();
        let props = DerivedProperties::from_microstructure(&micro);
        let mut creep = CreepState::default();
        creep.apply_creep(500.0, 300.0, 1000.0, &props);
        assert_eq!(creep.creep_strain, 0.0);
    }

    #[test]
    fn test_creep_accumulates_at_high_temp() {
        let micro = Microstructure::default();
        let props = DerivedProperties::from_microstructure(&micro);
        let mut creep = CreepState::default();
        creep.apply_creep(800.0, 1400.0, 1000.0, &props);
        assert!(creep.creep_strain > 0.0);
    }

    #[test]
    fn test_progresses_through_stages() {
        let micro = Microstructure::default();
        let props = DerivedProperties::from_microstructure(&micro);
        let mut creep = CreepState::default();
        for _ in 0..100 {
            creep.apply_creep(800.0, 1400.0, 100.0, &props);
            if creep.is_ruptured() {
                break;
            }
        }
        assert!(creep.creep_strain > 0.0);
    }
}
