use glam::Vec2;
use rand::Rng as _;
use serde::{Deserialize, Serialize};

use crate::heightmap::Heightmap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErosionConfig {
    pub iterations: u32,
    pub erosion_rate: f32,
    pub deposition_rate: f32,
    pub evaporation_rate: f32,
    pub min_slope: f32,
    pub gravity: f32,
    pub capacity_factor: f32,
}

impl Default for ErosionConfig {
    fn default() -> Self {
        Self {
            iterations: 50000,
            erosion_rate: 0.3,
            deposition_rate: 0.3,
            evaporation_rate: 0.01,
            min_slope: 0.01,
            gravity: 4.0,
            capacity_factor: 4.0,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Drop {
    x: f32,
    y: f32,
    sediment: f32,
    water: f32,
    speed: Vec2,
    volume: f32,
}

impl Drop {
    fn new(x: f32, y: f32) -> Self {
        Self { x, y, sediment: 0.0, water: 1.0, speed: Vec2::ZERO, volume: 1.0 }
    }
}

pub fn hydraulic_erosion(heightmap: &mut Heightmap, config: &ErosionConfig, seed: u64) {
    let mut rng: rand::rngs::StdRng = rand::SeedableRng::seed_from_u64(seed);
    let w = heightmap.width as f32;
    let h = heightmap.height as f32;
    let inv_w = 1.0 / (w - 1.0);
    let inv_h = 1.0 / (h - 1.0);

    for _ in 0..config.iterations {
        let mut drop = Drop::new(rng.gen_range(0.0..w), rng.gen_range(0.0..h));

        for _ in 0..100 {
            let ix = drop.x as usize;
            let iy = drop.y as usize;
            if ix >= heightmap.width - 1 || iy >= heightmap.height - 1 || ix == 0 || iy == 0 {
                break;
            }

            let fx = drop.x - ix as f32;
            let fy = drop.y - iy as f32;

            let h00 = heightmap.get(ix, iy);
            let h10 = heightmap.get(ix + 1, iy);
            let h01 = heightmap.get(ix, iy + 1);
            let h11 = heightmap.get(ix + 1, iy + 1);

            let gradient_x = (h10 - h00) * (1.0 - fy) + (h11 - h01) * fy;
            let gradient_y = (h01 - h00) * (1.0 - fx) + (h11 - h10) * fx;

            drop.speed = Vec2::new(
                drop.speed.x * 0.95 + gradient_x * config.gravity * inv_w,
                drop.speed.y * 0.95 + gradient_y * config.gravity * inv_h,
            );

            let speed = drop.speed.length();
            if speed < config.min_slope {
                break;
            }

            drop.x += drop.speed.x;
            drop.y += drop.speed.y;

            if drop.x < 0.0 || drop.x >= w || drop.y < 0.0 || drop.y >= h {
                break;
            }

            let new_h = heightmap.get(drop.x as usize, drop.y as usize);
            let old_h = h00 * (1.0 - fx) * (1.0 - fy)
                + h10 * fx * (1.0 - fy)
                + h01 * (1.0 - fx) * fy
                + h11 * fx * fy;
            let delta_h = old_h - new_h;

            let capacity = delta_h.max(0.0) * speed * drop.water * config.capacity_factor;

            if drop.sediment > capacity {
                let deposit = (drop.sediment - capacity) * config.deposition_rate;
                drop.sediment -= deposit;
                let idx = (drop.y as usize).min(heightmap.height - 1) * heightmap.width
                    + (drop.x as usize).min(heightmap.width - 1);
                heightmap.data[idx] += deposit;
            } else {
                let erode = ((capacity - drop.sediment) * config.erosion_rate).min(-delta_h);
                drop.sediment += erode;
                let idx = (drop.y as usize).min(heightmap.height - 1) * heightmap.width
                    + (drop.x as usize).min(heightmap.width - 1);
                heightmap.data[idx] -= erode;
            }

            drop.water *= 1.0 - config.evaporation_rate;
            if drop.water < 0.01 {
                break;
            }
        }
    }

    heightmap.normalize();
}

pub fn thermal_erosion(heightmap: &mut Heightmap, iterations: u32, talus_angle: f32) {
    let w = heightmap.width;
    let h = heightmap.height;

    for _ in 0..iterations {
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let center = heightmap.get(x, y);
                let _total_diff = 0.0;
                let mut max_diff = 0.0;
                let mut max_dx: i32 = 0;
                let mut max_dy: i32 = 0;

                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = (x as i32 + dx) as usize;
                        let ny = (y as i32 + dy) as usize;
                        let neighbor = heightmap.get(nx, ny);
                        let diff = center - neighbor;
                        if diff > max_diff {
                            max_diff = diff;
                            max_dx = dx;
                            max_dy = dy;
                        }
                    }
                }

                if max_diff > talus_angle {
                    let move_amount = (max_diff - talus_angle) * 0.5;
                    heightmap.set(x, y, center - move_amount);
                    let nx = (x as i32 + max_dx) as usize;
                    let ny = (y as i32 + max_dy) as usize;
                    heightmap.set(nx, ny, heightmap.get(nx, ny) + move_amount);
                }
            }
        }
    }

    heightmap.normalize();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noise::PermutationTable;

    #[test]
    fn test_hydraulic_erosion_runs() {
        let table = PermutationTable::new(42);
        let mut hm = Heightmap::new(64, 64);
        hm.generate_fbm(&table, 4.0, 4, 2.0, 0.5);
        let config = ErosionConfig { iterations: 1000, ..Default::default() };
        hydraulic_erosion(&mut hm, &config, 123);
        assert!(hm.max_height <= 1.0);
        assert!(hm.min_height >= 0.0);
    }

    #[test]
    fn test_thermal_erosion_runs() {
        let table = PermutationTable::new(7);
        let mut hm = Heightmap::new(32, 32);
        hm.generate_fbm(&table, 4.0, 4, 2.0, 0.5);
        thermal_erosion(&mut hm, 50, 0.01);
        assert!(hm.max_height <= 1.0);
        assert!(hm.min_height >= 0.0);
    }

    #[test]
    fn test_erosion_many_iterations() {
        let table = PermutationTable::new(1);
        let mut hm = Heightmap::new(16, 16);
        hm.generate_fbm(&table, 2.0, 2, 2.0, 0.5);
        let config = ErosionConfig {
            iterations: 2000,
            erosion_rate: 0.5,
            deposition_rate: 0.5,
            ..Default::default()
        };
        hydraulic_erosion(&mut hm, &config, 456);
        assert!(hm.min_height >= 0.0);
        assert!(hm.max_height <= 1.0);
    }
}
