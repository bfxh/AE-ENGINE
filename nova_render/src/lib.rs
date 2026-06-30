//! Nova Render - v2 高可维护渲染框架
//!
//! 设计理念：
//! - **双 World 分离** (借鉴 bevy): MainWorld 逻辑侧 ↔ RenderWorld 渲染侧
//! - **四阶段管线** (借鉴 bevy): Extract → Prepare → Queue → Render
//! - **Handle + Arc 资源** (借鉴 rend3): 强类型引用计数资源管理
//! - **Pool + Generational Arena** (借鉴 Fyrox): O(1) 索引访问 + 悬空检测
//! - **RenderGraph** (借鉴 kajiya): DAG + temporal + imageops 声明式渲染
//! - **Material Trait ABI** (借鉴 rend3): 材质作为可扩展 trait
//! - **Visit Trait IR** (借鉴 Fyrox): 统一序列化中间表示
//! - **GraphicsServer trait** (借鉴 Fyrox): 后端抽象 + wgpu 实现
//! - **乒乓球纹理** (借鉴 bevy): 后处理栈高效读写
//! - **PipelineCache** (借鉴 bevy): 异步管线编译
//! - **profiling crate** (借鉴 rend3): 零开销性能追踪
//!
//! 模块架构：
//! ```text
//! core/         基础设施 (Handle, Pool, World 分离, Extract)
//! backend/      GraphicsServer trait + wgpu 实现 + PipelineCache
//! render_graph/ RenderGraph (DAG + temporal + imageops)
//! assets/       Mesh + Megabuffer + Texture + Material trait + Shader
//! scene/        Scene Graph + Node + Camera + Light + Prefab
//! renderer/     PhaseItem + RenderPhase + Culling + Batch
//! passes/       Shadow / Forward / Skybox / Water / Particles
//! post_process/ EffectStack + Bloom / Tonemap / TAA / SSAO / SSR / ...
//! gi/           DDGI + SSGI + RT
//! serialize/    Visit trait + Prefab
//! compat/       v1_adapter 对接 ae_engine
//! profiling/    性能追踪
//! ```

#![allow(clippy::module_inception)]
#![allow(clippy::too_many_arguments)]

pub mod core;
pub mod backend;
pub mod render_graph;
pub mod assets;
pub mod scene;
pub mod renderer;
pub mod passes;
pub mod post_process;
pub mod gi;
pub mod serialize;
pub mod compat;
pub mod profiling;
pub mod procedural;
pub mod application;

pub use core::{Handle, Pool, World, MainWorld, RenderWorld};
pub use backend::{GraphicsServer, WgpuBackend, PipelineCache};

/// Nova Render 版本
pub const VERSION: &str = "0.1.0";