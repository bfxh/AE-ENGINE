use godot::prelude::*;
use std::sync::Mutex;
use uuid::Uuid;
use ae_character::{
    BodyPart, DeformationHistory, ForceFeedbackBus, Skeleton, SurfaceContactDetector,
};

#[derive(GodotClass)]
#[class(base=Node3D)]
struct WastelandCharacter {
    skeleton: Mutex<Option<Skeleton>>,
    contact_detector: Mutex<SurfaceContactDetector>,
    deformation: Mutex<DeformationHistory>,
    feedback: Mutex<ForceFeedbackBus>,

    #[base]
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for WastelandCharacter {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            skeleton: Mutex::new(None),
            contact_detector: Mutex::new(SurfaceContactDetector::new()),
            deformation: Mutex::new(DeformationHistory::new()),
            feedback: Mutex::new(ForceFeedbackBus::new()),
            base,
        }
    }
}

#[godot_api]
impl WastelandCharacter {
    #[func]
    fn create_humanoid_skeleton(&mut self) {
        if let Ok(mut guard) = self.skeleton.lock() {
            let skel = Skeleton::humanoid();
            if let Ok(mut detector) = self.contact_detector.lock() {
                detector.register_skeleton(&skel);
            }
            *guard = Some(skel);
        }
    }

    #[func]
    fn bone_count(&self) -> i64 {
        if let Ok(guard) = self.skeleton.lock() {
            if let Some(ref s) = *guard {
                return s.bone_count() as i64;
            }
        }
        0
    }

    #[func]
    fn get_bone_names(&self) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        if let Ok(guard) = self.skeleton.lock() {
            if let Some(ref s) = *guard {
                for bone in &s.bones {
                    arr.push(bone.name.as_str());
                }
            }
        }
        arr
    }

    #[func]
    fn get_bone_ids(&self) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        if let Ok(guard) = self.skeleton.lock() {
            if let Some(ref s) = *guard {
                for bone in &s.bones {
                    let id_str = bone.id.to_string();
                    arr.push(id_str.as_str());
                }
            }
        }
        arr
    }

    #[func]
    fn get_bone_position(&self, bone_id: GString) -> Vector3 {
        if let Ok(parsed) = Uuid::parse_str(&bone_id.to_string()) {
            if let Ok(guard) = self.skeleton.lock() {
                if let Some(ref s) = *guard {
                    if let Some((pos, _)) = s.get_world_transform(parsed) {
                        return Vector3::new(pos.x, pos.y, pos.z);
                    }
                }
            }
        }
        Vector3::ZERO
    }

    #[func]
    fn get_bone_endpoints(&self, bone_id: GString) -> Dictionary<Variant, Variant> {
        if let Ok(parsed) = Uuid::parse_str(&bone_id.to_string()) {
            if let Ok(guard) = self.skeleton.lock() {
                if let Some(ref s) = *guard {
                    if let Some((start, end)) = s.get_bone_endpoints_world(parsed) {
                        return dict! {
                            "start_x" => start.x,
                            "start_y" => start.y,
                            "start_z" => start.z,
                            "end_x" => end.x,
                            "end_y" => end.y,
                            "end_z" => end.z,
                        };
                    }
                }
            }
        }
        dict! {}
    }

    #[func]
    fn get_bone_by_part(&self, part: GString) -> GString {
        let bp = match part.to_string().as_str() {
            "head" => BodyPart::Head,
            "torso" => BodyPart::Torso,
            "upper_arm_l" => BodyPart::UpperArmL,
            "upper_arm_r" => BodyPart::UpperArmR,
            "lower_arm_l" => BodyPart::LowerArmL,
            "lower_arm_r" => BodyPart::LowerArmR,
            "hand_l" => BodyPart::HandL,
            "hand_r" => BodyPart::HandR,
            "upper_leg_l" => BodyPart::UpperLegL,
            "upper_leg_r" => BodyPart::UpperLegR,
            "lower_leg_l" => BodyPart::LowerLegL,
            "lower_leg_r" => BodyPart::LowerLegR,
            "foot_l" => BodyPart::FootL,
            "foot_r" => BodyPart::FootR,
            _ => return GString::new(),
        };
        if let Ok(guard) = self.skeleton.lock() {
            if let Some(ref s) = *guard {
                if let Some(bone) = s.get_bone_by_part(bp) {
                    let id_str = bone.id.to_string();
                    return GString::from(id_str.as_str());
                }
            }
        }
        GString::new()
    }

    #[func]
    #[allow(clippy::too_many_arguments)]
    fn record_deformation(
        &mut self,
        px: f32,
        py: f32,
        pz: f32,
        depth: f32,
        timestamp: f32,
        source_bone_id: GString,
        target_bone_id: GString,
        force_magnitude: f32,
        dx: f32,
        dy: f32,
        dz: f32,
        source_material: GString,
    ) {
        if let (Ok(src), Ok(tgt)) = (
            Uuid::parse_str(&source_bone_id.to_string()),
            Uuid::parse_str(&target_bone_id.to_string()),
        ) {
            if let Ok(mut deform) = self.deformation.lock() {
                deform.record_deformation(
                    glam::Vec3::new(px, py, pz),
                    depth,
                    timestamp,
                    src,
                    tgt,
                    force_magnitude,
                    glam::Vec3::new(dx, dy, dz),
                    &source_material.to_string(),
                );
            }
        }
    }

    #[func]
    fn total_deformations(&self) -> i64 {
        if let Ok(deform) = self.deformation.lock() {
            return deform.total_deformations() as i64;
        }
        0
    }

    #[func]
    fn get_cumulative_strain(&self) -> f32 {
        if let Ok(deform) = self.deformation.lock() {
            return deform.cumulative_strain;
        }
        0.0
    }

    #[func]
    fn get_wear_depth(&self) -> f32 {
        if let Ok(deform) = self.deformation.lock() {
            return deform.wear_depth;
        }
        0.0
    }

    #[func]
    #[allow(clippy::too_many_arguments)]
    fn emit_surface_slide(
        &mut self,
        bone_id: GString,
        px: f32,
        py: f32,
        pz: f32,
        vx: f32,
        vy: f32,
        vz: f32,
        friction: f32,
        roughness: f32,
        force_magnitude: f32,
        time: f32,
    ) {
        if let Ok(parsed) = Uuid::parse_str(&bone_id.to_string()) {
            if let Ok(mut fb) = self.feedback.lock() {
                fb.emit_surface_slide(
                    parsed,
                    glam::Vec3::new(px, py, pz),
                    glam::Vec3::new(vx, vy, vz),
                    friction,
                    roughness,
                    force_magnitude,
                    time,
                );
            }
        }
    }

    #[func]
    fn get_feedback_event_count(&self) -> i64 {
        if let Ok(fb) = self.feedback.lock() {
            return fb.events.len() as i64;
        }
        0
    }

    #[func]
    fn is_sliding(&self) -> bool {
        if let Ok(fb) = self.feedback.lock() {
            return fb.active_sliding;
        }
        false
    }

    #[func]
    fn get_sliding_force(&self) -> f32 {
        if let Ok(fb) = self.feedback.lock() {
            return fb.sliding_force;
        }
        0.0
    }

    #[func]
    fn get_sliding_frequency(&self) -> f32 {
        if let Ok(fb) = self.feedback.lock() {
            return fb.sliding_frequency;
        }
        0.0
    }

    #[func]
    fn detect_contact(&self, bone_a_id: GString, bone_b_id: GString) -> bool {
        if let (Ok(a), Ok(b)) =
            (Uuid::parse_str(&bone_a_id.to_string()), Uuid::parse_str(&bone_b_id.to_string()))
        {
            if let Ok(guard) = self.skeleton.lock() {
                if let Some(ref s) = *guard {
                    if let Ok(detector) = self.contact_detector.lock() {
                        let mut rng = rand::thread_rng();
                        return detector.detect_contact(s, a, b, &mut rng).is_some();
                    }
                }
            }
        }
        false
    }
}
