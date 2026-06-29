use glam::Vec3;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_center_half(center: Vec3, half_extents: Vec3) -> Self {
        Self { min: center - half_extents, max: center + half_extents }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    pub fn expand(&self, margin: f32) -> Self {
        let m = Vec3::splat(margin);
        Self { min: self.min - m, max: self.max + m }
    }

    pub fn surface_area(&self) -> f32 {
        let d = self.max - self.min;
        2.0 * (d.x * d.y + d.y * d.z + d.z * d.x)
    }
}

#[derive(Debug, Clone)]
pub struct BroadPhaseEntry {
    pub id: Uuid,
    pub aabb: Aabb,
    pub layer: u32,
    pub mask: u32,
    pub is_static: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CellKey(i32, i32, i32);

pub struct SpatialHashGrid {
    #[allow(dead_code)]
    cell_size: f32,
    inv_cell_size: f32,
    cells: HashMap<CellKey, Vec<BroadPhaseEntry>>,
    entries: HashMap<Uuid, BroadPhaseEntry>,
    pairs: Vec<(Uuid, Uuid)>,
    margin: f32,
}

impl SpatialHashGrid {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size: cell_size.max(0.01),
            inv_cell_size: 1.0 / cell_size.max(0.01),
            cells: HashMap::new(),
            entries: HashMap::new(),
            pairs: Vec::new(),
            margin: 0.1,
        }
    }

    pub fn with_margin(cell_size: f32, margin: f32) -> Self {
        Self {
            cell_size: cell_size.max(0.01),
            inv_cell_size: 1.0 / cell_size.max(0.01),
            cells: HashMap::new(),
            entries: HashMap::new(),
            pairs: Vec::new(),
            margin,
        }
    }

    pub fn insert(&mut self, entry: BroadPhaseEntry) {
        let expanded = entry.aabb.expand(self.margin);
        let cells = self.aabb_to_cells(&expanded);
        for &cell in &cells {
            self.cells.entry(cell).or_default().push(entry.clone());
        }
        self.entries.insert(entry.id, entry);
    }

    pub fn remove(&mut self, id: Uuid) {
        if let Some(entry) = self.entries.remove(&id) {
            let expanded = entry.aabb.expand(self.margin);
            let cells = self.aabb_to_cells(&expanded);
            for cell in &cells {
                if let Some(list) = self.cells.get_mut(cell) {
                    list.retain(|e| e.id != id);
                }
            }
        }
    }

    pub fn update(&mut self, id: Uuid, new_aabb: Aabb) {
        self.remove(id);
        if let Some(mut entry) = self.entries.get(&id).cloned() {
            entry.aabb = new_aabb;
            self.insert(entry);
        }
    }

    pub fn clear(&mut self) {
        self.cells.clear();
        self.entries.clear();
        self.pairs.clear();
    }

    pub fn find_pairs(&mut self) -> &[(Uuid, Uuid)] {
        self.pairs.clear();
        let mut seen: HashMap<(Uuid, Uuid), bool> = HashMap::new();

        for cell_entries in self.cells.values() {
            let n = cell_entries.len();
            for i in 0..n {
                for j in (i + 1)..n {
                    let a = &cell_entries[i];
                    let b = &cell_entries[j];
                    if a.is_static && b.is_static {
                        continue;
                    }
                    if !Self::layer_match(a, b) {
                        continue;
                    }
                    let pair = if a.id < b.id { (a.id, b.id) } else { (b.id, a.id) };
                    if !seen.contains_key(&pair) && a.aabb.intersects(&b.aabb) {
                        seen.insert(pair, true);
                        self.pairs.push(pair);
                    }
                }
            }
        }
        &self.pairs
    }

    pub fn query_aabb(&self, aabb: &Aabb) -> Vec<Uuid> {
        let expanded = aabb.expand(self.margin);
        let cells = self.aabb_to_cells(&expanded);
        let mut result = Vec::new();
        let mut seen: HashMap<Uuid, bool> = HashMap::new();

        for cell in &cells {
            if let Some(entries) = self.cells.get(cell) {
                for entry in entries {
                    if !seen.contains_key(&entry.id) && entry.aabb.intersects(aabb) {
                        seen.insert(entry.id, true);
                        result.push(entry.id);
                    }
                }
            }
        }
        result
    }

    pub fn query_ray(&self, origin: Vec3, direction: Vec3, max_dist: f32) -> Vec<(Uuid, f32)> {
        let inv_dir = Vec3::new(1.0 / direction.x, 1.0 / direction.y, 1.0 / direction.z);
        let mut results = Vec::new();

        for entry in self.entries.values() {
            if let Some(t) = ray_aabb_intersect(origin, direction, inv_dir, &entry.aabb) {
                if t <= max_dist {
                    results.push((entry.id, t));
                }
            }
        }
        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    fn aabb_to_cells(&self, aabb: &Aabb) -> Vec<CellKey> {
        let min_x = (aabb.min.x * self.inv_cell_size).floor() as i32;
        let min_y = (aabb.min.y * self.inv_cell_size).floor() as i32;
        let min_z = (aabb.min.z * self.inv_cell_size).floor() as i32;
        let max_x = (aabb.max.x * self.inv_cell_size).ceil() as i32;
        let max_y = (aabb.max.y * self.inv_cell_size).ceil() as i32;
        let max_z = (aabb.max.z * self.inv_cell_size).ceil() as i32;

        let mut cells = Vec::new();
        for x in min_x..=max_x {
            for y in min_y..=max_y {
                for z in min_z..=max_z {
                    cells.push(CellKey(x, y, z));
                }
            }
        }
        cells
    }

    fn layer_match(a: &BroadPhaseEntry, b: &BroadPhaseEntry) -> bool {
        (a.layer & b.mask) != 0 && (b.layer & a.mask) != 0
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn pair_count(&self) -> usize {
        self.pairs.len()
    }
}

fn ray_aabb_intersect(origin: Vec3, _dir: Vec3, inv_dir: Vec3, aabb: &Aabb) -> Option<f32> {
    let t1 = (aabb.min.x - origin.x) * inv_dir.x;
    let t2 = (aabb.max.x - origin.x) * inv_dir.x;
    let t3 = (aabb.min.y - origin.y) * inv_dir.y;
    let t4 = (aabb.max.y - origin.y) * inv_dir.y;
    let t5 = (aabb.min.z - origin.z) * inv_dir.z;
    let t6 = (aabb.max.z - origin.z) * inv_dir.z;

    let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
    let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

    if tmax < 0.0 || tmin > tmax {
        return None;
    }
    Some(if tmin < 0.0 { tmax } else { tmin })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(id: u8, x: f32, y: f32, z: f32) -> BroadPhaseEntry {
        BroadPhaseEntry {
            id: Uuid::from_u128(id as u128),
            aabb: Aabb::from_center_half(Vec3::new(x, y, z), Vec3::splat(0.5)),
            layer: 0x0001,
            mask: 0xFFFF,
            is_static: false,
        }
    }

    #[test]
    fn test_aabb_intersection() {
        let a = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 2.0, 2.0));
        let b = Aabb::new(Vec3::new(1.0, 1.0, 1.0), Vec3::new(3.0, 3.0, 3.0));
        assert!(a.intersects(&b));

        let c = Aabb::new(Vec3::new(5.0, 5.0, 5.0), Vec3::new(6.0, 6.0, 6.0));
        assert!(!a.intersects(&c));
    }

    #[test]
    fn test_spatial_hash_insert_find_pairs() {
        let mut grid = SpatialHashGrid::new(2.0);
        grid.insert(make_entry(1, 0.0, 0.0, 0.0));
        grid.insert(make_entry(2, 0.5, 0.5, 0.5));
        grid.insert(make_entry(3, 10.0, 10.0, 10.0));

        let pairs = grid.find_pairs();
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn test_spatial_hash_no_pairs() {
        let mut grid = SpatialHashGrid::new(2.0);
        grid.insert(make_entry(1, 0.0, 0.0, 0.0));
        grid.insert(make_entry(2, 10.0, 10.0, 10.0));
        grid.insert(make_entry(3, 20.0, 20.0, 20.0));

        let pairs = grid.find_pairs();
        assert_eq!(pairs.len(), 0);
    }

    #[test]
    fn test_query_aabb() {
        let mut grid = SpatialHashGrid::new(2.0);
        grid.insert(make_entry(1, 0.0, 0.0, 0.0));
        grid.insert(make_entry(2, 3.0, 3.0, 3.0));
        grid.insert(make_entry(3, 10.0, 10.0, 10.0));

        let query = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(5.0, 5.0, 5.0));
        let results = grid.query_aabb(&query);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_ray_query() {
        let mut grid = SpatialHashGrid::new(2.0);
        grid.insert(make_entry(1, 5.0, 0.0, 0.0));
        grid.insert(make_entry(2, 10.0, 0.0, 0.0));

        let results = grid.query_ray(Vec3::ZERO, Vec3::X, 20.0);
        assert_eq!(results.len(), 2);
        assert!(results[0].1 < results[1].1);
    }

    #[test]
    fn test_remove() {
        let mut grid = SpatialHashGrid::new(2.0);
        let id = Uuid::from_u128(1);
        grid.insert(make_entry(1, 0.0, 0.0, 0.0));
        grid.insert(make_entry(2, 0.5, 0.5, 0.5));
        grid.remove(id);
        assert_eq!(grid.entry_count(), 1);
        let pairs = grid.find_pairs();
        assert_eq!(pairs.len(), 0);
    }

    #[test]
    fn test_static_skip() {
        let mut grid = SpatialHashGrid::new(2.0);
        let mut a = make_entry(1, 0.0, 0.0, 0.0);
        a.is_static = true;
        let mut b = make_entry(2, 0.5, 0.5, 0.5);
        b.is_static = true;
        grid.insert(a);
        grid.insert(b);
        let pairs = grid.find_pairs();
        assert_eq!(pairs.len(), 0);
    }
}
