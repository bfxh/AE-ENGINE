use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::fixed_point::{FixedPoint, FixedQuat, FixedVec3};
use crate::joints::{Joint, JointSystem, JointType};
use crate::world::{BodyType, RigidBody};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CollisionShape {
    Box,
    Sphere,
    Capsule,
    Cylinder,
    ConvexHull,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagdollBone {
    pub name: String,
    pub body_id: Uuid,
    pub parent_bone: Option<String>,
    pub local_transform: FixedQuat,
    pub local_position: FixedVec3,
    pub mass: FixedPoint,
    pub collision_shape: CollisionShape,
    pub collision_size: FixedVec3,
    pub joint_id: Option<Uuid>,
    pub joint_type: Option<JointType>,
}

impl RagdollBone {
    pub fn new(
        name: &str,
        body_id: Uuid,
        parent_bone: Option<&str>,
        local_position: FixedVec3,
        mass: FixedPoint,
        shape: CollisionShape,
        size: FixedVec3,
    ) -> Self {
        Self {
            name: name.to_string(),
            body_id,
            parent_bone: parent_bone.map(|s| s.to_string()),
            local_transform: FixedQuat::IDENTITY,
            local_position,
            mass,
            collision_shape: shape,
            collision_size: size,
            joint_id: None,
            joint_type: None,
        }
    }

    pub fn with_joint(mut self, joint_type: JointType) -> Self {
        self.joint_type = Some(joint_type);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkeletonBone {
    pub name: String,
    pub parent: Option<String>,
    pub local_transform: FixedQuat,
    pub local_position: FixedVec3,
    pub length: FixedPoint,
    pub mass: FixedPoint,
    pub collision_radius: FixedPoint,
    pub joint_type: JointType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skeleton {
    pub bones: Vec<SkeletonBone>,
    pub root_position: FixedVec3,
}

impl Skeleton {
    pub fn new() -> Self {
        Self { bones: Vec::new(), root_position: FixedVec3::ZERO }
    }

    pub fn humanoid() -> Self {
        let mut skeleton = Self::new();
        skeleton.root_position =
            FixedVec3::new(FixedPoint::ZERO, FixedPoint::from_f32(1.0), FixedPoint::ZERO);

        skeleton.bones = vec![
            SkeletonBone {
                name: "hips".into(),
                parent: None,
                local_transform: FixedQuat::IDENTITY,
                local_position: FixedVec3::new(
                    FixedPoint::ZERO,
                    FixedPoint::from_f32(0.9),
                    FixedPoint::ZERO,
                ),
                length: FixedPoint::from_f32(0.2),
                mass: FixedPoint::from_f32(10.0),
                collision_radius: FixedPoint::from_f32(0.15),
                joint_type: JointType::Ball,
            },
            SkeletonBone {
                name: "spine".into(),
                parent: Some("hips".into()),
                local_transform: FixedQuat::IDENTITY,
                local_position: FixedVec3::new(
                    FixedPoint::ZERO,
                    FixedPoint::from_f32(0.2),
                    FixedPoint::ZERO,
                ),
                length: FixedPoint::from_f32(0.3),
                mass: FixedPoint::from_f32(8.0),
                collision_radius: FixedPoint::from_f32(0.12),
                joint_type: JointType::Ball,
            },
            SkeletonBone {
                name: "chest".into(),
                parent: Some("spine".into()),
                local_transform: FixedQuat::IDENTITY,
                local_position: FixedVec3::new(
                    FixedPoint::ZERO,
                    FixedPoint::from_f32(0.3),
                    FixedPoint::ZERO,
                ),
                length: FixedPoint::from_f32(0.25),
                mass: FixedPoint::from_f32(12.0),
                collision_radius: FixedPoint::from_f32(0.2),
                joint_type: JointType::Ball,
            },
            SkeletonBone {
                name: "head".into(),
                parent: Some("chest".into()),
                local_transform: FixedQuat::IDENTITY,
                local_position: FixedVec3::new(
                    FixedPoint::ZERO,
                    FixedPoint::from_f32(0.25),
                    FixedPoint::ZERO,
                ),
                length: FixedPoint::from_f32(0.2),
                mass: FixedPoint::from_f32(5.0),
                collision_radius: FixedPoint::from_f32(0.12),
                joint_type: JointType::Ball,
            },
            SkeletonBone {
                name: "left_upper_arm".into(),
                parent: Some("chest".into()),
                local_transform: FixedQuat::IDENTITY,
                local_position: FixedVec3::new(
                    FixedPoint::from_f32(-0.25),
                    FixedPoint::from_f32(0.05),
                    FixedPoint::ZERO,
                ),
                length: FixedPoint::from_f32(0.3),
                mass: FixedPoint::from_f32(3.0),
                collision_radius: FixedPoint::from_f32(0.06),
                joint_type: JointType::Ball,
            },
            SkeletonBone {
                name: "left_forearm".into(),
                parent: Some("left_upper_arm".into()),
                local_transform: FixedQuat::IDENTITY,
                local_position: FixedVec3::new(
                    FixedPoint::from_f32(-0.3),
                    FixedPoint::ZERO,
                    FixedPoint::ZERO,
                ),
                length: FixedPoint::from_f32(0.25),
                mass: FixedPoint::from_f32(2.0),
                collision_radius: FixedPoint::from_f32(0.05),
                joint_type: JointType::Hinge,
            },
            SkeletonBone {
                name: "right_upper_arm".into(),
                parent: Some("chest".into()),
                local_transform: FixedQuat::IDENTITY,
                local_position: FixedVec3::new(
                    FixedPoint::from_f32(0.25),
                    FixedPoint::from_f32(0.05),
                    FixedPoint::ZERO,
                ),
                length: FixedPoint::from_f32(0.3),
                mass: FixedPoint::from_f32(3.0),
                collision_radius: FixedPoint::from_f32(0.06),
                joint_type: JointType::Ball,
            },
            SkeletonBone {
                name: "right_forearm".into(),
                parent: Some("right_upper_arm".into()),
                local_transform: FixedQuat::IDENTITY,
                local_position: FixedVec3::new(
                    FixedPoint::from_f32(0.3),
                    FixedPoint::ZERO,
                    FixedPoint::ZERO,
                ),
                length: FixedPoint::from_f32(0.25),
                mass: FixedPoint::from_f32(2.0),
                collision_radius: FixedPoint::from_f32(0.05),
                joint_type: JointType::Hinge,
            },
            SkeletonBone {
                name: "left_thigh".into(),
                parent: Some("hips".into()),
                local_transform: FixedQuat::IDENTITY,
                local_position: FixedVec3::new(
                    FixedPoint::from_f32(-0.1),
                    FixedPoint::from_f32(-0.2),
                    FixedPoint::ZERO,
                ),
                length: FixedPoint::from_f32(0.4),
                mass: FixedPoint::from_f32(6.0),
                collision_radius: FixedPoint::from_f32(0.08),
                joint_type: JointType::Ball,
            },
            SkeletonBone {
                name: "left_shin".into(),
                parent: Some("left_thigh".into()),
                local_transform: FixedQuat::IDENTITY,
                local_position: FixedVec3::new(
                    FixedPoint::ZERO,
                    FixedPoint::from_f32(-0.4),
                    FixedPoint::ZERO,
                ),
                length: FixedPoint::from_f32(0.4),
                mass: FixedPoint::from_f32(4.0),
                collision_radius: FixedPoint::from_f32(0.06),
                joint_type: JointType::Hinge,
            },
            SkeletonBone {
                name: "right_thigh".into(),
                parent: Some("hips".into()),
                local_transform: FixedQuat::IDENTITY,
                local_position: FixedVec3::new(
                    FixedPoint::from_f32(0.1),
                    FixedPoint::from_f32(-0.2),
                    FixedPoint::ZERO,
                ),
                length: FixedPoint::from_f32(0.4),
                mass: FixedPoint::from_f32(6.0),
                collision_radius: FixedPoint::from_f32(0.08),
                joint_type: JointType::Ball,
            },
            SkeletonBone {
                name: "right_shin".into(),
                parent: Some("right_thigh".into()),
                local_transform: FixedQuat::IDENTITY,
                local_position: FixedVec3::new(
                    FixedPoint::ZERO,
                    FixedPoint::from_f32(-0.4),
                    FixedPoint::ZERO,
                ),
                length: FixedPoint::from_f32(0.4),
                mass: FixedPoint::from_f32(4.0),
                collision_radius: FixedPoint::from_f32(0.06),
                joint_type: JointType::Hinge,
            },
        ];

        skeleton
    }
}

impl Default for Skeleton {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ragdoll {
    pub bones: Vec<RagdollBone>,
    pub is_active: bool,
    pub blend_time: FixedPoint,
    pub blend_progress: FixedPoint,
    pub root_body_id: Uuid,
    pub joint_system: JointSystem,
    pub total_mass: FixedPoint,
    pub damping: FixedPoint,
    pub gravity_scale: FixedPoint,
}

impl Ragdoll {
    pub fn new() -> Self {
        Self {
            bones: Vec::new(),
            is_active: false,
            blend_time: FixedPoint::from_f32(0.3),
            blend_progress: FixedPoint::ZERO,
            root_body_id: Uuid::nil(),
            joint_system: JointSystem::new(),
            total_mass: FixedPoint::ZERO,
            damping: FixedPoint::from_f32(0.1),
            gravity_scale: FixedPoint::ONE,
        }
    }

    pub fn create_from_skeleton(&mut self, skeleton: &Skeleton) -> Vec<Uuid> {
        let mut body_ids = Vec::new();
        let mut bone_map: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for bone in &skeleton.bones {
            let body_id = Uuid::new_v4();
            let body = RagdollBone::new(
                &bone.name,
                body_id,
                bone.parent.as_deref(),
                bone.local_position,
                bone.mass,
                CollisionShape::Capsule,
                FixedVec3::new(bone.collision_radius, bone.length, bone.collision_radius),
            )
            .with_joint(bone.joint_type);

            self.bones.push(body);
            self.total_mass += bone.mass;
            body_ids.push(body_id);
            bone_map.insert(bone.name.clone(), self.bones.len() - 1);
        }

        for bone in &skeleton.bones {
            if let Some(parent_name) = &bone.parent {
                if let (Some(&child_idx), Some(&parent_idx)) =
                    (bone_map.get(&bone.name), bone_map.get(parent_name))
                {
                    let child_body_id = self.bones[child_idx].body_id;
                    let parent_body_id = self.bones[parent_idx].body_id;
                    let joint_type = self.bones[child_idx].joint_type.unwrap_or(JointType::Ball);

                    let joint = Joint::new(joint_type, parent_body_id, child_body_id).with_anchors(
                        self.bones[parent_idx].local_position
                            + FixedVec3::new(
                                FixedPoint::ZERO,
                                skeleton.bones[parent_idx].length * FixedPoint::from_f32(0.5),
                                FixedPoint::ZERO,
                            ),
                        self.bones[child_idx].local_position
                            - FixedVec3::new(
                                FixedPoint::ZERO,
                                skeleton
                                    .bones
                                    .iter()
                                    .find(|b| b.name == bone.name)
                                    .map(|b| b.length)
                                    .unwrap_or(FixedPoint::ONE)
                                    * FixedPoint::from_f32(0.5),
                                FixedPoint::ZERO,
                            ),
                    );

                    let joint_id = self.joint_system.add_joint(joint);
                    self.bones[child_idx].joint_id = Some(joint_id);
                }
            }
        }

        if let Some(root_bone) = skeleton.bones.first() {
            if let Some(&root_idx) = bone_map.get(&root_bone.name) {
                self.root_body_id = self.bones[root_idx].body_id;
            }
        }

        body_ids
    }

    pub fn create_bodies(&self) -> Vec<RigidBody> {
        let mut bodies = Vec::new();
        let mut child_to_parent: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        for bone in &self.bones {
            if let Some(parent) = &bone.parent_bone {
                child_to_parent.insert(bone.name.clone(), parent.clone());
            }
        }

        let mut world_positions: std::collections::HashMap<String, FixedVec3> =
            std::collections::HashMap::new();

        for bone in &self.bones {
            let world_pos = if let Some(parent_name) = &bone.parent_bone {
                if let Some(&parent_pos) = world_positions.get(parent_name) {
                    parent_pos + bone.local_position
                } else {
                    bone.local_position
                }
            } else {
                bone.local_position
            };

            world_positions.insert(bone.name.clone(), world_pos);

            let body = RigidBody {
                id: bone.body_id,
                position: world_pos,
                rotation: FixedQuat::IDENTITY,
                velocity: FixedVec3::ZERO,
                angular_velocity: FixedVec3::ZERO,
                mass: bone.mass,
                material: crate::material::MaterialProperties::default(),
                body_type: if self.is_active { BodyType::Dynamic } else { BodyType::Kinematic },
                is_sleeping: false,
                sleep_timer: FixedPoint::ZERO,
                forces: FixedVec3::ZERO,
                torque: FixedVec3::ZERO,
                linear_damping: self.damping,
                angular_damping: self.damping,
                mpss_index: None,
            };

            bodies.push(body);
        }

        bodies
    }

    pub fn activate(&mut self) {
        self.is_active = true;
        self.blend_progress = FixedPoint::ZERO;
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.blend_progress = FixedPoint::ZERO;
    }

    pub fn update(&mut self, dt: FixedPoint) {
        if self.is_active && self.blend_progress < FixedPoint::ONE {
            self.blend_progress += dt / self.blend_time;
            if self.blend_progress > FixedPoint::ONE {
                self.blend_progress = FixedPoint::ONE;
            }
        } else if !self.is_active && self.blend_progress > FixedPoint::ZERO {
            self.blend_progress -= dt / self.blend_time;
            if self.blend_progress < FixedPoint::ZERO {
                self.blend_progress = FixedPoint::ZERO;
            }
        }
    }

    pub fn set_pose(&mut self, bone_poses: &[(String, FixedVec3, FixedQuat)]) {
        for (bone_name, position, rotation) in bone_poses {
            if let Some(bone) = self.bones.iter_mut().find(|b| &b.name == bone_name) {
                bone.local_position = *position;
                bone.local_transform = *rotation;
            }
        }
    }

    pub fn blend_to_animation(
        &mut self,
        animated_positions: &[(String, FixedVec3)],
        animated_rotations: &[(String, FixedQuat)],
    ) -> Vec<(String, FixedVec3, FixedQuat)> {
        let mut blended = Vec::new();

        for bone in &self.bones {
            let anim_pos =
                animated_positions.iter().find(|(n, _)| n == &bone.name).map(|(_, p)| *p);
            let anim_rot =
                animated_rotations.iter().find(|(n, _)| n == &bone.name).map(|(_, r)| *r);

            match (anim_pos, anim_rot) {
                (Some(ap), Some(ar)) => {
                    let t = self.blend_progress;
                    let one_minus_t = FixedPoint::ONE - t;
                    let blended_pos = FixedVec3::new(
                        bone.local_position.x * one_minus_t + ap.x * t,
                        bone.local_position.y * one_minus_t + ap.y * t,
                        bone.local_position.z * one_minus_t + ap.z * t,
                    );
                    let blended_rot = bone.local_transform.slerp(ar, t.to_f32());
                    blended.push((bone.name.clone(), blended_pos, blended_rot));
                },
                (Some(ap), None) => {
                    blended.push((bone.name.clone(), ap, bone.local_transform));
                },
                (None, Some(ar)) => {
                    blended.push((bone.name.clone(), bone.local_position, ar));
                },
                (None, None) => {
                    blended.push((bone.name.clone(), bone.local_position, bone.local_transform));
                },
            }
        }

        blended
    }

    pub fn solve(&mut self, dt: FixedPoint, bodies: &mut [RigidBody]) {
        if !self.is_active {
            return;
        }
        self.joint_system.solve(dt, bodies);
    }

    pub fn bone_count(&self) -> usize {
        self.bones.len()
    }
}

impl Default for Ragdoll {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_from_skeleton() {
        let skeleton = Skeleton::humanoid();
        let mut ragdoll = Ragdoll::new();
        let body_ids = ragdoll.create_from_skeleton(&skeleton);

        assert_eq!(ragdoll.bone_count(), 12);
        assert_eq!(body_ids.len(), 12);
        assert!(ragdoll.total_mass > FixedPoint::ZERO);
    }

    #[test]
    fn test_activate_deactivate() {
        let mut ragdoll = Ragdoll::new();
        assert!(!ragdoll.is_active);

        ragdoll.activate();
        assert!(ragdoll.is_active);

        ragdoll.deactivate();
        assert!(!ragdoll.is_active);
    }

    #[test]
    fn test_blend_progress() {
        let mut ragdoll = Ragdoll::new();
        ragdoll.blend_time = FixedPoint::from_f32(0.3);
        ragdoll.activate();

        ragdoll.update(FixedPoint::from_f32(0.1));
        assert!(ragdoll.blend_progress > FixedPoint::ZERO);
        assert!(ragdoll.blend_progress < FixedPoint::ONE);

        ragdoll.update(FixedPoint::from_f32(0.3));
        assert_eq!(ragdoll.blend_progress, FixedPoint::ONE);
    }

    #[test]
    fn test_set_pose() {
        let mut ragdoll = Ragdoll::new();
        let skeleton = Skeleton::humanoid();
        ragdoll.create_from_skeleton(&skeleton);

        let new_pos =
            FixedVec3::new(FixedPoint::ONE, FixedPoint::from_f32(2.0), FixedPoint::from_f32(3.0));
        let new_rot = FixedQuat::IDENTITY;

        ragdoll.set_pose(&[("head".to_string(), new_pos, new_rot)]);

        let head = ragdoll.bones.iter().find(|b| b.name == "head").unwrap();
        assert_eq!(head.local_position, new_pos);
    }

    #[test]
    fn test_create_bodies() {
        let mut ragdoll = Ragdoll::new();
        let skeleton = Skeleton::humanoid();
        ragdoll.create_from_skeleton(&skeleton);

        let bodies = ragdoll.create_bodies();
        assert_eq!(bodies.len(), 12);

        ragdoll.activate();
        let active_bodies = ragdoll.create_bodies();
        assert_eq!(active_bodies.len(), 12);
        for body in &active_bodies {
            assert_eq!(body.body_type, BodyType::Dynamic);
        }
    }
}
