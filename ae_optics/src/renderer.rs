use crate::materials::*;
use crate::spectrum::*;
use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightSource {
    pub position: Vec3,
    pub intensity: Spectrum,
    pub direction: Vec3,
    pub color: Vec3,
}

impl LightSource {
    pub fn new_point(position: Vec3, color: Vec3, power: f32) -> Self {
        Self { position, intensity: rgb_to_spectrum(color).scale(power), direction: Vec3::Y, color }
    }

    pub fn new_directional(direction: Vec3, color: Vec3, power: f32) -> Self {
        Self {
            position: Vec3::ZERO,
            intensity: rgb_to_spectrum(color).scale(power),
            direction: direction.normalize(),
            color,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpticalPath {
    pub origin: Vec3,
    pub direction: Vec3,
    pub length: f32,
    pub attenuation: Spectrum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectralRenderer {
    pub lights: Vec<LightSource>,
    pub max_bounces: u32,
    pub samples_per_pixel: u32,
}

impl SpectralRenderer {
    pub fn new() -> Self {
        Self { lights: Vec::new(), max_bounces: 5, samples_per_pixel: 1 }
    }

    pub fn add_light(&mut self, light: LightSource) {
        self.lights.push(light);
    }

    pub fn trace_path(
        &self,
        origin: Vec3,
        direction: Vec3,
        _material: &OpticalMaterial,
    ) -> Spectrum {
        let mut radiance = Spectrum::new_constant(0.0);
        let mut attenuation = Spectrum::new_constant(1.0);
        let mut current_origin = origin;
        let mut current_dir = direction;

        for _bounce in 0..self.max_bounces {
            let mut closest_t = f32::INFINITY;
            let mut hit_light = None;

            for light in &self.lights {
                let to_light = light.position - current_origin;
                let t = to_light.dot(current_dir);
                if t > 0.0 && t < closest_t {
                    let closest_point = current_origin + current_dir * t;
                    let dist = (closest_point - light.position).length();
                    if dist < 0.1 {
                        closest_t = t;
                        hit_light = Some(light);
                    }
                }
            }

            if let Some(light) = hit_light {
                radiance = radiance.add(&light.intensity.multiply(&attenuation));
                break;
            }

            let reflectivity = fresnel(current_dir.dot(-Vec3::Y).max(0.0), 1.0, 1.5);
            let reflect = Spectrum::new_constant(reflectivity);
            let _transmit = Spectrum::new_constant(1.0 - reflectivity);
            radiance = radiance.add(&Spectrum::new_constant(0.0).multiply(&attenuation));

            attenuation = attenuation.multiply(&reflect);
            current_dir = current_dir - 2.0 * current_dir.dot(Vec3::Y) * Vec3::Y;
            current_origin += current_dir * 0.01;
        }

        radiance
    }

    pub fn compute_ambient(&self, position: Vec3) -> Spectrum {
        let mut ambient = Spectrum::new_constant(0.0);
        for light in &self.lights {
            let dist = (light.position - position).length().max(0.1);
            let attenuation = 1.0 / (dist * dist);
            ambient = ambient.add(&light.intensity.scale(attenuation * 0.01));
        }
        ambient
    }

    pub fn sample(&self, position: Vec3, normal: Vec3, _material: &OpticalMaterial) -> Vec3 {
        let mut total = Spectrum::new_constant(0.0);

        for light in &self.lights {
            let to_light = (light.position - position).normalize();
            let cos_theta = normal.dot(to_light).max(0.0);
            let dist = (light.position - position).length().max(0.1);
            let attenuation = 1.0 / (dist * dist);
            let light_contrib = light.intensity.scale(cos_theta * attenuation);
            total = total.add(&light_contrib);
        }

        total = total.add(&self.compute_ambient(position));
        total = total.multiply(&Spectrum::new_constant(1.0));

        total.to_rgb()
    }
}

impl Default for SpectralRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectrum_rgb_conversion() {
        let rgb = Vec3::new(1.0, 0.0, 0.0);
        let spectrum = rgb_to_spectrum(rgb);
        let result = spectrum.to_rgb();
        assert!((result.x - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_fresnel() {
        let f = fresnel(1.0, 1.0, 1.5);
        assert!(f < 1.0 && f > 0.0);
    }

    #[test]
    fn test_blackbody() {
        let spectrum = Spectrum::new_blackbody(5800.0);
        let rgb = spectrum.to_rgb();
        assert!(rgb.x > 0.8);
        assert!(rgb.y > 0.8);
        assert!(rgb.z > 0.7);
    }
}
