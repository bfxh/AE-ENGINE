use crate::properties::*;
use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluidCell {
    pub pressure: f32,
    pub velocity: Vec3,
    pub density: f32,
    pub temperature: f32,
    pub is_fluid: bool,
    pub is_solid: bool,
}

impl Default for FluidCell {
    fn default() -> Self {
        Self {
            pressure: 0.0,
            velocity: Vec3::ZERO,
            density: 0.0,
            temperature: 0.0,
            is_fluid: false,
            is_solid: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavierStokesSolver {
    pub dimensions: (usize, usize, usize),
    pub spacing: f32,
    pub time_step: f32,
    pub cells: Vec<FluidCell>,
    pub properties: FluidProperties,
    pub iterations: u32,
    pub tolerance: f32,
    pub gravity: Vec3,
    pub use_smoke: bool,
    #[serde(skip, default)]
    pub pressure_buf: Vec<f32>,
    #[serde(skip, default)]
    pub velocity_scratch: Vec<Vec3>,
    #[serde(skip, default)]
    pub velocity_buf: Vec<Vec3>,
    #[serde(skip, default)]
    pub density_buf: Vec<f32>,
    #[serde(skip, default)]
    pub temperature_buf: Vec<f32>,
}

impl NavierStokesSolver {
    pub fn new(
        dimensions: (usize, usize, usize),
        spacing: f32,
        properties: FluidProperties,
    ) -> Self {
        let (nx, ny, nz) = dimensions;
        let total = nx * ny * nz;
        let mut cells = vec![FluidCell::default(); total];
        for (i, cell) in cells.iter_mut().enumerate() {
            let idx = Self::index_to_coord(i, dimensions);
            let y = idx.1 as f32 * spacing;
            if y < (ny as f32 * spacing * 0.3) {
                cell.is_fluid = true;
                cell.density = properties.density;
                cell.temperature = 293.15;
            }
            if idx.0 == 0
                || idx.0 == nx - 1
                || idx.1 == 0
                || idx.1 == ny - 1
                || idx.2 == 0
                || idx.2 == nz - 1
            {
                cell.is_solid = true;
                cell.is_fluid = false;
            }
        }
        Self {
            dimensions,
            spacing,
            time_step: 1.0 / 60.0,
            cells,
            properties,
            iterations: 40,
            tolerance: 1e-4,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            use_smoke: false,
            pressure_buf: Vec::new(),
            velocity_scratch: Vec::new(),
            velocity_buf: Vec::new(),
            density_buf: Vec::new(),
            temperature_buf: Vec::new(),
        }
    }

    pub fn index(&self, x: usize, y: usize, z: usize) -> usize {
        let (nx, ny, _) = self.dimensions;
        x + y * nx + z * nx * ny
    }

    pub fn index_to_coord(idx: usize, (nx, ny, _): (usize, usize, usize)) -> (usize, usize, usize) {
        let z = idx / (nx * ny);
        let rem = idx % (nx * ny);
        let y = rem / nx;
        let x = rem % nx;
        (x, y, z)
    }

    pub fn add_force(&mut self, pos: Vec3, force: Vec3, radius: f32) {
        let (nx, ny, nz) = self.dimensions;
        let half_r = (radius / self.spacing).ceil() as usize;
        let cx = (pos.x / self.spacing) as isize;
        let cy = (pos.y / self.spacing) as isize;
        let cz = (pos.z / self.spacing) as isize;

        for dz in -(half_r as isize)..=half_r as isize {
            for dy in -(half_r as isize)..=half_r as isize {
                for dx in -(half_r as isize)..=half_r as isize {
                    let x = cx + dx;
                    let y = cy + dy;
                    let z = cz + dz;
                    if x < 0
                        || x >= nx as isize
                        || y < 0
                        || y >= ny as isize
                        || z < 0
                        || z >= nz as isize
                    {
                        continue;
                    }
                    let idx = self.index(x as usize, y as usize, z as usize);
                    if !self.cells[idx].is_fluid {
                        continue;
                    }
                    let rel_pos = Vec3::new(dx as f32, dy as f32, dz as f32) * self.spacing;
                    let dist = rel_pos.length();
                    if dist > radius {
                        continue;
                    }
                    let falloff = 1.0 - (dist / radius).powi(2);
                    self.cells[idx].velocity += force * falloff;
                }
            }
        }
    }

    pub fn apply_gravity(&mut self) {
        for cell in self.cells.iter_mut() {
            if cell.is_fluid {
                cell.velocity += self.gravity * self.time_step;
            }
        }
    }

    pub fn diffuse(&mut self) {
        let (nx, ny, nz) = self.dimensions;
        let nu = self.properties.kinematic_viscosity();
        let factor = nu * self.time_step / (self.spacing * self.spacing);

        let len = self.cells.len();
        let mut new_velocities = std::mem::take(&mut self.velocity_scratch);
        new_velocities.resize(len, Vec3::ZERO);
        new_velocities.fill(Vec3::ZERO);
        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let idx = self.index(x, y, z);
                    if !self.cells[idx].is_fluid {
                        continue;
                    }
                    let mut neighbors = 0.0;
                    let mut avg_vel = Vec3::ZERO;

                    for dir in [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z] {
                        let adj_x = x as isize + dir.x as isize;
                        let adj_y = y as isize + dir.y as isize;
                        let adj_z = z as isize + dir.z as isize;
                        if adj_x < 0
                            || adj_x >= nx as isize
                            || adj_y < 0
                            || adj_y >= ny as isize
                            || adj_z < 0
                            || adj_z >= nz as isize
                        {
                            continue;
                        }
                        let n_idx = self.index(adj_x as usize, adj_y as usize, adj_z as usize);
                        if self.cells[n_idx].is_fluid || self.cells[n_idx].is_solid {
                            avg_vel += self.cells[n_idx].velocity;
                            neighbors += 1.0;
                        }
                    }

                    if neighbors > 0.0 {
                        avg_vel /= neighbors;
                        new_velocities[idx] = self.cells[idx].velocity
                            + (avg_vel - self.cells[idx].velocity) * factor;
                    }
                }
            }
        }

        for (idx, vel) in new_velocities.iter().enumerate() {
            if self.cells[idx].is_fluid {
                self.cells[idx].velocity = *vel;
            }
        }
        self.velocity_scratch = new_velocities;
    }

    pub fn advect(&mut self) {
        let (nx, ny, nz) = self.dimensions;
        let len = self.cells.len();

        self.velocity_buf.resize(len, Vec3::ZERO);
        self.density_buf.resize(len, 0.0);
        self.temperature_buf.resize(len, 0.0);

        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let idx = self.index(x, y, z);
                    if !self.cells[idx].is_fluid {
                        continue;
                    }

                    let pos = Vec3::new(x as f32, y as f32, z as f32) * self.spacing;
                    let prev_pos = pos - self.cells[idx].velocity * self.time_step;
                    let prev_grid = prev_pos / self.spacing;

                    let ix = prev_grid.x.floor() as isize;
                    let iy = prev_grid.y.floor() as isize;
                    let iz = prev_grid.z.floor() as isize;

                    let tx = prev_grid.x - ix as f32;
                    let ty = prev_grid.y - iy as f32;
                    let tz = prev_grid.z - iz as f32;

                    let mut vel = Vec3::ZERO;
                    let mut density = 0.0;
                    let mut temp = 0.0;

                    for dz in 0..=1 {
                        for dy in 0..=1 {
                            for dx in 0..=1 {
                                let cx = ix + dx as isize;
                                let cy = iy + dy as isize;
                                let cz = iz + dz as isize;
                                if cx < 0
                                    || cx >= nx as isize
                                    || cy < 0
                                    || cy >= ny as isize
                                    || cz < 0
                                    || cz >= nz as isize
                                {
                                    continue;
                                }
                                let c_idx = self.index(cx as usize, cy as usize, cz as usize);
                                let weight = (if dx == 0 { 1.0 - tx } else { tx })
                                    * (if dy == 0 { 1.0 - ty } else { ty })
                                    * (if dz == 0 { 1.0 - tz } else { tz });
                                vel += self.cells[c_idx].velocity * weight;
                                density += self.cells[c_idx].density * weight;
                                temp += self.cells[c_idx].temperature * weight;
                            }
                        }
                    }

                    self.velocity_buf[idx] = vel;
                    self.density_buf[idx] = density;
                    self.temperature_buf[idx] = temp;
                }
            }
        }

        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let idx = self.index(x, y, z);
                    if self.cells[idx].is_fluid {
                        self.cells[idx].velocity = self.velocity_buf[idx];
                        self.cells[idx].density = self.density_buf[idx];
                        self.cells[idx].temperature = self.temperature_buf[idx];
                    }
                }
            }
        }
    }

    pub fn solve_pressure(&mut self) {
        let (nx, ny, nz) = self.dimensions;
        let mut pressures = vec![0.0; self.cells.len()];
        let mut divergence = vec![0.0; self.cells.len()];

        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let idx = self.index(x, y, z);
                    if !self.cells[idx].is_fluid {
                        continue;
                    }

                    let mut div = 0.0;
                    let h = self.spacing;

                    if x > 0 {
                        div += self.cells[self.index(x - 1, y, z)].velocity.x;
                    }
                    if x < nx - 1 {
                        div -= self.cells[self.index(x + 1, y, z)].velocity.x;
                    }
                    if y > 0 {
                        div += self.cells[self.index(x, y - 1, z)].velocity.y;
                    }
                    if y < ny - 1 {
                        div -= self.cells[self.index(x, y + 1, z)].velocity.y;
                    }
                    if z > 0 {
                        div += self.cells[self.index(x, y, z - 1)].velocity.z;
                    }
                    if z < nz - 1 {
                        div -= self.cells[self.index(x, y, z + 1)].velocity.z;
                    }

                    divergence[idx] = div * 0.5 / h;
                }
            }
        }

        let mut buf = std::mem::take(&mut self.pressure_buf);
        buf.resize(self.cells.len(), 0.0);
        buf.fill(0.0);

        for _ in 0..self.iterations {
            let mut max_change: f32 = 0.0;

            for z in 0..nz {
                for y in 0..ny {
                    for x in 0..nx {
                        let idx = self.index(x, y, z);
                        if !self.cells[idx].is_fluid {
                            continue;
                        }

                        let mut sum_p = 0.0;
                        let mut count = 0.0;

                        if x > 0
                            && (self.cells[self.index(x - 1, y, z)].is_fluid
                                || self.cells[self.index(x - 1, y, z)].is_solid)
                        {
                            sum_p += pressures[self.index(x - 1, y, z)];
                            count += 1.0;
                        }
                        if x < nx - 1
                            && (self.cells[self.index(x + 1, y, z)].is_fluid
                                || self.cells[self.index(x + 1, y, z)].is_solid)
                        {
                            sum_p += pressures[self.index(x + 1, y, z)];
                            count += 1.0;
                        }
                        if y > 0
                            && (self.cells[self.index(x, y - 1, z)].is_fluid
                                || self.cells[self.index(x, y - 1, z)].is_solid)
                        {
                            sum_p += pressures[self.index(x, y - 1, z)];
                            count += 1.0;
                        }
                        if y < ny - 1
                            && (self.cells[self.index(x, y + 1, z)].is_fluid
                                || self.cells[self.index(x, y + 1, z)].is_solid)
                        {
                            sum_p += pressures[self.index(x, y + 1, z)];
                            count += 1.0;
                        }
                        if z > 0
                            && (self.cells[self.index(x, y, z - 1)].is_fluid
                                || self.cells[self.index(x, y, z - 1)].is_solid)
                        {
                            sum_p += pressures[self.index(x, y, z - 1)];
                            count += 1.0;
                        }
                        if z < nz - 1
                            && (self.cells[self.index(x, y, z + 1)].is_fluid
                                || self.cells[self.index(x, y, z + 1)].is_solid)
                        {
                            sum_p += pressures[self.index(x, y, z + 1)];
                            count += 1.0;
                        }

                        if count > 0.0 {
                            let h_sq = self.spacing * self.spacing;
                            let rho = self.cells[idx].density.max(1e-6);
                            let new_p = (sum_p - h_sq * divergence[idx] * rho) / count;
                            max_change = max_change.max((new_p - pressures[idx]).abs());
                            buf[idx] = new_p;
                        }
                    }
                }
            }

            std::mem::swap(&mut pressures, &mut buf);

            if max_change < self.tolerance {
                break;
            }
        }

        self.pressure_buf = buf;

        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let idx = self.index(x, y, z);
                    if !self.cells[idx].is_fluid {
                        continue;
                    }
                    self.cells[idx].pressure = pressures[idx];

                    let h = self.spacing;
                    let rho = self.cells[idx].density.max(1e-6);

                    if x > 0 && x < nx - 1 {
                        self.cells[idx].velocity.x -= (pressures[self.index(x + 1, y, z)]
                            - pressures[self.index(x - 1, y, z)])
                            / (2.0 * h * rho)
                            * self.time_step;
                    }
                    if y > 0 && y < ny - 1 {
                        self.cells[idx].velocity.y -= (pressures[self.index(x, y + 1, z)]
                            - pressures[self.index(x, y - 1, z)])
                            / (2.0 * h * rho)
                            * self.time_step;
                    }
                    if z > 0 && z < nz - 1 {
                        self.cells[idx].velocity.z -= (pressures[self.index(x, y, z + 1)]
                            - pressures[self.index(x, y, z - 1)])
                            / (2.0 * h * rho)
                            * self.time_step;
                    }
                }
            }
        }
    }

    pub fn step(&mut self) {
        self.apply_gravity();
        self.diffuse();
        self.solve_pressure();
        self.advect();
    }

    pub fn velocity_at(&self, pos: Vec3) -> Vec3 {
        let (nx, ny, nz) = self.dimensions;
        let grid = pos / self.spacing;
        let ix = grid.x.floor() as isize;
        let iy = grid.y.floor() as isize;
        let iz = grid.z.floor() as isize;
        let tx = grid.x - ix as f32;
        let ty = grid.y - iy as f32;
        let tz = grid.z - iz as f32;

        let mut vel = Vec3::ZERO;
        for dz in 0..=1 {
            for dy in 0..=1 {
                for dx in 0..=1 {
                    let x = ix + dx as isize;
                    let y = iy + dy as isize;
                    let z = iz + dz as isize;
                    if x < 0
                        || x >= nx as isize
                        || y < 0
                        || y >= ny as isize
                        || z < 0
                        || z >= nz as isize
                    {
                        continue;
                    }
                    let idx = self.index(x as usize, y as usize, z as usize);
                    let weight = (if dx == 0 { 1.0 - tx } else { tx })
                        * (if dy == 0 { 1.0 - ty } else { ty })
                        * (if dz == 0 { 1.0 - tz } else { tz });
                    vel += self.cells[idx].velocity * weight;
                }
            }
        }
        vel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_creation() {
        let solver = NavierStokesSolver::new((32, 32, 32), 0.1, FLUID_WATER);
        assert_eq!(solver.cells.len(), 32 * 32 * 32);
    }

    #[test]
    fn test_gravity() {
        let mut solver = NavierStokesSolver::new((32, 32, 32), 0.1, FLUID_WATER);
        let idx = solver.index(16, 5, 16);
        let before = solver.cells[idx].velocity;
        solver.apply_gravity();
        assert!(solver.cells[idx].velocity.y < before.y);
    }

    #[test]
    fn test_velocity_at() {
        let solver = NavierStokesSolver::new((32, 32, 32), 0.1, FLUID_WATER);
        let vel = solver.velocity_at(Vec3::new(1.6, 1.6, 1.6));
        assert_eq!(vel, Vec3::ZERO);
    }
}
