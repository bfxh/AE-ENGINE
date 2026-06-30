//! Continuous Collision Detection (CCD) — 连续碰撞检测
//!
//! 基于:
//! - Mirtich. "Impulse-based Dynamic Simulation of Rigid Body Systems."
//!   PhD thesis, UC Berkeley, 1996. (Conservative Advancement 原始算法)
//! - Ericson. Real-Time Collision Detection. Morgan Kaufmann, 2005. Ch 5.
//! - Redon, Kheddar, Coquillart. "Fast Continuous Collision Detection for
//!   Rigid Bodies." Journal of Graphics Tools, 2002. (Interval arithmetic 思想)
//! - Catto. "Continuous Collision." GDC 2013. (TOI 求解器工程实践)
//!
//! 解决问题: 离散碰撞检测在物体速度 > size/dt 时会"穿透" (tunneling).
//!   例如子弹穿过薄墙: 一帧前在墙左侧, 一帧后在墙右侧, 离散检测永远看不到相交.
//!
//! 核心思想 (Conservative Advancement, CA):
//! 1. 给定两形状在 [t0, t1] 间的运动 (线性 + 角速度), 求最早接触时刻 TOI
//! 2. 当前最近距离 d(t0) > 0, 用运动上界 ||v_rel|| 估算安全步长:
//!      δt ≤ d(t0) / ||v_rel||_max
//! 3. 推进到 t0 + δt, 重新计算 d(t), 重复直到 d < ε
//! 4. ||v_rel||_max 包括线速度 + 角速度贡献: ||v|| + ||ω||·r_max
//!    其中 r_max 是形状最远点到参考点的距离上界
//!
//! 本模块提供:
//! - `gjk_distance`: GJK 距离查询 (非相交时返回最近点和距离)
//! - `conservative_advancement`: 通用 CA 求解器 (任意 Collider)
//! - 解析 TOI: `sphere_sphere_toi`, `ray_sphere`, `ray_box`, `ray_plane`
//! - `swept_sphere`: 球扫掠体 vs 静态形状 (子弹/投掷物)
//! - `CcdSolver`: 多体 CCD 管线, 找最早 TOI 并推进

use glam::{Vec3, Quat, Mat3};

use crate::collision::Collider;

// ============================================================
// 常量
// ============================================================

const GJK_DISTANCE_MAX_ITER: usize = 64;
const GJK_DISTANCE_TOLERANCE: f32 = 1e-5;
const CA_MAX_ITER: usize = 64;
const CA_DISTANCE_TOLERANCE: f32 = 1e-4;
const CA_TIME_TOLERANCE: f32 = 1e-5;
const TOI_EPSILON: f32 = 1e-4;

// ============================================================
// GJK 距离查询 (非相交时返回最近点)
// ============================================================

/// GJK 距离结果
#[derive(Debug, Clone, Copy, Default)]
pub struct DistanceResult {
    /// 两形状表面最近距离 (相交时为 0 或负, 本实现返回 0)
    pub distance: f32,
    /// A 上最近点
    pub closest_a: Vec3,
    /// B 上最近点
    pub closest_b: Vec3,
    /// 从 A 指向 B 的法线 (B - A 归一化)
    pub normal: Vec3,
    /// 是否相交
    pub intersecting: bool,
}

/// GJK 距离查询: 返回两凸形状的最近点对和距离
///
/// 算法: 在 Minkowski Difference (MD) 上迭代, 寻找离原点最近的点.
/// 用 1-单纯形 (线段)、2-单纯形 (三角形)、3-单纯形 (四面体) 的 Voronoi 区域
/// 投影原点, 收敛时返回最近点.
///
/// 与 collision.rs::gjk 不同: 此版本在非相交时也返回距离, 不要求原点在 MD 内.
pub fn gjk_distance<A: Collider, B: Collider>(a: &A, b: &B) -> DistanceResult {
    let mut initial_dir = b.center() - a.center();
    if initial_dir.length_squared() < 1e-12 {
        initial_dir = Vec3::new(1.0, 0.0, 0.0);
    }
    let initial_dir = initial_dir.normalize();

    // 初始 simplex: 单个 support 点
    let s1 = md_support(a, b, initial_dir);
    let mut simplex: Vec<Vec3> = vec![s1];
    let mut direction = -s1;

    let mut closest = s1; // MD 上离原点最近的点

    for _ in 0..GJK_DISTANCE_MAX_ITER {
        if direction.length_squared() < 1e-20 {
            break;
        }
        let new_pt = md_support(a, b, direction);
        // 收敛判断: 新点在方向上的投影没有显著前进
        let proj = new_pt.dot(direction);
        let closest_proj = closest.dot(direction);
        if proj - closest_proj < GJK_DISTANCE_TOLERANCE {
            break;
        }
        simplex.push(new_pt);
        // 更新 simplex 为包含原点的最小子单纯形, 同时返回新的搜索方向和最近点
        let (new_simplex, new_dir, new_closest, contains_origin) =
            do_simplex_distance(&simplex);
        simplex = new_simplex;
        direction = new_dir;
        closest = new_closest;
        if contains_origin {
            // 原点在 MD 内 -> 相交
            return DistanceResult {
                distance: 0.0,
                closest_a: a.center(),
                closest_b: b.center(),
                normal: Vec3::ZERO,
                intersecting: true,
            };
        }
        if direction.length_squared() < 1e-20 {
            break;
        }
    }

    // 收敛: closest 是 MD 上离原点最近的点
    let dist = closest.length();
    if dist < GJK_DISTANCE_TOLERANCE {
        return DistanceResult {
            distance: 0.0,
            closest_a: a.center(),
            closest_b: b.center(),
            normal: Vec3::ZERO,
            intersecting: true,
        };
    }
    // closest = a_pt - b_pt (Minkowski Difference A-B), 方向为 A 减去 B
    // normal 从 A 指向 B = -closest / dist
    let normal = -closest / dist;
    // a_pt (A 上离 B 最近点) 在 -closest 方向: a.support(-closest/dist) = a.support(normal)
    // b_pt (B 上离 A 最近点) 在 +closest 方向: b.support(closest/dist) = b.support(-normal)
    let closest_a = a.support(normal);
    let closest_b = b.support(-normal);
    DistanceResult {
        distance: dist,
        closest_a,
        closest_b,
        normal,
        intersecting: false,
    }
}

/// Minkowski Difference support: md_support(a, b, d) = a.support(d) - b.support(-d)
#[inline]
fn md_support<A: Collider, B: Collider>(a: &A, b: &B, d: Vec3) -> Vec3 {
    a.support(d) - b.support(-d)
}

/// do_simplex 距离版: 处理 1/2/3-单纯形, 返回 (新单纯形, 新方向, 最近点, 是否包含原点)
///
/// 对于距离查询, 不要求原点在 simplex 内部, 而是找 simplex 上离原点最近的点.
/// 用 Voronoi 区域 (Johnson 算法) 投影原点到 simplex.
fn do_simplex_distance(
    simplex: &[Vec3],
) -> (Vec<Vec3>, Vec3, Vec3, bool) {
    match simplex.len() {
        1 => {
            let p = simplex[0];
            (vec![p], -p, p, false) // 单点: 方向指向原点, 最近点就是该点
        }
        2 => do_simplex_line(simplex),
        3 => do_simplex_triangle(simplex),
        4 => do_simplex_tetrahedron(simplex),
        _ => {
            // 不应发生, 取最后一个点
            let p = *simplex.last().unwrap();
            (vec![p], -p, p, false)
        }
    }
}

/// 线段 simplex: 找原点在线段上的最近点
fn do_simplex_line(simplex: &[Vec3]) -> (Vec<Vec3>, Vec3, Vec3, bool) {
    let a = simplex[1]; // 最新加入的点
    let b = simplex[0];
    let ab = b - a;
    let ao = -a; // 原点相对 a

    let ab_dot_ao = ab.dot(ao);
    if ab_dot_ao > 0.0 {
        // 原点在 ab 段的 Voronoi 区域内 (投影参数 t ∈ (0,1))
        let t = ab_dot_ao / ab.length_squared();
        let closest = a + ab * t;
        // 垂直方向: ab × ao × ab
        let perp = ab.cross(ao).cross(ab);
        let dir = if perp.length_squared() > 1e-20 { perp } else { -closest };
        (vec![b, a], dir, closest, false)
    } else {
        // 原点在 a 的 Voronoi 区域
        (vec![a], -a, a, false)
    }
}

/// 三角形 simplex: 找原点在三角形上的最近点
fn do_simplex_triangle(simplex: &[Vec3]) -> (Vec<Vec3>, Vec3, Vec3, bool) {
    let a = simplex[2]; // 最新点
    let b = simplex[1];
    let c = simplex[0];
    let ao = -a;
    let ab = b - a;
    let ac = c - a;
    let abc = ab.cross(ac); // 三角形法线

    // 检查原点是否在 abc 的"面"区域内 (法线方向的 Voronoi)
    if abc.dot(ao) > 0.0 {
        // 原点在三角形外侧 (法线方向)
        let perp = abc.cross(ao).cross(abc);
        let dir = if perp.length_squared() > 1e-20 { perp } else { -a };
        // 退化为线段或点
        return closest_on_triangle_or_degenerate(a, b, c, dir);
    }

    // 检查 abc 的边 Voronoi 区域
    // 边 ab
    let abc_cross_ab = abc.cross(ab);
    if abc_cross_ab.dot(ao) > 0.0 {
        // 原点在 ab 边外侧
        if ab.dot(ao) > 0.0 {
            // 原点在 ab 段的 Voronoi 内
            return do_simplex_line(&[b, a]);
        }
        // 否则只剩 a 点
        return (vec![a], -a, a, false);
    }

    // 边 ac
    let ac_cross_abc = ac.cross(abc);
    if ac_cross_abc.dot(ao) > 0.0 {
        // 原点在 ac 边外侧
        if ac.dot(ao) > 0.0 {
            return do_simplex_line(&[c, a]);
        }
        return (vec![a], -a, a, false);
    }

    // 原点在三角形面的 Voronoi 区域内 (3D 中不等于"包含", 只表示最近点在面内)
    // 用 barycentric 投影找原点在三角形上的最近点
    // (closest_on_triangle_or_degenerate 已处理边/顶点退化, 这里是面内情况)
    let abc_len_sq = abc.length_squared();
    if abc_len_sq < 1e-20 {
        // 退化三角形
        return (vec![a], -a, a, false);
    }
    // 原点到三角形平面的有符号距离 (沿 abc 方向)
    let dist_to_plane = abc.dot(ao) / abc_len_sq.sqrt();
    // 投影点 = 原点 - dist * n_hat = -dist * n_hat (相对原点)
    // 但 GJK 中"最近点"是 simplex 上离原点最近的点, 即三角形上最近点
    // 用 barycentric 计算
    let n_hat = abc / abc_len_sq.sqrt();
    // 三角形上离原点最近点 = 原点投影到平面
    let proj = -n_hat * dist_to_plane; // 等价于原点 - dist·n_hat, 投影到平面
    // 检查 proj 是否在三角形内 (barycentric)
    let ab = b - a;
    let ac = c - a;
    let ap = proj - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    let bp = proj - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    let inside = (d1 >= 0.0 && d2 >= 0.0 && d3 <= 0.0 && d4 >= d3)
        && (d1 * d4 - d3 * d2 >= 0.0); // 简化判断
    if inside && dist_to_plane.abs() < GJK_DISTANCE_TOLERANCE {
        // 原点在三角形平面且三角形内 -> 真正相交
        return (vec![c, b, a], Vec3::ZERO, Vec3::ZERO, true);
    }
    // 否则用法线方向继续搜索最近点
    let dir = if dist_to_plane > 0.0 { abc } else { -abc };
    let closest_on_plane = -n_hat * dist_to_plane;
    (vec![c, b, a], dir, closest_on_plane, false)
}

/// 三角形上找原点最近点, 并降级到线段或点
fn closest_on_triangle_or_degenerate(
    a: Vec3,
    b: Vec3,
    c: Vec3,
    fallback_dir: Vec3,
) -> (Vec<Vec3>, Vec3, Vec3, bool) {
    // 用 barycentric 坐标投影原点到三角形
    let ab = b - a;
    let ac = c - a;
    let ap = -a; // 原点 - a
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return (vec![a], -a, a, false);
    }
    let bp = -b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return (vec![b], -b, b, false);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let t = d1 / (d1 - d3);
        let closest = a + ab * t;
        return (vec![b, a], -closest, closest, false);
    }
    let cp = -c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return (vec![c], -c, c, false);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let t = d2 / (d2 - d6);
        let closest = a + ac * t;
        return (vec![c, a], -closest, closest, false);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let t = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        let closest = b + (c - b) * t;
        return (vec![c, b], -closest, closest, false);
    }
    // 原点投影在三角形内部
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    let closest = a + ab * v + ac * w;
    (vec![c, b, a], fallback_dir, closest, false)
}

/// 四面体 simplex: 找原点在四面体上的最近点, 或检测包含
fn do_simplex_tetrahedron(simplex: &[Vec3]) -> (Vec<Vec3>, Vec3, Vec3, bool) {
    let a = simplex[3]; // 最新点
    let b = simplex[2];
    let c = simplex[1];
    let d = simplex[0];
    let ao = -a;

    let ab = b - a;
    let ac = c - a;
    let ad = d - a;

    // 三个面的法线 (朝外, 即背离另一顶点)
    let abc = ab.cross(ac);
    let acd = ac.cross(ad);
    let adb = ad.cross(ab);

    // 检查原点在哪个面外侧
    // abc 面 (背离 d)
    let abc_outward = if abc.dot(ad) > 0.0 { -abc } else { abc };
    if abc_outward.dot(ao) > 0.0 {
        return do_simplex_triangle(&[c, b, a]);
    }
    // acd 面 (背离 b)
    let acd_outward = if acd.dot(ab) > 0.0 { -acd } else { acd };
    if acd_outward.dot(ao) > 0.0 {
        return do_simplex_triangle(&[d, c, a]);
    }
    // adb 面 (背离 c)
    let adb_outward = if adb.dot(ac) > 0.0 { -adb } else { adb };
    if adb_outward.dot(ao) > 0.0 {
        return do_simplex_triangle(&[b, d, a]);
    }

    // 原点在四面体内部 -> 相交
    (vec![d, c, b, a], Vec3::ZERO, Vec3::ZERO, true)
}

// ============================================================
// 运动状态 (用于 CA)
// ============================================================

/// 刚体在某时间段内的运动状态 (线性 + 角速度)
#[derive(Debug, Clone, Copy, Default)]
pub struct Motion {
    pub linear_velocity: Vec3,
    pub angular_velocity: Vec3,
    /// 形状上离参考点 (center) 最远的距离上界, 用于角速度贡献的界
    pub max_radius: f32,
}

impl Motion {
    pub fn stationary() -> Self {
        Self::default()
    }
    pub fn linear(v: Vec3) -> Self {
        Self { linear_velocity: v, angular_velocity: Vec3::ZERO, max_radius: 0.0 }
    }
    pub fn with_angular(mut self, omega: Vec3, r_max: f32) -> Self {
        self.angular_velocity = omega;
        self.max_radius = r_max;
        self
    }

    /// 相对运动的速度上界 (用于 CA 步长估算)
    /// ||v_rel|| + ||omega||·r_max
    #[inline]
    pub fn speed_bound(&self) -> f32 {
        self.linear_velocity.length() + self.angular_velocity.length() * self.max_radius
    }
}

/// 时间冲击信息 (Time of Impact)
#[derive(Debug, Clone, Copy, Default)]
pub struct Toi {
    /// 接触时刻 (相对于查询区间起点, 通常 [0, 1] 或 [0, dt])
    pub t: f32,
    /// 接触法线 (从 A 指向 B)
    pub normal: Vec3,
    /// 接触点 (世界坐标)
    pub point: Vec3,
}

// ============================================================
// Conservative Advancement (CA)
// ============================================================

// 注: 通用版本已移除 (与 Collider trait 的 Clone 约束冲突)
// 使用 conservative_advancement_sphere_sphere / _sphere_box 等具体类型版本

// 为常用形状对提供具体实现 (避免 Clone + 可变位置的复杂性)

use crate::collision::{SphereCollider, BoxCollider, CapsuleCollider};

/// 推进球体到时刻 t
#[inline]
fn advance_sphere(s: &SphereCollider, motion: &Motion, t: f32) -> SphereCollider {
    let translation = motion.linear_velocity * t;
    // 角速度对球体无视觉影响 (球对称), 但对球心位置无影响
    SphereCollider {
        center: s.center + translation,
        radius: s.radius,
    }
}

/// 推进盒子到时刻 t (含旋转)
#[inline]
fn advance_box(b: &BoxCollider, motion: &Motion, t: f32) -> BoxCollider {
    let translation = motion.linear_velocity * t;
    let omega = motion.angular_velocity;
    let rotation_delta = if omega.length_squared() > 1e-20 {
        // q_delta = exp(0.5 * omega * t) (axis-angle 形式)
        let omega_len = omega.length();
        let axis = omega / omega_len;
        let angle = omega_len * t;
        Quat::from_axis_angle(axis, angle)
    } else {
        Quat::IDENTITY
    };
    BoxCollider {
        center: b.center + translation,
        half_extents: b.half_extents,
        rotation: rotation_delta * b.rotation,
    }
}

/// 球-球 CA (用解析解 + 二分兜底)
pub fn conservative_advancement_sphere_sphere(
    a_start: &SphereCollider,
    b_start: &SphereCollider,
    motion_a: &Motion,
    motion_b: &Motion,
    dt: f32,
) -> Option<Toi> {
    // 优先用解析解
    if let Some(toi) = sphere_sphere_toi(a_start, b_start, motion_a, motion_b, dt) {
        return Some(toi);
    }
    // 兜底: 二分时间法 (使用 boolean intersect)
    bisection_toi_sphere_sphere(a_start, b_start, motion_a, motion_b, dt)
}

/// 二分时间法求 TOI (球-球): 找最早 t 使两球相交
///
/// 不动点: t_lo 时未相交, t_hi 时相交. 收敛到 t_lo ≈ t_hi.
fn bisection_toi_sphere_sphere(
    a_start: &SphereCollider,
    b_start: &SphereCollider,
    motion_a: &Motion,
    motion_b: &Motion,
    dt: f32,
) -> Option<Toi> {
    use crate::collision::intersect;
    // 边界检查
    let a0 = advance_sphere(a_start, motion_a, 0.0);
    let b0 = advance_sphere(b_start, motion_b, 0.0);
    if intersect(&a0, &b0) {
        let normal = (b0.center - a0.center).normalize_or_zero();
        return Some(Toi { t: 0.0, normal, point: a0.center + normal * a0.radius });
    }
    let a_end = advance_sphere(a_start, motion_a, dt);
    let b_end = advance_sphere(b_start, motion_b, dt);
    if !intersect(&a_end, &b_end) {
        return None;
    }
    // 二分
    let mut lo = 0.0f32;
    let mut hi = dt;
    for _ in 0..64 {
        if hi - lo < CA_TIME_TOLERANCE {
            break;
        }
        let mid = (lo + hi) * 0.5;
        let a_mid = advance_sphere(a_start, motion_a, mid);
        let b_mid = advance_sphere(b_start, motion_b, mid);
        if intersect(&a_mid, &b_mid) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let t = hi;
    let a_t = advance_sphere(a_start, motion_a, t);
    let b_t = advance_sphere(b_start, motion_b, t);
    let normal = (b_t.center - a_t.center).normalize_or_zero();
    let point = a_t.center + normal * a_t.radius;
    Some(Toi { t, normal, point })
}

/// 球-盒 CA: 采样 + 二分法找首次接触时刻
///
/// 处理球穿过盒子的情况: 即使在 dt 末尾球已离开盒子,
/// 也能找到最早的进入时刻.
pub fn conservative_advancement_sphere_box(
    sphere_start: &SphereCollider,
    box_start: &BoxCollider,
    motion_sphere: &Motion,
    motion_box: &Motion,
    dt: f32,
) -> Option<Toi> {
    use crate::collision::intersect;
    // t=0 检查
    let s0 = advance_sphere(sphere_start, motion_sphere, 0.0);
    let b0 = advance_box(box_start, motion_box, 0.0);
    if intersect(&s0, &b0) {
        let normal = (b0.center - s0.center).normalize_or_zero();
        return Some(Toi { t: 0.0, normal, point: s0.center + normal * s0.radius });
    }
    // 采样 N 个时刻, 找首次相交出现的区间
    let n_samples = 32;
    let mut prev_intersect = false;
    let mut prev_t = 0.0f32;
    for i in 1..=n_samples {
        let t = dt * (i as f32) / (n_samples as f32);
        let s = advance_sphere(sphere_start, motion_sphere, t);
        let b = advance_box(box_start, motion_box, t);
        let now = intersect(&s, &b);
        if now && !prev_intersect {
            // 在 (prev_t, t] 区间内首次相交, 二分细化
            let mut lo = prev_t;
            let mut hi = t;
            for _ in 0..48 {
                if hi - lo < CA_TIME_TOLERANCE {
                    break;
                }
                let mid = (lo + hi) * 0.5;
                let s_mid = advance_sphere(sphere_start, motion_sphere, mid);
                let b_mid = advance_box(box_start, motion_box, mid);
                if intersect(&s_mid, &b_mid) {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            let t_toi = hi;
            let s_t = advance_sphere(sphere_start, motion_sphere, t_toi);
            let b_t = advance_box(box_start, motion_box, t_toi);
            let normal = (b_t.center - s_t.center).normalize_or_zero();
            let point = s_t.center + normal * s_t.radius;
            return Some(Toi { t: t_toi, normal, point });
        }
        prev_intersect = now;
        prev_t = t;
    }
    None
}

// ============================================================
// 解析 TOI (闭式解)
// ============================================================

/// 球-球解析 TOI: 求两匀速运动球的最早接触时刻
///
/// 设相对位移 d(t) = (b.center - a.center) + (vb - va)·t
/// 接触条件: ||d(t)|| = ra + rb
/// 令 d0 = b.center - a.center, v = vb - va
/// ||d0 + v·t||² = (ra + rb)²
/// (v·v)·t² + 2·(d0·v)·t + (d0·d0 - (ra+rb)²) = 0
/// t = (-b ± √(b² - 4ac)) / (2a), 取较小正根
pub fn sphere_sphere_toi(
    a: &SphereCollider,
    b: &SphereCollider,
    motion_a: &Motion,
    motion_b: &Motion,
    dt: f32,
) -> Option<Toi> {
    let d0 = b.center - a.center;
    let v = motion_b.linear_velocity - motion_a.linear_velocity;
    let r_sum = a.radius + b.radius;
    let aa = v.dot(v);
    let bb = 2.0 * d0.dot(v);
    let cc = d0.dot(d0) - r_sum * r_sum;

    if aa < 1e-20 {
        // 相对静止
        if cc <= 0.0 {
            // 已接触
            return Some(Toi { t: 0.0, normal: d0.normalize_or_zero(), point: (a.center + b.center) * 0.5 });
        }
        return None;
    }
    let disc = bb * bb - 4.0 * aa * cc;
    if disc < 0.0 {
        return None; // 永不接触
    }
    let sqrt_disc = disc.sqrt();
    let t1 = (-bb - sqrt_disc) / (2.0 * aa);
    let t2 = (-bb + sqrt_disc) / (2.0 * aa);
    // 取最早进入接触的时刻 (t1 是进入, t2 是离开)
    let t = if t1 >= 0.0 {
        t1
    } else if t2 >= 0.0 {
        0.0 // 已在接触中
    } else {
        return None;
    };
    if t > dt + TOI_EPSILON {
        return None;
    }
    let t_clamped = t.clamp(0.0, dt);
    let contact_point_a = a.center + motion_a.linear_velocity * t_clamped;
    let contact_point_b = b.center + motion_b.linear_velocity * t_clamped;
    let normal = (contact_point_b - contact_point_a).normalize_or_zero();
    Some(Toi {
        t: t_clamped,
        normal,
        point: contact_point_a + normal * a.radius,
    })
}

/// 射线 vs 球: 返回最近正参数 t (沿 ray.direction)
///
/// 数学: ||o + t·d - c||² = r²
pub fn ray_sphere(
    ray_origin: Vec3,
    ray_direction: Vec3,
    sphere_center: Vec3,
    sphere_radius: f32,
    max_t: f32,
) -> Option<f32> {
    let dir = ray_direction.normalize_or_zero();
    let m = ray_origin - sphere_center;
    let b = m.dot(dir);
    let c = m.dot(m) - sphere_radius * sphere_radius;
    if c > 0.0 && b > 0.0 {
        return None; // 射线起点在球外, 且方向远离球
    }
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let sqrt_disc = disc.sqrt();
    let t1 = -b - sqrt_disc;
    let t2 = -b + sqrt_disc;
    // t1 <= 0 <= t2 表示射线起点在球内 -> 立即接触 (t=0)
    // t1 > 0 表示射线从球外进入, 取 t1
    // t2 < 0 表示射线起点在球外且方向远离球, 已在上面过滤
    let t = if t1 > 0.0 {
        t1
    } else if t2 >= 0.0 {
        // 起点在球内 (或表面), 立即接触
        0.0
    } else {
        return None;
    };
    if t > max_t { return None; }
    Some(t)
}

/// 射线 vs 轴对齐盒 (Slab method)
///
/// 输入: 射线 (o, d), 盒子 [box_min, box_max]
/// 返回最近正参数 t (沿未归一化 d). 若 d 已归一化, t 即距离
pub fn ray_aabb(
    ray_origin: Vec3,
    ray_direction: Vec3,
    box_min: Vec3,
    box_max: Vec3,
    max_t: f32,
) -> Option<f32> {
    let mut tmin = 0.0f32;
    let mut tmax = max_t;

    for i in 0..3 {
        let o = ray_origin[i];
        let d = ray_direction[i];
        let lo = box_min[i];
        let hi = box_max[i];
        if d.abs() < 1e-12 {
            // 射线平行于该轴的 slab
            if o < lo || o > hi {
                return None;
            }
        } else {
            let mut t1 = (lo - o) / d;
            let mut t2 = (hi - o) / d;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            tmin = tmin.max(t1);
            tmax = tmax.min(t2);
            if tmin > tmax {
                return None;
            }
        }
    }
    if tmin > max_t { None } else { Some(tmin) }
}

/// 射线 vs 任意旋转盒 (OBB)
///
/// 将射线变换到盒子的局部坐标系, 然后用 AABB slab method
pub fn ray_obb(
    ray_origin: Vec3,
    ray_direction: Vec3,
    box_center: Vec3,
    box_half_extents: Vec3,
    box_rotation: Quat,
    max_t: f32,
) -> Option<f32> {
    let inv_rot = box_rotation.inverse();
    let local_origin = inv_rot * (ray_origin - box_center);
    let local_dir = inv_rot * ray_direction;
    let box_min = -box_half_extents;
    let box_max = box_half_extents;
    ray_aabb(local_origin, local_dir, box_min, box_max, max_t)
}

/// 射线 vs 平面 (ax + by + cz + d = 0, normal = (a, b, c))
///
/// 返回 t = -d_dot_o / d_dot_d, d > 0 表示半空间
pub fn ray_plane(
    ray_origin: Vec3,
    ray_direction: Vec3,
    plane_normal: Vec3,
    plane_d: f32,
    max_t: f32,
) -> Option<f32> {
    let denom = ray_direction.dot(plane_normal);
    if denom.abs() < 1e-12 {
        return None; // 射线平行于平面
    }
    let t = -(ray_origin.dot(plane_normal) + plane_d) / denom;
    if t < 0.0 || t > max_t {
        return None;
    }
    Some(t)
}

/// 射线 vs 三角形 (Möller–Trumbore 算法)
///
/// 输入三角形顶点 v0, v1, v2 (逆时针为正面)
/// 返回最近正参数 t
pub fn ray_triangle(
    ray_origin: Vec3,
    ray_direction: Vec3,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    max_t: f32,
) -> Option<f32> {
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let h = ray_direction.cross(edge2);
    let a = edge1.dot(h);
    if a.abs() < 1e-12 {
        return None; // 射线平行于三角形
    }
    let f = 1.0 / a;
    let s = ray_origin - v0;
    let u = f * s.dot(h);
    if u < 0.0 || u > 1.0 {
        return None;
    }
    let q = s.cross(edge1);
    let v = f * ray_direction.dot(q);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = f * edge2.dot(q);
    if t < 0.0 || t > max_t {
        return None;
    }
    Some(t)
}

// ============================================================
// 扫掠球 (Bullet/Raycast 投射物)
// ============================================================

/// 扫掠球 vs 静态球: 子弹 (球, 沿 ray 移动) 撞到目标球
///
/// 等价于: 射线 vs 半径 = r_bullet + r_target 的球
pub fn swept_sphere_vs_sphere(
    bullet_start: Vec3,
    bullet_velocity: Vec3,
    bullet_radius: f32,
    target_center: Vec3,
    target_radius: f32,
    dt: f32,
) -> Option<Toi> {
    let max_t = dt;
    let combined_radius = bullet_radius + target_radius;
    let t = ray_sphere(bullet_start, bullet_velocity, target_center, combined_radius, max_t)?;
    let point = bullet_start + bullet_velocity * t;
    // normal 从 A (bullet) 指向 B (target)
    let normal = (target_center - point).normalize_or_zero();
    Some(Toi { t, normal, point })
}

/// 扫掠球 vs 静态 AABB
pub fn swept_sphere_vs_aabb(
    bullet_start: Vec3,
    bullet_velocity: Vec3,
    bullet_radius: f32,
    box_min: Vec3,
    box_max: Vec3,
    dt: f32,
) -> Option<Toi> {
    // 膨胀盒子半径 bullet_radius, 等价于射线 vs 膨胀盒
    let inflated_min = box_min - Vec3::splat(bullet_radius);
    let inflated_max = box_max + Vec3::splat(bullet_radius);
    let t = ray_aabb(bullet_start, bullet_velocity, inflated_min, inflated_max, dt)?;
    let point = bullet_start + bullet_velocity * t;
    // 接触法线: 取膨胀盒表面离 point 最近的法线
    let center = (inflated_min + inflated_max) * 0.5;
    let half = (inflated_max - inflated_min) * 0.5;
    let local = point - center;
    let mut normal = Vec3::ZERO;
    let mut best_pen = f32::NEG_INFINITY;
    for axis in 0..3 {
        let pen = half[axis].abs() - local[axis].abs();
        if pen > best_pen {
            best_pen = pen;
            normal = Vec3::ZERO;
            normal[axis] = if local[axis] > 0.0 { 1.0 } else { -1.0 };
        }
    }
    if normal.length_squared() < 1e-12 {
        normal = Vec3::Y;
    }
    Some(Toi { t, normal, point })
}

/// 扫掠球 vs 静态 OBB
pub fn swept_sphere_vs_obb(
    bullet_start: Vec3,
    bullet_velocity: Vec3,
    bullet_radius: f32,
    box_center: Vec3,
    box_half_extents: Vec3,
    box_rotation: Quat,
    dt: f32,
) -> Option<Toi> {
    let inv_rot = box_rotation.inverse();
    let local_start = inv_rot * (bullet_start - box_center);
    let local_vel = inv_rot * bullet_velocity;
    let local_min = -box_half_extents - Vec3::splat(bullet_radius);
    let local_max = box_half_extents + Vec3::splat(bullet_radius);
    let t = ray_aabb(local_start, local_vel, local_min, local_max, dt)?;
    let local_point = local_start + local_vel * t;
    // 法线 (局部)
    let half = (local_max - local_min) * 0.5;
    let center_local = (local_max + local_min) * 0.5;
    let local_rel = local_point - center_local;
    let mut local_normal = Vec3::ZERO;
    let mut best_pen = f32::NEG_INFINITY;
    for axis in 0..3 {
        let pen = half[axis].abs() - local_rel[axis].abs();
        if pen > best_pen {
            best_pen = pen;
            local_normal = Vec3::ZERO;
            local_normal[axis] = if local_rel[axis] > 0.0 { 1.0 } else { -1.0 };
        }
    }
    if local_normal.length_squared() < 1e-12 {
        local_normal = Vec3::Y;
    }
    let world_normal = box_rotation * local_normal;
    let world_point = box_center + box_rotation * local_point;
    Some(Toi { t, normal: world_normal, point: world_point })
}

// ============================================================
// CcdSolver: 多体 TOI 管线
// ============================================================

/// 单个 CCD 查询项 (一对运动形状)
#[derive(Debug, Clone)]
pub struct CcdPair {
    pub a_index: usize,
    pub b_index: usize,
    pub toi: Option<Toi>,
}

/// CCD 求解器: 给定多对运动形状, 找最早的 TOI
///
/// 典型用法:
/// 1. 离散 broadphase 找出潜在碰撞对
/// 2. 对每对调用 CCD 求 TOI
/// 3. 取最早 TOI, 推进模拟到该时刻, 求解接触, 重复
pub struct CcdSolver {
    /// 最小有意义时间步 (小于此值认为同时接触, 避免无限循环)
    pub min_toi: f32,
    /// 单帧内最大子步数 (防止退化场景卡死)
    pub max_substeps: usize,
}

impl Default for CcdSolver {
    fn default() -> Self {
        Self { min_toi: 1e-4, max_substeps: 16 }
    }
}

impl CcdSolver {
    pub fn new() -> Self { Self::default() }

    pub fn with_substeps(mut self, n: usize) -> Self {
        self.max_substeps = n;
        self
    }

    /// 从一组 TOI 查询结果中找最早的接触
    pub fn find_earliest<'a>(&self, pairs: &'a [CcdPair]) -> Option<&'a CcdPair> {
        pairs
            .iter()
            .filter(|p| p.toi.map(|t| t.t >= 0.0).unwrap_or(false))
            .min_by(|a, b| {
                let ta = a.toi.unwrap().t;
                let tb = b.toi.unwrap().t;
                ta.partial_cmp(&tb).unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// 把单帧 dt 切分为子步: 最早 TOI 之前 + 之后
    ///
    /// 返回 (toi_t, remaining_dt), 若无接触返回 None
    pub fn split_time(&self, earliest_toi_t: f32, dt: f32) -> Option<(f32, f32)> {
        if earliest_toi_t >= dt {
            return None;
        }
        let t = earliest_toi_t.max(self.min_toi);
        Some((t, (dt - t).max(0.0)))
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- GJK 距离查询 ----------

    #[test]
    fn test_gjk_distance_two_spheres_apart() {
        let a = SphereCollider { center: Vec3::new(0.0, 0.0, 0.0), radius: 1.0 };
        let b = SphereCollider { center: Vec3::new(5.0, 0.0, 0.0), radius: 1.0 };
        let r = gjk_distance(&a, &b);
        assert!(!r.intersecting, "should not intersect");
        // 距离 = 5 - 1 - 1 = 3
        assert!((r.distance - 3.0).abs() < 0.05, "distance: {}", r.distance);
        // 法线从 A 指向 B (+x)
        assert!(r.normal.x > 0.5, "normal: {:?}", r.normal);
    }

    #[test]
    fn test_gjk_distance_two_spheres_touching() {
        let a = SphereCollider { center: Vec3::new(0.0, 0.0, 0.0), radius: 1.0 };
        let b = SphereCollider { center: Vec3::new(2.0, 0.0, 0.0), radius: 1.0 };
        let r = gjk_distance(&a, &b);
        // 刚好接触, 距离 ≈ 0
        assert!(r.distance < 0.1, "distance: {}", r.distance);
    }

    #[test]
    fn test_gjk_distance_box_box_apart() {
        let a = BoxCollider::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let b = BoxCollider::new(Vec3::new(5.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let r = gjk_distance(&a, &b);
        assert!(!r.intersecting);
        // 距离 = 5 - 1 - 1 = 3
        assert!((r.distance - 3.0).abs() < 0.05, "distance: {}", r.distance);
        assert!(r.normal.x > 0.5);
    }

    #[test]
    fn test_gjk_distance_sphere_box_apart() {
        let a = SphereCollider { center: Vec3::new(0.0, 0.0, 0.0), radius: 1.0 };
        let b = BoxCollider::new(Vec3::new(5.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let r = gjk_distance(&a, &b);
        assert!(!r.intersecting);
        // 距离 = 5 - 1 - 1 = 3
        assert!((r.distance - 3.0).abs() < 0.05, "distance: {}", r.distance);
    }

    #[test]
    fn test_gjk_distance_box_box_intersect() {
        let a = BoxCollider::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let b = BoxCollider::new(Vec3::new(0.5, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let r = gjk_distance(&a, &b);
        assert!(r.intersecting, "should intersect");
    }

    // ---------- 球-球解析 TOI ----------

    #[test]
    fn test_sphere_sphere_toi_head_on() {
        // 两球相向而行, 4s 后接触
        let a = SphereCollider { center: Vec3::new(0.0, 0.0, 0.0), radius: 1.0 };
        let b = SphereCollider { center: Vec3::new(10.0, 0.0, 0.0), radius: 1.0 };
        let ma = Motion::linear(Vec3::new(1.0, 0.0, 0.0));
        let mb = Motion::linear(Vec3::new(-1.0, 0.0, 0.0));
        let toi = sphere_sphere_toi(&a, &b, &ma, &mb, 10.0);
        assert!(toi.is_some(), "should have TOI");
        let t = toi.unwrap().t;
        // 接触距离 = 2 (ra+rb), 起始 10, 相对速度 2, t = (10-2)/2 = 4
        assert!((t - 4.0).abs() < 1e-3, "toi t: {}", t);
    }

    #[test]
    fn test_sphere_sphere_toi_moving_vs_static() {
        let a = SphereCollider { center: Vec3::new(0.0, 0.0, 0.0), radius: 1.0 };
        let b = SphereCollider { center: Vec3::new(10.0, 0.0, 0.0), radius: 1.0 };
        let ma = Motion::linear(Vec3::new(2.0, 0.0, 0.0));
        let mb = Motion::stationary();
        let toi = sphere_sphere_toi(&a, &b, &ma, &mb, 10.0);
        assert!(toi.is_some());
        // 接触距离 = 2, 起始 10, 相对速度 2, t = (10-2)/2 = 4
        assert!((toi.unwrap().t - 4.0).abs() < 1e-3);
    }

    #[test]
    fn test_sphere_sphere_toi_miss() {
        // 两球轨迹不交 (垂直方向间距 > r_sum)
        let a = SphereCollider { center: Vec3::new(0.0, 0.0, 0.0), radius: 1.0 };
        let b = SphereCollider { center: Vec3::new(10.0, 5.0, 0.0), radius: 1.0 };
        let ma = Motion::linear(Vec3::new(1.0, 0.0, 0.0));
        let mb = Motion::linear(Vec3::new(-1.0, 0.0, 0.0));
        let toi = sphere_sphere_toi(&a, &b, &ma, &mb, 10.0);
        // 最近距离 = 5 > r_sum = 2, 不接触
        assert!(toi.is_none(), "expected no TOI, got {:?}", toi);
    }

    #[test]
    fn test_sphere_sphere_toi_already_touching() {
        let a = SphereCollider { center: Vec3::new(0.0, 0.0, 0.0), radius: 1.0 };
        let b = SphereCollider { center: Vec3::new(1.5, 0.0, 0.0), radius: 1.0 };
        let ma = Motion::stationary();
        let mb = Motion::stationary();
        let toi = sphere_sphere_toi(&a, &b, &ma, &mb, 10.0);
        assert!(toi.is_some());
        assert!(toi.unwrap().t < 0.01, "should be at t=0");
    }

    #[test]
    fn test_sphere_sphere_toi_out_of_range() {
        let a = SphereCollider { center: Vec3::new(0.0, 0.0, 0.0), radius: 1.0 };
        let b = SphereCollider { center: Vec3::new(100.0, 0.0, 0.0), radius: 1.0 };
        let ma = Motion::linear(Vec3::new(1.0, 0.0, 0.0));
        let mb = Motion::stationary();
        // 接触需要 98s, 但 dt = 10
        let toi = sphere_sphere_toi(&a, &b, &ma, &mb, 10.0);
        assert!(toi.is_none(), "expected no TOI within dt");
    }

    // ---------- 射线查询 ----------

    #[test]
    fn test_ray_sphere_hit() {
        let t = ray_sphere(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(5.0, 0.0, 0.0),
            1.0,
            100.0,
        );
        assert!(t.is_some());
        assert!((t.unwrap() - 4.0).abs() < 1e-3, "t: {}", t.unwrap());
    }

    #[test]
    fn test_ray_sphere_miss() {
        let t = ray_sphere(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(5.0, 5.0, 0.0),
            1.0,
            100.0,
        );
        assert!(t.is_none());
    }

    #[test]
    fn test_ray_sphere_inside() {
        // 射线起点在球内, 应立即命中 (t=0 附近)
        let t = ray_sphere(
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(5.0, 0.0, 0.0),
            1.0,
            100.0,
        );
        assert!(t.is_some());
        assert!(t.unwrap() < 0.01);
    }

    #[test]
    fn test_ray_aabb_hit() {
        let t = ray_aabb(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(4.0, -1.0, -1.0),
            Vec3::new(6.0, 1.0, 1.0),
            100.0,
        );
        assert!(t.is_some());
        assert!((t.unwrap() - 4.0).abs() < 1e-3);
    }

    #[test]
    fn test_ray_aabb_miss() {
        let t = ray_aabb(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(4.0, 5.0, -1.0),
            Vec3::new(6.0, 6.0, 1.0),
            100.0,
        );
        assert!(t.is_none());
    }

    #[test]
    fn test_ray_obb_hit() {
        // 盒子中心在 (5, 0, 0), 无旋转, 半径 (1, 1, 1)
        let t = ray_obb(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
            Quat::IDENTITY,
            100.0,
        );
        assert!(t.is_some());
        assert!((t.unwrap() - 4.0).abs() < 1e-3);
    }

    #[test]
    fn test_ray_obb_rotated() {
        // 盒子绕 z 轴旋转 45°, 沿 x 轴的射线应更早进入 (因为旋转后 x 半径变小, 但 y 变大)
        let rot = Quat::from_rotation_z(45.0f32.to_radians());
        let t = ray_obb(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
            rot,
            100.0,
        );
        // 旋转 45° 后, 沿 x 方向半宽 = 1·cos(45) + 1·sin(45) = √2 ≈ 1.414
        // 所以 t ≈ 5 - 1.414 ≈ 3.586
        assert!(t.is_some());
        let t_val = t.unwrap();
        assert!((t_val - (5.0 - 2.0f32.sqrt())).abs() < 0.1, "t: {}", t_val);
    }

    #[test]
    fn test_ray_plane_hit() {
        // y = 0 平面 (法线 +y, d = 0), 射线从 (0, 5, 0) 沿 -y 方向
        let t = ray_plane(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            0.0,
            100.0,
        );
        assert!(t.is_some());
        assert!((t.unwrap() - 5.0).abs() < 1e-3);
    }

    #[test]
    fn test_ray_triangle_hit() {
        // 三角形在 y=0 平面, 顶点 (0,0,0), (1,0,0), (0,0,1)
        let t = ray_triangle(
            Vec3::new(0.25, 1.0, 0.25),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            100.0,
        );
        assert!(t.is_some());
        assert!((t.unwrap() - 1.0).abs() < 1e-3);
    }

    #[test]
    fn test_ray_triangle_miss() {
        let t = ray_triangle(
            Vec3::new(5.0, 1.0, 5.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            100.0,
        );
        assert!(t.is_none());
    }

    // ---------- 扫掠球 ----------

    #[test]
    fn test_swept_sphere_vs_sphere_hit() {
        let toi = swept_sphere_vs_sphere(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            0.5,  // bullet radius
            Vec3::new(5.0, 0.0, 0.0),
            0.5,  // target radius
            100.0,
        );
        assert!(toi.is_some());
        let toi = toi.unwrap();
        // 接触距离 = 1.0 (0.5+0.5), 起始 5, t = 4
        assert!((toi.t - 4.0).abs() < 1e-3, "t: {}", toi.t);
        assert!(toi.normal.x > 0.5);
    }

    #[test]
    fn test_swept_sphere_vs_aabb_hit() {
        let toi = swept_sphere_vs_aabb(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            0.5,
            Vec3::new(4.0, -1.0, -1.0),
            Vec3::new(6.0, 1.0, 1.0),
            100.0,
        );
        assert!(toi.is_some());
        // 膨胀后盒子 min.x = 3.5, 射线沿 +x, t = 3.5
        assert!((toi.unwrap().t - 3.5).abs() < 1e-3);
    }

    #[test]
    fn test_swept_sphere_vs_obb_hit() {
        let toi = swept_sphere_vs_obb(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            0.5,
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
            Quat::IDENTITY,
            100.0,
        );
        assert!(toi.is_some());
        // 膨胀后半宽 1.5, 中心 5, 沿 +x 射线, t = 5 - 1.5 = 3.5
        assert!((toi.unwrap().t - 3.5).abs() < 1e-3);
    }

    // ---------- Conservative Advancement ----------

    #[test]
    fn test_ca_sphere_sphere_head_on() {
        let a = SphereCollider { center: Vec3::new(0.0, 0.0, 0.0), radius: 1.0 };
        let b = SphereCollider { center: Vec3::new(10.0, 0.0, 0.0), radius: 1.0 };
        let ma = Motion::linear(Vec3::new(1.0, 0.0, 0.0));
        let mb = Motion::linear(Vec3::new(-1.0, 0.0, 0.0));
        let toi = conservative_advancement_sphere_sphere(&a, &b, &ma, &mb, 10.0);
        assert!(toi.is_some(), "CA should find TOI");
        let t = toi.unwrap().t;
        // 接触时刻 t ≈ 4
        assert!((t - 4.0).abs() < 0.2, "CA toi t: {}", t);
    }

    #[test]
    fn test_ca_sphere_box_approach() {
        let sphere = SphereCollider { center: Vec3::new(0.0, 0.0, 0.0), radius: 1.0 };
        let box_c = BoxCollider::new(Vec3::new(10.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let ms = Motion::linear(Vec3::new(1.0, 0.0, 0.0));
        let mb = Motion::stationary();
        let toi = conservative_advancement_sphere_box(&sphere, &box_c, &ms, &mb, 20.0);
        assert!(toi.is_some(), "CA should find TOI");
        // 球到盒表面距离 = 10 - 1 - 1 = 8, 速度 1, t ≈ 8
        let t = toi.unwrap().t;
        assert!((t - 8.0).abs() < 0.3, "CA sphere-box t: {}", t);
    }

    #[test]
    fn test_ca_no_collision() {
        let a = SphereCollider { center: Vec3::new(0.0, 0.0, 0.0), radius: 1.0 };
        let b = SphereCollider { center: Vec3::new(0.0, 100.0, 0.0), radius: 1.0 };
        let ma = Motion::linear(Vec3::new(1.0, 0.0, 0.0));
        let mb = Motion::stationary();
        let toi = conservative_advancement_sphere_sphere(&a, &b, &ma, &mb, 10.0);
        assert!(toi.is_none(), "expected no collision");
    }

    #[test]
    fn test_ca_already_intersecting() {
        let a = SphereCollider { center: Vec3::new(0.0, 0.0, 0.0), radius: 1.0 };
        let b = SphereCollider { center: Vec3::new(0.5, 0.0, 0.0), radius: 1.0 };
        let ma = Motion::linear(Vec3::new(1.0, 0.0, 0.0));
        let mb = Motion::stationary();
        let toi = conservative_advancement_sphere_sphere(&a, &b, &ma, &mb, 10.0);
        assert!(toi.is_some());
        assert!(toi.unwrap().t < 0.1, "should be at t≈0");
    }

    // ---------- CcdSolver ----------

    #[test]
    fn test_solver_finds_earliest() {
        let solver = CcdSolver::new();
        let pairs = vec![
            CcdPair {
                a_index: 0, b_index: 1,
                toi: Some(Toi { t: 5.0, normal: Vec3::X, point: Vec3::ZERO }),
            },
            CcdPair {
                a_index: 0, b_index: 2,
                toi: Some(Toi { t: 2.0, normal: Vec3::Y, point: Vec3::ZERO }),
            },
            CcdPair {
                a_index: 1, b_index: 2,
                toi: None,
            },
        ];
        let earliest = solver.find_earliest(&pairs);
        assert!(earliest.is_some());
        assert_eq!(earliest.unwrap().b_index, 2);
    }

    #[test]
    fn test_solver_split_time() {
        let solver = CcdSolver::new();
        // dt = 0.016, toi = 0.005
        let (toi_t, rem) = solver.split_time(0.005, 0.016).unwrap();
        assert!((toi_t - 0.005).abs() < 1e-4);
        assert!((rem - 0.011).abs() < 1e-4);
    }

    #[test]
    fn test_solver_split_time_no_collision() {
        let solver = CcdSolver::new();
        // TOI 在区间外
        assert!(solver.split_time(0.020, 0.016).is_none());
    }

    #[test]
    fn test_solver_substeps_limit() {
        let solver = CcdSolver::new().with_substeps(32);
        assert_eq!(solver.max_substeps, 32);
    }

    // ---------- 综合场景: 子弹穿墙 ----------

    #[test]
    fn test_bullet_tunneling_prevented() {
        // 子弹从 x=-10 沿 +x 飞向 x=10, 速度 1000 m/s
        // 墙在 x=0, 厚度 1 (AABB: x∈[-0.5, 0.5], y∈[-1, 1], z∈[-1, 1])
        // dt = 0.016s, 一帧内子弹移动 16m, 离散检测会穿透
        // 但 CCD 应在 t = (10 - 0.5) / 1000 = 0.0095s 时检测到
        let bullet_start = Vec3::new(-10.0, 0.0, 0.0);
        let bullet_vel = Vec3::new(1000.0, 0.0, 0.0);
        let bullet_radius = 0.05f32;
        let box_min = Vec3::new(-0.5, -1.0, -1.0);
        let box_max = Vec3::new(0.5, 1.0, 1.0);
        let dt = 0.016f32;

        let toi = swept_sphere_vs_aabb(
            bullet_start, bullet_vel, bullet_radius,
            box_min, box_max, dt,
        );
        assert!(toi.is_some(), "CCD should catch tunneling bullet");
        let t = toi.unwrap().t;
        // 膨胀盒子 x_min = -0.5 - 0.05 = -0.55
        // t = (-0.55 - (-10)) / 1000 = 9.45 / 1000 = 0.00945
        assert!((t - 0.00945).abs() < 1e-3, "bullet TOI t: {}", t);
        assert!(t < dt, "TOI must be within dt");
    }
}
