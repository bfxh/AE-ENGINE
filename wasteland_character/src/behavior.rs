//! behavior.rs - 角色行为系统
//!
//! 覆盖：
//! - 需求系统（Maslow 层次：生理/安全/社交/尊重/自我实现）
//! - 情绪系统（Plutchik 八基本情绪 + 价/唤醒度）
//! - 行为状态机（FSM）
//! - 行为树（Selector/Sequence/Action/Condition/Decorator）
//! - 动机驱动与决策
//! - 群体行为（Reynolds 1987 Boids）
//!
//! 参考：
//! - Maslow 1943 "A Theory of Human Motivation"
//! - Plutchik 1980 "A General Psychoevolutionary Theory of Emotion"
//! - OCC 情绪模型 (Ortony/Clore/Collins 1988)
//! - Reynolds 1987 "Flocks, Herds and Schools"

use serde::{Deserialize, Serialize};

// ============================================================================
// 1. 需求系统（Maslow 层次）
// ============================================================================

/// 需求类型（Maslow 层次结构）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NeedType {
    Hunger,       // 饥饿
    Thirst,       // 口渴
    Fatigue,      // 疲劳/睡眠
    Temperature,  // 体温调节
    Oxygen,       // 氧气
    Pain,         // 疼痛回避
    Safety,       // 安全感
    Shelter,      // 庇护所
    Health,       // 健康
    Social,       // 社交
    Belonging,    // 归属感
    Affection,    // 情感
    Esteem,       // 自尊
    Recognition,  // 认可
    Achievement,  // 成就
    SelfActualization, // 自我实现
    Creativity,   // 创造
    Learning,     // 学习
}

/// Maslow 需求层次
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaslowLevel {
    Physiological,
    Safety,
    Love,
    Esteem,
    SelfActualization,
}

impl NeedType {
    pub fn maslow_level(&self) -> MaslowLevel {
        match self {
            NeedType::Hunger | NeedType::Thirst | NeedType::Fatigue
            | NeedType::Temperature | NeedType::Oxygen | NeedType::Pain => MaslowLevel::Physiological,
            NeedType::Safety | NeedType::Shelter | NeedType::Health => MaslowLevel::Safety,
            NeedType::Social | NeedType::Belonging | NeedType::Affection => MaslowLevel::Love,
            NeedType::Esteem | NeedType::Recognition | NeedType::Achievement => MaslowLevel::Esteem,
            NeedType::SelfActualization | NeedType::Creativity | NeedType::Learning => MaslowLevel::SelfActualization,
        }
    }

    /// 需求权重（低层次需求优先级更高）
    pub fn priority_weight(&self) -> f32 {
        match self.maslow_level() {
            MaslowLevel::Physiological => 1.0,
            MaslowLevel::Safety => 0.8,
            MaslowLevel::Love => 0.6,
            MaslowLevel::Esteem => 0.4,
            MaslowLevel::SelfActualization => 0.2,
        }
    }
}

/// 单个需求状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Need {
    pub need_type: NeedType,
    /// 当前值 0..1（0=完全满足，1=极度匮乏）
    pub value: f32,
    /// 增长速率（每秒增加多少）
    pub growth_rate: f32,
    /// 满足阈值
    pub satisfaction_threshold: f32,
    /// 紧急阈值
    pub critical_threshold: f32,
}

impl Need {
    pub fn new(need_type: NeedType, growth_rate: f32) -> Self {
        Self {
            need_type,
            value: 0.0,
            growth_rate,
            satisfaction_threshold: 0.3,
            critical_threshold: 0.8,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.value = (self.value + self.growth_rate * dt).min(1.0);
    }

    pub fn satisfy(&mut self, amount: f32) {
        self.value = (self.value - amount).max(0.0);
    }

    pub fn is_critical(&self) -> bool {
        self.value >= self.critical_threshold
    }

    pub fn is_satisfied(&self) -> bool {
        self.value <= self.satisfaction_threshold
    }

    /// 紧迫度（综合值和优先级权重）
    pub fn urgency(&self) -> f32 {
        let weight = self.need_type.priority_weight();
        let level = if self.is_critical() { 1.0 } else { self.value };
        level * weight
    }
}

// ============================================================================
// 2. 情绪系统（Plutchik 八基本情绪）
// ============================================================================

/// Plutchik 八基本情绪
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BasicEmotion {
    Joy,
    Trust,
    Fear,
    Surprise,
    Sadness,
    Disgust,
    Anger,
    Anticipation,
}

/// 情绪状态（强度 0..1）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionState {
    pub joy: f32,
    pub trust: f32,
    pub fear: f32,
    pub surprise: f32,
    pub sadness: f32,
    pub disgust: f32,
    pub anger: f32,
    pub anticipation: f32,
}

impl Default for EmotionState {
    fn default() -> Self {
        Self {
            joy: 0.5, trust: 0.5, fear: 0.0, surprise: 0.0,
            sadness: 0.0, disgust: 0.0, anger: 0.0, anticipation: 0.3,
        }
    }
}

impl EmotionState {
    pub fn intensity(&self, emotion: BasicEmotion) -> f32 {
        match emotion {
            BasicEmotion::Joy => self.joy,
            BasicEmotion::Trust => self.trust,
            BasicEmotion::Fear => self.fear,
            BasicEmotion::Surprise => self.surprise,
            BasicEmotion::Sadness => self.sadness,
            BasicEmotion::Disgust => self.disgust,
            BasicEmotion::Anger => self.anger,
            BasicEmotion::Anticipation => self.anticipation,
        }
    }

    pub fn set_intensity(&mut self, emotion: BasicEmotion, value: f32) {
        let v = value.clamp(0.0, 1.0);
        match emotion {
            BasicEmotion::Joy => self.joy = v,
            BasicEmotion::Trust => self.trust = v,
            BasicEmotion::Fear => self.fear = v,
            BasicEmotion::Surprise => self.surprise = v,
            BasicEmotion::Sadness => self.sadness = v,
            BasicEmotion::Disgust => self.disgust = v,
            BasicEmotion::Anger => self.anger = v,
            BasicEmotion::Anticipation => self.anticipation = v,
        }
    }

    /// 情绪衰减（随时间回归基线）
    pub fn decay(&mut self, dt: f32, decay_rate: f32) {
        let baseline = EmotionState::default();
        self.joy = lerp(self.joy, baseline.joy, decay_rate * dt);
        self.trust = lerp(self.trust, baseline.trust, decay_rate * dt);
        self.fear = lerp(self.fear, baseline.fear, decay_rate * dt);
        self.surprise = lerp(self.surprise, baseline.surprise, decay_rate * dt);
        self.sadness = lerp(self.sadness, baseline.sadness, decay_rate * dt);
        self.disgust = lerp(self.disgust, baseline.disgust, decay_rate * dt);
        self.anger = lerp(self.anger, baseline.anger, decay_rate * dt);
        self.anticipation = lerp(self.anticipation, baseline.anticipation, decay_rate * dt);
    }

    /// 情绪价（valence: -1=消极, +1=积极）
    pub fn valence(&self) -> f32 {
        self.joy + self.trust + self.anticipation
            - self.fear - self.sadness - self.disgust - self.anger
    }

    /// 情绪唤醒度（arousal: 0=平静, 1=极度兴奋）
    pub fn arousal(&self) -> f32 {
        (self.fear + self.surprise + self.anger + self.joy) * 0.25
    }

    /// 主导情绪
    pub fn dominant_emotion(&self) -> BasicEmotion {
        let emotions = [
            (BasicEmotion::Joy, self.joy),
            (BasicEmotion::Trust, self.trust),
            (BasicEmotion::Fear, self.fear),
            (BasicEmotion::Surprise, self.surprise),
            (BasicEmotion::Sadness, self.sadness),
            (BasicEmotion::Disgust, self.disgust),
            (BasicEmotion::Anger, self.anger),
            (BasicEmotion::Anticipation, self.anticipation),
        ];
        emotions.iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(e, _)| *e)
            .unwrap_or(BasicEmotion::Anticipation)
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

// ============================================================================
// 3. 行为状态机（FSM）
// ============================================================================

/// 角色行为状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BehaviorState {
    Idle,
    Wandering,
    SeekingFood,
    SeekingWater,
    Sleeping,
    Fleeing,
    Fighting,
    Working,
    Socializing,
    Exploring,
    Crafting,
    Resting,
    Panic,
}

impl BehaviorState {
    pub fn is_interruptible(&self) -> bool {
        !matches!(self, BehaviorState::Fighting | BehaviorState::Panic)
    }

    /// 能量消耗/恢复速率（负=恢复）
    pub fn energy_cost(&self) -> f32 {
        match self {
            BehaviorState::Sleeping => -0.1,
            BehaviorState::Resting => -0.05,
            BehaviorState::Idle => 0.0,
            BehaviorState::Wandering => 0.1,
            BehaviorState::Socializing | BehaviorState::Exploring => 0.15,
            BehaviorState::SeekingFood | BehaviorState::SeekingWater => 0.2,
            BehaviorState::Working | BehaviorState::Crafting => 0.3,
            BehaviorState::Fleeing => 0.5,
            BehaviorState::Fighting => 0.6,
            BehaviorState::Panic => 0.7,
        }
    }
}

/// 状态转换条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: BehaviorState,
    pub to: BehaviorState,
    pub condition: TransitionCondition,
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionCondition {
    NeedCritical(NeedType),
    NeedSatisfied(NeedType),
    ThreatDetected,
    ThreatGone,
    LowEnergy,
    DayTime,
    NightTime,
    Custom(String),
}

// ============================================================================
// 4. 行为树
// ============================================================================

/// 行为树节点状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    Success,
    Failure,
    Running,
}

/// 行为树节点类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BehaviorNode {
    Action { name: String, action_type: ActionType },
    Condition { name: String, condition: ConditionCheck },
    Selector { children: Vec<BehaviorNode> },
    Sequence { children: Vec<BehaviorNode> },
    Decorator { child: Box<BehaviorNode>, decorator: DecoratorType },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    MoveToTarget,
    AttackTarget,
    FleeFromThreat,
    EatFood,
    DrinkWater,
    Sleep,
    GatherResource,
    CraftItem,
    Communicate,
    Patrol,
    Guard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionCheck {
    IsHungry,
    IsThreatened,
    IsTired,
    HasTarget,
    HasResource,
    IsDayTime,
    IsNearAlly,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecoratorType {
    Inverter,
    Repeater(u32),
    UntilFailure,
    TimeLimit(f32),
}

// ============================================================================
// 5. 动机与决策
// ============================================================================

/// 动机（驱动行为选择）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Motivation {
    pub behavior: BehaviorState,
    pub score: f32,
    pub driving_need: Option<NeedType>,
}

/// 角色行为系统
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorSystem {
    pub needs: Vec<Need>,
    pub emotions: EmotionState,
    pub current_state: BehaviorState,
    pub state_duration: f32,
    pub energy: f32,
    pub morale: f32,
}

impl Default for BehaviorSystem {
    fn default() -> Self {
        Self {
            needs: default_needs(),
            emotions: EmotionState::default(),
            current_state: BehaviorState::Idle,
            state_duration: 0.0,
            energy: 1.0,
            morale: 0.7,
        }
    }
}

fn default_needs() -> Vec<Need> {
    vec![
        Need::new(NeedType::Hunger, 0.0008),
        Need::new(NeedType::Thirst, 0.0012),
        Need::new(NeedType::Fatigue, 0.0005),
        Need::new(NeedType::Safety, 0.0003),
        Need::new(NeedType::Social, 0.0002),
        Need::new(NeedType::Esteem, 0.0001),
    ]
}

impl BehaviorSystem {
    /// 更新行为系统
    pub fn update(&mut self, dt: f32) {
        for need in &mut self.needs {
            need.update(dt);
        }
        self.emotions.decay(dt, 0.1);
        let cost = self.current_state.energy_cost();
        self.energy = (self.energy - cost * dt * 0.01).clamp(0.0, 1.0);
        self.state_duration += dt;
    }

    /// 最紧迫的需求
    pub fn most_urgent_need(&self) -> Option<&Need> {
        self.needs.iter()
            .max_by(|a, b| a.urgency().partial_cmp(&b.urgency()).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// 生成动机列表（按分数降序）
    pub fn compute_motivations(&self) -> Vec<Motivation> {
        let mut motivations = Vec::new();
        for need in &self.needs {
            if need.is_critical() {
                let behavior = match need.need_type {
                    NeedType::Hunger => BehaviorState::SeekingFood,
                    NeedType::Thirst => BehaviorState::SeekingWater,
                    NeedType::Fatigue => BehaviorState::Sleeping,
                    NeedType::Safety => BehaviorState::Fleeing,
                    NeedType::Social => BehaviorState::Socializing,
                    _ => continue,
                };
                motivations.push(Motivation {
                    behavior,
                    score: need.urgency(),
                    driving_need: Some(need.need_type),
                });
            }
        }
        if self.energy < 0.2 {
            motivations.push(Motivation {
                behavior: BehaviorState::Resting,
                score: (0.2 - self.energy) * 5.0,
                driving_need: Some(NeedType::Fatigue),
            });
        }
        if self.morale < 0.3 {
            motivations.push(Motivation {
                behavior: BehaviorState::Panic,
                score: (0.3 - self.morale) * 3.0,
                driving_need: Some(NeedType::Safety),
            });
        }
        if motivations.is_empty() {
            motivations.push(Motivation {
                behavior: BehaviorState::Idle,
                score: 0.1,
                driving_need: None,
            });
        }
        motivations.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        motivations
    }

    /// 选择行为
    pub fn select_behavior(&mut self) -> BehaviorState {
        let motivations = self.compute_motivations();
        if let Some(best) = motivations.first() {
            if best.behavior != self.current_state {
                self.current_state = best.behavior;
                self.state_duration = 0.0;
            }
        }
        self.current_state
    }

    pub fn satisfy_need(&mut self, need_type: NeedType, amount: f32) {
        if let Some(need) = self.needs.iter_mut().find(|n| n.need_type == need_type) {
            need.satisfy(amount);
        }
    }

    pub fn perceive_threat(&mut self, threat_level: f32) {
        self.emotions.set_intensity(BasicEmotion::Fear, threat_level);
        if threat_level > 0.7 {
            self.morale = (self.morale - 0.2).clamp(0.0, 1.0);
        }
    }

    /// 整体福祉指数（0..1）
    pub fn wellbeing(&self) -> f32 {
        let need_avg: f32 = self.needs.iter()
            .map(|n| 1.0 - n.value)
            .sum::<f32>() / self.needs.len().max(1) as f32;
        let emotion_val = (self.emotions.valence() + 1.0) * 0.5;
        (need_avg * 0.5 + emotion_val * 0.2 + self.energy * 0.2 + self.morale * 0.1).clamp(0.0, 1.0)
    }
}

// ============================================================================
// 6. 群体行为（Reynolds 1987 Boids）
// ============================================================================

/// 群体行为参数
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FlockParams {
    pub separation_radius: f32,
    pub alignment_radius: f32,
    pub cohesion_radius: f32,
    pub separation_weight: f32,
    pub alignment_weight: f32,
    pub cohesion_weight: f32,
}

impl Default for FlockParams {
    fn default() -> Self {
        Self {
            separation_radius: 1.5,
            alignment_radius: 5.0,
            cohesion_radius: 5.0,
            separation_weight: 1.5,
            alignment_weight: 1.0,
            cohesion_weight: 1.0,
        }
    }
}

/// Boids 群体行为力计算
pub fn flock_force(
    self_pos: [f32; 3],
    self_vel: [f32; 3],
    neighbors_pos: &[[f32; 3]],
    neighbors_vel: &[[f32; 3]],
    params: &FlockParams,
) -> [f32; 3] {
    if neighbors_pos.is_empty() {
        return [0.0; 3];
    }
    let mut sep = [0.0f32; 3];
    let mut align = [0.0f32; 3];
    let mut cohes = [0.0f32; 3];
    let mut sep_count = 0u32;
    let mut align_count = 0u32;
    let mut cohes_count = 0u32;
    for (npos, nvel) in neighbors_pos.iter().zip(neighbors_vel.iter()) {
        let dist = ((npos[0] - self_pos[0]).powi(2)
            + (npos[1] - self_pos[1]).powi(2)
            + (npos[2] - self_pos[2]).powi(2)).sqrt();
        if dist < params.separation_radius && dist > 0.0 {
            let factor = 1.0 / dist;
            sep[0] += (self_pos[0] - npos[0]) * factor;
            sep[1] += (self_pos[1] - npos[1]) * factor;
            sep[2] += (self_pos[2] - npos[2]) * factor;
            sep_count += 1;
        }
        if dist < params.alignment_radius {
            align[0] += nvel[0];
            align[1] += nvel[1];
            align[2] += nvel[2];
            align_count += 1;
        }
        if dist < params.cohesion_radius {
            cohes[0] += npos[0];
            cohes[1] += npos[1];
            cohes[2] += npos[2];
            cohes_count += 1;
        }
    }
    if sep_count > 0 {
        let c = sep_count as f32;
        sep[0] /= c; sep[1] /= c; sep[2] /= c;
    }
    if align_count > 0 {
        let c = align_count as f32;
        align[0] = align[0] / c - self_vel[0];
        align[1] = align[1] / c - self_vel[1];
        align[2] = align[2] / c - self_vel[2];
    }
    if cohes_count > 0 {
        let c = cohes_count as f32;
        cohes[0] = cohes[0] / c - self_pos[0];
        cohes[1] = cohes[1] / c - self_pos[1];
        cohes[2] = cohes[2] / c - self_pos[2];
    }
    [
        sep[0] * params.separation_weight + align[0] * params.alignment_weight + cohes[0] * params.cohesion_weight,
        sep[1] * params.separation_weight + align[1] * params.alignment_weight + cohes[1] * params.cohesion_weight,
        sep[2] * params.separation_weight + align[2] * params.alignment_weight + cohes[2] * params.cohesion_weight,
    ]
}

// ============================================================================
// 7. 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_need_growth_and_satisfaction() {
        let mut hunger = Need::new(NeedType::Hunger, 0.01);
        hunger.update(10.0);
        assert!(hunger.value > 0.0, "饥饿应增长");
        hunger.satisfy(0.5);
        assert!(hunger.value < 0.5, "满足后应降低");
    }

    #[test]
    fn test_need_urgency() {
        let critical = Need { value: 0.9, ..Need::new(NeedType::Hunger, 0.0) };
        let low = Need { value: 0.2, ..Need::new(NeedType::Hunger, 0.0) };
        assert!(critical.urgency() > low.urgency(), "临界需求紧迫度更高");
    }

    #[test]
    fn test_maslow_priority() {
        assert!(NeedType::Hunger.priority_weight() > NeedType::Social.priority_weight(),
            "生理需求优先级高于社交");
        assert!(NeedType::Safety.priority_weight() > NeedType::Esteem.priority_weight(),
            "安全需求优先级高于尊重");
    }

    #[test]
    fn test_emotion_decay() {
        let mut emo = EmotionState::default();
        emo.fear = 0.9;
        emo.decay(1.0, 0.5);
        assert!(emo.fear < 0.9, "恐惧应衰减");
    }

    #[test]
    fn test_emotion_valence() {
        let positive = EmotionState { joy: 0.8, trust: 0.7, fear: 0.0, surprise: 0.0,
            sadness: 0.0, disgust: 0.0, anger: 0.0, anticipation: 0.6 };
        assert!(positive.valence() > 0.0, "积极情绪价应为正");
        let negative = EmotionState { joy: 0.1, trust: 0.1, fear: 0.8, surprise: 0.0,
            sadness: 0.7, disgust: 0.5, anger: 0.6, anticipation: 0.1 };
        assert!(negative.valence() < 0.0, "消极情绪价应为负");
    }

    #[test]
    fn test_behavior_system_update() {
        let mut bs = BehaviorSystem::default();
        bs.update(100.0);
        assert!(bs.needs.iter().any(|n| n.value > 0.0), "需求应随时间增长");
    }

    #[test]
    fn test_behavior_selection() {
        let mut bs = BehaviorSystem::default();
        if let Some(hunger) = bs.needs.iter_mut().find(|n| n.need_type == NeedType::Hunger) {
            hunger.value = 0.95;
        }
        let selected = bs.select_behavior();
        assert_eq!(selected, BehaviorState::SeekingFood, "饥饿时应寻找食物");
    }

    #[test]
    fn test_wellbeing() {
        let bs = BehaviorSystem::default();
        assert!(bs.wellbeing() > 0.5, "初始福祉应较高");
    }

    #[test]
    fn test_flock_force() {
        let params = FlockParams::default();
        let force = flock_force(
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            &[[1.0, 0.0, 0.0]],
            &[[1.0, 0.0, 0.0]],
            &params,
        );
        assert!(force[0] != 0.0 || force[1] != 0.0 || force[2] != 0.0, "应有群体力");
    }

    #[test]
    fn test_perceive_threat() {
        let mut bs = BehaviorSystem::default();
        bs.perceive_threat(0.8);
        assert!(bs.emotions.fear > 0.5, "高威胁应引发恐惧");
        assert!(bs.morale < 0.7, "威胁应降低士气");
    }

    #[test]
    fn test_energy_cost() {
        assert!(BehaviorState::Fighting.energy_cost() > BehaviorState::Idle.energy_cost());
        assert!(BehaviorState::Sleeping.energy_cost() < 0.0, "睡眠应恢复能量");
    }
}
