//! Wasteland Render - 统一渲染抽象层
//!
//! 基于 wgpu 24 的跨平台渲染系统，提供：
//! - GPU 设备抽象
//! - 纹理系统（PNG/DDS/KTX2）
//! - PBR 材质系统
//! - 网格与几何体
//! - glTF 模型加载
//! - Shader 管理
//! - 相机与视图
//! - 渲染管线

pub mod camera;
pub mod device;
pub mod instanced;
pub mod material;
pub mod mesh;
pub mod mesh_renderer;
pub mod model;
pub mod pipeline;
pub mod post_process;
pub mod procedural;
pub mod shader;
pub mod shadow_map;
pub mod skybox;
pub mod surface;
pub mod texture;
pub mod particles;
pub mod ssao;
pub mod ssr;
pub mod taa;
pub mod volumetric_fog;
pub mod water;

pub use camera::{Camera, CameraProjection, CameraUniform};
pub use device::RenderContext;
pub use instanced::{CubeVertex, InstancedRenderer, InstanceData, PointInstanceData};
pub use material::{Material, MaterialFlags, MaterialType, PbrMaterial, PbrMaterialParams};
pub use mesh::{Mesh, MeshBuilder, Vertex};
pub use mesh_renderer::{LightUniform, MeshInstanceData, MeshRenderer, RegisteredMesh};
pub use model::{GltfLoader, GltfModel, GltfPrimitive, ModelNode, Transform};
pub use pipeline::RenderPipelineCache;
pub use particles::{ParticleData, ParticleSystem, ParticleUniform};
pub use post_process::{PostProcessParams, PostProcessRenderer, PostProcessUniform};
pub use ssao::{SsaoNoiseSample, SsaoKernelSample, SsaoRenderer, SsaoUniform};
pub use shadow_map::{ShadowMapRenderer, ShadowUniform};
pub use skybox::{SkyboxRenderer, SunUniform};
pub use volumetric_fog::{FogUniform, VolumetricFogRenderer};
pub use water::{WaterRenderer, WaterUniform};
pub use shader::ShaderLibrary;
pub use surface::{SurfaceRenderer, CLEAR_COLOR};
pub use taa::{TaaRenderer, TaaUniform};
pub use texture::{Texture, TextureCache, TextureFormat, TextureLoader, TextureUsage};

/// 渲染系统版本
pub const VERSION: &str = "0.1.0";
