//! 全局光照模块
//!
//! 设计：
//! - DDGI: Dynamic Diffuse Global Illumination
//! - SSGI: Screen Space Global Illumination
//! - RT: 硬件光追（未来）

pub mod ddgi;
pub mod ssgi;
pub mod rt;

pub use ddgi::Ddgi;
pub use ssgi::Ssgi;
pub use rt::RtGi;