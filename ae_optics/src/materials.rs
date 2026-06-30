use crate::spectrum::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpticalMaterial {
    pub name: &'static str,
    pub refractive_index: f32,
    pub absorption: Spectrum,
    pub scattering: f32,
    pub roughness: f32,
    pub metallic: f32,
    pub emissive: Spectrum,
}

impl OpticalMaterial {
    pub fn air() -> Self {
        Self {
            name: "Air",
            refractive_index: 1.0,
            absorption: Spectrum::new_constant(0.0),
            scattering: 0.0,
            roughness: 0.0,
            metallic: 0.0,
            emissive: Spectrum::new_constant(0.0),
        }
    }

    pub fn water() -> Self {
        Self {
            name: "Water",
            refractive_index: 1.33,
            absorption: Spectrum::new_constant(0.001),
            scattering: 0.01,
            roughness: 0.0,
            metallic: 0.0,
            emissive: Spectrum::new_constant(0.0),
        }
    }

    pub fn glass() -> Self {
        Self {
            name: "Glass",
            refractive_index: 1.52,
            absorption: Spectrum::new_constant(0.0001),
            scattering: 0.001,
            roughness: 0.02,
            metallic: 0.0,
            emissive: Spectrum::new_constant(0.0),
        }
    }

    pub fn metal() -> Self {
        Self {
            name: "Metal",
            refractive_index: 2.0,
            absorption: Spectrum::new_constant(1.0),
            scattering: 0.0,
            roughness: 0.3,
            metallic: 1.0,
            emissive: Spectrum::new_constant(0.0),
        }
    }

    pub fn plastic() -> Self {
        Self {
            name: "Plastic",
            refractive_index: 1.5,
            absorption: Spectrum::new_constant(0.01),
            scattering: 0.05,
            roughness: 0.4,
            metallic: 0.0,
            emissive: Spectrum::new_constant(0.0),
        }
    }

    pub fn rubber() -> Self {
        Self {
            name: "Rubber",
            refractive_index: 1.5,
            absorption: Spectrum::new_constant(0.8),
            scattering: 0.02,
            roughness: 0.8,
            metallic: 0.0,
            emissive: Spectrum::new_constant(0.0),
        }
    }
}

pub fn fresnel(cos_theta: f32, n1: f32, n2: f32) -> f32 {
    let n_ratio = n1 / n2;
    let sin_theta2_sq = n_ratio * n_ratio * (1.0 - cos_theta * cos_theta);
    if sin_theta2_sq > 1.0 {
        return 1.0;
    }
    let cos_theta2 = (1.0 - sin_theta2_sq).sqrt();
    let rs = (n1 * cos_theta - n2 * cos_theta2) / (n1 * cos_theta + n2 * cos_theta2);
    let rp = (n2 * cos_theta - n1 * cos_theta2) / (n2 * cos_theta + n1 * cos_theta2);
    (rs * rs + rp * rp) * 0.5
}
