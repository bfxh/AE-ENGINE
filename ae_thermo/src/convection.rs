use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::properties::ThermalProperties;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvectionSolver {
    pub ambient_temperature: f32,
    pub air_density: f32,
    pub air_viscosity: f32,
    pub air_specific_heat: f32,
    pub air_thermal_conductivity: f32,
    pub gravity: f32,
    pub time_step: f32,
}

impl Default for ConvectionSolver {
    fn default() -> Self {
        Self {
            ambient_temperature: 293.15,
            air_density: 1.2,
            air_viscosity: 1.8e-5,
            air_specific_heat: 1005.0,
            air_thermal_conductivity: 0.026,
            gravity: 9.81,
            time_step: 1.0 / 60.0,
        }
    }
}

impl ConvectionSolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn grashof_number(&self, surface_temp: f32, characteristic_length: f32) -> f32 {
        let beta = 1.0 / self.ambient_temperature.max(1.0);
        let delta_t = (surface_temp - self.ambient_temperature).abs();
        let nu = self.air_viscosity / self.air_density;

        self.gravity * beta * delta_t * characteristic_length.powi(3) / (nu * nu).max(1e-12)
    }

    pub fn prandtl_number(&self) -> f32 {
        self.air_viscosity * self.air_specific_heat / self.air_thermal_conductivity.max(1e-12)
    }

    pub fn rayleigh_number(&self, surface_temp: f32, characteristic_length: f32) -> f32 {
        self.grashof_number(surface_temp, characteristic_length) * self.prandtl_number()
    }

    pub fn nusselt_number(&self, surface_temp: f32, characteristic_length: f32) -> f32 {
        let ra = self.rayleigh_number(surface_temp, characteristic_length);

        if ra < 1e4 {
            1.0
        } else if ra < 1e7 {
            0.54 * ra.powf(0.25)
        } else if ra < 1e11 {
            0.15 * ra.powf(1.0 / 3.0)
        } else {
            0.13 * ra.powf(1.0 / 3.0)
        }
    }

    pub fn natural_convection_coefficient(
        &self,
        surface_temp: f32,
        characteristic_length: f32,
    ) -> f32 {
        let nu = self.nusselt_number(surface_temp, characteristic_length);
        nu * self.air_thermal_conductivity / characteristic_length.max(1e-6)
    }

    pub fn forced_convection_coefficient(&self, velocity: Vec3, characteristic_length: f32) -> f32 {
        let speed = velocity.length().max(1e-6);
        let re = self.air_density * speed * characteristic_length / self.air_viscosity.max(1e-12);
        let pr = self.prandtl_number();

        let nu = if re < 5e5 {
            0.664 * re.sqrt() * pr.powf(1.0 / 3.0)
        } else {
            0.037 * re.powf(0.8) * pr.powf(1.0 / 3.0)
        };

        nu * self.air_thermal_conductivity / characteristic_length.max(1e-6)
    }

    pub fn solve_natural(
        &self,
        object_temp: f32,
        surface_area: f32,
        characteristic_length: f32,
        mass: f32,
        props: &ThermalProperties,
    ) -> f32 {
        let h = self.natural_convection_coefficient(object_temp, characteristic_length);
        let q = h * surface_area * (self.ambient_temperature - object_temp);
        let q_clamped = q.clamp(
            -(object_temp - self.ambient_temperature).abs()
                * props.heat_capacity(mass / props.density),
            (object_temp - self.ambient_temperature).abs()
                * props.heat_capacity(mass / props.density),
        );

        q_clamped * self.time_step / props.heat_capacity(mass / props.density)
    }

    pub fn solve_forced(
        &self,
        object_temp: f32,
        wind_velocity: Vec3,
        surface_area: f32,
        characteristic_length: f32,
        mass: f32,
        props: &ThermalProperties,
    ) -> f32 {
        let h = self.forced_convection_coefficient(wind_velocity, characteristic_length);
        let effective_temp = self.ambient_temperature;
        let q = h * surface_area * (effective_temp - object_temp);
        let q_clamped = q.clamp(
            -(object_temp - effective_temp).abs() * props.heat_capacity(mass / props.density),
            (object_temp - effective_temp).abs() * props.heat_capacity(mass / props.density),
        );

        q_clamped * self.time_step / props.heat_capacity(mass / props.density)
    }

    pub fn wind_chill_temperature(&self, air_temp_k: f32, wind_speed: f32) -> f32 {
        if wind_speed < 1.34 {
            return air_temp_k;
        }
        let air_temp_c = air_temp_k - 273.15;
        let v = wind_speed.powf(0.16);
        let twc_c = 13.12 + 0.6215 * air_temp_c - 11.37 * v + 0.3965 * air_temp_c * v;
        let twc_k = twc_c + 273.15;
        twc_k.min(air_temp_k)
    }
}

pub struct FluidConvectionCell {
    pub temperature: f32,
    pub velocity: Vec3,
    pub pressure: f32,
    pub density: f32,
}

impl FluidConvectionCell {
    pub fn new(temperature: f32, velocity: Vec3, pressure: f32, density: f32) -> Self {
        Self { temperature, velocity, pressure, density }
    }

    pub fn buoyancy_force(&self, ambient_temp: f32, gravity: f32, beta: f32) -> Vec3 {
        let delta_t = self.temperature - ambient_temp;
        let force = -self.density * beta * delta_t * gravity;
        Vec3::new(0.0, force, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grashof_number() {
        let solver = ConvectionSolver::default();
        let gr = solver.grashof_number(400.0, 0.1);
        assert!(gr > 0.0);
    }

    #[test]
    fn test_prandtl_number() {
        let solver = ConvectionSolver::default();
        let pr = solver.prandtl_number();
        assert!(pr > 0.5 && pr < 1.0);
    }

    #[test]
    fn test_nusselt_laminar() {
        let solver = ConvectionSolver::default();
        let nu = solver.nusselt_number(305.0, 0.05);
        assert!(nu >= 1.0);
    }

    #[test]
    fn test_wind_chill() {
        let solver = ConvectionSolver::default();
        let twc = solver.wind_chill_temperature(273.15, 10.0);
        assert!(twc < 273.15);
    }
}
