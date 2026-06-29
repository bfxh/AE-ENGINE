use godot::prelude::*;

use wasteland_animation::blend::{BoneTransform, Pose};
use wasteland_animation::gait::{GaitController, GaitType};
use wasteland_animation::ik::{IKBone, IKChain};
use wasteland_animation::state_machine::AnimationStateMachine;

#[derive(GodotClass)]
#[class(base=Node)]
pub(crate) struct WastelandAnimation {
    #[var]
    ik_iterations: i64,
    #[var]
    ik_tolerance: f32,
    #[var]
    blend_speed: f32,
    #[var]
    gait_speed: f32,

    ik_chains: Vec<IKChain>,
    current_pose: Option<Pose>,
    target_pose: Option<Pose>,
    gait: GaitController,
    state_machine: AnimationStateMachine<String>,
    blend_t: f32,
    active_animations: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandAnimation {
    fn init(base: Base<Node>) -> Self {
        Self {
            ik_iterations: 10,
            ik_tolerance: 0.001,
            blend_speed: 5.0,
            gait_speed: 1.0,
            ik_chains: Vec::new(),
            current_pose: None,
            target_pose: None,
            gait: GaitController::new(GaitType::Walk, 4),
            state_machine: AnimationStateMachine::new(String::from("idle")),
            blend_t: 0.0,
            active_animations: 0,
            base,
        }
    }
}

#[godot_api]
impl WastelandAnimation {
    #[func]
    fn create_ik_chain(&mut self, bone_count: i64) -> i64 {
        if bone_count <= 0 {
            return -1;
        }
        let bones = (0..bone_count as usize)
            .map(|i| IKBone {
                position: glam::Vec3::new(i as f32, 0.0, 0.0),
                length: 1.0,
                name: format!("bone_{}", i),
            })
            .collect();
        let chain = IKChain::new(bones, self.ik_iterations as usize, self.ik_tolerance);
        let idx = self.ik_chains.len() as i64;
        self.ik_chains.push(chain);
        idx
    }

    #[func]
    fn set_ik_target(&mut self, chain_idx: i64, tx: f32, ty: f32, tz: f32) -> bool {
        if chain_idx < 0 || chain_idx >= self.ik_chains.len() as i64 {
            return false;
        }
        let target = glam::Vec3::new(tx, ty, tz);
        self.ik_chains[chain_idx as usize].solve_fabrik(target)
    }

    #[func]
    fn get_ik_bone_position(&self, chain_idx: i64, bone_idx: i64) -> Vector3 {
        if chain_idx < 0 || chain_idx >= self.ik_chains.len() as i64 {
            return Vector3::ZERO;
        }
        let chain = &self.ik_chains[chain_idx as usize];
        if bone_idx < 0 || bone_idx >= chain.bones.len() as i64 {
            return Vector3::ZERO;
        }
        let p = chain.bones[bone_idx as usize].position;
        Vector3::new(p.x, p.y, p.z)
    }

    #[func]
    fn get_ik_chain_length(&self, chain_idx: i64) -> f32 {
        if chain_idx < 0 || chain_idx >= self.ik_chains.len() as i64 {
            return 0.0;
        }
        self.ik_chains[chain_idx as usize].total_length()
    }

    #[func]
    fn create_pose(&mut self, bone_count: i64) {
        self.current_pose = Some(Pose::new(bone_count as usize));
        self.target_pose = Some(Pose::new(bone_count as usize));
        self.blend_t = 0.0;
        self.active_animations += 1;
    }

    #[func]
    #[allow(clippy::too_many_arguments)]
    fn set_pose_bone(
        &mut self,
        bone_idx: i64,
        tx: f32,
        ty: f32,
        tz: f32,
        rx: f32,
        ry: f32,
        rz: f32,
        rw: f32,
        sx: f32,
        sy: f32,
        sz: f32,
    ) {
        if let Some(ref mut pose) = self.target_pose {
            if bone_idx >= 0 && (bone_idx as usize) < pose.bones.len() {
                pose.bones[bone_idx as usize] = BoneTransform {
                    translation: glam::Vec3::new(tx, ty, tz),
                    rotation: glam::Quat::from_xyzw(rx, ry, rz, rw),
                    scale: glam::Vec3::new(sx, sy, sz),
                };
            }
        }
    }

    #[func]
    fn blend_update(&mut self, delta: f32) {
        if self.current_pose.is_none() || self.target_pose.is_none() {
            return;
        }
        self.blend_t = (self.blend_t + delta * self.blend_speed).min(1.0);
        if self.blend_t >= 1.0 {
            self.current_pose = self.target_pose.clone();
            self.blend_t = 0.0;
        }
    }

    #[func]
    fn get_blend_factor(&self) -> f32 {
        self.blend_t
    }

    #[func]
    fn get_bone_transform(&self, bone_idx: i64) -> Dictionary<Variant, Variant> {
        if let Some(ref pose) = self.current_pose {
            if let Some(ref target) = self.target_pose {
                if bone_idx >= 0 && (bone_idx as usize) < pose.bones.len() {
                    let current = &pose.bones[bone_idx as usize];
                    let target_bone = &target.bones[bone_idx as usize];
                    let interp = current.lerp(target_bone, self.blend_t);
                    return dict! {
                        "tx" => interp.translation.x,
                        "ty" => interp.translation.y,
                        "tz" => interp.translation.z,
                        "rx" => interp.rotation.x,
                        "ry" => interp.rotation.y,
                        "rz" => interp.rotation.z,
                        "rw" => interp.rotation.w,
                        "sx" => interp.scale.x,
                        "sy" => interp.scale.y,
                        "sz" => interp.scale.z,
                    };
                }
            }
        }
        dict! {}
    }

    #[func]
    fn set_gait_type(&mut self, gait_name: GString) {
        let gait = match gait_name.to_string().as_str() {
            "walk" => GaitType::Walk,
            "run" => GaitType::Run,
            "sneak" => GaitType::Sneak,
            "limp" => GaitType::Limp,
            _ => GaitType::Walk,
        };
        self.gait.gait = gait;
        self.gait.speed = self.gait_speed;
    }

    #[func]
    fn update_gait(&mut self, delta: f32) {
        self.gait.update(delta);
    }

    #[func]
    #[allow(clippy::too_many_arguments)]
    fn get_foot_position(
        &self,
        leg_index: i64,
        leg_count: i64,
        rx: f32,
        ry: f32,
        rz: f32,
        dx: f32,
        dy: f32,
        dz: f32,
    ) -> Vector3 {
        let rest = glam::Vec3::new(rx, ry, rz);
        let dir = glam::Vec3::new(dx, dy, dz);
        let pos = self.gait.foot_position(leg_index as usize, leg_count as usize, rest, dir);
        Vector3::new(pos.x, pos.y, pos.z)
    }

    #[func]
    fn set_animation_state(&mut self, state_name: GString) {
        let name = state_name.to_string();
        if !self.state_machine.states.contains_key(&name) {
            self.state_machine.add_state(name.clone(), &name, 1.0, true);
        }
        self.state_machine.current_state = name;
    }

    #[func]
    fn update_state_machine(&mut self, delta: f32) {
        self.state_machine.update(delta);
    }

    #[func]
    fn set_state_bool(&mut self, name: GString, value: bool) {
        self.state_machine.set_bool(&name.to_string(), value);
    }

    #[func]
    fn set_state_float(&mut self, name: GString, value: f32) {
        self.state_machine.set_float(&name.to_string(), value);
    }

    #[func]
    fn trigger_state(&mut self, name: GString) {
        self.state_machine.trigger(&name.to_string());
    }

    #[func]
    fn get_animation_stats(&self) -> Dictionary<Variant, Variant> {
        let bone_count = self.current_pose.as_ref().map_or(0, |p| p.bones.len());
        dict! {
            "ik_chains" => self.ik_chains.len() as i64,
            "ik_iterations" => self.ik_iterations,
            "ik_tolerance" => self.ik_tolerance,
            "blend_factor" => self.blend_t,
            "blend_speed" => self.blend_speed,
            "gait_speed" => self.gait_speed,
            "active_animations" => self.active_animations,
            "pose_bone_count" => bone_count as i64,
        }
    }
}
