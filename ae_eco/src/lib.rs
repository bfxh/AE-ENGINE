pub mod biodiversity;
pub mod competition;
pub mod foodweb;
pub mod habitat;
pub mod migration;
pub mod nutrient_cycle;
pub mod population;
pub mod succession;

pub mod prelude {
    pub use crate::biodiversity::*;
    pub use crate::competition::*;
    pub use crate::foodweb::*;
    pub use crate::habitat::*;
    pub use crate::migration::*;
    pub use crate::nutrient_cycle::*;
    pub use crate::population::*;
    pub use crate::succession::*;
}
