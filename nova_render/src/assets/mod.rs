//! 资源模块（Mesh + Texture + Material + Shader）
//!
//! 借鉴：
//! - rend3: RenderAsset 模式（CPU↔GPU 1:1）
//! - rend3: Material 作为 Trait（ABI 设计）
//! - rend3: Megabuffer 子分配
//! - Fyrox: UUID 资源系统

pub mod mesh;
pub mod texture;
pub mod material;
pub mod model;
pub mod shader;

pub use mesh::{Mesh, MeshData, Vertex, MeshHandle};
pub use texture::{Texture, TextureData, TextureHandle};
pub use material::{Material, MaterialHandle, MaterialInstance};
pub use model::{AlphaMode, GltfMaterial, Model, ModelFormat, ModelLoader, ModelNode};
pub use shader::{Shader, ShaderSource, ShaderHandle};