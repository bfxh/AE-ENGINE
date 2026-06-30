//! NPC 系统
//!
//! 游戏层 NPC 逻辑：AI/管理/脚本/优化

slotmap::new_key_type! { pub struct NpcId; }

pub mod ai_optimizer;
pub mod manager;
pub mod scripting;
pub mod system;

pub use ai_optimizer::*;
pub use manager::*;
pub use scripting::*;
pub use system::*;
