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
//! 退化处理 (本实现关键):
//! - 当原点落在 simplex 的边/面/顶点上时 (对称形状如球-球、盒-盒常见),
//!   triple_product 退化为零向量。不能直接返回 true (会让 EPA 初始 polytope 退化)。
//!   应选垂直方向继续迭代,直到形成真正包含原点的四面体。

use glam::{Quat, Vec3};

const GJK_MAX_ITER: usize = 128;
const EPA_MAX_ITER: usize = 256;
const EPA_TOLERANCE: f32 = 1e-6;
const DEGEN_EPS: f32 = 1e-12;

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
    fn center(&self) -> Vec3 {
        self.center
    }
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
    fn center(&self) -> Vec3 {
        self.center
    }
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
    fn center(&self) -> Vec3 {
        (self.a + self.b) * 0.5
    }
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
    fn center(&self) -> Vec3 {
        self.centroid
    }
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

/// 返回与 v 垂直的单位向量 (用于逃退化)
/// 选 v 最小分量对应的轴做叉积,保证叉积结果非零
#[inline]
fn perpendicular(v: Vec3) -> Vec3 {
    let ax = v.x.abs();
    let ay = v.y.abs();
    let az = v.z.abs();
    let axis = if ax <= ay && ax <= az {
        Vec3::new(1.0, 0.0, 0.0)
    } else if ay <= az {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        Vec3::new(0.0, 0.0, 1.0)
    };
    v.cross(axis).normalize_or_zero()
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
        if d.length_squared() < DEGEN_EPS {
            // 退化: 原点在线段 AB 上。MD 可能仍有 3D 体积
            // (典型场景: 球-球、盒-盒沿轴对齐时 support 函数返回共线点)
            // 选垂直于 AB 的方向继续搜索,构造真正包含原点的四面体
            *d = perpendicular(ab);
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

    if abc.length_squared() < DEGEN_EPS {
        // 三角形退化 (三点共线), 退化为线段
        *simplex = vec![b, a];
        return do_simplex_line(simplex, d);
    }

    if abc.cross(ac).dot(ao) > 0.0 {
        // 原点在 AC 边外侧
        if ac.dot(ao) > 0.0 {
            *simplex = vec![c, a];
            *d = triple_product(ac, ao, ac);
            if d.length_squared() < DEGEN_EPS {
                // 退化: 原点在 AC 线段上
                *d = perpendicular(ac);
            }
        } else {
            // 原点 past A on AC 线, A 顶点区域
            *simplex = vec![a];
            *d = ao;
        }
    } else if ab.cross(abc).dot(ao) > 0.0 {
        // 原点在 AB 边外侧, 退化为 line [B, A]
        *simplex = vec![b, a];
        return do_simplex_line(simplex, d);
    } else {
        // 原点在 ABC 面的法线方向 (face region)
        if abc.dot(ao) > 0.0 {
            *d = abc;
        } else if abc.dot(ao) < 0.0 {
            *d = -abc;
            *simplex = vec![b, c, a]; // 翻转绕序
        } else {
            // abc.dot(ao) == 0: 原点在 ABC 面上 (退化)
            // 选垂直于 ABC 面的方向继续搜索
            *d = abc.normalize_or_zero();
            if d.length_squared() < DEGEN_EPS {
                *d = perpendicular(ab);
            }
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
    if abc.dot(ad) > 0.0 {
        abc = -abc;
    }
    if acd.dot(ab) > 0.0 {
        acd = -acd;
    }
    if adb.dot(ac) > 0.0 {
        adb = -adb;
    }

    // 用容差判断: dot > eps 表示原点在面外
    // dot 接近 0 表示原点在面上 (退化),也应降维处理
    let face_eps = 1e-7 * (abc.length() + acd.length() + adb.length()).max(1e-12);

    if abc.dot(ao) > face_eps {
        *simplex = vec![c, b, a];
        return do_simplex_triangle(simplex, d);
    }
    if acd.dot(ao) > face_eps {
        *simplex = vec![d_pt, c, a];
        return do_simplex_triangle(simplex, d);
    }
    if adb.dot(ao) > face_eps {
        *simplex = vec![b, d_pt, a];
        return do_simplex_triangle(simplex, d);
    }

    // 原点在所有面"内侧"或某面上
    // 检查是否在某面上 (退化)
    if abc.dot(ao).abs() <= face_eps && abc.length_squared() > DEGEN_EPS {
        // 原点在 ABC 面上,降维
        *simplex = vec![c, b, a];
        return do_simplex_triangle(simplex, d);
    }
    if acd.dot(ao).abs() <= face_eps && acd.length_squared() > DEGEN_EPS {
        *simplex = vec![d_pt, c, a];
        return do_simplex_triangle(simplex, d);
    }
    if adb.dot(ao).abs() <= face_eps && adb.length_squared() > DEGEN_EPS {
        *simplex = vec![b, d_pt, a];
        return do_simplex_triangle(simplex, d);
    }

    // 原点严格在四面体内 -> 相交
    true
}

fn do_simplex(simplex: &mut Vec<Vec3>, d: &mut Vec3) -> bool {
    match simplex.len() {
        2 => do_simplex_line(simplex, d),
        3 => do_simplex_triangle(simplex, d),
        4 => do_simplex_tetrahedron(simplex, d),
        _ => false,
    }
}

/// GJK 退化时的 fallback: 用 4 个轴方向构造四面体
/// 用于 GJK 收敛但未形成包围原点的四面体时 (对称形状、完全包含等)
fn fallback_simplex<A: Collider, B: Collider>(a: &A, b: &B) -> [Vec3; 4] {
    let px = md_support(a, b, Vec3::new(1.0, 0.0, 0.0));
    let nx = md_support(a, b, Vec3::new(-1.0, 0.0, 0.0));
    let py = md_support(a, b, Vec3::new(0.0, 1.0, 0.0));
    let ny = md_support(a, b, Vec3::new(0.0, -1.0, 0.0));
    [px, nx, py, ny]
}

fn gjk<A: Collider, B: Collider>(a: &A, b: &B) -> Option<[Vec3; 4]> {
    let mut initial_dir = b.center() - a.center();
    if initial_dir.length_squared() < 1e-12 {
        initial_dir = Vec3::new(1.0, 0.0, 0.0);
    }
    // 加微小扰动打破对称 (球-球/盒-盒沿轴对齐时 support 共线)
    let perturb = Vec3::new(0.0, 1e-3, 3e-4);
    initial_dir = (initial_dir + perturb).normalize();

    let s1 = md_support(a, b, initial_dir);
    let mut simplex: Vec<Vec3> = vec![s1];
    let mut d = -s1;
    let mut last_new_pt = s1;
    let mut last_d = d;

    for _ in 0..GJK_MAX_ITER {
        let new_pt = md_support(a, b, d);
        if new_pt.dot(d) < -1e-7 {
            return None; // 不相交
        }
        // 检查新点是否已存在 (避免死循环)
        if simplex.iter().any(|&p| (p - new_pt).length_squared() < 1e-16) {
            // support 收敛但 do_simplex 没返回 true,可能是边界相切或完全包含
            // 形状确实相交 (support 没穿过原点), 用 fallback 构造四面体
            return Some(fallback_simplex(a, b));
        }
        simplex.push(new_pt);
        last_new_pt = new_pt;
        last_d = d;

        if do_simplex(&mut simplex, &mut d) {
            return Some(ensure_tetrahedron(a, b, &simplex, d));
        }
    }
    // 迭代用尽: 若最后一次 support 没穿过原点, 认为相交 (退化场景)
    if last_new_pt.dot(last_d) >= -1e-7 {
        return Some(fallback_simplex(a, b));
    }
    None
}

// 确保 GJK 返回 4 点四面体 (处理原点在线/三角形上的退化情况)
// 关键: 必须构造严格包含原点的四面体,否则 EPA 会退化
fn ensure_tetrahedron<A: Collider, B: Collider>(
    a: &A,
    b: &B,
    simplex: &[Vec3],
    d: Vec3,
) -> [Vec3; 4] {
    if simplex.len() == 4 {
        return [simplex[0], simplex[1], simplex[2], simplex[3]];
    }

    if simplex.len() == 3 {
        // 原点在三角形上,加一个垂直方向的点
        let ab = simplex[1] - simplex[2];
        let ac = simplex[0] - simplex[2];
        let normal = ab.cross(ac);
        let dir = if normal.length_squared() > DEGEN_EPS {
            normal.normalize()
        } else if d.length_squared() > DEGEN_EPS {
            d.normalize()
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };
        let extra = md_support(a, b, dir);
        return [simplex[0], simplex[1], simplex[2], extra];
    }

    if simplex.len() == 2 {
        // 原点在线段上,构造垂直于线段的两个方向
        let line_dir = simplex[1] - simplex[0];
        let d2 = if line_dir.length_squared() > DEGEN_EPS {
            perpendicular(line_dir)
        } else if d.length_squared() > DEGEN_EPS {
            d.normalize()
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };
        let extra1 = md_support(a, b, d2);
        // 第二个方向垂直于 (line_dir × d2),形成非共面四面体
        let d3 = line_dir.cross(d2).normalize_or_zero();
        let d3 = if d3.length_squared() > DEGEN_EPS { d3 } else { perpendicular(d2) };
        let extra2 = md_support(a, b, d3);
        return [simplex[0], simplex[1], extra1, extra2];
    }

    // 单点 (不应发生), 用任意方向构造
    let e1 = md_support(a, b, Vec3::new(1.0, 0.0, 0.0));
    let e2 = md_support(a, b, Vec3::new(0.0, 1.0, 0.0));
    let e3 = md_support(a, b, Vec3::new(0.0, 0.0, 1.0));
    [simplex[0], e1, e2, e3]
}

// ============================================================
// EPA 算法
// ============================================================

/// 检测四面体是否退化 (有重复顶点或原点在面上)
fn is_degenerate(simplex: &[Vec3; 4]) -> bool {
    // 重复顶点
    for i in 0..4 {
        for j in (i + 1)..4 {
            if (simplex[i] - simplex[j]).length_squared() < 1e-14 {
                return true;
            }
        }
    }
    // 原点在面上 (距离 ~0)
    let faces = [(0, 1, 2), (0, 3, 1), (0, 2, 3), (1, 3, 2)];
    for (i, j, k) in faces {
        let a = simplex[i];
        let b = simplex[j];
        let c = simplex[k];
        let normal = (b - a).cross(c - a);
        let n_len = normal.length();
        if n_len < 1e-12 {
            return true;
        }
        let dist = (a.dot(normal) / n_len).abs();
        if dist < 1e-6 {
            return true;
        }
    }
    false
}

fn epa<A: Collider, B: Collider>(a: &A, b: &B, initial_simplex: [Vec3; 4]) -> ContactInfo {
    let mut polytope: Vec<Vec3>;
    let mut faces: Vec<(usize, usize, usize)>;

    if is_degenerate(&initial_simplex) {
        // GJK 返回退化 simplex (原点在面/边/顶点上), 用 octahedron 替换
        // 6 轴方向 support 点确保原点严格在 polytope 内部
        polytope = vec![
            md_support(a, b, Vec3::new(1.0, 0.0, 0.0)),  // 0: +x
            md_support(a, b, Vec3::new(-1.0, 0.0, 0.0)), // 1: -x
            md_support(a, b, Vec3::new(0.0, 1.0, 0.0)),  // 2: +y
            md_support(a, b, Vec3::new(0.0, -1.0, 0.0)), // 3: -y
            md_support(a, b, Vec3::new(0.0, 0.0, 1.0)),  // 4: +z
            md_support(a, b, Vec3::new(0.0, 0.0, -1.0)), // 5: -z
        ];
        faces = vec![
            (0, 2, 4),
            (0, 4, 3),
            (0, 3, 5),
            (0, 5, 2),
            (1, 4, 2),
            (1, 3, 4),
            (1, 5, 3),
            (1, 2, 5),
        ];
    } else {
        polytope = initial_simplex.to_vec();
        // 检查初始 polytope 是否有重复顶点, 若有则用不同方向重新取点
        dedup_and_fix_polytope(a, b, &mut polytope);
        faces = vec![(0, 1, 2), (0, 3, 1), (0, 2, 3), (1, 3, 2)];
    }

    fix_face_outward(&polytope, &mut faces);

    // 移除退化面 (面积为 0)
    faces.retain(|&f| {
        let a_p = polytope[f.0];
        let b_p = polytope[f.1];
        let c_p = polytope[f.2];
        (b_p - a_p).cross(c_p - a_p).length_squared() > DEGEN_EPS
    });

    if faces.is_empty() {
        // 完全退化, 返回安全值
        return ContactInfo {
            intersecting: true,
            penetration_depth: 0.0,
            normal: Vec3::new(0.0, 1.0, 0.0),
        };
    }

    for iter in 0..EPA_MAX_ITER {
        let (normal, distance, _face_idx) = find_closest_face(&polytope, &faces);
        if normal.length_squared() < 1e-16 || !normal.is_finite() {
            break;
        }
        let normal_unit = normal.normalize();
        let support_pt = md_support(a, b, normal_unit);
        let support_dist = support_pt.dot(normal_unit);
        if !support_dist.is_finite() {
            break;
        }
        if support_dist - distance < EPA_TOLERANCE {
            return ContactInfo {
                intersecting: true,
                penetration_depth: distance,
                normal: normal_unit,
            };
        }
        if polytope.iter().any(|&p| (p - support_pt).length_squared() < 1e-14) {
            break;
        }
        let new_idx = polytope.len();
        polytope.push(support_pt);
        expand_polytope(&polytope, &mut faces, new_idx);
        faces.retain(|&f| {
            let a = polytope[f.0];
            let b = polytope[f.1];
            let c = polytope[f.2];
            let n = (b - a).cross(c - a);
            let n_len = n.length();
            if n_len < 1e-12 {
                return false;
            }
            let d = a.dot(n).abs() / n_len;
            d > 1e-7
        });
        if faces.is_empty() {
            break;
        }
    }

    // 超时, 返回当前最佳
    let (normal, distance, _) = find_closest_face(&polytope, &faces);
    let normal_unit = if normal.length_squared() > 1e-16 && normal.is_finite() {
        normal.normalize()
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    ContactInfo {
        intersecting: true,
        penetration_depth: if distance.is_finite() { distance } else { 0.0 },
        normal: normal_unit,
    }
}

/// 检查并修复 polytope 中的重复顶点
fn dedup_and_fix_polytope<A: Collider, B: Collider>(a: &A, b: &B, polytope: &mut Vec<Vec3>) {
    // 检查是否有重复
    let mut has_dup = false;
    for i in 0..polytope.len() {
        for j in (i + 1)..polytope.len() {
            if (polytope[i] - polytope[j]).length_squared() < 1e-14 {
                has_dup = true;
                break;
            }
        }
        if has_dup {
            break;
        }
    }

    if !has_dup {
        return;
    }

    // 用不同方向重新构造 4 个 support 点
    let dirs = [
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(-1.0, 0.3, 0.0),
        Vec3::new(0.0, 1.0, 0.2),
        Vec3::new(0.2, 0.0, 1.0),
    ];
    polytope.clear();
    for &dir in &dirs {
        polytope.push(md_support(a, b, dir));
    }
}

fn fix_face_outward(polytope: &[Vec3], faces: &mut Vec<(usize, usize, usize)>) {
    // 原点在 polytope 内部, 外法线满足 vertex·normal > 0
    for face in faces.iter_mut() {
        let a = polytope[face.0];
        let b = polytope[face.1];
        let c = polytope[face.2];
        let normal = (b - a).cross(c - a);
        if normal.dot(a) < 0.0 {
            *face = (face.0, face.2, face.1); // 翻转绕序使法线朝外
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
        if n_len < 1e-12 {
            continue;
        }
        let mut n_unit = normal / n_len;
        // 原点在 polytope 内部, 外法线满足 a·n > 0
        let mut dist = a.dot(n_unit);
        if dist < 0.0 {
            n_unit = -n_unit;
            dist = -dist;
        }
        if dist < best_dist {
            best_dist = dist;
            best_normal = n_unit;
            best_idx = i;
        }
    }

    (best_normal, best_dist, best_idx)
}

fn expand_polytope(polytope: &Vec<Vec3>, faces: &mut Vec<(usize, usize, usize)>, new_idx: usize) {
    let support_pt = polytope[new_idx];

    let mut edges: Vec<(usize, usize)> = Vec::new();
    let mut visible: Vec<usize> = Vec::new();

    for (i, face) in faces.iter().enumerate() {
        let a = polytope[face.0];
        let b = polytope[face.1];
        let c = polytope[face.2];
        let normal = (b - a).cross(c - a);
        // 原点在内部, 外法线满足 a·n > 0
        let outward = if normal.dot(a) < 0.0 { -normal } else { normal };

        // 可见面: support_pt 超过面所在平面 (而非仅超过原点)
        // outward·a = 面到原点的 (未归一化) 距离
        // outward·support_pt > outward·a 表示 support_pt 在面外侧
        if outward.dot(support_pt) > outward.dot(a) {
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
        None => ContactInfo { intersecting: false, penetration_depth: 0.0, normal: Vec3::ZERO },
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
        assert!(
            (info.penetration_depth - 0.5).abs() < 0.1,
            "penetration_depth = {}",
            info.penetration_depth
        );
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
        assert!(
            (info.penetration_depth - 0.5).abs() < 0.15,
            "penetration_depth = {}",
            info.penetration_depth
        );
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
        assert!(info.penetration_depth > 0.9, "penetration = {}", info.penetration_depth);
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
        assert!(info.penetration_depth > 3.8, "penetration = {}", info.penetration_depth);
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
