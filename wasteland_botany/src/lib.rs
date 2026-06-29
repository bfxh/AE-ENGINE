//! # wasteland_botany
//!
//! 植物生物学模拟 crate，覆盖 8 大模块：
//! - 光合作用 (photosynthesis)
//! - 植物解剖 (plant_anatomy)
//! - 植物生长 (plant_growth)
//! - 次生代谢 (secondary_metabolites)
//! - 植物繁殖 (plant_reproduction)
//! - 植物生态 (plant_ecology)
//! - 根系系统 (root_system)
//! - 物候学 (phenology)
//!
//! 所有结构体派生 `Debug/Clone/Serialize/Deserialize`，
//! 数值与生物学量纲保持一致 (SI 单位除非另有说明)。

#![allow(dead_code)]

pub mod photosynthesis;
pub mod plant_anatomy;
pub mod plant_growth;
pub mod secondary_metabolites;
pub mod plant_reproduction;
pub mod plant_ecology;
pub mod root_system;
pub mod phenology;

#[allow(ambiguous_glob_reexports)]
pub mod prelude {
    pub use crate::photosynthesis::*;
    pub use crate::plant_anatomy::*;
    pub use crate::plant_growth::*;
    pub use crate::secondary_metabolites::*;
    pub use crate::plant_reproduction::*;
    pub use crate::plant_ecology::*;
    pub use crate::root_system::*;
    pub use crate::phenology::*;
}