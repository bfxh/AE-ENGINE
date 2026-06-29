use crate::navmesh::NavMesh;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlowCell {
    pub direction: [f32; 2],
    pub cost: f32,
    pub integrated: bool,
}

#[derive(Debug, Clone)]
pub struct FlowField {
    pub width: usize,
    pub depth: usize,
    pub cells: Vec<FlowCell>,
    pub cell_size: f32,
    pub origin: [f32; 2],
}

impl FlowField {
    pub fn new(width: usize, depth: usize, cell_size: f32, origin: [f32; 2]) -> Self {
        let cells =
            vec![FlowCell { direction: [0.0, 0.0], cost: 1.0, integrated: false }; width * depth];
        Self { width, depth, cells, cell_size, origin }
    }

    fn cell_index(&self, x: usize, z: usize) -> usize {
        z * self.width + x
    }

    pub fn world_to_cell(&self, world_pos: &[f32; 3]) -> (usize, usize) {
        let x = ((world_pos[0] - self.origin[0]) / self.cell_size) as usize;
        let z = ((world_pos[2] - self.origin[1]) / self.cell_size) as usize;
        (x.min(self.width - 1), z.min(self.depth - 1))
    }

    pub fn cell_to_world(&self, x: usize, z: usize) -> [f32; 3] {
        [
            self.origin[0] + x as f32 * self.cell_size + self.cell_size * 0.5,
            0.0,
            self.origin[1] + z as f32 * self.cell_size + self.cell_size * 0.5,
        ]
    }

    pub fn set_cost(&mut self, x: usize, z: usize, cost: f32) {
        if x < self.width && z < self.depth {
            let idx = self.cell_index(x, z);
            self.cells[idx].cost = cost.max(0.01);
        }
    }

    pub fn set_blocked(&mut self, x: usize, z: usize) {
        if x < self.width && z < self.depth {
            let idx = self.cell_index(x, z);
            self.cells[idx].cost = f32::MAX;
        }
    }

    pub fn is_blocked(&self, x: usize, z: usize) -> bool {
        if x >= self.width || z >= self.depth {
            return true;
        }
        self.cells[self.cell_index(x, z)].cost >= f32::MAX * 0.5
    }

    pub fn build(&mut self, target_x: usize, target_z: usize) {
        for cell in &mut self.cells {
            cell.integrated = false;
        }

        let mut wavefront = vec![(target_x, target_z)];
        let idx = self.cell_index(target_x, target_z);
        self.cells[idx].integrated = true;
        self.cells[idx].cost = 0.0;

        let neighbors = [(0, -1), (1, 0), (0, 1), (-1, 0), (-1, -1), (1, -1), (-1, 1), (1, 1)];

        while !wavefront.is_empty() {
            let mut next_wave = Vec::new();
            for (cx, cz) in &wavefront {
                let current_idx = self.cell_index(*cx, *cz);
                let current_cost = self.cells[current_idx].cost;

                for (dx, dz) in &neighbors {
                    let nx = *cx as isize + *dx;
                    let nz = *cz as isize + *dz;
                    if nx < 0 || nz < 0 || nx >= self.width as isize || nz >= self.depth as isize {
                        continue;
                    }
                    let (nx, nz) = (nx as usize, nz as usize);
                    if self.is_blocked(nx, nz) {
                        continue;
                    }

                    let nidx = self.cell_index(nx, nz);
                    let step_cost = if *dx != 0 && *dz != 0 { 1.414 } else { 1.0 };
                    let new_cost = current_cost + step_cost * self.cells[nidx].cost;

                    if !self.cells[nidx].integrated || new_cost < self.cells[nidx].cost {
                        self.cells[nidx].cost = new_cost;
                        self.cells[nidx].integrated = true;
                        next_wave.push((nx, nz));
                    }
                }
            }
            wavefront = next_wave;
        }

        for x in 0..self.width {
            for z in 0..self.depth {
                let idx = self.cell_index(x, z);
                if self.is_blocked(x, z) {
                    self.cells[idx].direction = [0.0, 0.0];
                    continue;
                }

                let mut best_dir = [0.0, 0.0];
                let mut best_cost = f32::MAX;

                for (dx, dz) in &neighbors {
                    let nx = x as isize + *dx;
                    let nz = z as isize + *dz;
                    if nx < 0 || nz < 0 || nx >= self.width as isize || nz >= self.depth as isize {
                        continue;
                    }
                    let (nx, nz) = (nx as usize, nz as usize);
                    if self.is_blocked(nx, nz) {
                        continue;
                    }

                    let nidx = self.cell_index(nx, nz);
                    if self.cells[nidx].integrated && self.cells[nidx].cost < best_cost {
                        best_cost = self.cells[nidx].cost;
                        best_dir = [*dx as f32, *dz as f32];
                    }
                }

                if best_dir[0] != 0.0 || best_dir[1] != 0.0 {
                    let len = (best_dir[0] * best_dir[0] + best_dir[1] * best_dir[1]).sqrt();
                    self.cells[idx].direction = [best_dir[0] / len, best_dir[1] / len];
                }
            }
        }
    }

    pub fn get_direction(&self, world_pos: &[f32; 3]) -> [f32; 2] {
        let (x, z) = self.world_to_cell(world_pos);
        self.cells[self.cell_index(x, z)].direction
    }

    pub fn build_from_navmesh(&mut self, nav: &NavMesh, target_x: usize, target_z: usize) {
        for z in 0..self.depth {
            for x in 0..self.width {
                let world_pos = self.cell_to_world(x, z);
                let nearest = nav.find_nearest_poly(&world_pos);
                if nearest.is_none() {
                    self.set_blocked(x, z);
                }
            }
        }
        self.build(target_x, target_z);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow_field_create() {
        let ff = FlowField::new(5, 5, 1.0, [0.0, 0.0]);
        assert_eq!(ff.width, 5);
        assert_eq!(ff.depth, 5);
        assert_eq!(ff.cells.len(), 25);
    }

    #[test]
    fn test_flow_field_direction() {
        let mut ff = FlowField::new(5, 5, 1.0, [0.0, 0.0]);
        ff.build(2, 2);
        let dir = ff.get_direction(&[0.5, 0.0, 0.5]);
        assert!(dir[0] != 0.0 || dir[1] != 0.0);
    }

    #[test]
    fn test_world_to_cell() {
        let ff = FlowField::new(10, 10, 2.0, [0.0, 0.0]);
        let (x, z) = ff.world_to_cell(&[5.0, 0.0, 5.0]);
        assert_eq!(x, 2);
        assert_eq!(z, 2);
    }

    #[test]
    fn test_blocked_cells() {
        let mut ff = FlowField::new(5, 5, 1.0, [0.0, 0.0]);
        ff.set_blocked(1, 1);
        ff.set_blocked(1, 2);
        ff.set_blocked(1, 3);
        ff.build(4, 2);
        assert!(ff.is_blocked(1, 1));
        let dir = ff.get_direction(&[0.5, 0.0, 2.5]);
        assert!(dir[0] != 0.0 || dir[1] != 0.0);
    }
}
