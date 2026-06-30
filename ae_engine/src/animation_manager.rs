use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AnimationState {
    Idle,
    Walk,
    Run,
    Attack,
    Hit,
    Death,
    Jump,
    Crouch,
}

impl AnimationState {
    pub fn as_u8(&self) -> u8 {
        match self {
            AnimationState::Idle => 0,
            AnimationState::Walk => 1,
            AnimationState::Run => 2,
            AnimationState::Attack => 3,
            AnimationState::Hit => 4,
            AnimationState::Death => 5,
            AnimationState::Jump => 6,
            AnimationState::Crouch => 7,
        }
    }

    pub fn from_u8(val: u8) -> Self {
        match val {
            1 => AnimationState::Walk,
            2 => AnimationState::Run,
            3 => AnimationState::Attack,
            4 => AnimationState::Hit,
            5 => AnimationState::Death,
            6 => AnimationState::Jump,
            7 => AnimationState::Crouch,
            _ => AnimationState::Idle,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoneTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: f32,
}

impl Default for BoneTransform {
    fn default() -> Self {
        BoneTransform { translation: Vec3::ZERO, rotation: Quat::IDENTITY, scale: 1.0 }
    }
}

impl BoneTransform {
    pub fn to_mat4(&self) -> Mat4 {
        Mat4::from_translation(self.translation)
            * Mat4::from_quat(self.rotation)
            * Mat4::from_scale(Vec3::splat(self.scale))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimatedCharacter {
    pub id: u64,
    pub bone_count: usize,
    pub bone_transforms: Vec<BoneTransform>,
    pub current_state: AnimationState,
    pub previous_state: AnimationState,
    pub state_time: f32,
    pub blend_time: f32,
    pub blend_duration: f32,
    pub playback_speed: f32,
    pub loop_animation: bool,
}

impl AnimatedCharacter {
    pub fn new(id: u64, bone_count: usize) -> Self {
        AnimatedCharacter {
            id,
            bone_count,
            bone_transforms: vec![BoneTransform::default(); bone_count],
            current_state: AnimationState::Idle,
            previous_state: AnimationState::Idle,
            state_time: 0.0,
            blend_time: 0.0,
            blend_duration: 0.2,
            playback_speed: 1.0,
            loop_animation: true,
        }
    }

    pub fn set_state(&mut self, state: AnimationState) {
        if self.current_state != state {
            self.previous_state = self.current_state;
            self.current_state = state;
            self.state_time = 0.0;
            self.blend_time = self.blend_duration;
        }
    }

    pub fn set_state_from_u8(&mut self, state_u8: u8) {
        self.set_state(AnimationState::from_u8(state_u8));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VatInstance {
    pub id: u64,
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: f32,
    pub animation_id: u32,
    pub frame: f32,
    pub playback_speed: f32,
}

pub struct AnimationManager {
    pub characters: Vec<AnimatedCharacter>,
    pub vat_instances: Vec<VatInstance>,
    pub max_characters: usize,
    pub max_vat_instances: usize,
    pub global_time: f32,
}

impl AnimationManager {
    pub fn new(max_characters: usize, max_vat_instances: usize) -> Self {
        AnimationManager {
            characters: Vec::with_capacity(max_characters),
            vat_instances: Vec::with_capacity(max_vat_instances),
            max_characters,
            max_vat_instances,
            global_time: 0.0,
        }
    }

    pub fn register_character(&mut self, id: u64, bone_count: usize) {
        if self.characters.len() >= self.max_characters {
            return;
        }
        self.characters.push(AnimatedCharacter::new(id, bone_count));
    }

    pub fn unregister_character(&mut self, id: u64) {
        self.characters.retain(|c| c.id != id);
    }

    pub fn set_character_state(&mut self, id: u64, state: AnimationState) {
        if let Some(char) = self.characters.iter_mut().find(|c| c.id == id) {
            char.set_state(state);
        }
    }

    pub fn set_character_state_u8(&mut self, id: u64, state_u8: u8) {
        if let Some(char) = self.characters.iter_mut().find(|c| c.id == id) {
            char.set_state_from_u8(state_u8);
        }
    }

    pub fn register_vat_instance(&mut self, instance: VatInstance) {
        if self.vat_instances.len() >= self.max_vat_instances {
            self.vat_instances.remove(0);
        }
        self.vat_instances.push(instance);
    }

    pub fn step(&mut self, dt: f32) {
        self.global_time += dt;

        for char in &mut self.characters {
            char.state_time += dt * char.playback_speed;
            if char.blend_time > 0.0 {
                char.blend_time = (char.blend_time - dt).max(0.0);
            }

            if char.current_state == AnimationState::Death {
                char.loop_animation = false;
            }

            // Non-looping animations naturally stop at final pose once state_time
            // exceeds duration; no explicit action needed.
        }

        for vat in &mut self.vat_instances {
            vat.frame += dt * vat.playback_speed * 30.0; // 30fps VAT
            if vat.frame > 60.0 {
                vat.frame = 0.0; // Loop
            }
        }
    }

    pub fn get_bone_matrices(&self, character_id: u64) -> Vec<Mat4> {
        self.characters
            .iter()
            .find(|c| c.id == character_id)
            .map(|c| c.bone_transforms.iter().map(|t| t.to_mat4()).collect())
            .unwrap_or_default()
    }

    pub fn active_character_count(&self) -> usize {
        self.characters.len()
    }

    pub fn vat_instance_count(&self) -> usize {
        self.vat_instances.len()
    }
}
