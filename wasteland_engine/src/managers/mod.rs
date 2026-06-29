//! 分层管理器模块
//!
//! 将 GameWorld 的职责按层级拆分为独立的管理器。

pub mod data;
pub mod domain_isolation;
pub mod game_logic;
pub mod modding;
pub mod rendering;
pub mod simulation;

pub use data::DataManager;
pub use game_logic::GameLogicManager;
pub use modding::ModdingManager;
pub use rendering::RenderingManager;
pub use simulation::SimulationManager;
