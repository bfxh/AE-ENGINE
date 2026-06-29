//! 跨域事件类型定义
//!
//! 定义子系统间跨域效果传播的事件类型，用于解耦各层之间的直接依赖。

use std::any::Any;

use glam::Vec3;

use super::event::{Event, EventType};

// 事件类型 ID 常量
pub const COLLISION_DAMAGE: EventType = EventType::new(1);
pub const CHEMICAL_REACTION: EventType = EventType::new(2);

/// 伤害类型枚举（跨域通用）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossDomainDamageType {
    Explosive,
    Kinetic,
    Piercing,
    Thermal,
    Chemical,
    Radiation,
    Toxic,
    Biological,
}

/// 碰撞伤害事件
///
/// 由物理系统发布，触发化学/物理/游戏逻辑层的响应。
#[derive(Debug, Clone)]
pub struct CollisionDamageEvent {
    pub damage_type: CrossDomainDamageType,
    pub position: Vec3,
    pub radius: f32,
    pub damage: f32,
}

impl Event for CollisionDamageEvent {
    fn event_type(&self) -> EventType {
        COLLISION_DAMAGE
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// 化学反应类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossDomainReactionType {
    Explosion,
    Combustion,
    RadioactiveDecay,
    Corrosion,
    Oxidation,
    Other,
}

/// 化学副产品信息
#[derive(Debug, Clone)]
pub struct ChemicalByproductInfo {
    pub hazard: CrossDomainHazardType,
    pub amount: f32,
    pub spread_radius: f32,
    pub duration: f32,
}

/// 危害类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossDomainHazardType {
    Radiation,
    ToxicFumes,
    BiologicalContamination,
    Other,
}

/// 化学反应完成事件
///
/// 由化学系统发布，触发物理/游戏逻辑层的响应。
#[derive(Debug, Clone)]
pub struct ChemicalReactionEvent {
    pub reaction_type: CrossDomainReactionType,
    pub position: Vec3,
    pub energy_released: f32,
    pub byproducts: Vec<ChemicalByproductInfo>,
}

impl Event for ChemicalReactionEvent {
    fn event_type(&self) -> EventType {
        CHEMICAL_REACTION
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
