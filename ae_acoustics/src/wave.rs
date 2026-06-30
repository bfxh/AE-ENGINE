use crate::sources::*;
use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcousticCell {
    pub pressure: f32,
    pub velocity: Vec3,
    pub density: f32,
}

impl Default for AcousticCell {
    fn default() -> Self {
        Self { pressure: 0.0, velocity: Vec3::ZERO, density: 1.21 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcousticSolver {
    pub dimensions: (usize, usize, usize),
    pub spacing: f32,
    pub time_step: f32,
    pub pressure_field: Vec<f32>,
    pub pressure_prev_field: Vec<f32>,
    pub velocity_field: Vec<Vec3>,
    pub speed_of_sound: f32,
    pub sources: Vec<SoundSource>,
    pub absorption: Vec<f32>,
}

impl AcousticSolver {
    pub fn new(dimensions: (usize, usize, usize), spacing: f32) -> Self {
        let (nx, ny, nz) = dimensions;
        let total = nx * ny * nz;
        Self {
            dimensions,
            spacing,
            time_step: spacing / (343.0 * 1.414),
            pressure_field: vec![0.0; total],
            pressure_prev_field: vec![0.0; total],
            velocity_field: vec![Vec3::ZERO; total],
            speed_of_sound: 343.0,
            sources: Vec::new(),
            absorption: vec![0.01; total],
        }
    }

    pub fn index(&self, x: usize, y: usize, z: usize) -> usize {
        let (nx, ny, _) = self.dimensions;
        x + y * nx + z * nx * ny
    }

    pub fn add_source(&mut self, source: SoundSource) {
        self.sources.push(source);
    }

    pub fn step(&mut self, dt: f32) {
        let (nx, ny, nz) = self.dimensions;
        let mut new_pressure = self.pressure_field.clone();
        let c = self.speed_of_sound;
        let h = self.spacing;
        let dt_clamped = dt.min(self.time_step);
        let c2 = c * c;
        let dt2 = dt_clamped * dt_clamped;
        let h2 = h * h;

        for source in &mut self.sources {
            source.step(dt_clamped);
        }

        for z in 1..nz - 1 {
            for y in 1..ny - 1 {
                for x in 1..nx - 1 {
                    let idx = self.index(x, y, z);
                    let laplacian = (self.pressure_field[self.index(x + 1, y, z)]
                        - 2.0 * self.pressure_field[idx]
                        + self.pressure_field[self.index(x - 1, y, z)])
                        / h2
                        + (self.pressure_field[self.index(x, y + 1, z)]
                            - 2.0 * self.pressure_field[idx]
                            + self.pressure_field[self.index(x, y - 1, z)])
                            / h2
                        + (self.pressure_field[self.index(x, y, z + 1)]
                            - 2.0 * self.pressure_field[idx]
                            + self.pressure_field[self.index(x, y, z - 1)])
                            / h2;

                    let source_term = self
                        .sources
                        .iter()
                        .map(|s| {
                            let pos = Vec3::new(x as f32, y as f32, z as f32) * h;
                            s.pressure_at(pos, s.time)
                        })
                        .sum::<f32>();

                    let absorption_factor = 1.0 - self.absorption[idx];
                    new_pressure[idx] = 2.0 * self.pressure_field[idx] * absorption_factor
                        - self.pressure_prev_field[idx] * absorption_factor * absorption_factor
                        + c2 * dt2 * laplacian
                        + dt2 * source_term;
                }
            }
        }

        self.pressure_prev_field = std::mem::replace(&mut self.pressure_field, new_pressure);
    }

    pub fn pressure_at(&self, pos: Vec3) -> f32 {
        let (nx, ny, nz) = self.dimensions;
        let grid = pos / self.spacing;
        let ix = grid.x.floor() as isize;
        let iy = grid.y.floor() as isize;
        let iz = grid.z.floor() as isize;
        let tx = grid.x - ix as f32;
        let ty = grid.y - iy as f32;
        let tz = grid.z - iz as f32;

        let mut p = 0.0;
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
                    p += self.pressure_field[idx] * weight;
                }
            }
        }
        p
    }

    pub fn raycast(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> Vec<(Vec3, f32)> {
        let mut hits = Vec::new();
        let step = self.spacing;
        let mut t = 0.0;
        let dir = direction.normalize();
        while t < max_distance {
            let pos = origin + dir * t;
            let p = self.pressure_at(pos);
            if p.abs() > 0.001 {
                hits.push((pos, p));
            }
            t += step;
        }
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_creation() {
        let solver = AcousticSolver::new((32, 32, 32), 0.1);
        assert_eq!(solver.pressure_field.len(), 32 * 32 * 32);
    }

    #[test]
    fn test_source_pressure() {
        let source = SoundSource::new_point(0, Vec3::ZERO, 440.0, 1.0);
        let p = source.pressure_at(Vec3::new(1.0, 0.0, 0.0), 0.0);
        assert!(p.is_finite());
    }

    #[test]
    fn test_solver_step() {
        let mut solver = AcousticSolver::new((32, 32, 32), 0.1);
        let source = SoundSource::new_point(0, Vec3::new(1.6, 1.6, 1.6), 440.0, 100.0);
        solver.add_source(source);
        solver.step(1.0 / 60.0);
        let p = solver.pressure_at(Vec3::new(1.6, 1.6, 1.6));
        assert!(p.is_finite());
    }
}
