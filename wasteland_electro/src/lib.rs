pub mod electrostatics;
pub mod magnetostatics;
pub mod materials;
pub mod properties;

pub mod prelude {
    pub use crate::electrostatics::*;
    pub use crate::magnetostatics::*;
    pub use crate::materials::*;
    pub use crate::properties::*;
}
