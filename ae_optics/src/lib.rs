pub mod brdf;
pub mod emission;
pub mod materials;
pub mod renderer;
pub mod scattering;
pub mod spectrum;

pub mod prelude {
    pub use crate::brdf::*;
    pub use crate::emission::*;
    pub use crate::materials::*;
    pub use crate::renderer::*;
    pub use crate::scattering::*;
    pub use crate::spectrum::*;
}
