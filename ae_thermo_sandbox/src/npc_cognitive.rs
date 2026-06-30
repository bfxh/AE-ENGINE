//! V8 沙盒 NPC 认知轨模块
//!
//! 区别于 npc_entity.rs（物理轨），本模块处理 NPC 的需求驱动、行为决策和记忆。
//! 实现 Maslow 需求层次驱动的行为决策模型：
//! - 生理需求（呼吸/生存/口渴/饥饿/休息）→ 安全需求 → 社交需求 → 自我实现
//! - Big Five 性格特质影响决策偏好
//! - 短期记忆系统记录危险/资源/社交事件
//!
//! 耦合方向：
//! - 物理轨 → 认知轨：体温/血氧/血量/疲劳/饥饿/口渴/健康/危险状态 驱动需求紧急度
//! - 认知轨 → 物理轨：行为决策（MoveTo/Flee/Sleep 等）影响物理动作

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// ─── Need 枚举 ───────────────────────────────────────────────
/// Maslow 需求层次类型（优先级从高到低）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Need {
    // 生理需求（最高优先级）
    /// 呼吸（O2 不足时触发）
    Breathe,
    /// 生存（失血/燃烧/极端温度）
    Survive,
    /// 口渴
    Thirst,
    /// 饥饿
    Hunger,
    /// 休息（疲劳）
    Rest,
    // 安全需求
    /// 安全（远离火源/危险）
    Safety,
    /// 庇护所
    Shelter,
    /// 健康
    Health,
    // 社交需求
    /// 社交
    Social,
    // 自我实现
    /// 探索
    Explore,
    /// 工作
    Work,
}

impl Need {
    /// 需求的优先级权重 0..1（越高越优先）
    pub fn priority_weight(self) -> f32 {
        match self {
            Need::Breathe => 1.0,
            Need::Survive => 1.0,
            Need::Thirst => 0.8,
            Need::Hunger => 0.7,
            Need::Rest => 0.6,
            Need::Safety => 0.5,
            Need::Shelter => 0.4,
            Need::Health => 0.5,
            Need::Social => 0.2,
            Need::Explore => 0.1,
            Need::Work => 0.15,
        }
    }
}

// ─── NeedState ───────────────────────────────────────────────
/// 需求状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedState {
    pub need: Need,
    /// 0..1 紧急度（0=满足, 1=极度匮乏）
    pub urgency: f32,
    /// urgency * priority_weight，用于排序
    pub priority: f32,
}

impl NeedState {
    pub fn new(need: Need, urgency: f32) -> Self {
        let urgency = urgency.clamp(0.0, 1.0);
        let priority = urgency * need.priority_weight();
        Self { need, urgency, priority }
    }
}

// ─── ActionType ──────────────────────────────────────────────
/// 行为类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    /// 待机
    Idle,
    /// 移动到目标
    MoveTo,
    /// 逃跑
    Flee,
    /// 喝水
    Drink,
    /// 进食
    Eat,
    /// 睡眠
    Sleep,
    /// 寻找庇护所
    SeekShelter,
    /// 战斗
    Fight,
    /// 对话
    Talk,
    /// 探索
    Explore,
    /// 工作
    Work,
    /// 扑灭自身火焰
    ExtinguishSelf,
}

// ─── Action ──────────────────────────────────────────────────
/// 行为决策
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub action_type: ActionType,
    /// 目标位置（MoveTo/Flee 等）
    pub target_position: Option<[f32; 3]>,
    /// 目标 NPC ID（Talk/Fight）
    pub target_id: Option<u64>,
    /// 预计持续时间 s
    pub duration: f32,
    /// 行为优先级
    pub priority: f32,
}

impl Action {
    /// 构造一个默认的 Idle 行为
    pub fn idle() -> Self {
        Self {
            action_type: ActionType::Idle,
            target_position: None,
            target_id: None,
            duration: 1.0,
            priority: 0.0,
        }
    }
}

impl Default for Action {
    fn default() -> Self {
        Self::idle()
    }
}

// ─── MemoryType ──────────────────────────────────────────────
/// 记忆事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryType {
    /// 危险（火源/敌人）
    Danger,
    /// 资源（水/食物）
    Resource,
    /// 庇护所
    Shelter,
    /// 社交事件
    Social,
    /// 发现
    Discovery,
}

// ─── MemoryEvent ─────────────────────────────────────────────
/// 记忆事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEvent {
    /// 发生时间
    pub timestamp: f32,
    pub event_type: MemoryType,
    pub position: [f32; 3],
    pub description: String,
}

// ─── Memory ──────────────────────────────────────────────────
/// 记忆系统
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub events: Vec<MemoryEvent>,
    /// 最大记忆容量（默认 100）
    pub max_events: usize,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            max_events: 100,
        }
    }
}

impl Memory {
    /// 记录事件，超过容量删除最旧的
    pub fn record(&mut self, event: MemoryEvent) {
        self.events.push(event);
        if self.events.len() > self.max_events {
            self.events.remove(0);
        }
    }

    /// 查询近期危险
    pub fn recent_dangers(&self, within_seconds: f32, current_time: f32) -> Vec<&MemoryEvent> {
        self.events
            .iter()
            .filter(|e| {
                e.event_type == MemoryType::Danger
                    && (current_time - e.timestamp) <= within_seconds
            })
            .collect()
    }

    /// 查询已知资源
    pub fn known_resources(&self, resource_type: MemoryType) -> Vec<&MemoryEvent> {
        self.events
            .iter()
            .filter(|e| e.event_type == resource_type)
            .collect()
    }

    /// 遗忘旧记忆
    pub fn forget_older_than(&mut self, seconds: f32, current_time: f32) {
        self.events
            .retain(|e| (current_time - e.timestamp) <= seconds);
    }
}

// ─── Personality ─────────────────────────────────────────────
/// Big Five 性格特质（简化版）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Personality {
    /// 0..1 开放性（影响探索倾向）
    pub openness: f32,
    /// 0..1 尽责性（影响工作倾向）
    pub conscientiousness: f32,
    /// 0..1 外向性（影响社交倾向）
    pub extraversion: f32,
    /// 0..1 宜人性（影响合作倾向）
    pub agreeableness: f32,
    /// 0..1 神经质（影响恐惧反应）
    pub neuroticism: f32,
}

impl Default for Personality {
    fn default() -> Self {
        Self {
            openness: 0.5,
            conscientiousness: 0.5,
            extraversion: 0.5,
            agreeableness: 0.5,
            neuroticism: 0.5,
        }
    }
}

/// 简单的伪随机数生成器（LCG，不依赖 rand crate）
pub struct Rng {
    state: u64,
}

impl Rng {
    /// 从种子创建
    pub fn from_seed(seed: u64) -> Self {
        // 避免全 0 状态
        Self { state: seed.max(1) }
    }

    /// 从系统时间创建种子
    pub fn from_time() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        Self::from_seed(nanos)
    }

    /// 下一个 u32
    pub fn next_u32(&mut self) -> u32 {
        // Numerical Recipes LCG 常量
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 32) as u32
    }

    /// 0..1 浮点
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }
}

impl Personality {
    /// 使用系统时间作为种子生成随机性格
    pub fn random() -> Self {
        let mut rng = Rng::from_time();
        Self::random_with_rng(&mut rng)
    }

    /// 使用指定种子生成（便于测试复现）
    pub fn random_with_seed(seed: u64) -> Self {
        let mut rng = Rng::from_seed(seed);
        Self::random_with_rng(&mut rng)
    }

    fn random_with_rng(rng: &mut Rng) -> Self {
        Self {
            openness: rng.next_f32(),
            conscientiousness: rng.next_f32(),
            extraversion: rng.next_f32(),
            agreeableness: rng.next_f32(),
            neuroticism: rng.next_f32(),
        }
    }
}

// ─── CognitiveState ──────────────────────────────────────────
/// 认知状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveState {
    pub needs: Vec<NeedState>,
    pub current_action: Action,
    /// 0..1 当前行为进度
    pub action_progress: f32,
    pub memory: Memory,
    pub personality: Personality,
    /// 0..1 技能等级
    pub skill_hunting: f32,
    pub skill_crafting: f32,
    pub skill_medicine: f32,
    /// 上次决策时间
    pub last_decision_time: f32,
    /// 决策间隔（默认 1.0s）
    pub decision_interval: f32,
    /// 缓存着火状态（由 update_needs 写入，decide 读取）
    on_fire: bool,
    /// 缓存血氧饱和度（由 update_needs 写入，decide 读取）
    oxygen_sat: f32,
}

impl CognitiveState {
    pub fn new(personality: Personality) -> Self {
        let needs = vec![
            NeedState::new(Need::Breathe, 0.0),
            NeedState::new(Need::Survive, 0.0),
            NeedState::new(Need::Thirst, 0.0),
            NeedState::new(Need::Hunger, 0.0),
            NeedState::new(Need::Rest, 0.0),
            NeedState::new(Need::Safety, 0.0),
            NeedState::new(Need::Shelter, 0.0),
            NeedState::new(Need::Health, 0.0),
            NeedState::new(Need::Social, 0.3),
            NeedState::new(Need::Explore, 0.2),
            NeedState::new(Need::Work, 0.25),
        ];
        Self {
            needs,
            current_action: Action::idle(),
            action_progress: 0.0,
            memory: Memory::default(),
            personality,
            skill_hunting: 0.1,
            skill_crafting: 0.1,
            skill_medicine: 0.1,
            last_decision_time: 0.0,
            decision_interval: 1.0,
            on_fire: false,
            oxygen_sat: 0.98,
        }
    }

    /// 从物理轨状态更新需求紧急度
    pub fn update_needs(
        &mut self,
        body_temp: f32,
        oxygen_sat: f32,
        blood_volume: f32,
        fatigue: f32,
        hunger: f32,
        thirst: f32,
        health: f32,
        in_danger: bool,
        on_fire: bool,
        near_fire: bool,
    ) {
        self.on_fire = on_fire;
        self.oxygen_sat = oxygen_sat;

        for n in &mut self.needs {
            let mut urgency = compute_urgency(
                n.need,
                body_temp,
                oxygen_sat,
                blood_volume,
                fatigue,
                hunger,
                thirst,
                health,
                in_danger,
                on_fire,
            );

            // near_fire 提升 Safety 紧急度（compute_urgency 不处理 near_fire）
            // 设为 0.4：低于普通危险阈值 0.5，但高于高神经质阈值 0.3
            if n.need == Need::Safety && near_fire && urgency < 0.4 {
                urgency = 0.4;
            }

            // 性格影响：高外向/开放/尽责 → 对应需求 urgency +0.2
            match n.need {
                Need::Social if self.personality.extraversion > 0.6 => {
                    urgency = (urgency + 0.2).min(1.0);
                }
                Need::Explore if self.personality.openness > 0.6 => {
                    urgency = (urgency + 0.2).min(1.0);
                }
                Need::Work if self.personality.conscientiousness > 0.6 => {
                    urgency = (urgency + 0.2).min(1.0);
                }
                _ => {}
            }

            n.urgency = urgency.clamp(0.0, 1.0);
            n.priority = n.urgency * n.need.priority_weight();
        }
    }

    /// 决策：选择优先级最高的需求，生成对应行为
    ///
    /// 决策优先级链：
    /// 1. 着火 → 扑灭自身
    /// 2. 低血氧 → 逃跑（寻找新鲜空气）
    /// 3. 危险 → 逃跑
    /// 4. 高口渴 → 移动寻找水
    /// 5. 高饥饿 → 移动寻找食物
    /// 6. 高疲劳 → 睡眠
    /// 7. 低健康 → 寻找庇护所
    /// 8. 性格+需求综合决策（社交/探索/工作）
    /// 9. 默认待机
    pub fn decide(&mut self, current_position: [f32; 3], current_time: f32) -> Action {
        // 节流：未到决策间隔直接返回当前 action
        if (current_time - self.last_decision_time) < self.decision_interval {
            return self.current_action.clone();
        }
        self.last_decision_time = current_time;

        // 性格影响：高神经质 → in_danger 更易触发 Flee（阈值降低）
        let safety_urgency = self.urgency_of(Need::Safety);
        let danger_threshold = if self.personality.neuroticism > 0.7 { 0.3 } else { 0.5 };
        let in_danger_eff = safety_urgency > danger_threshold;

        // 1. 着火 → 扑灭自身
        if self.on_fire {
            return self.set_action(Action {
                action_type: ActionType::ExtinguishSelf,
                target_position: None,
                target_id: None,
                duration: 2.0,
                priority: 1.0,
            });
        }

        // 2. 低血氧 → 逃跑（寻找新鲜空气）
        if self.oxygen_sat < 0.85 {
            return self.set_action(Action {
                action_type: ActionType::Flee,
                target_position: Some(self.flee_target(current_position)),
                target_id: None,
                duration: 3.0,
                priority: 1.0,
            });
        }

        // 3. 危险 → 逃跑
        if in_danger_eff {
            return self.set_action(Action {
                action_type: ActionType::Flee,
                target_position: Some(self.flee_target(current_position)),
                target_id: None,
                duration: 3.0,
                priority: 0.95,
            });
        }

        // 4. 高口渴 → 移动寻找水
        if self.urgency_of(Need::Thirst) > 0.7 {
            return self.set_action(Action {
                action_type: ActionType::MoveTo,
                target_position: Some(self.find_resource_or_explore("water", current_position)),
                target_id: None,
                duration: 5.0,
                priority: 0.8,
            });
        }

        // 5. 高饥饿 → 移动寻找食物
        if self.urgency_of(Need::Hunger) > 0.7 {
            return self.set_action(Action {
                action_type: ActionType::MoveTo,
                target_position: Some(self.find_resource_or_explore("food", current_position)),
                target_id: None,
                duration: 5.0,
                priority: 0.7,
            });
        }

        // 6. 高疲劳 → 睡眠
        if self.urgency_of(Need::Rest) > 0.8 {
            return self.set_action(Action {
                action_type: ActionType::Sleep,
                target_position: None,
                target_id: None,
                duration: 8.0 * 3600.0,
                priority: 0.6,
            });
        }

        // 7. 低健康 → 寻找庇护所
        if self.urgency_of(Need::Health) > 0.5 {
            return self.set_action(Action {
                action_type: ActionType::SeekShelter,
                target_position: Some(self.find_shelter_or_explore(current_position)),
                target_id: None,
                duration: 10.0,
                priority: 0.5,
            });
        }

        // 8. 性格+需求综合决策（社交/探索/工作）
        let mut sorted: Vec<&NeedState> = self.needs.iter().collect();
        sorted.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if let Some(top) = sorted.first() {
            // 仅对自我实现类需求触发，且 urgency 需超过 0.4
            match top.need {
                Need::Social if top.urgency > 0.4 => {
                    return self.set_action(Action {
                        action_type: ActionType::Talk,
                        target_position: None,
                        target_id: None,
                        duration: 60.0,
                        priority: 0.2,
                    });
                }
                Need::Explore if top.urgency > 0.4 => {
                    return self.set_action(Action {
                        action_type: ActionType::Explore,
                        target_position: Some(self.explore_target(current_position)),
                        target_id: None,
                        duration: 30.0,
                        priority: 0.1,
                    });
                }
                Need::Work if top.urgency > 0.4 => {
                    return self.set_action(Action {
                        action_type: ActionType::Work,
                        target_position: None,
                        target_id: None,
                        duration: 120.0,
                        priority: 0.15,
                    });
                }
                _ => {}
            }
        }

        // 9. 默认待机
        self.set_action(Action::idle())
    }

    /// 推进当前行为，返回 true 表示行为完成
    pub fn tick_action(&mut self, dt: f32) -> bool {
        if self.current_action.duration <= 0.0 {
            return true;
        }
        self.action_progress += dt / self.current_action.duration;
        if self.action_progress >= 1.0 {
            self.action_progress = 1.0;
            true
        } else {
            false
        }
    }

    /// 记忆事件
    pub fn remember(&mut self, event: MemoryEvent) {
        self.memory.record(event);
    }

    // ─── 内部辅助方法 ───

    /// 设置当前行为并重置进度
    fn set_action(&mut self, action: Action) -> Action {
        self.current_action = action;
        self.action_progress = 0.0;
        self.current_action.clone()
    }

    /// 查询指定需求的紧急度
    fn urgency_of(&self, need: Need) -> f32 {
        self.needs
            .iter()
            .find(|n| n.need == need)
            .map(|n| n.urgency)
            .unwrap_or(0.0)
    }

    /// 逃跑目标：远离当前位置 10m
    fn flee_target(&self, current_position: [f32; 3]) -> [f32; 3] {
        [
            current_position[0] + 10.0,
            current_position[1],
            current_position[2] + 10.0,
        ]
    }

    /// 探索目标：基于当前位置和记忆的伪随机偏移
    fn explore_target(&self, current_position: [f32; 3]) -> [f32; 3] {
        let seed = (current_position[0].to_bits() as u64)
            .wrapping_add(current_position[2].to_bits() as u64)
            .wrapping_add(self.memory.events.len() as u64)
            .max(1);
        let mut rng = Rng::from_seed(seed);
        let dx = (rng.next_f32() - 0.5) * 20.0;
        let dz = (rng.next_f32() - 0.5) * 20.0;
        [
            current_position[0] + dx,
            current_position[1],
            current_position[2] + dz,
        ]
    }

    /// 从记忆查询资源位置，无记忆则探索
    fn find_resource_or_explore(&self, keyword: &str, current_position: [f32; 3]) -> [f32; 3] {
        self.memory
            .known_resources(MemoryType::Resource)
            .into_iter()
            .find(|e| e.description.contains(keyword))
            .map(|e| e.position)
            .unwrap_or_else(|| self.explore_target(current_position))
    }

    /// 从记忆查询庇护所位置，无记忆则探索
    fn find_shelter_or_explore(&self, current_position: [f32; 3]) -> [f32; 3] {
        self.memory
            .known_resources(MemoryType::Shelter)
            .into_iter()
            .next()
            .map(|e| e.position)
            .unwrap_or_else(|| self.explore_target(current_position))
    }
}

// ─── 紧急度计算 ──────────────────────────────────────────────
/// 从生理状态计算需求紧急度
fn compute_urgency(
    need: Need,
    body_temp: f32,
    oxygen_sat: f32,
    blood_volume: f32,
    fatigue: f32,
    hunger: f32,
    thirst: f32,
    health: f32,
    in_danger: bool,
    on_fire: bool,
) -> f32 {
    match need {
        Need::Breathe => (0.95 - oxygen_sat).max(0.0) / 0.95,
        Need::Survive => {
            let temp_risk = ((body_temp - 310.15).abs() / 10.0).min(1.0);
            let blood_risk = ((5.0 - blood_volume) / 5.0).max(0.0);
            let fire_risk = if on_fire { 1.0 } else { 0.0 };
            temp_risk.max(blood_risk).max(fire_risk)
        }
        Need::Thirst => thirst,
        Need::Hunger => hunger,
        Need::Rest => fatigue,
        Need::Safety => {
            if in_danger {
                0.9
            } else {
                0.0
            }
        }
        Need::Shelter => 0.0,
        Need::Health => 1.0 - health,
        Need::Social => 0.3,
        Need::Explore => 0.2,
        Need::Work => 0.25,
    }
}

// ─── 单元测试 ───────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个默认健康状态的 NPC 认知状态（禁用决策节流便于测试）
    fn healthy_state(personality: Personality) -> CognitiveState {
        let mut s = CognitiveState::new(personality);
        s.decision_interval = 0.0;
        s.update_needs(
            310.15, // body_temp 正常
            0.98,   // oxygen_sat 正常
            5.0,    // blood_volume 正常
            0.0,    // fatigue
            0.0,    // hunger
            0.0,    // thirst
            1.0,    // health
            false,  // in_danger
            false,  // on_fire
            false,  // near_fire
        );
        s
    }

    // ─── compute_urgency 测试 ───

    #[test]
    fn test_compute_urgency_breathe() {
        // 正常血氧 → 接近 0
        let u = compute_urgency(Need::Breathe, 310.15, 0.98, 5.0, 0.0, 0.0, 0.0, 1.0, false, false);
        assert!(u < 0.01, "正常血氧呼吸紧急度应接近 0: {}", u);
        // 低血氧 → 高紧急度
        let u = compute_urgency(Need::Breathe, 310.15, 0.5, 5.0, 0.0, 0.0, 0.0, 1.0, false, false);
        assert!(u > 0.4, "低血氧呼吸紧急度应高: {}", u);
    }

    #[test]
    fn test_compute_urgency_survive() {
        // 正常 → 0
        let u = compute_urgency(Need::Survive, 310.15, 0.98, 5.0, 0.0, 0.0, 0.0, 1.0, false, false);
        assert!(u < 0.01, "正常生存紧急度应低: {}", u);
        // 着火 → 1.0
        let u = compute_urgency(Need::Survive, 310.15, 0.98, 5.0, 0.0, 0.0, 0.0, 1.0, false, true);
        assert!((u - 1.0).abs() < 0.01, "着火生存紧急度应为 1.0: {}", u);
        // 失血 → 高
        let u = compute_urgency(Need::Survive, 310.15, 0.98, 2.0, 0.0, 0.0, 0.0, 1.0, false, false);
        assert!(u > 0.5, "失血生存紧急度应高: {}", u);
        // 极端体温
        let u = compute_urgency(Need::Survive, 330.0, 0.98, 5.0, 0.0, 0.0, 0.0, 1.0, false, false);
        assert!(u > 0.9, "高温生存紧急度应高: {}", u);
    }

    #[test]
    fn test_compute_urgency_thirst_hunger_rest() {
        // Thirst
        let u = compute_urgency(Need::Thirst, 310.15, 0.98, 5.0, 0.0, 0.0, 0.8, 1.0, false, false);
        assert!((u - 0.8).abs() < 0.01, "口渴紧急度应等于 thirst: {}", u);
        // Hunger
        let u = compute_urgency(Need::Hunger, 310.15, 0.98, 5.0, 0.0, 0.7, 0.0, 1.0, false, false);
        assert!((u - 0.7).abs() < 0.01, "饥饿紧急度应等于 hunger: {}", u);
        // Rest
        let u = compute_urgency(Need::Rest, 310.15, 0.98, 5.0, 0.9, 0.0, 0.0, 1.0, false, false);
        assert!((u - 0.9).abs() < 0.01, "疲劳紧急度应等于 fatigue: {}", u);
    }

    #[test]
    fn test_compute_urgency_safety_shelter_health_social_explore_work() {
        // Safety: in_danger=true → 0.9
        let u = compute_urgency(Need::Safety, 310.15, 0.98, 5.0, 0.0, 0.0, 0.0, 1.0, true, false);
        assert!((u - 0.9).abs() < 0.01, "危险时安全紧急度应为 0.9: {}", u);
        // Safety: in_danger=false → 0
        let u = compute_urgency(Need::Safety, 310.15, 0.98, 5.0, 0.0, 0.0, 0.0, 1.0, false, false);
        assert!(u < 0.01, "无危险时安全紧急度应为 0: {}", u);
        // Shelter: 恒为 0
        let u = compute_urgency(Need::Shelter, 310.15, 0.98, 5.0, 0.0, 0.0, 0.0, 1.0, false, false);
        assert!(u < 0.01, "庇护所紧急度应为 0: {}", u);
        // Health: 1.0 - health
        let u = compute_urgency(Need::Health, 310.15, 0.98, 5.0, 0.0, 0.0, 0.0, 0.3, false, false);
        assert!((u - 0.7).abs() < 0.01, "健康紧急度应为 0.7: {}", u);
        // Social: 基础 0.3
        let u = compute_urgency(Need::Social, 310.15, 0.98, 5.0, 0.0, 0.0, 0.0, 1.0, false, false);
        assert!((u - 0.3).abs() < 0.01, "社交紧急度应为 0.3: {}", u);
        // Explore: 基础 0.2
        let u = compute_urgency(Need::Explore, 310.15, 0.98, 5.0, 0.0, 0.0, 0.0, 1.0, false, false);
        assert!((u - 0.2).abs() < 0.01, "探索紧急度应为 0.2: {}", u);
        // Work: 基础 0.25
        let u = compute_urgency(Need::Work, 310.15, 0.98, 5.0, 0.0, 0.0, 0.0, 1.0, false, false);
        assert!((u - 0.25).abs() < 0.01, "工作紧急度应为 0.25: {}", u);
    }

    // ─── decide 决策测试 ───

    #[test]
    fn test_decide_on_fire_extinguish() {
        let mut s = healthy_state(Personality::default());
        s.update_needs(310.15, 0.98, 5.0, 0.0, 0.0, 0.0, 1.0, false, true, false);
        let action = s.decide([0.0, 0.0, 0.0], 1.0);
        assert_eq!(action.action_type, ActionType::ExtinguishSelf);
        assert!((action.priority - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_decide_low_oxygen_flee() {
        let mut s = healthy_state(Personality::default());
        s.update_needs(310.15, 0.7, 5.0, 0.0, 0.0, 0.0, 1.0, false, false, false);
        let action = s.decide([0.0, 0.0, 0.0], 1.0);
        assert_eq!(action.action_type, ActionType::Flee);
        assert!(action.target_position.is_some());
    }

    #[test]
    fn test_decide_high_thirst_moveto() {
        let mut s = healthy_state(Personality::default());
        s.update_needs(310.15, 0.98, 5.0, 0.0, 0.0, 0.8, 1.0, false, false, false);
        let action = s.decide([0.0, 0.0, 0.0], 1.0);
        assert_eq!(action.action_type, ActionType::MoveTo);
        assert!(action.target_position.is_some());
    }

    #[test]
    fn test_decide_high_fatigue_sleep() {
        let mut s = healthy_state(Personality::default());
        s.update_needs(310.15, 0.98, 5.0, 0.9, 0.0, 0.0, 1.0, false, false, false);
        let action = s.decide([0.0, 0.0, 0.0], 1.0);
        assert_eq!(action.action_type, ActionType::Sleep);
    }

    #[test]
    fn test_decide_low_health_seek_shelter() {
        let mut s = healthy_state(Personality::default());
        s.update_needs(310.15, 0.98, 5.0, 0.0, 0.0, 0.0, 0.3, false, false, false);
        let action = s.decide([0.0, 0.0, 0.0], 1.0);
        assert_eq!(action.action_type, ActionType::SeekShelter);
    }

    #[test]
    fn test_decide_in_danger_flee() {
        let mut s = healthy_state(Personality::default());
        s.update_needs(310.15, 0.98, 5.0, 0.0, 0.0, 0.0, 1.0, true, false, false);
        let action = s.decide([0.0, 0.0, 0.0], 1.0);
        assert_eq!(action.action_type, ActionType::Flee, "危险时应逃跑");
    }

    #[test]
    fn test_decide_default_idle() {
        let mut s = healthy_state(Personality::default());
        let action = s.decide([0.0, 0.0, 0.0], 1.0);
        assert_eq!(action.action_type, ActionType::Idle, "无需求时应待机");
    }

    #[test]
    fn test_decide_thirst_with_memory() {
        // 记忆中有水源 → MoveTo 目标应为水源位置
        let mut s = healthy_state(Personality::default());
        s.remember(MemoryEvent {
            timestamp: 0.0,
            event_type: MemoryType::Resource,
            position: [10.0, 0.0, 5.0],
            description: "water pool".into(),
        });
        s.update_needs(310.15, 0.98, 5.0, 0.0, 0.0, 0.8, 1.0, false, false, false);
        let action = s.decide([0.0, 0.0, 0.0], 1.0);
        assert_eq!(action.action_type, ActionType::MoveTo);
        let target = action.target_position.expect("应有目标位置");
        assert!((target[0] - 10.0).abs() < 0.01, "目标 X 应为 10: {}", target[0]);
        assert!((target[2] - 5.0).abs() < 0.01, "目标 Z 应为 5: {}", target[2]);
    }

    // ─── 记忆测试 ───

    #[test]
    fn test_memory_record_and_forget() {
        let mut mem = Memory::default();
        // 记录 5 个事件
        for i in 0..5 {
            mem.record(MemoryEvent {
                timestamp: i as f32,
                event_type: MemoryType::Discovery,
                position: [i as f32, 0.0, 0.0],
                description: format!("event {}", i),
            });
        }
        assert_eq!(mem.events.len(), 5);
        // 遗忘 3 秒前的事件（current_time=5, 保留 3 秒内 → timestamp >= 2）
        mem.forget_older_than(3.0, 5.0);
        assert_eq!(mem.events.len(), 3, "应保留 3 个近期事件");
        assert_eq!(mem.events[0].timestamp, 2.0);
    }

    #[test]
    fn test_memory_capacity_limit() {
        let mut mem = Memory::default();
        mem.max_events = 3;
        for i in 0..10 {
            mem.record(MemoryEvent {
                timestamp: i as f32,
                event_type: MemoryType::Discovery,
                position: [0.0; 3],
                description: format!("e{}", i),
            });
        }
        assert_eq!(mem.events.len(), 3, "应限制为 max_events");
        assert_eq!(mem.events[0].description, "e7", "最旧的应被删除: {}", mem.events[0].description);
    }

    #[test]
    fn test_memory_recent_dangers() {
        let mut mem = Memory::default();
        mem.record(MemoryEvent {
            timestamp: 1.0,
            event_type: MemoryType::Danger,
            position: [5.0, 0.0, 0.0],
            description: "fire".into(),
        });
        mem.record(MemoryEvent {
            timestamp: 5.0,
            event_type: MemoryType::Resource,
            position: [0.0, 0.0, 0.0],
            description: "water".into(),
        });
        mem.record(MemoryEvent {
            timestamp: 8.0,
            event_type: MemoryType::Danger,
            position: [10.0, 0.0, 0.0],
            description: "enemy".into(),
        });
        // 查询 5 秒内的危险（current_time=10）
        let dangers = mem.recent_dangers(5.0, 10.0);
        assert_eq!(dangers.len(), 1, "应只有 1 个近期危险");
        assert_eq!(dangers[0].description, "enemy");
    }

    #[test]
    fn test_memory_known_resources() {
        let mut mem = Memory::default();
        mem.record(MemoryEvent {
            timestamp: 1.0,
            event_type: MemoryType::Resource,
            position: [1.0, 0.0, 0.0],
            description: "water".into(),
        });
        mem.record(MemoryEvent {
            timestamp: 2.0,
            event_type: MemoryType::Resource,
            position: [2.0, 0.0, 0.0],
            description: "food".into(),
        });
        mem.record(MemoryEvent {
            timestamp: 3.0,
            event_type: MemoryType::Shelter,
            position: [3.0, 0.0, 0.0],
            description: "cave".into(),
        });
        let resources = mem.known_resources(MemoryType::Resource);
        assert_eq!(resources.len(), 2, "应有 2 个资源记忆");
        let shelters = mem.known_resources(MemoryType::Shelter);
        assert_eq!(shelters.len(), 1, "应有 1 个庇护所记忆");
    }

    #[test]
    fn test_remember_event() {
        let mut s = CognitiveState::new(Personality::default());
        s.remember(MemoryEvent {
            timestamp: 1.0,
            event_type: MemoryType::Danger,
            position: [5.0, 0.0, 0.0],
            description: "fire".into(),
        });
        assert_eq!(s.memory.events.len(), 1);
        let dangers = s.memory.recent_dangers(10.0, 5.0);
        assert_eq!(dangers.len(), 1);
    }

    // ─── 性格影响测试 ───

    #[test]
    fn test_personality_neuroticism_affects_flee() {
        // 高神经质 + near_fire（Safety urgency=0.4）→ Flee（阈值 0.3）
        let high_neuro = Personality {
            neuroticism: 0.8,
            ..Personality::default()
        };
        let mut s = CognitiveState::new(high_neuro);
        s.decision_interval = 0.0;
        s.update_needs(310.15, 0.98, 5.0, 0.0, 0.0, 0.0, 1.0, false, false, true);
        let action = s.decide([0.0, 0.0, 0.0], 1.0);
        assert_eq!(action.action_type, ActionType::Flee, "高神经质+近火应逃跑");

        // 低神经质 + near_fire → 不逃跑（阈值 0.5，urgency 0.4 不够）
        let low_neuro = Personality {
            neuroticism: 0.4,
            ..Personality::default()
        };
        let mut s = CognitiveState::new(low_neuro);
        s.decision_interval = 0.0;
        s.update_needs(310.15, 0.98, 5.0, 0.0, 0.0, 0.0, 1.0, false, false, true);
        let action = s.decide([0.0, 0.0, 0.0], 1.0);
        assert_ne!(action.action_type, ActionType::Flee, "低神经质+近火不应逃跑");
    }

    #[test]
    fn test_personality_extraversion_affects_social() {
        // 高外向 → Social urgency = 0.3 + 0.2 = 0.5 > 0.4 → Talk
        let p = Personality {
            extraversion: 0.8,
            ..Personality::default()
        };
        let mut s = healthy_state(p);
        let action = s.decide([0.0, 0.0, 0.0], 1.0);
        assert_eq!(action.action_type, ActionType::Talk, "高外向应倾向社交");

        // 默认性格 → Social urgency = 0.3 < 0.4 → Idle
        let mut s = healthy_state(Personality::default());
        let action = s.decide([0.0, 0.0, 0.0], 1.0);
        assert_eq!(action.action_type, ActionType::Idle, "默认性格应待机");
    }

    // ─── 行为进度测试 ───

    #[test]
    fn test_tick_action_progress() {
        let mut s = CognitiveState::new(Personality::default());
        s.current_action = Action {
            action_type: ActionType::Sleep,
            target_position: None,
            target_id: None,
            duration: 10.0,
            priority: 0.5,
        };
        s.action_progress = 0.0;

        // 推进 5s → 50% 进度，未完成
        assert!(!s.tick_action(5.0));
        assert!(
            (s.action_progress - 0.5).abs() < 0.01,
            "进度应为 0.5: {}",
            s.action_progress
        );

        // 再推进 5s → 100%，完成
        assert!(s.tick_action(5.0));
        assert!(
            (s.action_progress - 1.0).abs() < 0.01,
            "进度应为 1.0: {}",
            s.action_progress
        );
    }

    #[test]
    fn test_tick_action_zero_duration() {
        let mut s = CognitiveState::new(Personality::default());
        s.current_action = Action {
            action_type: ActionType::Idle,
            target_position: None,
            target_id: None,
            duration: 0.0,
            priority: 0.0,
        };
        assert!(s.tick_action(1.0), "持续时间为 0 应立即完成");
    }

    // ─── 需求优先级排序测试 ───

    #[test]
    fn test_need_priority_weight() {
        assert!((Need::Breathe.priority_weight() - 1.0).abs() < 0.001);
        assert!((Need::Survive.priority_weight() - 1.0).abs() < 0.001);
        assert!((Need::Thirst.priority_weight() - 0.8).abs() < 0.001);
        assert!((Need::Hunger.priority_weight() - 0.7).abs() < 0.001);
        assert!((Need::Rest.priority_weight() - 0.6).abs() < 0.001);
        assert!((Need::Safety.priority_weight() - 0.5).abs() < 0.001);
        assert!((Need::Shelter.priority_weight() - 0.4).abs() < 0.001);
        assert!((Need::Health.priority_weight() - 0.5).abs() < 0.001);
        assert!((Need::Social.priority_weight() - 0.2).abs() < 0.001);
        assert!((Need::Explore.priority_weight() - 0.1).abs() < 0.001);
        assert!((Need::Work.priority_weight() - 0.15).abs() < 0.001);
    }

    #[test]
    fn test_need_state_priority_sorting() {
        let breathe = NeedState::new(Need::Breathe, 0.5);
        let social = NeedState::new(Need::Social, 1.0);
        let thirst = NeedState::new(Need::Thirst, 0.8);

        // priority: breathe=0.5*1.0=0.5, social=1.0*0.2=0.2, thirst=0.8*0.8=0.64
        assert!((breathe.priority - 0.5).abs() < 0.001, "Breathe priority: {}", breathe.priority);
        assert!((social.priority - 0.2).abs() < 0.001, "Social priority: {}", social.priority);
        assert!((thirst.priority - 0.64).abs() < 0.001, "Thirst priority: {}", thirst.priority);

        // 排序：thirst(0.64) > breathe(0.5) > social(0.2)
        let mut needs = vec![breathe, social, thirst];
        needs.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        assert_eq!(needs[0].need, Need::Thirst);
        assert_eq!(needs[1].need, Need::Breathe);
        assert_eq!(needs[2].need, Need::Social);
    }

    // ─── 随机性格生成测试 ───

    #[test]
    fn test_personality_random_deterministic() {
        // 相同种子应产生相同性格
        let p1 = Personality::random_with_seed(42);
        let p2 = Personality::random_with_seed(42);
        assert!((p1.openness - p2.openness).abs() < 1e-6, "相同种子开放性应相同");
        assert!((p1.neuroticism - p2.neuroticism).abs() < 1e-6, "相同种子神经质应相同");
        // 所有值应在 0..1 范围
        assert!(p1.openness >= 0.0 && p1.openness <= 1.0);
        assert!(p1.conscientiousness >= 0.0 && p1.conscientiousness <= 1.0);
        assert!(p1.extraversion >= 0.0 && p1.extraversion <= 1.0);
        assert!(p1.agreeableness >= 0.0 && p1.agreeableness <= 1.0);
        assert!(p1.neuroticism >= 0.0 && p1.neuroticism <= 1.0);
    }

    #[test]
    fn test_personality_random_different_seeds() {
        let p1 = Personality::random_with_seed(1);
        let p2 = Personality::random_with_seed(999);
        // 不同种子应产生不同性格（至少一个维度应不同）
        let diff = (p1.openness - p2.openness).abs()
            + (p1.conscientiousness - p2.conscientiousness).abs()
            + (p1.extraversion - p2.extraversion).abs()
            + (p1.agreeableness - p2.agreeableness).abs()
            + (p1.neuroticism - p2.neuroticism).abs();
        assert!(diff > 0.01, "不同种子应产生不同性格: diff={}", diff);
    }
}
