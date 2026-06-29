use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::fixed_point::{FixedPoint, FixedQuat, FixedVec3};
use crate::material::MaterialProperties;

pub const FIXED_TIMESTEP: FixedPoint = FixedPoint::from_raw(71582788);
pub const FIXED_TIMESTEP_F64: f64 = 1.0 / 60.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicsBackendType {
    Jolt,
    Rapier,
    XPBD,
    IPC,
    Custom,
}

pub trait PhysicsTrait {
    fn step(&mut self, dt: FixedPoint);
    fn apply_force(&mut self, body_id: Uuid, force: FixedVec3);
    fn apply_impulse(&mut self, body_id: Uuid, impulse: FixedVec3, at_point: FixedVec3);
    fn apply_torque(&mut self, body_id: Uuid, torque: FixedVec3);
    fn add_rigid_body(&mut self, body: FixedRigidBody) -> Uuid;
    fn remove_rigid_body(&mut self, body_id: Uuid);
    fn get_rigid_body(&self, body_id: Uuid) -> Option<&FixedRigidBody>;
    fn get_rigid_body_mut(&mut self, body_id: Uuid) -> Option<&mut FixedRigidBody>;
    fn rigid_body_count(&self) -> usize;
    fn get_collision_events(&self) -> &[FixedCollisionEvent];
    fn drain_collision_events(&mut self) -> Vec<FixedCollisionEvent>;
    fn set_gravity(&mut self, gravity: FixedVec3);
    fn set_paused(&mut self, paused: bool);
    fn set_time_scale(&mut self, scale: FixedPoint);
    fn backend_type(&self) -> PhysicsBackendType;
    fn serialize_state(&self) -> Vec<u8>;
    fn deserialize_state(&mut self, data: &[u8]) -> bool;
    fn world_time(&self) -> f64;
    fn step_count(&self) -> u64;
    fn reset(&mut self);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedRigidBody {
    pub id: Uuid,
    pub position: FixedVec3,
    pub rotation: FixedQuat,
    pub velocity: FixedVec3,
    pub angular_velocity: FixedVec3,
    pub mass: FixedPoint,
    pub inv_mass: FixedPoint,
    pub material: MaterialProperties,
    pub body_type: FixedBodyType,
    pub is_sleeping: bool,
    pub sleep_timer: FixedPoint,
    pub forces: FixedVec3,
    pub torque: FixedVec3,
    pub linear_damping: FixedPoint,
    pub angular_damping: FixedPoint,
    pub collision_group: u32,
    pub collision_mask: u32,
    pub ccd_enabled: bool,
}

impl FixedRigidBody {
    pub fn new(
        position: FixedVec3,
        mass: FixedPoint,
        material: MaterialProperties,
        body_type: FixedBodyType,
    ) -> Self {
        let inv_mass = if body_type == FixedBodyType::Static || mass.raw == 0 {
            FixedPoint::ZERO
        } else {
            FixedPoint::ONE / mass
        };
        Self {
            id: Uuid::new_v4(),
            position,
            rotation: FixedQuat::IDENTITY,
            velocity: FixedVec3::ZERO,
            angular_velocity: FixedVec3::ZERO,
            mass,
            inv_mass,
            material,
            body_type,
            is_sleeping: false,
            sleep_timer: FixedPoint::ZERO,
            forces: FixedVec3::ZERO,
            torque: FixedVec3::ZERO,
            linear_damping: FixedPoint::from_f32(0.01),
            angular_damping: FixedPoint::from_f32(0.01),
            collision_group: 1,
            collision_mask: 0xFFFFFFFF,
            ccd_enabled: false,
        }
    }

    pub fn wake_up(&mut self) {
        self.is_sleeping = false;
        self.sleep_timer = FixedPoint::ZERO;
    }

    pub fn put_to_sleep(&mut self) {
        self.is_sleeping = true;
        self.velocity = FixedVec3::ZERO;
        self.angular_velocity = FixedVec3::ZERO;
    }

    pub fn apply_force_at_point(&mut self, force: FixedVec3, point: FixedVec3) {
        self.forces += force;
        let r = point - self.position;
        self.torque += r.cross(force);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixedBodyType {
    Static,
    Dynamic,
    Kinematic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedCollisionEvent {
    pub body_a: Uuid,
    pub body_b: Uuid,
    pub contact_point: FixedVec3,
    pub contact_normal: FixedVec3,
    pub penetration_depth: FixedPoint,
    pub impulse: FixedVec3,
    pub relative_velocity: FixedVec3,
    pub friction: FixedPoint,
    pub restitution: FixedPoint,
}

impl FixedCollisionEvent {
    pub fn is_significant(&self) -> bool {
        self.impulse.length().to_f32() > 10.0 || self.penetration_depth.to_f32() > 0.01
    }

    pub fn total_kinetic_energy(&self) -> FixedPoint {
        let v_sq = self.relative_velocity.length_squared();
        let half = FixedPoint::from_f32(0.5);
        half * v_sq
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsSimulationParams {
    pub gravity: FixedVec3,
    pub air_density: FixedPoint,
    pub wind: FixedVec3,
    pub time_scale: FixedPoint,
    pub max_substeps: u32,
    pub sleep_threshold: FixedPoint,
    pub sleep_time: FixedPoint,
    pub solver_iterations: u32,
    pub enable_ccd: bool,
}

impl Default for PhysicsSimulationParams {
    fn default() -> Self {
        Self {
            gravity: FixedVec3::from_f32(0.0, -9.81, 0.0),
            air_density: FixedPoint::from_f32(1.225),
            wind: FixedVec3::ZERO,
            time_scale: FixedPoint::ONE,
            max_substeps: 4,
            sleep_threshold: FixedPoint::from_f32(0.05),
            sleep_time: FixedPoint::from_f32(1.0),
            solver_iterations: 4,
            enable_ccd: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DummyPhysicsBackend {
    pub bodies: Vec<FixedRigidBody>,
    pub collision_events: Vec<FixedCollisionEvent>,
    pub params: PhysicsSimulationParams,
    pub time: f64,
    pub step_count: u64,
    pub paused: bool,
}

impl DummyPhysicsBackend {
    pub fn new() -> Self {
        Self {
            bodies: Vec::new(),
            collision_events: Vec::new(),
            params: PhysicsSimulationParams::default(),
            time: 0.0,
            step_count: 0,
            paused: false,
        }
    }
}

impl Default for DummyPhysicsBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicsTrait for DummyPhysicsBackend {
    fn step(&mut self, dt: FixedPoint) {
        if self.paused {
            return;
        }
        let substeps = self.params.max_substeps;
        let sub_dt = dt / FixedPoint::from_i32(substeps as i32);

        for _ in 0..substeps {
            let gravity = self.params.gravity;
            let air_density = self.params.air_density;
            let wind = self.params.wind;

            for body in &mut self.bodies {
                if body.body_type != FixedBodyType::Dynamic {
                    continue;
                }
                body.velocity += gravity * sub_dt;
                let buoyancy = -gravity * (air_density / body.material.density);
                body.velocity += buoyancy * sub_dt;

                if wind.length_squared().raw > 0 {
                    let drag_coeff = FixedPoint::from_f32(0.47);
                    let mass_pow = FixedPoint::from_f32(body.mass.to_f32().powf(2.0 / 3.0));
                    let drag_scalar =
                        FixedPoint::from_f32(0.5) * air_density * drag_coeff * mass_pow;
                    let drag_force = wind * drag_scalar;
                    body.velocity += drag_force / body.mass * sub_dt;
                }
            }

            for body in &mut self.bodies {
                if body.body_type != FixedBodyType::Dynamic || body.is_sleeping {
                    continue;
                }
                let accel = body.forces / body.mass;
                body.velocity += accel * sub_dt;
                body.velocity *= FixedPoint::ONE - body.linear_damping * sub_dt;
                body.angular_velocity *= FixedPoint::ONE - body.angular_damping * sub_dt;
                body.forces = FixedVec3::ZERO;
                body.torque = FixedVec3::ZERO;
            }

            for body in &mut self.bodies {
                if body.body_type == FixedBodyType::Static || body.is_sleeping {
                    continue;
                }
                body.position += body.velocity * sub_dt;
                if body.angular_velocity.length_squared().raw > 0 {
                    let angle = body.angular_velocity.length() * sub_dt;
                    let axis = body.angular_velocity.normalize();
                    let half_angle = angle / FixedPoint::from_i32(2);
                    let sin_half = FixedPoint::from_f32(half_angle.to_f32().sin());
                    let cos_half = FixedPoint::from_f32(half_angle.to_f32().cos());
                    let dq = FixedQuat {
                        x: axis.x * sin_half,
                        y: axis.y * sin_half,
                        z: axis.z * sin_half,
                        w: cos_half,
                    };
                    body.rotation = FixedQuat {
                        x: dq.w * body.rotation.x + dq.x * body.rotation.w + dq.y * body.rotation.z
                            - dq.z * body.rotation.y,
                        y: dq.w * body.rotation.y - dq.x * body.rotation.z
                            + dq.y * body.rotation.w
                            + dq.z * body.rotation.x,
                        z: dq.w * body.rotation.z + dq.x * body.rotation.y - dq.y * body.rotation.x
                            + dq.z * body.rotation.w,
                        w: dq.w * body.rotation.w
                            - dq.x * body.rotation.x
                            - dq.y * body.rotation.y
                            - dq.z * body.rotation.z,
                    };
                    body.rotation = body.rotation.normalize();
                }
            }

            for body in &mut self.bodies {
                if body.body_type != FixedBodyType::Dynamic {
                    continue;
                }
                if body.velocity.length() < self.params.sleep_threshold
                    && body.angular_velocity.length() < self.params.sleep_threshold
                {
                    body.sleep_timer += sub_dt;
                    if body.sleep_timer > self.params.sleep_time {
                        body.put_to_sleep();
                    }
                } else {
                    body.sleep_timer = FixedPoint::ZERO;
                    body.is_sleeping = false;
                }
            }
        }

        self.time += dt.to_f64();
        self.step_count += 1;
    }

    fn apply_force(&mut self, body_id: Uuid, force: FixedVec3) {
        if let Some(body) = self.bodies.iter_mut().find(|b| b.id == body_id) {
            body.forces += force;
            body.wake_up();
        }
    }

    fn apply_impulse(&mut self, body_id: Uuid, impulse: FixedVec3, _at_point: FixedVec3) {
        if let Some(body) = self.bodies.iter_mut().find(|b| b.id == body_id) {
            body.velocity += impulse / body.mass;
            body.wake_up();
        }
    }

    fn apply_torque(&mut self, body_id: Uuid, torque: FixedVec3) {
        if let Some(body) = self.bodies.iter_mut().find(|b| b.id == body_id) {
            body.torque += torque;
            body.wake_up();
        }
    }

    fn add_rigid_body(&mut self, body: FixedRigidBody) -> Uuid {
        let id = body.id;
        self.bodies.push(body);
        id
    }

    fn remove_rigid_body(&mut self, body_id: Uuid) {
        self.bodies.retain(|b| b.id != body_id);
    }

    fn get_rigid_body(&self, body_id: Uuid) -> Option<&FixedRigidBody> {
        self.bodies.iter().find(|b| b.id == body_id)
    }

    fn get_rigid_body_mut(&mut self, body_id: Uuid) -> Option<&mut FixedRigidBody> {
        self.bodies.iter_mut().find(|b| b.id == body_id)
    }

    fn rigid_body_count(&self) -> usize {
        self.bodies.len()
    }

    fn get_collision_events(&self) -> &[FixedCollisionEvent] {
        &self.collision_events
    }

    fn drain_collision_events(&mut self) -> Vec<FixedCollisionEvent> {
        std::mem::take(&mut self.collision_events)
    }

    fn set_gravity(&mut self, gravity: FixedVec3) {
        self.params.gravity = gravity;
    }

    fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    fn set_time_scale(&mut self, scale: FixedPoint) {
        self.params.time_scale = scale;
    }

    fn backend_type(&self) -> PhysicsBackendType {
        PhysicsBackendType::Custom
    }

    fn serialize_state(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }

    fn deserialize_state(&mut self, data: &[u8]) -> bool {
        if let Ok(state) = bincode::deserialize::<DummyPhysicsBackend>(data) {
            *self = state;
            true
        } else {
            false
        }
    }

    fn world_time(&self) -> f64 {
        self.time
    }

    fn step_count(&self) -> u64 {
        self.step_count
    }

    fn reset(&mut self) {
        self.bodies.clear();
        self.collision_events.clear();
        self.time = 0.0;
        self.step_count = 0;
        self.paused = false;
    }
}
