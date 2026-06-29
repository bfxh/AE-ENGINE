use serde::{Deserialize, Serialize};

const SPECTRAL_SAMPLES: usize = 64;
const ANGULAR_SAMPLES: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolographicMaterial {
    pub name: String,
    pub category: MaterialCategory,
    pub spectral_response: SpectralResponse,
    pub angular_response: AngularResponse,
    pub chemical_state: Option<ChemicalMaterialState>,
    pub emission_spectrum: Option<EmissionSpectrum>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterialCategory {
    Metal,
    Dielectric,
    Semiconductor,
    Organic,
    Composite,
    Liquid,
    Gas,
    Plasma,
    VoxelBased,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectralResponse {
    pub wavelengths: Vec<f32>,
    pub reflectance: Vec<f32>,
    pub transmittance: Vec<f32>,
    pub absorptance: Vec<f32>,
    pub roughness_vs_wavelength: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AngularResponse {
    pub incident_angles: Vec<f32>,
    pub reflection_distribution: Vec<Vec<f32>>,
    pub fresnel_coefficients: Vec<[f32; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemicalMaterialState {
    pub oxidation_depth: f32,
    pub hydration_level: f32,
    pub impurity_concentration: Vec<(String, f32)>,
    pub crystal_defect_density: f32,
    pub grain_boundary_density: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionSpectrum {
    pub peak_wavelength: f32,
    pub bandwidth: f32,
    pub intensity: f32,
    pub is_fluorescent: bool,
    pub is_phosphorescent: bool,
    pub phosphorescence_lifetime: f32,
}

impl HolographicMaterial {
    pub fn new(name: &str, category: MaterialCategory) -> Self {
        let wavelengths: Vec<f32> = (0..SPECTRAL_SAMPLES)
            .map(|i| 380.0 + i as f32 * (780.0 - 380.0) / (SPECTRAL_SAMPLES - 1) as f32)
            .collect();

        let angles: Vec<f32> = (0..ANGULAR_SAMPLES)
            .map(|i| i as f32 * std::f32::consts::FRAC_PI_2 / (ANGULAR_SAMPLES - 1) as f32)
            .collect();

        let reflectance = match category {
            MaterialCategory::Metal => wavelengths
                .iter()
                .map(|w| {
                    if *w < 500.0 {
                        0.6
                    } else if *w < 600.0 {
                        0.5
                    } else {
                        0.45
                    }
                })
                .collect(),
            MaterialCategory::Organic => wavelengths
                .iter()
                .map(|w| {
                    if *w < 500.0 {
                        0.05
                    } else if *w < 580.0 {
                        0.3
                    } else {
                        0.15
                    }
                })
                .collect(),
            _ => vec![0.5; SPECTRAL_SAMPLES],
        };

        let abs = wavelengths.iter().map(|&r| 1.0 - r).collect::<Vec<f32>>();

        let angular_dist = angles
            .iter()
            .map(|&a| {
                let mut dist = vec![0.0f32; ANGULAR_SAMPLES];
                let idx = (a / std::f32::consts::FRAC_PI_2 * (ANGULAR_SAMPLES - 1) as f32) as usize;
                if idx < dist.len() {
                    dist[idx] = 1.0;
                }
                dist
            })
            .collect();

        Self {
            name: name.to_string(),
            category,
            spectral_response: SpectralResponse {
                wavelengths,
                reflectance,
                transmittance: vec![0.0; SPECTRAL_SAMPLES],
                absorptance: abs,
                roughness_vs_wavelength: vec![0.5; SPECTRAL_SAMPLES],
            },
            angular_response: AngularResponse {
                incident_angles: angles,
                reflection_distribution: angular_dist,
                fresnel_coefficients: vec![[0.04, 0.04]; ANGULAR_SAMPLES],
            },
            chemical_state: None,
            emission_spectrum: None,
        }
    }

    pub fn with_chemical_state(name: &str, category: MaterialCategory, oxidation: f32) -> Self {
        let mut mat = Self::new(name, category);
        mat.chemical_state = Some(ChemicalMaterialState {
            oxidation_depth: oxidation,
            hydration_level: 0.0,
            impurity_concentration: Vec::new(),
            crystal_defect_density: 0.0,
            grain_boundary_density: 0.0,
        });
        mat.apply_oxidation(oxidation);
        mat
    }

    pub fn apply_oxidation(&mut self, depth: f32) {
        if depth <= 0.0 {
            return;
        }

        let t = (depth / 10.0).min(1.0);

        for i in 0..SPECTRAL_SAMPLES {
            let wl = self.spectral_response.wavelengths[i];

            let red_shift = if wl > 600.0 {
                1.0 + t * 0.3
            } else if wl < 500.0 {
                1.0 - t * 0.4
            } else {
                1.0 - t * 0.1
            };

            self.spectral_response.reflectance[i] =
                (self.spectral_response.reflectance[i] * red_shift).clamp(0.0, 1.0);
            self.spectral_response.transmittance[i] *= 1.0 - t * 0.5;
            self.spectral_response.roughness_vs_wavelength[i] =
                (self.spectral_response.roughness_vs_wavelength[i] + t * 0.3).min(1.0);
        }

        if let Some(ref mut state) = self.chemical_state {
            state.oxidation_depth = depth;
        }
    }

    pub fn get_color_at_angle(&self, wavelength: f32, incident_angle: f32) -> [f32; 3] {
        let wl_idx = self
            .spectral_response
            .wavelengths
            .iter()
            .position(|&w| w >= wavelength)
            .unwrap_or(SPECTRAL_SAMPLES - 1);

        let angle_idx = self
            .angular_response
            .incident_angles
            .iter()
            .position(|&a| a >= incident_angle)
            .unwrap_or(ANGULAR_SAMPLES - 1);

        let r = self.spectral_response.reflectance[wl_idx];
        let t = self.spectral_response.transmittance[wl_idx];
        let fresnel = self.angular_response.fresnel_coefficients[angle_idx];

        let effective_r = r * (1.0 - fresnel[0]) + fresnel[0];
        let _effective_t = t * (1.0 - fresnel[1]);

        self.wavelength_to_rgb(wavelength, effective_r)
    }

    fn wavelength_to_rgb(&self, wavelength: f32, intensity: f32) -> [f32; 3] {
        let (r, g, b) = if (380.0..440.0).contains(&wavelength) {
            let t = (wavelength - 380.0) / 60.0;
            (-t, 0.0, 1.0)
        } else if (440.0..490.0).contains(&wavelength) {
            let t = (wavelength - 440.0) / 50.0;
            (0.0, t, 1.0)
        } else if (490.0..510.0).contains(&wavelength) {
            let t = (wavelength - 490.0) / 20.0;
            (0.0, 1.0, 1.0 - t)
        } else if (510.0..580.0).contains(&wavelength) {
            let t = (wavelength - 510.0) / 70.0;
            (t, 1.0, 0.0)
        } else if (580.0..645.0).contains(&wavelength) {
            let t = (wavelength - 580.0) / 65.0;
            (1.0, 1.0 - t, 0.0)
        } else if (645.0..=780.0).contains(&wavelength) {
            let _t = (wavelength - 645.0) / 135.0;
            (1.0, 0.0, 0.0)
        } else {
            (0.0, 0.0, 0.0)
        };

        let factor = if (380.0..420.0).contains(&wavelength) {
            0.3 + 0.7 * (wavelength - 380.0) / 40.0
        } else if wavelength > 700.0 && wavelength <= 780.0 {
            0.3 + 0.7 * (780.0 - wavelength) / 80.0
        } else {
            1.0
        };

        [
            (r * factor * intensity).clamp(0.0, 1.0),
            (g * factor * intensity).clamp(0.0, 1.0),
            (b * factor * intensity).clamp(0.0, 1.0),
        ]
    }

    pub fn integrate_spectrum(&self) -> [f32; 3] {
        let mut total = [0.0f32; 3];
        let samples: usize = 32;

        for i in 0..samples {
            let wavelength = 380.0 + (780.0 - 380.0) * i as f32 / (samples - 1) as f32;
            let color = self.get_color_at_angle(wavelength, 0.0);
            total[0] += color[0];
            total[1] += color[1];
            total[2] += color[2];
        }

        let n = samples as f32;
        [total[0] / n, total[1] / n, total[2] / n]
    }

    pub fn iron_rusted() -> Self {
        Self::with_chemical_state("RustedIron", MaterialCategory::Metal, 8.0)
    }

    pub fn concrete_weathered() -> Self {
        let mut mat = Self::new("WeatheredConcrete", MaterialCategory::Dielectric);
        mat.chemical_state = Some(ChemicalMaterialState {
            oxidation_depth: 2.0,
            hydration_level: 0.6,
            impurity_concentration: vec![("sulfur".into(), 0.05), ("carbon".into(), 0.1)],
            crystal_defect_density: 0.3,
            grain_boundary_density: 0.4,
        });
        for i in 0..SPECTRAL_SAMPLES {
            mat.spectral_response.reflectance[i] = 0.4 + i as f32 * 0.1 / SPECTRAL_SAMPLES as f32;
            mat.spectral_response.roughness_vs_wavelength[i] = 0.8;
        }
        mat
    }

    pub fn organic_bark() -> Self {
        let mut mat = Self::new("TreeBark", MaterialCategory::Organic);
        mat.chemical_state = Some(ChemicalMaterialState {
            oxidation_depth: 0.5,
            hydration_level: 0.3,
            impurity_concentration: vec![("lignin".into(), 0.3), ("cellulose".into(), 0.5)],
            crystal_defect_density: 0.6,
            grain_boundary_density: 0.8,
        });
        for i in 0..SPECTRAL_SAMPLES {
            let wl = mat.spectral_response.wavelengths[i];
            mat.spectral_response.reflectance[i] = if wl < 500.0 {
                0.05
            } else if wl < 600.0 {
                0.15
            } else {
                0.25
            };
            mat.spectral_response.roughness_vs_wavelength[i] = 0.95;
        }
        mat
    }
}
