use std::collections::VecDeque;

use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::collision::CollisionEvent;
use crate::destruction::VoxelGrid;
use crate::fixed_point::{FixedPoint, FixedQuat, FixedVec3};
use crate::material::MaterialProperties;

pub const FIXED_TIMESTEP: FixedPoint = FixedPoint { raw: 71582788 };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsWorld {
    pub gravity: FixedVec3,
    pub air_density: FixedPoint,
    pub wind: FixedVec3,
    pub temperature: FixedPoint,
    pub ambient_radiation: FixedPoint,
    pub time: f64,
    pub step_count: u64,
    pub paused: bool,
    pub time_scale: FixedPoint,
    pub rigid_bodies: Vec<RigidBody>,
    pub voxel_grids: Vec<VoxelGrid>,
    pub collision_events: VecDeque<CollisionEvent>,
    pub simulation_speed: SimulationSpeed,
    pub max_substeps: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimulationSpeed {
    Normal,
    Fast,
    VeryFast,
    Slow,
    Paused,
}

impl SimulationSpeed {
    pub fn time_scale(&self) -> FixedPoint {
        match self {
            Self::Normal => FixedPoint::ONE,
            Self::Fast => FixedPoint::from_f32(2.0),
            Self::VeryFast => FixedPoint::from_f32(4.0),
            Self::Slow => FixedPoint::from_f32(0.5),
            Self::Paused => FixedPoint::ZERO,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RigidBody {
    pub id: Uuid,
    pub position: FixedVec3,
    pub rotation: FixedQuat,
    pub velocity: FixedVec3,
    pub angular_velocity: FixedVec3,
    pub mass: FixedPoint,
    pub material: MaterialProperties,
    pub body_type: BodyType,
    pub is_sleeping: bool,
    pub sleep_timer: FixedPoint,
    pub forces: FixedVec3,
    pub torque: FixedVec3,
    pub linear_damping: FixedPoint,
    pub angular_damping: FixedPoint,
    pub mpss_index: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BodyType {
    Static,
    Dynamic,
    Kinematic,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self {
            gravity: FixedVec3::from_f32(0.0, -9.81, 0.0),
            air_density: FixedPoint::from_f32(1.225),
            wind: FixedVec3::ZERO,
            temperature: FixedPoint::from_f32(293.0),
            ambient_radiation: FixedPoint::ZERO,
            time: 0.0,
            step_count: 0,
            paused: false,
            time_scale: FixedPoint::ONE,
            rigid_bodies: Vec::new(),
            voxel_grids: Vec::new(),
            collision_events: VecDeque::with_capacity(1024),
            simulation_speed: SimulationSpeed::Normal,
            max_substeps: 4,
        }
    }
}

impl PhysicsWorld {
    pub fn step(&mut self) {
        if self.paused {
            return;
        }
        let dt = FIXED_TIMESTEP * self.time_scale * self.simulation_speed.time_scale();
        let substeps = self.max_substeps;
        let sub_dt = dt / FixedPoint::from_f32(substeps as f32);

        for _ in 0..substeps {
            self.apply_gravity(sub_dt);
            self.apply_wind(sub_dt);
            self.integrate_velocities(sub_dt);
            self.integrate_positions(sub_dt);
            self.update_sleep_state(sub_dt);
            self.apply_thermal_effects(sub_dt);
            self.apply_radiation_effects(sub_dt);
        }

        self.time += dt.to_f64();
        self.step_count += 1;
    }

    fn apply_gravity(&mut self, dt: FixedPoint) {
        for body in &mut self.rigid_bodies {
            if body.body_type != BodyType::Dynamic {
                continue;
            }
            body.velocity += self.gravity * dt;
            let buoyancy = -self.gravity * (self.air_density / body.material.density);
            body.velocity += buoyancy * dt;
        }
    }

    fn apply_wind(&mut self, dt: FixedPoint) {
        if self.wind.length_squared() < FixedPoint::from_f32(0.001) {
            return;
        }
        for body in &mut self.rigid_bodies {
            if body.body_type != BodyType::Dynamic {
                continue;
            }
            let drag_coefficient = FixedPoint::from_f32(0.47);
            let cross_section = FixedPoint::from_f32(body.mass.to_f32().powf(2.0 / 3.0));
            let drag_force = FixedPoint::from_f32(0.5)
                * self.air_density
                * drag_coefficient
                * cross_section
                * self.wind;
            body.velocity += drag_force / body.mass * dt;
        }
    }

    fn integrate_velocities(&mut self, dt: FixedPoint) {
        for body in &mut self.rigid_bodies {
            if body.body_type != BodyType::Dynamic || body.is_sleeping {
                continue;
            }
            let net_force = body.forces;
            let acceleration = net_force / body.mass;
            body.velocity += acceleration * dt;
            body.velocity *= FixedPoint::ONE - body.linear_damping * dt;
            body.angular_velocity *= FixedPoint::ONE - body.angular_damping * dt;
            body.forces = FixedVec3::ZERO;
            body.torque = FixedVec3::ZERO;
        }
    }

    fn integrate_positions(&mut self, dt: FixedPoint) {
        for body in &mut self.rigid_bodies {
            if body.body_type == BodyType::Static || body.is_sleeping {
                continue;
            }
            body.position += body.velocity * dt;
            if body.angular_velocity.length_squared() > FixedPoint::from_f32(0.001) {
                let av_glam = body.angular_velocity.to_glam();
                let delta_rot = glam::Quat::from_scaled_axis(av_glam * dt.to_f32());
                let rot_glam = body.rotation.to_glam();
                body.rotation = FixedQuat::from_glam((delta_rot * rot_glam).normalize());
            }
        }
    }

    fn update_sleep_state(&mut self, dt: FixedPoint) {
        let sleep_threshold = FixedPoint::from_f32(0.05);
        let sleep_time = FixedPoint::ONE;

        for body in &mut self.rigid_bodies {
            if body.body_type != BodyType::Dynamic {
                continue;
            }
            if body.velocity.length() < sleep_threshold
                && body.angular_velocity.length() < sleep_threshold
            {
                body.sleep_timer += dt;
                if body.sleep_timer > sleep_time {
                    body.is_sleeping = true;
                    body.velocity = FixedVec3::ZERO;
                    body.angular_velocity = FixedVec3::ZERO;
                }
            } else {
                body.sleep_timer = FixedPoint::ZERO;
                body.is_sleeping = false;
            }
        }
    }

    fn apply_thermal_effects(&mut self, dt: FixedPoint) {
        for grid in &mut self.voxel_grids {
            let ambient_diff = self.temperature - FixedPoint::from_f32(293.0);
            if ambient_diff.abs() > FixedPoint::ONE {
                let thermal_stress = ambient_diff * FixedPoint::from_f32(0.01) * dt;
                for voxel in &mut grid.voxels {
                    if voxel.flags.contains(crate::destruction::VoxelFlags::ACTIVE) {
                        voxel.health -= thermal_stress.abs();
                        if thermal_stress > FixedPoint::ZERO {
                            voxel.temperature += thermal_stress * FixedPoint::from_f32(0.1);
                        }
                    }
                }
            }
        }
    }

    fn apply_radiation_effects(&mut self, dt: FixedPoint) {
        if self.ambient_radiation < FixedPoint::from_f32(0.01) {
            return;
        }
        for grid in &mut self.voxel_grids {
            let absorbed = self.ambient_radiation
                * (FixedPoint::ONE - grid.material.radiation_resistance)
                * dt;
            for voxel in &mut grid.voxels {
                if voxel.flags.contains(crate::destruction::VoxelFlags::ACTIVE) {
                    voxel.radiation_level += absorbed;
                    voxel.health -= absorbed * FixedPoint::from_f32(0.001);
                }
            }
        }
    }

    pub fn add_rigid_body(&mut self, body: RigidBody) {
        self.rigid_bodies.push(body);
    }

    pub fn remove_rigid_body(&mut self, id: Uuid) {
        self.rigid_bodies.retain(|b| b.id != id);
    }

    pub fn add_voxel_grid(&mut self, grid: VoxelGrid) {
        self.voxel_grids.push(grid);
    }

    pub fn apply_force(&mut self, id: Uuid, force: Vec3) {
        if let Some(body) = self.rigid_bodies.iter_mut().find(|b| b.id == id) {
            body.forces += FixedVec3::from_glam(force);
            body.is_sleeping = false;
        }
    }

    pub fn apply_impulse(&mut self, id: Uuid, impulse: Vec3) {
        if let Some(body) = self.rigid_bodies.iter_mut().find(|b| b.id == id) {
            body.velocity += FixedVec3::from_glam(impulse) / body.mass;
            body.is_sleeping = false;
            body.sleep_timer = FixedPoint::ZERO;
        }
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    pub fn set_time_scale(&mut self, scale: f32) {
        self.time_scale =
            FixedPoint::from_f32(scale).clamp(FixedPoint::ZERO, FixedPoint::from_f32(10.0));
    }

    pub fn set_simulation_speed(&mut self, speed: SimulationSpeed) {
        self.simulation_speed = speed;
    }

    pub fn serialize_state(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }

    pub fn deserialize_state(data: &[u8]) -> Option<Self> {
        bincode::deserialize(data).ok()
    }
}
