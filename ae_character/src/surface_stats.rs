use glam::Vec3;
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::skeleton::BodyPart;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SurfaceStats {
    pub roughness: f32,
    pub texture_direction: Vec3,
    pub bump_density: f32,
    pub bump_height_mean: f32,
    pub bump_height_stddev: f32,
    pub hardness: f32,
    pub friction_coefficient: f32,
}

impl Default for SurfaceStats {
    fn default() -> Self {
        Self {
            roughness: 0.1,
            texture_direction: Vec3::X,
            bump_density: 10.0,
            bump_height_mean: 0.001,
            bump_height_stddev: 0.0005,
            hardness: 5.0,
            friction_coefficient: 0.5,
        }
    }
}

impl SurfaceStats {
    pub fn metal_blade() -> Self {
        Self {
            roughness: 0.05,
            texture_direction: Vec3::Y,
            bump_density: 5.0,
            bump_height_mean: 0.0002,
            bump_height_stddev: 0.0001,
            hardness: 8.0,
            friction_coefficient: 0.3,
        }
    }

    pub fn stone_rough() -> Self {
        Self {
            roughness: 0.85,
            texture_direction: Vec3::ZERO,
            bump_density: 50.0,
            bump_height_mean: 0.005,
            bump_height_stddev: 0.003,
            hardness: 7.0,
            friction_coefficient: 0.7,
        }
    }

    pub fn wood_grain() -> Self {
        Self {
            roughness: 0.5,
            texture_direction: Vec3::Y,
            bump_density: 20.0,
            bump_height_mean: 0.002,
            bump_height_stddev: 0.001,
            hardness: 2.0,
            friction_coefficient: 0.4,
        }
    }

    pub fn flesh() -> Self {
        Self {
            roughness: 0.3,
            texture_direction: Vec3::ZERO,
            bump_density: 30.0,
            bump_height_mean: 0.003,
            bump_height_stddev: 0.002,
            hardness: 0.5,
            friction_coefficient: 0.8,
        }
    }

    pub fn iron_plate() -> Self {
        Self {
            roughness: 0.3,
            texture_direction: Vec3::ZERO,
            bump_density: 15.0,
            bump_height_mean: 0.001,
            bump_height_stddev: 0.0005,
            hardness: 4.0,
            friction_coefficient: 0.7,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MicroContactPatch {
    pub points: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub total_area: f32,
    pub asperity_count: u32,
    pub mean_pressure: f32,
}

impl MicroContactPatch {
    pub fn generate(
        surface_a: &SurfaceStats,
        surface_b: &SurfaceStats,
        contact_normal: Vec3,
        base_area: f32,
        rng: &mut impl Rng,
    ) -> Self {
        let combined_roughness = (surface_a.roughness * surface_a.roughness
            + surface_b.roughness * surface_b.roughness)
            .sqrt();
        let asperity_count = (base_area * surface_a.bump_density * surface_b.bump_density * 0.01)
            .clamp(1.0, 200.0) as u32;

        let mut points = Vec::with_capacity(asperity_count as usize);
        let mut normals = Vec::with_capacity(asperity_count as usize);
        let radius = (base_area / std::f32::consts::PI).sqrt();

        for _ in 0..asperity_count {
            let angle = rng.gen::<f32>() * 2.0 * std::f32::consts::PI;
            let r = rng.gen::<f32>().sqrt() * radius;
            let x = r * angle.cos();
            let z = r * angle.sin();

            let height_a = surface_a.bump_height_mean
                + rng.gen::<f32>() * surface_a.bump_height_stddev * 2.0
                - surface_a.bump_height_stddev;
            let height_b = surface_b.bump_height_mean
                + rng.gen::<f32>() * surface_b.bump_height_stddev * 2.0
                - surface_b.bump_height_stddev;

            let offset = height_a + height_b;
            let point = Vec3::new(x, offset * combined_roughness, z);

            let normal_deviation = combined_roughness * (rng.gen::<f32>() - 0.5) * 0.5;
            let tangent1 = contact_normal.any_orthogonal_vector();
            let tangent2 = contact_normal.cross(tangent1);
            let normal =
                (contact_normal + tangent1 * normal_deviation + tangent2 * normal_deviation * 0.5)
                    .normalize();

            points.push(point);
            normals.push(normal);
        }

        let actual_contact_area = base_area * (1.0 - combined_roughness * 0.3).max(0.1);

        Self {
            points,
            normals,
            total_area: actual_contact_area,
            asperity_count,
            mean_pressure: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceProfile {
    pub body_part: BodyPart,
    pub base_stats: SurfaceStats,
    pub current_stats: SurfaceStats,
    pub bone_id: Uuid,
    pub contact_area_estimate: f32,
}

impl SurfaceProfile {
    pub fn new(body_part: BodyPart, bone_id: Uuid, stats: SurfaceStats, contact_area: f32) -> Self {
        Self {
            body_part,
            base_stats: stats,
            current_stats: stats,
            bone_id,
            contact_area_estimate: contact_area,
        }
    }

    pub fn for_body_part(body_part: BodyPart, bone_id: Uuid) -> Self {
        let (stats, area) = match body_part {
            BodyPart::HandL | BodyPart::HandR => (SurfaceStats::flesh(), 0.02),
            BodyPart::Head => (SurfaceStats::flesh(), 0.04),
            BodyPart::Torso => (SurfaceStats::flesh(), 0.25),
            BodyPart::UpperArmL
            | BodyPart::UpperArmR
            | BodyPart::LowerArmL
            | BodyPart::LowerArmR => (SurfaceStats::flesh(), 0.03),
            BodyPart::UpperLegL
            | BodyPart::UpperLegR
            | BodyPart::LowerLegL
            | BodyPart::LowerLegR => (SurfaceStats::flesh(), 0.04),
            BodyPart::FootL | BodyPart::FootR => (SurfaceStats::flesh(), 0.03),
            _ => (SurfaceStats::default(), 0.05),
        };
        Self::new(body_part, bone_id, stats, area)
    }

    pub fn wear(&mut self, amount: f32) {
        self.current_stats.roughness = (self.current_stats.roughness + amount * 0.1).min(1.0);
        self.current_stats.bump_density =
            (self.current_stats.bump_density + amount * 5.0).min(100.0);
        self.current_stats.hardness = (self.current_stats.hardness - amount * 0.05).max(0.1);
    }

    pub fn sharpen(&mut self, amount: f32) {
        self.current_stats.roughness = (self.current_stats.roughness - amount * 0.2).max(0.01);
        self.current_stats.hardness = (self.current_stats.hardness + amount * 0.1).min(10.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surface_stats_creation() {
        let blade = SurfaceStats::metal_blade();
        assert!(blade.roughness < 0.1);
        assert!(blade.hardness > 7.0);
    }

    #[test]
    fn test_micro_contact_generation() {
        let metal = SurfaceStats::metal_blade();
        let iron = SurfaceStats::iron_plate();
        let mut rng = rand::thread_rng();
        let patch = MicroContactPatch::generate(&metal, &iron, Vec3::Y, 0.01, &mut rng);
        assert!(patch.asperity_count > 0);
        assert!(patch.total_area > 0.0);
    }

    #[test]
    fn test_surface_wear() {
        let mut profile = SurfaceProfile::for_body_part(BodyPart::HandL, Uuid::new_v4());
        let orig_roughness = profile.current_stats.roughness;
        profile.wear(0.5);
        assert!(profile.current_stats.roughness > orig_roughness);
    }
}
