use glam::Vec3;
use rand::Rng;
use std::collections::HashMap;
use uuid::Uuid;
use ae_physics::fixed_point::FixedPoint;
use ae_weave::constraint::{
    ConstraintGroup, ConstraintInput, ConstraintType, SurfaceParams,
};

use crate::skeleton::Skeleton;
use crate::surface_stats::{MicroContactPatch, SurfaceProfile, SurfaceStats};

#[derive(Debug, Clone)]
pub struct SurfaceContactPoint {
    pub position: Vec3,
    pub normal: Vec3,
    pub penetration_depth: f32,
    pub contact_area: f32,
    pub bone_id: Uuid,
    pub target_entity_id: Uuid,
    pub surface_stats: SurfaceStats,
}

#[derive(Debug, Clone)]
pub struct SurfaceConstraintPair {
    pub point_a: SurfaceContactPoint,
    pub point_b: SurfaceContactPoint,
    pub weave_constraint: ConstraintInput,
    pub micro_patch: MicroContactPatch,
}

#[derive(Debug, Clone)]
pub struct SurfaceContactDetector {
    pub profiles: HashMap<Uuid, SurfaceProfile>,
    pub time: f32,
}

impl SurfaceContactDetector {
    pub fn new() -> Self {
        Self { profiles: HashMap::new(), time: 0.0 }
    }

    pub fn register_profile(&mut self, profile: SurfaceProfile) {
        self.profiles.insert(profile.bone_id, profile);
    }

    pub fn register_skeleton(&mut self, skeleton: &Skeleton) {
        for bone in &skeleton.bones {
            let profile = SurfaceProfile::for_body_part(bone.body_part, bone.id);
            self.profiles.insert(bone.id, profile);
        }
    }

    pub fn get_profile(&self, bone_id: Uuid) -> Option<&SurfaceProfile> {
        self.profiles.get(&bone_id)
    }

    pub fn get_profile_mut(&mut self, bone_id: Uuid) -> Option<&mut SurfaceProfile> {
        self.profiles.get_mut(&bone_id)
    }

    pub fn detect_contact(
        &self,
        skeleton: &Skeleton,
        bone_a_id: Uuid,
        bone_b_id: Uuid,
        rng: &mut impl Rng,
    ) -> Option<SurfaceConstraintPair> {
        let bone_a = skeleton.get_bone(bone_a_id)?;
        let bone_b = skeleton.get_bone(bone_b_id)?;

        if bone_a.parent_id == Some(bone_b_id) || bone_b.parent_id == Some(bone_a_id) {
            return None;
        }

        let (start_a, end_a) = skeleton.get_bone_endpoints_world(bone_a_id)?;
        let (start_b, end_b) = skeleton.get_bone_endpoints_world(bone_b_id)?;

        let (closest_a, closest_b) =
            closest_points_between_segments(start_a, end_a, start_b, end_b);
        let distance = closest_a.distance(closest_b);

        let combined_radius = bone_a.radius + bone_b.radius;
        if distance >= combined_radius {
            return None;
        }

        let penetration = combined_radius - distance;
        let normal = if distance > 0.0001 { (closest_b - closest_a).normalize() } else { Vec3::Y };

        let profile_a = self.profiles.get(&bone_a_id)?;
        let profile_b = self.profiles.get(&bone_b_id)?;

        let contact_area = estimate_contact_area(penetration, bone_a.radius, bone_b.radius);
        let micro_patch = MicroContactPatch::generate(
            &profile_a.current_stats,
            &profile_b.current_stats,
            normal,
            contact_area,
            rng,
        );

        let point_a = SurfaceContactPoint {
            position: closest_a,
            normal,
            penetration_depth: penetration,
            contact_area,
            bone_id: bone_a_id,
            target_entity_id: bone_b_id,
            surface_stats: profile_a.current_stats,
        };

        let point_b = SurfaceContactPoint {
            position: closest_b,
            normal: -normal,
            penetration_depth: penetration,
            contact_area,
            bone_id: bone_b_id,
            target_entity_id: bone_a_id,
            surface_stats: profile_b.current_stats,
        };

        let yield_strength =
            profile_a.current_stats.hardness * profile_b.current_stats.hardness * 2.5e7;
        let ultimate_strength = yield_strength * 2.0;
        let combined_friction = (profile_a.current_stats.friction_coefficient
            * profile_b.current_stats.friction_coefficient)
            .sqrt();
        let combined_roughness =
            (profile_a.current_stats.roughness + profile_b.current_stats.roughness) * 0.5;

        let surface_params = SurfaceParams {
            yield_strength: FixedPoint::from_f32(yield_strength),
            ultimate_strength: FixedPoint::from_f32(ultimate_strength),
            hardness: FixedPoint::from_f32(profile_a.current_stats.hardness),
            friction_coefficient: FixedPoint::from_f32(combined_friction),
            surface_roughness: FixedPoint::from_f32(combined_roughness),
            contact_area: FixedPoint::from_f32(contact_area),
            plastic_strain: FixedPoint::ZERO,
        };

        Some(SurfaceConstraintPair {
            point_a,
            point_b,
            weave_constraint: ConstraintInput {
                node_a: ae_weave::constraint::NodeId::default(),
                node_b: ae_weave::constraint::NodeId::default(),
                constraint_type: ConstraintType::Surface,
                rest_length: FixedPoint::from_f32(distance),
                stiffness: FixedPoint::from_f32(1e8),
                compliance: FixedPoint::from_f32(0.0001),
                group: ConstraintGroup::Contact,
                surface_params: Some(surface_params),
            },
            micro_patch,
        })
    }

    pub fn detect_all_contacts(
        &self,
        skeleton: &Skeleton,
        bone_ids: &[Uuid],
        rng: &mut impl Rng,
    ) -> Vec<SurfaceConstraintPair> {
        let mut contacts = Vec::new();
        for i in 0..bone_ids.len() {
            for j in (i + 1)..bone_ids.len() {
                if let Some(contact) = self.detect_contact(skeleton, bone_ids[i], bone_ids[j], rng)
                {
                    contacts.push(contact);
                }
            }
        }
        contacts
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
    }
}

impl Default for SurfaceContactDetector {
    fn default() -> Self {
        Self::new()
    }
}

fn closest_points_between_segments(
    a_start: Vec3,
    a_end: Vec3,
    b_start: Vec3,
    b_end: Vec3,
) -> (Vec3, Vec3) {
    let d1 = a_end - a_start;
    let d2 = b_end - b_start;
    let r = a_start - b_start;

    let a = d1.dot(d1);
    let e = d2.dot(d2);
    let f = d2.dot(r);

    let mut s = 0.0;
    let mut t;

    if a <= 1e-10 && e <= 1e-10 {
        return (a_start, b_start);
    }

    if a <= 1e-10 {
        t = (f / e).clamp(0.0, 1.0);
    } else {
        let c = d1.dot(r);
        if e <= 1e-10 {
            s = (-c / a).clamp(0.0, 1.0);
            t = 0.0;
        } else {
            let b = d1.dot(d2);
            let denom = a * e - b * b;

            if denom.abs() > 1e-10 {
                let sn = (b * f - c * e) / denom;
                s = sn.clamp(0.0, 1.0);
                t = (b * sn + f) / e;
                if t < 0.0 {
                    t = 0.0;
                    s = (-c / a).clamp(0.0, 1.0);
                } else if t > 1.0 {
                    t = 1.0;
                    s = ((b - c) / a).clamp(0.0, 1.0);
                }
            } else {
                s = 0.0;
                t = (f / e).clamp(0.0, 1.0);
            }
        }
    }

    let closest_a = a_start + d1 * s;
    let closest_b = b_start + d2 * t;
    (closest_a, closest_b)
}

fn estimate_contact_area(penetration: f32, radius_a: f32, radius_b: f32) -> f32 {
    let effective_radius = (radius_a * radius_b) / (radius_a + radius_b).max(0.0001);
    let chord_length =
        2.0 * (2.0 * effective_radius * penetration - penetration * penetration).max(0.0).sqrt();
    let contact_width = chord_length.min(effective_radius * 2.0);
    std::f32::consts::PI * contact_width * contact_width * 0.25
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skeleton::BodyPart;
    use crate::skeleton::Skeleton;

    #[test]
    fn test_contact_detection_no_contact() {
        let skeleton = Skeleton::humanoid();
        let detector = SurfaceContactDetector::new();
        let torso = skeleton.get_bone_by_part(BodyPart::Torso).unwrap();
        let head = skeleton.get_bone_by_part(BodyPart::Head).unwrap();
        let mut rng = rand::thread_rng();
        let result = detector.detect_contact(&skeleton, torso.id, head.id, &mut rng);
        assert!(result.is_none());
    }

    #[test]
    fn test_contact_detection_self_filter() {
        let skeleton = Skeleton::humanoid();
        let detector = SurfaceContactDetector::new();
        let torso = skeleton.get_bone_by_part(BodyPart::Torso).unwrap();
        let upper_arm = skeleton.get_bone_by_part(BodyPart::UpperArmL).unwrap();
        let mut rng = rand::thread_rng();
        let result = detector.detect_contact(&skeleton, torso.id, upper_arm.id, &mut rng);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_all_contacts() {
        let skeleton = Skeleton::humanoid();
        let mut detector = SurfaceContactDetector::new();
        detector.register_skeleton(&skeleton);
        let bone_ids: Vec<Uuid> = skeleton.bones.iter().map(|b| b.id).collect();
        let mut rng = rand::thread_rng();
        let _contacts = detector.detect_all_contacts(&skeleton, &bone_ids, &mut rng);
    }

    #[test]
    fn test_contact_area_estimation() {
        let area = estimate_contact_area(0.01, 0.05, 0.05);
        assert!(area > 0.0);
        assert!(area < 0.01);
    }
}
