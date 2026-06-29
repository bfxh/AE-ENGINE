//! 后端抽象层（借鉴 Fyrox GraphicsServer trait）
//!
//! 设计：
//! - `GraphicsServer` trait：抽象 GPU 后端（wgpu / Vulkan / DX12 / Metal）
//! - `WgpuBackend`：wgpu 24 实现
//! - `PipelineCache`：异步渲染管线编译缓存

pub mod server;
pub mod wgpu_backend;
pub mod pipeline_cache;

pub use server::{GraphicsServer, ServerCaps, ServerInfo};
pub use wgpu_backend::WgpuBackend;
pub use pipeline_cache::{PipelineCache, PipelineId, PipelineState};
