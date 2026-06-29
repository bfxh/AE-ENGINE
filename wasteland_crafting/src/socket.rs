use glam::Mat4;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SlotType {
    Blade,
    Handle,
    Guard,
    Pommel,
    Engine,
    Armor,
    Barrel,
    Stock,
    Scope,
    Magazine,
    Core,
    Frame,
    Decoration,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Socket {
    pub slot_type: SlotType,
    pub compatible_tags: Vec<String>,
    pub required: bool,
    pub transform: Mat4,
    pub max_parts: u8,
    pub label: String,
}

impl Socket {
    pub fn new(slot_type: SlotType, transform: Mat4, required: bool) -> Self {
        Self {
            slot_type,
            compatible_tags: Vec::new(),
            required,
            transform,
            max_parts: 1,
            label: String::new(),
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.compatible_tags = tags;
        self
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    pub fn is_compatible(&self, part_tags: &[String]) -> bool {
        if self.compatible_tags.is_empty() {
            return true;
        }
        part_tags.iter().any(|t| self.compatible_tags.contains(t))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketConnection {
    pub socket_index: usize,
    pub part_id: uuid::Uuid,
    pub part_tags: Vec<String>,
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub constraint_type: ConstraintType,
    pub socket_a: usize,
    pub socket_b: usize,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintType {
    /// 互斥: 不能同时存在
    MutuallyExclusive,
    /// 依赖: B必须存在
    RequiresB,
    /// 排斥: 如果A存在, 不能有B
    Excludes,
    /// 协同: 两者同时存在时加成
    Synergy,
    /// 替代: B可以替代A
    Alternative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionRule {
    pub rule_type: FusionType,
    pub source_sockets: Vec<usize>,
    pub target_socket: usize,
    pub operation: FusionOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FusionType {
    WeightedAverage,
    Maximum,
    Minimum,
    Sum,
    Harmonic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FusionOperation {
    Hardness,
    Toughness,
    Mass,
    Durability,
    EdgeSharpness,
    ElementalDamage,
    SpecialEffect,
}
