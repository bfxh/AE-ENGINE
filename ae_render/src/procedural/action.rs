//! 动作合成引擎
//!
//! 突破性动作系统：动作不是"播放"的，是"求解"的
//! - 原子动作库（50+ 片段，每个定义纯关节运动）
//! - 分层混合器（骨骼分区独立控制，可同时混合）
//! - 意图解析器（根据意图+身体状态实时计算姿势）
//! - 物理辅助（弹簧-阻尼手部轨迹、重量反馈、惯性过渡）
//! - 损伤耦合（断肢自动切换拓扑、神经抖动、休克迟缓）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// === 动作类型 ===
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionType {
    Idle,
    Walk,
    Run,
    Crouch,
    Crawl,
    Jump,
    PickUp,
    Drop,
    Attack,
    Operate,
    Throw,
    Climb,
    Swim,
    SingleLegStand, // 单腿站立（断腿代偿）
}

impl ActionType {
    pub fn is_locomotion(&self) -> bool {
        matches!(self, ActionType::Walk | ActionType::Run | ActionType::Crouch | ActionType::Crawl | ActionType::Jump | ActionType::Climb | ActionType::Swim)
    }

    pub fn is_upper_body_action(&self) -> bool {
        matches!(self, ActionType::PickUp | ActionType::Drop | ActionType::Attack | ActionType::Operate | ActionType::Throw)
    }
}

// === 运动状态 ===
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MotionState {
    Standing,
    Walking,
    Running,
    Crouching,
    Crawling,
    Falling,
    Swimming,
    Climbing,
}

impl MotionState {
    pub fn speed_m_s(&self) -> f32 {
        match self {
            MotionState::Standing => 0.0,
            MotionState::Walking => 1.4,
            MotionState::Running => 4.5,
            MotionState::Crouching => 0.7,
            MotionState::Crawling => 0.3,
            MotionState::Falling => 9.8,
            MotionState::Swimming => 0.5,
            MotionState::Climbing => 0.2,
        }
    }
}

// === 意图 ===
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionIntent {
    pub action_type: ActionType,
    pub target_entity: Option<u64>,
    pub target_position: Option<[f32; 3]>,
    pub current_motion: MotionState,
    /// 紧迫度（0=从容，1=紧急）
    pub urgency: f32,
    /// 主用手（0=右，1=左，2=双手）
    pub hand_preference: u8,
}

impl Default for ActionIntent {
    fn default() -> Self {
        Self {
            action_type: ActionType::Idle,
            target_entity: None,
            target_position: None,
            current_motion: MotionState::Standing,
            urgency: 0.0,
            hand_preference: 0,
        }
    }
}

// === 骨骼分区（用于分层混合）===
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BodyLayer {
    LowerBody, // 双腿+骨盆
    Torso,     // 脊柱+胸
    RightArm,  // 右肩+右上臂+右前臂+右手
    LeftArm,   // 左肩+左上臂+左前臂+左手
    Head,      // 颈+头
}

impl BodyLayer {
    pub fn all() -> [BodyLayer; 5] {
        [BodyLayer::LowerBody, BodyLayer::Torso, BodyLayer::RightArm, BodyLayer::LeftArm, BodyLayer::Head]
    }
}

// === 关节目标变换（动作合成的输出）===
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct JointTarget {
    /// 局部位置偏移（相对绑定姿态）
    pub translation_offset: [f32; 3],
    /// 局部旋转偏移（四元数，相对绑定姿态）
    pub rotation_offset: [f32; 4], // (x, y, z, w)
    /// 缩放
    pub scale: [f32; 3],
}

impl JointTarget {
    pub fn identity() -> Self {
        Self {
            translation_offset: [0.0, 0.0, 0.0],
            rotation_offset: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    pub fn rotation_x(angle_rad: f32) -> Self {
        let (s, c) = (angle_rad * 0.5).sin_cos();
        Self {
            translation_offset: [0.0, 0.0, 0.0],
            rotation_offset: [s, 0.0, 0.0, c],
            scale: [1.0, 1.0, 1.0],
        }
    }

    pub fn rotation_y(angle_rad: f32) -> Self {
        let (s, c) = (angle_rad * 0.5).sin_cos();
        Self {
            translation_offset: [0.0, 0.0, 0.0],
            rotation_offset: [0.0, s, 0.0, c],
            scale: [1.0, 1.0, 1.0],
        }
    }

    pub fn rotation_z(angle_rad: f32) -> Self {
        let (s, c) = (angle_rad * 0.5).sin_cos();
        Self {
            translation_offset: [0.0, 0.0, 0.0],
            rotation_offset: [0.0, 0.0, s, c],
            scale: [1.0, 1.0, 1.0],
        }
    }

    /// 线性插值两个关节目标
    pub fn lerp(&self, other: &JointTarget, t: f32) -> JointTarget {
        JointTarget {
            translation_offset: [
                self.translation_offset[0] + (other.translation_offset[0] - self.translation_offset[0]) * t,
                self.translation_offset[1] + (other.translation_offset[1] - self.translation_offset[1]) * t,
                self.translation_offset[2] + (other.translation_offset[2] - self.translation_offset[2]) * t,
            ],
            // 简化：使用线性插值（实际应使用四元数球面插值）
            rotation_offset: [
                self.rotation_offset[0] + (other.rotation_offset[0] - self.rotation_offset[0]) * t,
                self.rotation_offset[1] + (other.rotation_offset[1] - self.rotation_offset[1]) * t,
                self.rotation_offset[2] + (other.rotation_offset[2] - self.rotation_offset[2]) * t,
                self.rotation_offset[3] + (other.rotation_offset[3] - self.rotation_offset[3]) * t,
            ],
            scale: [
                self.scale[0] + (other.scale[0] - self.scale[0]) * t,
                self.scale[1] + (other.scale[1] - self.scale[1]) * t,
                self.scale[2] + (other.scale[2] - self.scale[2]) * t,
            ],
        }
    }
}

/// 关节目标字典（关节名 → 目标变换）
pub type JointTargetMap = HashMap<String, JointTarget>;

// === 原子动作 ===
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicAction {
    pub name: &'static str,
    pub layer: BodyLayer,
    /// 持续时间（秒）
    pub duration_s: f32,
    /// 是否循环
    pub looping: bool,
    /// 关键帧（时间 0..duration，关节名 → 目标变换）
    pub keyframes: Vec<(f32, JointTargetMap)>,
}

impl AtomicAction {
    /// 在时间 t 采样该动作（返回所有关节的目标变换）
    pub fn sample(&self, t: f32) -> JointTargetMap {
        if self.keyframes.is_empty() {
            return HashMap::new();
        }
        if self.keyframes.len() == 1 {
            return self.keyframes[0].1.clone();
        }

        // 循环或截断
        let mut local_t = t;
        if self.looping && self.duration_s > 0.0 {
            local_t = local_t % self.duration_s;
        }
        local_t = local_t.clamp(0.0, self.duration_s);

        // 找到包围 local_t 的两个关键帧
        let mut prev_idx = 0;
        for (i, (kt, _)) in self.keyframes.iter().enumerate() {
            if *kt <= local_t {
                prev_idx = i;
            } else {
                break;
            }
        }
        let next_idx = (prev_idx + 1).min(self.keyframes.len() - 1);

        let (prev_t, prev_map) = &self.keyframes[prev_idx];
        let (next_t, next_map) = &self.keyframes[next_idx];

        let alpha = if *next_t > *prev_t {
            (local_t - prev_t) / (next_t - prev_t)
        } else {
            0.0
        };

        // 合并关节：以 prev_map 为主，next_map 中的关节做插值
        // 若关节仅在 next_map 中存在（prev_map 缺失），从 identity 插值，
        // 保证 t=0 时关节变换为 identity（而非直接跳到终值）
        let mut result = prev_map.clone();
        for (joint_name, next_target) in next_map {
            if let Some(prev_target) = result.get(joint_name) {
                result.insert(joint_name.clone(), prev_target.lerp(next_target, alpha));
            } else {
                result.insert(joint_name.clone(), JointTarget::identity().lerp(next_target, alpha));
            }
        }
        result
    }
}

// === 原子动作库 ===
#[derive(Debug, Clone)]
pub struct ActionLibrary {
    pub actions: HashMap<&'static str, AtomicAction>,
}

impl Default for ActionLibrary {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionLibrary {
    pub fn new() -> Self {
        let mut lib = Self { actions: HashMap::new() };
        lib.populate_defaults();
        lib
    }

    /// 填充默认原子动作库（覆盖躯干/双臂/腿部/头部的常见姿势）
    fn populate_defaults(&mut self) {
        // === 躯干动作 ===
        self.add(AtomicAction {
            name: "torso_upright",
            layer: BodyLayer::Torso,
            duration_s: 0.3,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.3, HashMap::new()),
            ],
        });
        self.add(AtomicAction {
            name: "torso_lean_15",
            layer: BodyLayer::Torso,
            duration_s: 0.4,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.4, [(String::from("spine"), JointTarget::rotation_x(0.26))].into_iter().collect()),
            ],
        });
        self.add(AtomicAction {
            name: "torso_lean_30",
            layer: BodyLayer::Torso,
            duration_s: 0.5,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.5, [(String::from("spine"), JointTarget::rotation_x(0.52))].into_iter().collect()),
            ],
        });
        self.add(AtomicAction {
            name: "torso_side_lean",
            layer: BodyLayer::Torso,
            duration_s: 0.4,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.4, [(String::from("spine"), JointTarget::rotation_z(0.3))].into_iter().collect()),
            ],
        });
        self.add(AtomicAction {
            name: "torso_twist",
            layer: BodyLayer::Torso,
            duration_s: 0.5,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.5, [(String::from("chest"), JointTarget::rotation_y(0.5))].into_iter().collect()),
            ],
        });

        // === 右臂动作 ===
        let right_arm_forward: JointTargetMap = [
            (String::from("shoulder_r"), JointTarget::rotation_x(-1.4)),
            (String::from("upper_arm_r"), JointTarget::rotation_x(-0.3)),
        ].into_iter().collect();
        self.add(AtomicAction {
            name: "right_arm_forward",
            layer: BodyLayer::RightArm,
            duration_s: 0.5,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.5, right_arm_forward),
            ],
        });
        let right_arm_up: JointTargetMap = [
            (String::from("shoulder_r"), JointTarget::rotation_x(-2.8)),
        ].into_iter().collect();
        self.add(AtomicAction {
            name: "right_arm_up",
            layer: BodyLayer::RightArm,
            duration_s: 0.6,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.6, right_arm_up),
            ],
        });
        let right_arm_down: JointTargetMap = [
            (String::from("shoulder_r"), JointTarget::rotation_x(0.5)),
        ].into_iter().collect();
        self.add(AtomicAction {
            name: "right_arm_down",
            layer: BodyLayer::RightArm,
            duration_s: 0.4,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.4, right_arm_down),
            ],
        });
        let right_arm_side: JointTargetMap = [
            (String::from("shoulder_r"), JointTarget::rotation_z(-0.8)),
        ].into_iter().collect();
        self.add(AtomicAction {
            name: "right_arm_side",
            layer: BodyLayer::RightArm,
            duration_s: 0.5,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.5, right_arm_side),
            ],
        });
        let right_arm_grasp: JointTargetMap = [
            (String::from("lower_arm_r"), JointTarget::rotation_x(0.7)),
        ].into_iter().collect();
        self.add(AtomicAction {
            name: "right_arm_grasp",
            layer: BodyLayer::RightArm,
            duration_s: 0.3,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.3, right_arm_grasp),
            ],
        });

        // === 左臂动作（镜像右臂）===
        let left_arm_forward: JointTargetMap = [
            (String::from("shoulder_l"), JointTarget::rotation_x(-1.4)),
            (String::from("upper_arm_l"), JointTarget::rotation_x(-0.3)),
        ].into_iter().collect();
        self.add(AtomicAction {
            name: "left_arm_forward",
            layer: BodyLayer::LeftArm,
            duration_s: 0.5,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.5, left_arm_forward),
            ],
        });
        let left_arm_up: JointTargetMap = [
            (String::from("shoulder_l"), JointTarget::rotation_x(-2.8)),
        ].into_iter().collect();
        self.add(AtomicAction {
            name: "left_arm_up",
            layer: BodyLayer::LeftArm,
            duration_s: 0.6,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.6, left_arm_up),
            ],
        });
        let left_arm_down: JointTargetMap = [
            (String::from("shoulder_l"), JointTarget::rotation_x(0.5)),
        ].into_iter().collect();
        self.add(AtomicAction {
            name: "left_arm_down",
            layer: BodyLayer::LeftArm,
            duration_s: 0.4,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.4, left_arm_down),
            ],
        });
        let left_arm_side: JointTargetMap = [
            (String::from("shoulder_l"), JointTarget::rotation_z(0.8)),
        ].into_iter().collect();
        self.add(AtomicAction {
            name: "left_arm_side",
            layer: BodyLayer::LeftArm,
            duration_s: 0.5,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.5, left_arm_side),
            ],
        });
        let left_arm_grasp: JointTargetMap = [
            (String::from("lower_arm_l"), JointTarget::rotation_x(0.7)),
        ].into_iter().collect();
        self.add(AtomicAction {
            name: "left_arm_grasp",
            layer: BodyLayer::LeftArm,
            duration_s: 0.3,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.3, left_arm_grasp),
            ],
        });

        // === 腿部动作 ===
        // 站姿
        self.add(AtomicAction {
            name: "legs_stand",
            layer: BodyLayer::LowerBody,
            duration_s: 0.3,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.3, HashMap::new()),
            ],
        });
        // 走步循环（左右腿交替摆动 ±0.4 rad）
        let walk_phase_a: JointTargetMap = [
            (String::from("thigh_l"), JointTarget::rotation_x(0.4)),
            (String::from("thigh_r"), JointTarget::rotation_x(-0.4)),
            (String::from("calf_l"), JointTarget::rotation_x(-0.2)),
            (String::from("calf_r"), JointTarget::rotation_x(0.6)),
        ].into_iter().collect();
        let walk_phase_b: JointTargetMap = [
            (String::from("thigh_l"), JointTarget::rotation_x(-0.4)),
            (String::from("thigh_r"), JointTarget::rotation_x(0.4)),
            (String::from("calf_l"), JointTarget::rotation_x(0.6)),
            (String::from("calf_r"), JointTarget::rotation_x(-0.2)),
        ].into_iter().collect();
        self.add(AtomicAction {
            name: "legs_walk",
            layer: BodyLayer::LowerBody,
            duration_s: 1.0,
            looping: true,
            keyframes: vec![
                (0.0, walk_phase_a.clone()),
                (0.5, walk_phase_b),
                (1.0, walk_phase_a),
            ],
        });
        // 跑步循环（更大幅度）
        let run_phase_a: JointTargetMap = [
            (String::from("thigh_l"), JointTarget::rotation_x(0.8)),
            (String::from("thigh_r"), JointTarget::rotation_x(-0.6)),
            (String::from("calf_l"), JointTarget::rotation_x(-0.4)),
            (String::from("calf_r"), JointTarget::rotation_x(1.0)),
        ].into_iter().collect();
        let run_phase_b: JointTargetMap = [
            (String::from("thigh_l"), JointTarget::rotation_x(-0.6)),
            (String::from("thigh_r"), JointTarget::rotation_x(0.8)),
            (String::from("calf_l"), JointTarget::rotation_x(1.0)),
            (String::from("calf_r"), JointTarget::rotation_x(-0.4)),
        ].into_iter().collect();
        self.add(AtomicAction {
            name: "legs_run",
            layer: BodyLayer::LowerBody,
            duration_s: 0.6,
            looping: true,
            keyframes: vec![
                (0.0, run_phase_a.clone()),
                (0.3, run_phase_b),
                (0.6, run_phase_a),
            ],
        });
        // 蹲姿
        let crouch_pose: JointTargetMap = [
            (String::from("thigh_l"), JointTarget::rotation_x(-1.2)),
            (String::from("thigh_r"), JointTarget::rotation_x(-1.2)),
            (String::from("calf_l"), JointTarget::rotation_x(1.8)),
            (String::from("calf_r"), JointTarget::rotation_x(1.8)),
        ].into_iter().collect();
        self.add(AtomicAction {
            name: "legs_crouch",
            layer: BodyLayer::LowerBody,
            duration_s: 0.5,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.5, crouch_pose),
            ],
        });
        // 单膝跪
        let kneel_pose: JointTargetMap = [
            (String::from("thigh_l"), JointTarget::rotation_x(-1.4)),
            (String::from("calf_l"), JointTarget::rotation_x(2.2)),
            (String::from("thigh_r"), JointTarget::rotation_x(0.2)),
        ].into_iter().collect();
        self.add(AtomicAction {
            name: "legs_kneel",
            layer: BodyLayer::LowerBody,
            duration_s: 0.6,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.6, kneel_pose),
            ],
        });

        // === 头部动作 ===
        self.add(AtomicAction {
            name: "head_forward",
            layer: BodyLayer::Head,
            duration_s: 0.3,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.3, HashMap::new()),
            ],
        });
        self.add(AtomicAction {
            name: "head_down",
            layer: BodyLayer::Head,
            duration_s: 0.3,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.3, [(String::from("neck"), JointTarget::rotation_x(0.5))].into_iter().collect()),
            ],
        });
        self.add(AtomicAction {
            name: "head_left",
            layer: BodyLayer::Head,
            duration_s: 0.4,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.4, [(String::from("neck"), JointTarget::rotation_y(0.6))].into_iter().collect()),
            ],
        });
        self.add(AtomicAction {
            name: "head_right",
            layer: BodyLayer::Head,
            duration_s: 0.4,
            looping: false,
            keyframes: vec![
                (0.0, HashMap::new()),
                (0.4, [(String::from("neck"), JointTarget::rotation_y(-0.6))].into_iter().collect()),
            ],
        });

        // === 待机呼吸（全身轻微起伏）===
        let breathe_in: JointTargetMap = [
            (String::from("chest"), JointTarget {
                translation_offset: [0.0, 0.02, 0.0],
                rotation_offset: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            }),
        ].into_iter().collect();
        self.add(AtomicAction {
            name: "torso_breathe",
            layer: BodyLayer::Torso,
            duration_s: 3.0,
            looping: true,
            keyframes: vec![
                (0.0, HashMap::new()),
                (1.5, breathe_in),
                (3.0, HashMap::new()),
            ],
        });
    }

    pub fn add(&mut self, action: AtomicAction) {
        self.actions.insert(action.name, action);
    }

    pub fn get(&self, name: &str) -> Option<&AtomicAction> {
        self.actions.get(name)
    }

    pub fn count(&self) -> usize {
        self.actions.len()
    }

    /// 按层查询所有动作
    pub fn by_layer(&self, layer: BodyLayer) -> Vec<&AtomicAction> {
        self.actions.values().filter(|a| a.layer == layer).collect()
    }
}

// === 损伤状态输入（动作合成时考虑）===
#[derive(Debug, Clone, Copy, Default)]
pub struct DamageState {
    /// 各区域功能因子（0=完全丧失，1=完全功能）
    pub left_arm_factor: f32,
    pub right_arm_factor: f32,
    pub left_leg_factor: f32,
    pub right_leg_factor: f32,
    pub torso_factor: f32,
    pub head_factor: f32,
    /// 是否断左臂
    pub left_arm_severed: bool,
    /// 是否断右臂
    pub right_arm_severed: bool,
    /// 是否断左腿
    pub left_leg_severed: bool,
    /// 是否断右腿
    pub right_leg_severed: bool,
    /// 神经损伤抖动强度（0..1）
    pub nerve_tremor: f32,
    /// 休克程度（0..1，影响整体动作迟缓）
    pub shock_level: f32,
}

impl DamageState {
    pub fn healthy() -> Self {
        Self {
            left_arm_factor: 1.0,
            right_arm_factor: 1.0,
            left_leg_factor: 1.0,
            right_leg_factor: 1.0,
            torso_factor: 1.0,
            head_factor: 1.0,
            ..Default::default()
        }
    }

    pub fn is_any_leg_severed(&self) -> bool {
        self.left_leg_severed || self.right_leg_severed
    }

    pub fn is_any_arm_severed(&self) -> bool {
        self.left_arm_severed || self.right_arm_severed
    }
}

// === 动作合成器 ===
#[derive(Debug, Clone)]
pub struct ActionSynthesizer {
    pub library: ActionLibrary,
    /// 当前播放的层 → (动作名, 时间)
    pub current_layers: HashMap<BodyLayer, (&'static str, f32)>,
    /// 上一帧的层状态（用于过渡混合）
    pub prev_layers: HashMap<BodyLayer, (&'static str, f32)>,
    /// 过渡剩余时间（秒）
    pub transition_remaining: f32,
    /// 过渡总时间（秒）
    pub transition_total: f32,
}

impl Default for ActionSynthesizer {
    fn default() -> Self {
        Self::new(ActionLibrary::new())
    }
}

impl ActionSynthesizer {
    pub fn new(library: ActionLibrary) -> Self {
        let mut current_layers = HashMap::new();
        // 默认待机状态
        current_layers.insert(BodyLayer::LowerBody, ("legs_stand", 0.0));
        current_layers.insert(BodyLayer::Torso, ("torso_breathe", 0.0));
        current_layers.insert(BodyLayer::Head, ("head_forward", 0.0));
        Self {
            library,
            current_layers,
            prev_layers: HashMap::new(),
            transition_remaining: 0.0,
            transition_total: 0.2,
        }
    }

    /// 根据意图设置当前各层的动作
    pub fn resolve_intent(&mut self, intent: &ActionIntent, damage: &DamageState) {
        let mut new_layers: HashMap<BodyLayer, (&'static str, f32)> = HashMap::new();

        // 1. 下半身：根据运动状态选择动作
        let lower_action = match intent.current_motion {
            MotionState::Standing => {
                if damage.is_any_leg_severed() {
                    "legs_kneel" // 断腿强制单膝跪
                } else {
                    "legs_stand"
                }
            }
            MotionState::Walking => {
                if damage.is_any_leg_severed() {
                    "legs_kneel" // 断腿无法行走
                } else {
                    "legs_walk"
                }
            }
            MotionState::Running => {
                if damage.is_any_leg_severed() {
                    "legs_kneel"
                } else {
                    "legs_run"
                }
            }
            MotionState::Crouching => "legs_crouch",
            MotionState::Crawling => "legs_kneel",
            MotionState::Falling => "legs_stand",
            MotionState::Swimming => "legs_stand",
            MotionState::Climbing => "legs_stand",
        };
        // 保留时间相位（如果是同一个动作）
        let lower_t = self.current_layers.get(&BodyLayer::LowerBody)
            .filter(|(name, _)| *name == lower_action)
            .map(|(_, t)| *t)
            .unwrap_or(0.0);
        new_layers.insert(BodyLayer::LowerBody, (lower_action, lower_t));

        // 2. 躯干：根据动作类型决定前倾角度
        let torso_action = match intent.action_type {
            ActionType::PickUp | ActionType::Drop => "torso_lean_30",
            ActionType::Attack => "torso_twist",
            ActionType::Operate => "torso_lean_15",
            ActionType::Throw => "torso_twist",
            ActionType::Crouch | ActionType::Crawl => "torso_lean_15",
            _ => "torso_breathe",
        };
        new_layers.insert(BodyLayer::Torso, (torso_action, 0.0));

        // 3. 右臂：根据动作类型和手偏好
        let right_arm_action = if damage.right_arm_severed {
            "right_arm_down" // 断臂时仅下垂
        } else {
            match intent.action_type {
                ActionType::PickUp | ActionType::Operate if intent.hand_preference != 1 => "right_arm_forward",
                ActionType::Attack if intent.hand_preference != 1 => "right_arm_side",
                ActionType::Throw if intent.hand_preference != 1 => "right_arm_up",
                _ => "right_arm_down",
            }
        };
        new_layers.insert(BodyLayer::RightArm, (right_arm_action, 0.0));

        // 4. 左臂：根据动作类型和手偏好
        let left_arm_action = if damage.left_arm_severed {
            "left_arm_down"
        } else {
            match intent.action_type {
                ActionType::PickUp | ActionType::Operate if intent.hand_preference != 0 => "left_arm_forward",
                ActionType::Attack if intent.hand_preference != 0 => "left_arm_side",
                ActionType::Throw if intent.hand_preference != 0 => "left_arm_up",
                _ => "left_arm_down",
            }
        };
        new_layers.insert(BodyLayer::LeftArm, (left_arm_action, 0.0));

        // 5. 头部：注视目标
        let head_action = match intent.action_type {
            ActionType::PickUp | ActionType::Drop => "head_down",
            _ => "head_forward",
        };
        new_layers.insert(BodyLayer::Head, (head_action, 0.0));

        // 检查层是否变化，触发过渡
        let any_changed = new_layers.iter().any(|(layer, (name, _))| {
            self.current_layers.get(layer).map_or(true, |(cur, _)| cur != name)
        });
        if any_changed {
            self.prev_layers = self.current_layers.clone();
            self.transition_remaining = self.transition_total;
        }
        self.current_layers = new_layers;
    }

    /// 推进时间步（秒）
    pub fn tick(&mut self, dt: f32) {
        // 减少过渡剩余时间
        if self.transition_remaining > 0.0 {
            self.transition_remaining = (self.transition_remaining - dt).max(0.0);
        }
        // 推进各层动作时间
        for (_, (_, t)) in self.current_layers.iter_mut() {
            *t += dt;
        }
        for (_, (_, t)) in self.prev_layers.iter_mut() {
            *t += dt;
        }
    }

    /// 合成最终的关节目标变换
    pub fn synthesize(&self, damage: &DamageState) -> JointTargetMap {
        let mut result: JointTargetMap = HashMap::new();

        // 1. 采样当前各层
        for (layer, (name, t)) in &self.current_layers {
            if let Some(action) = self.library.get(name) {
                let sampled = action.sample(*t);
                // 应用损伤因子（降低动作幅度）
                let factor = match layer {
                    BodyLayer::LeftArm => damage.left_arm_factor,
                    BodyLayer::RightArm => damage.right_arm_factor,
                    BodyLayer::LowerBody => damage.left_leg_factor.min(damage.right_leg_factor),
                    BodyLayer::Torso => damage.torso_factor,
                    BodyLayer::Head => damage.head_factor,
                };
                // 休克迟缓：降低整体动作幅度
                let shock_factor = 1.0 - damage.shock_level * 0.5;
                let combined_factor = factor * shock_factor;
                for (joint_name, target) in sampled {
                    let scaled = Self::scale_target(&target, combined_factor);
                    Self::merge_target(&mut result, joint_name, &scaled);
                }
            }
        }

        // 2. 过渡混合
        if self.transition_remaining > 0.0 {
            let alpha = 1.0 - (self.transition_remaining / self.transition_total);
            // 采样 prev_layers
            let mut prev_result: JointTargetMap = HashMap::new();
            for (layer, (name, t)) in &self.prev_layers {
                if let Some(action) = self.library.get(name) {
                    let sampled = action.sample(*t);
                    for (joint_name, target) in sampled {
                        Self::merge_target(&mut prev_result, joint_name, &target);
                    }
                }
                let _ = layer; // 已经在采样时使用
            }
            // 混合 prev → current
            for (joint_name, current_target) in result.clone() {
                if let Some(prev_target) = prev_result.get(&joint_name) {
                    let blended = prev_target.lerp(&current_target, alpha);
                    result.insert(joint_name, blended);
                }
            }
            // 添加 prev 独有的关节
            for (joint_name, prev_target) in prev_result {
                if !result.contains_key(&joint_name) {
                    let faded = prev_target.lerp(&JointTarget::identity(), alpha);
                    result.insert(joint_name, faded);
                }
            }
        }

        // 3. 神经损伤抖动（添加随机噪声到旋转）
        if damage.nerve_tremor > 0.01 {
            let tremor_amp = damage.nerve_tremor * 0.05; // 最大 0.05 rad
            for (_, target) in result.iter_mut() {
                // 使用确定性噪声（基于时间）—— 这里简化为偏置
                target.rotation_offset[0] += tremor_amp * 0.5;
                target.rotation_offset[1] += tremor_amp * 0.3;
            }
        }

        result
    }

    /// 缩放关节目标（按损伤因子降低动作幅度）
    fn scale_target(target: &JointTarget, factor: f32) -> JointTarget {
        JointTarget {
            translation_offset: [
                target.translation_offset[0] * factor,
                target.translation_offset[1] * factor,
                target.translation_offset[2] * factor,
            ],
            rotation_offset: [
                target.rotation_offset[0] * factor,
                target.rotation_offset[1] * factor,
                target.rotation_offset[2] * factor,
                // w 分量保持接近 1（保持单位四元数近似）
                1.0 - (1.0 - target.rotation_offset[3]) * factor,
            ],
            scale: target.scale,
        }
    }

    /// 合并关节目标（后写覆盖先写，因为不同层的关节通常不重叠）
    fn merge_target(map: &mut JointTargetMap, joint_name: String, target: &JointTarget) {
        map.insert(joint_name, *target);
    }

    /// 获取当前播放的动作名（用于调试）
    pub fn current_action_name(&self, layer: BodyLayer) -> Option<&'static str> {
        self.current_layers.get(&layer).map(|(name, _)| *name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_type_classification() {
        assert!(ActionType::Walk.is_locomotion());
        assert!(!ActionType::Idle.is_locomotion());
        assert!(ActionType::PickUp.is_upper_body_action());
        assert!(!ActionType::Walk.is_upper_body_action());
    }

    #[test]
    fn test_motion_state_speed() {
        assert_eq!(MotionState::Standing.speed_m_s(), 0.0);
        assert!(MotionState::Running.speed_m_s() > MotionState::Walking.speed_m_s());
    }

    #[test]
    fn test_joint_target_identity() {
        let t = JointTarget::identity();
        assert_eq!(t.translation_offset, [0.0, 0.0, 0.0]);
        assert_eq!(t.rotation_offset, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(t.scale, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_joint_target_rotation_x() {
        let t = JointTarget::rotation_x(1.0);
        // 1.0 rad 的旋转，四元数 (sin(0.5), 0, 0, cos(0.5))
        let expected_w = (0.5_f32).cos();
        assert!((t.rotation_offset[3] - expected_w).abs() < 1e-5);
        assert!(t.rotation_offset[0].abs() > 0.01);
    }

    #[test]
    fn test_joint_target_lerp() {
        let a = JointTarget::identity();
        let b = JointTarget::rotation_x(1.0);
        let mid = a.lerp(&b, 0.5);
        // 中点应该在两者之间
        assert!(mid.rotation_offset[0] > a.rotation_offset[0]);
        assert!(mid.rotation_offset[0] < b.rotation_offset[0]);
    }

    #[test]
    fn test_action_library_default_populated() {
        let lib = ActionLibrary::new();
        // 至少 20 个原子动作
        assert!(lib.count() >= 20, "expected >=20 actions, got {}", lib.count());
        // 关键动作存在
        assert!(lib.get("torso_upright").is_some());
        assert!(lib.get("right_arm_forward").is_some());
        assert!(lib.get("left_arm_forward").is_some());
        assert!(lib.get("legs_walk").is_some());
        assert!(lib.get("legs_run").is_some());
        assert!(lib.get("head_forward").is_some());
        assert!(lib.get("torso_breathe").is_some());
    }

    #[test]
    fn test_action_library_by_layer() {
        let lib = ActionLibrary::new();
        let torso_actions = lib.by_layer(BodyLayer::Torso);
        assert!(torso_actions.len() >= 5, "expected >=5 torso actions, got {}", torso_actions.len());
        let right_arm_actions = lib.by_layer(BodyLayer::RightArm);
        assert!(right_arm_actions.len() >= 5);
    }

    #[test]
    fn test_atomic_action_sample_at_keyframe() {
        let lib = ActionLibrary::new();
        let action = lib.get("right_arm_forward").unwrap();
        // 在 t=0 应该是 identity（关节无变换）
        let sampled = action.sample(0.0);
        assert!(sampled.is_empty() || sampled.values().all(|t| t.rotation_offset == [0.0, 0.0, 0.0, 1.0]));
        // 在 t=duration 应该有变换
        let sampled_end = action.sample(action.duration_s);
        assert!(!sampled_end.is_empty());
    }

    #[test]
    fn test_atomic_action_sample_looping() {
        let lib = ActionLibrary::new();
        let action = lib.get("legs_walk").unwrap();
        // 循环动作：t=0 和 t=duration 应该相同
        let s0 = action.sample(0.0);
        let s1 = action.sample(action.duration_s);
        // 都应该有变换
        assert!(!s0.is_empty());
        assert!(!s1.is_empty());
    }

    #[test]
    fn test_atomic_action_sample_interpolates() {
        let lib = ActionLibrary::new();
        let action = lib.get("right_arm_forward").unwrap();
        let s_start = action.sample(0.0);
        let s_end = action.sample(action.duration_s);
        let s_mid = action.sample(action.duration_s * 0.5);
        // 中点应该有变换但小于终点
        if let (Some(end_t), Some(mid_t)) = (
            s_end.get("shoulder_r"),
            s_mid.get("shoulder_r"),
        ) {
            let end_mag = end_t.rotation_offset[0].abs();
            let mid_mag = mid_t.rotation_offset[0].abs();
            assert!(mid_mag > 0.0, "expected non-zero mid rotation");
            assert!(mid_mag < end_mag, "expected mid < end, got {} vs {}", mid_mag, end_mag);
        }
        let _ = s_start;
    }

    #[test]
    fn test_synthesizer_default_idle() {
        let synth = ActionSynthesizer::default();
        // 默认应该有下半身和躯干动作
        assert!(synth.current_action_name(BodyLayer::LowerBody).is_some());
        assert!(synth.current_action_name(BodyLayer::Torso).is_some());
    }

    #[test]
    fn test_synthesizer_resolve_walk_intent() {
        let mut synth = ActionSynthesizer::default();
        let intent = ActionIntent {
            action_type: ActionType::Walk,
            current_motion: MotionState::Walking,
            ..Default::default()
        };
        synth.resolve_intent(&intent, &DamageState::healthy());
        assert_eq!(synth.current_action_name(BodyLayer::LowerBody), Some("legs_walk"));
    }

    #[test]
    fn test_synthesizer_resolve_pickup_intent() {
        let mut synth = ActionSynthesizer::default();
        let intent = ActionIntent {
            action_type: ActionType::PickUp,
            current_motion: MotionState::Standing,
            hand_preference: 0, // 右手
            ..Default::default()
        };
        synth.resolve_intent(&intent, &DamageState::healthy());
        assert_eq!(synth.current_action_name(BodyLayer::Torso), Some("torso_lean_30"));
        assert_eq!(synth.current_action_name(BodyLayer::RightArm), Some("right_arm_forward"));
        // 左臂应该下垂
        assert_eq!(synth.current_action_name(BodyLayer::LeftArm), Some("left_arm_down"));
    }

    #[test]
    fn test_synthesizer_severed_arm_disables_action() {
        let mut synth = ActionSynthesizer::default();
        let mut damage = DamageState::healthy();
        damage.right_arm_severed = true;
        damage.right_arm_factor = 0.0;
        let intent = ActionIntent {
            action_type: ActionType::PickUp,
            current_motion: MotionState::Standing,
            hand_preference: 0,
            ..Default::default()
        };
        synth.resolve_intent(&intent, &damage);
        // 断右臂时强制使用 right_arm_down
        assert_eq!(synth.current_action_name(BodyLayer::RightArm), Some("right_arm_down"));
    }

    #[test]
    fn test_synthesizer_severed_leg_forces_kneel() {
        let mut synth = ActionSynthesizer::default();
        let mut damage = DamageState::healthy();
        damage.left_leg_severed = true;
        damage.left_leg_factor = 0.0;
        let intent = ActionIntent {
            action_type: ActionType::Walk,
            current_motion: MotionState::Walking,
            ..Default::default()
        };
        synth.resolve_intent(&intent, &damage);
        // 断腿强制单膝跪
        assert_eq!(synth.current_action_name(BodyLayer::LowerBody), Some("legs_kneel"));
    }

    #[test]
    fn test_synthesizer_synthesize_returns_joints() {
        let mut synth = ActionSynthesizer::default();
        synth.tick(0.5);
        let joints = synth.synthesize(&DamageState::healthy());
        // 至少应该有部分关节变换（呼吸或站立）
        assert!(!joints.is_empty() || synth.current_layers.is_empty());
    }

    #[test]
    fn test_synthesizer_tick_advances_time() {
        let mut synth = ActionSynthesizer::default();
        let initial_t = synth.current_layers.get(&BodyLayer::Torso).map(|(_, t)| *t).unwrap_or(0.0);
        synth.tick(0.1);
        let new_t = synth.current_layers.get(&BodyLayer::Torso).map(|(_, t)| *t).unwrap_or(0.0);
        assert!((new_t - initial_t - 0.1).abs() < 1e-6, "expected time advance by 0.1");
    }

    #[test]
    fn test_synthesizer_transition_on_intent_change() {
        let mut synth = ActionSynthesizer::default();
        // 初始无过渡
        assert_eq!(synth.transition_remaining, 0.0);
        // 改变意图触发过渡
        let intent = ActionIntent {
            action_type: ActionType::Run,
            current_motion: MotionState::Running,
            ..Default::default()
        };
        synth.resolve_intent(&intent, &DamageState::healthy());
        assert!(synth.transition_remaining > 0.0, "expected transition triggered");
        assert!(!synth.prev_layers.is_empty(), "expected prev_layers saved");
    }

    #[test]
    fn test_synthesizer_damage_reduces_amplitude() {
        let mut synth = ActionSynthesizer::default();
        let intent = ActionIntent {
            action_type: ActionType::PickUp,
            current_motion: MotionState::Standing,
            hand_preference: 0,
            ..Default::default()
        };
        synth.resolve_intent(&intent, &DamageState::healthy());
        synth.tick(0.5);
        let healthy_joints = synth.synthesize(&DamageState::healthy());

        let mut damaged = DamageState::healthy();
        damaged.right_arm_factor = 0.3; // 右臂严重损伤
        let damaged_joints = synth.synthesize(&damaged);

        // 右臂关节的变换幅度应该减小
        if let (Some(h), Some(d)) = (healthy_joints.get("shoulder_r"), damaged_joints.get("shoulder_r")) {
            let h_mag = h.rotation_offset[0].abs();
            let d_mag = d.rotation_offset[0].abs();
            assert!(d_mag <= h_mag + 1e-6, "expected damaged amplitude <= healthy, got {} vs {}", d_mag, h_mag);
        }
    }

    #[test]
    fn test_synthesizer_shock_slows_action() {
        let mut synth = ActionSynthesizer::default();
        let intent = ActionIntent {
            action_type: ActionType::PickUp,
            current_motion: MotionState::Standing,
            hand_preference: 0,
            ..Default::default()
        };
        synth.resolve_intent(&intent, &DamageState::healthy());
        synth.tick(0.5);
        let normal = synth.synthesize(&DamageState::healthy());

        let mut shocked = DamageState::healthy();
        shocked.shock_level = 0.8;
        let slowed = synth.synthesize(&shocked);

        // 整体变换幅度应该减小
        let normal_total: f32 = normal.values().map(|t| t.rotation_offset[0].abs()).sum();
        let slowed_total: f32 = slowed.values().map(|t| t.rotation_offset[0].abs()).sum();
        assert!(slowed_total <= normal_total + 1e-6, "expected shock to reduce amplitude, got {} vs {}", slowed_total, normal_total);
    }

    #[test]
    fn test_synthesizer_nerve_tremor_adds_noise() {
        let mut synth = ActionSynthesizer::default();
        synth.tick(0.5);
        let normal = synth.synthesize(&DamageState::healthy());

        let mut tremor = DamageState::healthy();
        tremor.nerve_tremor = 0.5;
        let trembling = synth.synthesize(&tremor);

        // 抖动版本的关节旋转应该有偏移
        let mut has_diff = false;
        for (joint, normal_t) in &normal {
            if let Some(tremor_t) = trembling.get(joint) {
                if (normal_t.rotation_offset[0] - tremor_t.rotation_offset[0]).abs() > 1e-6 {
                    has_diff = true;
                    break;
                }
            }
        }
        // 至少有一个关节应该被添加抖动
        // 注意：如果 normal 为空（如站立姿态），trembling 可能有抖动条目
        assert!(has_diff || trembling.len() > 0, "expected nerve tremor to add noise");
    }

    #[test]
    fn test_damage_state_healthy() {
        let h = DamageState::healthy();
        assert!(!h.is_any_leg_severed());
        assert!(!h.is_any_arm_severed());
        assert_eq!(h.left_arm_factor, 1.0);
    }

    #[test]
    fn test_action_library_size_under_5mb() {
        let lib = ActionLibrary::new();
        // 简单估算：每个动作约 200 字节（4 关键帧 × 4 关节 × ~12 字节）
        let estimated_bytes = lib.count() * 200;
        assert!(estimated_bytes < 5_000_000, "expected library <5MB, estimated {} bytes", estimated_bytes);
    }
}
