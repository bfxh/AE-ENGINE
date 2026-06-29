use glam::Vec2;
use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::noise::{PermutationTable, fbm_2d, worley_2d};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heightmap {
    pub width: usize,
    pub height: usize,
    pub data: Vec<f32>,
    pub min_height: f32,
    pub max_height: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TerrainType {
    DeepWater,
    ShallowWater,
    Sand,
    Grass,
    Forest,
    Rock,
    Snow,
}

impl Heightmap {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height, data: vec![0.0; width * height], min_height: 0.0, max_height: 0.0 }
    }

    pub fn get(&self, x: usize, y: usize) -> f32 {
        if x < self.width && y < self.height { self.data[y * self.width + x] } else { 0.0 }
    }

    pub fn set(&mut self, x: usize, y: usize, value: f32) {
        if x < self.width && y < self.height {
            self.data[y * self.width + x] = value;
        }
    }

    pub fn generate_fbm(
        &mut self,
        table: &PermutationTable,
        scale: f32,
        octaves: u32,
        lacunarity: f32,
        gain: f32,
    ) {
        let inv_w = 1.0 / self.width as f32;
        let inv_h = 1.0 / self.height as f32;
        for y in 0..self.height {
            for x in 0..self.width {
                let p = Vec2::new(x as f32 * inv_w * scale, y as f32 * inv_h * scale);
                self.data[y * self.width + x] = fbm_2d(table, p, octaves, lacunarity, gain);
            }
        }
        self.update_range();
    }

    pub fn generate_ridged(
        &mut self,
        table: &PermutationTable,
        scale: f32,
        octaves: u32,
        lacunarity: f32,
        gain: f32,
    ) {
        let inv_w = 1.0 / self.width as f32;
        let inv_h = 1.0 / self.height as f32;
        for y in 0..self.height {
            for x in 0..self.width {
                let mut value = 0.0;
                let mut amplitude = 1.0;
                let mut frequency = 1.0;
                let mut weight = 1.0;
                let mut max_value = 0.0;

                for _ in 0..octaves {
                    let p = Vec2::new(
                        x as f32 * inv_w * scale * frequency,
                        y as f32 * inv_h * scale * frequency,
                    );
                    let n = fbm_2d(table, p, 1, 1.0, 1.0);
                    let n = 1.0 - n.abs();
                    value += n * n * weight;
                    max_value += amplitude;
                    weight = n * gain;
                    amplitude *= gain;
                    frequency *= lacunarity;
                }

                self.data[y * self.width + x] = value / max_value;
            }
        }
        self.update_range();
    }

    pub fn combine_worley(&mut self, table: &PermutationTable, cell_count: u32, weight: f32) {
        let inv_w = 1.0 / self.width as f32;
        let inv_h = 1.0 / self.height as f32;
        for y in 0..self.height {
            for x in 0..self.width {
                let p = Vec2::new(x as f32 * inv_w, y as f32 * inv_h);
                let w = worley_2d(table, p, cell_count);
                self.data[y * self.width + x] =
                    self.data[y * self.width + x] * (1.0 - weight) + w * weight;
            }
        }
        self.update_range();
    }

    pub fn normalize(&mut self) {
        self.update_range();
        let range = self.max_height - self.min_height;
        if range > f32::EPSILON {
            let inv_range = 1.0 / range;
            for v in &mut self.data {
                *v = (*v - self.min_height) * inv_range;
            }
            self.min_height = 0.0;
            self.max_height = 1.0;
        }
    }

    pub fn apply_terrace(&mut self, levels: u32) {
        let inv_levels = 1.0 / levels as f32;
        for v in &mut self.data {
            *v = ((*v * levels as f32).floor() + 0.5) * inv_levels;
        }
    }

    fn update_range(&mut self) {
        self.min_height = f32::MAX;
        self.max_height = f32::MIN;
        for &v in &self.data {
            self.min_height = self.min_height.min(v);
            self.max_height = self.max_height.max(v);
        }
    }

    pub fn sample(&self, uv: Vec2) -> f32 {
        let u = uv.x.clamp(0.0, 1.0);
        let v = uv.y.clamp(0.0, 1.0);
        let fx = u * (self.width - 1) as f32;
        let fy = v * (self.height - 1) as f32;
        let x0 = fx.floor() as usize;
        let y0 = fy.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;

        let v00 = self.get(x0, y0);
        let v10 = self.get(x1, y0);
        let v01 = self.get(x0, y1);
        let v11 = self.get(x1, y1);

        let v0 = v00 + (v10 - v00) * tx;
        let v1 = v01 + (v11 - v01) * tx;
        v0 + (v1 - v0) * ty
    }

    pub fn classify_terrain(&self, value: f32) -> TerrainType {
        match value {
            v if v < 0.1 => TerrainType::DeepWater,
            v if v < 0.2 => TerrainType::ShallowWater,
            v if v < 0.3 => TerrainType::Sand,
            v if v < 0.5 => TerrainType::Grass,
            v if v < 0.7 => TerrainType::Forest,
            v if v < 0.9 => TerrainType::Rock,
            _ => TerrainType::Snow,
        }
    }

    pub fn to_3d(&self, scale: Vec3) -> Vec<Vec3> {
        let mut vertices = Vec::with_capacity((self.width + 1) * (self.height + 1));
        for y in 0..=self.height {
            for x in 0..=self.width {
                let cx = self.get(x.min(self.width - 1), y.min(self.height - 1));
                vertices.push(Vec3::new(x as f32 * scale.x, cx * scale.y, y as f32 * scale.z));
            }
        }
        vertices
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heightmap_new() {
        let hm = Heightmap::new(64, 64);
        assert_eq!(hm.width, 64);
        assert_eq!(hm.height, 64);
        assert_eq!(hm.data.len(), 64 * 64);
    }

    #[test]
    fn test_generate_fbm() {
        let table = PermutationTable::new(42);
        let mut hm = Heightmap::new(32, 32);
        hm.generate_fbm(&table, 4.0, 4, 2.0, 0.5);
        assert!(hm.max_height > hm.min_height);
    }

    #[test]
    fn test_normalize() {
        let table = PermutationTable::new(7);
        let mut hm = Heightmap::new(32, 32);
        hm.generate_fbm(&table, 4.0, 4, 2.0, 0.5);
        hm.normalize();
        assert!(hm.min_height >= 0.0);
        assert!(hm.max_height <= 1.0);
    }

    #[test]
    fn test_bilinear_sample() {
        let table = PermutationTable::new(1);
        let mut hm = Heightmap::new(16, 16);
        hm.generate_fbm(&table, 2.0, 2, 2.0, 0.5);
        let s = hm.sample(Vec2::new(0.5, 0.5));
        assert!((-1.0..=1.0).contains(&s));
    }

    #[test]
    fn test_classify_terrain() {
        let hm = Heightmap::new(1, 1);
        assert!(matches!(hm.classify_terrain(0.05), TerrainType::DeepWater));
        assert!(matches!(hm.classify_terrain(0.15), TerrainType::ShallowWater));
        assert!(matches!(hm.classify_terrain(0.25), TerrainType::Sand));
        assert!(matches!(hm.classify_terrain(0.4), TerrainType::Grass));
        assert!(matches!(hm.classify_terrain(0.6), TerrainType::Forest));
        assert!(matches!(hm.classify_terrain(0.8), TerrainType::Rock));
        assert!(matches!(hm.classify_terrain(0.95), TerrainType::Snow));
    }
}
