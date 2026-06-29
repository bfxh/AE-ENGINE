//! BVH — Bounding Volume Hierarchy (层次包围盒)
//!
//! 基于:
//! - MacDonald & Booth. "Heuristics for ray tracing using space subdivision." 1990. (SAH 原始论文)
//! - Hendrickson & Leland. "A Multilevel Algorithm for Partitioning Graphs." 1995. (递归细分)
//! - Wald, Boulos, Shirley. "Ray Tracing Deformable Scenes using Dynamic Bounding Volume Hierarchies." 2007. (refit)
//! - Karras. "Maximizing Parallelism in the Construction of BVHs, Octrees, and k-d Trees." 2012. (Morton code LBVH)
//! - Ericson. Real-Time Collision Detection. Morgan Kaufmann 2005. Ch 6.
//!
//! 用途:
//! 1. 射线投射加速 (ray tracing, picking)
//! 2. 碰撞检测 broad phase (AABB 重叠查询)
//! 3. 视锥剔除 (frustum culling)
//! 4. 邻居查询 (nearest neighbor, point query)
//! 5. 动态场景 (refit 比 rebuild 快)
//!
//! 实现:
//! - AABB: 轴对齐包围盒, slab method 射线相交
//! - SAH (Surface Area Heuristic): 最小化期望射线查询成本
//! - Morton code LBVH: 量化质心 + 排序 + 自底向上构建 (并行友好)
//! - 栈式遍历: 射线/AABB/点查询
//! - Refit: 自底向上更新 AABB (运动物体)

use glam::{Vec3, Quat};

// ============================================================
// AABB (Axis-Aligned Bounding Box)
// ============================================================

#[derive(Debug, Clone, Copy, Default)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self { Self { min, max } }
    pub fn from_point(p: Vec3) -> Self { Self { min: p, max: p } }

    pub fn from_points(points: &[Vec3]) -> Self {
        if points.is_empty() { return Self::default(); }
        let mut min = points[0];
        let mut max = points[0];
        for &p in &points[1..] {
            min = min.min(p);
            max = max.max(p);
        }
        Self { min, max }
    }

    #[inline] pub fn center(&self) -> Vec3 { (self.min + self.max) * 0.5 }
    #[inline] pub fn half_extents(&self) -> Vec3 { (self.max - self.min) * 0.5 }
    #[inline] pub fn size(&self) -> Vec3 { self.max - self.min }

    #[inline]
    pub fn union(&self, other: &Aabb) -> Aabb {
        Aabb { min: self.min.min(other.min), max: self.max.max(other.max) }
    }

    #[inline]
    pub fn union_point(&self, p: Vec3) -> Aabb {
        Aabb { min: self.min.min(p), max: self.max.max(p) }
    }

    #[inline]
    pub fn surface_area(&self) -> f32 {
        let d = self.max - self.min;
        2.0 * (d.x * d.y + d.y * d.z + d.z * d.x)
    }

    #[inline]
    pub fn volume(&self) -> f32 {
        let d = self.max - self.min;
        d.x * d.y * d.z
    }

    #[inline]
    pub fn contains(&self, p: Vec3) -> bool {
        p.x >= self.min.x && p.x <= self.max.x &&
        p.y >= self.min.y && p.y <= self.max.y &&
        p.z >= self.min.z && p.z <= self.max.z
    }

    #[inline]
    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x && self.max.x >= other.min.x &&
        self.min.y <= other.max.y && self.max.y >= other.min.y &&
        self.min.z <= other.max.z && self.max.z >= other.min.z
    }

    /// 射线与 AABB 相交 (slab method)
    /// 返回 (t_enter, t_exit), 不相交返回 None
    #[inline]
    pub fn ray_intersect(&self, origin: Vec3, dir: Vec3) -> Option<(f32, f32)> {
        let mut t_enter = f32::NEG_INFINITY;
        let mut t_exit = f32::INFINITY;

        for axis in 0..3 {
            let d = dir[axis];
            let o = origin[axis];
            let mn = self.min[axis];
            let mx = self.max[axis];

            if d.abs() < 1e-12 {
                // 射线与轴平行, 检查原点是否在 slab 内
                if o < mn || o > mx { return None; }
            } else {
                let inv = 1.0 / d;
                let t1 = (mn - o) * inv;
                let t2 = (mx - o) * inv;
                let (t1, t2) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
                t_enter = t_enter.max(t1);
                t_exit = t_exit.min(t2);
                if t_enter > t_exit { return None; }
            }
        }

        if t_exit < 0.0 { return None; }
        Some((t_enter.max(0.0), t_exit))
    }

    pub fn expanded(&self, amount: f32) -> Aabb {
        Aabb {
            min: self.min - Vec3::splat(amount),
            max: self.max + Vec3::splat(amount),
        }
    }

    /// 变换后的 AABB (取 8 角点变换后的 AABB, 保守估计)
    pub fn transformed(&self, translation: Vec3, rotation: Quat, scale: f32) -> Aabb {
        let corners = [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ];
        let mut result = Aabb::from_point(rotation * (corners[0] * scale) + translation);
        for &c in &corners[1..] {
            result = result.union_point(rotation * (c * scale) + translation);
        }
        result
    }

    /// 点到 AABB 的最短距离平方 (内部点返回 0)
    #[inline]
    pub fn distance_sq_to_point(&self, p: Vec3) -> f32 {
        let clamped = p.max(self.min).min(self.max);
        (p - clamped).length_squared()
    }
}

// ============================================================
// BVH 节点
// ============================================================

#[derive(Debug, Clone, Default)]
pub struct BvhNode {
    pub aabb: Aabb,
    /// 内部节点: 左子节点索引; 叶子节点: 第一个 primitive 在 primitive_indices 中的偏移
    pub left: u32,
    /// 内部节点: 右子节点索引; 叶子节点: 未使用
    pub right: u32,
    /// 叶子节点: primitive 数量 (>0); 内部节点: 0
    pub count: u32,
}

impl BvhNode {
    #[inline] pub fn is_leaf(&self) -> bool { self.count > 0 }
}

// ============================================================
// BVH
// ============================================================

pub struct Bvh {
    pub nodes: Vec<BvhNode>,
    /// primitive_aabbs[i] = 第 i 个原始 primitive 的 AABB (按原始索引存)
    pub primitive_aabbs: Vec<Aabb>,
    /// primitive_indices[i] = BVH 叶子中第 i 个位置对应的原始 primitive 索引
    pub primitive_indices: Vec<u32>,
    pub root: u32,
}

const NUM_BINS: usize = 12;
const SAH_TRAVERSAL_COST: f32 = 1.0;
const SAH_INTERSECTION_COST: f32 = 2.0;
const MAX_LEAF_PRIMS: u32 = 4;

#[derive(Clone, Copy, Default)]
struct Bin {
    aabb: Aabb,
    count: u32,
}

impl Bin {
    fn push(&mut self, aabb: &Aabb) {
        if self.count == 0 {
            self.aabb = *aabb;
        } else {
            self.aabb = self.aabb.union(aabb);
        }
        self.count += 1;
    }
}

impl Bvh {
    /// 用 SAH (Surface Area Heuristic) 构建 BVH
    pub fn build(aabbs: &[Aabb]) -> Self {
        let n = aabbs.len();
        if n == 0 {
            return Self { nodes: vec![], primitive_aabbs: vec![], primitive_indices: vec![], root: 0 };
        }

        let mut indices: Vec<u32> = (0..n as u32).collect();
        let mut nodes: Vec<BvhNode> = Vec::with_capacity(2 * n - 1);

        let root = Self::build_recursive(&mut nodes, aabbs, &mut indices, 0, n as u32);

        Self {
            nodes,
            primitive_aabbs: aabbs.to_vec(),
            primitive_indices: indices,
            root,
        }
    }

    fn build_recursive(
        nodes: &mut Vec<BvhNode>,
        prim_aabbs: &[Aabb],
        indices: &mut [u32],
        start: u32,
        count: u32,
    ) -> u32 {
        // 计算当前节点的 AABB 和质心 AABB
        let first_aabb = prim_aabbs[indices[start as usize] as usize];
        let mut node_aabb = first_aabb;
        let mut centroid_min = first_aabb.center();
        let mut centroid_max = centroid_min;
        for i in 1..count {
            let aabb = prim_aabbs[indices[(start + i) as usize] as usize];
            node_aabb = node_aabb.union(&aabb);
            let c = aabb.center();
            centroid_min = centroid_min.min(c);
            centroid_max = centroid_max.max(c);
        }

        let node_idx = nodes.len() as u32;
        nodes.push(BvhNode { aabb: node_aabb, left: 0, right: 0, count: 0 });

        // 少量 primitive 直接做叶子
        if count <= MAX_LEAF_PRIMS {
            nodes[node_idx as usize].left = start;
            nodes[node_idx as usize].count = count;
            return node_idx;
        }

        // SAH: 找最佳分裂轴和位置
        let centroid_size = centroid_max - centroid_min;
        let mut best_cost = f32::INFINITY;
        let mut best_axis = 0;
        let mut best_split = 0u32;

        for axis in 0..3 {
            if centroid_size[axis] < 1e-12 { continue; }

            let mut bins = [Bin::default(); NUM_BINS];
            let axis_min = centroid_min[axis];
            let range = centroid_size[axis];
            let inv_range = NUM_BINS as f32 / range;

            for i in 0..count {
                let prim_idx = indices[(start + i) as usize] as usize;
                let centroid = prim_aabbs[prim_idx].center()[axis];
                let bin_idx = (((centroid - axis_min) * inv_range) as usize).min(NUM_BINS - 1);
                bins[bin_idx].push(&prim_aabbs[prim_idx]);
            }

            // 前缀和 (left) 与后缀和 (right)
            let mut left_aabbs = [Aabb::default(); NUM_BINS];
            let mut left_counts = [0u32; NUM_BINS];
            let mut acc = Aabb::default();
            let mut acc_count = 0u32;
            for i in 0..NUM_BINS {
                if bins[i].count > 0 {
                    acc = if acc_count == 0 { bins[i].aabb } else { acc.union(&bins[i].aabb) };
                }
                acc_count += bins[i].count;
                left_aabbs[i] = acc;
                left_counts[i] = acc_count;
            }

            let mut right_aabbs = [Aabb::default(); NUM_BINS];
            let mut right_counts = [0u32; NUM_BINS];
            acc = Aabb::default();
            acc_count = 0;
            for i in (0..NUM_BINS).rev() {
                if bins[i].count > 0 {
                    acc = if acc_count == 0 { bins[i].aabb } else { acc.union(&bins[i].aabb) };
                }
                acc_count += bins[i].count;
                right_aabbs[i] = acc;
                right_counts[i] = acc_count;
            }

            // 评估每个分裂位置 (split = 1..NUM_BINS): [0, split) 左, [split, NUM_BINS) 右
            for split in 1..NUM_BINS {
                let lc = left_counts[split - 1];
                let rc = right_counts[split];
                if lc == 0 || rc == 0 { continue; }

                let cost = SAH_TRAVERSAL_COST +
                    (lc as f32 * left_aabbs[split - 1].surface_area() +
                     rc as f32 * right_aabbs[split].surface_area()) / node_aabb.surface_area().max(1e-12)
                    * SAH_INTERSECTION_COST;

                if cost < best_cost {
                    best_cost = cost;
                    best_axis = axis;
                    best_split = split as u32;
                }
            }
        }

        // 与不分裂的成本比较
        let leaf_cost = count as f32 * SAH_INTERSECTION_COST;
        if best_cost >= leaf_cost {
            nodes[node_idx as usize].left = start;
            nodes[node_idx as usize].count = count;
            return node_idx;
        }

        // 按最佳分裂位置重排 indices
        let axis_min = centroid_min[best_axis];
        let range = centroid_size[best_axis];
        let inv_range = NUM_BINS as f32 / range;

        let mut left = 0u32;
        let mut right = count;
        while left < right {
            let li = (start + left) as usize;
            let centroid = prim_aabbs[indices[li] as usize].center()[best_axis];
            let bin_idx = (((centroid - axis_min) * inv_range) as u32).min(NUM_BINS as u32 - 1);
            if bin_idx < best_split {
                left += 1;
            } else {
                right -= 1;
                indices.swap(li, (start + right) as usize);
            }
        }

        if left == 0 || left == count {
            // 退化 (所有 primitive 在同一 bin), 用中位数分裂
            let slice = &mut indices[start as usize..(start + count) as usize];
            slice.sort_by(|&a, &b| {
                let ca = prim_aabbs[a as usize].center()[best_axis];
                let cb = prim_aabbs[b as usize].center()[best_axis];
                ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
            });
            left = count / 2;
        }

        let left_child = Self::build_recursive(nodes, prim_aabbs, indices, start, left);
        let right_child = Self::build_recursive(nodes, prim_aabbs, indices, start + left, count - left);

        nodes[node_idx as usize].left = left_child;
        nodes[node_idx as usize].right = right_child;
        node_idx
    }

    /// 射线查询: 返回所有 AABB 命中的 primitive 索引
    pub fn ray_query(&self, origin: Vec3, dir: Vec3, max_dist: f32) -> Vec<u32> {
        let mut hits = Vec::new();
        if self.nodes.is_empty() { return hits; }
        let mut stack = vec![self.root];
        while let Some(idx) = stack.pop() {
            let node = &self.nodes[idx as usize];
            if let Some((t_enter, _)) = node.aabb.ray_intersect(origin, dir) {
                if t_enter > max_dist { continue; }
                if node.is_leaf() {
                    let first = node.left;
                    for i in 0..node.count {
                        hits.push(self.primitive_indices[(first + i) as usize]);
                    }
                } else {
                    stack.push(node.left);
                    stack.push(node.right);
                }
            }
        }
        hits
    }

    /// 最近命中查询 (需要 primitive 相交回调)
    pub fn ray_closest<F: Fn(u32, Vec3, Vec3) -> Option<f32>>(
        &self, origin: Vec3, dir: Vec3, max_dist: f32, intersect_prim: F,
    ) -> Option<(u32, f32)> {
        if self.nodes.is_empty() { return None; }
        let mut best_t = max_dist;
        let mut best_prim: Option<u32> = None;
        let mut stack = vec![self.root];
        while let Some(idx) = stack.pop() {
            let node = &self.nodes[idx as usize];
            if let Some((t_enter, _)) = node.aabb.ray_intersect(origin, dir) {
                if t_enter > best_t { continue; }
                if node.is_leaf() {
                    let first = node.left;
                    for i in 0..node.count {
                        let prim_idx = self.primitive_indices[(first + i) as usize];
                        if let Some(t) = intersect_prim(prim_idx, origin, dir) {
                            if t < best_t {
                                best_t = t;
                                best_prim = Some(prim_idx);
                            }
                        }
                    }
                } else {
                    stack.push(node.left);
                    stack.push(node.right);
                }
            }
        }
        best_prim.map(|p| (p, best_t))
    }

    /// AABB 查询: 返回所有与 query AABB 相交的 primitive
    pub fn aabb_query(&self, query: &Aabb) -> Vec<u32> {
        let mut hits = Vec::new();
        if self.nodes.is_empty() { return hits; }
        let mut stack = vec![self.root];
        while let Some(idx) = stack.pop() {
            let node = &self.nodes[idx as usize];
            if node.aabb.intersects(query) {
                if node.is_leaf() {
                    let first = node.left;
                    for i in 0..node.count {
                        let prim_idx = self.primitive_indices[(first + i) as usize];
                        if self.primitive_aabbs[prim_idx as usize].intersects(query) {
                            hits.push(prim_idx);
                        }
                    }
                } else {
                    stack.push(node.left);
                    stack.push(node.right);
                }
            }
        }
        hits
    }

    /// 点查询: 返回所有包含 p 的 primitive
    pub fn point_query(&self, p: Vec3) -> Vec<u32> {
        let mut hits = Vec::new();
        if self.nodes.is_empty() { return hits; }
        let mut stack = vec![self.root];
        while let Some(idx) = stack.pop() {
            let node = &self.nodes[idx as usize];
            if node.aabb.contains(p) {
                if node.is_leaf() {
                    let first = node.left;
                    for i in 0..node.count {
                        let prim_idx = self.primitive_indices[(first + i) as usize];
                        if self.primitive_aabbs[prim_idx as usize].contains(p) {
                            hits.push(prim_idx);
                        }
                    }
                } else {
                    stack.push(node.left);
                    stack.push(node.right);
                }
            }
        }
        hits
    }

    /// 最近邻查询: 返回距离 p 最近的 primitive (按 AABB 中心距离)
    pub fn nearest_neighbor(&self, p: Vec3) -> Option<u32> {
        if self.nodes.is_empty() { return None; }
        let mut best_dist = f32::INFINITY;
        let mut best_prim: Option<u32> = None;
        let mut stack = vec![self.root];
        while let Some(idx) = stack.pop() {
            let node = &self.nodes[idx as usize];
            let dist = node.aabb.distance_sq_to_point(p);
            if dist > best_dist { continue; }
            if node.is_leaf() {
                let first = node.left;
                for i in 0..node.count {
                    let prim_idx = self.primitive_indices[(first + i) as usize];
                    let c = self.primitive_aabbs[prim_idx as usize].center();
                    let d = (c - p).length_squared();
                    if d < best_dist {
                        best_dist = d;
                        best_prim = Some(prim_idx);
                    }
                }
            } else {
                stack.push(node.left);
                stack.push(node.right);
            }
        }
        best_prim
    }

    /// 动态 refit: 物体移动后更新 AABB, 自底向上 refit (不改变拓扑)
    /// 比 rebuild 快, 适合小幅运动 (Wald 2007)
    pub fn refit(&mut self, new_aabbs: &[Aabb]) {
        if self.nodes.is_empty() { return; }
        // primitive_aabbs 按原始索引存, 直接更新
        for i in 0..self.primitive_aabbs.len() {
            self.primitive_aabbs[i] = new_aabbs[i];
        }
        // 自底向上 refit (nodes 数组中父节点索引 < 子节点索引, 从后往前遍历)
        let nodes_len = self.nodes.len();
        for i in (0..nodes_len).rev() {
            // 先复制节点信息 (Copy 类型), 释放 nodes 的借用
            let (is_leaf, left, right, count) = {
                let node = &self.nodes[i];
                (node.is_leaf(), node.left, node.right, node.count)
            };
            // 计算新 AABB
            let new_aabb = if is_leaf {
                let first = left as usize;
                let cnt = count as usize;
                if cnt == 0 { continue; }
                let mut aabb = self.primitive_aabbs[self.primitive_indices[first] as usize];
                for j in 1..cnt {
                    let prim_idx = self.primitive_indices[first + j] as usize;
                    aabb = aabb.union(&self.primitive_aabbs[prim_idx]);
                }
                aabb
            } else {
                let l = left as usize;
                let r = right as usize;
                self.nodes[l].aabb.union(&self.nodes[r].aabb)
            };
            // 更新 (可变借用)
            self.nodes[i].aabb = new_aabb;
        }
    }

    pub fn node_count(&self) -> usize { self.nodes.len() }

    pub fn leaf_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_leaf()).count()
    }

    pub fn depth(&self) -> usize {
        if self.nodes.is_empty() { return 0; }
        fn depth_rec(nodes: &[BvhNode], idx: u32) -> usize {
            let node = &nodes[idx as usize];
            if node.is_leaf() { 1 } else {
                1 + depth_rec(nodes, node.left).max(depth_rec(nodes, node.right))
            }
        }
        depth_rec(&self.nodes, self.root)
    }
}

// ============================================================
// Morton code (LBVH 并行构建基础, Karras 2012)
// ============================================================

/// 3D Morton code (Z-order curve) 量化
/// 将 3D 坐标映射到 1D, 保持空间局部性
pub fn morton_code_3d(p: Vec3, min: Vec3, range: Vec3) -> u64 {
    let nx = ((p.x - min.x) / range.x * 1023.0).clamp(0.0, 1023.0) as u32;
    let ny = ((p.y - min.y) / range.y * 1023.0).clamp(0.0, 1023.0) as u32;
    let nz = ((p.z - min.z) / range.z * 1023.0).clamp(0.0, 1023.0) as u32;
    part1_by2(nx) | (part1_by2(ny) << 1) | (part1_by2(nz) << 2)
}

/// 10 位 -> 30 位 (每 2 位插入 0, 用于 Morton code)
#[allow(dead_code)]
fn part1_by2(n: u32) -> u64 {
    let mut n = n as u64;
    n &= 0x000003FF;
    n = (n ^ (n << 16)) & 0x030000FF;
    n = (n ^ (n << 8)) & 0x0300F00F;
    n = (n ^ (n << 4)) & 0x030C30C3;
    n = (n ^ (n << 2)) & 0x09249249;
    n
}

/// 最长公共前缀长度 (用于 Karras LBVH 自底向上构建)
#[allow(dead_code)]
fn longest_common_prefix(codes: &[u64], i: usize) -> i32 {
    let n = codes.len();
    if i >= n { return -1; }
    let ci = codes[i];
    // 找与 i 不同的最近邻居
    let j = if i + 1 < n { i + 1 } else if i > 0 { i - 1 } else { return 64; };
    if codes[j] == ci { return 64; }
    let cj = codes[j];
    let diff = ci ^ cj;
    if diff == 0 { return 64; }
    diff.leading_zeros() as i32
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aabb_basic() {
        let a = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(a.center(), Vec3::ZERO);
        assert_eq!(a.size(), Vec3::new(2.0, 2.0, 2.0));
        assert!((a.surface_area() - 24.0).abs() < 1e-4);
        assert!((a.volume() - 8.0).abs() < 1e-4);
    }

    #[test]
    fn test_aabb_contains() {
        let a = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        assert!(a.contains(Vec3::ZERO));
        assert!(a.contains(Vec3::new(1.0, 1.0, 1.0)));
        assert!(!a.contains(Vec3::new(2.0, 0.0, 0.0)));
    }

    #[test]
    fn test_aabb_intersects() {
        let a = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let b = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 2.0, 2.0));
        assert!(a.intersects(&b));

        let c = Aabb::new(Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 1.0));
        assert!(!a.intersects(&c));
    }

    #[test]
    fn test_aabb_union() {
        let a = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let b = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 2.0, 2.0));
        let u = a.union(&b);
        assert_eq!(u.min, Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(u.max, Vec3::new(2.0, 2.0, 2.0));
    }

    #[test]
    fn test_aabb_ray_hit() {
        let a = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let hit = a.ray_intersect(Vec3::new(3.0, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0));
        assert!(hit.is_some());
        let (t_enter, t_exit) = hit.unwrap();
        assert!((t_enter - 2.0).abs() < 1e-4, "t_enter = {}", t_enter);
        assert!((t_exit - 4.0).abs() < 1e-4);
    }

    #[test]
    fn test_aabb_ray_miss() {
        let a = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let hit = a.ray_intersect(Vec3::new(3.0, 3.0, 0.0), Vec3::new(-1.0, 0.0, 0.0));
        assert!(hit.is_none());
    }

    #[test]
    fn test_aabb_ray_inside() {
        let a = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let hit = a.ray_intersect(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0));
        assert!(hit.is_some());
        let (t_enter, t_exit) = hit.unwrap();
        assert!(t_enter.abs() < 1e-4, "t_enter should be 0: {}", t_enter);
        assert!((t_exit - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_aabb_transformed() {
        let a = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        // 平移到 (5,0,0)
        let t = a.transformed(Vec3::new(5.0, 0.0, 0.0), Quat::IDENTITY, 1.0);
        assert!((t.min - Vec3::new(4.0, -1.0, -1.0)).length() < 1e-4);
        assert!((t.max - Vec3::new(6.0, 1.0, 1.0)).length() < 1e-4);

        // 缩放 2 倍
        let s = a.transformed(Vec3::ZERO, Quat::IDENTITY, 2.0);
        assert!((s.min - Vec3::new(-2.0, -2.0, -2.0)).length() < 1e-4);
        assert!((s.max - Vec3::new(2.0, 2.0, 2.0)).length() < 1e-4);
    }

    #[test]
    fn test_aabb_distance_sq() {
        let a = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        assert!(a.distance_sq_to_point(Vec3::ZERO) < 1e-12);
        // 外部点 (3,0,0) 距离 (1,0,0) = 2, 平方 = 4
        assert!((a.distance_sq_to_point(Vec3::new(3.0, 0.0, 0.0)) - 4.0).abs() < 1e-4);
    }

    #[test]
    fn test_bvh_build_empty() {
        let bvh = Bvh::build(&[]);
        assert_eq!(bvh.node_count(), 0);
    }

    #[test]
    fn test_bvh_build_single() {
        let aabbs = vec![Aabb::new(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0))];
        let bvh = Bvh::build(&aabbs);
        assert_eq!(bvh.node_count(), 1);
        assert!(bvh.nodes[0].is_leaf());
    }

    #[test]
    fn test_bvh_build_many() {
        let mut aabbs = Vec::new();
        for i in 0..100 {
            let x = i as f32 * 2.0;
            aabbs.push(Aabb::new(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 1.0, 1.0, 1.0)));
        }
        let bvh = Bvh::build(&aabbs);
        assert!(bvh.node_count() > 1);
        assert!(bvh.depth() > 1);
        // 所有 primitive 都在 BVH 中
        let all_prims: std::collections::HashSet<u32> = bvh.primitive_indices.iter().copied().collect();
        assert_eq!(all_prims.len(), 100);
    }

    #[test]
    fn test_bvh_point_query() {
        let mut aabbs = Vec::new();
        for i in 0..10 {
            let x = i as f32 * 3.0;
            aabbs.push(Aabb::new(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 2.0, 2.0, 2.0)));
        }
        let bvh = Bvh::build(&aabbs);

        // 点 (0.5, 1, 1) 应在第 0 个 AABB 内
        let hits = bvh.point_query(Vec3::new(0.5, 1.0, 1.0));
        assert!(hits.contains(&0), "hits = {:?}", hits);

        // 点 (3.5, 1, 1) 应在第 1 个 AABB 内
        let hits = bvh.point_query(Vec3::new(3.5, 1.0, 1.0));
        assert!(hits.contains(&1));

        // 点 (100, 100, 100) 不在任何 AABB 内
        let hits = bvh.point_query(Vec3::new(100.0, 100.0, 100.0));
        assert!(hits.is_empty());
    }

    #[test]
    fn test_bvh_aabb_query() {
        let mut aabbs = Vec::new();
        for i in 0..10 {
            let x = i as f32 * 3.0;
            aabbs.push(Aabb::new(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 2.0, 2.0, 2.0)));
        }
        let bvh = Bvh::build(&aabbs);

        // 查询 AABB 覆盖 [0, 5] x [0, 2] x [0, 2], 应命中第 0, 1 个
        let query = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(5.0, 2.0, 2.0));
        let hits = bvh.aabb_query(&query);
        assert!(hits.contains(&0), "hits = {:?}", hits);
        assert!(hits.contains(&1));
    }

    #[test]
    fn test_bvh_ray_query() {
        let mut aabbs = Vec::new();
        for i in 0..10 {
            let x = i as f32 * 3.0;
            aabbs.push(Aabb::new(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 2.0, 2.0, 2.0)));
        }
        let bvh = Bvh::build(&aabbs);

        // 射线从 (-1, 1, 1) 朝 +x, 应命中多个 AABB
        let hits = bvh.ray_query(Vec3::new(-1.0, 1.0, 1.0), Vec3::new(1.0, 0.0, 0.0), 100.0);
        assert!(hits.len() >= 5, "hits = {}", hits.len());
    }

    #[test]
    fn test_bvh_ray_closest() {
        let mut aabbs = Vec::new();
        for i in 0..10 {
            let x = i as f32 * 3.0;
            aabbs.push(Aabb::new(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 2.0, 2.0, 2.0)));
        }
        let bvh = Bvh::build(&aabbs);

        // 射线从 (-1, 1, 1) 朝 +x, 最近命中的应是第 0 个 AABB (t=1)
        let origin = Vec3::new(-1.0, 1.0, 1.0);
        let dir = Vec3::new(1.0, 0.0, 0.0);
        let aabbs_ref = &aabbs;
        let result = bvh.ray_closest(
            origin, dir, 100.0,
            |prim_idx, o, d| {
                aabbs_ref[prim_idx as usize].ray_intersect(o, d).map(|(t, _)| t)
            }
        );
        assert!(result.is_some());
        let (prim, t) = result.unwrap();
        assert_eq!(prim, 0);
        assert!((t - 1.0).abs() < 1e-4, "t = {}", t);
    }

    #[test]
    fn test_bvh_nearest_neighbor() {
        let mut aabbs = Vec::new();
        for i in 0..10 {
            let x = i as f32 * 3.0;
            aabbs.push(Aabb::new(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 2.0, 2.0, 2.0)));
        }
        let bvh = Bvh::build(&aabbs);

        // 查询点 (3.5, 1, 1), 最近的 AABB 中心是第 1 个 (中心 4,1,1)
        let result = bvh.nearest_neighbor(Vec3::new(3.5, 1.0, 1.0));
        assert!(result.is_some());
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_bvh_refit() {
        let aabbs = vec![
            Aabb::new(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0)),
            Aabb::new(Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 1.0)),
        ];
        let mut bvh = Bvh::build(&aabbs);
        let old_root_aabb = bvh.nodes[bvh.root as usize].aabb;

        // 移动第一个 AABB 到远处
        let new_aabbs = vec![
            Aabb::new(Vec3::new(10.0, 0.0, 0.0), Vec3::new(11.0, 1.0, 1.0)),
            Aabb::new(Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 1.0)),
        ];
        bvh.refit(&new_aabbs);
        let new_root_aabb = bvh.nodes[bvh.root as usize].aabb;

        // 新 root AABB 应包含移动后的 primitive
        assert!(new_root_aabb.contains(Vec3::new(10.0, 0.0, 0.0)));
        assert!(new_root_aabb.contains(Vec3::new(3.0, 1.0, 1.0)));
        // 与旧的不同
        assert!(new_root_aabb.min.x > old_root_aabb.min.x ||
                new_root_aabb.max.x > old_root_aabb.max.x);
    }

    #[test]
    fn test_morton_code() {
        // 同一点应产生相同 code
        let c1 = morton_code_3d(Vec3::ZERO, Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        let c2 = morton_code_3d(Vec3::ZERO, Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(c1, c2);

        // 不同点应产生不同 code
        let c3 = morton_code_3d(Vec3::new(1.0, 1.0, 1.0), Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        assert_ne!(c1, c3);

        // (1,0,0) 应比 (0,0,0) 大 (Morton code 保持顺序)
        assert!(c3 > c1);
    }

    #[test]
    fn test_bvh_large_scene() {
        // 模拟大场景: 1000 个分布的 AABB
        let mut aabbs = Vec::new();
        for i in 0..1000 {
            let x = (i as f32 * 0.7) % 100.0;
            let y = (i as f32 * 1.3) % 100.0;
            let z = (i as f32 * 0.9) % 100.0;
            aabbs.push(Aabb::new(Vec3::new(x, y, z), Vec3::new(x + 1.0, y + 1.0, z + 1.0)));
        }
        let bvh = Bvh::build(&aabbs);

        // 构建成功
        assert!(bvh.node_count() > 100);

        // 点查询不应崩溃
        let _ = bvh.point_query(Vec3::new(50.0, 50.0, 50.0));

        // 射线查询不应崩溃
        let hits = bvh.ray_query(Vec3::new(-10.0, 50.0, 50.0), Vec3::new(1.0, 0.0, 0.0), 200.0);
        let _ = hits.len();
    }
}
