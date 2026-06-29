//! 渲染 Pass 模块
//!
//! 借鉴：
//! - bevy RenderPass 注册式扩展
//! - v1 的 9-pass HDR 管线经验
//! - kajiya 后处理栈顺序

pub mod shadow;
pub mod shadow_map;
pub mod forward;
pub mod skybox;
pub mod water;
pub mod particles;
pub mod volumetric_fog;

pub use shadow::ShadowPass;
pub use forward::ForwardPass;
pub use skybox::SkyboxPass;
pub use water::WaterPass;
pub use particles::ParticlePass;
pub use volumetric_fog::{VolumetricFogPass, FogUniform};

pub mod svgf;
pub use svgf::{SvgfPass, SvgfConfig, SvgfTemporalUniform, SvgfFilterUniform};

pub mod visibility_buffer;
pub use visibility_buffer::{VisibilityBufferPass, VisibilityUniform, VisibilityData};

pub mod pcss;
pub use pcss::{PcssPass, PcssConfig, PcssUniform};

pub mod volumetric_fire;
pub use volumetric_fire::{VolumetricFirePass, FireUniform};

pub mod taa;
pub use taa::{TaaPass, TaaUniform};