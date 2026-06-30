use crate::properties::ElectromagneticProperties;

pub const EM_COPPER: ElectromagneticProperties = ElectromagneticProperties {
    electrical_conductivity: 5.96e7,
    dielectric_constant: 1.0,
    magnetic_permeability: 0.999994,
    dielectric_strength: 3.0e6,
    curie_temperature: f32::MAX,
};

pub const EM_IRON: ElectromagneticProperties = ElectromagneticProperties {
    electrical_conductivity: 1.0e7,
    dielectric_constant: 1.0,
    magnetic_permeability: 5000.0,
    dielectric_strength: 1.0e6,
    curie_temperature: 1043.0,
};

pub const EM_ALUMINUM: ElectromagneticProperties = ElectromagneticProperties {
    electrical_conductivity: 3.77e7,
    dielectric_constant: 1.0,
    magnetic_permeability: 1.000022,
    dielectric_strength: 1.5e6,
    curie_temperature: f32::MAX,
};

pub const EM_GOLD: ElectromagneticProperties = ElectromagneticProperties {
    electrical_conductivity: 4.1e7,
    dielectric_constant: 1.0,
    magnetic_permeability: 0.99996,
    dielectric_strength: 1.0e7,
    curie_temperature: f32::MAX,
};

pub const EM_SILVER: ElectromagneticProperties = ElectromagneticProperties {
    electrical_conductivity: 6.3e7,
    dielectric_constant: 1.0,
    magnetic_permeability: 0.99998,
    dielectric_strength: 1.0e7,
    curie_temperature: f32::MAX,
};

pub const EM_WATER: ElectromagneticProperties = ElectromagneticProperties {
    electrical_conductivity: 5.0e-6,
    dielectric_constant: 80.0,
    magnetic_permeability: 0.999992,
    dielectric_strength: 1.5e7,
    curie_temperature: f32::MAX,
};

pub const EM_GLASS: ElectromagneticProperties = ElectromagneticProperties {
    electrical_conductivity: 1.0e-14,
    dielectric_constant: 5.0,
    magnetic_permeability: 1.0,
    dielectric_strength: 1.0e7,
    curie_temperature: f32::MAX,
};

pub const EM_AIR: ElectromagneticProperties = ElectromagneticProperties {
    electrical_conductivity: 3.0e-15,
    dielectric_constant: 1.0006,
    magnetic_permeability: 1.000_000_4,
    dielectric_strength: 3.0e6,
    curie_temperature: f32::MAX,
};

pub const EM_SILICON: ElectromagneticProperties = ElectromagneticProperties {
    electrical_conductivity: 1.0e-3,
    dielectric_constant: 11.7,
    magnetic_permeability: 1.0,
    dielectric_strength: 3.0e7,
    curie_temperature: f32::MAX,
};
