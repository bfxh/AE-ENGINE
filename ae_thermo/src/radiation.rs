use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::properties::ThermalProperties;

const STEFAN_BOLTZMANN: f32 = 5.670367e-8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadiationSolver {
    pub ambient_temperature: f32,
    pub time_step: f32,
}

impl Default for RadiationSolver {
    fn default() -> Self {
        Self { ambient_temperature: 293.15, time_step: 1.0 / 60.0 }
    }
}

impl RadiationSolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn emissive_power(&self, temperature: f32, emissivity: f32) -> f32 {
        emissivity * STEFAN_BOLTZMANN * temperature.powi(4)
    }

    pub fn net_radiation(
        &self,
        temp_a: f32,
        temp_b: f32,
        emissivity_a: f32,
        emissivity_b: f32,
        view_factor: f32,
        surface_area: f32,
    ) -> f32 {
        let effective_emissivity = 1.0 / (1.0 / emissivity_a + 1.0 / emissivity_b - 1.0).max(0.01);
        effective_emissivity
            * STEFAN_BOLTZMANN
            * (temp_a.powi(4) - temp_b.powi(4))
            * view_factor
            * surface_area
    }

    pub fn solve_surface_to_ambient(
        &self,
        object_temp: f32,
        surface_area: f32,
        mass: f32,
        props: &ThermalProperties,
    ) -> f32 {
        let q = self.net_radiation(
            object_temp,
            self.ambient_temperature,
            props.emissivity,
            1.0,
            1.0,
            surface_area,
        );

        let delta_t = (object_temp - self.ambient_temperature).abs();
        let max_energy = delta_t * props.heat_capacity(mass / props.density);
        let q_clamped = q.clamp(-max_energy, max_energy);

        -q_clamped * self.time_step / props.heat_capacity(mass / props.density)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn solve_surface_to_surface(
        &self,
        temp_a: f32,
        temp_b: f32,
        mass_a: f32,
        mass_b: f32,
        surface_area: f32,
        view_factor: f32,
        props_a: &ThermalProperties,
        props_b: &ThermalProperties,
    ) -> (f32, f32) {
        let q = self.net_radiation(
            temp_a,
            temp_b,
            props_a.emissivity,
            props_b.emissivity,
            view_factor,
            surface_area,
        );

        let delta_t = (temp_a - temp_b).abs();
        let max_energy_a = delta_t * props_a.heat_capacity(mass_a / props_a.density);
        let max_energy_b = delta_t * props_b.heat_capacity(mass_b / props_b.density);
        let q_clamped = q.clamp(-max_energy_a.min(max_energy_b), max_energy_a.min(max_energy_b));

        let delta_a = -q_clamped * self.time_step / props_a.heat_capacity(mass_a / props_a.density);
        let delta_b = q_clamped * self.time_step / props_b.heat_capacity(mass_b / props_b.density);

        (delta_a, delta_b)
    }

    pub fn view_factor_parallel_plates(&self, distance: f32, area: f32) -> f32 {
        if distance < 1e-6 {
            return 1.0;
        }
        let aspect = area.sqrt() / distance;
        let x = (1.0 + aspect.powi(2)).sqrt();
        let f = 2.0 / (std::f32::consts::PI * aspect.powi(2))
            * (aspect * x.atan() - (1.0 / x).atan() + aspect / (2.0 * x));
        f.clamp(0.0, 1.0)
    }

    pub fn view_factor_perpendicular_plates(&self, h: f32, w: f32) -> f32 {
        let a = h / w.max(1e-6);
        let b = w / w.max(1e-6);
        let term1 = (1.0 + a.powi(2)).sqrt() * (1.0 + b.powi(2)).sqrt();
        let term2 = a * (b * (1.0 + a.powi(2) + b.powi(2)).sqrt()).atan();
        let _term3 = a * b.atan() + b * a.atan();

        let f = (1.0 / (std::f32::consts::PI * a))
            * (a * b.atan() + b * a.atan() + (term1.ln() - term2) / 2.0);
        f.clamp(0.0, 1.0)
    }

    pub fn solar_heating(
        &self,
        _surface_temp: f32,
        surface_area: f32,
        solar_irradiance: f32,
        absorptivity: f32,
        mass: f32,
        props: &ThermalProperties,
    ) -> f32 {
        let q = absorptivity * solar_irradiance * surface_area;
        q * self.time_step / props.heat_capacity(mass / props.density)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SolarConfig {
    pub irradiance: f32,
    pub direction: Vec3,
    pub cloud_cover: f32,
}

impl Default for SolarConfig {
    fn default() -> Self {
        Self { irradiance: 1000.0, direction: Vec3::new(0.0, -1.0, 0.0), cloud_cover: 0.0 }
    }
}

impl SolarConfig {
    pub fn effective_irradiance(&self) -> f32 {
        let cloud_factor = 1.0 - 0.75 * self.cloud_cover.clamp(0.0, 1.0);
        self.irradiance * cloud_factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stefan_boltzmann_hotter_radiates_more() {
        let solver = RadiationSolver::default();
        let p_cold = solver.emissive_power(300.0, 0.9);
        let p_hot = solver.emissive_power(500.0, 0.9);
        assert!(p_hot > p_cold);
    }

    #[test]
    fn test_net_radiation_zero_equal_temp() {
        let solver = RadiationSolver::default();
        let q = solver.net_radiation(300.0, 300.0, 0.9, 0.9, 1.0, 1.0);
        assert!((q).abs() < 1e-6);
    }

    #[test]
    fn test_net_radiation_flow_hot_to_cold() {
        let solver = RadiationSolver::default();
        let q = solver.net_radiation(500.0, 300.0, 0.9, 0.9, 1.0, 1.0);
        assert!(q > 0.0);
    }

    #[test]
    fn test_view_factor_parallel() {
        let solver = RadiationSolver::default();
        let f = solver.view_factor_parallel_plates(0.1, 1.0);
        assert!(f > 0.0 && f <= 1.0);
    }
}
