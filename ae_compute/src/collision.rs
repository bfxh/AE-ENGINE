//! Collision Detection — GJK + EPA 凸包碰撞检测
//!
//! 基于:
//! - Gilbert, Johnson, Keerthi. "A Fast Procedure for Computing the Distance
//!   between Complex Objects in Three Dimensions." Journal of Guidance (1988).
//! - van den Bergen. "A Fast and Robust GJK Implementation for Collision
//!   Detection of Convex Objects." JGT 1999.
//! - Ericson. Real-Time Collision Detection. Morgan Kaufmann 2005. Ch 5.
//!
//! 核心思想:
//! 1. GJK 用 Minkowski Difference (MD) 检测两个凸形状相交
//!    - MD = { a - b : a ∈ A, b ∈ B }
//!    - A ∩ B ≠ ∅ ⟺ origin ∈ MD
//!    - 用 support function 隐式查询 MD (无需显式构造)
//! 2. EPA 在相交时扩展 GJK 的 simplex 为多面体
//!    - 找离原点最近的面
//!    - 沿该面法线方向 support 点扩展多面体
//!    - 收敛时返回 (穿透深度, 接触法线)
//!
//! 优点:
//! - 通用: 只需 support function, 支持任意凸形状
//! - 高效: GJK 通常 2-4 次迭代收敛
//! - 鲁棒: EPA 提供精确穿透信息
//!
//! 局限:
//! - 仅凸形状 (凹形状需分解为凸子块)
//! - EPA 不提供接触点对 (仅穿透深度+法线)

use glam::{Vec3, Quat};

const GJK_MAX_ITER: usize = 64;
const EPA_MAX_ITER: usize = 64;
const EPA_TOLERANCE: f32 = 1e-6;

// ============================================================
// Collider trait 和形状
// ============================================================

/// 凸形状 trait: 提供 support function 和质心
pub trait Collider {
    /// 返回形状上沿 direction 方向最远的点
    fn support(&self, direction: Vec3) -> Vec3;
    /// 返回质心 (用于 GJK 初始方向)
    fn center(&self) -> Vec3;
}

#[derive(Debug, Clone, Copy)]
pub struct SphereCollider {
    pub center: Vec3,
    pub radius: f32,
}

impl Collider for SphereCollider {
    #[inline]
    fn support(&self, direction: Vec3) -> Vec3 {
        let dir = if direction.length_squared() > 1e-12 {
            direction.normalize()
        } else {
            Vec3::new(1.0, 0.0, 0.0)
        };
        self.center + dir * self.radius
    }
    #[inline]
    fn center(&self) -> Vec3 { self.center }
}

#[derive(Debug, Clone, Copy)]
pub struct BoxCollider {
    pub center: Vec3,
    pub half_extents: Vec3,
    pub rotation: Quat,
}

impl BoxCollider {
    pub fn new(center: Vec3, half_extents: Vec3) -> Self {
        Self { center, half_extents, rotation: Quat::IDENTITY }
    }
    pub fn with_rotation(mut self, rot: Quat) -> Self {
        self.rotation = rot;
        self
    }
}

impl Collider for BoxCollider {
    #[inline]
    fn support(&self, direction: Vec3) -> Vec3 {
        let local_dir = self.rotation.inverse() * direction;
        let local_pt = Vec3::new(
            if local_dir.x > 0.0 { self.half_extents.x } else { -self.half_extents.x },
            if local_dir.y > 0.0 { self.half_extents.y } else { -self.half_extents.y },
            if local_dir.z > 0.0 { self.half_extents.z } else { -self.half_extents.z },
        );
        self.center + self.rotation * local_pt
    }
    #[inline]
    fn center(&self) -> Vec3 { self.center }
}

#[derive(Debug, Clone, Copy)]
pub struct CapsuleCollider {
    pub a: Vec3,
    pub b: Vec3,
    pub radius: f32,
}

impl Collider for CapsuleCollider {
    #[inline]
    fn support(&self, direction: Vec3) -> Vec3 {
        let dir = if direction.length_squared() > 1e-12 {
            direction.normalize()
        } else {
            Vec3::new(1.0, 0.0, 0.0)
        };
        let dot_a = self.a.dot(direction);
        let dot_b = self.b.dot(direction);
        let end = if dot_a > dot_b { self.a } else { self.b };
        end + dir * self.radius
    }
    #[inline]
    fn center(&self) -> Vec3 { (self.a + self.b) * 0.5 }
}

#[derive(Debug, Clone)]
pub struct ConvexHullCollider {
    pub points: Vec<Vec3>,
    pub centroid: Vec3,
}

impl ConvexHullCollider {
    pub fn new(points: Vec<Vec3>) -> Self {
        let centroid = if points.is_empty() {
            Vec3::ZERO
        } else {
            points.iter().sum::<Vec3>() / points.len() as f32
        };
        Self { points, centroid }
    }
}

impl Collider for ConvexHullCollider {
    #[inline]
    fn support(&self, direction: Vec3) -> Vec3 {
        let mut best = self.points[0];
        let mut best_dot = best.dot(direction);
        for &p in &self.points[1..] {
            let d = p.dot(direction);
            if d > best_dot {
                best_dot = d;
                best = p;
            }
        }
        best
    }
    #[inline]
    fn center(&self) -> Vec3 { self.centroid }
}

#[derive(Debug, Clone, Copy)]
pub struct TetrahedronCollider {
    pub vertices: [Vec3; 4],
}

impl Collider for TetrahedronCollider {
    #[inline]
    fn support(&self, direction: Vec3) -> Vec3 {
        let mut best = self.vertices[0];
        let mut best_dot = best.dot(direction);
        for &p in &self.vertices[1..] {
            let d = p.dot(direction);
            if d > best_dot {
                best_dot = d;
                best = p;
            }
        }
        best
    }
    #[inline]
    fn center(&self) -> Vec3 {
        self.vertices.iter().sum::<Vec3>() * 0.25
    }
}

// ============================================================
// 接触信息
// ============================================================

#[derive(Debug, Clone, Copy, Default)]
pub struct ContactInfo {
    pub intersecting: bool,
    /// 穿透深度 (相交时 > 0, 不相交时 = 0)
    pub penetration_depth: f32,
    /// 接触法线 (从 A 指向 B, 即 A 应被推开的方向)
    pub normal: Vec3,
}

// ============================================================
// 辅助函数
// ============================================================

#[inline]
fn triple_product(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    // (a × b) × c = b * (a·c) - a * (b·c)
    let ac = a.dot(c);
    let bc = b.dot(c);
    b * ac - a * bc
}

#[inline]
fn md_support<A: Collider, B: Collider>(a: &A, b: &B, d: Vec3) -> Vec3 {
    a.support(d) - b.support(-d)
}

// ============================================================
// GJK 算法
// ============================================================

// simplex = [B, A], A 是最新点 (索引 1)
fn do_simplex_line(simplex: &mut Vec<Vec3>, d: &mut Vec3) -> bool {
    let a = simplex[1];
    let b = simplex[0];
    let ab = b - a;
    let ao = -a;

    if ab.dot(ao) > 0.0 {
        *d = triple_product(ab, ao, ab);
        if d.length_squared() < 1e-16 {
            return true; // 退化: 原点在线段上
        }
    } else {
        *simplex = vec![a];
        *d = ao;
    }
    false
}

// simplex = [C, B, A], A 是最新点 (索引 2)
fn do_simplex_triangle(simplex: &mut Vec<Vec3>, d: &mut Vec3) -> bool {
    let a = simplex[2];
    let b = simplex[1];
    let c = simplex[0];
    let ab = b - a;
    let ac = c - a;
    let ao = -a;
    let abc = ab.cross(ac);

    if abc.cross(ac).dot(ao) > 0.0 {
        // 原点在 AC 边外侧
        if ac.dot(ao) > 0.0 {
            *simplex = vec![c, a];
            *d = triple_product(ac, ao, ac);
            if d.length_squared() < 1e-16 { return true; }
        } else {
            *simplex = vec![a];
            *d = ao;
        }
    } else if ab.cross(abc).dot(ao) > 0.0 {
        // 原点在 AB 边外侧, 退化为 line [B, A]
        *simplex = vec![b, a];
        return do_simplex_line(simplex, d);
    } else {
        // 原点在 ABC 面的法线方向
        if abc.dot(ao) > 0.0 {
            *d = abc;
        } else {
            *d = -abc;
            *simplex = vec![b, c, a]; // 翻转绕序
        }
    }
    false
}

// simplex = [D, C, B, A], A 是最新点 (索引 3)
fn do_simplex_tetrahedron(simplex: &mut Vec<Vec3>, d: &mut Vec3) -> bool {
    let a = simplex[3];
    let b = simplex[2];
    let c = simplex[1];
    let d_pt = simplex[0];
    let ab = b - a;
    let ac = c - a;
    let ad = d_pt - a;
    let ao = -a;

    let mut abc = ab.cross(ac);
    let mut acd = ac.cross(ad);
    let mut adb = ad.cross(ab);

    // 调整法线朝外 (远离对面顶点)
    if abc.dot(ad) > 0.0 { abc = -abc; }
    if acd.dot(ab) > 0.0 { acd = -acd; }
    if adb.dot(ac) > 0.0 { adb = -adb; }

    if abc.dot(ao) > 0.0 {
        // 原点在 ABC 面外, 丢弃 D
        *simplex = vec![c, b, a];
        return do_simplex_triangle(simplex, d);
    } else if acd.dot(ao) > 0.0 {
        // 原点在 ACD 面外, 丢弃 B
        *simplex = vec![d_pt, c, a];
        return do_simplex_triangle(simplex, d);
    } else if adb.dot(ao) > 0.0 {
        // 原点在 ADB 面外, 丢弃 C
        *simplex = vec![b, d_pt, a];
        return do_simplex_triangle(simplex, d);
    } else {
        // 原点在四面体内 -> 相交
        true
    }
}

fn do_simplex(simplex: &mut Vec<Vec3>, d: &mut Vec3) -> bool {
    match simplex.len() {
        2 => do_simplex_line(simplex, d),
        3 => do_simplex_triangle(simplex, d),
        4 => do_simplex_tetrahedron(simplex, d),
        _ => false,
    }
}

fn gjk<A: Collider, B: Collider>(a: &A, b: &B) -> Option<[Vec3; 4]> {
    let mut initial_dir = b.center() - a.center();
    if initial_dir.length_squared() < 1e-12 {
        initial_dir = Vec3::new(1.0, 0.0, 0.0);
    }

    let s1 = md_support(a, b, initial_dir);
    let mut simplex: Vec<Vec3> = vec![s1];
    let mut d = -s1;

    for _ in 0..GJK_MAX_ITER {
        let new_pt = md_support(a, b, d);
        if new_pt.dot(d) < -1e-7 {
            return None; // 不相交
        }
        simplex.push(new_pt);

        if do_simplex(&mut simplex, &mut d) {
            // 原点在 simplex 内
            return Some(ensure_tetrahedron(a, b, &simplex, d));
        }
    }
    None
}

// 确保 GJK 返回 4 点四面体 (处理原点在线/三角形上的退化情况)
fn ensure_tetrahedron<A: Collider, B: Collider>(
    a: &A, b: &B, simplex: &[Vec3], d: Vec3
) -> [Vec3; 4] {
    if simplex.len() == 4 {
        [simplex[0], simplex[1], simplex[2], simplex[3]]
    } else if simplex.len() == 3 {
        let extra = md_support(a, b, d);
        [simplex[0], simplex[1], simplex[2], extra]
    } else if simplex.len() == 2 {
        let d2 = if d.length_squared() < 1e-12 { Vec3::new(0.0, 1.0, 0.0) } else { d };
        let extra1 = md_support(a, b, d2);
        let d3 = d2.cross(Vec3::new(1.0, 0.0, 0.0));
        let d3 = if d3.length_squared() < 1e-12 { d2.cross(Vec3::new(0.0, 1.0, 0.0)) } else { d3 };
        let extra2 = md_support(a, b, d3);
        [simplex[0], simplex[1], extra1, extra2]
    } else {
        // 单点 (不应发生), 用任意方向构造
        let e1 = md_support(a, b, Vec3::new(1.0, 0.0, 0.0));
        let e2 = md_support(a, b, Vec3::new(0.0, 1.0, 0.0));
        let e3 = md_support(a, b, Vec3::new(0.0, 0.0, 1.0));
        [simplex[0], e1, e2, e3]
    }
}

// ============================================================
// EPA 算法
// ============================================================

fn epa<A: Collider, B: Collider>(a: &A, b: &B, initial_simplex: [Vec3; 4]) -> ContactInfo {
    let mut polytope: Vec<Vec3> = initial_simplex.to_vec();
    let mut faces: Vec<(usize, usize, usize)> = vec![
        (0, 1, 2),
        (0, 3, 1),
        (0, 2, 3),
        (1, 3, 2),
    ];

    fix_face_outward(&polytope, &mut faces);

    for _ in 0..EPA_MAX_ITER {
        let (normal, distance, _face_idx) = find_closest_face(&polytope, &faces);

        if normal.length_squared() < 1e-16 {
            break;
        }

        let normal_unit = normal.normalize();
        let support_pt = md_support(a, b, normal_unit);
        let support_dist = support_pt.dot(normal_unit);

        if support_dist - distance < EPA_TOLERANCE {
            return ContactInfo {
                intersecting: true,
                penetration_depth: distance,
                normal: normal_unit,
            };
        }

        let new_idx = polytope.len();
        polytope.push(support_pt);
        expand_polytope(&polytope, &mut faces, new_idx);

        if faces.is_empty() {
            break;
        }
    }

    // 超时, 返回当前最佳
    let (normal, distance, _) = find_closest_face(&polytope, &faces);
    let normal_unit = if normal.length_squared() > 1e-16 {
        normal.normalize()
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    ContactInfo {
        intersecting: true,
        penetration_depth: distance,
        normal: normal_unit,
    }
}

fn fix_face_outward(polytope: &[Vec3], faces: &mut Vec<(usize, usize, usize)>) {
    let center = polytope.iter().sum::<Vec3>() / polytope.len() as f32;
    for face in faces.iter_mut() {
        let a = polytope[face.0];
        let b = polytope[face.1];
        let c = polytope[face.2];
        let normal = (b - a).cross(c - a);
        let face_center = (a + b + c) / 3.0;
        if normal.dot(face_center - center) < 0.0 {
            *face = (face.0, face.2, face.1); // 翻转绕序
        }
    }
}

fn find_closest_face(polytope: &[Vec3], faces: &[(usize, usize, usize)]) -> (Vec3, f32, usize) {
    let mut best_dist = f32::INFINITY;
    let mut best_normal = Vec3::new(0.0, 1.0, 0.0);
    let mut best_idx = 0;

    for (i, face) in faces.iter().enumerate() {
        let a = polytope[face.0];
        let b = polytope[face.1];
        let c = polytope[face.2];
        let normal = (b - a).cross(c - a);
        let n_len = normal.length();
        if n_len < 1e-12 { continue; }
        let n_unit = normal / n_len;
        // 原点到面的距离 (法线朝外时 a·n_unit > 0)
        let dist = a.dot(n_unit).abs();
        if dist < best_dist {
            best_dist = dist;
            best_normal = n_unit;
            best_idx = i;
        }
    }

    (best_normal, best_dist, best_idx)
}

fn expand_polytope(
    polytope: &Vec<Vec3>,
    faces: &mut Vec<(usize, usize, usize)>,
    new_idx: usize,
) {
    let support_pt = polytope[new_idx];

    // polytope 中心 (不含新点)
    let center = polytope[..new_idx].iter().sum::<Vec3>() / new_idx as f32;

    let mut edges: Vec<(usize, usize)> = Vec::new();
    let mut visible: Vec<usize> = Vec::new();

    for (i, face) in faces.iter().enumerate() {
        let a = polytope[face.0];
        let b = polytope[face.1];
        let c = polytope[face.2];
        let normal = (b - a).cross(c - a);
        let face_center = (a + b + c) / 3.0;
        let outward = if normal.dot(face_center - center) < 0.0 { -normal } else { normal };

        if outward.dot(support_pt) > 0.0 {
            visible.push(i);
            edges.push((face.0, face.1));
            edges.push((face.1, face.2));
            edges.push((face.2, face.0));
        }
    }

    // 从后往前删除可见面, 保持索引
    visible.sort_unstable_by(|a, b| b.cmp(a));
    for &i in &visible {
        faces.remove(i);
    }

    // 边界边 = 在 edges 中没有反向边的边
    // 新面 = (boundary_edge.0, boundary_edge.1, new_idx)
    for e in &edges {
        let reverse = (e.1, e.0);
        let has_reverse = edges.iter().any(|&other| other == reverse);
        if !has_reverse {
            faces.push((e.0, e.1, new_idx));
        }
    }
}

// ============================================================
// 公共 API
// ============================================================

/// GJK 相交检测 (无穿透信息)
pub fn intersect<A: Collider, B: Collider>(a: &A, b: &B) -> bool {
    gjk(a, b).is_some()
}

/// GJK + EPA 完整碰撞检测 (含穿透信息)
pub fn collide<A: Collider, B: Collider>(a: &A, b: &B) -> ContactInfo {
    match gjk(a, b) {
        Some(simplex) => epa(a, b, simplex),
        None => ContactInfo {
            intersecting: false,
            penetration_depth: 0.0,
            normal: Vec3::ZERO,
        },
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sphere_sphere_intersect() {
        let a = SphereCollider { center: Vec3::new(0.0, 0.0, 0.0), radius: 1.0 };
        let b = SphereCollider { center: Vec3::new(1.5, 0.0, 0.0), radius: 1.0 };
        let info = collide(&a, &b);
        assert!(info.intersecting);
        // 穿透深度 = 2*radius - distance = 2 - 1.5 = 0.5
        assert!((info.penetration_depth - 0.5).abs() < 0.1,
            "penetration_depth = {}", info.penetration_depth);
        // 法线从 A 指向 B (正 x)
        assert!(info.normal.x > 0.0, "normal = {:?}", info.normal);
    }

    #[test]
    fn test_sphere_sphere_no_intersect() {
        let a = SphereCollider { center: Vec3::new(0.0, 0.0, 0.0), radius: 1.0 };
        let b = SphereCollider { center: Vec3::new(3.0, 0.0, 0.0), radius: 1.0 };
        let info = collide(&a, &b);
        assert!(!info.intersecting);
    }

    #[test]
    fn test_box_box_intersect() {
        let a = BoxCollider::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let b = BoxCollider::new(Vec3::new(1.5, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let info = collide(&a, &b);
        assert!(info.intersecting);
        // 穿透深度应该接近 0.5
        assert!((info.penetration_depth - 0.5).abs() < 0.15,
            "penetration_depth = {}", info.penetration_depth);
        assert!(info.normal.x > 0.0);
    }

    #[test]
    fn test_box_box_no_intersect() {
        let a = BoxCollider::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let b = BoxCollider::new(Vec3::new(5.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let info = collide(&a, &b);
        assert!(!info.intersecting);
    }

    #[test]
    fn test_box_sphere_intersect() {
        let a = BoxCollider::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let b = SphereCollider { center: Vec3::new(1.5, 0.0, 0.0), radius: 0.8 };
        let info = collide(&a, &b);
        assert!(info.intersecting);
        assert!(info.normal.x > 0.0);
    }

    #[test]
    fn test_capsule_sphere_intersect() {
        let a = CapsuleCollider {
            a: Vec3::new(0.0, -1.0, 0.0),
            b: Vec3::new(0.0, 1.0, 0.0),
            radius: 0.5,
        };
        let b = SphereCollider { center: Vec3::new(0.8, 0.0, 0.0), radius: 0.5 };
        let info = collide(&a, &b);
        assert!(info.intersecting);
        assert!(info.normal.x > 0.0);
    }

    #[test]
    fn test_capsule_capsule_intersect() {
        let a = CapsuleCollider {
            a: Vec3::new(-1.0, 0.0, 0.0),
            b: Vec3::new(1.0, 0.0, 0.0),
            radius: 0.5,
        };
        let b = CapsuleCollider {
            a: Vec3::new(0.0, -1.0, 0.0),
            b: Vec3::new(0.0, 1.0, 0.0),
            radius: 0.5,
        };
        let info = collide(&a, &b);
        assert!(info.intersecting);
    }

    #[test]
    fn test_convex_hull_intersect() {
        let hull_a = ConvexHullCollider::new(vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(1.0, 1.0, 2.0),
        ]);
        let hull_b = ConvexHullCollider::new(vec![
            Vec3::new(1.5, 0.5, 0.5),
            Vec3::new(3.5, 0.5, 0.5),
            Vec3::new(2.5, 2.5, 0.5),
            Vec3::new(2.5, 1.5, 2.5),
        ]);
        let info = collide(&hull_a, &hull_b);
        assert!(info.intersecting);
    }

    #[test]
    fn test_tetrahedron_intersect() {
        let a = TetrahedronCollider {
            vertices: [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(2.0, 0.0, 0.0),
                Vec3::new(1.0, 2.0, 0.0),
                Vec3::new(1.0, 1.0, 2.0),
            ],
        };
        let b = TetrahedronCollider {
            vertices: [
                Vec3::new(0.5, 0.5, 0.5),
                Vec3::new(2.5, 0.5, 0.5),
                Vec3::new(1.5, 2.5, 0.5),
                Vec3::new(1.5, 1.5, 2.5),
            ],
        };
        let info = collide(&a, &b);
        assert!(info.intersecting);
    }

    #[test]
    fn test_box_box_full_overlap() {
        let a = BoxCollider::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let b = BoxCollider::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.5, 0.5, 0.5));
        let info = collide(&a, &b);
        assert!(info.intersecting);
        // 完全包含, 穿透深度应接近 1.0 (盒子半边长)
        assert!(info.penetration_depth > 0.9,
            "penetration = {}", info.penetration_depth);
    }

    #[test]
    fn test_normal_direction() {
        // A 在原点, B 在 +x 方向, 法线应朝 +x
        let a = SphereCollider { center: Vec3::new(0.0, 0.0, 0.0), radius: 1.0 };
        let b = SphereCollider { center: Vec3::new(1.0, 0.0, 0.0), radius: 1.0 };
        let info = collide(&a, &b);
        assert!(info.intersecting);
        assert!(info.normal.x > 0.9, "normal.x = {}", info.normal.x);
        assert!(info.normal.y.abs() < 0.1);
        assert!(info.normal.z.abs() < 0.1);
    }

    #[test]
    fn test_rotated_box_intersect() {
        let a = BoxCollider::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.5, 1.0));
        let b = BoxCollider::new(Vec3::new(1.5, 0.0, 0.0), Vec3::new(1.0, 0.5, 1.0))
            .with_rotation(Quat::from_rotation_z(45.0_f32.to_radians()));
        let info = collide(&a, &b);
        assert!(info.intersecting);
    }

    #[test]
    fn test_intersect_function() {
        let a = SphereCollider { center: Vec3::new(0.0, 0.0, 0.0), radius: 1.0 };
        let b = SphereCollider { center: Vec3::new(1.5, 0.0, 0.0), radius: 1.0 };
        assert!(intersect(&a, &b));

        let c = SphereCollider { center: Vec3::new(5.0, 0.0, 0.0), radius: 1.0 };
        assert!(!intersect(&a, &c));
    }

    #[test]
    fn test_separated_spheres_along_y() {
        let a = SphereCollider { center: Vec3::new(0.0, 0.0, 0.0), radius: 1.0 };
        let b = SphereCollider { center: Vec3::new(0.0, 3.0, 0.0), radius: 1.0 };
        let info = collide(&a, &b);
        assert!(!info.intersecting);
    }

    #[test]
    fn test_deeply_penetrating() {
        let a = SphereCollider { center: Vec3::new(0.0, 0.0, 0.0), radius: 2.0 };
        let b = SphereCollider { center: Vec3::new(0.1, 0.0, 0.0), radius: 2.0 };
        let info = collide(&a, &b);
        assert!(info.intersecting);
        // 穿透深度 ≈ 2*radius - distance = 4 - 0.1 = 3.9
        assert!(info.penetration_depth > 3.8,
            "penetration = {}", info.penetration_depth);
    }

    #[test]
    fn test_convex_hull_no_intersect() {
        let hull_a = ConvexHullCollider::new(vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
            Vec3::new(0.5, 0.5, 1.0),
        ]);
        let hull_b = ConvexHullCollider::new(vec![
            Vec3::new(10.0, 10.0, 10.0),
            Vec3::new(11.0, 10.0, 10.0),
            Vec3::new(10.5, 11.0, 10.0),
            Vec3::new(10.5, 10.5, 11.0),
        ]);
        let info = collide(&hull_a, &hull_b);
        assert!(!info.intersecting);
    }

    #[test]
    fn test_box_capsule_intersect() {
        let a = BoxCollider::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let b = CapsuleCollider {
            a: Vec3::new(0.0, 0.0, -2.0),
            b: Vec3::new(0.0, 0.0, 2.0),
            radius: 0.5,
        };
        let info = collide(&a, &b);
        assert!(info.intersecting);
    }

    #[test]
    fn test_normal_direction_y() {
        // A 在原点, B 在 +y 方向, 法线应朝 +y
        let a = BoxCollider::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.5, 0.5, 0.5));
        let b = BoxCollider::new(Vec3::new(0.0, 0.9, 0.0), Vec3::new(0.5, 0.5, 0.5));
        let info = collide(&a, &b);
        assert!(info.intersecting);
        assert!(info.normal.y > 0.8, "normal = {:?}", info.normal);
    }
}
