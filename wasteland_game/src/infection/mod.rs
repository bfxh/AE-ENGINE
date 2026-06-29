//! 感染系统
//!
//! 游戏层感染逻辑：体素网格/阶段/实体/治疗
//! 从 doomsday-survival 项目移植，适配 wasteland 类型系统

pub mod cure;
pub mod entity;
pub mod stage;
pub mod voxel;

pub use cure::*;
pub use entity::*;
pub use stage::*;
pub use voxel::*;
