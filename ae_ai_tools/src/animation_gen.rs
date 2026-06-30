use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationRequest {
    pub animation_type: AnimationType,
    pub skeleton: SkeletonDefinition,
    pub duration_seconds: f32,
    pub fps: u32,
    pub constraints: Vec<AnimationConstraint>,
    pub style: AnimationStyle,
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimationType {
    Walk,
    Run,
    Jump,
    Climb,
    Attack,
    Idle,
    Death,
    Interact,
    Custom(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimationStyle {
    Realistic,
    Stylized,
    Mechanical,
    Organic,
    Weighted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkeletonDefinition {
    pub name: String,
    pub joints: Vec<JointDefinition>,
    pub root_joint: usize,
    pub total_bones: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JointDefinition {
    pub name: String,
    pub parent: Option<usize>,
    pub bind_position: [f32; 3],
    pub bind_rotation: [f32; 4],
    pub constraints: JointConstraints,
    pub mass: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JointConstraints {
    pub rotation_limit: [[f32; 2]; 3],
    pub translation_limit: [[f32; 2]; 3],
    pub stiffness: f32,
    pub damping: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationConstraint {
    pub constraint_type: ConstraintType,
    pub target: String,
    pub weight: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintType {
    FootPlacement,
    LookAt,
    HandTarget,
    Balance,
    Momentum,
    EnvironmentContact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationResult {
    pub animation_type: AnimationType,
    pub duration_seconds: f32,
    pub frame_count: u32,
    pub fps: u32,
    pub keyframes: Vec<Keyframe>,
    pub metadata: AnimationMetadata,
    pub root_motion: Vec<[f32; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyframe {
    pub time: f32,
    pub joint_poses: Vec<JointPose>,
    pub events: Vec<AnimationEvent>,
    pub blend_weight: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct JointPose {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationEvent {
    pub event_type: String,
    pub time: f32,
    pub data: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationMetadata {
    pub total_keyframes: u32,
    pub total_duration_ms: u64,
    pub joint_count: u32,
    pub compression_ratio: f32,
    pub memory_bytes: u64,
    pub generation_time_ms: u64,
}

pub struct AnimationGenerator {
    pub config: AnimationGenConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationGenConfig {
    pub default_fps: u32,
    pub ik_iterations: u32,
    pub ik_tolerance: f32,
    pub blend_window: f32,
    pub foot_ik_enabled: bool,
    pub motion_matching_enabled: bool,
}

impl Default for AnimationGenConfig {
    fn default() -> Self {
        AnimationGenConfig {
            default_fps: 30,
            ik_iterations: 10,
            ik_tolerance: 0.001,
            blend_window: 0.2,
            foot_ik_enabled: true,
            motion_matching_enabled: false,
        }
    }
}

impl AnimationGenerator {
    pub fn new(config: AnimationGenConfig) -> Self {
        AnimationGenerator { config }
    }

    pub fn generate(&self, request: &AnimationRequest) -> AnimationResult {
        let frame_count = (request.duration_seconds * request.fps as f32) as u32;
        let joint_count = request.skeleton.joints.len();
        let mut keyframes = Vec::with_capacity(frame_count as usize);
        let mut root_motion = Vec::with_capacity(frame_count as usize);

        for frame in 0..frame_count {
            let time = frame as f32 / request.fps as f32;
            let progress = time / request.duration_seconds;

            let mut joint_poses = Vec::with_capacity(joint_count);
            for (i, joint) in request.skeleton.joints.iter().enumerate() {
                let pose = self.sample_joint_pose(
                    i,
                    joint,
                    progress,
                    &request.animation_type,
                    &request.style,
                    &request.constraints,
                );
                joint_poses.push(pose);
            }

            root_motion.push(self.calculate_root_motion(
                progress,
                &request.animation_type,
                &request.style,
            ));

            let events = self.generate_frame_events(time, progress, &request.animation_type);

            keyframes.push(Keyframe { time, joint_poses, events, blend_weight: 1.0 });
        }

        let total_duration_ms = (request.duration_seconds * 1000.0) as u64;
        let memory_bytes = frame_count as u64 * joint_count as u64 * 48;
        AnimationResult {
            animation_type: request.animation_type.clone(),
            duration_seconds: request.duration_seconds,
            frame_count,
            fps: request.fps,
            keyframes,
            root_motion,
            metadata: AnimationMetadata {
                total_keyframes: frame_count,
                total_duration_ms,
                joint_count: joint_count as u32,
                compression_ratio: 1.0,
                memory_bytes,
                generation_time_ms: 1,
            },
        }
    }

    fn sample_joint_pose(
        &self,
        joint_idx: usize,
        joint: &JointDefinition,
        progress: f32,
        anim_type: &AnimationType,
        style: &AnimationStyle,
        constraints: &[AnimationConstraint],
    ) -> JointPose {
        let (tx, ty, tz, rx, ry, rz, rw) =
            Self::procedural_pose(joint_idx, joint, progress, anim_type, style);

        let mut pose =
            JointPose { translation: [tx, ty, tz], rotation: [rx, ry, rz, rw], scale: [1.0; 3] };

        for constraint in constraints {
            if constraint.target == joint.name {
                self.apply_constraint(&mut pose, constraint, progress);
            }
        }

        pose
    }

    fn procedural_pose(
        joint_idx: usize,
        _joint: &JointDefinition,
        progress: f32,
        anim_type: &AnimationType,
        style: &AnimationStyle,
    ) -> (f32, f32, f32, f32, f32, f32, f32) {
        let freq = match anim_type {
            AnimationType::Walk => 1.0,
            AnimationType::Run => 2.0,
            AnimationType::Jump => 0.5,
            _ => 1.0,
        };
        let amplitude = match style {
            AnimationStyle::Realistic => 0.3,
            AnimationStyle::Stylized => 0.6,
            AnimationStyle::Mechanical => 0.1,
            AnimationStyle::Organic => 0.4,
            AnimationStyle::Weighted => 0.2,
        };
        let phase = joint_idx as f32 * 0.3;
        let angle = (progress * std::f32::consts::TAU * freq + phase) * amplitude;
        let tx = angle.sin() * 0.1 * amplitude;
        let ty = angle.cos() * 0.05 * amplitude;
        let tz = 0.0;
        let half = (angle * 0.5).sin();
        let rx = half;
        let ry = 0.0;
        let rz = 0.0;
        let rw = (1.0 - half * half).sqrt().max(0.0);
        (tx, ty, tz, rx, ry, rz, rw)
    }

    fn apply_constraint(
        &self,
        pose: &mut JointPose,
        constraint: &AnimationConstraint,
        progress: f32,
    ) {
        match constraint.constraint_type {
            ConstraintType::FootPlacement => {
                pose.translation[1] = pose.translation[1].max(0.0);
            },
            ConstraintType::LookAt => {
                pose.rotation[1] += progress * constraint.weight * 0.2;
            },
            ConstraintType::HandTarget => {
                pose.translation[0] += constraint.weight * 0.1;
                pose.translation[1] += constraint.weight * 0.1;
            },
            ConstraintType::Balance => {
                pose.translation[0] -= constraint.weight * 0.05;
            },
            _ => {},
        }
    }

    fn calculate_root_motion(
        &self,
        progress: f32,
        anim_type: &AnimationType,
        _style: &AnimationStyle,
    ) -> [f32; 3] {
        match anim_type {
            AnimationType::Walk => [progress * 1.5, 0.0, 0.0],
            AnimationType::Run => [progress * 4.0, 0.0, 0.0],
            AnimationType::Jump => [0.0, (progress * std::f32::consts::PI).sin() * 2.0, 0.0],
            _ => [0.0; 3],
        }
    }

    fn generate_frame_events(
        &self,
        time: f32,
        progress: f32,
        anim_type: &AnimationType,
    ) -> Vec<AnimationEvent> {
        let mut events = Vec::new();
        match anim_type {
            AnimationType::Walk | AnimationType::Run if (progress * 2.0).fract() < 0.05 => {
                let mut data = HashMap::new();
                data.insert(
                    "foot".into(),
                    if progress.fract() < 0.5 { "left".into() } else { "right".into() },
                );
                data.insert("type".into(), "footstep".into());
                events.push(AnimationEvent { event_type: "footstep".into(), time, data });
            },
            AnimationType::Jump if progress > 0.35 && progress < 0.55 => {
                let mut data = HashMap::new();
                data.insert("phase".into(), "apex".into());
                events.push(AnimationEvent { event_type: "jump_apex".into(), time, data });
            },
            AnimationType::Attack if progress > 0.3 && progress < 0.35 => {
                let mut data = HashMap::new();
                data.insert("phase".into(), "impact".into());
                events.push(AnimationEvent { event_type: "attack_impact".into(), time, data });
            },
            _ => {},
        }
        events
    }

    pub fn blend_animations(
        &self,
        from: &AnimationResult,
        to: &AnimationResult,
        blend_time: f32,
    ) -> AnimationResult {
        let blend_frames = (blend_time * self.config.default_fps as f32) as u32;
        let total_frames = from.frame_count + to.frame_count.saturating_sub(blend_frames);
        let mut keyframes = Vec::with_capacity(total_frames as usize);

        for kf in &from.keyframes {
            keyframes.push(kf.clone());
        }

        let blend_start = from.frame_count.saturating_sub(blend_frames);
        for i in 0..blend_frames.min(to.frame_count) {
            let t = i as f32 / blend_frames as f32;
            let from_idx = (blend_start + i) as usize;
            let to_idx = i as usize;

            if from_idx < from.keyframes.len() && to_idx < to.keyframes.len() {
                let from_kf = &from.keyframes[from_idx];
                let to_kf = &to.keyframes[to_idx];
                let blended_poses: Vec<JointPose> = from_kf
                    .joint_poses
                    .iter()
                    .zip(to_kf.joint_poses.iter())
                    .map(|(fp, tp)| JointPose {
                        translation: [
                            fp.translation[0] + (tp.translation[0] - fp.translation[0]) * t,
                            fp.translation[1] + (tp.translation[1] - fp.translation[1]) * t,
                            fp.translation[2] + (tp.translation[2] - fp.translation[2]) * t,
                        ],
                        rotation: Self::slerp(fp.rotation, tp.rotation, t),
                        scale: [
                            fp.scale[0] + (tp.scale[0] - fp.scale[0]) * t,
                            fp.scale[1] + (tp.scale[1] - fp.scale[1]) * t,
                            fp.scale[2] + (tp.scale[2] - fp.scale[2]) * t,
                        ],
                    })
                    .collect();

                keyframes.push(Keyframe {
                    time: from_kf.time + blend_time * t,
                    joint_poses: blended_poses,
                    events: vec![],
                    blend_weight: 1.0 - t,
                });
            }
        }

        for i in blend_frames as usize..to.keyframes.len() {
            keyframes.push(to.keyframes[i].clone());
        }

        let joint_count = from.metadata.joint_count;
        AnimationResult {
            animation_type: to.animation_type.clone(),
            duration_seconds: from.duration_seconds + to.duration_seconds - blend_time,
            frame_count: keyframes.len() as u32,
            fps: self.config.default_fps,
            keyframes,
            root_motion: from.root_motion.clone(),
            metadata: AnimationMetadata {
                total_keyframes: total_frames,
                total_duration_ms: ((from.duration_seconds + to.duration_seconds - blend_time)
                    * 1000.0) as u64,
                joint_count,
                compression_ratio: 1.0,
                memory_bytes: (total_frames as u64 * joint_count as u64 * 48),
                generation_time_ms: 1,
            },
        }
    }

    fn slerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
        let mut dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
        let mut b = b;
        if dot < 0.0 {
            b = [-b[0], -b[1], -b[2], -b[3]];
            dot = -dot;
        }
        if dot > 0.9995 {
            let result = [
                a[0] + t * (b[0] - a[0]),
                a[1] + t * (b[1] - a[1]),
                a[2] + t * (b[2] - a[2]),
                a[3] + t * (b[3] - a[3]),
            ];
            let len =
                (result[0].powi(2) + result[1].powi(2) + result[2].powi(2) + result[3].powi(2))
                    .sqrt();
            return [result[0] / len, result[1] / len, result[2] / len, result[3] / len];
        }
        let theta_0 = dot.acos();
        let theta = theta_0 * t;
        let sin_theta = theta.sin();
        let sin_theta_0 = theta_0.sin();
        let s0 = (theta_0 - theta).cos() - dot * sin_theta / sin_theta_0;
        let s1 = sin_theta / sin_theta_0;
        [s0 * a[0] + s1 * b[0], s0 * a[1] + s1 * b[1], s0 * a[2] + s1 * b[2], s0 * a[3] + s1 * b[3]]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IkSolver {
    pub max_iterations: u32,
    pub tolerance: f32,
    pub damping: f32,
}

impl Default for IkSolver {
    fn default() -> Self {
        IkSolver { max_iterations: 10, tolerance: 0.001, damping: 0.5 }
    }
}

impl IkSolver {
    pub fn solve_ccd(
        &self,
        joints: &[JointDefinition],
        target: [f32; 3],
        effector_idx: usize,
    ) -> Vec<JointPose> {
        let mut poses: Vec<JointPose> = joints
            .iter()
            .map(|j| JointPose {
                translation: j.bind_position,
                rotation: j.bind_rotation,
                scale: [1.0; 3],
            })
            .collect();

        for _ in 0..self.max_iterations {
            let effector_pos = Self::forward_kinematics_point(&poses, joints, effector_idx);
            let error = [
                target[0] - effector_pos[0],
                target[1] - effector_pos[1],
                target[2] - effector_pos[2],
            ];
            let error_mag = (error[0].powi(2) + error[1].powi(2) + error[2].powi(2)).sqrt();
            if error_mag < self.tolerance {
                break;
            }

            let mut current = effector_idx;
            loop {
                let joint_pos = Self::forward_kinematics_point(&poses, joints, current);
                let to_effector = [
                    effector_pos[0] - joint_pos[0],
                    effector_pos[1] - joint_pos[1],
                    effector_pos[2] - joint_pos[2],
                ];
                let to_target =
                    [target[0] - joint_pos[0], target[1] - joint_pos[1], target[2] - joint_pos[2]];
                let to_eff_len =
                    (to_effector[0].powi(2) + to_effector[1].powi(2) + to_effector[2].powi(2))
                        .sqrt();
                let to_tgt_len =
                    (to_target[0].powi(2) + to_target[1].powi(2) + to_target[2].powi(2)).sqrt();
                if to_eff_len < 1e-8 || to_tgt_len < 1e-8 {
                    if let Some(parent) = joints[current].parent {
                        current = parent;
                        continue;
                    }
                    break;
                }
                let to_eff_norm = [
                    to_effector[0] / to_eff_len,
                    to_effector[1] / to_eff_len,
                    to_effector[2] / to_eff_len,
                ];
                let to_tgt_norm = [
                    to_target[0] / to_tgt_len,
                    to_target[1] / to_tgt_len,
                    to_target[2] / to_tgt_len,
                ];
                let cos_angle = (to_eff_norm[0] * to_tgt_norm[0]
                    + to_eff_norm[1] * to_tgt_norm[1]
                    + to_eff_norm[2] * to_tgt_norm[2])
                    .clamp(-1.0, 1.0);
                let angle = cos_angle.acos() * self.damping;
                let axis = [
                    to_eff_norm[1] * to_tgt_norm[2] - to_eff_norm[2] * to_tgt_norm[1],
                    to_eff_norm[2] * to_tgt_norm[0] - to_eff_norm[0] * to_tgt_norm[2],
                    to_eff_norm[0] * to_tgt_norm[1] - to_eff_norm[1] * to_tgt_norm[0],
                ];
                let axis_len = (axis[0].powi(2) + axis[1].powi(2) + axis[2].powi(2)).sqrt();
                if axis_len > 1e-8 {
                    let axis_norm = [axis[0] / axis_len, axis[1] / axis_len, axis[2] / axis_len];
                    let rot = Self::axis_angle_to_quat(axis_norm, angle);
                    let existing = poses[current].rotation;
                    poses[current].rotation = Self::quat_mul(rot, existing);
                }
                if let Some(parent) = joints[current].parent {
                    let _effector_pos =
                        Self::forward_kinematics_point(&poses, joints, effector_idx);
                    current = parent;
                } else {
                    break;
                }
            }
        }
        poses
    }

    fn forward_kinematics_point(
        poses: &[JointPose],
        joints: &[JointDefinition],
        idx: usize,
    ) -> [f32; 3] {
        let mut pos = [0.0f32; 3];
        let mut current = idx;
        loop {
            pos[0] += poses[current].translation[0];
            pos[1] += poses[current].translation[1];
            pos[2] += poses[current].translation[2];
            if let Some(parent) = joints[current].parent {
                current = parent;
            } else {
                break;
            }
        }
        pos
    }

    fn axis_angle_to_quat(axis: [f32; 3], angle: f32) -> [f32; 4] {
        let half = angle * 0.5;
        let s = half.sin();
        [axis[0] * s, axis[1] * s, axis[2] * s, half.cos()]
    }

    fn quat_mul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
        [
            a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
            a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
            a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
            a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionSynthesisConfig {
    pub motion_graph: bool,
    pub physics_driven: bool,
    pub neural_blend: bool,
    pub footstep_detection: bool,
}

impl Default for MotionSynthesisConfig {
    fn default() -> Self {
        MotionSynthesisConfig {
            motion_graph: false,
            physics_driven: true,
            neural_blend: false,
            footstep_detection: true,
        }
    }
}

pub struct MotionSynthesizer {
    pub config: MotionSynthesisConfig,
    pub ik_solver: IkSolver,
    pub animation_gen: AnimationGenerator,
    pub motion_library: HashMap<String, AnimationResult>,
}

impl Default for MotionSynthesizer {
    fn default() -> Self {
        Self::new()
    }
}

impl MotionSynthesizer {
    pub fn new() -> Self {
        MotionSynthesizer {
            config: MotionSynthesisConfig::default(),
            ik_solver: IkSolver::default(),
            animation_gen: AnimationGenerator::new(AnimationGenConfig::default()),
            motion_library: HashMap::new(),
        }
    }

    pub fn synthesize(
        &mut self,
        skeleton: &SkeletonDefinition,
        target: AnimationType,
        duration: f32,
    ) -> AnimationResult {
        if let Some(cached) = self.motion_library.values().find(|a| a.animation_type == target) {
            return cached.clone();
        }
        let request = AnimationRequest {
            animation_type: target.clone(),
            skeleton: skeleton.clone(),
            duration_seconds: duration,
            fps: 30,
            constraints: self.default_constraints(&target),
            style: AnimationStyle::Realistic,
            seed: None,
        };
        let result = self.animation_gen.generate(&request);
        self.motion_library.insert(format!("{:?}", target), result.clone());
        result
    }

    fn default_constraints(&self, anim_type: &AnimationType) -> Vec<AnimationConstraint> {
        match anim_type {
            AnimationType::Walk | AnimationType::Run => vec![
                AnimationConstraint {
                    constraint_type: ConstraintType::FootPlacement,
                    target: "foot_l".into(),
                    weight: 1.0,
                },
                AnimationConstraint {
                    constraint_type: ConstraintType::FootPlacement,
                    target: "foot_r".into(),
                    weight: 1.0,
                },
                AnimationConstraint {
                    constraint_type: ConstraintType::Balance,
                    target: "spine".into(),
                    weight: 0.5,
                },
            ],
            AnimationType::Climb => vec![
                AnimationConstraint {
                    constraint_type: ConstraintType::HandTarget,
                    target: "hand_l".into(),
                    weight: 1.0,
                },
                AnimationConstraint {
                    constraint_type: ConstraintType::HandTarget,
                    target: "hand_r".into(),
                    weight: 1.0,
                },
                AnimationConstraint {
                    constraint_type: ConstraintType::FootPlacement,
                    target: "foot_l".into(),
                    weight: 0.8,
                },
                AnimationConstraint {
                    constraint_type: ConstraintType::FootPlacement,
                    target: "foot_r".into(),
                    weight: 0.8,
                },
            ],
            _ => vec![],
        }
    }

    pub fn library_size(&self) -> usize {
        self.motion_library.len()
    }

    pub fn clear_library(&mut self) {
        self.motion_library.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_skeleton() -> SkeletonDefinition {
        SkeletonDefinition {
            name: "humanoid".into(),
            joints: vec![
                JointDefinition {
                    name: "root".into(),
                    parent: None,
                    bind_position: [0.0, 0.0, 0.0],
                    bind_rotation: [0.0, 0.0, 0.0, 1.0],
                    constraints: JointConstraints {
                        rotation_limit: [[0.0; 2]; 3],
                        translation_limit: [[0.0; 2]; 3],
                        stiffness: 1.0,
                        damping: 0.5,
                    },
                    mass: 1.0,
                },
                JointDefinition {
                    name: "spine".into(),
                    parent: Some(0),
                    bind_position: [0.0, 0.5, 0.0],
                    bind_rotation: [0.0, 0.0, 0.0, 1.0],
                    constraints: JointConstraints {
                        rotation_limit: [[-0.5, 0.5], [-0.3, 0.3], [-0.3, 0.3]],
                        translation_limit: [[0.0; 2]; 3],
                        stiffness: 1.0,
                        damping: 0.5,
                    },
                    mass: 2.0,
                },
                JointDefinition {
                    name: "foot_l".into(),
                    parent: Some(1),
                    bind_position: [-0.2, -0.5, 0.0],
                    bind_rotation: [0.0, 0.0, 0.0, 1.0],
                    constraints: JointConstraints {
                        rotation_limit: [[-0.8, 0.8], [-0.2, 0.2], [-0.2, 0.2]],
                        translation_limit: [[0.0; 2]; 3],
                        stiffness: 1.0,
                        damping: 0.5,
                    },
                    mass: 0.5,
                },
                JointDefinition {
                    name: "foot_r".into(),
                    parent: Some(1),
                    bind_position: [0.2, -0.5, 0.0],
                    bind_rotation: [0.0, 0.0, 0.0, 1.0],
                    constraints: JointConstraints {
                        rotation_limit: [[-0.8, 0.8], [-0.2, 0.2], [-0.2, 0.2]],
                        translation_limit: [[0.0; 2]; 3],
                        stiffness: 1.0,
                        damping: 0.5,
                    },
                    mass: 0.5,
                },
                JointDefinition {
                    name: "hand_l".into(),
                    parent: Some(1),
                    bind_position: [-0.3, 0.8, 0.0],
                    bind_rotation: [0.0, 0.0, 0.0, 1.0],
                    constraints: JointConstraints {
                        rotation_limit: [[-1.5, 1.5], [-1.0, 1.0], [-1.0, 1.0]],
                        translation_limit: [[0.0; 2]; 3],
                        stiffness: 1.0,
                        damping: 0.5,
                    },
                    mass: 0.3,
                },
                JointDefinition {
                    name: "hand_r".into(),
                    parent: Some(1),
                    bind_position: [0.3, 0.8, 0.0],
                    bind_rotation: [0.0, 0.0, 0.0, 1.0],
                    constraints: JointConstraints {
                        rotation_limit: [[-1.5, 1.5], [-1.0, 1.0], [-1.0, 1.0]],
                        translation_limit: [[0.0; 2]; 3],
                        stiffness: 1.0,
                        damping: 0.5,
                    },
                    mass: 0.3,
                },
            ],
            root_joint: 0,
            total_bones: 6,
        }
    }

    #[test]
    fn test_generate_walk() {
        let gen = AnimationGenerator::new(AnimationGenConfig::default());
        let skeleton = make_test_skeleton();
        let request = AnimationRequest {
            animation_type: AnimationType::Walk,
            skeleton,
            duration_seconds: 1.0,
            fps: 30,
            constraints: vec![],
            style: AnimationStyle::Realistic,
            seed: None,
        };
        let result = gen.generate(&request);
        assert_eq!(result.frame_count, 30);
        assert_eq!(result.keyframes.len(), 30);
        assert!(!result.keyframes[0].joint_poses.is_empty());
        assert!(result.root_motion.len() == 30);
        assert!(result.root_motion[29][0] > 0.0);
    }

    #[test]
    fn test_generate_jump() {
        let gen = AnimationGenerator::new(AnimationGenConfig::default());
        let skeleton = make_test_skeleton();
        let request = AnimationRequest {
            animation_type: AnimationType::Jump,
            skeleton,
            duration_seconds: 0.5,
            fps: 30,
            constraints: vec![],
            style: AnimationStyle::Realistic,
            seed: None,
        };
        let result = gen.generate(&request);
        assert_eq!(result.frame_count, 15);
        let has_apex =
            result.keyframes.iter().any(|kf| kf.events.iter().any(|e| e.event_type == "jump_apex"));
        assert!(has_apex);
    }

    #[test]
    fn test_generate_attack() {
        let gen = AnimationGenerator::new(AnimationGenConfig::default());
        let skeleton = make_test_skeleton();
        let request = AnimationRequest {
            animation_type: AnimationType::Attack,
            skeleton,
            duration_seconds: 0.8,
            fps: 30,
            constraints: vec![],
            style: AnimationStyle::Weighted,
            seed: None,
        };
        let result = gen.generate(&request);
        let has_impact = result
            .keyframes
            .iter()
            .any(|kf| kf.events.iter().any(|e| e.event_type == "attack_impact"));
        assert!(has_impact);
    }

    #[test]
    fn test_footstep_constraints() {
        let gen = AnimationGenerator::new(AnimationGenConfig::default());
        let skeleton = make_test_skeleton();
        let constraints = vec![
            AnimationConstraint {
                constraint_type: ConstraintType::FootPlacement,
                target: "foot_l".into(),
                weight: 1.0,
            },
            AnimationConstraint {
                constraint_type: ConstraintType::FootPlacement,
                target: "foot_r".into(),
                weight: 1.0,
            },
        ];
        let request = AnimationRequest {
            animation_type: AnimationType::Walk,
            skeleton,
            duration_seconds: 1.0,
            fps: 30,
            constraints,
            style: AnimationStyle::Realistic,
            seed: None,
        };
        let result = gen.generate(&request);
        let has_footsteps =
            result.keyframes.iter().any(|kf| kf.events.iter().any(|e| e.event_type == "footstep"));
        assert!(has_footsteps);
    }

    #[test]
    fn test_blend_animations() {
        let gen = AnimationGenerator::new(AnimationGenConfig::default());
        let skeleton = make_test_skeleton();
        let walk_req = AnimationRequest {
            animation_type: AnimationType::Walk,
            skeleton: skeleton.clone(),
            duration_seconds: 1.0,
            fps: 30,
            constraints: vec![],
            style: AnimationStyle::Realistic,
            seed: None,
        };
        let run_req = AnimationRequest {
            animation_type: AnimationType::Run,
            skeleton,
            duration_seconds: 1.0,
            fps: 30,
            constraints: vec![],
            style: AnimationStyle::Realistic,
            seed: None,
        };
        let walk = gen.generate(&walk_req);
        let run = gen.generate(&run_req);
        let blended = gen.blend_animations(&walk, &run, 0.3);
        assert!(blended.frame_count > 30);
        assert!(blended.frame_count <= 60);
    }

    #[test]
    fn test_ik_solver_ccd() {
        let solver = IkSolver::default();
        let joints = vec![
            JointDefinition {
                name: "root".into(),
                parent: None,
                bind_position: [0.0, 0.0, 0.0],
                bind_rotation: [0.0, 0.0, 0.0, 1.0],
                constraints: JointConstraints {
                    rotation_limit: [[0.0; 2]; 3],
                    translation_limit: [[0.0; 2]; 3],
                    stiffness: 1.0,
                    damping: 0.5,
                },
                mass: 1.0,
            },
            JointDefinition {
                name: "elbow".into(),
                parent: Some(0),
                bind_position: [0.0, 1.0, 0.0],
                bind_rotation: [0.0, 0.0, 0.0, 1.0],
                constraints: JointConstraints {
                    rotation_limit: [[0.0; 2]; 3],
                    translation_limit: [[0.0; 2]; 3],
                    stiffness: 1.0,
                    damping: 0.5,
                },
                mass: 0.5,
            },
            JointDefinition {
                name: "hand".into(),
                parent: Some(1),
                bind_position: [0.0, 1.0, 0.0],
                bind_rotation: [0.0, 0.0, 0.0, 1.0],
                constraints: JointConstraints {
                    rotation_limit: [[0.0; 2]; 3],
                    translation_limit: [[0.0; 2]; 3],
                    stiffness: 1.0,
                    damping: 0.5,
                },
                mass: 0.2,
            },
        ];
        let target = [1.0, 1.0, 0.0];
        let poses = solver.solve_ccd(&joints, target, 2);
        assert_eq!(poses.len(), 3);
        for pose in &poses {
            assert!(pose.rotation[3].is_finite());
        }
    }

    #[test]
    fn test_motion_synthesizer() {
        let mut synth = MotionSynthesizer::new();
        let skeleton = make_test_skeleton();
        let result = synth.synthesize(&skeleton, AnimationType::Walk, 1.0);
        assert_eq!(result.animation_type, AnimationType::Walk);
        assert_eq!(synth.library_size(), 1);
        let result2 = synth.synthesize(&skeleton, AnimationType::Walk, 1.0);
        assert_eq!(result.frame_count, result2.frame_count);
        assert_eq!(synth.library_size(), 1);
    }

    #[test]
    fn test_slerp_identity() {
        let a = [0.0, 0.0, 0.0, 1.0];
        let b = [0.0, 0.0, 0.0, 1.0];
        let result = AnimationGenerator::slerp(a, b, 0.5);
        let diff = (result[0] - a[0]).abs()
            + (result[1] - a[1]).abs()
            + (result[2] - a[2]).abs()
            + (result[3] - a[3]).abs();
        assert!(diff < 0.001);
    }
}
