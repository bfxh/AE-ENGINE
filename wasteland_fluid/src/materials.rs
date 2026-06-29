use crate::properties::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundaryType {
    Solid,
    FreeSurface,
    Inflow,
    Outflow,
    Periodic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluidMaterial {
    pub name: &'static str,
    pub properties: FluidProperties,
    pub color: [f32; 3],
    pub refractive_index: f32,
}

pub const FLUID_MATERIALS: [FluidMaterial; 5] = [
    FluidMaterial {
        name: "Water",
        properties: FLUID_WATER,
        color: [0.2, 0.4, 0.8],
        refractive_index: 1.33,
    },
    FluidMaterial {
        name: "Air",
        properties: FLUID_AIR,
        color: [1.0, 1.0, 1.0],
        refractive_index: 1.0,
    },
    FluidMaterial {
        name: "Oil",
        properties: FLUID_OIL,
        color: [0.4, 0.3, 0.2],
        refractive_index: 1.47,
    },
    FluidMaterial {
        name: "Steam",
        properties: FluidProperties {
            density: 0.6,
            viscosity: 1.2e-5,
            surface_tension: 0.0,
            thermal_conductivity: 0.016,
            specific_heat: 2010.0,
            bulk_modulus: 1e5,
        },
        color: [0.9, 0.9, 0.9],
        refractive_index: 1.0,
    },
    FluidMaterial {
        name: "Lava",
        properties: FluidProperties {
            density: 2700.0,
            viscosity: 100.0,
            surface_tension: 0.5,
            thermal_conductivity: 1.0,
            specific_heat: 840.0,
            bulk_modulus: 3e10,
        },
        color: [0.9, 0.3, 0.0],
        refractive_index: 1.5,
    },
];
