//! 核心基础设施模块
//!
//! 包含：
//! - `Handle<T>`: 强类型引用计数句柄（借鉴 Fyrox + rend3）
//! - `Pool<T>`: Generational Arena 分配器（借鉴 Fyrox）
//! - `World` + `MainWorld` + `RenderWorld`: 双世界分离（借鉴 bevy）

pub mod handle;
pub mod pool;
pub mod world;
pub mod extract;

pub use handle::{Handle, HandleError, WeakHandle};
pub use pool::{Pool, SlotIndex};
pub use world::{World, MainWorld, RenderWorld, WorldId};
pub use extract::{ExtractStage, ExtractContext};