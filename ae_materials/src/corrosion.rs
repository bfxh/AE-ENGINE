use serde::{Deserialize, Serialize};

use crate::microstructure::Microstructure;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrosionState {
    pub uniform_depth: f32,
    pub pit_depth: f32,
    pub pit_density: f32,
    pub exposure_time: f32,
    pub scc_threshold: f32,
    pub galvanic_factor: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CorrosionEnvironment {
    Atmosphere,
    FreshWater,
    Seawater,
    Acidic(f32),
    Alkaline(f32),
    Industrial,
    Soil,
}

impl CorrosionEnvironment {
    pub fn corrosivity(&self) -> f32 {
        match self {
            CorrosionEnvironment::Atmosphere => 0.02,
            CorrosionEnvironment::FreshWater => 0.05,
            CorrosionEnvironment::Seawater => 0.13,
            CorrosionEnvironment::Acidic(ph) => 0.5 * (7.0 - ph).max(0.0) / 7.0,
            CorrosionEnvironment::Alkaline(ph) => 0.1 * (ph - 7.0).max(0.0) / 7.0,
            CorrosionEnvironment::Industrial => 0.08,
            CorrosionEnvironment::Soil => 0.03,
        }
    }
}

impl Default for CorrosionState {
    fn default() -> Self {
        Self {
            uniform_depth: 0.0,
            pit_depth: 0.0,
            pit_density: 0.0,
            exposure_time: 0.0,
            scc_threshold: 0.6,
            galvanic_factor: 1.0,
        }
    }
}

impl CorrosionState {
    pub fn apply_corrosion(
        &mut self,
        dt: f32,
        environment: &CorrosionEnvironment,
        micro: &Microstructure,
        stress_fraction: f32,
    ) {
        self.exposure_time += dt;

        let base_rate = environment.corrosivity();
        let carbon_factor = 1.0 + micro.carbon_content * 2.0;
        let grain_factor = 1.0 / (micro.grain_size * 0.01).max(0.1);
        let phase_factor = micro
            .phase_fractions
            .iter()
            .map(|(p, frac)| {
                (match p {
                    crate::phases::MaterialPhase::Ferrite => 1.0,
                    crate::phases::MaterialPhase::Austenite => 0.8,
                    crate::phases::MaterialPhase::Martensite => 1.5,
                    crate::phases::MaterialPhase::Cementite => 0.5,
                    _ => 1.0,
                }) * frac
            })
            .sum::<f32>();

        let uniform_rate =
            base_rate * carbon_factor * grain_factor * phase_factor * self.galvanic_factor;
        self.uniform_depth += uniform_rate * dt / 31536000.0;

        let pit_rate = uniform_rate * 10.0;
        self.pit_depth =
            (self.pit_depth + pit_rate * dt / 31536000.0).min(self.uniform_depth * 20.0);
        self.pit_density = (self.pit_density + base_rate * 100.0 * dt / 31536000.0).min(100.0);

        if stress_fraction > self.scc_threshold {
            let scc_rate = base_rate * 50.0 * (stress_fraction - self.scc_threshold)
                / (1.0 - self.scc_threshold);
            self.uniform_depth += scc_rate * dt / 31536000.0;
        }
    }

    pub fn galvanic_corrosion_factor(&self, potential_a: f32, potential_b: f32) -> f32 {
        let diff = (potential_a - potential_b).abs();
        1.0 + diff * 2.0
    }

    pub fn total_penetration(&self) -> f32 {
        self.uniform_depth + self.pit_depth * 0.5
    }

    pub fn cross_section_loss(&self, original_area: f32) -> f32 {
        let penetration = self.total_penetration();
        let perimeter = 2.0 * original_area.sqrt();
        (penetration * perimeter).min(original_area)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atmosphere_corrosion_slow() {
        let mut state = CorrosionState::default();
        let micro = Microstructure::default();
        state.apply_corrosion(31536000.0, &CorrosionEnvironment::Atmosphere, &micro, 0.0);
        assert!(state.uniform_depth < 0.1);
    }

    #[test]
    fn test_seawater_corrosion_faster() {
        let mut state = CorrosionState::default();
        let micro = Microstructure::default();
        state.apply_corrosion(31536000.0, &CorrosionEnvironment::Seawater, &micro, 0.0);
        assert!(state.uniform_depth > 0.01);
    }

    #[test]
    fn test_scc_with_high_stress() {
        let mut state = CorrosionState::default();
        let micro = Microstructure::default();
        let depth_before = state.uniform_depth;
        state.apply_corrosion(100000.0, &CorrosionEnvironment::Atmosphere, &micro, 0.9);
        assert!(state.uniform_depth > depth_before);
    }

    #[test]
    fn test_acidic_corrosion_increases_with_ph() {
        let mut state = CorrosionState::default();
        let micro = Microstructure::default();
        state.apply_corrosion(10000.0, &CorrosionEnvironment::Acidic(2.0), &micro, 0.0);
        assert!(state.uniform_depth > 0.0);
    }
}
