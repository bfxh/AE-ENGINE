//! 兼容层
//!
//! - `compat-v1`: 对接 v1 ae_render（资源类型转换）
//! - `engine-bridge`: 通过 `GameWorldSource` trait 对接上层引擎（从 GameWorld 提取渲染数据）
//!
//! `engine-bridge` 不直接依赖 `ae_engine`，由 game crate 为具体引擎类型
//! impl `GameWorldSource`，nova_render 只依赖 trait。

#[cfg(feature = "compat-v1")]
pub mod v1_adapter;

#[cfg(feature = "engine-bridge")]
pub mod engine_bridge;

#[cfg(feature = "compat-v1")]
pub use v1_adapter::{V1Adapter, V1MeshConverter, V1TextureConverter};

#[cfg(feature = "engine-bridge")]
pub use engine_bridge::{
    DomainZoneInfo, EngineBridge, ExtractedSceneData, GameWorldExtractor, GameWorldSource,
    WorldBounds, WorldStats,
};