use crate::rocks::*;
use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErosionParams {
    pub water_erosion_rate: f32,
    pub wind_erosion_rate: f32,
    pub chemical_erosion_rate: f32,
    pub freeze_thaw_cycles: f32,
    pub temperature_range: f32,
}

impl Default for ErosionParams {
    fn default() -> Self {
        Self {
            water_erosion_rate: 1.0,
            wind_erosion_rate: 0.3,
            chemical_erosion_rate: 0.1,
            freeze_thaw_cycles: 0.0,
            temperature_range: 20.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErosionSolver {
    pub terrain_height: Vec<f32>,
    pub dimensions: (usize, usize),
    pub spacing: f32,
    pub sediment: Vec<f32>,
}

impl ErosionSolver {
    pub fn new(dimensions: (usize, usize), spacing: f32) -> Self {
        let (nx, ny) = dimensions;
        let total = nx * ny;
        let mut terrain_height = vec![0.0; total];
        for (i, h) in terrain_height.iter_mut().enumerate() {
            let x = (i % nx) as f32 * spacing;
            let y = (i / nx) as f32 * spacing;
            *h = (x.sin() * 0.3 + y.cos() * 0.3) * 50.0 + 100.0;
        }
        Self { terrain_height, dimensions, spacing, sediment: vec![0.0; total] }
    }

    fn index(&self, x: usize, y: usize) -> usize {
        let (nx, _) = self.dimensions;
        x + y * nx
    }

    pub fn step(&mut self, params: &ErosionParams, rock_type: RockType, dt: f32) {
        let (nx, ny) = self.dimensions;
        let resistance = rock_type.erosion_resistance();
        let mut new_height = self.terrain_height.clone();

        for y in 1..ny - 1 {
            for x in 1..nx - 1 {
                let idx = self.index(x, y);
                let h = self.terrain_height[idx];

                let h_xp = self.terrain_height[self.index(x + 1, y)];
                let h_xn = self.terrain_height[self.index(x - 1, y)];
                let h_yp = self.terrain_height[self.index(x, y + 1)];
                let h_yn = self.terrain_height[self.index(x, y - 1)];

                let gradient = Vec3::new(
                    (h_xp - h_xn) / (2.0 * self.spacing),
                    (h_yp - h_yn) / (2.0 * self.spacing),
                    0.0,
                );
                let slope = gradient.length();
                let water_power = slope * params.water_erosion_rate;
                let wind_power = params.wind_erosion_rate * (1.0 - (h / 200.0).min(1.0));
                let chemical_power = params.chemical_erosion_rate;
                let freeze_power = params.freeze_thaw_cycles * 0.01;

                let total_erosion =
                    (water_power + wind_power + chemical_power + freeze_power) * (1.0 - resistance);
                new_height[idx] -= total_erosion * dt;

                self.sediment[idx] += total_erosion * dt;
            }
        }

        for y in 1..ny - 1 {
            for x in 1..nx - 1 {
                let idx = self.index(x, y);
                if self.sediment[idx] > 0.01 {
                    let flow_dir = Vec3::new(
                        new_height[self.index(x - 1, y)] - new_height[self.index(x + 1, y)],
                        new_height[self.index(x, y - 1)] - new_height[self.index(x, y + 1)],
                        0.0,
                    )
                    .normalize_or_zero();

                    let tx = (flow_dir.x / self.spacing).round() as isize;
                    let ty = (flow_dir.y / self.spacing).round() as isize;
                    let tx = x as isize + tx;
                    let ty = y as isize + ty;

                    if tx >= 0 && tx < nx as isize && ty >= 0 && ty < ny as isize {
                        let target_idx = self.index(tx as usize, ty as usize);
                        let transfer = self.sediment[idx] * 0.5 * dt;
                        self.sediment[idx] -= transfer;
                        self.sediment[target_idx] += transfer;
                        new_height[target_idx] += transfer * 0.1;
                    }
                }
            }
        }

        self.terrain_height = new_height;
    }

    pub fn height_at(&self, x: usize, y: usize) -> f32 {
        let (nx, ny) = self.dimensions;
        if x >= nx || y >= ny {
            return 0.0;
        }
        self.terrain_height[self.index(x, y)]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoilLayer {
    pub fertility: f32,
    pub moisture: f32,
    pub ph: f32,
    pub organic_content: f32,
    pub thickness: f32,
    pub parent_rock: RockType,
}

impl SoilLayer {
    pub fn new(parent_rock: RockType) -> Self {
        Self {
            fertility: 0.3,
            moisture: 0.5,
            ph: 6.5,
            organic_content: 0.05,
            thickness: 0.3,
            parent_rock,
        }
    }

    pub fn erode_into_soil(&mut self, erosion_rate: f32, dt: f32) {
        self.thickness += erosion_rate * dt * 0.01;
        self.fertility = (self.fertility + erosion_rate * dt * 0.001).min(1.0);
    }
}
