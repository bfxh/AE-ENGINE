use glam::Vec2;
use glam::Vec3;
use rand::Rng as _;

#[derive(Clone)]
pub struct PermutationTable {
    perm: [u8; 512],
}

impl PermutationTable {
    pub fn new(seed: u64) -> Self {
        let mut rng: rand::rngs::StdRng = rand::SeedableRng::seed_from_u64(seed);
        let mut perm = [0u8; 512];
        let mut p: Vec<u8> = (0..256).map(|i| i as u8).collect();
        for i in (1..256).rev() {
            let j = rng.gen_range(0..=i);
            p.swap(i, j);
        }
        perm[..256].copy_from_slice(&p);
        perm[256..512].copy_from_slice(&p);
        Self { perm }
    }

    fn hash(&self, x: i32, y: i32, z: i32) -> u8 {
        let zi = (z & 255) as usize;
        let yi = (y & 255) as usize;
        let xi = (x & 255) as usize;
        let pz = self.perm[zi] as usize;
        let py = self.perm[yi + pz] as usize;
        self.perm[xi + py]
    }

    fn grad(&self, hash: u8, x: f32, y: f32, z: f32) -> f32 {
        let h = hash & 15;
        let u = if h < 8 { x } else { y };
        let v = if h < 4 { y } else { if h == 12 || h == 14 { x } else { z } };
        (if h & 1 == 0 { u } else { -u }) + (if h & 2 == 0 { v } else { -v })
    }
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

pub fn perlin_3d(table: &PermutationTable, p: Vec3) -> f32 {
    let xi = p.x.floor() as i32;
    let yi = p.y.floor() as i32;
    let zi = p.z.floor() as i32;
    let xf = p.x - xi as f32;
    let yf = p.y - yi as f32;
    let zf = p.z - zi as f32;

    let u = fade(xf);
    let v = fade(yf);
    let w = fade(zf);

    let aaa = table.hash(xi, yi, zi);
    let aba = table.hash(xi, yi + 1, zi);
    let aab = table.hash(xi, yi, zi + 1);
    let abb = table.hash(xi, yi + 1, zi + 1);
    let baa = table.hash(xi + 1, yi, zi);
    let bba = table.hash(xi + 1, yi + 1, zi);
    let bab = table.hash(xi + 1, yi, zi + 1);
    let bbb = table.hash(xi + 1, yi + 1, zi + 1);

    let x1 = lerp(table.grad(aaa, xf, yf, zf), table.grad(baa, xf - 1.0, yf, zf), u);
    let x2 = lerp(table.grad(aba, xf, yf - 1.0, zf), table.grad(bba, xf - 1.0, yf - 1.0, zf), u);
    let y1 = lerp(x1, x2, v);

    let x1 = lerp(table.grad(aab, xf, yf, zf - 1.0), table.grad(bab, xf - 1.0, yf, zf - 1.0), u);
    let x2 = lerp(
        table.grad(abb, xf, yf - 1.0, zf - 1.0),
        table.grad(bbb, xf - 1.0, yf - 1.0, zf - 1.0),
        u,
    );
    let y2 = lerp(x1, x2, v);

    lerp(y1, y2, w)
}

pub fn perlin_2d(table: &PermutationTable, p: Vec2) -> f32 {
    perlin_3d(table, Vec3::new(p.x, p.y, 0.0))
}

pub fn simplex_3d(table: &PermutationTable, p: Vec3) -> f32 {
    let _f = (3.0_f32).sqrt() - 1.0;
    let g = (3.0_f32 - (3.0_f32).sqrt()) / 6.0;

    let s = (p.x + p.y + p.z) * (1.0 / 3.0);
    let i = (p.x + s).floor() as i32;
    let j = (p.y + s).floor() as i32;
    let k = (p.z + s).floor() as i32;
    let t = (i + j + k) as f32 * g;
    let x0 = p.x - (i as f32 - t);
    let y0 = p.y - (j as f32 - t);
    let z0 = p.z - (k as f32 - t);

    let (i1, j1, k1, i2, j2, k2) = if x0 >= y0 {
        if y0 >= z0 {
            (1, 0, 0, 1, 1, 0)
        } else if x0 >= z0 {
            (1, 0, 0, 1, 0, 1)
        } else {
            (0, 0, 1, 1, 0, 1)
        }
    } else {
        if y0 < z0 {
            (0, 0, 1, 0, 1, 1)
        } else if x0 < z0 {
            (0, 1, 0, 0, 1, 1)
        } else {
            (0, 1, 0, 1, 1, 0)
        }
    };

    let x1 = x0 - i1 as f32 + g;
    let y1 = y0 - j1 as f32 + g;
    let z1 = z0 - k1 as f32 + g;
    let x2 = x0 - i2 as f32 + 2.0 * g;
    let y2 = y0 - j2 as f32 + 2.0 * g;
    let z2 = z0 - k2 as f32 + 2.0 * g;
    let x3 = x0 - 1.0 + 3.0 * g;
    let y3 = y0 - 1.0 + 3.0 * g;
    let z3 = z0 - 1.0 + 3.0 * g;

    let n0 = {
        let t = 0.6 - x0 * x0 - y0 * y0 - z0 * z0;
        if t < 0.0 {
            0.0
        } else {
            let t4 = t * t;
            t4 * t4 * table.grad(table.hash(i, j, k), x0, y0, z0)
        }
    };
    let n1 = {
        let t = 0.6 - x1 * x1 - y1 * y1 - z1 * z1;
        if t < 0.0 {
            0.0
        } else {
            let t4 = t * t;
            t4 * t4 * table.grad(table.hash(i + i1, j + j1, k + k1), x1, y1, z1)
        }
    };
    let n2 = {
        let t = 0.6 - x2 * x2 - y2 * y2 - z2 * z2;
        if t < 0.0 {
            0.0
        } else {
            let t4 = t * t;
            t4 * t4 * table.grad(table.hash(i + i2, j + j2, k + k2), x2, y2, z2)
        }
    };
    let n3 = {
        let t = 0.6 - x3 * x3 - y3 * y3 - z3 * z3;
        if t < 0.0 {
            0.0
        } else {
            let t4 = t * t;
            t4 * t4 * table.grad(table.hash(i + 1, j + 1, k + 1), x3, y3, z3)
        }
    };

    32.0 * (n0 + n1 + n2 + n3)
}

pub fn simplex_2d(table: &PermutationTable, p: Vec2) -> f32 {
    simplex_3d(table, Vec3::new(p.x, p.y, 0.0))
}

pub fn fbm_3d(table: &PermutationTable, p: Vec3, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut max_value = 0.0;
    let pos = p;

    for _ in 0..octaves {
        value += amplitude * perlin_3d(table, pos * frequency);
        max_value += amplitude;
        amplitude *= gain;
        frequency *= lacunarity;
    }

    value / max_value
}

pub fn fbm_2d(table: &PermutationTable, p: Vec2, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    fbm_3d(table, Vec3::new(p.x, p.y, 0.0), octaves, lacunarity, gain)
}

pub fn worley_3d(table: &PermutationTable, p: Vec3, cell_count: u32) -> f32 {
    let cell_size = cell_count as f32;
    let cx = (p.x * cell_size).floor() as i32;
    let cy = (p.y * cell_size).floor() as i32;
    let cz = (p.z * cell_size).floor() as i32;
    let fx = p.x * cell_size - cx as f32;
    let fy = p.y * cell_size - cy as f32;
    let fz = p.z * cell_size - cz as f32;

    let mut min_dist = f32::MAX;

    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let nx = cx + dx;
                let ny = cy + dy;
                let nz = cz + dz;
                let h = table.hash(nx, ny, nz) as f32 / 255.0;
                let px = dx as f32 + h * 0.5 - fx;
                let py = dy as f32 + (h * 0.7).fract() - fy;
                let pz = dz as f32 + (h * 0.3).fract() - fz;
                let dist = (px * px + py * py + pz * pz).sqrt();
                min_dist = min_dist.min(dist);
            }
        }
    }

    min_dist
}

pub fn worley_2d(table: &PermutationTable, p: Vec2, cell_count: u32) -> f32 {
    worley_3d(table, Vec3::new(p.x, p.y, 0.0), cell_count)
}

pub fn domain_warp_3d(table: &PermutationTable, p: Vec3, warp_strength: f32) -> Vec3 {
    let nx = perlin_3d(table, p);
    let ny = perlin_3d(table, p + Vec3::new(5.2, 1.3, 0.0));
    let nz = perlin_3d(table, p + Vec3::new(1.7, 9.2, 0.0));
    Vec3::new(p.x + nx * warp_strength, p.y + ny * warp_strength, p.z + nz * warp_strength)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permutation_table_deterministic() {
        let t1 = PermutationTable::new(42);
        let t2 = PermutationTable::new(42);
        let p = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(perlin_3d(&t1, p), perlin_3d(&t2, p));
    }

    #[test]
    fn test_perlin_range() {
        let table = PermutationTable::new(123);
        for x in 0..20 {
            for y in 0..20 {
                let v = perlin_2d(&table, Vec2::new(x as f32 * 0.1, y as f32 * 0.1));
                assert!((-1.0..=1.0).contains(&v), "value {} out of range", v);
            }
        }
    }

    #[test]
    fn test_fbm_octaves() {
        let table = PermutationTable::new(7);
        let p = Vec3::new(0.5, 0.5, 0.0);
        let v1 = fbm_3d(&table, p, 1, 2.0, 0.5);
        let v4 = fbm_3d(&table, p, 4, 2.0, 0.5);
        assert!(v1 != v4, "different octaves should produce different values");
    }

    #[test]
    fn test_worley_positive() {
        let table = PermutationTable::new(99);
        let v = worley_2d(&table, Vec2::new(0.5, 0.5), 4);
        assert!((0.0..=2.0).contains(&v));
    }

    #[test]
    fn test_domain_warp() {
        let table = PermutationTable::new(55);
        let p = Vec3::new(1.0, 2.0, 3.0);
        let warped = domain_warp_3d(&table, p, 0.5);
        assert!(warped.x != p.x || warped.y != p.y);
    }
}
