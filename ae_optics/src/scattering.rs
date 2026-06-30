use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    Fog,
    Smoke,
    Cloud,
    Underwater,
    Atmosphere,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumetricScattering {
    pub media_type: MediaType,
    pub density: f32,
    pub albedo: Vec3,
    pub anisotropy: f32,
    pub extinction: Vec3,
    pub scattering_coeff: Vec3,
    pub absorption_coeff: Vec3,
}

impl VolumetricScattering {
    pub fn new(media_type: MediaType, density: f32) -> Self {
        let (albedo, anisotropy, scattering, absorption) = match media_type {
            MediaType::Fog => (Vec3::splat(0.99), 0.0, Vec3::splat(0.1), Vec3::splat(0.001)),
            MediaType::Smoke => {
                (Vec3::new(0.01, 0.01, 0.01), 0.1, Vec3::splat(0.5), Vec3::splat(0.5))
            },
            MediaType::Cloud => (Vec3::splat(0.999), 0.85, Vec3::splat(0.05), Vec3::splat(0.0001)),
            MediaType::Underwater => (
                Vec3::new(0.3, 0.7, 0.9),
                0.95,
                Vec3::new(0.02, 0.01, 0.005),
                Vec3::new(0.05, 0.02, 0.1),
            ),
            MediaType::Atmosphere => (
                Vec3::new(0.5, 0.6, 0.8),
                0.0,
                Vec3::new(0.002, 0.005, 0.01),
                Vec3::new(0.0, 0.0, 0.0),
            ),
        };

        Self {
            media_type,
            density,
            albedo,
            anisotropy,
            extinction: scattering + absorption,
            scattering_coeff: scattering * density,
            absorption_coeff: absorption * density,
        }
    }

    pub fn set_density(&mut self, density: f32) {
        self.density = density;
        self.scattering_coeff = self.scattering_coeff / self.density.max(0.001) * density;
        self.absorption_coeff = self.absorption_coeff / self.density.max(0.001) * density;
        self.extinction = self.scattering_coeff + self.absorption_coeff;
    }

    pub fn rayleigh_phase(cos_theta: f32) -> f32 {
        3.0 / (16.0 * std::f32::consts::PI) * (1.0 + cos_theta * cos_theta)
    }

    pub fn mie_phase(cos_theta: f32, anisotropy: f32) -> f32 {
        let g = anisotropy;
        let g2 = g * g;
        let denom = (1.0 + g2 - 2.0 * g * cos_theta).powf(1.5);
        (1.0 - g2) / (4.0 * std::f32::consts::PI * denom.max(0.001))
    }

    pub fn phase_function(&self, cos_theta: f32) -> f32 {
        match self.media_type {
            MediaType::Atmosphere => {
                Self::rayleigh_phase(cos_theta) * 0.5
                    + Self::mie_phase(cos_theta, self.anisotropy) * 0.5
            },
            MediaType::Cloud | MediaType::Smoke => Self::mie_phase(cos_theta, self.anisotropy),
            _ => Self::mie_phase(cos_theta, self.anisotropy),
        }
    }

    pub fn transmittance(&self, distance: f32) -> Vec3 {
        let ext = self.extinction;
        Vec3::new((-ext.x * distance).exp(), (-ext.y * distance).exp(), (-ext.z * distance).exp())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipatingMedia {
    pub scattering: VolumetricScattering,
    pub density_field: DensityField,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DensityField {
    pub resolution: (usize, usize, usize),
    pub bounds: (Vec3, Vec3),
    pub data: Vec<f32>,
}

impl DensityField {
    pub fn new(resolution: (usize, usize, usize), bounds: (Vec3, Vec3)) -> Self {
        let (nx, ny, nz) = resolution;
        let total = nx * ny * nz;
        Self { resolution, bounds, data: vec![0.0; total] }
    }

    pub fn uniform(resolution: (usize, usize, usize), bounds: (Vec3, Vec3), value: f32) -> Self {
        let (nx, ny, nz) = resolution;
        let total = nx * ny * nz;
        Self { resolution, bounds, data: vec![value; total] }
    }

    pub fn index(&self, x: usize, y: usize, z: usize) -> usize {
        let (nx, ny, _) = self.resolution;
        x + y * nx + z * nx * ny
    }

    pub fn sample(&self, pos: Vec3) -> f32 {
        let (nx, ny, nz) = self.resolution;
        let size = self.bounds.1 - self.bounds.0;
        let local = (pos - self.bounds.0) / size;
        let ix = (local.x * (nx - 1) as f32).floor() as isize;
        let iy = (local.y * (ny - 1) as f32).floor() as isize;
        let iz = (local.z * (nz - 1) as f32).floor() as isize;

        if ix < 0 || ix >= nx as isize || iy < 0 || iy >= ny as isize || iz < 0 || iz >= nz as isize
        {
            return 0.0;
        }

        let tx = local.x * (nx - 1) as f32 - ix as f32;
        let ty = local.y * (ny - 1) as f32 - iy as f32;
        let tz = local.z * (nz - 1) as f32 - iz as f32;

        let mut val = 0.0_f32;
        for dz in 0..=1 {
            for dy in 0..=1 {
                for dx in 0..=1 {
                    let x = (ix + dx as isize) as usize;
                    let y = (iy + dy as isize) as usize;
                    let z = (iz + dz as isize) as usize;
                    let idx = self.index(x, y, z);
                    let weight = (if dx == 0 { 1.0 - tx } else { tx })
                        * (if dy == 0 { 1.0 - ty } else { ty })
                        * (if dz == 0 { 1.0 - tz } else { tz });
                    val += self.data[idx] * weight;
                }
            }
        }
        val
    }

    pub fn ray_march(
        &self,
        origin: Vec3,
        direction: Vec3,
        step_size: f32,
        max_distance: f32,
    ) -> f32 {
        let mut total = 0.0_f32;
        let dir = direction.normalize();
        let mut t = 0.0_f32;
        while t < max_distance {
            let pos = origin + dir * t;
            total += self.sample(pos) * step_size;
            t += step_size;
        }
        total
    }
}

impl ParticipatingMedia {
    pub fn new(media_type: MediaType, density: f32) -> Self {
        let resolution = (32, 32, 32);
        let bounds = (Vec3::splat(-10.0), Vec3::splat(10.0));
        let density_field = DensityField::uniform(resolution, bounds, density);

        Self {
            scattering: VolumetricScattering::new(media_type, density),
            density_field,
            temperature: 300.0,
        }
    }

    pub fn sample_density(&self, pos: Vec3) -> f32 {
        self.density_field.sample(pos)
    }

    pub fn transmittance(&self, origin: Vec3, direction: Vec3, distance: f32) -> Vec3 {
        let optical_depth = self.density_field.ray_march(origin, direction, 0.1, distance);
        let ext = self.scattering.extinction;
        Vec3::new(
            (-ext.x * optical_depth).exp(),
            (-ext.y * optical_depth).exp(),
            (-ext.z * optical_depth).exp(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volumetric_scattering_creation() {
        let vs = VolumetricScattering::new(MediaType::Fog, 0.5);
        assert!((vs.density - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_rayleigh_phase() {
        let phase = VolumetricScattering::rayleigh_phase(0.0);
        assert!(phase > 0.0);
        let phase_90 = VolumetricScattering::rayleigh_phase(0.0_f32.cos());
        assert!(phase_90 > 0.0);
    }

    #[test]
    fn test_mie_phase() {
        let phase = VolumetricScattering::mie_phase(1.0, 0.5);
        assert!(phase > 0.0);
    }

    #[test]
    fn test_phase_function() {
        let vs = VolumetricScattering::new(MediaType::Atmosphere, 0.1);
        let phase = vs.phase_function(0.5);
        assert!(phase > 0.0);
        assert!(phase.is_finite());
    }

    #[test]
    fn test_transmittance() {
        let vs = VolumetricScattering::new(MediaType::Fog, 0.1);
        let tr = vs.transmittance(1.0);
        assert!(tr.x > 0.0);
        assert!(tr.x < 1.0);
    }

    #[test]
    fn test_density_field() {
        let field = DensityField::uniform((8, 8, 8), (Vec3::ZERO, Vec3::ONE), 1.0);
        let val = field.sample(Vec3::new(0.5, 0.5, 0.5));
        assert!((val - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_density_field_out_of_bounds() {
        let field = DensityField::uniform((8, 8, 8), (Vec3::ZERO, Vec3::ONE), 1.0);
        let val = field.sample(Vec3::new(-1.0, -1.0, -1.0));
        assert!((val - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_density_field_ray_march() {
        let field = DensityField::uniform((8, 8, 8), (Vec3::splat(-10.0), Vec3::splat(10.0)), 0.5);
        let total = field.ray_march(Vec3::ZERO, Vec3::X, 0.5, 10.0);
        assert!(total > 0.0);
    }

    #[test]
    fn test_participating_media() {
        let media = ParticipatingMedia::new(MediaType::Smoke, 0.3);
        let density = media.sample_density(Vec3::ZERO);
        assert!(density > 0.0);
    }

    #[test]
    fn test_participating_media_transmittance() {
        let media = ParticipatingMedia::new(MediaType::Fog, 0.1);
        let tr = media.transmittance(Vec3::ZERO, Vec3::X, 5.0);
        assert!(tr.x > 0.0);
        assert!(tr.x < 1.0);
    }

    #[test]
    fn test_all_media_types() {
        let types = [
            MediaType::Fog,
            MediaType::Smoke,
            MediaType::Cloud,
            MediaType::Underwater,
            MediaType::Atmosphere,
        ];
        for t in &types {
            let vs = VolumetricScattering::new(*t, 0.5);
            assert!(vs.phase_function(0.0) > 0.0);
        }
    }

    #[test]
    fn test_set_density() {
        let mut vs = VolumetricScattering::new(MediaType::Fog, 0.5);
        vs.set_density(1.0);
        assert!((vs.density - 1.0).abs() < 0.01);
    }
}
