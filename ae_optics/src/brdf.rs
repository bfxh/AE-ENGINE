use glam::Vec3;
use serde::{Deserialize, Serialize};

pub trait BrdfTrait: Send + Sync {
    fn evaluate(&self, wi: Vec3, wo: Vec3, normal: Vec3) -> Vec3;
    fn pdf(&self, wi: Vec3, wo: Vec3, normal: Vec3) -> f32;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LambertianBrdf {
    pub albedo: Vec3,
}

impl LambertianBrdf {
    pub fn new(albedo: Vec3) -> Self {
        Self { albedo }
    }
}

impl BrdfTrait for LambertianBrdf {
    fn evaluate(&self, _wi: Vec3, wo: Vec3, normal: Vec3) -> Vec3 {
        let cos_theta = normal.dot(wo).max(0.0);
        self.albedo * std::f32::consts::FRAC_1_PI * cos_theta
    }

    fn pdf(&self, _wi: Vec3, wo: Vec3, normal: Vec3) -> f32 {
        let cos_theta = normal.dot(wo).max(0.0);
        cos_theta * std::f32::consts::FRAC_1_PI
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecularBrdf {
    pub color: Vec3,
    pub exponent: f32,
}

impl SpecularBrdf {
    pub fn new(color: Vec3, exponent: f32) -> Self {
        Self { color, exponent }
    }
}

impl BrdfTrait for SpecularBrdf {
    fn evaluate(&self, wi: Vec3, wo: Vec3, normal: Vec3) -> Vec3 {
        let reflect = (2.0 * normal.dot(wi) * normal - wi).normalize();
        let cos_alpha = reflect.dot(wo).max(0.0);
        let spec = cos_alpha.powf(self.exponent);
        let normalization = (self.exponent + 2.0) / (2.0 * std::f32::consts::PI);
        self.color * spec * normalization
    }

    fn pdf(&self, wi: Vec3, wo: Vec3, normal: Vec3) -> f32 {
        let reflect = (2.0 * normal.dot(wi) * normal - wi).normalize();
        let cos_alpha = reflect.dot(wo).max(0.0);
        let spec = cos_alpha.powf(self.exponent);
        spec * (self.exponent + 1.0) / (2.0 * std::f32::consts::PI)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrofacetBrdf {
    pub color: Vec3,
    pub roughness: f32,
    pub metallic: f32,
    pub fresnel_f0: Vec3,
}

impl MicrofacetBrdf {
    pub fn new(color: Vec3, roughness: f32, metallic: f32) -> Self {
        let f0 = Vec3::splat(0.04).lerp(color, metallic);
        Self { color, roughness: roughness.clamp(0.001, 1.0), metallic, fresnel_f0: f0 }
    }

    pub fn ggx_distribution(nh: Vec3, roughness: f32) -> f32 {
        let alpha = roughness * roughness;
        let alpha2 = alpha * alpha;
        let cos_theta = nh.z.max(0.0);
        let cos2 = cos_theta * cos_theta;
        let denom = cos2 * (alpha2 - 1.0) + 1.0;
        alpha2 / (std::f32::consts::PI * denom * denom)
    }

    pub fn geometry_smith(n: Vec3, v: Vec3, roughness: f32) -> f32 {
        let cos_theta = n.dot(v).max(0.0);
        let alpha = roughness * roughness;
        let k = (alpha + 1.0) * (alpha + 1.0) / 8.0;
        cos_theta / (cos_theta * (1.0 - k) + k)
    }

    pub fn fresnel_schlick(f0: Vec3, cos_theta: f32) -> Vec3 {
        let t = (1.0 - cos_theta.max(0.0)).powi(5);
        f0 + (Vec3::ONE - f0) * t
    }
}

impl BrdfTrait for MicrofacetBrdf {
    fn evaluate(&self, wi: Vec3, wo: Vec3, normal: Vec3) -> Vec3 {
        let cos_theta_i = normal.dot(wi).max(0.0);
        let cos_theta_o = normal.dot(wo).max(0.0);

        if cos_theta_i < 0.001 || cos_theta_o < 0.001 {
            return Vec3::ZERO;
        }

        let half = (wi + wo).normalize();

        let d = Self::ggx_distribution(half, self.roughness);
        let g = Self::geometry_smith(normal, wi, self.roughness)
            * Self::geometry_smith(normal, wo, self.roughness);
        let f = Self::fresnel_schlick(self.fresnel_f0, half.dot(wo).max(0.0));

        let specular = f * d * g / (4.0 * cos_theta_i * cos_theta_o + 0.001);
        let kd = (Vec3::ONE - f) * (1.0 - self.metallic);
        let diffuse = self.color * kd * std::f32::consts::FRAC_1_PI * cos_theta_o;

        specular + diffuse
    }

    fn pdf(&self, wi: Vec3, wo: Vec3, normal: Vec3) -> f32 {
        let half = (wi + wo).normalize();
        let cos_theta_h = half.dot(normal).max(0.0);
        let d = Self::ggx_distribution(half, self.roughness);
        let cos_theta_o = normal.dot(wo).max(0.0);
        if cos_theta_o < 0.001 {
            return 0.0;
        }
        d * cos_theta_h / (4.0 * half.dot(wo).abs().max(0.001))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Brdf {
    Lambertian(LambertianBrdf),
    Specular(SpecularBrdf),
    Microfacet(MicrofacetBrdf),
}

impl Brdf {
    pub fn evaluate(&self, wi: Vec3, wo: Vec3, normal: Vec3) -> Vec3 {
        match self {
            Brdf::Lambertian(b) => b.evaluate(wi, wo, normal),
            Brdf::Specular(b) => b.evaluate(wi, wo, normal),
            Brdf::Microfacet(b) => b.evaluate(wi, wo, normal),
        }
    }

    pub fn pdf(&self, wi: Vec3, wo: Vec3, normal: Vec3) -> f32 {
        match self {
            Brdf::Lambertian(b) => b.pdf(wi, wo, normal),
            Brdf::Specular(b) => b.pdf(wi, wo, normal),
            Brdf::Microfacet(b) => b.pdf(wi, wo, normal),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lambertian_evaluate() {
        let brdf = LambertianBrdf::new(Vec3::ONE);
        let normal = Vec3::Z;
        let wo = Vec3::Z;
        let wi = Vec3::new(0.5, 0.5, 0.707).normalize();
        let result = brdf.evaluate(wi, wo, normal);
        assert!(result.x > 0.0);
        assert!(result.y > 0.0);
        assert!(result.z > 0.0);
    }

    #[test]
    fn test_lambertian_pdf() {
        let brdf = LambertianBrdf::new(Vec3::ONE);
        let pdf = brdf.pdf(Vec3::Z, Vec3::Z, Vec3::Z);
        let expected = 1.0 / std::f32::consts::PI;
        assert!((pdf - expected).abs() < 0.01);
    }

    #[test]
    fn test_specular_evaluate() {
        let brdf = SpecularBrdf::new(Vec3::ONE, 32.0);
        let normal = Vec3::Z;
        let wi = Vec3::new(0.3, 0.0, 0.954).normalize();
        let reflect = (2.0 * normal.dot(wi) * normal - wi).normalize();
        let wo = reflect;
        let result = brdf.evaluate(wi, wo, normal);
        assert!(result.x > 0.0);
    }

    #[test]
    fn test_specular_pdf() {
        let brdf = SpecularBrdf::new(Vec3::ONE, 32.0);
        let pdf = brdf.pdf(Vec3::Z, Vec3::Z, Vec3::Z);
        assert!(pdf > 0.0);
    }

    #[test]
    fn test_microfacet_creation() {
        let brdf = MicrofacetBrdf::new(Vec3::new(0.8, 0.6, 0.4), 0.5, 0.0);
        assert!((brdf.roughness - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_microfacet_evaluate() {
        let brdf = MicrofacetBrdf::new(Vec3::new(0.8, 0.6, 0.4), 0.3, 0.0);
        let normal = Vec3::Z;
        let wo = Vec3::Z;
        let wi = Vec3::new(0.3, 0.0, 0.954).normalize();
        let result = brdf.evaluate(wi, wo, normal);
        assert!(result.x.is_finite());
        assert!(result.y.is_finite());
        assert!(result.z.is_finite());
    }

    #[test]
    fn test_microfacet_ggx() {
        let d = MicrofacetBrdf::ggx_distribution(Vec3::Z, 0.5);
        assert!(d > 0.0);
        assert!(d.is_finite());
    }

    #[test]
    fn test_microfacet_geometry() {
        let g = MicrofacetBrdf::geometry_smith(Vec3::Z, Vec3::Z, 0.5);
        assert!(g > 0.0);
        assert!(g <= 1.0 + 0.001);
    }

    #[test]
    fn test_microfacet_fresnel() {
        let f = MicrofacetBrdf::fresnel_schlick(Vec3::splat(0.04), 1.0);
        assert!(f.x > 0.0);
        let f_grazing = MicrofacetBrdf::fresnel_schlick(Vec3::splat(0.04), 0.0);
        assert!((f_grazing.x - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_brdf_enum() {
        let brdf = Brdf::Lambertian(LambertianBrdf::new(Vec3::ONE));
        let result = brdf.evaluate(Vec3::Z, Vec3::Z, Vec3::Z);
        assert!(result.x > 0.0);
    }

    #[test]
    fn test_microfacet_metallic() {
        let brdf = MicrofacetBrdf::new(Vec3::new(0.9, 0.8, 0.7), 0.2, 1.0);
        let normal = Vec3::Z;
        let wo = Vec3::Z;
        let wi = Vec3::new(0.5, 0.0, 0.866).normalize();
        let result = brdf.evaluate(wi, wo, normal);
        assert!(result.x.is_finite());
    }
}
