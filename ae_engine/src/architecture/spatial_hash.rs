use std::collections::HashMap;

#[derive(Debug)]
pub struct SpatialHashGrid {
    cell_size: f32,
    cells: HashMap<(i32, i32, i32), Vec<usize>>,
}

impl SpatialHashGrid {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }

    pub fn insert(&mut self, index: usize, pos: [f32; 3]) {
        let key = self.cell_key(pos);
        self.cells.entry(key).or_insert_with(Vec::new).push(index);
    }

    pub fn build(&mut self, positions: &[[f32; 3]]) {
        self.clear();
        for (i, pos) in positions.iter().enumerate() {
            self.insert(i, *pos);
        }
    }

    fn cell_key(&self, pos: [f32; 3]) -> (i32, i32, i32) {
        (
            (pos[0] / self.cell_size).floor() as i32,
            (pos[1] / self.cell_size).floor() as i32,
            (pos[2] / self.cell_size).floor() as i32,
        )
    }

    pub fn query_neighbors(&self, pos: [f32; 3]) -> Vec<usize> {
        let (cx, cy, cz) = self.cell_key(pos);
        let mut result = Vec::new();

        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if let Some(cell) = self.cells.get(&(cx + dx, cy + dy, cz + dz)) {
                        result.extend_from_slice(cell);
                    }
                }
            }
        }

        result
    }

    pub fn query_radius(&self, pos: [f32; 3], _radius: f32) -> Vec<usize> {
        self.query_neighbors(pos)
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }
}
