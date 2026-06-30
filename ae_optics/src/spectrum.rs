use glam::Vec3;
use serde::{Deserialize, Serialize};

pub const MIN_WAVELENGTH: f32 = 380.0;
pub const MAX_WAVELENGTH: f32 = 780.0;
pub const DEFAULT_SAMPLES: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spectrum {
    pub samples: [f32; DEFAULT_SAMPLES],
    pub min_wavelength: f32,
    pub max_wavelength: f32,
}

impl Spectrum {
    pub fn new_constant(value: f32) -> Self {
        Self {
            samples: [value; DEFAULT_SAMPLES],
            min_wavelength: MIN_WAVELENGTH,
            max_wavelength: MAX_WAVELENGTH,
        }
    }

    pub fn new_blackbody(temperature: f32) -> Self {
        let mut samples = [0.0; DEFAULT_SAMPLES];
        let step = (MAX_WAVELENGTH - MIN_WAVELENGTH) / (DEFAULT_SAMPLES - 1) as f32;
        for (i, sample) in samples.iter_mut().enumerate() {
            let lambda = (MIN_WAVELENGTH + i as f32 * step) * 1e-9;
            let lambda5 = lambda.powi(5);
            let exp_arg = (6.626_07e-34 * 299792500.0) / (lambda * 1.380649e-23 * temperature);
            let planck = (2.0 * 6.626_07e-34 * 299792500.0 * 299792500.0)
                / (lambda5 * (exp_arg.exp() - 1.0));
            *sample = planck;
        }
        let mut max_val: f32 = 0.0;
        for &s in &samples {
            if s > max_val {
                max_val = s;
            }
        }
        for s in &mut samples {
            *s /= max_val;
        }
        Self { samples, min_wavelength: MIN_WAVELENGTH, max_wavelength: MAX_WAVELENGTH }
    }

    pub fn wavelength_at(&self, index: usize) -> f32 {
        let step = (self.max_wavelength - self.min_wavelength) / (self.samples.len() - 1) as f32;
        self.min_wavelength + index as f32 * step
    }

    pub fn sample(&self, wavelength: f32) -> f32 {
        if wavelength < self.min_wavelength || wavelength > self.max_wavelength {
            return 0.0;
        }
        let t = (wavelength - self.min_wavelength) / (self.max_wavelength - self.min_wavelength);
        let idx = t * (self.samples.len() - 1) as f32;
        let i = idx.floor() as usize;
        let fract = idx - i as f32;
        if i >= self.samples.len() - 1 {
            return self.samples[self.samples.len() - 1];
        }
        self.samples[i] * (1.0 - fract) + self.samples[i + 1] * fract
    }

    pub fn to_rgb(&self) -> Vec3 {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut z = 0.0;
        for (i, &sample) in self.samples.iter().enumerate() {
            let lambda = self.wavelength_at(i);
            let (cx, cy, cz) = cie_xyz(lambda);
            x += sample * cx;
            y += sample * cy;
            z += sample * cz;
        }
        xyz_to_rgb(Vec3::new(x, y, z))
    }

    pub fn add(&self, other: &Spectrum) -> Self {
        let mut samples = [0.0; DEFAULT_SAMPLES];
        for (i, (&a, &b)) in self.samples.iter().zip(other.samples.iter()).enumerate() {
            samples[i] = a + b;
        }
        Self { samples, min_wavelength: self.min_wavelength, max_wavelength: self.max_wavelength }
    }

    pub fn multiply(&self, other: &Spectrum) -> Self {
        let mut samples = [0.0; DEFAULT_SAMPLES];
        for (i, (&a, &b)) in self.samples.iter().zip(other.samples.iter()).enumerate() {
            samples[i] = a * b;
        }
        Self { samples, min_wavelength: self.min_wavelength, max_wavelength: self.max_wavelength }
    }

    pub fn scale(&self, factor: f32) -> Self {
        let mut samples = [0.0; DEFAULT_SAMPLES];
        for (i, &sample) in self.samples.iter().enumerate() {
            samples[i] = sample * factor;
        }
        Self { samples, min_wavelength: self.min_wavelength, max_wavelength: self.max_wavelength }
    }
}

pub fn cie_xyz(wavelength: f32) -> (f32, f32, f32) {
    let t = (wavelength - MIN_WAVELENGTH) / (MAX_WAVELENGTH - MIN_WAVELENGTH);
    let x = (-(t - 0.25).powi(2) * 30.0).exp() * 1.05;
    let y = (-(t - 0.5).powi(2) * 25.0).exp();
    let z = (-(t - 0.75).powi(2) * 30.0).exp() * 0.95;
    (x, y, z)
}

pub fn xyz_to_rgb(xyz: Vec3) -> Vec3 {
    let r = 3.2406 * xyz.x - 1.5372 * xyz.y - 0.4986 * xyz.z;
    let g = -0.9689 * xyz.x + 1.8758 * xyz.y + 0.0415 * xyz.z;
    let b = 0.0557 * xyz.x - 0.2040 * xyz.y + 1.0570 * xyz.z;
    Vec3::new(r, g, b).clamp(Vec3::ZERO, Vec3::ONE)
}

pub fn rgb_to_spectrum(rgb: Vec3) -> Spectrum {
    let mut samples = [0.0; DEFAULT_SAMPLES];
    for (i, sample) in samples.iter_mut().enumerate() {
        let lambda = MIN_WAVELENGTH
            + (MAX_WAVELENGTH - MIN_WAVELENGTH) * (i as f32 / (DEFAULT_SAMPLES - 1) as f32);
        let (cx, cy, cz) = cie_xyz(lambda);
        let r = rgb.x;
        let g = rgb.y;
        let b = rgb.z;
        let x = 0.412453 * r + 0.357580 * g + 0.180423 * b;
        let y = 0.212671 * r + 0.715160 * g + 0.072169 * b;
        let z = 0.019334 * r + 0.119193 * g + 0.950227 * b;
        let sum = cx + cy + cz + 1e-6;
        *sample = (x * cx + y * cy + z * cz) / sum;
    }
    Spectrum { samples, min_wavelength: MIN_WAVELENGTH, max_wavelength: MAX_WAVELENGTH }
}
