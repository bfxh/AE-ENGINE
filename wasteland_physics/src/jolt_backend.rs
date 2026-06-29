use serde::{Deserialize, Serialize};
#[cfg(feature = "jolt")]
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(feature = "jolt")]
use crate::fixed_point::FixedQuat;
use crate::fixed_point::{FixedPoint, FixedVec3};
use crate::physics_trait::{
    FixedBodyType, FixedCollisionEvent, FixedRigidBody, PhysicsBackendType,
    PhysicsSimulationParams, PhysicsTrait,
};

#[cfg(feature = "jolt")]
use rolt::{
    Body, BodyCreationSettings, BodyId, BodyInterface, BodyType, BroadPhaseLayer, DVec3,
    EActivation, EMotionType, MotionType, ObjectLayer, PhysicsSystem, Quat, Vec3,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoltPhysicsBackend {
    pub bodies: Vec<FixedRigidBody>,
    #[serde(skip)]
    #[cfg(feature = "jolt")]
    pub jolt_system: Option<JoltSystemWrapper>,
    pub collision_events: Vec<FixedCollisionEvent>,
    pub params: PhysicsSimulationParams,
    pub time: f64,
    pub step_count: u64,
    pub paused: bool,
}

#[cfg(feature = "jolt")]
#[derive(Debug, Clone)]
pub struct JoltSystemWrapper {
    pub physics_system: PhysicsSystem,
    pub body_interface: BodyInterface,
    pub id_map: HashMap<Uuid, BodyId>,
    pub reverse_id_map: HashMap<BodyId, Uuid>,
}

impl JoltPhysicsBackend {
    pub fn new() -> Self {
        let backend = Self {
            bodies: Vec::new(),
            #[cfg(feature = "jolt")]
            jolt_system: None,
            collision_events: Vec::new(),
            params: PhysicsSimulationParams::default(),
            time: 0.0,
            step_count: 0,
            paused: false,
        };

        #[cfg(feature = "jolt")]
        {
            backend.init_jolt()
        }
        #[cfg(not(feature = "jolt"))]
        {
            backend
        }
    }

    #[cfg(feature = "jolt")]
    fn init_jolt(mut self) -> Self {
        const MAX_BODIES: u32 = 16384;
        const MAX_BODY_PAIRS: u32 = 65536;
        const MAX_CONTACT_CONSTRAINTS: u32 = 16384;

        let physics_system = PhysicsSystem::new();
        physics_system.init(
            MAX_BODIES,
            0,
            MAX_BODY_PAIRS,
            MAX_CONTACT_CONSTRAINTS,
            ObjectLayer::default(),
            BroadPhaseLayer::default(),
            BroadPhaseLayer::default(),
            1,
        );

        let body_interface = physics_system.body_interface();

        self.jolt_system = Some(JoltSystemWrapper {
            physics_system,
            body_interface,
            id_map: HashMap::new(),
            reverse_id_map: HashMap::new(),
        });

        self
    }

    #[cfg(feature = "jolt")]
    fn fixed_to_jolt_vec3(v: FixedVec3) -> DVec3 {
        DVec3::new(v.x.to_f64(), v.y.to_f64(), v.z.to_f64())
    }

    #[cfg(feature = "jolt")]
    fn fixed_to_jolt_quat(q: FixedQuat) -> Quat {
        Quat::new(q.x.to_f32(), q.y.to_f32(), q.z.to_f32(), q.w.to_f32())
    }

    #[cfg(feature = "jolt")]
    fn jolt_to_fixed_vec3(v: DVec3) -> FixedVec3 {
        FixedVec3::from_f32(v.x() as f32, v.y() as f32, v.z() as f32)
    }

    #[cfg(feature = "jolt")]
    fn body_type_to_motion(bt: FixedBodyType) -> MotionType {
        match bt {
            FixedBodyType::Static => MotionType::Static,
            FixedBodyType::Dynamic => MotionType::Dynamic,
            FixedBodyType::Kinematic => MotionType::Kinematic,
        }
    }
}

impl Default for JoltPhysicsBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicsTrait for JoltPhysicsBackend {
    fn step(&mut self, dt: FixedPoint) {
        if self.paused {
            return;
        }

        #[cfg(feature = "jolt")]
        {
            if let Some(ref mut wrapper) = self.jolt_system {
                let dt_f32 = dt.to_f32();
                let collision_steps = 1;

                let result = wrapper.physics_system.update(
                    dt_f32,
                    collision_steps,
                    1,
                    &Default::default(),
                    &Default::default(),
                );

                if let Some(err) = result.err() {
                    log::error!("Jolt physics step error: {:?}", err);
                }

                for (body_id, _) in &wrapper.id_map {
                    let jolt_body = wrapper.body_interface.lock_body(*body_id);
                    if let Ok(mut jolt_body) = jolt_body {
                        let pos = jolt_body.position();
                        let rot = jolt_body.rotation();
                        let vel = jolt_body.linear_velocity();
                        let ang_vel = jolt_body.angular_velocity();

                        if let Some(uuid) = wrapper.reverse_id_map.get(body_id) {
                            if let Some(body) = self.bodies.iter_mut().find(|b| b.id == *uuid) {
                                body.position = Self::jolt_to_fixed_vec3(pos.translation);
                                body.rotation = Self::jolt_to_fixed_quat(rot);
                                body.velocity = Self::jolt_to_fixed_vec3(vel);
                                body.angular_velocity = Self::jolt_to_fixed_vec3(ang_vel);
                            }
                        }
                    }
                }
            }
        }

        #[cfg(not(feature = "jolt"))]
        {
            let substeps = self.params.max_substeps;
            let sub_dt = dt / FixedPoint::from_i32(substeps as i32);
            let gravity = self.params.gravity;

            for _ in 0..substeps {
                for body in &mut self.bodies {
                    if body.body_type != FixedBodyType::Dynamic {
                        continue;
                    }
                    body.velocity += gravity * sub_dt;
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
                }
            }
        }

        self.time += dt.to_f64();
        self.step_count += 1;
    }

    fn apply_force(&mut self, body_id: Uuid, force: FixedVec3) {
        #[cfg(feature = "jolt")]
        {
            if let Some(ref wrapper) = self.jolt_system {
                if let Some(jolt_id) = wrapper.id_map.get(&body_id) {
                    let jolt_force =
                        Vec3::new(force.x.to_f32(), force.y.to_f32(), force.z.to_f32());
                    wrapper.body_interface.add_force(*jolt_id, jolt_force);
                }
            }
        }
        #[cfg(not(feature = "jolt"))]
        {
            if let Some(body) = self.bodies.iter_mut().find(|b| b.id == body_id) {
                body.forces += force;
                body.wake_up();
            }
        }
    }

    fn apply_impulse(&mut self, body_id: Uuid, impulse: FixedVec3, _at_point: FixedVec3) {
        #[cfg(feature = "jolt")]
        {
            if let Some(ref wrapper) = self.jolt_system {
                if let Some(jolt_id) = wrapper.id_map.get(&body_id) {
                    let jolt_impulse =
                        Vec3::new(impulse.x.to_f32(), impulse.y.to_f32(), impulse.z.to_f32());
                    let jolt_point =
                        DVec3::new(at_point.x.to_f64(), at_point.y.to_f64(), at_point.z.to_f64());
                    wrapper.body_interface.add_impulse(*jolt_id, jolt_impulse, jolt_point);
                }
            }
        }
        #[cfg(not(feature = "jolt"))]
        {
            if let Some(body) = self.bodies.iter_mut().find(|b| b.id == body_id) {
                body.velocity += impulse / body.mass;
                body.wake_up();
            }
        }
    }

    fn apply_torque(&mut self, body_id: Uuid, torque: FixedVec3) {
        #[cfg(feature = "jolt")]
        {
            if let Some(ref wrapper) = self.jolt_system {
                if let Some(jolt_id) = wrapper.id_map.get(&body_id) {
                    let jolt_torque =
                        Vec3::new(torque.x.to_f32(), torque.y.to_f32(), torque.z.to_f32());
                    wrapper.body_interface.add_torque(*jolt_id, jolt_torque);
                }
            }
        }
        #[cfg(not(feature = "jolt"))]
        {
            if let Some(body) = self.bodies.iter_mut().find(|b| b.id == body_id) {
                body.torque += torque;
                body.wake_up();
            }
        }
    }

    fn add_rigid_body(&mut self, body: FixedRigidBody) -> Uuid {
        let id = body.id;

        #[cfg(feature = "jolt")]
        {
            if let Some(ref mut wrapper) = self.jolt_system {
                let position = Self::fixed_to_jolt_vec3(body.position);
                let rotation = Self::fixed_to_jolt_quat(body.rotation);
                let motion_type = Self::body_type_to_motion(body.body_type);
                let layer = ObjectLayer::default();

                let settings = BodyCreationSettings::new(
                    layer,
                    motion_type,
                    position,
                    rotation,
                    body.collision_group as u16,
                );

                let jolt_body = wrapper.body_interface.create_body(settings);
                if let Ok(jolt_body) = jolt_body {
                    let jolt_id = jolt_body.id();
                    wrapper.id_map.insert(id, jolt_id);
                    wrapper.reverse_id_map.insert(jolt_id, id);
                }
            }
        }

        self.bodies.push(body);
        id
    }

    fn remove_rigid_body(&mut self, body_id: Uuid) {
        #[cfg(feature = "jolt")]
        {
            if let Some(ref mut wrapper) = self.jolt_system {
                if let Some(jolt_id) = wrapper.id_map.remove(&body_id) {
                    wrapper.reverse_id_map.remove(&jolt_id);
                    wrapper.body_interface.remove_body(jolt_id);
                    wrapper.body_interface.destroy_body(jolt_id);
                }
            }
        }
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
        #[cfg(feature = "jolt")]
        {
            if let Some(ref mut wrapper) = self.jolt_system {
                let jolt_gravity =
                    Vec3::new(gravity.x.to_f32(), gravity.y.to_f32(), gravity.z.to_f32());
                wrapper.physics_system.set_gravity(jolt_gravity);
            }
        }
    }

    fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    fn set_time_scale(&mut self, scale: FixedPoint) {
        self.params.time_scale = scale;
    }

    fn backend_type(&self) -> PhysicsBackendType {
        #[cfg(feature = "jolt")]
        {
            PhysicsBackendType::Jolt
        }
        #[cfg(not(feature = "jolt"))]
        {
            PhysicsBackendType::Custom
        }
    }

    fn serialize_state(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }

    fn deserialize_state(&mut self, data: &[u8]) -> bool {
        if let Ok(state) = bincode::deserialize::<JoltPhysicsBackend>(data) {
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
        #[cfg(feature = "jolt")]
        {
            if let Some(ref mut wrapper) = self.jolt_system {
                for (_, jolt_id) in &wrapper.id_map {
                    wrapper.body_interface.remove_body(*jolt_id);
                    wrapper.body_interface.destroy_body(*jolt_id);
                }
                wrapper.id_map.clear();
                wrapper.reverse_id_map.clear();
            }
        }
        self.bodies.clear();
        self.collision_events.clear();
        self.time = 0.0;
        self.step_count = 0;
        self.paused = false;
    }
}
