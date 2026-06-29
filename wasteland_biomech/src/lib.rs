// wasteland_biomech - 骨骼生物力学模拟 crate
// 包含: 相场法断裂 / Prendergast 机械调控 / Wolff 重塑 / BMP 骨生成 / 蝾螈再生 / 义肢

pub mod bmp_osteogenesis;
pub mod material_properties;
pub mod phase_field;
pub mod prendergast;
pub mod prosthetics;
pub mod regeneration;
pub mod wolff_remodeling;

#[allow(ambiguous_glob_reexports)]
pub mod prelude {
    pub use crate::bmp_osteogenesis::*;
    pub use crate::material_properties::*;
    pub use crate::phase_field::*;
    pub use crate::prendergast::*;
    pub use crate::prosthetics::*;
    pub use crate::regeneration::*;
    pub use crate::wolff_remodeling::*;
}
