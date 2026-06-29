use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThermalProperties {
    pub thermal_conductivity: f32,
    pub specific_heat: f32,
    pub density: f32,
    pub emissivity: f32,
    pub melting_point: f32,
    pub boiling_point: f32,
    pub latent_heat_fusion: f32,
    pub latent_heat_vaporization: f32,
    pub thermal_expansion_coefficient: f32,
}

impl Default for ThermalProperties {
    fn default() -> Self {
        Self {
            thermal_conductivity: 1.0,
            specific_heat: 1000.0,
            density: 1000.0,
            emissivity: 0.9,
            melting_point: f32::MAX,
            boiling_point: f32::MAX,
            latent_heat_fusion: 0.0,
            latent_heat_vaporization: 0.0,
            thermal_expansion_coefficient: 1e-5,
        }
    }
}

impl ThermalProperties {
    pub fn thermal_diffusivity(&self) -> f32 {
        if self.density * self.specific_heat > 0.0 {
            self.thermal_conductivity / (self.density * self.specific_heat)
        } else {
            0.0
        }
    }

    pub fn thermal_effusivity(&self) -> f32 {
        (self.thermal_conductivity * self.density * self.specific_heat).sqrt()
    }

    pub fn heat_capacity(&self, volume: f32) -> f32 {
        self.density * self.specific_heat * volume
    }
}

pub const THERMAL_IRON: ThermalProperties = ThermalProperties {
    thermal_conductivity: 80.0,
    specific_heat: 450.0,
    density: 7870.0,
    emissivity: 0.8,
    melting_point: 1811.0,
    boiling_point: 3134.0,
    latent_heat_fusion: 247000.0,
    latent_heat_vaporization: 6090000.0,
    thermal_expansion_coefficient: 1.2e-5,
};

pub const THERMAL_COPPER: ThermalProperties = ThermalProperties {
    thermal_conductivity: 401.0,
    specific_heat: 385.0,
    density: 8960.0,
    emissivity: 0.6,
    melting_point: 1358.0,
    boiling_point: 2835.0,
    latent_heat_fusion: 206000.0,
    latent_heat_vaporization: 4730000.0,
    thermal_expansion_coefficient: 1.7e-5,
};

pub const THERMAL_WATER: ThermalProperties = ThermalProperties {
    thermal_conductivity: 0.6,
    specific_heat: 4184.0,
    density: 1000.0,
    emissivity: 0.95,
    melting_point: 273.15,
    boiling_point: 373.15,
    latent_heat_fusion: 334000.0,
    latent_heat_vaporization: 2260000.0,
    thermal_expansion_coefficient: 2.1e-4,
};

pub const THERMAL_AIR: ThermalProperties = ThermalProperties {
    thermal_conductivity: 0.026,
    specific_heat: 1005.0,
    density: 1.2,
    emissivity: 0.02,
    melting_point: 0.0,
    boiling_point: 0.0,
    latent_heat_fusion: 0.0,
    latent_heat_vaporization: 0.0,
    thermal_expansion_coefficient: 3.4e-3,
};

pub const THERMAL_STONE: ThermalProperties = ThermalProperties {
    thermal_conductivity: 2.5,
    specific_heat: 840.0,
    density: 2600.0,
    emissivity: 0.9,
    melting_point: 1500.0,
    boiling_point: 3000.0,
    latent_heat_fusion: 500000.0,
    latent_heat_vaporization: 8000000.0,
    thermal_expansion_coefficient: 8.0e-6,
};

pub const THERMAL_WOOD: ThermalProperties = ThermalProperties {
    thermal_conductivity: 0.15,
    specific_heat: 1700.0,
    density: 700.0,
    emissivity: 0.9,
    melting_point: 500.0,
    boiling_point: 0.0,
    latent_heat_fusion: 0.0,
    latent_heat_vaporization: 0.0,
    thermal_expansion_coefficient: 5.0e-6,
};

pub const THERMAL_GLASS: ThermalProperties = ThermalProperties {
    thermal_conductivity: 1.0,
    specific_heat: 840.0,
    density: 2500.0,
    emissivity: 0.9,
    melting_point: 1700.0,
    boiling_point: 2500.0,
    latent_heat_fusion: 300000.0,
    latent_heat_vaporization: 5000000.0,
    thermal_expansion_coefficient: 9.0e-6,
};
