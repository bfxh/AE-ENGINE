use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::fixed_point::{FixedPoint, FixedVec3};
use crate::world::RigidBody;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JointType {
    Ball,
    Hinge,
    Prismatic,
    Fixed,
    Cylindrical,
    Universal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct JointLimits {
    pub min_angle: FixedPoint,
    pub max_angle: FixedPoint,
    pub min_linear: FixedPoint,
    pub max_linear: FixedPoint,
    pub enabled: bool,
}

impl Default for JointLimits {
    fn default() -> Self {
        Self {
            min_angle: FixedPoint::from_f32(-std::f32::consts::PI),
            max_angle: FixedPoint::from_f32(std::f32::consts::PI),
            min_linear: FixedPoint::from_f32(-1.0),
            max_linear: FixedPoint::from_f32(1.0),
            enabled: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct JointMotor {
    pub speed: FixedPoint,
    pub max_torque: FixedPoint,
    pub enabled: bool,
    pub target_angle: FixedPoint,
}

impl Default for JointMotor {
    fn default() -> Self {
        Self {
            speed: FixedPoint::ZERO,
            max_torque: FixedPoint::from_f32(100.0),
            enabled: false,
            target_angle: FixedPoint::ZERO,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct JointSpring {
    pub stiffness: FixedPoint,
    pub damping: FixedPoint,
    pub rest_angle: FixedPoint,
    pub enabled: bool,
}

impl Default for JointSpring {
    fn default() -> Self {
        Self {
            stiffness: FixedPoint::from_f32(100.0),
            damping: FixedPoint::from_f32(10.0),
            rest_angle: FixedPoint::ZERO,
            enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Joint {
    pub id: Uuid,
    pub joint_type: JointType,
    pub body_a: Uuid,
    pub body_b: Uuid,
    pub local_anchor_a: FixedVec3,
    pub local_anchor_b: FixedVec3,
    pub local_axis_a: FixedVec3,
    pub local_axis_b: FixedVec3,
    pub limits: JointLimits,
    pub motor: JointMotor,
    pub spring: JointSpring,
    pub current_angle: FixedPoint,
    pub current_linear: FixedPoint,
    pub active: bool,
    pub break_force: FixedPoint,
    pub broken: bool,
}

impl Joint {
    pub fn new(joint_type: JointType, body_a: Uuid, body_b: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            joint_type,
            body_a,
            body_b,
            local_anchor_a: FixedVec3::ZERO,
            local_anchor_b: FixedVec3::ZERO,
            local_axis_a: FixedVec3::new(FixedPoint::ZERO, FixedPoint::ONE, FixedPoint::ZERO),
            local_axis_b: FixedVec3::new(FixedPoint::ZERO, FixedPoint::ONE, FixedPoint::ZERO),
            limits: JointLimits::default(),
            motor: JointMotor::default(),
            spring: JointSpring::default(),
            current_angle: FixedPoint::ZERO,
            current_linear: FixedPoint::ZERO,
            active: true,
            break_force: FixedPoint::from_f32(10000.0),
            broken: false,
        }
    }

    pub fn with_anchors(mut self, anchor_a: FixedVec3, anchor_b: FixedVec3) -> Self {
        self.local_anchor_a = anchor_a;
        self.local_anchor_b = anchor_b;
        self
    }

    pub fn with_axis(mut self, axis_a: FixedVec3, axis_b: FixedVec3) -> Self {
        self.local_axis_a = axis_a;
        self.local_axis_b = axis_b;
        self
    }

    pub fn with_limits(mut self, limits: JointLimits) -> Self {
        self.limits = limits;
        self
    }

    pub fn with_motor(mut self, motor: JointMotor) -> Self {
        self.motor = motor;
        self
    }

    pub fn with_spring(mut self, spring: JointSpring) -> Self {
        self.spring = spring;
        self
    }

    pub fn with_break_force(mut self, force: FixedPoint) -> Self {
        self.break_force = force;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JointState {
    pub joint_id: Uuid,
    pub angle: FixedPoint,
    pub linear_displacement: FixedPoint,
    pub angular_velocity: FixedPoint,
    pub linear_velocity: FixedPoint,
    pub force: FixedVec3,
    pub torque: FixedVec3,
    pub broken: bool,
}

impl Default for JointState {
    fn default() -> Self {
        Self {
            joint_id: Uuid::nil(),
            angle: FixedPoint::ZERO,
            linear_displacement: FixedPoint::ZERO,
            angular_velocity: FixedPoint::ZERO,
            linear_velocity: FixedPoint::ZERO,
            force: FixedVec3::ZERO,
            torque: FixedVec3::ZERO,
            broken: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JointSystem {
    pub joints: Vec<Joint>,
}

impl JointSystem {
    pub fn new() -> Self {
        Self { joints: Vec::new() }
    }

    pub fn add_joint(&mut self, joint: Joint) -> Uuid {
        let id = joint.id;
        self.joints.push(joint);
        id
    }

    pub fn remove_joint(&mut self, id: Uuid) -> bool {
        let len_before = self.joints.len();
        self.joints.retain(|j| j.id != id);
        self.joints.len() < len_before
    }

    pub fn get_joint(&self, id: Uuid) -> Option<&Joint> {
        self.joints.iter().find(|j| j.id == id)
    }

    pub fn get_joint_mut(&mut self, id: Uuid) -> Option<&mut Joint> {
        self.joints.iter_mut().find(|j| j.id == id)
    }

    pub fn get_joint_state(&self, id: Uuid, bodies: &[RigidBody]) -> Option<JointState> {
        let joint = self.get_joint(id)?;
        if joint.broken {
            return Some(JointState { joint_id: id, broken: true, ..Default::default() });
        }

        let body_a = bodies.iter().find(|b| b.id == joint.body_a)?;
        let body_b = bodies.iter().find(|b| b.id == joint.body_b)?;

        let world_anchor_a = body_a.position + body_a.rotation.rotate_vec3(joint.local_anchor_a);
        let world_anchor_b = body_b.position + body_b.rotation.rotate_vec3(joint.local_anchor_b);
        let world_axis_a = body_a.rotation.rotate_vec3(joint.local_axis_a);
        let world_axis_b = body_b.rotation.rotate_vec3(joint.local_axis_b);

        let angle = {
            let dot = world_axis_a.dot(world_axis_b);
            let clamped = if dot > FixedPoint::ONE {
                FixedPoint::ONE
            } else if dot < FixedPoint::NEG_ONE {
                FixedPoint::NEG_ONE
            } else {
                dot
            };
            let raw = clamped.to_f64().acos();
            FixedPoint::from_f64(raw)
        };

        let linear_displacement = (world_anchor_b - world_anchor_a).length();

        let rel_vel = body_b.velocity - body_a.velocity;
        let angular_vel = body_b.angular_velocity - body_a.angular_velocity;

        Some(JointState {
            joint_id: id,
            angle,
            linear_displacement,
            angular_velocity: angular_vel.length(),
            linear_velocity: rel_vel.length(),
            force: FixedVec3::ZERO,
            torque: FixedVec3::ZERO,
            broken: false,
        })
    }

    pub fn set_motor_speed(&mut self, id: Uuid, speed: FixedPoint) -> bool {
        if let Some(joint) = self.get_joint_mut(id) {
            joint.motor.speed = speed;
            joint.motor.enabled = true;
            true
        } else {
            false
        }
    }

    pub fn set_motor_target(&mut self, id: Uuid, target: FixedPoint) -> bool {
        if let Some(joint) = self.get_joint_mut(id) {
            joint.motor.target_angle = target;
            true
        } else {
            false
        }
    }

    pub fn set_limits(&mut self, id: Uuid, limits: JointLimits) -> bool {
        if let Some(joint) = self.get_joint_mut(id) {
            joint.limits = limits;
            true
        } else {
            false
        }
    }

    pub fn set_spring(&mut self, id: Uuid, spring: JointSpring) -> bool {
        if let Some(joint) = self.get_joint_mut(id) {
            joint.spring = spring;
            true
        } else {
            false
        }
    }

    pub fn enable_motor(&mut self, id: Uuid, enabled: bool) -> bool {
        if let Some(joint) = self.get_joint_mut(id) {
            joint.motor.enabled = enabled;
            true
        } else {
            false
        }
    }

    pub fn solve(&mut self, dt: FixedPoint, bodies: &mut [RigidBody]) {
        for joint in &mut self.joints {
            if !joint.active || joint.broken {
                continue;
            }

            let idx_a = bodies.iter().position(|b| b.id == joint.body_a);
            let idx_b = bodies.iter().position(|b| b.id == joint.body_b);

            let (idx_a, idx_b) = match (idx_a, idx_b) {
                (Some(a), Some(b)) => (a, b),
                _ => continue,
            };

            if idx_a == idx_b {
                continue;
            }

            let (body_a, body_b) = unsafe {
                let ptr = bodies.as_mut_ptr();
                (&mut *ptr.add(idx_a), &mut *ptr.add(idx_b))
            };

            let world_anchor_a =
                body_a.position + body_a.rotation.rotate_vec3(joint.local_anchor_a);
            let world_anchor_b =
                body_b.position + body_b.rotation.rotate_vec3(joint.local_anchor_b);
            let world_axis_a = body_a.rotation.rotate_vec3(joint.local_axis_a);

            let delta = world_anchor_b - world_anchor_a;

            match joint.joint_type {
                JointType::Ball => {
                    let inv_mass_a = Self::inv_mass(body_a);
                    let inv_mass_b = Self::inv_mass(body_b);
                    let total_inv_mass = inv_mass_a + inv_mass_b;
                    if total_inv_mass.raw > 0 {
                        let correction = delta * FixedPoint::from_f32(0.8);
                        if body_a.body_type != crate::world::BodyType::Static {
                            body_a.position += correction * (inv_mass_a / total_inv_mass);
                        }
                        if body_b.body_type != crate::world::BodyType::Static {
                            body_b.position -= correction * (inv_mass_b / total_inv_mass);
                        }
                    }
                },
                JointType::Hinge => {
                    let perp = delta - world_axis_a * delta.dot(world_axis_a);
                    let inv_mass_a = Self::inv_mass(body_a);
                    let inv_mass_b = Self::inv_mass(body_b);
                    let total_inv_mass = inv_mass_a + inv_mass_b;
                    if total_inv_mass.raw > 0 {
                        let correction = perp * FixedPoint::from_f32(0.8);
                        if body_a.body_type != crate::world::BodyType::Static {
                            body_a.position += correction * (inv_mass_a / total_inv_mass);
                        }
                        if body_b.body_type != crate::world::BodyType::Static {
                            body_b.position -= correction * (inv_mass_b / total_inv_mass);
                        }
                    }
                },
                JointType::Fixed => {
                    let inv_mass_a = Self::inv_mass(body_a);
                    let inv_mass_b = Self::inv_mass(body_b);
                    let total_inv_mass = inv_mass_a + inv_mass_b;
                    if total_inv_mass.raw > 0 {
                        let correction = delta;
                        if body_a.body_type != crate::world::BodyType::Static {
                            body_a.position += correction * (inv_mass_a / total_inv_mass);
                        }
                        if body_b.body_type != crate::world::BodyType::Static {
                            body_b.position -= correction * (inv_mass_b / total_inv_mass);
                        }
                    }
                },
                JointType::Prismatic => {
                    let parallel = world_axis_a * delta.dot(world_axis_a);
                    let inv_mass_a = Self::inv_mass(body_a);
                    let inv_mass_b = Self::inv_mass(body_b);
                    let total_inv_mass = inv_mass_a + inv_mass_b;
                    if total_inv_mass.raw > 0 {
                        let correction = parallel * FixedPoint::from_f32(0.2);
                        if body_a.body_type != crate::world::BodyType::Static {
                            body_a.position += correction * (inv_mass_a / total_inv_mass);
                        }
                        if body_b.body_type != crate::world::BodyType::Static {
                            body_b.position -= correction * (inv_mass_b / total_inv_mass);
                        }
                    }
                },
                JointType::Cylindrical => {
                    let perp = delta - world_axis_a * delta.dot(world_axis_a);
                    let inv_mass_a = Self::inv_mass(body_a);
                    let inv_mass_b = Self::inv_mass(body_b);
                    let total_inv_mass = inv_mass_a + inv_mass_b;
                    if total_inv_mass.raw > 0 {
                        let correction = perp * FixedPoint::from_f32(0.8);
                        if body_a.body_type != crate::world::BodyType::Static {
                            body_a.position += correction * (inv_mass_a / total_inv_mass);
                        }
                        if body_b.body_type != crate::world::BodyType::Static {
                            body_b.position -= correction * (inv_mass_b / total_inv_mass);
                        }
                    }
                },
                JointType::Universal => {
                    let inv_mass_a = Self::inv_mass(body_a);
                    let inv_mass_b = Self::inv_mass(body_b);
                    let total_inv_mass = inv_mass_a + inv_mass_b;
                    if total_inv_mass.raw > 0 {
                        let correction = delta * FixedPoint::from_f32(0.6);
                        if body_a.body_type != crate::world::BodyType::Static {
                            body_a.position += correction * (inv_mass_a / total_inv_mass);
                        }
                        if body_b.body_type != crate::world::BodyType::Static {
                            body_b.position -= correction * (inv_mass_b / total_inv_mass);
                        }
                    }
                },
            }

            if joint.spring.enabled {
                let world_axis_b = body_b.rotation.rotate_vec3(joint.local_axis_b);
                let dot = world_axis_a.dot(world_axis_b);
                let clamped = if dot > FixedPoint::ONE {
                    FixedPoint::ONE
                } else if dot < FixedPoint::NEG_ONE {
                    FixedPoint::NEG_ONE
                } else {
                    dot
                };
                let current_angle = FixedPoint::from_f64(clamped.to_f64().acos());
                joint.current_angle = current_angle;

                let angle_error = current_angle - joint.spring.rest_angle;
                let spring_torque = angle_error * joint.spring.stiffness;

                let rel_ang_vel = body_b.angular_velocity - body_a.angular_velocity;
                let damping_torque = rel_ang_vel.length() * joint.spring.damping;

                let total_torque = (spring_torque + damping_torque).abs();
                if total_torque > joint.break_force {
                    joint.broken = true;
                }
            }

            if joint.motor.enabled {
                let angle_error = joint.motor.target_angle - joint.current_angle;
                let motor_torque = angle_error * joint.motor.speed.abs();
                let clamped_torque = if motor_torque > joint.motor.max_torque {
                    joint.motor.max_torque
                } else if motor_torque < -joint.motor.max_torque {
                    -joint.motor.max_torque
                } else {
                    motor_torque
                };

                let inv_mass_a = Self::inv_mass(body_a);
                let inv_mass_b = Self::inv_mass(body_b);
                let total_inv_mass = inv_mass_a + inv_mass_b;
                if total_inv_mass.raw > 0 {
                    let axis = world_axis_a;
                    let impulse = axis * clamped_torque * dt;
                    if body_a.body_type != crate::world::BodyType::Static {
                        body_a.angular_velocity -= impulse * (inv_mass_a / total_inv_mass);
                    }
                    if body_b.body_type != crate::world::BodyType::Static {
                        body_b.angular_velocity += impulse * (inv_mass_b / total_inv_mass);
                    }
                }
            }
        }
    }

    fn inv_mass(body: &RigidBody) -> FixedPoint {
        if body.body_type == crate::world::BodyType::Static {
            FixedPoint::ZERO
        } else {
            FixedPoint::ONE / body.mass
        }
    }

    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }

    pub fn active_count(&self) -> usize {
        self.joints.iter().filter(|j| j.active && !j.broken).count()
    }
}

impl Default for JointSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed_point::FixedQuat;
    use crate::material::MaterialProperties;
    use crate::world::BodyType;

    fn make_test_body(id: Uuid, pos: FixedVec3, mass: FixedPoint) -> RigidBody {
        RigidBody {
            id,
            position: pos,
            rotation: FixedQuat::IDENTITY,
            velocity: FixedVec3::ZERO,
            angular_velocity: FixedVec3::ZERO,
            mass,
            material: MaterialProperties::default(),
            body_type: if mass.raw > 0 { BodyType::Dynamic } else { BodyType::Static },
            is_sleeping: false,
            sleep_timer: FixedPoint::ZERO,
            forces: FixedVec3::ZERO,
            torque: FixedVec3::ZERO,
            linear_damping: FixedPoint::from_f32(0.01),
            angular_damping: FixedPoint::from_f32(0.01),
            mpss_index: None,
        }
    }

    #[test]
    fn test_add_remove_joint() {
        let mut system = JointSystem::new();
        let joint = Joint::new(JointType::Ball, Uuid::new_v4(), Uuid::new_v4());
        let id = system.add_joint(joint);
        assert_eq!(system.joint_count(), 1);
        assert!(system.get_joint(id).is_some());
        assert!(system.remove_joint(id));
        assert_eq!(system.joint_count(), 0);
    }

    #[test]
    fn test_motor_control() {
        let mut system = JointSystem::new();
        let joint = Joint::new(JointType::Hinge, Uuid::new_v4(), Uuid::new_v4());
        let id = system.add_joint(joint);

        assert!(system.set_motor_speed(id, FixedPoint::from_f32(10.0)));
        assert!(system.set_motor_target(id, FixedPoint::from_f32(1.57)));
        let j = system.get_joint(id).unwrap();
        assert!(j.motor.enabled);
        assert_eq!(j.motor.speed, FixedPoint::from_f32(10.0));
    }

    #[test]
    fn test_joint_state() {
        let mut system = JointSystem::new();
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let joint =
            Joint::new(JointType::Ball, id_a, id_b).with_anchors(FixedVec3::ZERO, FixedVec3::ZERO);
        let id = system.add_joint(joint);

        let bodies = vec![
            make_test_body(id_a, FixedVec3::ZERO, FixedPoint::ONE),
            make_test_body(
                id_b,
                FixedVec3::new(FixedPoint::ONE, FixedPoint::ZERO, FixedPoint::ZERO),
                FixedPoint::ONE,
            ),
        ];

        let state = system.get_joint_state(id, &bodies);
        assert!(state.is_some());
        assert!(!state.unwrap().broken);
    }

    #[test]
    fn test_spring_enable() {
        let mut system = JointSystem::new();
        let joint =
            Joint::new(JointType::Hinge, Uuid::new_v4(), Uuid::new_v4()).with_spring(JointSpring {
                stiffness: FixedPoint::from_f32(50.0),
                damping: FixedPoint::from_f32(5.0),
                rest_angle: FixedPoint::ZERO,
                enabled: true,
            });
        let id = system.add_joint(joint);

        let j = system.get_joint(id).unwrap();
        assert!(j.spring.enabled);
        assert_eq!(j.spring.stiffness, FixedPoint::from_f32(50.0));
    }
}
