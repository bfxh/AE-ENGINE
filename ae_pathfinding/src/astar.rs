use crate::navmesh::{NavMesh, NavPolyKey};
use hashbrown::HashMap;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Debug, Clone)]
pub struct PathNode {
    pub position: [f32; 3],
    pub poly: NavPolyKey,
    pub cost_from_start: f32,
    pub total_cost: f32,
    pub parent: Option<NavPolyKey>,
}

#[derive(Debug, Clone)]
pub struct AStarResult {
    pub path: Vec<[f32; 3]>,
    pub total_cost: f32,
    pub nodes_visited: usize,
    pub success: bool,
}

struct HeapEntry {
    poly: NavPolyKey,
    total_cost: f32,
}

impl Eq for HeapEntry {}
impl PartialEq for HeapEntry {
    fn eq(&self, other: &Self) -> bool {
        self.total_cost == other.total_cost
    }
}
impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other.total_cost.total_cmp(&self.total_cost)
    }
}

pub struct AStarPathfinder {
    pub max_iterations: usize,
    pub straight_line_weight: f32,
}

impl Default for AStarPathfinder {
    fn default() -> Self {
        Self { max_iterations: 10000, straight_line_weight: 1.2 }
    }
}

impl AStarPathfinder {
    pub fn find_path(&self, nav: &NavMesh, start: &[f32; 3], end: &[f32; 3]) -> AStarResult {
        let start_poly = match nav.find_nearest_poly(start) {
            Some(p) => p,
            None => {
                return AStarResult {
                    path: vec![*start, *end],
                    total_cost: 0.0,
                    nodes_visited: 0,
                    success: false,
                };
            },
        };
        let end_poly = match nav.find_nearest_poly(end) {
            Some(p) => p,
            None => {
                return AStarResult {
                    path: vec![*start, *end],
                    total_cost: 0.0,
                    nodes_visited: 0,
                    success: false,
                };
            },
        };

        if start_poly == end_poly {
            return AStarResult {
                path: vec![*start, *end],
                total_cost: Self::distance(start, end),
                nodes_visited: 1,
                success: true,
            };
        }

        let mut open = BinaryHeap::new();
        let mut g_score = HashMap::new();
        let mut came_from: HashMap<NavPolyKey, Option<NavPolyKey>> = HashMap::new();
        let mut iterations = 0;

        g_score.insert(start_poly, 0.0);
        open.push(HeapEntry { poly: start_poly, total_cost: Self::heuristic(start, end) });

        while let Some(entry) = open.pop() {
            iterations += 1;
            if iterations > self.max_iterations {
                break;
            }

            if entry.poly == end_poly {
                let path = self.reconstruct_path(start, end, &came_from, nav, start_poly, end_poly);
                return AStarResult {
                    path,
                    total_cost: *g_score.get(&end_poly).unwrap_or(&0.0),
                    nodes_visited: iterations,
                    success: true,
                };
            }

            let current_cost = *g_score.get(&entry.poly).unwrap_or(&f32::MAX);
            if entry.total_cost > current_cost * self.straight_line_weight {
                continue;
            }

            let poly = match nav.polys.get(entry.poly) {
                Some(p) => p,
                None => continue,
            };

            for &neighbor in &poly.neighbors {
                let nb_poly = match nav.polys.get(neighbor) {
                    Some(p) => p,
                    None => continue,
                };

                let edge_cost = Self::distance(&poly.center, &nb_poly.center) * nb_poly.area_cost;
                let tentative = current_cost + edge_cost;

                let existing = g_score.get(&neighbor).copied().unwrap_or(f32::MAX);
                if tentative < existing {
                    g_score.insert(neighbor, tentative);
                    came_from.insert(neighbor, Some(entry.poly));
                    let h = Self::heuristic(&nb_poly.center, end);
                    open.push(HeapEntry { poly: neighbor, total_cost: tentative + h });
                }
            }
        }

        AStarResult {
            path: vec![*start, *end],
            total_cost: 0.0,
            nodes_visited: iterations,
            success: false,
        }
    }

    fn heuristic(a: &[f32; 3], b: &[f32; 3]) -> f32 {
        let dx = a[0] - b[0];
        let dy = a[1] - b[1];
        let dz = a[2] - b[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    fn distance(a: &[f32; 3], b: &[f32; 3]) -> f32 {
        Self::heuristic(a, b)
    }

    fn reconstruct_path(
        &self,
        start: &[f32; 3],
        end: &[f32; 3],
        came_from: &HashMap<NavPolyKey, Option<NavPolyKey>>,
        nav: &NavMesh,
        start_poly: NavPolyKey,
        end_poly: NavPolyKey,
    ) -> Vec<[f32; 3]> {
        let mut path = vec![*end];
        let mut current = Some(end_poly);

        while let Some(poly_key) = current {
            if poly_key == start_poly {
                break;
            }
            if let Some(poly) = nav.polys.get(poly_key) {
                path.push(poly.center);
            }
            current = came_from.get(&poly_key).copied().flatten();
        }

        path.push(*start);
        path.reverse();
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_grid_nav() -> NavMesh {
        let mut nav = NavMesh::default();
        let heights = vec![0.0; 25];
        nav.generate_grid(4, 4, &heights);
        nav
    }

    #[test]
    fn test_direct_path() {
        let nav = make_grid_nav();
        let pf = AStarPathfinder::default();
        let result = pf.find_path(&nav, &[0.25, 0.0, 0.25], &[0.75, 0.0, 0.75]);
        assert!(!result.path.is_empty());
    }

    #[test]
    fn test_long_path() {
        let nav = make_grid_nav();
        let pf = AStarPathfinder::default();
        let result = pf.find_path(&nav, &[0.1, 0.0, 0.1], &[1.5, 0.0, 1.5]);
        assert!(result.total_cost >= 0.0);
    }

    #[test]
    fn test_same_point() {
        let nav = make_grid_nav();
        let pf = AStarPathfinder::default();
        let result = pf.find_path(&nav, &[1.0, 0.0, 1.0], &[1.0, 0.0, 1.0]);
        assert!(result.success);
        assert_eq!(result.total_cost, 0.0);
    }
}
