// MPSS: Meta-Particle State System — SoA (Structure-of-Arrays) particle buffer
// v8.0 — APIC-MPM unified particle representation
//
// Key design:
//   - SoA layout for cache-friendly SIMD processing
//   - Single buffer shared across all subsystems (physics/chemistry/biology)
//   - APIC: affine velocity matrix C for angular momentum conservation
//   - Dead particle recycling via free list

use glam::Vec3;

/// Particle type discriminant — unifies physics/chemistry/biology categories
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParticleKind {
    // Physics base
    Inert = 0,    // passive mass (e.g., sand grain)
    Rigid = 1,    // rigid body fragment
    Soft = 2,     // deformable
    Fluid = 3,    // Lagrangian fluid parcel
    Gas = 4,      // gaseous
    Granular = 5, // granular flow
    // Chemistry
    Chemical = 10,
    // Biology
    Biological = 20,
    Cell = 21,
    Spore = 22,
    // Emergent
    Emergent = 30,
    // Custom
    Custom(u8) = 255,
}

impl ParticleKind {
    pub fn discriminant(&self) -> u8 {
        match self {
            ParticleKind::Inert => 0,
            ParticleKind::Rigid => 1,
            ParticleKind::Soft => 2,
            ParticleKind::Fluid => 3,
            ParticleKind::Gas => 4,
            ParticleKind::Granular => 5,
            ParticleKind::Chemical => 10,
            ParticleKind::Biological => 20,
            ParticleKind::Cell => 21,
            ParticleKind::Spore => 22,
            ParticleKind::Emergent => 30,
            ParticleKind::Custom(_) => 255,
        }
    }

    pub fn is_physics(&self) -> bool {
        self.discriminant() < 10
    }
    pub fn is_chemical(&self) -> bool {
        (10..20).contains(&self.discriminant())
    }
    pub fn is_biological(&self) -> bool {
        (20..30).contains(&self.discriminant())
    }
}

/// Physical phase (shared with existing system, re-exported)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpssPhase {
    Solid = 0,
    Liquid = 1,
    Gas = 2,
    Plasma = 3,
    Granular = 4,
    Crystal = 5,
    Amorphous = 6,
}

/// Material property descriptor (compact, fixed-size)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialDesc {
    pub young_modulus: f32,
    pub poisson_ratio: f32,
    pub yield_stress: f32,
    pub hardening: f32,
    pub density: f32,
    pub friction: f32,
}

impl Default for MaterialDesc {
    fn default() -> Self {
        Self {
            young_modulus: 1e6,
            poisson_ratio: 0.3,
            yield_stress: 1e4,
            hardening: 0.01,
            density: 1000.0,
            friction: 0.5,
        }
    }
}

/// Single particle in the MPSS buffer (SoA backing)
///
/// All fields with the same index across arrays form one particle.
#[derive(Debug, Clone)]
pub struct MpssBuffer {
    pub capacity: usize,
    pub count: usize,

    // --- Position / velocity (primary data) ---
    pub pos: Vec<[f32; 3]>, // x, y, z
    pub vel: Vec<[f32; 3]>, // vx, vy, vz

    // --- Deformation gradient (3x3, row-major) ---
    pub strain: Vec<[f32; 9]>, // F[0..8]

    pub jacobian: Vec<f32>, // determinant of F

    // --- APIC affine velocity matrix (3x3, row-major) ---
    pub c: Vec<[f32; 9]>, // C for APIC transfer

    // --- Force buffer (per-frame, 3D) ---
    pub force: Vec<[f32; 3]>, // accumulated force

    // --- Grid velocity (intermediate, 3D) ---
    pub grid_vel: Vec<[f32; 3]>, // grid velocity at particle

    // --- Mass / identity ---
    pub mass: Vec<f32>,
    pub kind: Vec<ParticleKind>,
    pub chemical_id: Vec<u32>,
    pub biomass: Vec<f32>,

    // --- Thermodynamic ---
    pub temperature: Vec<f32>,

    // --- Charge (for EM coupling) ---
    pub charge: Vec<f32>, // electric charge (C)

    // --- Hierarchy ---
    pub parent_id: Vec<i32>, // -1 = root

    pub lifetime: Vec<f32>,
    pub age: Vec<f32>,

    // --- Sub-cell (for multi-resolution MPM) ---
    pub subcell_strain: Vec<[f32; 3]>, // sub-cell strain trace

    // --- Material (index into material table) ---
    pub material_idx: Vec<u16>,

    // --- Phase (Solid/Liquid/Gas/Plasma/...) driven by temperature & material ---
    pub phase: Vec<MpssPhase>,

    // --- Active flag + free list ---
    pub active: Vec<bool>,
    free_list: Vec<usize>,
}

impl MpssBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            count: 0,
            pos: vec![[0.0; 3]; capacity],
            vel: vec![[0.0; 3]; capacity],
            strain: vec![[0.0; 9]; capacity],
            jacobian: vec![1.0; capacity],
            c: vec![[0.0; 9]; capacity],
            force: vec![[0.0; 3]; capacity],
            grid_vel: vec![[0.0; 3]; capacity],
            mass: vec![1.0; capacity],
            kind: vec![ParticleKind::Inert; capacity],
            chemical_id: vec![0; capacity],
            biomass: vec![0.0; capacity],
            temperature: vec![293.0; capacity],
            charge: vec![0.0; capacity],
            parent_id: vec![-1; capacity],
            lifetime: vec![f32::MAX; capacity],
            age: vec![0.0; capacity],
            subcell_strain: vec![[0.0; 3]; capacity],
            material_idx: vec![0; capacity],
            phase: vec![MpssPhase::Solid; capacity],
            active: vec![false; capacity],
            free_list: Vec::with_capacity(capacity / 4),
        }
    }

    /// Allocate a new particle slot, returns index
    pub fn spawn(&mut self) -> Option<usize> {
        if let Some(idx) = self.free_list.pop() {
            self.reset_slot(idx);
            self.active[idx] = true;
            self.count += 1;
            Some(idx)
        } else if self.count < self.capacity {
            let idx = self.count;
            self.reset_slot(idx);
            self.active[idx] = true;
            self.count += 1;
            Some(idx)
        } else {
            None
        }
    }

    /// Mark a particle as dead (slot will be recycled)
    pub fn kill(&mut self, idx: usize) {
        if idx < self.capacity && self.active[idx] {
            self.active[idx] = false;
            self.free_list.push(idx);
            self.count -= 1;
        }
    }

    /// Reset a slot to defaults
    fn reset_slot(&mut self, idx: usize) {
        self.pos[idx] = [0.0; 3];
        self.vel[idx] = [0.0; 3];
        self.strain[idx] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]; // identity
        self.jacobian[idx] = 1.0;
        self.mass[idx] = 1.0;
        self.kind[idx] = ParticleKind::Inert;
        self.chemical_id[idx] = 0;
        self.biomass[idx] = 0.0;
        self.temperature[idx] = 293.0;
        self.parent_id[idx] = -1;
        self.lifetime[idx] = f32::MAX;
        self.age[idx] = 0.0;
        self.subcell_strain[idx] = [0.0; 3];
        self.material_idx[idx] = 0;
        self.phase[idx] = MpssPhase::Solid;
    }

    /// Number of active particles
    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Sequential iteration over active particle indices
    pub fn active_indices(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.capacity).filter(|&i| self.active[i])
    }

    /// Get position as Vec3
    pub fn pos_vec3(&self, idx: usize) -> Vec3 {
        let p = self.pos[idx];
        Vec3::new(p[0], p[1], p[2])
    }

    /// Set position from Vec3
    pub fn set_pos(&mut self, idx: usize, v: Vec3) {
        self.pos[idx] = [v.x, v.y, v.z];
    }

    /// Get velocity as Vec3
    pub fn vel_vec3(&self, idx: usize) -> Vec3 {
        let v = self.vel[idx];
        Vec3::new(v[0], v[1], v[2])
    }

    /// Set velocity from Vec3
    pub fn set_vel(&mut self, idx: usize, v: Vec3) {
        self.vel[idx] = [v.x, v.y, v.z];
    }

    /// Set deformation gradient to identity
    pub fn reset_strain(&mut self, idx: usize) {
        self.strain[idx] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        self.jacobian[idx] = 1.0;
    }

    /// Get deformation gradient as 3x3 matrix (row-major)
    pub fn strain_mat3(&self, idx: usize) -> [f32; 9] {
        self.strain[idx]
    }

    /// Compact the buffer by moving active particles to the front
    pub fn compact(&mut self) {
        let mut write = 0;
        for read in 0..self.capacity {
            if self.active[read] {
                if write != read {
                    self.swap_slots(write, read);
                }
                write += 1;
            }
        }
        self.count = write;
        self.free_list.clear();
        for i in write..self.capacity {
            self.active[i] = false;
        }
    }

    fn swap_slots(&mut self, a: usize, b: usize) {
        self.pos.swap(a, b);
        self.vel.swap(a, b);
        self.strain.swap(a, b);
        self.jacobian.swap(a, b);
        self.c.swap(a, b);
        self.force.swap(a, b);
        self.grid_vel.swap(a, b);
        self.mass.swap(a, b);
        self.kind.swap(a, b);
        self.chemical_id.swap(a, b);
        self.biomass.swap(a, b);
        self.temperature.swap(a, b);
        self.charge.swap(a, b);
        self.parent_id.swap(a, b);
        self.lifetime.swap(a, b);
        self.age.swap(a, b);
        self.subcell_strain.swap(a, b);
        self.material_idx.swap(a, b);
        self.phase.swap(a, b);
        self.active.swap(a, b);
    }

    /// Apply gravity to all active particles
    pub fn apply_gravity(&mut self, g: f32, dt: f32) {
        for i in 0..self.capacity {
            if self.active[i] {
                self.vel[i][1] -= g * dt;
            }
        }
    }

    /// Forward Euler position update for all active particles
    pub fn integrate_positions(&mut self, dt: f32) {
        for i in 0..self.capacity {
            if self.active[i] {
                self.pos[i][0] += self.vel[i][0] * dt;
                self.pos[i][1] += self.vel[i][1] * dt;
                self.pos[i][2] += self.vel[i][2] * dt;
            }
        }
    }

    /// Apply reflecting boundary conditions clamping particles to world bounds.
    /// Prevents particles from drifting infinitely far.
    pub fn apply_boundary_conditions(&mut self, min: [f32; 3], max: [f32; 3]) {
        for i in 0..self.count {
            if !self.active[i] {
                continue;
            }
            for d in 0..3 {
                if self.pos[i][d] < min[d] {
                    self.pos[i][d] = min[d];
                    if self.vel[i][d] < 0.0 {
                        self.vel[i][d] = -self.vel[i][d] * 0.5;
                    }
                } else if self.pos[i][d] > max[d] {
                    self.pos[i][d] = max[d];
                    if self.vel[i][d] > 0.0 {
                        self.vel[i][d] = -self.vel[i][d] * 0.5;
                    }
                }
            }
        }
    }

    /// Clamp all particle temperatures to a safe range to prevent thermal runaway.
    pub fn clamp_temperatures(&mut self, min_temp: f32, max_temp: f32) {
        for i in 0..self.count {
            if !self.active[i] {
                continue;
            }
            if self.temperature[i] > max_temp {
                self.temperature[i] = max_temp;
            } else if self.temperature[i] < min_temp {
                self.temperature[i] = min_temp;
            }
        }
    }

    /// Apply material-specific phase transitions based on temperature.
    ///
    /// Material index → transition table:
    ///   0 (wood):     500K  Solid→Gas (pyrolysis)
    ///   1 (water):    273K  Solid→Liquid, 373K Liquid→Gas
    ///   2 (concrete): 1923K Solid→Gas (decomposition)
    ///   3 (iron):     1811K Solid→Liquid, 3134K Liquid→Gas
    ///
    /// Cascades: a particle at 400K (water) will transition Solid→Liquid→Gas in one call.
    /// Returns the number of particles that changed phase.
    pub fn apply_phase_transitions(&mut self) -> u32 {
        let mut transitions: u32 = 0;
        for i in 0..self.count {
            if !self.active[i] {
                continue;
            }
            let t = self.temperature[i];
            let mat = self.material_idx[i];
            let mut current = self.phase[i];
            // Cascade: keep transitioning while possible (max 3 hops to avoid infinite loop)
            for _ in 0..3 {
                let next = match (mat, current) {
                    // Water: ice → water → steam
                    (1, MpssPhase::Solid) if t >= 273.0 => MpssPhase::Liquid,
                    (1, MpssPhase::Liquid) if t >= 373.0 => MpssPhase::Gas,
                    (1, MpssPhase::Gas) if t < 373.0 => MpssPhase::Liquid,
                    (1, MpssPhase::Liquid) if t < 273.0 => MpssPhase::Solid,
                    // Iron: solid → liquid → gas
                    (3, MpssPhase::Solid) if t >= 1811.0 => MpssPhase::Liquid,
                    (3, MpssPhase::Liquid) if t >= 3134.0 => MpssPhase::Gas,
                    (3, MpssPhase::Gas) if t < 3134.0 => MpssPhase::Liquid,
                    (3, MpssPhase::Liquid) if t < 1811.0 => MpssPhase::Solid,
                    // Wood: pyrolysis (solid → gas at 500K)
                    (0, MpssPhase::Solid) if t >= 500.0 => MpssPhase::Gas,
                    // Concrete: decomposition (solid → gas at 1923K)
                    (2, MpssPhase::Solid) if t >= 1923.0 => MpssPhase::Gas,
                    _ => current,
                };
                if next == current {
                    break;
                }
                current = next;
            }
            if current != self.phase[i] {
                self.phase[i] = current;
                transitions += 1;
            }
        }
        transitions
    }

    /// Update ages, removing expired particles
    pub fn update_lifetimes(&mut self, dt: f32) -> Vec<usize> {
        let mut expired = Vec::new();
        for i in 0..self.count {
            if self.active[i] {
                self.age[i] += dt;
                if self.age[i] >= self.lifetime[i] {
                    expired.push(i);
                }
            }
        }
        for &i in &expired {
            self.kill(i);
        }
        expired
    }

    /// Parallel version of `apply_boundary_conditions` using rayon.
    /// Splits the particle array into chunks for parallel processing.
    pub fn apply_boundary_conditions_par(&mut self, min: [f32; 3], max: [f32; 3]) {
        use rayon::prelude::*;
        let n = self.count;
        let pos = &mut self.pos[..n];
        let vel = &mut self.vel[..n];
        let active = &self.active[..n];
        pos.par_iter_mut()
            .zip(vel.par_iter_mut())
            .zip(active.par_iter())
            .for_each(|((p, v), &a)| {
                if !a {
                    return;
                }
                for d in 0..3 {
                    if p[d] < min[d] {
                        p[d] = min[d];
                        if v[d] < 0.0 {
                            v[d] = -v[d] * 0.5;
                        }
                    } else if p[d] > max[d] {
                        p[d] = max[d];
                        if v[d] > 0.0 {
                            v[d] = -v[d] * 0.5;
                        }
                    }
                }
            });
    }

    /// Parallel version of `clamp_temperatures` using rayon.
    pub fn clamp_temperatures_par(&mut self, min_temp: f32, max_temp: f32) {
        use rayon::prelude::*;
        let n = self.count;
        let temp = &mut self.temperature[..n];
        let active = &self.active[..n];
        temp.par_iter_mut()
            .zip(active.par_iter())
            .for_each(|(t, &a)| {
                if !a {
                    return;
                }
                if *t > max_temp {
                    *t = max_temp;
                } else if *t < min_temp {
                    *t = min_temp;
                }
            });
    }

    /// Parallel version of `apply_phase_transitions` using rayon.
    /// Returns the number of particles that changed phase.
    pub fn apply_phase_transitions_par(&mut self) -> u32 {
        use rayon::prelude::*;
        use std::sync::atomic::{AtomicU32, Ordering};
        let n = self.count;
        let counter = AtomicU32::new(0);
        let temp = &self.temperature[..n];
        let mat = &self.material_idx[..n];
        let phase = &mut self.phase[..n];
        let active = &self.active[..n];
        phase
            .par_iter_mut()
            .zip(temp.par_iter())
            .zip(mat.par_iter())
            .zip(active.par_iter())
            .for_each(|(((cur, &t), &m), &a)| {
                if !a {
                    return;
                }
                let mut next = *cur;
                for _ in 0..3 {
                    let n = match (m, next) {
                        (1, MpssPhase::Solid) if t >= 273.0 => MpssPhase::Liquid,
                        (1, MpssPhase::Liquid) if t >= 373.0 => MpssPhase::Gas,
                        (1, MpssPhase::Gas) if t < 373.0 => MpssPhase::Liquid,
                        (1, MpssPhase::Liquid) if t < 273.0 => MpssPhase::Solid,
                        (3, MpssPhase::Solid) if t >= 1811.0 => MpssPhase::Liquid,
                        (3, MpssPhase::Liquid) if t >= 3134.0 => MpssPhase::Gas,
                        (3, MpssPhase::Gas) if t < 3134.0 => MpssPhase::Liquid,
                        (3, MpssPhase::Liquid) if t < 1811.0 => MpssPhase::Solid,
                        (0, MpssPhase::Solid) if t >= 500.0 => MpssPhase::Gas,
                        (2, MpssPhase::Solid) if t >= 1923.0 => MpssPhase::Gas,
                        _ => next,
                    };
                    if n == next {
                        break;
                    }
                    next = n;
                }
                if next != *cur {
                    *cur = next;
                    counter.fetch_add(1, Ordering::Relaxed);
                }
            });
        counter.into_inner()
    }

    /// Get memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        let arrays = std::mem::size_of::<[f32; 3]>() * self.pos.capacity()     // pos
            + std::mem::size_of::<[f32; 3]>() * self.vel.capacity()             // vel
            + std::mem::size_of::<[f32; 9]>() * self.strain.capacity()          // strain
            + std::mem::size_of::<f32>() * self.jacobian.capacity()             // J
            + std::mem::size_of::<f32>() * self.mass.capacity()                 // mass
            + std::mem::size_of::<ParticleKind>() * self.kind.capacity()        // kind
            + std::mem::size_of::<u32>() * self.chemical_id.capacity()          // chem id
            + std::mem::size_of::<f32>() * self.biomass.capacity()              // biomass
            + std::mem::size_of::<f32>() * self.temperature.capacity()          // temp
            + std::mem::size_of::<i32>() * self.parent_id.capacity()            // parent
            + std::mem::size_of::<f32>() * self.lifetime.capacity()             // lifetime
            + std::mem::size_of::<f32>() * self.age.capacity()                  // age
            + std::mem::size_of::<[f32; 3]>() * self.subcell_strain.capacity()  // subcell
            + std::mem::size_of::<u16>() * self.material_idx.capacity()         // mat idx
            + std::mem::size_of::<bool>() * self.active.capacity(); // active
        arrays + std::mem::size_of::<usize>() * self.free_list.capacity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_and_kill() {
        let mut buf = MpssBuffer::new(100);
        assert_eq!(buf.len(), 0);

        let i0 = buf.spawn().unwrap();
        let _i1 = buf.spawn().unwrap();
        assert_eq!(buf.len(), 2);

        buf.kill(i0);
        assert_eq!(buf.len(), 1);

        // Should recycle i0
        let i2 = buf.spawn().unwrap();
        assert_eq!(i2, i0);
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn test_integration() {
        let mut buf = MpssBuffer::new(10);
        let i = buf.spawn().unwrap();
        buf.set_pos(i, Vec3::new(0.0, 10.0, 0.0));
        buf.set_vel(i, Vec3::new(0.0, -5.0, 0.0));
        buf.integrate_positions(0.1);
        let p = buf.pos_vec3(i);
        assert!((p.y - 9.5).abs() < 0.001);
    }

    #[test]
    fn test_gravity() {
        let mut buf = MpssBuffer::new(10);
        let i = buf.spawn().unwrap();
        buf.mass[i] = 1.0;
        buf.apply_gravity(9.8, 1.0);
        let v = buf.vel_vec3(i);
        assert!((v.y + 9.8).abs() < 0.001);
    }

    #[test]
    fn test_compact() {
        let mut buf = MpssBuffer::new(5);
        let _i0 = buf.spawn().unwrap();
        let i1 = buf.spawn().unwrap();
        let _i2 = buf.spawn().unwrap();
        buf.kill(i1);
        buf.compact();
        assert_eq!(buf.len(), 2);
        assert!(buf.active[0]);
        assert!(buf.active[1]);
        assert!(!buf.active[2]);
    }

    #[test]
    fn test_strain_identity() {
        let mut buf = MpssBuffer::new(10);
        let i = buf.spawn().unwrap();
        let s = buf.strain_mat3(i);
        assert!((s[0] - 1.0).abs() < 1e-6); // identity
        assert!((s[4] - 1.0).abs() < 1e-6);
        assert!((s[8] - 1.0).abs() < 1e-6);
    }

    /// Regression test for swap_slots bug: previously compact() failed to swap
    /// 5 fields (c, force, grid_vel, charge, phase), causing data corruption
    /// when particles were compacted. This test verifies all fields are swapped.
    #[test]
    fn test_swap_slots_all_fields() {
        let mut buf = MpssBuffer::new(4);
        let i0 = buf.spawn().unwrap();
        let i1 = buf.spawn().unwrap();
        let i2 = buf.spawn().unwrap();

        // Set distinctive values on i1 (the middle particle that will move to slot 0)
        buf.c[i1] = [1.1, 2.2, 3.3, 4.4, 5.5, 6.6, 7.7, 8.8, 9.9];
        buf.force[i1] = [10.0, 20.0, 30.0];
        buf.grid_vel[i1] = [40.0, 50.0, 60.0];
        buf.charge[i1] = 777.0;
        buf.phase[i1] = MpssPhase::Plasma;
        buf.material_idx[i1] = 42;
        buf.temperature[i1] = 5000.0;

        // Kill i0, compact: i1 should move to slot 0, i2 to slot 1
        buf.kill(i0);
        buf.compact();
        assert_eq!(buf.len(), 2);

        // Verify all fields from original i1 are now at slot 0
        assert!((buf.c[0][0] - 1.1).abs() < 1e-6, "c[0] not swapped");
        assert!((buf.c[0][8] - 9.9).abs() < 1e-6, "c[8] not swapped");
        assert!((buf.force[0][0] - 10.0).abs() < 1e-6, "force[0] not swapped");
        assert!((buf.force[0][2] - 30.0).abs() < 1e-6, "force[2] not swapped");
        assert!((buf.grid_vel[0][0] - 40.0).abs() < 1e-6, "grid_vel[0] not swapped");
        assert!((buf.grid_vel[0][2] - 60.0).abs() < 1e-6, "grid_vel[2] not swapped");
        assert!((buf.charge[0] - 777.0).abs() < 1e-6, "charge not swapped");
        assert_eq!(buf.phase[0], MpssPhase::Plasma, "phase not swapped");
        assert_eq!(buf.material_idx[0], 42, "material_idx not swapped");
        assert!((buf.temperature[0] - 5000.0).abs() < 1e-6, "temperature not swapped");
    }
}
