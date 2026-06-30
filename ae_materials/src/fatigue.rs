use serde::{Deserialize, Serialize};

use crate::properties::DerivedProperties;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FatigueState {
    pub accumulated_damage: f32,
    pub cycle_count: u64,
    pub max_stress_history: f32,
    pub mean_stress: f32,
}

impl Default for FatigueState {
    fn default() -> Self {
        Self { accumulated_damage: 0.0, cycle_count: 0, max_stress_history: 0.0, mean_stress: 0.0 }
    }
}

impl FatigueState {
    pub fn apply_cycle(
        &mut self,
        stress_amplitude: f32,
        mean_stress: f32,
        props: &DerivedProperties,
    ) {
        self.cycle_count += 1;
        self.max_stress_history = self.max_stress_history.max(stress_amplitude);
        self.mean_stress = mean_stress;

        if stress_amplitude < props.fatigue_limit {
            return;
        }

        let cycles_to_failure = self.sn_curve(stress_amplitude, props);
        if cycles_to_failure > 0.0 {
            let damage_increment = 1.0 / cycles_to_failure;
            self.accumulated_damage +=
                self.goodman_correction(stress_amplitude, mean_stress, props) * damage_increment;
        }

        self.accumulated_damage = self.accumulated_damage.min(1.0);
    }

    fn sn_curve(&self, stress_amplitude: f32, props: &DerivedProperties) -> f32 {
        let s = stress_amplitude / props.yield_strength;
        if s < 0.3 {
            return f32::MAX;
        }
        let n = 10.0f32.powf((1.0 - s) / 0.1);
        n.max(1.0)
    }

    fn goodman_correction(
        &self,
        _stress_amplitude: f32,
        mean_stress: f32,
        props: &DerivedProperties,
    ) -> f32 {
        if props.yield_strength < 1e-6 {
            return 1.0;
        }
        let correction = 1.0 / (1.0 - mean_stress.abs() / props.yield_strength).max(0.01);
        correction.min(5.0)
    }

    pub fn is_failed(&self) -> bool {
        self.accumulated_damage >= 1.0
    }

    pub fn damage_level(&self) -> f32 {
        self.accumulated_damage
    }

    pub fn remaining_life_fraction(&self) -> f32 {
        (1.0 - self.accumulated_damage).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::microstructure::Microstructure;

    #[test]
    fn test_below_fatigue_limit() {
        let micro = Microstructure::default();
        let props = DerivedProperties::from_microstructure(&micro);
        let mut fatigue = FatigueState::default();
        fatigue.apply_cycle(1.0, 0.0, &props);
        assert_eq!(fatigue.accumulated_damage, 0.0);
    }

    #[test]
    fn test_above_fatigue_limit() {
        let micro = Microstructure::default();
        let props = DerivedProperties::from_microstructure(&micro);
        let mut fatigue = FatigueState::default();
        let high_stress = props.yield_strength * 0.6;
        fatigue.apply_cycle(high_stress, 0.0, &props);
        assert!(fatigue.accumulated_damage > 0.0);
    }

    #[test]
    fn test_accumulates_to_failure() {
        let micro = Microstructure::default();
        let props = DerivedProperties::from_microstructure(&micro);
        let mut fatigue = FatigueState::default();
        let high_stress = props.yield_strength * 0.8;
        for _ in 0..10000 {
            fatigue.apply_cycle(high_stress, 0.0, &props);
            if fatigue.is_failed() {
                break;
            }
        }
        assert!(fatigue.is_failed());
    }
}
