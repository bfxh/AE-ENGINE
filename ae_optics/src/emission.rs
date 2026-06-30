use crate::spectrum::Spectrum;
use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlamePreset {
    Candle,
    Campfire,
    Incandescent,
    Daylight,
    ArcLamp,
    Lightning,
}

impl FlamePreset {
    pub fn temperature(&self) -> f32 {
        match self {
            FlamePreset::Candle => 1900.0,
            FlamePreset::Campfire => 2500.0,
            FlamePreset::Incandescent => 2800.0,
            FlamePreset::Daylight => 5600.0,
            FlamePreset::ArcLamp => 8000.0,
            FlamePreset::Lightning => 30000.0,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            FlamePreset::Candle => "Candle",
            FlamePreset::Campfire => "Campfire",
            FlamePreset::Incandescent => "Incandescent",
            FlamePreset::Daylight => "Daylight",
            FlamePreset::ArcLamp => "Arc Lamp",
            FlamePreset::Lightning => "Lightning",
        }
    }

    pub fn all() -> [FlamePreset; 6] {
        [
            FlamePreset::Candle,
            FlamePreset::Campfire,
            FlamePreset::Incandescent,
            FlamePreset::Daylight,
            FlamePreset::ArcLamp,
            FlamePreset::Lightning,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackbodySpectrum {
    pub temperature: f32,
    pub spectrum: Spectrum,
    pub rgb: Vec3,
}

impl BlackbodySpectrum {
    pub fn new(temperature: f32) -> Self {
        let spectrum = Spectrum::new_blackbody(temperature);
        let rgb = spectrum.to_rgb();
        Self { temperature, spectrum, rgb }
    }

    pub fn from_preset(preset: FlamePreset) -> Self {
        Self::new(preset.temperature())
    }

    pub fn radiance(&self) -> f32 {
        const STEFAN_BOLTZMANN: f32 = 5.670367e-8;
        STEFAN_BOLTZMANN * self.temperature.powi(4)
    }

    pub fn peak_wavelength(&self) -> f32 {
        const WIEN: f32 = 2.897772e-3;
        WIEN / self.temperature
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomEmission {
    pub spectrum: Spectrum,
    pub intensity: f32,
}

impl CustomEmission {
    pub fn new(spectrum: Spectrum, intensity: f32) -> Self {
        Self { spectrum, intensity }
    }

    pub fn rgb_from_wavelengths(wavelengths: &[(f32, f32)]) -> Self {
        let mut spectrum = Spectrum::new_constant(0.0);
        for &(wl, amplitude) in wavelengths {
            let wl_clamped = wl.clamp(spectrum.min_wavelength, spectrum.max_wavelength);
            let idx = ((wl_clamped - spectrum.min_wavelength)
                / (spectrum.max_wavelength - spectrum.min_wavelength)
                * (spectrum.samples.len() - 1) as f32) as usize;
            if idx < spectrum.samples.len() {
                spectrum.samples[idx] += amplitude;
            }
        }
        Self { spectrum, intensity: 1.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissiveMaterial {
    pub emission_type: EmissionType,
    pub power: f32,
    pub two_sided: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmissionType {
    Blackbody(BlackbodySpectrum),
    Custom(CustomEmission),
    Preset(FlamePreset),
}

impl EmissionType {
    pub fn spectrum(&self) -> Spectrum {
        match self {
            EmissionType::Blackbody(bb) => bb.spectrum.clone(),
            EmissionType::Custom(ce) => ce.spectrum.clone(),
            EmissionType::Preset(preset) => {
                let bb = BlackbodySpectrum::from_preset(*preset);
                bb.spectrum
            },
        }
    }

    pub fn rgb(&self) -> Vec3 {
        match self {
            EmissionType::Blackbody(bb) => bb.rgb,
            EmissionType::Custom(ce) => ce.spectrum.to_rgb(),
            EmissionType::Preset(preset) => {
                let bb = BlackbodySpectrum::from_preset(*preset);
                bb.rgb
            },
        }
    }

    pub fn intensity(&self) -> f32 {
        match self {
            EmissionType::Blackbody(bb) => bb.radiance(),
            EmissionType::Custom(ce) => ce.intensity,
            EmissionType::Preset(preset) => {
                let bb = BlackbodySpectrum::from_preset(*preset);
                bb.radiance()
            },
        }
    }
}

impl EmissiveMaterial {
    pub fn new_blackbody(temperature: f32, power: f32) -> Self {
        Self {
            emission_type: EmissionType::Blackbody(BlackbodySpectrum::new(temperature)),
            power,
            two_sided: false,
        }
    }

    pub fn from_preset(preset: FlamePreset, power: f32) -> Self {
        Self { emission_type: EmissionType::Preset(preset), power, two_sided: false }
    }

    pub fn new_custom(spectrum: Spectrum, intensity: f32, power: f32) -> Self {
        Self {
            emission_type: EmissionType::Custom(CustomEmission::new(spectrum, intensity)),
            power,
            two_sided: false,
        }
    }

    pub fn radiance(&self) -> Vec3 {
        let base_rgb = self.emission_type.rgb();
        let intensity = self.emission_type.intensity();
        base_rgb * intensity * self.power
    }

    pub fn spectrum(&self) -> Spectrum {
        self.emission_type.spectrum().scale(self.power)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flame_presets() {
        let presets = FlamePreset::all();
        assert_eq!(presets.len(), 6);
    }

    #[test]
    fn test_flame_temperatures() {
        assert!((FlamePreset::Candle.temperature() - 1900.0).abs() < 0.01);
        assert!((FlamePreset::Daylight.temperature() - 5600.0).abs() < 0.01);
        assert!((FlamePreset::Lightning.temperature() - 30000.0).abs() < 0.01);
    }

    #[test]
    fn test_blackbody_spectrum() {
        let bb = BlackbodySpectrum::new(5600.0);
        assert!(bb.rgb.x > 0.0);
        assert!(bb.radiance() > 0.0);
        assert!(bb.peak_wavelength() > 0.0);
    }

    #[test]
    fn test_blackbody_from_preset() {
        let bb = BlackbodySpectrum::from_preset(FlamePreset::Campfire);
        assert!((bb.temperature - 2500.0).abs() < 0.01);
    }

    #[test]
    fn test_wien_displacement() {
        let bb = BlackbodySpectrum::new(5600.0);
        let peak = bb.peak_wavelength();
        let expected = 2.897_772e-3 / 5600.0;
        assert!((peak - expected).abs() < 1e-9);
    }

    #[test]
    fn test_emissive_material_blackbody() {
        let mat = EmissiveMaterial::new_blackbody(5600.0, 100.0);
        let rad = mat.radiance();
        assert!(rad.x > 0.0);
        assert!(rad.y > 0.0);
        assert!(rad.z > 0.0);
    }

    #[test]
    fn test_emissive_material_preset() {
        let mat = EmissiveMaterial::from_preset(FlamePreset::ArcLamp, 50.0);
        let rad = mat.radiance();
        assert!(rad.x > 0.0);
        let spectrum = mat.spectrum();
        assert!(spectrum.samples.iter().any(|&s| s > 0.0));
    }

    #[test]
    fn test_emissive_material_custom() {
        let spectrum = Spectrum::new_constant(0.5);
        let mat = EmissiveMaterial::new_custom(spectrum, 1.0, 10.0);
        let rad = mat.radiance();
        assert!(rad.x > 0.0);
    }

    #[test]
    fn test_custom_emission_wavelengths() {
        let em = CustomEmission::rgb_from_wavelengths(&[(500.0, 1.0), (600.0, 0.5)]);
        assert!(em.intensity > 0.0);
    }

    #[test]
    fn test_emission_type_spectrum() {
        let et = EmissionType::Preset(FlamePreset::Candle);
        let spectrum = et.spectrum();
        assert!(spectrum.samples.iter().any(|&s| s > 0.0));
    }

    #[test]
    fn test_all_preset_rgb() {
        for preset in FlamePreset::all() {
            let mat = EmissiveMaterial::from_preset(preset, 1.0);
            let rad = mat.radiance();
            assert!(rad.x.is_finite());
            assert!(rad.y.is_finite());
            assert!(rad.z.is_finite());
        }
    }
}
