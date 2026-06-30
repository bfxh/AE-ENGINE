//! Wasteland Game Logic Layer
//!
//! 本 crate 包含废土生存游戏的具体游戏逻辑，与引擎核心（ae_engine）解耦。
//!
//! ## 架构分层
//!
//! - **引擎层** (`ae_engine`): 通用框架 — 物理/化学/生物/渲染/ECS/事件总线/管理器
//! - **游戏层** (`ae_game`): 具体游戏内容 — 战斗/NPC/建筑/制作/经济
//!
//! ## 模块
//!
//! - `combat`: 战斗系统（实体/武器/伤害/投射物）
//! - `npc`: NPC 系统（AI/管理/脚本/优化）
//! - `building`: 建筑编辑器
//! - `economy`: 经济系统（物品/背包/配方/交易/战利品）
//! - `infection`: 感染系统（体素/阶段/实体/治疗）

pub mod building;
pub mod combat;
pub mod economy;
pub mod infection;
pub mod npc;

pub use building::*;
pub use combat::*;
pub use economy::*;
pub use infection::*;
pub use npc::*;
