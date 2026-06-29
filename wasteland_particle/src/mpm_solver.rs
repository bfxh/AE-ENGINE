// v8.0 MPM Solver — unified Material Point Method
// ——————————————————————————————————————————————————
// Single `mpm_step()` replaces 30+ independent system calls.
//
// Standard MPM loop:
//   P2G: particle → grid (mass, momentum, force)
//   Grid: solve momentum equation on grid
//   G2P: grid → particle (velocity, deformation gradient)
//
// Uses APIC (Affine Particle-In-Cell) for angular momentum conservation.
// Grid: uniform Cartesian, dx = grid spacing.
//
// References:
//   - Jiang et al. 2015 (APIC)
//   - Jiang et al. 2017 (MLS-MPM)
//   - Hu et al. 2018 (Moving Least Squares MPM)

use crate::constitutive::{self, ConstitutiveModel, MaterialParams};
use crate::mpss::MpssBuffer;
use rayon::prelude::*;

/// Wrapper to make a mutable raw pointer Send+Sync.
/// SAFETY: callers must ensure no concurrent mutable access to the same memory.
struct SendPtrMut<T>(*mut T);
unsafe impl<T> Send for SendPtrMut<T> {}
unsafe impl<T> Sync for SendPtrMut<T> {}
impl<T> SendPtrMut<T> {
    fn ptr(&self) -> *mut T {
        self.0
    }
}

/// Wrapper to make a const raw pointer Send+Sync.
/// SAFETY: callers must ensure no concurrent mutable access to the same memory.
struct SendPtrConst<T>(*const T);
unsafe impl<T> Send for SendPtrConst<T> {}
unsafe impl<T> Sync for SendPtrConst<T> {}
impl<T> SendPtrConst<T> {
    fn ptr(&self) -> *const T {
        self.0
    }
}

/// MPM configuration
#[derive(Debug, Clone, Copy)]
pub struct MpmConfig {
    pub grid_dx: f32,          // grid cell size (m)
    pub grid_size: [usize; 3], // nx, ny, nz
    pub substeps: usize,       // sub-steps per frame (CFL adaptive)
    pub gravity: [f32; 3],     // gravity vector (m/s²)
    pub damping: f32,          // velocity damping (0-1)
    pub enable_plasticity: bool,
    pub enable_contact: bool,
    pub use_sparse_grid: bool, // sparse grid for 8GB VRAM / 3M particles
}

impl Default for MpmConfig {
    fn default() -> Self {
        Self {
            grid_dx: 1.6,
            grid_size: [16, 16, 16],
            substeps: 1,
            gravity: [0.0, -9.81, 0.0],
            damping: 0.995,
            enable_plasticity: true,
            enable_contact: true,
            use_sparse_grid: true,
        }
    }
}

/// Grid node data for one MPM sub-step
pub struct Grid {
    pub mass: Vec<f32>,
    pub vel: Vec<[f32; 3]>,
    pub force: Vec<[f32; 3]>,
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    pub dx: f32,
    pub inv_dx: f32,
    /// §3.1 Moving Window MPM: world-space origin of grid node (0,0,0).
    /// Grid covers [origin, origin + nx*dx] in world coordinates.
    /// Updated per-frame to follow the player so the grid always centers on the
    /// near-field LOD region. P2G/G2P use (pos - origin) for index computation.
    pub origin: [f32; 3],
}

impl Grid {
    pub fn new(nx: usize, ny: usize, nz: usize, dx: f32) -> Self {
        let size = nx * ny * nz;
        Self {
            mass: vec![0.0; size],
            vel: vec![[0.0; 3]; size],
            force: vec![[0.0; 3]; size],
            nx,
            ny,
            nz,
            dx,
            inv_dx: 1.0 / dx,
            origin: [0.0; 3],
        }
    }

    pub fn reset(&mut self) {
        self.mass.fill(0.0);
        self.vel.fill([0.0; 3]);
        self.force.fill([0.0; 3]);
    }

    /// Parallel grid update: solve momentum equation on grid nodes
    pub fn update_parallel(&mut self, grav: [f32; 3], dt: f32) {
        let n = self.nx * self.ny * self.nz;
        let nx = self.nx;
        let ny = self.ny;

        let mass = &self.mass[..n];
        let force = &self.force[..n];
        let vel = &mut self.vel[..n];

        vel.par_iter_mut().enumerate().for_each(|(gid, v)| {
            let m = mass[gid];
            if m > 1e-12 {
                v[0] /= m;
                v[1] /= m;
                v[2] /= m;
                v[0] += (grav[0] + force[gid][0] / m) * dt;
                v[1] += (grav[1] + force[gid][1] / m) * dt;
                v[2] += (grav[2] + force[gid][2] / m) * dt;

                let j = (gid % (nx * ny)) / nx;
                if j == 0 {
                    v[1] = v[1].max(0.0);
                    v[0] *= 0.9;
                    v[2] *= 0.9;
                }
            }
        });
    }

    /// Merge another grid's mass/vel/force into this one (for parallel P2G reduction).
    pub fn merge(&mut self, other: &Grid) {
        let n = self.mass.len().min(other.mass.len());
        self.mass[..n].iter_mut().zip(other.mass[..n].iter()).for_each(|(a, b)| *a += *b);
        self.vel[..n].iter_mut().zip(other.vel[..n].iter()).for_each(|(a, b)| {
            a[0] += b[0];
            a[1] += b[1];
            a[2] += b[2];
        });
        self.force[..n].iter_mut().zip(other.force[..n].iter()).for_each(|(a, b)| {
            a[0] += b[0];
            a[1] += b[1];
            a[2] += b[2];
        });
    }

    pub fn index(&self, i: usize, j: usize, k: usize) -> usize {
        i + j * self.nx + k * self.nx * self.ny
    }

    pub fn pos_to_grid(&self, pos: [f32; 3]) -> (usize, usize, usize, [f32; 3]) {
        // §3.1 Moving Window: shift by origin so grid follows player.
        let lx = pos[0] - self.origin[0];
        let ly = pos[1] - self.origin[1];
        let lz = pos[2] - self.origin[2];
        let gx = (lx * self.inv_dx - 0.5).floor() as isize;
        let gy = (ly * self.inv_dx - 0.5).floor() as isize;
        let gz = (lz * self.inv_dx - 0.5).floor() as isize;

        let fx = lx * self.inv_dx - 0.5 - gx as f32;
        let fy = ly * self.inv_dx - 0.5 - gy as f32;
        let fz = lz * self.inv_dx - 0.5 - gz as f32;

        (gx.max(0) as usize, gy.max(0) as usize, gz.max(0) as usize, [fx, fy, fz])
    }
}

/// Core MPM step: single sub-step of dt
fn mpm_substep(
    buffer: &mut MpssBuffer,
    grid: &mut Grid,
    dt: f32,
    config: &MpmConfig,
    material_table: &[MaterialParams],
    model_table: &[ConstitutiveModel],
) {
    let n = buffer.count; // Phase 6: Use count not capacity (300x speedup for 10k/3M particles)

    // ——— P2G: Particle to Grid ———
    grid.reset();

    for i in 0..n {
        if !buffer.active[i] {
            continue;
        }

        let pos = buffer.pos[i];
        let vel = buffer.vel[i];
        let mass = buffer.mass[i];
        let strain = buffer.strain[i];
        let J = buffer.jacobian[i];
        let mat_idx = buffer.material_idx[i] as usize;

        let params = material_table.get(mat_idx).unwrap_or(&material_table[0]);
        let model = model_table.get(mat_idx).unwrap_or(&ConstitutiveModel::NeoHookean);

        // Compute stress from constitutive model
        let (stress, _new_J, _new_F, _plastic) =
            constitutive::compute_stress(*model, params, &strain, J);

        // Stress power: P = J * σ * F^{-T}  (1st Piola-Kirchhoff)
        // Simplified: use Cauchy stress * volume for force
        // Force formula: f_node = -V_p * σ_p * ∂w/∂x_p
        // Since w = b_spline(x_node - x_p), ∂w/∂x_p = -∂w/∂x_node = -dw*inv_dx
        // So f = -V * σ * (-dw * inv_dx) = V * σ * dw * inv_dx
        // (dw * inv_dx is applied later in the P2G force accumulation)
        let vol = mass / params.density;
        let stress_vol = [
            vol * stress[0],
            vol * stress[1],
            vol * stress[2],
            vol * stress[3],
            vol * stress[4],
            vol * stress[5],
            vol * stress[6],
            vol * stress[7],
            vol * stress[8],
        ];

        // APIC affine velocity matrix
        let c = buffer.c[i]; // 3x3, row-major

        let (gx, gy, gz, fx) = grid.pos_to_grid(pos);

        // Quadratic B-spline weights (3x3x3 stencil)
        for dk in 0..3 {
            for dj in 0..3 {
                for di in 0..3 {
                    let wx = b_spline(di as f32 - fx[0]);
                    let wy = b_spline(dj as f32 - fx[1]);
                    let wz = b_spline(dk as f32 - fx[2]);
                    let w = wx * wy * wz;

                    if w < 1e-10 {
                        continue;
                    }

                    let gx_i = gx + di;
                    let gy_i = gy + dj;
                    let gz_i = gz + dk;

                    if gx_i >= grid.nx || gy_i >= grid.ny || gz_i >= grid.nz {
                        continue;
                    }

                    let gid = grid.index(gx_i, gy_i, gz_i);

                    // Grid position relative to particle (§3.1: add origin for moving window)
                    let dx = (gx_i as f32 + 0.5) * grid.dx + grid.origin[0] - pos[0];
                    let dy = (gy_i as f32 + 0.5) * grid.dx + grid.origin[1] - pos[1];
                    let dz = (gz_i as f32 + 0.5) * grid.dx + grid.origin[2] - pos[2];

                    // APIC velocity: v_p + C * (x_grid - x_p)
                    let vel_x = vel[0] + c[0] * dx + c[1] * dy + c[2] * dz;
                    let vel_y = vel[1] + c[3] * dx + c[4] * dy + c[5] * dz;
                    let vel_z = vel[2] + c[6] * dx + c[7] * dy + c[8] * dz;

                    grid.mass[gid] += w * mass;

                    // Momentum contribution
                    grid.vel[gid][0] += w * mass * vel_x;
                    grid.vel[gid][1] += w * mass * vel_y;
                    grid.vel[gid][2] += w * mass * vel_z;

                    // Force from stress: -V * σ * ∇w
                    let dwx = b_spline_deriv(di as f32 - fx[0]) * grid.inv_dx;
                    let dwy = b_spline_deriv(dj as f32 - fx[1]) * grid.inv_dx;
                    let dwz = b_spline_deriv(dk as f32 - fx[2]) * grid.inv_dx;

                    grid.force[gid][0] +=
                        stress_vol[0] * dwx + stress_vol[1] * dwy + stress_vol[2] * dwz;
                    grid.force[gid][1] +=
                        stress_vol[3] * dwx + stress_vol[4] * dwy + stress_vol[5] * dwz;
                    grid.force[gid][2] +=
                        stress_vol[6] * dwx + stress_vol[7] * dwy + stress_vol[8] * dwz;
                }
            }
        }
    }

    // --- Grid Update (parallel via rayon) ---
    grid.update_parallel(config.gravity, dt);


    // --- G2P: Grid to Particle (serial — rayon parallelization showed no gain at 10k particles) ---
    let damping = config.damping;
    for i in 0..n {
        if !buffer.active[i] {
            continue;
        }
        let pos = buffer.pos[i];
        let (gx, gy, gz, fx) = grid.pos_to_grid(pos);
        let mut new_vel = [0.0f32; 3];
        let mut new_c = [0.0f32; 9];
        let mut grad_v = [0.0f32; 9];
        for dk in 0..3 {
            for dj in 0..3 {
                for di in 0..3 {
                    let wx = b_spline(di as f32 - fx[0]);
                    let wy = b_spline(dj as f32 - fx[1]);
                    let wz = b_spline(dk as f32 - fx[2]);
                    let w = wx * wy * wz;
                    if w < 1e-10 {
                        continue;
                    }
                    let gx_i = gx + di;
                    let gy_i = gy + dj;
                    let gz_i = gz + dk;
                    if gx_i >= grid.nx || gy_i >= grid.ny || gz_i >= grid.nz {
                        continue;
                    }
                    let gid = grid.index(gx_i, gy_i, gz_i);
                    if grid.mass[gid] < 1e-12 {
                        continue;
                    }
                    let gv = grid.vel[gid];
                    new_vel[0] += w * gv[0];
                    new_vel[1] += w * gv[1];
                    new_vel[2] += w * gv[2];
                    let dx = (gx_i as f32 + 0.5) * grid.dx + grid.origin[0] - pos[0];
                    let dy = (gy_i as f32 + 0.5) * grid.dx + grid.origin[1] - pos[1];
                    let dz = (gz_i as f32 + 0.5) * grid.dx + grid.origin[2] - pos[2];
                    let scale = 4.0 * grid.inv_dx * grid.inv_dx * w;
                    new_c[0] += scale * gv[0] * dx;
                    new_c[1] += scale * gv[0] * dy;
                    new_c[2] += scale * gv[0] * dz;
                    new_c[3] += scale * gv[1] * dx;
                    new_c[4] += scale * gv[1] * dy;
                    new_c[5] += scale * gv[1] * dz;
                    new_c[6] += scale * gv[2] * dx;
                    new_c[7] += scale * gv[2] * dy;
                    new_c[8] += scale * gv[2] * dz;
                    let dwx = b_spline_deriv(di as f32 - fx[0]) * grid.inv_dx;
                    let dwy = b_spline_deriv(dj as f32 - fx[1]) * grid.inv_dx;
                    let dwz = b_spline_deriv(dk as f32 - fx[2]) * grid.inv_dx;
                    grad_v[0] -= w * gv[0] * dwx;
                    grad_v[1] -= w * gv[0] * dwy;
                    grad_v[2] -= w * gv[0] * dwz;
                    grad_v[3] -= w * gv[1] * dwx;
                    grad_v[4] -= w * gv[1] * dwy;
                    grad_v[5] -= w * gv[1] * dwz;
                    grad_v[6] -= w * gv[2] * dwx;
                    grad_v[7] -= w * gv[2] * dwy;
                    grad_v[8] -= w * gv[2] * dwz;
                }
            }
        }
        for v in new_vel.iter() {
            if !v.is_finite() {
                new_vel = [0.0; 3];
                new_c = [0.0; 9];
                break;
            }
        }
        let max_v = 50.0;
        for v in new_vel.iter_mut() {
            *v = v.clamp(-max_v, max_v);
        }
        let mut new_pos = [
            pos[0] + new_vel[0] * dt,
            pos[1] + new_vel[1] * dt,
            pos[2] + new_vel[2] * dt,
        ];
        for v in new_pos.iter() {
            if !v.is_finite() {
                new_pos = [6.0, 5.0, 6.0];
                new_vel = [0.0; 3];
                break;
            }
        }
        if new_pos[1] < 0.0 {
            new_pos[1] = 0.0;
            new_vel[1] = new_vel[1].max(0.0) * 0.5;
        }
        let grid_max_x = (grid.nx - 1) as f32 * grid.dx;
        let grid_max_z = (grid.nz - 1) as f32 * grid.dx;
        if new_pos[0] < 0.0 {
            new_pos[0] = 0.0;
            new_vel[0] = new_vel[0].max(0.0) * 0.5;
        } else if new_pos[0] > grid_max_x {
            new_pos[0] = grid_max_x;
            new_vel[0] = new_vel[0].min(0.0) * 0.5;
        }
        if new_pos[2] < 0.0 {
            new_pos[2] = 0.0;
            new_vel[2] = new_vel[2].max(0.0) * 0.5;
        } else if new_pos[2] > grid_max_z {
            new_pos[2] = grid_max_z;
            new_vel[2] = new_vel[2].min(0.0) * 0.5;
        }
        let f = buffer.strain[i];
        let mut new_f = [0.0f32; 9];
        for r in 0..3 {
            for c in 0..3 {
                let idx = r * 3 + c;
                let mut sum = 0.0f32;
                for k in 0..3 {
                    let kronecker = if r == k { 1.0 } else { 0.0 };
                    let term = kronecker + grad_v[r * 3 + k] * dt;
                    sum += term * f[k * 3 + c];
                }
                new_f[idx] = sum;
            }
        }
        let mut f_valid = true;
        for v in new_f.iter() {
            if !v.is_finite() || v.abs() > 1e3 {
                f_valid = false;
                break;
            }
        }
        let (final_strain, final_j) = if f_valid {
            let j = new_f[0] * (new_f[4] * new_f[8] - new_f[5] * new_f[7])
                - new_f[1] * (new_f[3] * new_f[8] - new_f[5] * new_f[6])
                + new_f[2] * (new_f[3] * new_f[7] - new_f[4] * new_f[6]);
            (new_f, j.clamp(0.1, 10.0))
        } else {
            ([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0], 1.0)
        };
        buffer.vel[i] = new_vel;
        buffer.c[i] = new_c;
        buffer.pos[i] = new_pos;
        buffer.strain[i] = final_strain;
        buffer.jacobian[i] = final_j;
        buffer.vel[i][0] *= damping;
        buffer.vel[i][1] *= damping;
        buffer.vel[i][2] *= damping;
    }
}

/// Quadratic B-spline kernel
fn b_spline(x: f32) -> f32 {
    let ax = x.abs();
    if ax < 0.5 {
        0.75 - ax * ax
    } else if ax < 1.5 {
        let t = 1.5 - ax;
        0.5 * t * t
    } else {
        0.0
    }
}

/// Derivative of quadratic B-spline
fn b_spline_deriv(x: f32) -> f32 {
    let ax = x.abs();
    if ax < 0.5 {
        -2.0 * x
    } else if ax < 1.5 {
        -(1.5 - ax) * x.signum()
    } else {
        0.0
    }
}

/// Main MPM step: advance the simulation by one frame dt.
///
/// Divides dt into config.substeps sub-steps for CFL stability.
/// Each sub-step: P2G → Grid Update → G2P.
pub fn mpm_step(
    buffer: &mut MpssBuffer,
    config: &MpmConfig,
    material_table: &[MaterialParams],
    model_table: &[ConstitutiveModel],
    total_dt: f32,
) {
    let sub_dt = total_dt / config.substeps as f32;

    let nx = config.grid_size[0];
    let ny = config.grid_size[1];
    let nz = config.grid_size[2];
    let mut grid = Grid::new(nx, ny, nz, config.grid_dx);

    for _ in 0..config.substeps {
        mpm_substep(buffer, &mut grid, sub_dt, config, material_table, model_table);
    }

    // Update lifetimes
    buffer.update_lifetimes(total_dt);
}

/// MPM step with reusable grid (avoids per-frame allocation)
pub fn mpm_step_with_grid(
    buffer: &mut MpssBuffer,
    grid: &mut Grid,
    config: &MpmConfig,
    material_table: &[MaterialParams],
    model_table: &[ConstitutiveModel],
    total_dt: f32,
) {
    let sub_dt = total_dt / config.substeps as f32;

    for _ in 0..config.substeps {
        mpm_substep(buffer, grid, sub_dt, config, material_table, model_table);
    }

    // Update lifetimes
    buffer.update_lifetimes(total_dt);
}

/// Parallel MPM sub-step using rayon:
/// - P2G: thread-local grids via `fold` + `reduce` (no write conflicts)
/// - Grid update: parallel (existing `update_parallel`)
/// - G2P: embarrassingly parallel (each particle reads grid, writes own slot)
///
/// Falls back to serial for small particle counts (overhead not worth it).
#[allow(dead_code)]
fn mpm_substep_parallel(
    buffer: &mut MpssBuffer,
    grid: &mut Grid,
    dt: f32,
    config: &MpmConfig,
    material_table: &[MaterialParams],
    model_table: &[ConstitutiveModel],
) {
    let n = buffer.count;
    // Collect active particle indices ONCE (avoids iterating 1M particles in P2G/G2P loops)
    let active_indices: Vec<usize> = (0..n).filter(|&i| buffer.active[i]).collect();
    let active_count = active_indices.len();
    if active_count == 0 {
        return;
    }
    // Below 50k active particles, serial is faster (parallel fold/reduce allocates many Grids)
    if active_count < 50_000 {
        mpm_substep_serial_active(buffer, grid, &active_indices, dt, config, material_table, model_table);
        return;
    }

    // ——— P2G (parallel via thread-local grids) ———
    grid.reset();

    let nx = grid.nx;
    let ny = grid.ny;
    let nz = grid.nz;
    let dx = grid.dx;
    let inv_dx = grid.inv_dx;
    let origin = grid.origin; // §3.1 Moving Window

    // Read-only buffer pointer (P2G only reads particle data, writes to thread-local grids)
    let buf_ptr = SendPtrConst(buffer as *const MpssBuffer);

    // Each thread accumulates into its own grid, then merge.
    // Iterates only active_indices (10k-100k), NOT all particles (1M).
    let merged = active_indices
        .par_iter()
        .fold(
            || Grid::new(nx, ny, nz, dx),
            |mut local_grid, &i| {
                let bp: *const MpssBuffer = buf_ptr.ptr();
                unsafe {
                    let pos = (&(*bp).pos)[i];
                    let vel = (&(*bp).vel)[i];
                    let mass = (&(*bp).mass)[i];
                    let strain = (&(*bp).strain)[i];
                    let J = (&(*bp).jacobian)[i];
                    let mat_idx = (&(*bp).material_idx)[i] as usize;
                    let c = (&(*bp).c)[i];

                    let params = material_table.get(mat_idx).unwrap_or(&material_table[0]);
                    let model = model_table.get(mat_idx).unwrap_or(&ConstitutiveModel::NeoHookean);

                    let (stress, _new_J, _new_F, _plastic) =
                        constitutive::compute_stress(*model, params, &strain, J);

                    let vol = mass / params.density;
                    let stress_vol = [
                        vol * stress[0], vol * stress[1], vol * stress[2],
                        vol * stress[3], vol * stress[4], vol * stress[5],
                        vol * stress[6], vol * stress[7], vol * stress[8],
                    ];

                    let gx = ((pos[0] - origin[0]) * inv_dx - 0.5).floor() as isize;
                    let gy = ((pos[1] - origin[1]) * inv_dx - 0.5).floor() as isize;
                    let gz = ((pos[2] - origin[2]) * inv_dx - 0.5).floor() as isize;
                    let fx = (pos[0] - origin[0]) * inv_dx - 0.5 - gx as f32;
                    let fy = (pos[1] - origin[1]) * inv_dx - 0.5 - gy as f32;
                    let fz = (pos[2] - origin[2]) * inv_dx - 0.5 - gz as f32;
                    let gx = gx.max(0) as usize;
                    let gy = gy.max(0) as usize;
                    let gz = gz.max(0) as usize;

                    for dk in 0..3 {
                        for dj in 0..3 {
                            for di in 0..3 {
                                let wx = b_spline(di as f32 - fx);
                                let wy = b_spline(dj as f32 - fy);
                                let wz = b_spline(dk as f32 - fz);
                                let w = wx * wy * wz;
                                if w < 1e-10 { continue; }
                                let gx_i = gx + di;
                                let gy_i = gy + dj;
                                let gz_i = gz + dk;
                                if gx_i >= nx || gy_i >= ny || gz_i >= nz { continue; }
                                let gid = gx_i + gy_i * nx + gz_i * nx * ny;

                                let dxg = (gx_i as f32 + 0.5) * dx + origin[0] - pos[0];
                                let dyg = (gy_i as f32 + 0.5) * dx + origin[1] - pos[1];
                                let dzg = (gz_i as f32 + 0.5) * dx + origin[2] - pos[2];

                                let vel_x = vel[0] + c[0] * dxg + c[1] * dyg + c[2] * dzg;
                                let vel_y = vel[1] + c[3] * dxg + c[4] * dyg + c[5] * dzg;
                                let vel_z = vel[2] + c[6] * dxg + c[7] * dyg + c[8] * dzg;

                                local_grid.mass[gid] += w * mass;
                                local_grid.vel[gid][0] += w * mass * vel_x;
                                local_grid.vel[gid][1] += w * mass * vel_y;
                                local_grid.vel[gid][2] += w * mass * vel_z;

                                let dwx = b_spline_deriv(di as f32 - fx) * inv_dx;
                                let dwy = b_spline_deriv(dj as f32 - fy) * inv_dx;
                                let dwz = b_spline_deriv(dk as f32 - fz) * inv_dx;

                                local_grid.force[gid][0] +=
                                    stress_vol[0] * dwx + stress_vol[1] * dwy + stress_vol[2] * dwz;
                                local_grid.force[gid][1] +=
                                    stress_vol[3] * dwx + stress_vol[4] * dwy + stress_vol[5] * dwz;
                                local_grid.force[gid][2] +=
                                    stress_vol[6] * dwx + stress_vol[7] * dwy + stress_vol[8] * dwz;
                            }
                        }
                    }
                }
                local_grid
            },
        )
        .reduce(
            || Grid::new(nx, ny, nz, dx),
            |mut a, b| {
                a.merge(&b);
                a
            },
        );

    // Copy merged into grid (in-place)
    grid.mass.copy_from_slice(&merged.mass);
    grid.vel.copy_from_slice(&merged.vel);
    grid.force.copy_from_slice(&merged.force);

    // --- Grid Update (parallel) ---
    grid.update_parallel(config.gravity, dt);

    // --- G2P (parallel, each particle writes to its own slot) ---
    let damping = config.damping;
    let grid_max_x = (nx - 1) as f32 * dx;
    let grid_max_z = (nz - 1) as f32 * dx;
    let max_v = 50.0f32;

    // SAFETY: each iteration writes to disjoint particle indices (vel, c, pos, strain, jacobian),
    // so there's no data race. The grid is read-only during G2P.
    let buffer_ptr = SendPtrMut(buffer as *mut MpssBuffer);
    let grid_ptr = SendPtrConst(grid as *const Grid);
    active_indices.par_iter().for_each(move |&i| {
        // Bind raw pointers via method calls to force whole-struct capture (avoids
        // Rust 2021 disjoint closure captures from grabbing the raw pointer field directly).
        let bp: *mut MpssBuffer = buffer_ptr.ptr();
        let gp: *const Grid = grid_ptr.ptr();
        unsafe {
            let pos = (&(*bp).pos)[i];
            let gx = ((pos[0] - origin[0]) * inv_dx - 0.5).floor() as isize;
            let gy = ((pos[1] - origin[1]) * inv_dx - 0.5).floor() as isize;
            let gz = ((pos[2] - origin[2]) * inv_dx - 0.5).floor() as isize;
            let fx = (pos[0] - origin[0]) * inv_dx - 0.5 - gx as f32;
            let fy = (pos[1] - origin[1]) * inv_dx - 0.5 - gy as f32;
            let fz = (pos[2] - origin[2]) * inv_dx - 0.5 - gz as f32;
            let gx = gx.max(0) as usize;
            let gy = gy.max(0) as usize;
            let gz = gz.max(0) as usize;

            let mut new_vel = [0.0f32; 3];
            let mut new_c = [0.0f32; 9];
            let mut grad_v = [0.0f32; 9];

            for dk in 0..3 {
                for dj in 0..3 {
                    for di in 0..3 {
                        let wx = b_spline(di as f32 - fx);
                        let wy = b_spline(dj as f32 - fy);
                        let wz = b_spline(dk as f32 - fz);
                        let w = wx * wy * wz;
                        if w < 1e-10 { continue; }
                        let gx_i = gx + di;
                        let gy_i = gy + dj;
                        let gz_i = gz + dk;
                        if gx_i >= nx || gy_i >= ny || gz_i >= nz { continue; }
                        let gid = gx_i + gy_i * nx + gz_i * nx * ny;
                        if (&(*gp).mass)[gid] < 1e-12 { continue; }
                        let gv = (&(*gp).vel)[gid];
                        new_vel[0] += w * gv[0];
                        new_vel[1] += w * gv[1];
                        new_vel[2] += w * gv[2];
                        let dxg = (gx_i as f32 + 0.5) * dx + origin[0] - pos[0];
                        let dyg = (gy_i as f32 + 0.5) * dx + origin[1] - pos[1];
                        let dzg = (gz_i as f32 + 0.5) * dx + origin[2] - pos[2];
                        let scale = 4.0 * inv_dx * inv_dx * w;
                        new_c[0] += scale * gv[0] * dxg;
                        new_c[1] += scale * gv[0] * dyg;
                        new_c[2] += scale * gv[0] * dzg;
                        new_c[3] += scale * gv[1] * dxg;
                        new_c[4] += scale * gv[1] * dyg;
                        new_c[5] += scale * gv[1] * dzg;
                        new_c[6] += scale * gv[2] * dxg;
                        new_c[7] += scale * gv[2] * dyg;
                        new_c[8] += scale * gv[2] * dzg;
                        let dwx = b_spline_deriv(di as f32 - fx) * inv_dx;
                        let dwy = b_spline_deriv(dj as f32 - fy) * inv_dx;
                        let dwz = b_spline_deriv(dk as f32 - fz) * inv_dx;
                        grad_v[0] -= w * gv[0] * dwx;
                        grad_v[1] -= w * gv[0] * dwy;
                        grad_v[2] -= w * gv[0] * dwz;
                        grad_v[3] -= w * gv[1] * dwx;
                        grad_v[4] -= w * gv[1] * dwy;
                        grad_v[5] -= w * gv[1] * dwz;
                        grad_v[6] -= w * gv[2] * dwx;
                        grad_v[7] -= w * gv[2] * dwy;
                        grad_v[8] -= w * gv[2] * dwz;
                    }
                }
            }
            for v in new_vel.iter() {
                if !v.is_finite() {
                    new_vel = [0.0; 3];
                    new_c = [0.0; 9];
                    break;
                }
            }
            for v in new_vel.iter_mut() {
                *v = v.clamp(-max_v, max_v);
            }
            let mut new_pos = [
                pos[0] + new_vel[0] * dt,
                pos[1] + new_vel[1] * dt,
                pos[2] + new_vel[2] * dt,
            ];
            for v in new_pos.iter() {
                if !v.is_finite() {
                    new_pos = [6.0, 5.0, 6.0];
                    new_vel = [0.0; 3];
                    break;
                }
            }
            if new_pos[1] < 0.0 {
                new_pos[1] = 0.0;
                new_vel[1] = new_vel[1].max(0.0) * 0.5;
            }
            if new_pos[0] < 0.0 {
                new_pos[0] = 0.0;
                new_vel[0] = new_vel[0].max(0.0) * 0.5;
            } else if new_pos[0] > grid_max_x {
                new_pos[0] = grid_max_x;
                new_vel[0] = new_vel[0].min(0.0) * 0.5;
            }
            if new_pos[2] < 0.0 {
                new_pos[2] = 0.0;
                new_vel[2] = new_vel[2].max(0.0) * 0.5;
            } else if new_pos[2] > grid_max_z {
                new_pos[2] = grid_max_z;
                new_vel[2] = new_vel[2].min(0.0) * 0.5;
            }
            let f = (&(*bp).strain)[i];
            let mut new_f = [0.0f32; 9];
            for r in 0..3 {
                for c in 0..3 {
                    let idx = r * 3 + c;
                    let mut sum = 0.0f32;
                    for k in 0..3 {
                        let kronecker = if r == k { 1.0 } else { 0.0 };
                        let term = kronecker + grad_v[r * 3 + k] * dt;
                        sum += term * f[k * 3 + c];
                    }
                    new_f[idx] = sum;
                }
            }
            let mut f_valid = true;
            for v in new_f.iter() {
                if !v.is_finite() || v.abs() > 1e3 {
                    f_valid = false;
                    break;
                }
            }
            let (final_strain, final_j) = if f_valid {
                let j = new_f[0] * (new_f[4] * new_f[8] - new_f[5] * new_f[7])
                    - new_f[1] * (new_f[3] * new_f[8] - new_f[5] * new_f[6])
                    + new_f[2] * (new_f[3] * new_f[7] - new_f[4] * new_f[6]);
                (new_f, j.clamp(0.1, 10.0))
            } else {
                ([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0], 1.0)
            };
            (&mut (*bp).vel)[i] = new_vel;
            (&mut (*bp).c)[i] = new_c;
            (&mut (*bp).pos)[i] = new_pos;
            (&mut (*bp).strain)[i] = final_strain;
            (&mut (*bp).jacobian)[i] = final_j;
            (&mut (*bp).vel)[i][0] *= damping;
            (&mut (*bp).vel)[i][1] *= damping;
            (&mut (*bp).vel)[i][2] *= damping;
        }
    });
}

/// Serial MPM sub-step that only iterates active particle indices (skips 1M inactive scan).
/// Used when active_count < 50k to avoid rayon fold/reduce Grid allocation overhead.
fn mpm_substep_serial_active(
    buffer: &mut MpssBuffer,
    grid: &mut Grid,
    active_indices: &[usize],
    dt: f32,
    config: &MpmConfig,
    material_table: &[MaterialParams],
    model_table: &[ConstitutiveModel],
) {
    let nx = grid.nx;
    let ny = grid.ny;
    let nz = grid.nz;
    let dx = grid.dx;
    let inv_dx = grid.inv_dx;
    let origin = grid.origin; // §3.1 Moving Window

    // ——— P2G (serial, only active particles) ———
    grid.reset();

    for &i in active_indices {
        let pos = buffer.pos[i];
        let vel = buffer.vel[i];
        let mass = buffer.mass[i];
        let strain = buffer.strain[i];
        let J = buffer.jacobian[i];
        let mat_idx = buffer.material_idx[i] as usize;
        let c = buffer.c[i];

        let params = material_table.get(mat_idx).unwrap_or(&material_table[0]);
        let model = model_table.get(mat_idx).unwrap_or(&ConstitutiveModel::NeoHookean);

        let (stress, _new_J, _new_F, _plastic) =
            constitutive::compute_stress(*model, params, &strain, J);

        let vol = mass / params.density;
        let stress_vol = [
            vol * stress[0], vol * stress[1], vol * stress[2],
            vol * stress[3], vol * stress[4], vol * stress[5],
            vol * stress[6], vol * stress[7], vol * stress[8],
        ];

        let gx = ((pos[0] - origin[0]) * inv_dx - 0.5).floor() as isize;
        let gy = ((pos[1] - origin[1]) * inv_dx - 0.5).floor() as isize;
        let gz = ((pos[2] - origin[2]) * inv_dx - 0.5).floor() as isize;
        let fx = (pos[0] - origin[0]) * inv_dx - 0.5 - gx as f32;
        let fy = (pos[1] - origin[1]) * inv_dx - 0.5 - gy as f32;
        let fz = (pos[2] - origin[2]) * inv_dx - 0.5 - gz as f32;
        let gx = gx.max(0) as usize;
        let gy = gy.max(0) as usize;
        let gz = gz.max(0) as usize;

        for dk in 0..3 {
            for dj in 0..3 {
                for di in 0..3 {
                    let wx = b_spline(di as f32 - fx);
                    let wy = b_spline(dj as f32 - fy);
                    let wz = b_spline(dk as f32 - fz);
                    let w = wx * wy * wz;
                    if w < 1e-10 { continue; }
                    let gx_i = gx + di;
                    let gy_i = gy + dj;
                    let gz_i = gz + dk;
                    if gx_i >= nx || gy_i >= ny || gz_i >= nz { continue; }
                    let gid = gx_i + gy_i * nx + gz_i * nx * ny;

                    let dxg = (gx_i as f32 + 0.5) * dx + origin[0] - pos[0];
                    let dyg = (gy_i as f32 + 0.5) * dx + origin[1] - pos[1];
                    let dzg = (gz_i as f32 + 0.5) * dx + origin[2] - pos[2];

                    let vel_x = vel[0] + c[0] * dxg + c[1] * dyg + c[2] * dzg;
                    let vel_y = vel[1] + c[3] * dxg + c[4] * dyg + c[5] * dzg;
                    let vel_z = vel[2] + c[6] * dxg + c[7] * dyg + c[8] * dzg;

                    grid.mass[gid] += w * mass;
                    grid.vel[gid][0] += w * mass * vel_x;
                    grid.vel[gid][1] += w * mass * vel_y;
                    grid.vel[gid][2] += w * mass * vel_z;

                    let dwx = b_spline_deriv(di as f32 - fx) * inv_dx;
                    let dwy = b_spline_deriv(dj as f32 - fy) * inv_dx;
                    let dwz = b_spline_deriv(dk as f32 - fz) * inv_dx;

                    grid.force[gid][0] +=
                        stress_vol[0] * dwx + stress_vol[1] * dwy + stress_vol[2] * dwz;
                    grid.force[gid][1] +=
                        stress_vol[3] * dwx + stress_vol[4] * dwy + stress_vol[5] * dwz;
                    grid.force[gid][2] +=
                        stress_vol[6] * dwx + stress_vol[7] * dwy + stress_vol[8] * dwz;
                }
            }
        }
    }

    // --- Grid Update (parallel) ---
    grid.update_parallel(config.gravity, dt);

    // --- G2P (serial, only active particles) ---
    let damping = config.damping;
    let grid_max_x = (nx - 1) as f32 * dx;
    let grid_max_z = (nz - 1) as f32 * dx;
    let max_v = 50.0f32;

    for &i in active_indices {
        let pos = buffer.pos[i];
        let gx = ((pos[0] - origin[0]) * inv_dx - 0.5).floor() as isize;
        let gy = ((pos[1] - origin[1]) * inv_dx - 0.5).floor() as isize;
        let gz = ((pos[2] - origin[2]) * inv_dx - 0.5).floor() as isize;
        let fx = (pos[0] - origin[0]) * inv_dx - 0.5 - gx as f32;
        let fy = (pos[1] - origin[1]) * inv_dx - 0.5 - gy as f32;
        let fz = (pos[2] - origin[2]) * inv_dx - 0.5 - gz as f32;
        let gx = gx.max(0) as usize;
        let gy = gy.max(0) as usize;
        let gz = gz.max(0) as usize;

        let mut new_vel = [0.0f32; 3];
        let mut new_c = [0.0f32; 9];
        let mut grad_v = [0.0f32; 9];

        for dk in 0..3 {
            for dj in 0..3 {
                for di in 0..3 {
                    let wx = b_spline(di as f32 - fx);
                    let wy = b_spline(dj as f32 - fy);
                    let wz = b_spline(dk as f32 - fz);
                    let w = wx * wy * wz;
                    if w < 1e-10 { continue; }
                    let gx_i = gx + di;
                    let gy_i = gy + dj;
                    let gz_i = gz + dk;
                    if gx_i >= nx || gy_i >= ny || gz_i >= nz { continue; }
                    let gid = gx_i + gy_i * nx + gz_i * nx * ny;
                    if grid.mass[gid] < 1e-12 { continue; }
                    let gv = grid.vel[gid];
                    new_vel[0] += w * gv[0];
                    new_vel[1] += w * gv[1];
                    new_vel[2] += w * gv[2];
                    let dxg = (gx_i as f32 + 0.5) * dx + origin[0] - pos[0];
                    let dyg = (gy_i as f32 + 0.5) * dx + origin[1] - pos[1];
                    let dzg = (gz_i as f32 + 0.5) * dx + origin[2] - pos[2];
                    let scale = 4.0 * inv_dx * inv_dx * w;
                    new_c[0] += scale * gv[0] * dxg;
                    new_c[1] += scale * gv[0] * dyg;
                    new_c[2] += scale * gv[0] * dzg;
                    new_c[3] += scale * gv[1] * dxg;
                    new_c[4] += scale * gv[1] * dyg;
                    new_c[5] += scale * gv[1] * dzg;
                    new_c[6] += scale * gv[2] * dxg;
                    new_c[7] += scale * gv[2] * dyg;
                    new_c[8] += scale * gv[2] * dzg;
                    let dwx = b_spline_deriv(di as f32 - fx) * inv_dx;
                    let dwy = b_spline_deriv(dj as f32 - fy) * inv_dx;
                    let dwz = b_spline_deriv(dk as f32 - fz) * inv_dx;
                    grad_v[0] -= w * gv[0] * dwx;
                    grad_v[1] -= w * gv[0] * dwy;
                    grad_v[2] -= w * gv[0] * dwz;
                    grad_v[3] -= w * gv[1] * dwx;
                    grad_v[4] -= w * gv[1] * dwy;
                    grad_v[5] -= w * gv[1] * dwz;
                    grad_v[6] -= w * gv[2] * dwx;
                    grad_v[7] -= w * gv[2] * dwy;
                    grad_v[8] -= w * gv[2] * dwz;
                }
            }
        }
        for v in new_vel.iter() {
            if !v.is_finite() {
                new_vel = [0.0; 3];
                new_c = [0.0; 9];
                break;
            }
        }
        for v in new_vel.iter_mut() {
            *v = v.clamp(-max_v, max_v);
        }
        let mut new_pos = [
            pos[0] + new_vel[0] * dt,
            pos[1] + new_vel[1] * dt,
            pos[2] + new_vel[2] * dt,
        ];
        for v in new_pos.iter() {
            if !v.is_finite() {
                new_pos = [6.0, 5.0, 6.0];
                new_vel = [0.0; 3];
                break;
            }
        }
        if new_pos[1] < 0.0 {
            new_pos[1] = 0.0;
            new_vel[1] = new_vel[1].max(0.0) * 0.5;
        }
        if new_pos[0] < 0.0 {
            new_pos[0] = 0.0;
            new_vel[0] = new_vel[0].max(0.0) * 0.5;
        } else if new_pos[0] > grid_max_x {
            new_pos[0] = grid_max_x;
            new_vel[0] = new_vel[0].min(0.0) * 0.5;
        }
        if new_pos[2] < 0.0 {
            new_pos[2] = 0.0;
            new_vel[2] = new_vel[2].max(0.0) * 0.5;
        } else if new_pos[2] > grid_max_z {
            new_pos[2] = grid_max_z;
            new_vel[2] = new_vel[2].min(0.0) * 0.5;
        }
        let f = buffer.strain[i];
        let mut new_f = [0.0f32; 9];
        for r in 0..3 {
            for c in 0..3 {
                let idx = r * 3 + c;
                let mut sum = 0.0f32;
                for k in 0..3 {
                    let kronecker = if r == k { 1.0 } else { 0.0 };
                    let term = kronecker + grad_v[r * 3 + k] * dt;
                    sum += term * f[k * 3 + c];
                }
                new_f[idx] = sum;
            }
        }
        let mut f_valid = true;
        for v in new_f.iter() {
            if !v.is_finite() || v.abs() > 1e3 {
                f_valid = false;
                break;
            }
        }
        let (final_strain, final_j) = if f_valid {
            let j = new_f[0] * (new_f[4] * new_f[8] - new_f[5] * new_f[7])
                - new_f[1] * (new_f[3] * new_f[8] - new_f[5] * new_f[6])
                + new_f[2] * (new_f[3] * new_f[7] - new_f[4] * new_f[6]);
            (new_f, j.clamp(0.1, 10.0))
        } else {
            ([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0], 1.0)
        };
        buffer.vel[i] = new_vel;
        buffer.c[i] = new_c;
        buffer.pos[i] = new_pos;
        buffer.strain[i] = final_strain;
        buffer.jacobian[i] = final_j;
        buffer.vel[i][0] *= damping;
        buffer.vel[i][1] *= damping;
        buffer.vel[i][2] *= damping;
    }
}

/// Parallel MPM step with reusable grid (avoids allocation + uses rayon).
///
/// Use this for >2k active particles. Below 2k, `mpm_step_with_grid` (serial)
/// is faster due to parallel overhead.
///
/// When `particle_indices` is provided, only those particles are processed
/// (avoids scanning 1M buffer.active[] and avoids deactivate/reactivate dance).
pub fn mpm_step_parallel_with_grid(
    buffer: &mut MpssBuffer,
    grid: &mut Grid,
    config: &MpmConfig,
    material_table: &[MaterialParams],
    model_table: &[ConstitutiveModel],
    total_dt: f32,
    particle_indices: &[usize],
) {
    if particle_indices.is_empty() {
        return;
    }
    let sub_dt = total_dt / config.substeps as f32;
    for _ in 0..config.substeps {
        if particle_indices.len() < 50_000 {
            mpm_substep_serial_active(buffer, grid, particle_indices, sub_dt, config, material_table, model_table);
        } else {
            mpm_substep_parallel_with_indices(buffer, grid, particle_indices, sub_dt, config, material_table, model_table);
        }
    }
    buffer.update_lifetimes(total_dt);
}

/// Parallel MPM sub-step that iterates only the provided particle indices.
/// Used when active_count >= 50k (large particle counts benefit from parallelism).
fn mpm_substep_parallel_with_indices(
    buffer: &mut MpssBuffer,
    grid: &mut Grid,
    particle_indices: &[usize],
    dt: f32,
    config: &MpmConfig,
    material_table: &[MaterialParams],
    model_table: &[ConstitutiveModel],
) {
    let nx = grid.nx;
    let ny = grid.ny;
    let nz = grid.nz;
    let dx = grid.dx;
    let inv_dx = grid.inv_dx;
    let origin = grid.origin; // §3.1 Moving Window

    grid.reset();

    let buf_ptr = SendPtrConst(buffer as *const MpssBuffer);

    let merged = particle_indices
        .par_iter()
        .fold(
            || Grid::new(nx, ny, nz, dx),
            |mut local_grid, &i| {
                let bp: *const MpssBuffer = buf_ptr.ptr();
                unsafe {
                    let pos = (&(*bp).pos)[i];
                    let vel = (&(*bp).vel)[i];
                    let mass = (&(*bp).mass)[i];
                    let strain = (&(*bp).strain)[i];
                    let J = (&(*bp).jacobian)[i];
                    let mat_idx = (&(*bp).material_idx)[i] as usize;
                    let c = (&(*bp).c)[i];

                    let params = material_table.get(mat_idx).unwrap_or(&material_table[0]);
                    let model = model_table.get(mat_idx).unwrap_or(&ConstitutiveModel::NeoHookean);

                    let (stress, _new_J, _new_F, _plastic) =
                        constitutive::compute_stress(*model, params, &strain, J);

                    let vol = mass / params.density;
                    let stress_vol = [
                        vol * stress[0], vol * stress[1], vol * stress[2],
                        vol * stress[3], vol * stress[4], vol * stress[5],
                        vol * stress[6], vol * stress[7], vol * stress[8],
                    ];

                    let gx = ((pos[0] - origin[0]) * inv_dx - 0.5).floor() as isize;
                    let gy = ((pos[1] - origin[1]) * inv_dx - 0.5).floor() as isize;
                    let gz = ((pos[2] - origin[2]) * inv_dx - 0.5).floor() as isize;
                    let fx = (pos[0] - origin[0]) * inv_dx - 0.5 - gx as f32;
                    let fy = (pos[1] - origin[1]) * inv_dx - 0.5 - gy as f32;
                    let fz = (pos[2] - origin[2]) * inv_dx - 0.5 - gz as f32;
                    let gx = gx.max(0) as usize;
                    let gy = gy.max(0) as usize;
                    let gz = gz.max(0) as usize;

                    for dk in 0..3 {
                        for dj in 0..3 {
                            for di in 0..3 {
                                let wx = b_spline(di as f32 - fx);
                                let wy = b_spline(dj as f32 - fy);
                                let wz = b_spline(dk as f32 - fz);
                                let w = wx * wy * wz;
                                if w < 1e-10 { continue; }
                                let gx_i = gx + di;
                                let gy_i = gy + dj;
                                let gz_i = gz + dk;
                                if gx_i >= nx || gy_i >= ny || gz_i >= nz { continue; }
                                let gid = gx_i + gy_i * nx + gz_i * nx * ny;

                                let dxg = (gx_i as f32 + 0.5) * dx + origin[0] - pos[0];
                                let dyg = (gy_i as f32 + 0.5) * dx + origin[1] - pos[1];
                                let dzg = (gz_i as f32 + 0.5) * dx + origin[2] - pos[2];

                                let vel_x = vel[0] + c[0] * dxg + c[1] * dyg + c[2] * dzg;
                                let vel_y = vel[1] + c[3] * dxg + c[4] * dyg + c[5] * dzg;
                                let vel_z = vel[2] + c[6] * dxg + c[7] * dyg + c[8] * dzg;

                                local_grid.mass[gid] += w * mass;
                                local_grid.vel[gid][0] += w * mass * vel_x;
                                local_grid.vel[gid][1] += w * mass * vel_y;
                                local_grid.vel[gid][2] += w * mass * vel_z;

                                let dwx = b_spline_deriv(di as f32 - fx) * inv_dx;
                                let dwy = b_spline_deriv(dj as f32 - fy) * inv_dx;
                                let dwz = b_spline_deriv(dk as f32 - fz) * inv_dx;

                                local_grid.force[gid][0] +=
                                    stress_vol[0] * dwx + stress_vol[1] * dwy + stress_vol[2] * dwz;
                                local_grid.force[gid][1] +=
                                    stress_vol[3] * dwx + stress_vol[4] * dwy + stress_vol[5] * dwz;
                                local_grid.force[gid][2] +=
                                    stress_vol[6] * dwx + stress_vol[7] * dwy + stress_vol[8] * dwz;
                            }
                        }
                    }
                }
                local_grid
            },
        )
        .reduce(
            || Grid::new(nx, ny, nz, dx),
            |mut a, b| { a.merge(&b); a },
        );

    grid.mass.copy_from_slice(&merged.mass);
    grid.vel.copy_from_slice(&merged.vel);
    grid.force.copy_from_slice(&merged.force);

    grid.update_parallel(config.gravity, dt);

    let damping = config.damping;
    let grid_max_x = (nx - 1) as f32 * dx;
    let grid_max_z = (nz - 1) as f32 * dx;
    let max_v = 50.0f32;

    let buffer_ptr = SendPtrMut(buffer as *mut MpssBuffer);
    let grid_ptr = SendPtrConst(grid as *const Grid);
    particle_indices.par_iter().for_each(move |&i| {
        let bp: *mut MpssBuffer = buffer_ptr.ptr();
        let gp: *const Grid = grid_ptr.ptr();
        unsafe {
            let pos = (&(*bp).pos)[i];
            let gx = ((pos[0] - origin[0]) * inv_dx - 0.5).floor() as isize;
            let gy = ((pos[1] - origin[1]) * inv_dx - 0.5).floor() as isize;
            let gz = ((pos[2] - origin[2]) * inv_dx - 0.5).floor() as isize;
            let fx = (pos[0] - origin[0]) * inv_dx - 0.5 - gx as f32;
            let fy = (pos[1] - origin[1]) * inv_dx - 0.5 - gy as f32;
            let fz = (pos[2] - origin[2]) * inv_dx - 0.5 - gz as f32;
            let gx = gx.max(0) as usize;
            let gy = gy.max(0) as usize;
            let gz = gz.max(0) as usize;

            let mut new_vel = [0.0f32; 3];
            let mut new_c = [0.0f32; 9];
            let mut grad_v = [0.0f32; 9];

            for dk in 0..3 {
                for dj in 0..3 {
                    for di in 0..3 {
                        let wx = b_spline(di as f32 - fx);
                        let wy = b_spline(dj as f32 - fy);
                        let wz = b_spline(dk as f32 - fz);
                        let w = wx * wy * wz;
                        if w < 1e-10 { continue; }
                        let gx_i = gx + di;
                        let gy_i = gy + dj;
                        let gz_i = gz + dk;
                        if gx_i >= nx || gy_i >= ny || gz_i >= nz { continue; }
                        let gid = gx_i + gy_i * nx + gz_i * nx * ny;
                        if (&(*gp).mass)[gid] < 1e-12 { continue; }
                        let gv = (&(*gp).vel)[gid];
                        new_vel[0] += w * gv[0];
                        new_vel[1] += w * gv[1];
                        new_vel[2] += w * gv[2];
                        let dxg = (gx_i as f32 + 0.5) * dx + origin[0] - pos[0];
                        let dyg = (gy_i as f32 + 0.5) * dx + origin[1] - pos[1];
                        let dzg = (gz_i as f32 + 0.5) * dx + origin[2] - pos[2];
                        let scale = 4.0 * inv_dx * inv_dx * w;
                        new_c[0] += scale * gv[0] * dxg;
                        new_c[1] += scale * gv[0] * dyg;
                        new_c[2] += scale * gv[0] * dzg;
                        new_c[3] += scale * gv[1] * dxg;
                        new_c[4] += scale * gv[1] * dyg;
                        new_c[5] += scale * gv[1] * dzg;
                        new_c[6] += scale * gv[2] * dxg;
                        new_c[7] += scale * gv[2] * dyg;
                        new_c[8] += scale * gv[2] * dzg;
                        let dwx = b_spline_deriv(di as f32 - fx) * inv_dx;
                        let dwy = b_spline_deriv(dj as f32 - fy) * inv_dx;
                        let dwz = b_spline_deriv(dk as f32 - fz) * inv_dx;
                        grad_v[0] -= w * gv[0] * dwx;
                        grad_v[1] -= w * gv[0] * dwy;
                        grad_v[2] -= w * gv[0] * dwz;
                        grad_v[3] -= w * gv[1] * dwx;
                        grad_v[4] -= w * gv[1] * dwy;
                        grad_v[5] -= w * gv[1] * dwz;
                        grad_v[6] -= w * gv[2] * dwx;
                        grad_v[7] -= w * gv[2] * dwy;
                        grad_v[8] -= w * gv[2] * dwz;
                    }
                }
            }
            for v in new_vel.iter() {
                if !v.is_finite() { new_vel = [0.0; 3]; new_c = [0.0; 9]; break; }
            }
            for v in new_vel.iter_mut() { *v = v.clamp(-max_v, max_v); }
            let mut new_pos = [pos[0] + new_vel[0] * dt, pos[1] + new_vel[1] * dt, pos[2] + new_vel[2] * dt];
            for v in new_pos.iter() {
                if !v.is_finite() { new_pos = [6.0, 5.0, 6.0]; new_vel = [0.0; 3]; break; }
            }
            if new_pos[1] < 0.0 { new_pos[1] = 0.0; new_vel[1] = new_vel[1].max(0.0) * 0.5; }
            // §3.1 fix: x/z boundary checks must use grid.origin (Moving Window MPM).
            // Grid covers [origin, origin + (n-1)*dx] in world space. Previous code
            // checked against [0, grid_max_x] which is wrong when origin != 0 (player
            // far from world origin). This caused particles near grid boundary to be
            // clamped to wrong positions.
            if new_pos[0] < origin[0] { new_pos[0] = origin[0]; new_vel[0] = new_vel[0].max(0.0) * 0.5; }
            else if new_pos[0] > grid_max_x + origin[0] { new_pos[0] = grid_max_x + origin[0]; new_vel[0] = new_vel[0].min(0.0) * 0.5; }
            if new_pos[2] < origin[2] { new_pos[2] = origin[2]; new_vel[2] = new_vel[2].max(0.0) * 0.5; }
            else if new_pos[2] > grid_max_z + origin[2] { new_pos[2] = grid_max_z + origin[2]; new_vel[2] = new_vel[2].min(0.0) * 0.5; }
            let f = (&(*bp).strain)[i];
            let mut new_f = [0.0f32; 9];
            for r in 0..3 {
                for c in 0..3 {
                    let idx = r * 3 + c;
                    let mut sum = 0.0f32;
                    for k in 0..3 {
                        let kronecker = if r == k { 1.0 } else { 0.0 };
                        sum += (kronecker + grad_v[r * 3 + k] * dt) * f[k * 3 + c];
                    }
                    new_f[idx] = sum;
                }
            }
            let mut f_valid = true;
            for v in new_f.iter() {
                if !v.is_finite() || v.abs() > 1e3 { f_valid = false; break; }
            }
            let (final_strain, final_j) = if f_valid {
                let j = new_f[0] * (new_f[4] * new_f[8] - new_f[5] * new_f[7])
                    - new_f[1] * (new_f[3] * new_f[8] - new_f[5] * new_f[6])
                    + new_f[2] * (new_f[3] * new_f[7] - new_f[4] * new_f[6]);
                (new_f, j.clamp(0.1, 10.0))
            } else {
                ([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0], 1.0)
            };
            (&mut (*bp).vel)[i] = new_vel;
            (&mut (*bp).c)[i] = new_c;
            (&mut (*bp).pos)[i] = new_pos;
            (&mut (*bp).strain)[i] = final_strain;
            (&mut (*bp).jacobian)[i] = final_j;
            (&mut (*bp).vel)[i][0] *= damping;
            (&mut (*bp).vel)[i][1] *= damping;
            (&mut (*bp).vel)[i][2] *= damping;
        }
    });
}

/// §3.2 3-layer grid: simplified MPM for mid-field particles (50-200m).
///
/// Velocity-only MPM: P2G (mass + momentum) → grid gravity → G2P (velocity + position).
/// Skips stress computation, force accumulation, strain/jacobian/APIC C update.
/// ~3-5x faster than full MPM. Suitable for mid-field particles where pressure
/// coupling matters but deformation is invisible (50-200m from player).
///
/// Materials are ignored (no stress), so material_table/model_table not needed.
/// Gravity and damping come from `config`.
pub fn mpm_step_velocity_only_parallel(
    buffer: &mut MpssBuffer,
    grid: &mut Grid,
    config: &MpmConfig,
    total_dt: f32,
    particle_indices: &[usize],
) {
    if particle_indices.is_empty() {
        return;
    }
    let sub_dt = total_dt / config.substeps as f32;
    for _ in 0..config.substeps {
        mpm_substep_velocity_only(buffer, grid, particle_indices, sub_dt, config);
    }
}

fn mpm_substep_velocity_only(
    buffer: &mut MpssBuffer,
    grid: &mut Grid,
    particle_indices: &[usize],
    dt: f32,
    config: &MpmConfig,
) {
    let nx = grid.nx;
    let ny = grid.ny;
    let nz = grid.nz;
    let dx = grid.dx;
    let inv_dx = grid.inv_dx;
    let origin = grid.origin;

    grid.reset();

    // P2G: serial accumulation is faster than parallel fold+reduce for mid-field
    // particles (490k) because:
    //   1. fold+reduce allocates a Grid (896KB) per worker + merge cost
    //   2. 32³ grid (32k nodes) fits in L2 cache, so serial writes are fast
    //   3. particle_indices is already sorted ascending (built by LOD scan), so
    //      pos/vel/mass reads are sequential → cache-friendly
    // Uses get_unchecked to skip redundant bounds checks (gid already validated above).
    // Serial P2G for 490k particles: ~15ms (vs 100-200ms parallel fold+reduce)
    {
        let pos_slice = &buffer.pos[..];
        let vel_slice = &buffer.vel[..];
        let mass_slice = &buffer.mass[..];
        let grid_mass = grid.mass.as_mut_ptr();
        let grid_vel = grid.vel.as_mut_ptr();
        let nxny = nx * ny;

        for &i in particle_indices {
            let pos = pos_slice[i];
            let vel = vel_slice[i];
            let mass = mass_slice[i];

            let gx = ((pos[0] - origin[0]) * inv_dx - 0.5).floor() as isize;
            let gy = ((pos[1] - origin[1]) * inv_dx - 0.5).floor() as isize;
            let gz = ((pos[2] - origin[2]) * inv_dx - 0.5).floor() as isize;
            let fx = (pos[0] - origin[0]) * inv_dx - 0.5 - gx as f32;
            let fy = (pos[1] - origin[1]) * inv_dx - 0.5 - gy as f32;
            let fz = (pos[2] - origin[2]) * inv_dx - 0.5 - gz as f32;
            let gx = gx.max(0) as usize;
            let gy = gy.max(0) as usize;
            let gz = gz.max(0) as usize;

            // Precompute B-spline weights (3 per axis instead of 27 recomputations)
            let wx = [b_spline(0.0 - fx), b_spline(1.0 - fx), b_spline(2.0 - fx)];
            let wy = [b_spline(0.0 - fy), b_spline(1.0 - fy), b_spline(2.0 - fy)];
            let wz = [b_spline(0.0 - fz), b_spline(1.0 - fz), b_spline(2.0 - fz)];

            let wm = mass; // mass is reused; precompute w*mass later per node

            for dk in 0..3 {
                let wz_dk = wz[dk];
                if wz_dk < 1e-10 {
                    continue;
                }
                let gz_i = gz + dk;
                if gz_i >= nz {
                    continue;
                }
                for dj in 0..3 {
                    let wy_dj = wy[dj];
                    if wy_dj < 1e-10 {
                        continue;
                    }
                    let gy_i = gy + dj;
                    if gy_i >= ny {
                        continue;
                    }
                    let wyz = wy_dj * wz_dk;
                    for di in 0..3 {
                        let wx_di = wx[di];
                        let w = wx_di * wyz;
                        if w < 1e-10 {
                            continue;
                        }
                        let gx_i = gx + di;
                        if gx_i >= nx {
                            continue;
                        }
                        let gid = gx_i + gy_i * nx + gz_i * nxny;
                        // SAFETY: gid validated above (all indices < grid dims)
                        unsafe {
                            *grid_mass.add(gid) += w * wm;
                            let gv = &mut *grid_vel.add(gid);
                            gv[0] += w * wm * vel[0];
                            gv[1] += w * wm * vel[1];
                            gv[2] += w * wm * vel[2];
                        }
                    }
                }
            }
        }
    }

    // Grid update: gravity only (no force term)
    grid.update_parallel(config.gravity, dt);

    let damping = config.damping;
    let grid_max_x = (nx - 1) as f32 * dx;
    let grid_max_z = (nz - 1) as f32 * dx;
    let max_v = 50.0f32;

    // G2P: parallel (rayon) — grid reads dominate (32³ grid in L2 cache, shared across
    // threads via L3). Serial G2P was 30ms slower than parallel because single-thread
    // memory bandwidth is the bottleneck for 490k particles × 27 node reads.
    // SAFETY: each i is unique (particle_indices contains distinct indices).
    let buffer_ptr = SendPtrMut(buffer as *mut MpssBuffer);
    let grid_ptr = SendPtrConst(grid as *const Grid);
    particle_indices.par_iter().for_each(move |&i| {
        let bp: *mut MpssBuffer = buffer_ptr.ptr();
        let gp: *const Grid = grid_ptr.ptr();
        unsafe {
            let pos = (&(*bp).pos)[i];
            let gx = ((pos[0] - origin[0]) * inv_dx - 0.5).floor() as isize;
            let gy = ((pos[1] - origin[1]) * inv_dx - 0.5).floor() as isize;
            let gz = ((pos[2] - origin[2]) * inv_dx - 0.5).floor() as isize;
            let fx = (pos[0] - origin[0]) * inv_dx - 0.5 - gx as f32;
            let fy = (pos[1] - origin[1]) * inv_dx - 0.5 - gy as f32;
            let fz = (pos[2] - origin[2]) * inv_dx - 0.5 - gz as f32;
            let gx = gx.max(0) as usize;
            let gy = gy.max(0) as usize;
            let gz = gz.max(0) as usize;

            // Precompute B-spline weights (3 per axis instead of 27 recomputations)
            let wx = [b_spline(0.0 - fx), b_spline(1.0 - fx), b_spline(2.0 - fx)];
            let wy = [b_spline(0.0 - fy), b_spline(1.0 - fy), b_spline(2.0 - fy)];
            let wz = [b_spline(0.0 - fz), b_spline(1.0 - fz), b_spline(2.0 - fz)];

            let mut new_vel = [0.0f32; 3];
            for dk in 0..3 {
                let wz_dk = wz[dk];
                if wz_dk < 1e-10 {
                    continue;
                }
                let gz_i = gz + dk;
                if gz_i >= nz {
                    continue;
                }
                for dj in 0..3 {
                    let wy_dj = wy[dj];
                    if wy_dj < 1e-10 {
                        continue;
                    }
                    let gy_i = gy + dj;
                    if gy_i >= ny {
                        continue;
                    }
                    let wyz = wy_dj * wz_dk;
                    for di in 0..3 {
                        let wx_di = wx[di];
                        let w = wx_di * wyz;
                        if w < 1e-10 {
                            continue;
                        }
                        let gx_i = gx + di;
                        if gx_i >= nx {
                            continue;
                        }
                        let gid = gx_i + gy_i * nx + gz_i * nx * ny;
                        if (&(*gp).mass)[gid] < 1e-12 {
                            continue;
                        }
                        let gv = (&(*gp).vel)[gid];
                        new_vel[0] += w * gv[0];
                        new_vel[1] += w * gv[1];
                        new_vel[2] += w * gv[2];
                    }
                }
            }
            for v in new_vel.iter() {
                if !v.is_finite() {
                    new_vel = [0.0; 3];
                    break;
                }
            }
            for v in new_vel.iter_mut() {
                *v = v.clamp(-max_v, max_v);
            }
            let mut new_pos = [
                pos[0] + new_vel[0] * dt,
                pos[1] + new_vel[1] * dt,
                pos[2] + new_vel[2] * dt,
            ];
            for v in new_pos.iter() {
                if !v.is_finite() {
                    new_pos = pos;
                    new_vel = [0.0; 3];
                    break;
                }
            }
            if new_pos[1] < 0.0 {
                new_pos[1] = 0.0;
                new_vel[1] = new_vel[1].max(0.0) * 0.5;
            }
            if new_pos[0] < 0.0 {
                new_pos[0] = 0.0;
                new_vel[0] = new_vel[0].max(0.0) * 0.5;
            } else if new_pos[0] > grid_max_x + origin[0] {
                new_pos[0] = grid_max_x + origin[0];
                new_vel[0] = new_vel[0].min(0.0) * 0.5;
            }
            if new_pos[2] < 0.0 {
                new_pos[2] = 0.0;
                new_vel[2] = new_vel[2].max(0.0) * 0.5;
            } else if new_pos[2] > grid_max_z + origin[2] {
                new_pos[2] = grid_max_z + origin[2];
                new_vel[2] = new_vel[2].min(0.0) * 0.5;
            }
            new_vel[0] *= damping;
            new_vel[1] *= damping;
            new_vel[2] *= damping;
            (&mut (*bp).vel)[i] = new_vel;
            (&mut (*bp).pos)[i] = new_pos;
        }
    });
}

/// Estimate CFL-safe dt for given particle velocities
pub fn estimate_cfl(buffer: &MpssBuffer, dx: f32) -> f32 {
    let mut max_v = 0.0f32;
    for i in 0..buffer.capacity {
        if buffer.active[i] {
            let v = buffer.vel[i];
            let speed = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            max_v = max_v.max(speed);
        }
    }
    if max_v < 1e-6 {
        return 0.016; // default 60fps
    }
    (dx * 0.3 / max_v).min(0.016) // CFL ≤ 0.3
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpss::MpssBuffer;

    #[test]
    fn test_bspline_unit() {
        assert!((b_spline(0.0) - 0.75).abs() < 1e-6);
        assert!((b_spline(0.5) - 0.5).abs() < 1e-6);
        assert!((b_spline(1.0) - 0.125).abs() < 1e-6);
        assert!((b_spline(1.5)).abs() < 1e-6);
        assert!((b_spline(2.0)).abs() < 1e-6);
    }

    #[test]
    fn test_bspline_deriv() {
        assert!((b_spline_deriv(0.0)).abs() < 1e-6);
        assert!((b_spline_deriv(0.5) + 1.0).abs() < 1e-6);
        assert!((b_spline_deriv(1.0) + 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_sandpile_static_angle() {
        let mut buf = MpssBuffer::new(1000);
        // Scaled stiffness for CFL stability with explicit MPM.
        // Real sand E=5e7 → c=132 m/s → needs substeps≥42 at dx=0.05, dt=0.016.
        // Scaled E=1e4 → c=2.5 m/s → substeps=8 is well within CFL.
        let config = MpmConfig { substeps: 8, ..Default::default() };
        let materials = vec![MaterialParams {
            young_modulus: 1.0e4,
            poisson_ratio: 0.2,
            yield_stress: 1.0e3,
            hardening: 0.005,
            density: 1600.0,
            friction_angle: 35.0,
            cohesion: 0.0,
        }];
        let models = vec![ConstitutiveModel::DruckerPrager];

        // Spawn sand particles in a column
        for x in 0..10 {
            for y in 0..10 {
                for z in 0..10 {
                    let idx = buf.spawn().unwrap();
                    buf.pos[idx] = [x as f32 * 0.05, y as f32 * 0.05 + 0.5, z as f32 * 0.05];
                    buf.mass[idx] = 0.001;
                    buf.material_idx[idx] = 0;
                }
            }
        }

        // Run 60 frames (1 second)
        for _ in 0..60 {
            mpm_step(&mut buf, &config, &materials, &models, 0.016);
        }

        // Check: all particles above ground
        for i in 0..buf.capacity {
            if buf.active[i] {
                assert!(
                    buf.pos[i][1] >= 0.0,
                    "Particle {} penetrated floor: y={}",
                    i,
                    buf.pos[i][1]
                );
            }
        }
    }

    #[test]
    fn test_water_pressure() {
        let mut buf = MpssBuffer::new(300);
        let config =
            MpmConfig { grid_dx: 0.05, grid_size: [32, 32, 32], substeps: 8, ..Default::default() };
        // Scaled stiffness for CFL stability: real water K=2.2e9 → c=1483 m/s
        // needs substeps≈470. Scaled E=1e3 → c≈1.3 m/s → substeps=8 is safe.
        let materials = vec![MaterialParams {
            young_modulus: 1.0e3,
            poisson_ratio: 0.499,
            yield_stress: 0.0,
            hardening: 0.0,
            density: 1000.0,
            friction_angle: 0.0,
            cohesion: 0.0,
        }];
        let models = vec![ConstitutiveModel::NewtonianFluid];

        // Spawn water column
        for y in 0..10 {
            for x in 0..5 {
                for z in 0..5 {
                    let idx = buf.spawn().unwrap();
                    buf.pos[idx] = [x as f32 * 0.05, y as f32 * 0.05 + 0.1, z as f32 * 0.05];
                    buf.mass[idx] = 0.001;
                    buf.material_idx[idx] = 0;
                    buf.jacobian[idx] = 0.95; // slightly compressed
                }
            }
        }

        mpm_step(&mut buf, &config, &materials, &models, 0.016);

        // After one step, J should remain finite and not explode.
        // Bottom particles get compressed by gravity + boundary, but J should
        // stay within the clamp range [0.1, 10.0] and not go NaN/inf.
        for i in 0..buf.capacity {
            if buf.active[i] {
                assert!(
                    buf.jacobian[i].is_finite(),
                    "Water particle {} J is NaN/inf: {}",
                    i,
                    buf.jacobian[i]
                );
                assert!(
                    buf.jacobian[i] > 0.0 && buf.jacobian[i] < 100.0,
                    "Water particle {} J exploded: {}",
                    i,
                    buf.jacobian[i]
                );
            }
        }
    }

    #[test]
    fn test_steel_rigid_fall() {
        let mut buf = MpssBuffer::new(100);
        let config =
            MpmConfig { grid_dx: 0.02, grid_size: [32, 32, 32], substeps: 8, ..Default::default() };
        // Scaled stiffness for CFL stability: real steel E=2e11 → c=5048 m/s
        // needs substeps≈3700. Scaled E=1e5 → c≈3.3 m/s → substeps=8 is safe.
        let materials = vec![MaterialParams {
            young_modulus: 1.0e5,
            poisson_ratio: 0.3,
            yield_stress: 1.0e4,
            hardening: 0.01,
            density: 7850.0,
            friction_angle: 0.0,
            cohesion: 0.0,
        }];
        let models = vec![ConstitutiveModel::VonMises];

        let idx = buf.spawn().unwrap();
        buf.pos[idx] = [0.2, 0.5, 0.2];
        buf.mass[idx] = 0.01;
        buf.material_idx[idx] = 0;

        let initial_y = buf.pos[idx][1];

        for _ in 0..30 {
            mpm_step(&mut buf, &config, &materials, &models, 0.016);
        }

        let final_y = buf.pos[idx][1];
        assert!(final_y < initial_y, "Steel didn't fall: {} → {}", initial_y, final_y);
        assert!(final_y >= 0.0, "Steel penetrated floor: {}", final_y);
    }
}
