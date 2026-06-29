use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ElectromagneticProperties {
    pub electrical_conductivity: f32,
    pub dielectric_constant: f32,
    pub magnetic_permeability: f32,
    pub dielectric_strength: f32,
    pub curie_temperature: f32,
}

impl Default for ElectromagneticProperties {
    fn default() -> Self {
        Self {
            electrical_conductivity: 0.0,
            dielectric_constant: 1.0,
            magnetic_permeability: 1.0,
            dielectric_strength: f32::MAX,
            curie_temperature: f32::MAX,
        }
    }
}

impl ElectromagneticProperties {
    pub fn resistivity(&self) -> f32 {
        if self.electrical_conductivity > 0.0 {
            1.0 / self.electrical_conductivity
        } else {
            f32::MAX
        }
    }

    pub fn is_conductor(&self) -> bool {
        self.electrical_conductivity > 1e3
    }

    pub fn is_insulator(&self) -> bool {
        self.electrical_conductivity < 1e-6
    }

    pub fn is_ferromagnetic(&self) -> bool {
        self.magnetic_permeability > 10.0
    }
}

pub const VACUUM_PERMITTIVITY: f32 = 8.854_188e-12;
pub const VACUUM_PERMEABILITY: f32 = 1.256_637e-6;
pub const SPEED_OF_LIGHT: f32 = 299792458.0;
