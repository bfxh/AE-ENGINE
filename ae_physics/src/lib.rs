pub mod broad_phase;
pub mod collision;
pub mod constraints;
pub mod destruction;
pub mod dual_phase;
pub mod fixed_point;
pub mod joints;
pub mod jolt_backend;
pub mod material;
pub mod mpm;
pub mod narrow_phase;
pub mod octree;
pub mod performance_tiers;
pub mod physics_trait;
pub mod ragdoll;
pub mod world;

#[allow(ambiguous_glob_reexports)]
pub mod prelude {
    pub use crate::broad_phase::*;
    pub use crate::collision::*;
    pub use crate::constraints::*;
    pub use crate::destruction::*;
    pub use crate::dual_phase::*;
    pub use crate::fixed_point::*;
    pub use crate::joints::*;
    pub use crate::material::*;
    pub use crate::mpm::*;
    pub use crate::narrow_phase::*;
    pub use crate::octree::*;
    pub use crate::performance_tiers::*;
    pub use crate::physics_trait::*;
    pub use crate::ragdoll::*;
    pub use crate::world::*;
}
