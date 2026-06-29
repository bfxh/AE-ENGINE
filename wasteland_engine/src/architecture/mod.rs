//! 引擎核心架构模块
//!
//! 提供系统调度器、事件总线和世界上下文，支持分层架构设计。

pub mod context;
pub mod cross_domain_events;
pub mod event;
pub mod spatial_hash;
pub mod system;

pub use context::WorldContext;
pub use cross_domain_events::{
    CHEMICAL_REACTION, COLLISION_DAMAGE, ChemicalByproductInfo, ChemicalReactionEvent,
    CollisionDamageEvent, CrossDomainDamageType, CrossDomainHazardType, CrossDomainReactionType,
};
pub use event::{Event, EventBus, EventCounterHandler, EventHandler, EventType};
pub use system::{System, SystemId, SystemScheduler};
