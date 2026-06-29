//! Bounding Volume Hierarchy (BVH) for ray / AABB / frustum queries.
//!
//! Used for: ray casting (editor picking, weapon hitscan), area damage,
//! view-frustum culling. Static scenes benefit most (build once, query many);
//! dynamic scenes should use `refit` or rebuild on change.

use glam::{Mat4, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_center_half(center: Vec3, half: Vec3) -> Self {
        Self { min: center - half, max: center + half }
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

    pub fn surface_area(&self) -> f32 {
        let d = self.max - self.min;
        2.0 * (d.x * d.y + d.y * d.z + d.z * d.x)
    }

    pub fn union(&self, other: &Aabb) -> Aabb {
        Aabb {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    pub fn expand(&self, margin: f32) -> Aabb {
        let m = Vec3::splat(margin);
        Aabb { min: self.min - m, max: self.max + m }
    }

    /// Ray-AABB intersection (slab method). Returns hit distance t, or None.
    pub fn ray_intersect(&self, origin: Vec3, inv_dir: Vec3) -> Option<f32> {
        let t1 = (self.min.x - origin.x) * inv_dir.x;
        let t2 = (self.max.x - origin.x) * inv_dir.x;
        let t3 = (self.min.y - origin.y) * inv_dir.y;
        let t4 = (self.max.y - origin.y) * inv_dir.y;
        let t5 = (self.min.z - origin.z) * inv_dir.z;
        let t6 = (self.max.z - origin.z) * inv_dir.z;

        let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
        let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

        if tmax < 0.0 || tmin > tmax {
            return None;
        }
        Some(if tmin < 0.0 { tmax } else { tmin })
    }

    /// Test against a frustum defined by 6 inward-facing planes (normal . point + d >= 0 inside).
    pub fn in_frustum(&self, planes: &[Plane; 6]) -> bool {
        for plane in planes {
            let positive = Vec3::new(
                if plane.normal.x >= 0.0 { self.max.x } else { self.min.x },
                if plane.normal.y >= 0.0 { self.max.y } else { self.min.y },
                if plane.normal.z >= 0.0 { self.max.z } else { self.min.z },
            );
            if plane.normal.dot(positive) + plane.d < 0.0 {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Plane {
    pub normal: Vec3,
    pub d: f32,
}

impl Plane {
    pub fn new(normal: Vec3, d: f32) -> Self {
        Self { normal, d }
    }
}

/// Extract 6 inward-facing frustum planes from a view-projection matrix (column-major).
///
/// Uses the Gribb-Hartmann method. Each plane (normal, d) satisfies:
///   normal . point + d >= 0  for points inside the frustum.
/// Planes are normalized (unit normal) so that `d` gives the signed distance.
///
/// Order: [left, right, bottom, top, near, far]
pub fn frustum_from_view_proj(view_proj: Mat4) -> [Plane; 6] {
    let m = view_proj.to_cols_array_2d(); // m[col][row], column-major

    let row = |i: usize| -> [f32; 4] {
        [m[0][i], m[1][i], m[2][i], m[3][i]]
    };

    let r0 = row(0);
    let r1 = row(1);
    let r2 = row(2);
    let r3 = row(3);

    let make = |r: [f32; 4]| -> Plane {
        let normal = Vec3::new(r[0], r[1], r[2]);
        let len = normal.length();
        if len > 0.0 {
            Plane::new(normal / len, r[3] / len)
        } else {
            Plane::new(normal, r[3])
        }
    };

    [
        make([r3[0] + r0[0], r3[1] + r0[1], r3[2] + r0[2], r3[3] + r0[3]]), // left
        make([r3[0] - r0[0], r3[1] - r0[1], r3[2] - r0[2], r3[3] - r0[3]]), // right
        make([r3[0] + r1[0], r3[1] + r1[1], r3[2] + r1[2], r3[3] + r1[3]]), // bottom
        make([r3[0] - r1[0], r3[1] - r1[1], r3[2] - r1[2], r3[3] - r1[3]]), // top
        make([r3[0] + r2[0], r3[1] + r2[1], r3[2] + r2[2], r3[3] + r2[3]]), // near
        make([r3[0] - r2[0], r3[1] - r2[1], r3[2] - r2[2], r3[3] - r2[3]]), // far
    ]
}

#[derive(Debug, Clone)]
pub struct BvhNode {
    pub bounds: Aabb,
    pub left: u32,
    pub right: u32,
    pub start: u32,
    pub end: u32,
    pub leaf: bool,
}

impl BvhNode {
    fn internal(bounds: Aabb, left: u32, right: u32) -> Self {
        Self { bounds, left, right, start: 0, end: 0, leaf: false }
    }

    fn leaf(bounds: Aabb, start: u32, end: u32) -> Self {
        Self { bounds, left: 0, right: 0, start, end, leaf: true }
    }
}

#[derive(Debug, Clone)]
struct Primitive {
    aabb: Aabb,
    id: u32,
}

#[derive(Debug, Clone)]
pub struct Bvh {
    nodes: Vec<BvhNode>,
    primitives: Vec<Primitive>,
    root: u32,
}

impl Bvh {
    pub fn empty() -> Self {
        Self { nodes: Vec::new(), primitives: Vec::new(), root: u32::MAX }
    }

    pub fn build(items: &[(u32, Aabb)]) -> Self {
        if items.is_empty() {
            return Self::empty();
        }
        let mut primitives: Vec<Primitive> =
            items.iter().map(|(id, aabb)| Primitive { aabb: *aabb, id: *id }).collect();
        let total = primitives.len() as u32;
        let mut nodes: Vec<BvhNode> = Vec::with_capacity(primitives.len() * 2);
        let root = build_recursive(&mut primitives, 0, total, &mut nodes);
        Self { nodes, primitives, root }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn primitive_count(&self) -> usize {
        self.primitives.len()
    }

    /// Query all primitive ids whose AABB intersects `query_aabb`.
    pub fn aabb_query(&self, query_aabb: &Aabb) -> Vec<u32> {
        let mut hits = Vec::new();
        if self.root == u32::MAX {
            return hits;
        }
        let mut stack = vec![self.root];
        while let Some(idx) = stack.pop() {
            let node = &self.nodes[idx as usize];
            if !node.bounds.intersects(query_aabb) {
                continue;
            }
            if node.leaf {
                for p in &self.primitives[node.start as usize..node.end as usize] {
                    if p.aabb.intersects(query_aabb) {
                        hits.push(p.id);
                    }
                }
            } else {
                stack.push(node.left);
                stack.push(node.right);
            }
        }
        hits
    }

    /// Query all primitive ids hit by a ray (origin, dir) within max_t.
    /// Returns (id, t_hit) sorted by t ascending.
    pub fn ray_query(&self, origin: Vec3, dir: Vec3, max_t: f32) -> Vec<(u32, f32)> {
        let mut hits = Vec::new();
        if self.root == u32::MAX {
            return hits;
        }
        let inv_dir = Vec3::new(
            if dir.x != 0.0 { 1.0 / dir.x } else { f32::INFINITY },
            if dir.y != 0.0 { 1.0 / dir.y } else { f32::INFINITY },
            if dir.z != 0.0 { 1.0 / dir.z } else { f32::INFINITY },
        );
        let mut stack = vec![self.root];
        while let Some(idx) = stack.pop() {
            let node = &self.nodes[idx as usize];
            let Some(t_box) = node.bounds.ray_intersect(origin, inv_dir) else {
                continue;
            };
            if t_box > max_t {
                continue;
            }
            if node.leaf {
                for p in &self.primitives[node.start as usize..node.end as usize] {
                    if let Some(t) = p.aabb.ray_intersect(origin, inv_dir) {
                        if t <= max_t {
                            hits.push((p.id, t));
                        }
                    }
                }
            } else {
                stack.push(node.left);
                stack.push(node.right);
            }
        }
        hits.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        hits
    }

    /// Query all primitive ids whose AABB is inside or intersects the frustum.
    pub fn frustum_query(&self, planes: &[Plane; 6]) -> Vec<u32> {
        let mut hits = Vec::new();
        if self.root == u32::MAX {
            return hits;
        }
        let mut stack = vec![self.root];
        while let Some(idx) = stack.pop() {
            let node = &self.nodes[idx as usize];
            if !node.bounds.in_frustum(planes) {
                continue;
            }
            if node.leaf {
                for p in &self.primitives[node.start as usize..node.end as usize] {
                    if p.aabb.in_frustum(planes) {
                        hits.push(p.id);
                    }
                }
            } else {
                stack.push(node.left);
                stack.push(node.right);
            }
        }
        hits
    }

    /// Update a primitive's AABB and refit ancestor bounds.
    /// Returns true if the primitive was found.
    /// Note: This does not restructure the tree; for large movements, rebuild instead.
    pub fn refit_primitive(&mut self, id: u32, new_aabb: Aabb) -> bool {
        let mut found_idx = None;
        for (i, p) in self.primitives.iter_mut().enumerate() {
            if p.id == id {
                p.aabb = new_aabb;
                found_idx = Some(i);
                break;
            }
        }
        let Some(_) = found_idx else { return false };
        if self.root == u32::MAX {
            return true;
        }
        refit_recursive(self.root, &mut self.nodes, &self.primitives);
        true
    }
}

fn build_recursive(
    primitives: &mut [Primitive],
    start: u32,
    end: u32,
    nodes: &mut Vec<BvhNode>,
) -> u32 {
    let slice = &mut primitives[start as usize..end as usize];
    let bounds = slice
        .iter()
        .fold(Aabb { min: Vec3::splat(f32::INFINITY), max: Vec3::splat(f32::NEG_INFINITY) }, |acc, p| {
            acc.union(&p.aabb)
        });

    let count = (end - start) as usize;
    if count <= 4 {
        let idx = nodes.len() as u32;
        nodes.push(BvhNode::leaf(bounds, start, end));
        return idx;
    }

    let extent = bounds.max - bounds.min;
    let axis = if extent.x >= extent.y && extent.x >= extent.z {
        0
    } else if extent.y >= extent.z {
        1
    } else {
        2
    };

    slice.sort_by(|a, b| {
        let ca = a.aabb.center()[axis];
        let cb = b.aabb.center()[axis];
        ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
    });

    let mid = count / 2;
    let idx = nodes.len() as u32;
    nodes.push(BvhNode::internal(bounds, 0, 0));
    let left = build_recursive(primitives, start, start + mid as u32, nodes);
    let right = build_recursive(primitives, start + mid as u32, end, nodes);
    nodes[idx as usize].left = left;
    nodes[idx as usize].right = right;
    idx
}

fn refit_recursive(node_idx: u32, nodes: &mut [BvhNode], primitives: &[Primitive]) {
    let (is_leaf, left_idx, right_idx, start, end) = {
        let node = &nodes[node_idx as usize];
        (node.leaf, node.left, node.right, node.start, node.end)
    };
    if is_leaf {
        let slice = &primitives[start as usize..end as usize];
        let bounds = slice.iter().fold(
            Aabb { min: Vec3::splat(f32::INFINITY), max: Vec3::splat(f32::NEG_INFINITY) },
            |acc, p| acc.union(&p.aabb),
        );
        nodes[node_idx as usize].bounds = bounds;
        return;
    }
    refit_recursive(left_idx, nodes, primitives);
    refit_recursive(right_idx, nodes, primitives);
    let (left_bounds, right_bounds) = {
        let (left, right) = nodes.split_at_mut(right_idx as usize);
        (left[left_idx as usize].bounds, right[0].bounds)
    };
    nodes[node_idx as usize].bounds = left_bounds.union(&right_bounds);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_items() -> Vec<(u32, Aabb)> {
        vec![
            (0, Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0))),
            (1, Aabb::new(Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 1.0))),
            (2, Aabb::new(Vec3::new(0.0, 2.0, 0.0), Vec3::new(1.0, 3.0, 1.0))),
            (3, Aabb::new(Vec3::new(2.0, 2.0, 0.0), Vec3::new(3.0, 3.0, 1.0))),
            (4, Aabb::new(Vec3::new(10.0, 10.0, 10.0), Vec3::new(11.0, 11.0, 11.0))),
        ]
    }

    #[test]
    fn test_build_empty() {
        let bvh = Bvh::empty();
        assert_eq!(bvh.node_count(), 0);
        assert_eq!(bvh.primitive_count(), 0);
    }

    #[test]
    fn test_build_basic() {
        let bvh = Bvh::build(&make_items());
        assert!(bvh.node_count() >= 3);
        assert_eq!(bvh.primitive_count(), 5);
    }

    #[test]
    fn test_aabb_query_hit() {
        let bvh = Bvh::build(&make_items());
        let query = Aabb::new(Vec3::new(0.5, 0.5, 0.5), Vec3::new(2.5, 2.5, 2.5));
        let mut hits = bvh.aabb_query(&query);
        hits.sort();
        assert_eq!(hits, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_aabb_query_miss() {
        let bvh = Bvh::build(&make_items());
        let query = Aabb::new(Vec3::new(5.0, 5.0, 5.0), Vec3::new(6.0, 6.0, 6.0));
        let hits = bvh.aabb_query(&query);
        assert!(hits.is_empty());
    }

    #[test]
    fn test_ray_query() {
        let bvh = Bvh::build(&make_items());
        let origin = Vec3::new(-5.0, 0.5, 0.5);
        let dir = Vec3::new(1.0, 0.0, 0.0);
        let hits = bvh.ray_query(origin, dir, 100.0);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].0, 0);
        assert!(hits[0].1 > 0.0);
    }

    #[test]
    fn test_ray_query_max_t() {
        let bvh = Bvh::build(&make_items());
        let origin = Vec3::new(-5.0, 0.5, 0.5);
        let dir = Vec3::new(1.0, 0.0, 0.0);
        let hits = bvh.ray_query(origin, dir, 3.0);
        assert!(hits.iter().all(|(_, t)| *t <= 3.0));
    }

    #[test]
    fn test_frustum_query_all_in() {
        let bvh = Bvh::build(&make_items());
        let planes = [
            Plane::new(Vec3::new(1.0, 0.0, 0.0), 100.0),
            Plane::new(Vec3::new(-1.0, 0.0, 0.0), 100.0),
            Plane::new(Vec3::new(0.0, 1.0, 0.0), 100.0),
            Plane::new(Vec3::new(0.0, -1.0, 0.0), 100.0),
            Plane::new(Vec3::new(0.0, 0.0, 1.0), 100.0),
            Plane::new(Vec3::new(0.0, 0.0, -1.0), 100.0),
        ];
        let mut hits = bvh.frustum_query(&planes);
        hits.sort();
        assert_eq!(hits, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_frustum_query_partial() {
        let bvh = Bvh::build(&make_items());
        let planes = [
            Plane::new(Vec3::new(1.0, 0.0, 0.0), 0.0),
            Plane::new(Vec3::new(-1.0, 0.0, 0.0), 1.0),
            Plane::new(Vec3::new(0.0, 1.0, 0.0), 100.0),
            Plane::new(Vec3::new(0.0, -1.0, 0.0), 100.0),
            Plane::new(Vec3::new(0.0, 0.0, 1.0), 100.0),
            Plane::new(Vec3::new(0.0, 0.0, -1.0), 100.0),
        ];
        let mut hits = bvh.frustum_query(&planes);
        hits.sort();
        assert_eq!(hits, vec![0, 2]);
    }

    #[test]
    fn test_refit_primitive() {
        let mut bvh = Bvh::build(&make_items());
        let new_aabb = Aabb::new(Vec3::new(20.0, 20.0, 20.0), Vec3::new(21.0, 21.0, 21.0));
        assert!(bvh.refit_primitive(0, new_aabb));
        let query = Aabb::new(Vec3::new(19.0, 19.0, 19.0), Vec3::new(22.0, 22.0, 22.0));
        let hits = bvh.aabb_query(&query);
        assert!(hits.contains(&0));
        let old_query = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let old_hits = bvh.aabb_query(&old_query);
        assert!(!old_hits.contains(&0));
    }

    #[test]
    fn test_refit_missing_primitive() {
        let mut bvh = Bvh::build(&make_items());
        let new_aabb = Aabb::new(Vec3::new(20.0, 20.0, 20.0), Vec3::new(21.0, 21.0, 21.0));
        assert!(!bvh.refit_primitive(999, new_aabb));
    }

    #[test]
    fn test_frustum_from_view_proj_identity() {
        // Identity view-projection: frustum is the unit cube [-1, 1]^3.
        let planes = frustum_from_view_proj(Mat4::IDENTITY);

        // Origin should be inside.
        let origin = Vec3::ZERO;
        for plane in &planes {
            assert!(
                plane.normal.dot(origin) + plane.d >= 0.0,
                "Origin should be inside frustum, plane {:?} failed",
                plane
            );
        }

        // (2, 0, 0) is outside (x > 1) — should fail right plane.
        let p = Vec3::new(2.0, 0.0, 0.0);
        let mut outside_count = 0;
        for plane in &planes {
            if plane.normal.dot(p) + plane.d < 0.0 {
                outside_count += 1;
            }
        }
        assert!(outside_count > 0, "Point (2,0,0) should be outside frustum");

        // Check normals are normalized.
        for plane in &planes {
            assert!(
                (plane.normal.length() - 1.0).abs() < 1e-5,
                "Plane normal should be normalized, got length {}",
                plane.normal.length()
            );
        }
    }

    #[test]
    fn test_frustum_from_view_proj_perspective() {
        // Perspective camera at (0, 0, 5) looking at origin (down -Z).
        // View matrix: translate world by (0, 0, -5).
        let view = Mat4::from_translation(Vec3::new(0.0, 0.0, -5.0));
        // Right-handed perspective: fov 90°, aspect 1, near 0.1, far 100.
        let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
        let view_proj = proj * view;
        let planes = frustum_from_view_proj(view_proj);

        // Origin (0,0,0) in world is at (0,0,-5) in view space, within near/far.
        // Should be inside the frustum.
        let origin = Vec3::ZERO;
        for plane in &planes {
            assert!(
                plane.normal.dot(origin) + plane.d >= 0.0,
                "Origin should be inside perspective frustum, plane {:?} failed",
                plane
            );
        }

        // Point far behind camera (0, 0, 10) is outside.
        let behind = Vec3::new(0.0, 0.0, 10.0);
        let mut outside = false;
        for plane in &planes {
            if plane.normal.dot(behind) + plane.d < 0.0 {
                outside = true;
                break;
            }
        }
        assert!(outside, "Point behind camera should be outside frustum");

        // Point far to the side (100, 0, 0) is outside horizontal fov.
        let side = Vec3::new(100.0, 0.0, 0.0);
        let mut outside_side = false;
        for plane in &planes {
            if plane.normal.dot(side) + plane.d < 0.0 {
                outside_side = true;
                break;
            }
        }
        assert!(outside_side, "Point far to side should be outside frustum");
    }

    #[test]
    fn test_frustum_from_view_proj_normals_inward() {
        // For identity matrix, verify each plane's normal points inward
        // (toward the cube center at origin).
        let planes = frustum_from_view_proj(Mat4::IDENTITY);

        // left plane: normal should have +x component (points right, inward)
        assert!(planes[0].normal.x > 0.0, "Left plane normal should point +x");
        // right plane: normal should have -x component
        assert!(planes[1].normal.x < 0.0, "Right plane normal should point -x");
        // bottom plane: normal should have +y component
        assert!(planes[2].normal.y > 0.0, "Bottom plane normal should point +y");
        // top plane: normal should have -y component
        assert!(planes[3].normal.y < 0.0, "Top plane normal should point -y");
        // near plane: normal should have +z component
        assert!(planes[4].normal.z > 0.0, "Near plane normal should point +z");
        // far plane: normal should have -z component
        assert!(planes[5].normal.z < 0.0, "Far plane normal should point -z");
    }

    #[test]
    fn test_large_scale_build() {
        let items: Vec<(u32, Aabb)> = (0..1000)
            .map(|i| {
                let x = (i % 100) as f32 * 2.0;
                let y = (i / 100) as f32 * 2.0;
                (i, Aabb::new(Vec3::new(x, y, 0.0), Vec3::new(x + 1.0, y + 1.0, 1.0)))
            })
            .collect();
        let bvh = Bvh::build(&items);
        assert_eq!(bvh.primitive_count(), 1000);
        assert!(bvh.node_count() < 2000);
        let query = Aabb::new(Vec3::new(50.0, 10.0, 0.0), Vec3::new(52.0, 12.0, 1.0));
        let hits = bvh.aabb_query(&query);
        assert!(!hits.is_empty());
        for &id in &hits {
            let x = (id % 100) as f32 * 2.0;
            let y = (id / 100) as f32 * 2.0;
            assert!(x >= 50.0 && x <= 52.0);
            assert!(y >= 10.0 && y <= 12.0);
        }
    }
}
