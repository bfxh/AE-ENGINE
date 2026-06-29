use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FluidProperties {
    pub density: f32,
    pub viscosity: f32,
    pub surface_tension: f32,
    pub thermal_conductivity: f32,
    pub specific_heat: f32,
    pub bulk_modulus: f32,
}

impl Default for FluidProperties {
    fn default() -> Self {
        Self {
            density: 1000.0,
            viscosity: 0.001,
            surface_tension: 0.07,
            thermal_conductivity: 0.6,
            specific_heat: 4186.0,
            bulk_modulus: 2.2e9,
        }
    }
}

impl FluidProperties {
    pub fn speed_of_sound(&self) -> f32 {
        (self.bulk_modulus / self.density).sqrt()
    }

    pub fn kinematic_viscosity(&self) -> f32 {
        self.viscosity / self.density
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FluidState {
    Liquid,
    Gas,
    Supercritical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FluidContainerState {
    pub pressure: f32,
    pub temperature: f32,
    pub density: f32,
    pub velocity: Vec3,
    pub state: FluidState,
}

impl Default for FluidContainerState {
    fn default() -> Self {
        Self {
            pressure: 101325.0,
            temperature: 293.15,
            density: 1000.0,
            velocity: Vec3::ZERO,
            state: FluidState::Liquid,
        }
    }
}

pub const FLUID_WATER: FluidProperties = FluidProperties {
    density: 1000.0,
    viscosity: 0.001,
    surface_tension: 0.07,
    thermal_conductivity: 0.6,
    specific_heat: 4186.0,
    bulk_modulus: 2.2e9,
};

pub const FLUID_AIR: FluidProperties = FluidProperties {
    density: 1.225,
    viscosity: 1.8e-5,
    surface_tension: 0.0,
    thermal_conductivity: 0.024,
    specific_heat: 1005.0,
    bulk_modulus: 1.4e5,
};

pub const FLUID_OIL: FluidProperties = FluidProperties {
    density: 850.0,
    viscosity: 0.3,
    surface_tension: 0.03,
    thermal_conductivity: 0.15,
    specific_heat: 2000.0,
    bulk_modulus: 1.5e9,
};
