//! 感染体素网格

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct MyceliumVoxel {
    pub infected: f32,
    pub density: f32,
    pub nutrient: f32,
    pub spore: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyceliumGrid {
    pub size: usize,
    pub voxels: Vec<MyceliumVoxel>,
    pub growth_rate: f32,
    pub step_count: u64,
    pub rng_seed: u64,
}

impl MyceliumGrid {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            voxels: vec![MyceliumVoxel::default(); size * size * size],
            growth_rate: 0.3,
            step_count: 0,
            rng_seed: 42,
        }
    }

    pub fn seed_center(&mut self, x: usize, y: usize, z: usize) {
        if let Some(v) = self.get_mut(x, y, z) {
            v.infected = 1.0;
            v.density = 0.5;
            v.nutrient = 1.0;
        }
    }

    pub fn step(&mut self, dt: f32) {
        let size = self.size;
        let mut new_voxels = self.voxels.clone();

        for x in 1..size-1 {
            for y in 1..size-1 {
                for z in 1..size-1 {
                    let idx = x + y * size + z * size * size;
                    let current = self.voxels[idx];
                    if current.infected <= 0.0 {
                        continue;
                    }

                    // 从当前感染体素向邻居扩散密度
                    for &(dx, dy, dz) in &[(0,1,0),(0,-1,0),(1,0,0),(-1,0,0),(0,0,1),(0,0,-1)] {
                        let nx = (x as isize + dx) as usize;
                        let ny = (y as isize + dy) as usize;
                        let nz = (z as isize + dz) as usize;
                        let nidx = nx + ny * size + nz * size * size;
                        let spread = self.growth_rate * dt * current.infected;
                        let n = &mut new_voxels[nidx];
                        n.density = (n.density + spread).min(1.0);
                        n.nutrient = (n.nutrient - spread * 0.2).max(0.0);
                    }

                    // 当前体素产生孢子
                    let v = &mut new_voxels[idx];
                    v.spore += v.infected * dt * 0.01;
                }
            }
        }

        // 密度足够高的体素被感染
        for x in 1..size-1 {
            for y in 1..size-1 {
                for z in 1..size-1 {
                    let idx = x + y * size + z * size * size;
                    if new_voxels[idx].density > 0.3 && new_voxels[idx].infected < 1.0 {
                        new_voxels[idx].infected = (new_voxels[idx].infected + 0.2).min(1.0);
                    }
                }
            }
        }

        self.voxels = new_voxels;
        self.step_count += 1;
    }

    fn get_mut(&mut self, x: usize, y: usize, z: usize) -> Option<&mut MyceliumVoxel> {
        let size = self.size;
        if x < size && y < size && z < size {
            self.voxels.get_mut(x + y * size + z * size * size)
        } else {
            None
        }
    }

    pub fn infected_count(&self) -> usize {
        self.voxels.iter().filter(|v| v.infected > 0.5).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_creation() {
        let grid = MyceliumGrid::new(16);
        assert_eq!(grid.size, 16);
        assert_eq!(grid.voxels.len(), 16 * 16 * 16);
    }

    #[test]
    fn test_infection_spread() {
        let mut grid = MyceliumGrid::new(8);
        grid.seed_center(4, 4, 4);
        assert_eq!(grid.infected_count(), 1);

        for _ in 0..100 {
            grid.step(0.1);
        }
        assert!(grid.infected_count() > 1);
    }
}
