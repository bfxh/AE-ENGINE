use serde::{Deserialize, Serialize};
use slotmap::{SlotMap, new_key_type};

new_key_type! {
    pub struct NavNodeKey;
    pub struct NavPolyKey;
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NavNode {
    pub position: [f32; 3],
    pub radius: f32,
    pub flags: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavPoly {
    pub vertices: [usize; 3],
    pub center: [f32; 3],
    pub neighbors: Vec<NavPolyKey>,
    pub area_cost: f32,
    pub flags: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavMesh {
    pub nodes: SlotMap<NavNodeKey, NavNode>,
    pub polys: SlotMap<NavPolyKey, NavPoly>,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub cell_size: f32,
    pub cell_height: f32,
    pub max_slope: f32,
    pub max_climb: f32,
}

impl Default for NavMesh {
    fn default() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            polys: SlotMap::with_key(),
            bounds_min: [0.0; 3],
            bounds_max: [100.0; 3],
            cell_size: 0.5,
            cell_height: 0.2,
            max_slope: 45.0_f32.to_radians(),
            max_climb: 0.5,
        }
    }
}

impl NavMesh {
    pub fn add_node(&mut self, pos: [f32; 3], radius: f32) -> NavNodeKey {
        self.nodes.insert(NavNode { position: pos, radius, flags: 0 })
    }

    pub fn add_poly(&mut self, v0: NavNodeKey, v1: NavNodeKey, v2: NavNodeKey) -> NavPolyKey {
        let n0 = &self.nodes[v0];
        let n1 = &self.nodes[v1];
        let n2 = &self.nodes[v2];
        let center = [
            (n0.position[0] + n1.position[0] + n2.position[0]) / 3.0,
            (n0.position[1] + n1.position[1] + n2.position[1]) / 3.0,
            (n0.position[2] + n1.position[2] + n2.position[2]) / 3.0,
        ];

        let v0_idx = self.nodes.keys().position(|k| k == v0).unwrap_or(0);
        let v1_idx = self.nodes.keys().position(|k| k == v1).unwrap_or(0);
        let v2_idx = self.nodes.keys().position(|k| k == v2).unwrap_or(0);

        self.polys.insert(NavPoly {
            vertices: [v0_idx, v1_idx, v2_idx],
            center,
            neighbors: Vec::new(),
            area_cost: 1.0,
            flags: 0,
        })
    }

    pub fn find_nearest_poly(&self, point: &[f32; 3]) -> Option<NavPolyKey> {
        let mut best_key = None;
        let mut best_dist = f32::MAX;
        for (key, poly) in &self.polys {
            let dx = poly.center[0] - point[0];
            let dy = poly.center[1] - point[1];
            let dz = poly.center[2] - point[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            if dist < best_dist {
                best_dist = dist;
                best_key = Some(key);
            }
        }
        best_key
    }

    pub fn find_nearest_node(&self, point: &[f32; 3]) -> Option<NavNodeKey> {
        let mut best_key = None;
        let mut best_dist = f32::MAX;
        for (key, node) in &self.nodes {
            let dx = node.position[0] - point[0];
            let dy = node.position[1] - point[1];
            let dz = node.position[2] - point[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            if dist < best_dist {
                best_dist = dist;
                best_key = Some(key);
            }
        }
        best_key
    }

    pub fn build_connectivity(&mut self) {
        let poly_keys: Vec<NavPolyKey> = self.polys.keys().collect();
        let n = poly_keys.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let shared = self.shared_vertices(poly_keys[i], poly_keys[j]);
                if shared >= 2 {
                    let mut neighbors_i = self.polys[poly_keys[i]].neighbors.clone();
                    neighbors_i.push(poly_keys[j]);
                    self.polys[poly_keys[i]].neighbors = neighbors_i;

                    let mut neighbors_j = self.polys[poly_keys[j]].neighbors.clone();
                    neighbors_j.push(poly_keys[i]);
                    self.polys[poly_keys[j]].neighbors = neighbors_j;
                }
            }
        }
    }

    fn shared_vertices(&self, a: NavPolyKey, b: NavPolyKey) -> usize {
        let pa = &self.polys[a];
        let pb = &self.polys[b];
        let mut count = 0;
        for va in &pa.vertices {
            for vb in &pb.vertices {
                if va == vb {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn generate_grid(&mut self, width: usize, depth: usize, heights: &[f32]) {
        for z in 0..=depth {
            for x in 0..=width {
                let idx = z * (width + 1) + x;
                let h = heights.get(idx).copied().unwrap_or(0.0);
                self.add_node([x as f32 * self.cell_size, h, z as f32 * self.cell_size], 0.3);
            }
        }

        let node_keys: Vec<NavNodeKey> = self.nodes.keys().collect();
        for z in 0..depth {
            for x in 0..width {
                let tl = node_keys[z * (width + 1) + x];
                let tr = node_keys[z * (width + 1) + x + 1];
                let bl = node_keys[(z + 1) * (width + 1) + x];
                let br = node_keys[(z + 1) * (width + 1) + x + 1];
                self.add_poly(tl, tr, bl);
                self.add_poly(tr, br, bl);
            }
        }

        self.build_connectivity();
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn poly_count(&self) -> usize {
        self.polys.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_node() {
        let mut nav = NavMesh::default();
        let key = nav.add_node([0.0, 0.0, 0.0], 0.5);
        let node = &nav.nodes[key];
        assert_eq!(node.position, [0.0, 0.0, 0.0]);
        assert_eq!(node.radius, 0.5);
    }

    #[test]
    fn test_grid_generation() {
        let mut nav = NavMesh::default();
        let heights = vec![0.0; 25];
        nav.generate_grid(4, 4, &heights);
        assert_eq!(nav.node_count(), 25);
        assert_eq!(nav.poly_count(), 32);
    }

    #[test]
    fn test_nearest_node() {
        let mut nav = NavMesh::default();
        let heights = vec![0.0; 25];
        nav.generate_grid(4, 4, &heights);
        let nearest = nav.find_nearest_node(&[0.6, 0.0, 0.6]);
        assert!(nearest.is_some());
    }

    #[test]
    fn test_nearest_poly() {
        let mut nav = NavMesh::default();
        let heights = vec![0.0; 25];
        nav.generate_grid(4, 4, &heights);
        let nearest = nav.find_nearest_poly(&[1.0, 0.0, 1.0]);
        assert!(nearest.is_some());
    }
}
