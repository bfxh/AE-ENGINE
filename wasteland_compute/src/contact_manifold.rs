//! Contact Manifold Generation — 接触流形生成
//!
//! 基于:
//! - Ericson. Real-Time Collision Detection. Morgan Kaufmann 2005. Ch 5.5
//!   (Reference/Incident Face Clipping)
//! - Catto. "Contact Manifolds." Box2D Documentation.
//! - Bullet Physics: btManifoldResult, btPersistentManifold
//! - Sutherland-Hodgman polygon clipping (1974)
//!
//! 核心思想:
//! 1. GJK+EPA 返回单个接触点, 但稳定堆叠需要多点接触 (流形)
//! 2. 找参考面 (法线最平行于接触法线的面) 和入射面 (另一形状的对应面)
//! 3. Sutherland-Hodgman 裁剪: 将入射面多边形裁到参考面的侧平面内
//! 4. 保留穿透深度 > 0 的点 (在参考面"下方")
//! 5. 限制为 4 个接触点 (Box2D 策略: 保留构成最大面积的四边形)
//! 6. 跨帧持久化: 匹配旧点与新点, 保留累积冲量 (warm starting)

use glam::{Vec3, Quat};

use crate::collision::{BoxCollider, ContactInfo};
use crate::resting_rigid_bodies::ConvexPolyhedron;

const MAX_MANIFOLD_POINTS: usize = 4;
const PERSIST_DISTANCE_SQR: f32 = 0.01 * 0.01;
const PERSIST_NORMAL_COS: f32 = 0.99; // ~8 degrees

// ============================================================
// WorldFace — 世界空间中的面
// ============================================================

#[derive(Debug, Clone)]
pub struct WorldFace {
    /// 面顶点 (世界空间, 从外侧看逆时针)
    pub vertices: Vec<Vec3>,
    /// 外法线 (世界空间)
    pub normal: Vec3,
    /// 面中心 (世界空间)
    pub center: Vec3,
}

impl WorldFace {
    /// 面平面方程: normal · x = d
    #[inline]
    pub fn plane_d(&self) -> f32 {
        self.normal.dot(self.center)
    }

    /// 点到面的有符号距离 (正=面外, 负=面内)
    #[inline]
    pub fn signed_distance(&self, point: Vec3) -> f32 {
        self.normal.dot(point) - self.plane_d()
    }

    /// 面的边数
    #[inline]
    pub fn num_edges(&self) -> usize {
        self.vertices.len()
    }

    /// 获取第 i 条边的起点和终点
    #[inline]
    pub fn edge(&self, i: usize) -> (Vec3, Vec3) {
        let n = self.vertices.len();
        (self.vertices[i], self.vertices[(i + 1) % n])
    }

    /// 获取第 i 条边的侧平面法线 (指向面内部)
    pub fn edge_side_normal(&self, i: usize) -> Vec3 {
        let (a, b) = self.edge(i);
        let edge_dir = b - a;
        // 侧平面法线 = face_normal × edge_dir (指向面内部, CCW-from-outside 绕序)
        self.normal.cross(edge_dir).normalize_or_zero()
    }
}

// ============================================================
// ContactPoint — 单个接触点
// ============================================================

#[derive(Debug, Clone, Copy)]
pub struct ContactPoint {
    /// 世界空间接触点
    pub point: Vec3,
    /// 接触法线 (A → B)
    pub normal: Vec3,
    /// 穿透深度 (>0 = 重叠)
    pub penetration: f32,
    /// 累积法向冲量 (warm starting)
    pub normal_impulse: f32,
    /// 摩擦切向冲量 1
    pub tangent_impulse1: f32,
    /// 摩擦切向冲量 2
    pub tangent_impulse2: f32,
    /// 已存在的帧数 (持久化用)
    pub age: u32,
}

impl ContactPoint {
    pub fn new(point: Vec3, normal: Vec3, penetration: f32) -> Self {
        Self {
            point,
            normal,
            penetration,
            normal_impulse: 0.0,
            tangent_impulse1: 0.0,
            tangent_impulse2: 0.0,
            age: 0,
        }
    }
}

// ============================================================
// ContactManifold — 持久接触流形
// ============================================================

#[derive(Debug, Clone)]
pub struct ContactManifold {
    /// 接触点列表 (最多 MAX_MANIFOLD_POINTS 个)
    pub points: Vec<ContactPoint>,
    /// 主接触法线 (A → B)
    pub normal: Vec3,
    /// 摩擦切向方向 1
    pub tangent1: Vec3,
    /// 摩擦切向方向 2
    pub tangent2: Vec3,
    pub a_index: usize,
    pub b_index: usize,
}

impl ContactManifold {
    pub fn new(a_index: usize, b_index: usize) -> Self {
        Self {
            points: Vec::new(),
            normal: Vec3::Y,
            tangent1: Vec3::X,
            tangent2: Vec3::Z,
            a_index,
            b_index,
        }
    }

    /// 从碰撞结果生成接触点
    ///
    /// poly_a, poly_b: 两个凸多面体 (局部空间)
    /// pos_a, pos_b: 世界位置
    /// rot_a, rot_b: 世界旋转
    /// contact: GJK+EPA 的碰撞结果
    pub fn generate(
        &mut self,
        poly_a: &ConvexPolyhedron,
        pos_a: Vec3,
        rot_a: Quat,
        poly_b: &ConvexPolyhedron,
        pos_b: Vec3,
        rot_b: Quat,
        contact: &ContactInfo,
    ) {
        self.normal = contact.normal;
        self.compute_tangents();

        if !contact.intersecting {
            self.points.clear();
            return;
        }

        // 获取世界空间面
        let faces_a = world_face_groups(poly_a, pos_a, rot_a);
        let faces_b = world_face_groups(poly_b, pos_b, rot_b);

        // 找参考面和入射面
        // 接触法线 n: A -> B
        // 参考面 (在 A 上): 法线最平行于 +n (A 面向 B 的面)
        // 参考面 (在 B 上): 法线最平行于 -n (B 面向 A 的面)
        // 入射面: 另一形状上法线最反平行于接触法线的面
        let ref_a = find_reference_face(&faces_a, contact.normal);
        let ref_b = find_reference_face(&faces_b, -contact.normal);

        let (reference, incident) = match (ref_a, ref_b) {
            (Some(ra), Some(rb)) => {
                // 选择更对齐的面作为参考面
                let align_a = ra.normal.dot(contact.normal);
                let align_b = -rb.normal.dot(contact.normal);
                if align_a >= align_b {
                    (ra, find_incident_face(&faces_b, contact.normal).cloned())
                } else {
                    (rb, find_incident_face(&faces_a, -contact.normal).cloned())
                }
            }
            (Some(ra), None) => {
                (ra, find_incident_face(&faces_b, contact.normal).cloned())
            }
            (None, Some(rb)) => {
                (rb, find_incident_face(&faces_a, -contact.normal).cloned())
            }
            (None, None) => {
                // fallback: 用 EPA 的单点
                let cp = ContactPoint::new(
                    pos_a + contact.normal * (contact.penetration_depth * 0.5),
                    contact.normal,
                    contact.penetration_depth,
                );
                self.points = vec![cp];
                return;
            }
        };

        let Some(incident) = incident else {
            let cp = ContactPoint::new(
                pos_a + contact.normal * (contact.penetration_depth * 0.5),
                contact.normal,
                contact.penetration_depth,
            );
            self.points = vec![cp];
            return;
        };

        // Sutherland-Hodgman: 用参考面的侧平面裁剪入射面多边形
        let clipped = clip_polygon_by_face(&incident.vertices, &reference);

        // 保留穿透深度 > 0 的点 (在参考面"下方")
        let ref_plane_d = reference.plane_d();
        let mut new_points: Vec<ContactPoint> = Vec::new();
        for &v in &clipped {
            let dist = reference.normal.dot(v) - ref_plane_d;
            // dist < 0 表示点在参考面内侧 (穿透)
            let penetration = -dist;
            if penetration > 0.0 {
                new_points.push(ContactPoint::new(v, contact.normal, penetration));
            }
        }

        // 限制点数
        if new_points.len() > MAX_MANIFOLD_POINTS {
            new_points = select_best_points(&new_points, MAX_MANIFOLD_POINTS);
        }

        self.points = new_points;
    }

    /// 从旧流形持久化接触点 (warm starting)
    /// 匹配位置和法线接近的点, 保留累积冲量
    pub fn persist(&mut self, old: &ContactManifold) {
        // 法线方向变化太大则不持久化
        if self.normal.dot(old.normal) < PERSIST_NORMAL_COS {
            return;
        }

        for cp in &mut self.points {
            cp.age = 0;
            for &old_cp in &old.points {
                let dist_sqr = (cp.point - old_cp.point).length_squared();
                if dist_sqr < PERSIST_DISTANCE_SQR {
                    cp.normal_impulse = old_cp.normal_impulse;
                    cp.tangent_impulse1 = old_cp.tangent_impulse1;
                    cp.tangent_impulse2 = old_cp.tangent_impulse2;
                    cp.age = old_cp.age + 1;
                    break;
                }
            }
        }
    }

    /// 限制接触点数量 (保留最大面积的四边形)
    pub fn limit_points(&mut self) {
        if self.points.len() <= MAX_MANIFOLD_POINTS {
            return;
        }
        self.points = select_best_points(&self.points, MAX_MANIFOLD_POINTS);
    }

    /// 从法线计算切向方向
    pub fn compute_tangents(&mut self) {
        let n = self.normal;
        let t1 = if n.x.abs() < 0.9 {
            Vec3::X.cross(n).normalize_or_zero()
        } else {
            Vec3::Y.cross(n).normalize_or_zero()
        };
        self.tangent1 = t1;
        self.tangent2 = n.cross(t1).normalize_or_zero();
    }

    /// 接触点数量
    #[inline]
    pub fn num_points(&self) -> usize {
        self.points.len()
    }

    /// 是否有接触
    #[inline]
    pub fn has_contact(&self) -> bool {
        !self.points.is_empty()
    }
}

// ============================================================
// Sutherland-Hodgman 多边形裁剪
// ============================================================

/// 用平面裁剪多边形 (保留平面正侧的点)
/// plane_normal 指向保留侧
pub fn clip_polygon_by_plane(
    subject: &[Vec3],
    plane_point: Vec3,
    plane_normal: Vec3,
) -> Vec<Vec3> {
    if subject.is_empty() {
        return Vec::new();
    }
    if subject.len() == 1 {
        let d = plane_normal.dot(subject[0] - plane_point);
        return if d >= 0.0 { vec![subject[0]] } else { Vec::new() };
    }

    let mut output: Vec<Vec3> = Vec::new();
    let n = subject.len();

    for i in 0..n {
        let a = subject[i];
        let b = subject[(i + 1) % n];
        let da = plane_normal.dot(a - plane_point);
        let db = plane_normal.dot(b - plane_point);

        if da >= 0.0 {
            // a 在内侧
            output.push(a);
            if db < 0.0 {
                // b 在外侧 -> 交点
                let t = da / (da - db);
                output.push(a + (b - a) * t);
            }
        } else if db >= 0.0 {
            // a 在外侧, b 在内侧 -> 交点
            let t = da / (da - db);
            output.push(a + (b - a) * t);
        }
        // a, b 都在外侧 -> 不添加
    }

    output
}

/// 用参考面的侧平面裁剪入射面多边形
/// 依次用参考面每条边的侧平面裁剪
pub fn clip_polygon_by_face(subject: &[Vec3], reference: &WorldFace) -> Vec<Vec3> {
    let mut result: Vec<Vec3> = subject.to_vec();

    for i in 0..reference.num_edges() {
        let (a, _) = reference.edge(i);
        let side_normal = reference.edge_side_normal(i);
        // 侧平面指向面内部, 保留正侧
        result = clip_polygon_by_plane(&result, a, side_normal);
        if result.is_empty() {
            break;
        }
    }

    result
}

// ============================================================
// 面查找
// ============================================================

/// 找法线最平行于 direction 的面 (最大 dot product)
pub fn find_reference_face<'a>(
    faces: &'a [WorldFace],
    direction: Vec3,
) -> Option<&'a WorldFace> {
    faces.iter().max_by(|a, b| {
        let da = a.normal.dot(direction);
        let db = b.normal.dot(direction);
        da.partial_cmp(&db).unwrap()
    })
}

/// 找法线最反平行于 direction 的面 (最小 dot product)
pub fn find_incident_face<'a>(
    faces: &'a [WorldFace],
    direction: Vec3,
) -> Option<&'a WorldFace> {
    faces.iter().min_by(|a, b| {
        let da = a.normal.dot(direction);
        let db = b.normal.dot(direction);
        da.partial_cmp(&db).unwrap()
    })
}

// ============================================================
// ConvexPolyhedron -> WorldFace 转换
// ============================================================

/// 获取 ConvexPolyhedron 在世界空间中的面组 (合并共面三角形)
pub fn world_face_groups(
    poly: &ConvexPolyhedron,
    position: Vec3,
    rotation: Quat,
) -> Vec<WorldFace> {
    let groups = poly.unique_face_groups();
    groups
        .iter()
        .map(|(local_n, idxs)| {
            let wn = rotation * (*local_n);

            // 收集面顶点 (去重)
            let mut verts: Vec<Vec3> = Vec::new();
            for &fi in idxs {
                for &vi in &poly.faces[fi] {
                    let v = position + rotation * poly.vertices[vi];
                    if !verts.iter().any(|&ev| (ev - v).length_squared() < 1e-16) {
                        verts.push(v);
                    }
                }
            }
            if verts.is_empty() {
                return WorldFace {
                    vertices: Vec::new(),
                    normal: wn,
                    center: position,
                };
            }

            // 按角度排序 (从外侧看逆时针)
            let center = verts.iter().sum::<Vec3>() / verts.len() as f32;
            let t1 = if wn.x.abs() < 0.9 {
                Vec3::X.cross(wn).normalize_or_zero()
            } else {
                Vec3::Y.cross(wn).normalize_or_zero()
            };
            let t2 = wn.cross(t1).normalize_or_zero();

            verts.sort_by(|a, b| {
                let aa = (a - center).dot(t2).atan2((a - center).dot(t1));
                let bb = (b - center).dot(t2).atan2((b - center).dot(t1));
                aa.partial_cmp(&bb).unwrap()
            });

            WorldFace {
                vertices: verts,
                normal: wn,
                center,
            }
        })
        .collect()
}

// ============================================================
// BoxCollider -> ConvexPolyhedron
// ============================================================

/// 将 BoxCollider 转换为 ConvexPolyhedron (局部空间, 中心在原点)
pub fn box_to_polyhedron(box_collider: &BoxCollider) -> ConvexPolyhedron {
    let he = box_collider.half_extents;
    let vertices = vec![
        Vec3::new(-he.x, -he.y, -he.z),
        Vec3::new(he.x, -he.y, -he.z),
        Vec3::new(he.x, he.y, -he.z),
        Vec3::new(-he.x, he.y, -he.z),
        Vec3::new(-he.x, -he.y, he.z),
        Vec3::new(he.x, -he.y, he.z),
        Vec3::new(he.x, he.y, he.z),
        Vec3::new(-he.x, he.y, he.z),
    ];
    let faces = vec![
        [0, 3, 2],
        [0, 2, 1], // -z
        [4, 5, 6],
        [4, 6, 7], // +z
        [0, 4, 7],
        [0, 7, 3], // -x
        [1, 2, 6],
        [1, 6, 5], // +x
        [0, 1, 5],
        [0, 5, 4], // -y
        [3, 7, 6],
        [3, 6, 2], // +y
    ];
    ConvexPolyhedron { vertices, faces }
}

// ============================================================
// 辅助: 选择最佳接触点
// ============================================================

/// 从候选点中选择 n 个构成最大面积的多边形
/// 简化策略: 选择最远的点对, 然后选择离该点对连线最远的点
fn select_best_points(points: &[ContactPoint], n: usize) -> Vec<ContactPoint> {
    if points.len() <= n {
        return points.to_vec();
    }

    // 1. 找距离最远的两个点
    let mut best_i = 0;
    let mut best_j = 1;
    let mut best_dist = 0.0;
    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let d = (points[i].point - points[j].point).length_squared();
            if d > best_dist {
                best_dist = d;
                best_i = i;
                best_j = j;
            }
        }
    }

    let mut selected: Vec<usize> = vec![best_i, best_j];

    // 2. 选择离已选点连线最远的点
    while selected.len() < n && selected.len() < points.len() {
        let a = points[selected[0]].point;
        let b = points[selected[1]].point;
        let ab = b - a;

        let mut best_k = 0;
        let mut best_perp_dist = -1.0;
        for k in 0..points.len() {
            if selected.contains(&k) {
                continue;
            }
            let ap = points[k].point - a;
            let proj = ap.dot(ab) / ab.length_squared();
            let perp = ap - ab * proj;
            let d = perp.length_squared();
            if d > best_perp_dist {
                best_perp_dist = d;
                best_k = k;
            }
        }
        selected.push(best_k);
    }

    selected.iter().map(|&i| points[i]).collect()
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Quat, Vec3};

    #[test]
    fn test_clip_polygon_by_plane_keep_all() {
        // 正方形完全在平面正侧
        let poly = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let result = clip_polygon_by_plane(&poly, Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(result.len(), 4, "should keep all 4 vertices");
    }

    #[test]
    fn test_clip_polygon_by_plane_cut_half() {
        // 正方形被 x=0.5 平面裁剪
        let poly = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let result = clip_polygon_by_plane(&poly, Vec3::new(0.5, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
        // 保留 x >= 0.5 的部分: 应该有 4 个顶点 (2 原始 + 2 交点)
        assert_eq!(result.len(), 4, "should have 4 vertices after clip");
        for v in &result {
            assert!(v.x >= 0.5 - 1e-6, "vertex x = {} should be >= 0.5", v.x);
        }
    }

    #[test]
    fn test_clip_polygon_by_plane_remove_all() {
        let poly = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ];
        // 平面法线指向 -x, 平面在 x=2 -> 所有点都在负侧
        let result = clip_polygon_by_plane(&poly, Vec3::new(2.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
        assert!(result.is_empty(), "should remove all vertices");
    }

    #[test]
    fn test_box_to_polyhedron() {
        let box_c = BoxCollider::new(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        let poly = box_to_polyhedron(&box_c);
        assert_eq!(poly.vertices.len(), 8);
        assert_eq!(poly.faces.len(), 12);
        let vol = poly.volume();
        assert!((vol - 8.0).abs() < 1e-5, "box volume = {}, expected 8", vol);
    }

    #[test]
    fn test_world_face_groups_cube() {
        let cube = ConvexPolyhedron::cube(1.0);
        let faces = world_face_groups(&cube, Vec3::ZERO, Quat::IDENTITY);
        assert_eq!(faces.len(), 6, "cube should have 6 face groups");
        for f in &faces {
            assert_eq!(f.vertices.len(), 4, "each cube face should have 4 vertices");
        }
    }

    #[test]
    fn test_find_reference_face() {
        let cube = ConvexPolyhedron::cube(1.0);
        let faces = world_face_groups(&cube, Vec3::ZERO, Quat::IDENTITY);
        // direction = +x -> reference face should have normal ~+x
        let rf = find_reference_face(&faces, Vec3::new(1.0, 0.0, 0.0));
        assert!(rf.is_some());
        let rf = rf.unwrap();
        assert!((rf.normal - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn test_find_incident_face() {
        let cube = ConvexPolyhedron::cube(1.0);
        let faces = world_face_groups(&cube, Vec3::ZERO, Quat::IDENTITY);
        // direction = +x -> incident face should have normal ~-x
        let if_ = find_incident_face(&faces, Vec3::new(1.0, 0.0, 0.0));
        assert!(if_.is_some());
        let if_ = if_.unwrap();
        assert!((if_.normal - Vec3::new(-1.0, 0.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn test_manifold_face_to_face() {
        // 两个立方体面接触: A 在下方, B 在上方, B 的底面接触 A 的顶面
        let cube = ConvexPolyhedron::cube(1.0);
        let contact = ContactInfo {
            intersecting: true,
            penetration_depth: 0.2,
            normal: Vec3::new(0.0, 1.0, 0.0), // A → B (向上)
        };
        let mut m = ContactManifold::new(0, 1);
        m.generate(
            &cube, Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY,
            &cube, Vec3::new(0.0, 1.8, 0.0), Quat::IDENTITY,
            &contact,
        );
        // 面接触应该生成多个接触点 (正方形面的裁剪结果)
        assert!(m.points.len() >= 2, "face-to-face should generate {} points, got {}", m.points.len(), m.points.len());
        for p in &m.points {
            assert!(p.penetration > 0.0, "penetration should be positive");
        }
    }

    #[test]
    fn test_manifold_persist() {
        let mut m1 = ContactManifold::new(0, 1);
        m1.normal = Vec3::new(0.0, 1.0, 0.0);
        m1.compute_tangents();
        m1.points.push(ContactPoint::new(
            Vec3::new(0.5, 1.0, 0.5),
            Vec3::new(0.0, 1.0, 0.0),
            0.1,
        ));
        m1.points[0].normal_impulse = 5.0;

        let mut m2 = m1.clone();
        m2.points[0].normal_impulse = 0.0; // 新帧冲量清零
        m2.persist(&m1);

        assert!((m2.points[0].normal_impulse - 5.0).abs() < 1e-6, "should preserve impulse");
        assert_eq!(m2.points[0].age, 1, "age should increment");
    }

    #[test]
    fn test_manifold_persist_normal_change() {
        let mut m1 = ContactManifold::new(0, 1);
        m1.normal = Vec3::new(0.0, 1.0, 0.0);
        m1.points.push(ContactPoint::new(
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            0.1,
        ));
        m1.points[0].normal_impulse = 5.0;

        let mut m2 = m1.clone();
        m2.normal = Vec3::new(1.0, 0.0, 0.0); // 法线变了 90 度
        m2.points[0].normal_impulse = 0.0;
        m2.persist(&m1);

        assert!(m2.points[0].normal_impulse < 1e-6, "should not preserve impulse when normal changes");
    }

    #[test]
    fn test_select_best_points() {
        let mut pts = Vec::new();
        for i in 0..6 {
            let angle = i as f32 * std::f32::consts::PI / 3.0;
            pts.push(ContactPoint::new(
                Vec3::new(angle.cos(), 0.0, angle.sin()),
                Vec3::Y,
                0.1,
            ));
        }
        let selected = select_best_points(&pts, 4);
        assert_eq!(selected.len(), 4);
    }

    #[test]
    fn test_manifold_edge_to_face() {
        // A 立方体在原点, B 立方体旋转 45 度后部分穿透 A 的顶面
        let cube = ConvexPolyhedron::cube(1.0);
        let contact = ContactInfo {
            intersecting: true,
            penetration_depth: 0.5,
            normal: Vec3::new(0.0, 1.0, 0.0),
        };
        let rot_b = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4);
        let mut m = ContactManifold::new(0, 1);
        m.generate(
            &cube, Vec3::ZERO, Quat::IDENTITY,
            &cube, Vec3::new(0.0, 1.5, 0.0), rot_b,
            &contact,
        );
        assert!(!m.points.is_empty(), "edge-to-face should generate contacts");
    }

    #[test]
    fn test_clip_polygon_by_face_cube() {
        // 入射面: 一个正方形
        let incident = vec![
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(-1.0, 1.0, 0.0),
        ];
        // 参考面: 一个更大的正方形 (在 z=0 平面)
        let reference = WorldFace {
            vertices: vec![
                Vec3::new(-2.0, -2.0, 0.0),
                Vec3::new(2.0, -2.0, 0.0),
                Vec3::new(2.0, 2.0, 0.0),
                Vec3::new(-2.0, 2.0, 0.0),
            ],
            normal: Vec3::new(0.0, 0.0, 1.0),
            center: Vec3::ZERO,
        };
        let clipped = clip_polygon_by_face(&incident, &reference);
        // 入射面完全在参考面内, 应该保留全部
        assert_eq!(clipped.len(), 4, "should keep all 4 vertices");
    }

    #[test]
    fn test_clip_polygon_by_face_partial() {
        // 入射面: 大正方形
        let incident = vec![
            Vec3::new(-2.0, -2.0, 0.0),
            Vec3::new(2.0, -2.0, 0.0),
            Vec3::new(2.0, 2.0, 0.0),
            Vec3::new(-2.0, 2.0, 0.0),
        ];
        // 参考面: 小正方形 (在 z=0 平面)
        let reference = WorldFace {
            vertices: vec![
                Vec3::new(-1.0, -1.0, 0.0),
                Vec3::new(1.0, -1.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(-1.0, 1.0, 0.0),
            ],
            normal: Vec3::new(0.0, 0.0, 1.0),
            center: Vec3::ZERO,
        };
        let clipped = clip_polygon_by_face(&incident, &reference);
        // 入射面被裁剪到参考面范围内
        assert!(!clipped.is_empty(), "should have clipped vertices");
        for v in &clipped {
            assert!(v.x.abs() <= 1.0 + 1e-5, "x = {} should be within [-1, 1]", v.x);
            assert!(v.y.abs() <= 1.0 + 1e-5, "y = {} should be within [-1, 1]", v.y);
        }
    }

    #[test]
    fn test_world_face_plane_d() {
        let face = WorldFace {
            vertices: vec![
                Vec3::new(1.0, -1.0, 2.0),
                Vec3::new(1.0, 1.0, 2.0),
                Vec3::new(-1.0, 1.0, 2.0),
                Vec3::new(-1.0, -1.0, 2.0),
            ],
            normal: Vec3::new(0.0, 0.0, 1.0),
            center: Vec3::new(0.0, 0.0, 2.0),
        };
        let d = face.plane_d();
        assert!((d - 2.0).abs() < 1e-6, "plane_d = {}, expected 2", d);
    }

    #[test]
    fn test_contact_point_new() {
        let cp = ContactPoint::new(Vec3::new(1.0, 2.0, 3.0), Vec3::Y, 0.5);
        assert_eq!(cp.point, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(cp.normal, Vec3::Y);
        assert_eq!(cp.penetration, 0.5);
        assert_eq!(cp.normal_impulse, 0.0);
        assert_eq!(cp.age, 0);
    }

    #[test]
    fn test_manifold_new() {
        let m = ContactManifold::new(0, 1);
        assert_eq!(m.a_index, 0);
        assert_eq!(m.b_index, 1);
        assert!(m.points.is_empty());
        assert!(!m.has_contact());
    }
}
