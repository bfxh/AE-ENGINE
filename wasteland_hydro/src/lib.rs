pub mod aquifer;
pub mod evaporation;
pub mod flooding;
pub mod infiltration;
pub mod rivers;
pub mod runoff;

pub mod prelude {
    pub use crate::aquifer::*;
    pub use crate::evaporation::*;
    pub use crate::flooding::*;
    pub use crate::infiltration::*;
    pub use crate::rivers::*;
    pub use crate::runoff::*;
}
