pub mod blend;
pub mod gait;
pub mod ik;
pub mod state_machine;

pub mod prelude {
    pub use crate::blend::*;
    pub use crate::gait::*;
    pub use crate::ik::*;
    pub use crate::state_machine::*;
}
