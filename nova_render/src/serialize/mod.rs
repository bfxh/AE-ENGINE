//! 序列化模块（借鉴 Fyrox Visit trait）
//!
//! 设计：
//! - Visit trait：统一序列化 IR
//! - Prefab：属性继承 + 层级嵌套

pub mod visit;
pub mod prefab;

pub use visit::{Visit, VisitContext, VisitError, Visitor};
pub use prefab::{Prefab, PrefabNode, PrefabRegistry};