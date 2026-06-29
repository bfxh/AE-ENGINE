pub mod atmosphere;
pub mod climate;
pub mod precipitation;
pub mod wind;

pub mod prelude {
    pub use crate::atmosphere::*;
    pub use crate::climate::*;
    pub use crate::precipitation::*;
    pub use crate::wind::*;
}
