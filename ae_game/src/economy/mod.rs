//! 经济系统
//!
//! 游戏层经济逻辑：物品/背包/配方/交易/战利品
//! 从 doomsday-survival 项目移植，适配 ae 类型系统

pub mod inventory;
pub mod item;
pub mod loot;
pub mod recipe;
pub mod trader;

pub use inventory::*;
pub use item::*;
pub use loot::*;
pub use recipe::*;
pub use trader::*;
