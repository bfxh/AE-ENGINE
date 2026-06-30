pub mod acoustic_fdtd;
pub mod aizawa_attractor;
pub mod allen_cahn;
pub mod avbd;
pub mod barnes_hut;
pub mod broadphase_sap;
pub mod brusselator;
pub mod burgers;
pub mod bvh;
pub mod cahn_hilliard;
// pub mod ccd; // TODO: fix Clone trait bounds
pub mod cgle;
pub mod chaotic_dynamics;
pub mod ck_mpm;
pub mod clebsch_fluid;
pub mod cloth;
pub mod collision;
// pub mod contact_manifold; // TODO: fix unique_face_groups method
// pub mod contact_solver; // TODO: fix normalize_or_zero (glam 0.29 compat)
// pub mod constraint_solver; // TODO: fix borrow conflicts
pub mod corotational_fem;
pub mod cosserat_rod;
pub mod dispatcher;
pub mod double_pendulum;
pub mod drucker_prager;
pub mod duffing;
pub mod dynamic_aabb_tree;
pub mod eigenfluid;
pub mod electromagnetic_fdtd;
pub mod fisher_kpp;
pub mod fitzhugh_nagumo;
pub mod fluid;
pub mod fms;
pub mod fpu_beta;
pub mod gross_pitaevskii;
pub mod hardware;
pub mod heat_diffusion;
pub mod henon_heiles;
pub mod henon_map;
pub mod hodgkin_huxley;
pub mod ising_model;
pub mod job;
pub mod kdv_soliton;
pub mod keller_segel;
pub mod klein_gordon;
pub mod kuramoto;
pub mod kuramoto_sivashinsky;
pub mod lattice_boltzmann;
pub mod leapfrog_flow_maps;
pub mod logistic_map;
pub mod lorenz96;
pub mod lotka_volterra;
// pub mod mass_splitting_solver; // TODO: depends on contact_manifold
pub mod molecular_dynamics;
pub mod mpm_compute;
pub mod navier_stokes_2d;
pub mod nls_solver;
pub mod noise;
pub mod ogc;
pub mod parallel;
// pub mod pbf; // TODO: fix ambiguous numeric type
pub mod phase_change;
pub mod pic_plasma;
pub mod plastic_fem;
pub mod projective_dynamics;
pub mod reaction_diffusion;
// pub mod resting_rigid_bodies; // TODO: fix ambiguous numeric type and mismatched types
pub mod rigid_body;
pub mod sandpile;
pub mod schrodinger;
pub mod sdf;
pub mod shallow_water;
pub mod shape_matching;
pub mod sine_gordon;
pub mod sir_model;
pub mod stable_neo_hookean;
pub mod standard_map;
pub mod strain_based_dynamics;
pub mod swift_hohenberg;
pub mod thomas_attractor;
pub mod three_body;
pub mod van_der_pol;
pub mod vbd_solver;
pub mod vicsek;
pub mod wavelet_turbulence;
// pub mod wcsph; // TODO: depends on pbf

pub use avbd::{
    AvbdConfig, AvbdParticle, AvbdRigidBody, AvbdSolver, ContactConstraint, DistanceConstraint,
};
pub use fluid::{StamFluidSolver3D, blackbody_rgb};
pub use leapfrog_flow_maps::{LfmConfig, LfmSolver3D, mgpcg_solve_poisson};
pub use mpm_compute::{MpmConfig, MpmGrid3D, MpmParticle, MpmSolver};
