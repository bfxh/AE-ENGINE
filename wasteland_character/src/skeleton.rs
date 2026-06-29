use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum JointType {
    Fixed,
    Hinge { axis: Vec3, limit_min: f32, limit_max: f32 },
    Ball { swing_limit: f32 },
    Universal { axis1: Vec3, axis2: Vec3 },
    Slider { axis: Vec3, limit_min: f32, limit_max: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Joint {
    pub joint_type: JointType,
    pub stiffness: f32,
    pub damping: f32,
    pub friction: f32,
}

impl Default for Joint {
    fn default() -> Self {
        Self { joint_type: JointType::Fixed, stiffness: 100.0, damping: 10.0, friction: 0.5 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BodyPart {
    Head,
    Torso,
    UpperArmL,
    UpperArmR,
    LowerArmL,
    LowerArmR,
    HandL,
    HandR,
    UpperLegL,
    UpperLegR,
    LowerLegL,
    LowerLegR,
    FootL,
    FootR,
    Tail,
    WingL,
    WingR,
    Custom(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bone {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub children: Vec<Uuid>,
    pub body_part: BodyPart,
    pub local_position: Vec3,
    pub local_rotation: Quat,
    pub length: f32,
    pub radius: f32,
    pub joint: Joint,
    pub mass: f32,
    pub is_critical: bool,
}

impl Bone {
    pub fn new(
        name: &str,
        parent_id: Option<Uuid>,
        body_part: BodyPart,
        local_position: Vec3,
        length: f32,
        radius: f32,
        joint: Joint,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            parent_id,
            children: Vec::new(),
            body_part,
            local_position,
            local_rotation: Quat::IDENTITY,
            length,
            radius,
            joint,
            mass: length * radius * radius * std::f32::consts::PI * 1000.0,
            is_critical: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
    pub root_bone_id: Uuid,
    pub total_mass: f32,
    pub height: f32,
}

impl Skeleton {
    pub fn new() -> Self {
        Self { bones: Vec::new(), root_bone_id: Uuid::nil(), total_mass: 0.0, height: 0.0 }
    }

    pub fn add_bone(&mut self, bone: Bone) -> Uuid {
        let id = bone.id;
        if self.bones.is_empty() {
            self.root_bone_id = id;
        }
        if let Some(parent_id) = bone.parent_id {
            if let Some(parent) = self.bones.iter_mut().find(|b| b.id == parent_id) {
                parent.children.push(id);
            }
        }
        self.total_mass += bone.mass;
        self.bones.push(bone);
        self.recalculate_height();
        id
    }

    fn recalculate_height(&mut self) {
        self.height = self
            .bones
            .iter()
            .filter(|b| {
                b.parent_id.is_none()
                    || self
                        .bones
                        .iter()
                        .any(|p| p.id == b.parent_id.unwrap() && p.body_part == BodyPart::Torso)
            })
            .map(|b| b.length)
            .sum();
    }

    pub fn get_bone(&self, id: Uuid) -> Option<&Bone> {
        self.bones.iter().find(|b| b.id == id)
    }

    pub fn get_bone_mut(&mut self, id: Uuid) -> Option<&mut Bone> {
        self.bones.iter_mut().find(|b| b.id == id)
    }

    pub fn get_bone_by_part(&self, part: BodyPart) -> Option<&Bone> {
        self.bones.iter().find(|b| b.body_part == part)
    }

    pub fn bone_count(&self) -> usize {
        self.bones.len()
    }

    pub fn get_world_transform(&self, bone_id: Uuid) -> Option<(Vec3, Quat)> {
        let bone = self.get_bone(bone_id)?;
        let mut pos = bone.local_position;
        let mut rot = bone.local_rotation;
        let mut current = bone.parent_id;

        while let Some(parent_id) = current {
            let parent = self.get_bone(parent_id)?;
            pos = parent.local_rotation.mul_vec3(pos) + parent.local_position;
            rot = parent.local_rotation * rot;
            current = parent.parent_id;
        }
        Some((pos, rot))
    }

    pub fn get_bone_endpoints_world(&self, bone_id: Uuid) -> Option<(Vec3, Vec3)> {
        let (start, rot) = self.get_world_transform(bone_id)?;
        let bone = self.get_bone(bone_id)?;
        let end = start + rot.mul_vec3(Vec3::new(0.0, bone.length, 0.0));
        Some((start, end))
    }

    pub fn humanoid() -> Self {
        let mut skeleton = Self::new();
        let fixed = Joint::default();

        let torso = Bone::new("torso", None, BodyPart::Torso, Vec3::ZERO, 0.6, 0.15, fixed);
        let torso_id = skeleton.add_bone(torso);

        let head = Bone::new(
            "head",
            Some(torso_id),
            BodyPart::Head,
            Vec3::new(0.0, 0.6, 0.0),
            0.25,
            0.1,
            Joint {
                joint_type: JointType::Hinge { axis: Vec3::Z, limit_min: -1.2, limit_max: 0.8 },
                stiffness: 80.0,
                damping: 15.0,
                friction: 0.3,
            },
        );
        skeleton.add_bone(head);

        let shoulder_joint = Joint {
            joint_type: JointType::Ball { swing_limit: 2.5 },
            stiffness: 60.0,
            damping: 12.0,
            friction: 0.2,
        };

        let upper_arm_l = Bone::new(
            "upper_arm_l",
            Some(torso_id),
            BodyPart::UpperArmL,
            Vec3::new(-0.2, 0.5, 0.0),
            0.35,
            0.05,
            shoulder_joint.clone(),
        );
        skeleton.add_bone(upper_arm_l);

        let upper_arm_r = Bone::new(
            "upper_arm_r",
            Some(torso_id),
            BodyPart::UpperArmR,
            Vec3::new(0.2, 0.5, 0.0),
            0.35,
            0.05,
            shoulder_joint,
        );
        skeleton.add_bone(upper_arm_r);

        let elbow_joint = Joint {
            joint_type: JointType::Hinge { axis: Vec3::X, limit_min: 0.0, limit_max: 2.6 },
            stiffness: 70.0,
            damping: 10.0,
            friction: 0.15,
        };

        let upper_arm_l_id = skeleton.get_bone_by_part(BodyPart::UpperArmL).unwrap().id;
        let lower_arm_l = Bone::new(
            "lower_arm_l",
            Some(upper_arm_l_id),
            BodyPart::LowerArmL,
            Vec3::new(0.0, 0.35, 0.0),
            0.3,
            0.04,
            elbow_joint.clone(),
        );
        skeleton.add_bone(lower_arm_l);

        let upper_arm_r_id = skeleton.get_bone_by_part(BodyPart::UpperArmR).unwrap().id;
        let lower_arm_r = Bone::new(
            "lower_arm_r",
            Some(upper_arm_r_id),
            BodyPart::LowerArmR,
            Vec3::new(0.0, 0.35, 0.0),
            0.3,
            0.04,
            elbow_joint,
        );
        skeleton.add_bone(lower_arm_r);

        let hip_joint = Joint {
            joint_type: JointType::Ball { swing_limit: 1.8 },
            stiffness: 100.0,
            damping: 15.0,
            friction: 0.3,
        };

        let upper_leg_l = Bone::new(
            "upper_leg_l",
            Some(torso_id),
            BodyPart::UpperLegL,
            Vec3::new(-0.08, -0.05, 0.0),
            0.5,
            0.06,
            hip_joint.clone(),
        );
        skeleton.add_bone(upper_leg_l);

        let upper_leg_r = Bone::new(
            "upper_leg_r",
            Some(torso_id),
            BodyPart::UpperLegR,
            Vec3::new(0.08, -0.05, 0.0),
            0.5,
            0.06,
            hip_joint,
        );
        skeleton.add_bone(upper_leg_r);

        let knee_joint = Joint {
            joint_type: JointType::Hinge { axis: Vec3::X, limit_min: -2.4, limit_max: 0.0 },
            stiffness: 90.0,
            damping: 12.0,
            friction: 0.2,
        };

        let upper_leg_l_id = skeleton.get_bone_by_part(BodyPart::UpperLegL).unwrap().id;
        let lower_leg_l = Bone::new(
            "lower_leg_l",
            Some(upper_leg_l_id),
            BodyPart::LowerLegL,
            Vec3::new(0.0, -0.5, 0.0),
            0.45,
            0.05,
            knee_joint.clone(),
        );
        skeleton.add_bone(lower_leg_l);

        let upper_leg_r_id = skeleton.get_bone_by_part(BodyPart::UpperLegR).unwrap().id;
        let lower_leg_r = Bone::new(
            "lower_leg_r",
            Some(upper_leg_r_id),
            BodyPart::LowerLegR,
            Vec3::new(0.0, -0.5, 0.0),
            0.45,
            0.05,
            knee_joint,
        );
        skeleton.add_bone(lower_leg_r);

        skeleton
    }
}

impl Default for Skeleton {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skeleton_creation() {
        let skeleton = Skeleton::humanoid();
        assert!(skeleton.bone_count() > 5);
        assert!(skeleton.get_bone_by_part(BodyPart::Torso).is_some());
        assert!(skeleton.get_bone_by_part(BodyPart::Head).is_some());
    }

    #[test]
    fn test_world_transform() {
        let skeleton = Skeleton::humanoid();
        let torso = skeleton.get_bone_by_part(BodyPart::Torso).unwrap();
        let (pos, _) = skeleton.get_world_transform(torso.id).unwrap();
        assert!((pos - Vec3::ZERO).length() < 0.01);
    }

    #[test]
    fn test_bone_endpoints() {
        let skeleton = Skeleton::humanoid();
        let torso = skeleton.get_bone_by_part(BodyPart::Torso).unwrap();
        let (start, end) = skeleton.get_bone_endpoints_world(torso.id).unwrap();
        assert!(start.distance(end) > 0.0);
    }
}
