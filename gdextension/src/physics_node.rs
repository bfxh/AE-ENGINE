use godot::prelude::*;
use std::sync::Mutex;
use uuid::Uuid;
use wasteland_physics::fixed_point::{FixedPoint, FixedQuat, FixedVec3};
use wasteland_physics::world::{BodyType, PhysicsWorld, RigidBody, SimulationSpeed};

#[allow(dead_code)]
struct Constraint {
    id: i64,
    body_a_id: Uuid,
    body_b_id: Uuid,
    anchor: glam::Vec3,
    constraint_type: String,
}

#[allow(dead_code)]
struct Joint {
    id: i64,
    body_a_id: Uuid,
    body_b_id: Uuid,
    pivot: glam::Vec3,
    joint_type: String,
}

#[derive(GodotClass)]
#[class(base=Node3D)]
struct WastelandPhysics {
    world: Mutex<PhysicsWorld>,
    constraints: Mutex<Vec<Constraint>>,
    joints: Mutex<Vec<Joint>>,
    next_constraint_id: Mutex<i64>,
    next_joint_id: Mutex<i64>,

    #[var]
    paused: bool,

    #[var]
    time_scale: f64,

    #[base]
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for WastelandPhysics {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            world: Mutex::new(PhysicsWorld::default()),
            constraints: Mutex::new(Vec::new()),
            joints: Mutex::new(Vec::new()),
            next_constraint_id: Mutex::new(1),
            next_joint_id: Mutex::new(1),
            paused: false,
            time_scale: 1.0,
            base,
        }
    }

    fn process(&mut self, _delta: f64) {
        if self.paused {
            return;
        }
        if let Ok(mut world) = self.world.lock() {
            world.set_paused(self.paused);
            world.set_time_scale(self.time_scale as f32);
            world.step();
        }
    }
}

#[godot_api]
impl WastelandPhysics {
    #[func]
    fn add_rigid_body(&mut self, mass: f32, px: f32, py: f32, pz: f32, is_static: bool) -> GString {
        if let Ok(mut world) = self.world.lock() {
            let id = Uuid::new_v4();
            let body = RigidBody {
                id,
                position: FixedVec3::from_f32(px, py, pz),
                rotation: FixedQuat::from_glam(glam::Quat::IDENTITY),
                velocity: FixedVec3::ZERO,
                angular_velocity: FixedVec3::ZERO,
                mass: FixedPoint::from_f32(mass),
                material: wasteland_physics::material::MaterialProperties::concrete(),
                body_type: if is_static { BodyType::Static } else { BodyType::Dynamic },
                is_sleeping: false,
                sleep_timer: FixedPoint::ZERO,
                forces: FixedVec3::ZERO,
                torque: FixedVec3::ZERO,
                linear_damping: FixedPoint::from_f32(0.1),
                angular_damping: FixedPoint::from_f32(0.1),
                mpss_index: None,
            };
            world.add_rigid_body(body);
            let s = id.to_string();
            return GString::from(s.as_str());
        }
        GString::new()
    }

    #[func]
    fn remove_rigid_body(&mut self, id: GString) {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(mut world) = self.world.lock() {
                world.remove_rigid_body(parsed);
            }
        }
    }

    #[func]
    fn apply_force(&mut self, id: GString, fx: f32, fy: f32, fz: f32) {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(mut world) = self.world.lock() {
                world.apply_force(parsed, glam::Vec3::new(fx, fy, fz));
            }
        }
    }

    #[func]
    fn apply_impulse(&mut self, id: GString, ix: f32, iy: f32, iz: f32) {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(mut world) = self.world.lock() {
                world.apply_impulse(parsed, glam::Vec3::new(ix, iy, iz));
            }
        }
    }

    #[func]
    fn apply_torque(&mut self, id: GString, tx: f32, ty: f32, tz: f32) {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(mut world) = self.world.lock() {
                if let Some(body) = world.rigid_bodies.iter_mut().find(|b| b.id == parsed) {
                    body.torque += FixedVec3::from_f32(tx, ty, tz);
                    body.is_sleeping = false;
                }
            }
        }
    }

    #[func]
    fn get_body_position(&self, id: GString) -> Vector3 {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(world) = self.world.lock() {
                if let Some(body) = world.rigid_bodies.iter().find(|b| b.id == parsed) {
                    let p = body.position.to_glam();
                    return Vector3::new(p.x, p.y, p.z);
                }
            }
        }
        Vector3::ZERO
    }

    #[func]
    fn get_body_velocity(&self, id: GString) -> Vector3 {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(world) = self.world.lock() {
                if let Some(body) = world.rigid_bodies.iter().find(|b| b.id == parsed) {
                    let v = body.velocity.to_glam();
                    return Vector3::new(v.x, v.y, v.z);
                }
            }
        }
        Vector3::ZERO
    }

    #[func]
    fn get_body_rotation(&self, id: GString) -> Quaternion {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(world) = self.world.lock() {
                if let Some(body) = world.rigid_bodies.iter().find(|b| b.id == parsed) {
                    let r = body.rotation.to_glam();
                    return Quaternion::new(r.x, r.y, r.z, r.w);
                }
            }
        }
        Quaternion::IDENTITY
    }

    #[func]
    fn is_body_sleeping(&self, id: GString) -> bool {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(world) = self.world.lock() {
                if let Some(body) = world.rigid_bodies.iter().find(|b| b.id == parsed) {
                    return body.is_sleeping;
                }
            }
        }
        false
    }

    #[func]
    fn get_body_mass(&self, id: GString) -> f32 {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(world) = self.world.lock() {
                if let Some(body) = world.rigid_bodies.iter().find(|b| b.id == parsed) {
                    return body.mass.to_f32();
                }
            }
        }
        0.0
    }

    #[func]
    fn set_body_sleeping(&mut self, id: GString, sleeping: bool) {
        if let Ok(parsed) = Uuid::parse_str(&id.to_string()) {
            if let Ok(mut world) = self.world.lock() {
                if let Some(body) = world.rigid_bodies.iter_mut().find(|b| b.id == parsed) {
                    body.is_sleeping = sleeping;
                    if sleeping {
                        body.velocity = FixedVec3::ZERO;
                        body.angular_velocity = FixedVec3::ZERO;
                    } else {
                        body.sleep_timer = FixedPoint::ZERO;
                    }
                }
            }
        }
    }

    #[func]
    fn set_gravity_vector(&mut self, gx: f32, gy: f32, gz: f32) {
        if let Ok(mut world) = self.world.lock() {
            world.gravity = FixedVec3::from_f32(gx, gy, gz);
        }
    }

    #[func]
    fn set_wind(&mut self, wx: f32, wy: f32, wz: f32) {
        if let Ok(mut world) = self.world.lock() {
            world.wind = FixedVec3::from_f32(wx, wy, wz);
        }
    }

    #[func]
    fn set_temperature(&mut self, temp: f32) {
        if let Ok(mut world) = self.world.lock() {
            world.temperature = FixedPoint::from_f32(temp);
        }
    }

    #[func]
    fn set_radiation(&mut self, rad: f32) {
        if let Ok(mut world) = self.world.lock() {
            world.ambient_radiation = FixedPoint::from_f32(rad);
        }
    }

    #[func]
    fn set_simulation_speed(&mut self, speed: i64) {
        if let Ok(mut world) = self.world.lock() {
            let s = match speed {
                0 => SimulationSpeed::Normal,
                1 => SimulationSpeed::Fast,
                2 => SimulationSpeed::VeryFast,
                3 => SimulationSpeed::Slow,
                _ => SimulationSpeed::Paused,
            };
            world.set_simulation_speed(s);
        }
    }

    #[func]
    fn get_collision_events(&self) -> Array<Variant> {
        let mut arr = Array::new();
        if let Ok(world) = self.world.lock() {
            for evt in &world.collision_events {
                let pt = evt.point.to_glam();
                let nm = evt.normal.to_glam();
                let d: Dictionary<Variant, Variant> = dict! {
                    "entity_a" => evt.entity_a.to_string().as_str(),
                    "entity_b" => evt.entity_b.to_string().as_str(),
                    "point_x" => pt.x,
                    "point_y" => pt.y,
                    "point_z" => pt.z,
                    "normal_x" => nm.x,
                    "normal_y" => nm.y,
                    "normal_z" => nm.z,
                    "impulse" => evt.impulse.to_f32(),
                    "relative_velocity" => evt.relative_velocity.to_glam().length(),
                    "collision_type" => format!("{:?}", evt.collision_type).as_str(),
                    "timestamp" => evt.timestamp,
                };
                arr.push(&d);
            }
        }
        arr
    }

    #[func]
    fn collision_event_count(&self) -> i64 {
        if let Ok(world) = self.world.lock() {
            return world.collision_events.len() as i64;
        }
        0
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        if let Ok(world) = self.world.lock() {
            return dict! {
                "rigid_body_count" => world.rigid_bodies.len() as i64,
                "collision_event_count" => world.collision_events.len() as i64,
                "voxel_grid_count" => world.voxel_grids.len() as i64,
                "step_count" => world.step_count as i64,
                "time" => world.time,
                "temperature" => world.temperature.to_f32(),
                "air_density" => world.air_density.to_f32(),
            };
        }
        dict! {}
    }

    #[func]
    fn serialize_state(&self) -> PackedByteArray {
        if let Ok(world) = self.world.lock() {
            let data = world.serialize_state();
            let mut arr = PackedByteArray::new();
            for b in data {
                arr.push(b);
            }
            return arr;
        }
        PackedByteArray::new()
    }

    #[func]
    fn deserialize_state(&mut self, data: PackedByteArray) -> bool {
        let bytes: Vec<u8> = data.as_slice().to_vec();
        if let Some(world) = PhysicsWorld::deserialize_state(&bytes) {
            if let Ok(mut guard) = self.world.lock() {
                *guard = world;
                return true;
            }
        }
        false
    }

    #[func]
    fn add_constraint(
        &mut self,
        body_a_id: GString,
        body_b_id: GString,
        anchor_x: f32,
        anchor_y: f32,
        anchor_z: f32,
        constraint_type: GString,
    ) -> i64 {
        if let (Ok(a), Ok(b)) =
            (Uuid::parse_str(&body_a_id.to_string()), Uuid::parse_str(&body_b_id.to_string()))
        {
            let id = {
                let mut next = self.next_constraint_id.lock().unwrap();
                let id = *next;
                *next += 1;
                id
            };
            let constraint = Constraint {
                id,
                body_a_id: a,
                body_b_id: b,
                anchor: glam::Vec3::new(anchor_x, anchor_y, anchor_z),
                constraint_type: constraint_type.to_string(),
            };
            if let Ok(mut constraints) = self.constraints.lock() {
                constraints.push(constraint);
            }
            return id;
        }
        -1
    }

    #[func]
    fn remove_constraint(&mut self, id: i64) {
        if let Ok(mut constraints) = self.constraints.lock() {
            constraints.retain(|c| c.id != id);
        }
    }

    #[func]
    fn get_constraint_count(&self) -> i64 {
        if let Ok(constraints) = self.constraints.lock() {
            return constraints.len() as i64;
        }
        0
    }

    #[func]
    fn add_joint(
        &mut self,
        body_a_id: GString,
        body_b_id: GString,
        px: f32,
        py: f32,
        pz: f32,
        joint_type: GString,
    ) -> i64 {
        if let (Ok(a), Ok(b)) =
            (Uuid::parse_str(&body_a_id.to_string()), Uuid::parse_str(&body_b_id.to_string()))
        {
            let id = {
                let mut next = self.next_joint_id.lock().unwrap();
                let id = *next;
                *next += 1;
                id
            };
            let joint = Joint {
                id,
                body_a_id: a,
                body_b_id: b,
                pivot: glam::Vec3::new(px, py, pz),
                joint_type: joint_type.to_string(),
            };
            if let Ok(mut joints) = self.joints.lock() {
                joints.push(joint);
            }
            return id;
        }
        -1
    }

    #[func]
    fn remove_joint(&mut self, id: i64) {
        if let Ok(mut joints) = self.joints.lock() {
            joints.retain(|j| j.id != id);
        }
    }

    #[func]
    fn get_joint_count(&self) -> i64 {
        if let Ok(joints) = self.joints.lock() {
            return joints.len() as i64;
        }
        0
    }
}
