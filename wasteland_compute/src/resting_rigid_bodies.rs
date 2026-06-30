//! Putting Rigid Bodies to Rest — 静止刚体姿态预测
//!
//! 基于:
//! - Baktash, Sharp, Zhou, Crane, Jacobson. "Putting Rigid Bodies to Rest."
//!   ACM Transactions on Graphics (SIGGRAPH 2025). DOI: 10.1145/3731203
//! - Milnor. Morse Theory. Princeton University Press, 1963.
//! - Edelsbrunner. Geometry and Topology for Mesh Generation. Cambridge, 2001.
//!
//! 核心思想:
//! 1. 凸多面体的支撑函数 h(u) = max_{v∈V} (v·u) 在单位球面 S² 上
//! 2. 由 Morse 理论, h(u) 的临界点对应多面体的几何特征:
//!    - 面法向 = 局部极大值 (稳定平衡姿态)
//!    - 顶点方向 = 局部极小值 (不稳定平衡)
//!    - 边法向 = 鞍点
//! 3. 稳定静止条件: 面法向是 h(u) 的局部极大值 AND 质心投影落在面内
//! 4. 静止概率 = 该面在 Gauss 映射上的吸引域面积 / 4π
//! 5. 吸引域通过 Monte Carlo 采样估计
//! 6. 逆设计: 调整顶点位置使目标面达到目标概率

use glam::Vec3;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

const GRADIENT_STEP: f32 = 0.15;
const GRADIENT_MAX_ITER: usize = 200;
const GRADIENT_TOL: f32 = 1e-4;
const COPLANAR_COS_THRESHOLD: f32 = 0.9999;

// ============================================================
// ConvexPolyhedron
// ============================================================

#[derive(Debug, Clone)]
pub struct ConvexPolyhedron {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<[usize; 3]>,
}

impl ConvexPolyhedron {
    pub fn cube(h: f32) -> Self {
        let vertices = vec![
            Vec3::new(-h, -h, -h), Vec3::new(h, -h, -h),
            Vec3::new(h, h, -h),   Vec3::new(-h, h, -h),
            Vec3::new(-h, -h, h),  Vec3::new(h, -h, h),
            Vec3::new(h, h, h),    Vec3::new(-h, h, h),
        ];
        // 面绕序: 从外侧看逆时针 (右手定则给出外法线)
        let faces = vec![
            [0, 3, 2], [0, 2, 1],  // -z
            [4, 5, 6], [4, 6, 7],  // +z
            [0, 4, 7], [0, 7, 3],  // -x
            [1, 2, 6], [1, 6, 5],  // +x
            [0, 1, 5], [0, 5, 4],  // -y
            [3, 7, 6], [3, 6, 2],  // +y
        ];
        Self { vertices, faces }
    }

    pub fn tetrahedron(a: f32) -> Self {
        let vertices = vec![
            Vec3::new(a, a, a),
            Vec3::new(a, -a, -a),
            Vec3::new(-a, a, -a),
            Vec3::new(-a, -a, a),
        ];
        let faces = vec![
            [0, 1, 2],
            [0, 2, 3],
            [0, 3, 1],
            [1, 3, 2],
        ];
        Self { vertices, faces }
    }

    pub fn octahedron(r: f32) -> Self {
        let vertices = vec![
            Vec3::new(r, 0.0, 0.0), Vec3::new(-r, 0.0, 0.0),
            Vec3::new(0.0, r, 0.0), Vec3::new(0.0, -r, 0.0),
            Vec3::new(0.0, 0.0, r), Vec3::new(0.0, 0.0, -r),
        ];
        let faces = vec![
            [0, 2, 4], [2, 1, 4], [1, 3, 4], [3, 0, 4],
            [2, 0, 5], [1, 2, 5], [3, 1, 5], [0, 3, 5],
        ];
        Self { vertices, faces }
    }

    /// 支撑函数 h(u) = max_{v∈V} (v·u)
    pub fn support(&self, u: Vec3) -> f32 {
        self.vertices.iter()
            .map(|&v| v.dot(u))
            .fold(f32::NEG_INFINITY, f32::max)
    }

    /// 返回达到支撑函数最大值的顶点
    pub fn support_vertex(&self, u: Vec3) -> Vec3 {
        let mut best = self.vertices[0];
        let mut best_dot = best.dot(u);
        for &v in &self.vertices[1..] {
            let d = v.dot(u);
            if d > best_dot {
                best_dot = d;
                best = v;
            }
        }
        best
    }

    /// 支撑函数的次梯度 (∇h(u) = v*, 即达到最大的顶点)
    pub fn support_gradient(&self, u: Vec3) -> Vec3 {
        self.support_vertex(u)
    }

    /// 支撑函数在球面切平面上的梯度 (用于梯度流)
    /// ∇_T h = v* - (v*·u)·u  (投影到 u 的切平面)
    pub fn support_tangent_gradient(&self, u: Vec3) -> Vec3 {
        let v_star = self.support_vertex(u);
        v_star - u * v_star.dot(u)
    }

    pub fn face_normal(&self, face: &[usize; 3]) -> Vec3 {
        let a = self.vertices[face[0]];
        let b = self.vertices[face[1]];
        let c = self.vertices[face[2]];
        (b - a).cross(c - a).normalize_or_zero()
    }

    pub fn face_centroid(&self, face: &[usize; 3]) -> Vec3 {
        (self.vertices[face[0]] + self.vertices[face[1]] + self.vertices[face[2]]) / 3.0
    }

    pub fn face_area(&self, face: &[usize; 3]) -> f32 {
        let a = self.vertices[face[0]];
        let b = self.vertices[face[1]];
        let c = self.vertices[face[2]];
        0.5 * (b - a).cross(c - a).length()
    }

    /// 体积 (散度定理): V = (1/6) Σ_faces (a · (b × c))
    pub fn volume(&self) -> f32 {
        let mut vol = 0.0;
        for face in &self.faces {
            let a = self.vertices[face[0]];
            let b = self.vertices[face[1]];
            let c = self.vertices[face[2]];
            vol += a.dot(b.cross(c));
        }
        vol / 6.0
    }

    /// 质心 (体积加权)
    pub fn centroid(&self) -> Vec3 {
        let mut acc = Vec3::ZERO;
        let mut vol = 0.0;
        for face in &self.faces {
            let a = self.vertices[face[0]];
            let b = self.vertices[face[1]];
            let c = self.vertices[face[2]];
            let tet_vol = a.dot(b.cross(c)) / 6.0;
            acc += (a + b + c) * tet_vol * 0.25;
            vol += tet_vol;
        }
        if vol.abs() < 1e-12 { Vec3::ZERO } else { acc / vol }
    }

    pub fn center_at_origin(&mut self) {
        let c = self.centroid();
        for v in &mut self.vertices {
            *v -= c;
        }
    }

    /// 合并共面相邻三角形, 返回唯一面法向分组
    /// 每组: (法向, 该方向包含的三角形索引列表)
    pub fn unique_face_groups(&self) -> Vec<(Vec3, Vec<usize>)> {
        let mut groups: Vec<(Vec3, Vec<usize>)> = Vec::new();
        for (i, face) in self.faces.iter().enumerate() {
            let n = self.face_normal(face);
            if n.length_squared() < 1e-20 {
                continue;
            }
            let found = groups.iter_mut().find(|(gn, _)| gn.dot(n) > COPLANAR_COS_THRESHOLD);
            match found {
                Some((_, indices)) => indices.push(i),
                None => groups.push((n, vec![i])),
            }
        }
        groups
    }
}

// ============================================================
// Critical Points (Morse Theory)
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CriticalKind {
    Maximum,
    Minimum,
    Saddle,
}

#[derive(Debug, Clone)]
pub struct CriticalPoint {
    pub direction: Vec3,
    pub height: f32,
    pub kind: CriticalKind,
    pub feature_index: usize,
}

/// 寻找支撑函数的所有临界点
/// - 面法向 = 极大值 (稳定静止面)
/// - 顶点方向 = 极小值 (不稳定平衡点)
pub fn find_critical_points(poly: &ConvexPolyhedron) -> Vec<CriticalPoint> {
    let mut points = Vec::new();
    let groups = poly.unique_face_groups();

    for (i, (n, _)) in groups.iter().enumerate() {
        let h = poly.support(*n);
        points.push(CriticalPoint {
            direction: *n,
            height: h,
            kind: CriticalKind::Maximum,
            feature_index: i,
        });
    }

    for (i, &v) in poly.vertices.iter().enumerate() {
        let u = v.normalize_or_zero();
        if u.length_squared() < 1e-20 {
            continue;
        }
        let h = poly.support(u);
        points.push(CriticalPoint {
            direction: u,
            height: h,
            kind: CriticalKind::Minimum,
            feature_index: i,
        });
    }

    points
}

// ============================================================
// Stable Rest States
// ============================================================

/// 判断一组共面三角形是否构成稳定静止面
/// 条件: 质心投影落在合并后的多边形内
pub fn is_stable_face_group(poly: &ConvexPolyhedron, n: Vec3, face_indices: &[usize]) -> bool {
    if face_indices.is_empty() {
        return false;
    }

    let centroid = poly.centroid();
    let face_vertex = poly.vertices[poly.faces[face_indices[0]][0]];
    let d_plane = n.dot(face_vertex);
    let proj = centroid + n * (d_plane - n.dot(centroid));

    // 收集该组所有顶点 (去重)
    let mut face_verts: Vec<Vec3> = Vec::new();
    for &fi in face_indices {
        for &vi in &poly.faces[fi] {
            let v = poly.vertices[vi];
            if !face_verts.iter().any(|&fv| (fv - v).length_squared() < 1e-16) {
                face_verts.push(v);
            }
        }
    }
    if face_verts.len() < 3 {
        return false;
    }

    // 在面平面内按角度排序顶点, 构成凸多边形
    let center = face_verts.iter().sum::<Vec3>() / face_verts.len() as f32;
    let t1 = if n.x.abs() < 0.9 { Vec3::X } else { Vec3::Y }
        .cross(n).normalize_or_zero();
    let t2 = n.cross(t1).normalize_or_zero();

    let mut verts = face_verts.clone();
    verts.sort_by(|a, b| {
        let aa = (a - center).dot(t1).atan2((a - center).dot(t2));
        let bb = (b - center).dot(t1).atan2((b - center).dot(t2));
        aa.partial_cmp(&bb).unwrap()
    });

    point_in_convex_polygon(proj, &verts, n)
}

/// 点是否在凸多边形内 (所有边同侧判定)
pub fn point_in_convex_polygon(p: Vec3, polygon: &[Vec3], n: Vec3) -> bool {
    let m = polygon.len();
    if m < 3 {
        return false;
    }
    let mut sign: i32 = 0;
    for i in 0..m {
        let a = polygon[i];
        let b = polygon[(i + 1) % m];
        let edge = b - a;
        let to_p = p - a;
        let cross = edge.cross(to_p);
        let d = cross.dot(n);
        if d > 1e-6 {
            if sign == -1 { return false; }
            sign = 1;
        } else if d < -1e-6 {
            if sign == 1 { return false; }
            sign = -1;
        }
    }
    true
}

/// 点是否在三角形内 (barycentric 坐标法)
pub fn point_in_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3, _n: Vec3) -> bool {
    let v0 = b - a;
    let v1 = c - a;
    let v2 = p - a;
    let d00 = v0.dot(v0);
    let d01 = v0.dot(v1);
    let d11 = v1.dot(v1);
    let d20 = v2.dot(v0);
    let d21 = v2.dot(v1);
    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < 1e-12 {
        return false;
    }
    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;
    u >= -1e-6 && v >= -1e-6 && w >= -1e-6
}

#[derive(Debug, Clone)]
pub struct RestState {
    pub up_direction: Vec3,
    pub face_index: usize,
    pub probability: f32,
    pub stability: f32,
}

/// 找到所有稳定的静止状态, 用 Monte Carlo 估计概率
pub fn find_rest_states(poly: &ConvexPolyhedron, n_samples: usize) -> Vec<RestState> {
    let mut rng = StdRng::seed_from_u64(42);
    find_rest_states_with_rng(poly, n_samples, &mut rng)
}

pub fn find_rest_states_with_rng<R: Rng>(
    poly: &ConvexPolyhedron,
    n_samples: usize,
    rng: &mut R,
) -> Vec<RestState> {
    let groups = poly.unique_face_groups();

    let stable: Vec<(usize, Vec3)> = groups.iter()
        .enumerate()
        .filter_map(|(gi, (n, idxs))| {
            if is_stable_face_group(poly, *n, idxs) {
                Some((gi, *n))
            } else {
                None
            }
        })
        .collect();

    if stable.is_empty() {
        return Vec::new();
    }

    // Monte Carlo: 随机采样上方向 u, 找最反平行的面法向 (该面朝下 = 静止面)
    let mut counts = vec![0usize; stable.len()];
    for _ in 0..n_samples {
        let u = random_sphere_direction(rng);
        let mut best_i = 0;
        let mut best_dot = f32::INFINITY;
        for (i, (_, n)) in stable.iter().enumerate() {
            let d = n.dot(u);
            if d < best_dot {
                best_dot = d;
                best_i = i;
            }
        }
        counts[best_i] += 1;
    }

    stable.iter().enumerate().map(|(i, (gi, n))| {
        let prob = counts[i] as f32 / n_samples as f32;
        let stability = compute_stability(poly, *n, &groups[*gi].1);
        RestState {
            up_direction: -*n,
            face_index: groups[*gi].1[0],
            probability: prob,
            stability,
        }
    }).collect()
}

/// 别名 (兼容外部命名)
pub fn find_rest_states_with_samples(poly: &ConvexPolyhedron, n_samples: usize) -> Vec<RestState> {
    find_rest_states(poly, n_samples)
}

/// 跟踪梯度流: 从起点 u 沿 -∇h 下降到最近的稳定姿态
/// 在支撑函数非光滑点处使用面法向最近邻法收敛
pub fn trace_gradient_flow(poly: &ConvexPolyhedron, start: Vec3, max_iter: usize) -> Vec3 {
    let mut u = start.normalize_or_zero();
    let groups = poly.unique_face_groups();

    for _ in 0..max_iter {
        // 找最反平行的面法向 (该面朝下时, 上方向 u ≈ -n)
        let mut best_n = groups[0].0;
        let mut best_dot = f32::INFINITY;
        for (n, _) in &groups {
            let d = n.dot(u);
            if d < best_dot {
                best_dot = d;
                best_n = *n;
            }
        }
        let target = -best_n;
        let diff = target - u;
        if diff.length_squared() < GRADIENT_TOL * GRADIENT_TOL {
            return target;
        }
        u = (u + diff * GRADIENT_STEP).normalize_or_zero();
    }
    u
}

/// 稳定性度量: 质心到面的距离 / 面的特征尺寸
pub fn compute_stability(poly: &ConvexPolyhedron, n: Vec3, face_indices: &[usize]) -> f32 {
    let centroid = poly.centroid();
    let face_vertex = poly.vertices[poly.faces[face_indices[0]][0]];
    let d_plane = n.dot(face_vertex);
    let dist = d_plane - n.dot(centroid);

    let total_area: f32 = face_indices.iter()
        .map(|&fi| poly.face_area(&poly.faces[fi]))
        .sum();
    let char_size = total_area.sqrt();

    if dist < 1e-9 { 0.0 } else { dist / char_size }
}

// ============================================================
// Inverse Design
// ============================================================

/// 逆设计: 调整顶点位置使目标面达到目标概率
/// 沿面法向缩放顶点 (增大该面的"突出度"以提高概率)
pub fn inverse_design(
    poly: &mut ConvexPolyhedron,
    target_face: usize,
    target_prob: f32,
    iterations: usize,
) -> f32 {
    let mut rng = StdRng::seed_from_u64(123);
    let n_samples = 500;

    for _ in 0..iterations {
        let rest = find_rest_states_with_rng(poly, n_samples, &mut rng);
        let current_prob = rest.iter()
            .find(|r| r.face_index == target_face)
            .map(|r| r.probability)
            .unwrap_or(0.0);

        let err = target_prob - current_prob;
        if err.abs() < 0.02 {
            return current_prob;
        }

        // 沿目标面法向移动顶点
        let target_n = poly.face_normal(&poly.faces[target_face]);
        let delta = err * 0.1;
        for &vi in &poly.faces[target_face] {
            poly.vertices[vi] += target_n * delta;
        }
    }

    let rest = find_rest_states_with_rng(poly, n_samples, &mut rng);
    rest.iter()
        .find(|r| r.face_index == target_face)
        .map(|r| r.probability)
        .unwrap_or(0.0)
}

// ============================================================
// Convex Hull
// ============================================================

/// 凸包构造 (暴力 O(n⁴), 仅用于小点集)
pub fn convex_hull(points: &[Vec3]) -> ConvexPolyhedron {
    let n = points.len();
    if n < 4 {
        return ConvexPolyhedron { vertices: points.to_vec(), faces: vec![] };
    }

    let mut faces = Vec::new();

    for i in 0..n {
        for j in (i + 1)..n {
            for k in (j + 1)..n {
                let a = points[i];
                let b = points[j];
                let c = points[k];
                let normal = (b - a).cross(c - a);
                if normal.length_squared() < 1e-12 {
                    continue;
                }
                let normal = normal.normalize();

                let mut side = 0i32;
                let mut consistent = true;
                for l in 0..n {
                    if l == i || l == j || l == k {
                        continue;
                    }
                    let d = (points[l] - a).dot(normal);
                    if d > 1e-9 {
                        if side == -1 { consistent = false; break; }
                        side = 1;
                    } else if d < -1e-9 {
                        if side == 1 { consistent = false; break; }
                        side = -1;
                    }
                }

                if consistent && side != 0 {
                    if side < 0 {
                        faces.push([i, j, k]);
                    } else {
                        faces.push([i, k, j]);
                    }
                }
            }
        }
    }

    ConvexPolyhedron {
        vertices: points.to_vec(),
        faces,
    }
}

// ============================================================
// Helpers
// ============================================================

/// Marsaglia 算法: 均匀采样单位球面方向
pub fn random_sphere_direction<R: Rng>(rng: &mut R) -> Vec3 {
    let mut u1: f32;
    let mut u2: f32;
    loop {
        u1 = rng.gen::<f32>();
        u2 = rng.gen::<f32>();
        if u1 > 1e-6 && u1 < 1.0 - 1e-6 {
            break;
        }
    }
    let theta = 2.0 * std::f32::consts::PI * u2;
    let z = 1.0 - 2.0 * u1;
    let r = (1.0 - z * z).max(0.0).sqrt();
    Vec3::new(r * theta.cos(), r * theta.sin(), z)
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cube_volume() {
        let cube = ConvexPolyhedron::cube(1.0);
        let v = cube.volume();
        assert!((v - 8.0).abs() < 1e-5, "cube volume = {}, expected 8", v);
    }

    #[test]
    fn test_tetrahedron_volume() {
        let tet = ConvexPolyhedron::tetrahedron(1.0);
        let v = tet.volume();
        assert!((v - 8.0 / 3.0).abs() < 1e-5, "tet volume = {}, expected 8/3", v);
    }

    #[test]
    fn test_octahedron_volume() {
        let oct = ConvexPolyhedron::octahedron(1.0);
        let v = oct.volume();
        assert!((v - 4.0 / 3.0).abs() < 1e-5, "oct volume = {}, expected 4/3", v);
    }

    #[test]
    fn test_cube_centroid() {
        let cube = ConvexPolyhedron::cube(1.0);
        let c = cube.centroid();
        assert!(c.length() < 1e-5, "cube centroid = {:?}, expected origin", c);
    }

    #[test]
    fn test_support_function() {
        let cube = ConvexPolyhedron::cube(1.0);
        let h = cube.support(Vec3::new(1.0, 0.0, 0.0));
        assert!((h - 1.0).abs() < 1e-6, "cube support along +x = {}, expected 1", h);
    }

    #[test]
    fn test_support_gradient() {
        let cube = ConvexPolyhedron::cube(1.0);
        let g = cube.support_tangent_gradient(Vec3::new(1.0, 0.0, 0.0));
        assert!(g.x.abs() < 1e-6, "tangent gradient x = {}, expected ~0", g.x);
    }

    #[test]
    fn test_critical_points_cube() {
        let cube = ConvexPolyhedron::cube(1.0);
        let cps = find_critical_points(&cube);
        let maxima = cps.iter().filter(|c| c.kind == CriticalKind::Maximum).count();
        let minima = cps.iter().filter(|c| c.kind == CriticalKind::Minimum).count();
        assert_eq!(maxima, 6, "cube maxima = {}", maxima);
        assert_eq!(minima, 8, "cube minima = {}", minima);
    }

    #[test]
    fn test_stable_faces_cube() {
        let cube = ConvexPolyhedron::cube(1.0);
        let groups = cube.unique_face_groups();
        assert_eq!(groups.len(), 6, "cube face groups = {}", groups.len());
        for (n, idxs) in &groups {
            assert!(is_stable_face_group(&cube, *n, idxs), "cube face should be stable");
        }
    }

    #[test]
    fn test_rest_states_cube() {
        let cube = ConvexPolyhedron::cube(1.0);
        let rests = find_rest_states(&cube, 1000);
        assert_eq!(rests.len(), 6, "cube rest states = {}, expected 6", rests.len());
        for r in &rests {
            assert!((r.probability - 1.0 / 6.0).abs() < 0.05,
                "cube face {} prob = {}, expected ~1/6", r.face_index, r.probability);
        }
    }

    #[test]
    fn test_gradient_flow_convergence() {
        let cube = ConvexPolyhedron::cube(1.0);
        let start = Vec3::new(0.3, 0.5, 0.8).normalize();
        let target = trace_gradient_flow(&cube, start, 200);
        let max_comp = target.x.abs().max(target.y.abs()).max(target.z.abs());
        assert!(max_comp > 0.95, "gradient flow target = {:?}, should be near axis", target);
    }

    #[test]
    fn test_convex_hull_simple() {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ];
        let hull = convex_hull(&points);
        assert_eq!(hull.faces.len(), 4, "tet hull faces = {}", hull.faces.len());
    }

    #[test]
    fn test_convex_hull_cube() {
        let mut points = Vec::new();
        for x in &[-1.0, 1.0] {
            for y in &[-1.0, 1.0] {
                for z in &[-1.0, 1.0] {
                    points.push(Vec3::new(*x, *y, *z));
                }
            }
        }
        points.push(Vec3::new(0.0, 0.0, 0.0));
        points.push(Vec3::new(0.5, 0.5, 0.5));
        let hull = convex_hull(&points);
        assert!(hull.faces.len() >= 12, "cube hull faces = {}, expected >= 12", hull.faces.len());
    }

    #[test]
    fn test_dice_statistics() {
        let cube = ConvexPolyhedron::cube(1.0);
        let rests = find_rest_states(&cube, 1000);
        let total: f32 = rests.iter().map(|r| r.probability).sum();
        assert!((total - 1.0).abs() < 0.01, "total prob = {}, expected 1.0", total);
    }

    #[test]
    fn test_point_in_triangle() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(1.0, 0.0, 0.0);
        let c = Vec3::new(0.0, 1.0, 0.0);
        let n = Vec3::new(0.0, 0.0, 1.0);
        assert!(point_in_triangle(Vec3::new(0.25, 0.25, 0.0), a, b, c, n));
        assert!(!point_in_triangle(Vec3::new(0.9, 0.9, 0.0), a, b, c, n));
    }

    #[test]
    fn test_random_sphere_direction() {
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..100 {
            let u = random_sphere_direction(&mut rng);
            let len = u.length();
            assert!((len - 1.0).abs() < 1e-4, "sphere dir len = {}", len);
        }
    }

    #[test]
    fn test_tetrahedron_rest_states() {
        let tet = ConvexPolyhedron::tetrahedron(1.0);
        let rests = find_rest_states(&tet, 1000);
        assert_eq!(rests.len(), 4, "tet rest states = {}, expected 4", rests.len());
        for r in &rests {
            assert!((r.probability - 0.25).abs() < 0.08,
                "tet face {} prob = {}, expected ~0.25", r.face_index, r.probability);
        }
    }

    #[test]
    fn test_octahedron_rest_states() {
        let oct = ConvexPolyhedron::octahedron(1.0);
        let rests = find_rest_states(&oct, 1000);
        assert_eq!(rests.len(), 8, "oct rest states = {}, expected 8", rests.len());
    }

    #[test]
    fn test_face_groups_merge_coplanar() {
        let cube = ConvexPolyhedron::cube(1.0);
        let groups = cube.unique_face_groups();
        assert_eq!(groups.len(), 6, "should merge 12 triangles into 6 groups, got {}", groups.len());
        for (_, idxs) in &groups {
            assert_eq!(idxs.len(), 2, "each cube face group should have 2 triangles");
        }
    }
}
