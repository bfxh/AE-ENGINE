pub mod dispatcher;
pub mod fluid;
pub mod hardware;
pub mod job;
pub mod mpm_compute;
pub mod parallel;

pub use fluid::{StamFluidSolver3D, blackbody_rgb};
pub use mpm_compute::{MpmConfig, MpmSolver, MpmParticle, MpmGrid3D};
