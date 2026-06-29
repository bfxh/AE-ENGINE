//! SDF — Analytic Signed Distance Fields (解析符号距离场)
//!
//! 基于:
//! - Inigo Quilez. "Distance functions." https://iquilezles.org/articles/distfunctions/
//! - OGC (Offset Geometric Contact, SIGGRAPH 2025) 用 SDF 做碰撞检测,
//!   比 IPC 快 300x
//!
//! 用途:
//! 1. 碰撞检测: 点到 SDF 距离 < 0 = 穿透
//! 2. 渲染: ray marching (sphere tracing)
//! 3. 程序化建模: CSG (并/交/差) + 平滑混合
//!
//! 所有函数返回带符号距离: <0 内部, =0 表面, >0 外部

use glam::{Vec2, Vec3, Quat};

// ============================================================
// 基本体 SDF
// ============================================================

/// 球体 SDF
#[inline]
pub fn sdf_sphere(p: Vec3, center: Vec3, r: f32) -> f32 {
    (p - center).length() - r
}

/// 轴对齐盒 SDF (exact)
#[inline]
pub fn sdf_box(p: Vec3, center: Vec3, half_extents: Vec3) -> f32 {
    let q = (p - center).abs() - half_extents;
    let outside = q.max(Vec3::ZERO).length();
    let inside = q.max_element().min(0.0);
    outside + inside
}

/// 圆柱 SDF (Y 轴)
#[inline]
pub fn sdf_cylinder(p: Vec3, center: Vec3, radius: f32, half_height: f32) -> f32 {
    let d = Vec2::new(
        (Vec2::new(p.x, p.z) - Vec2::new(center.x, center.z)).length() - radius,
        (p.y - center.y).abs() - half_height,
    );
    d.x.max(d.y).min(0.0) + d.max(Vec2::ZERO).length()
}

/// 胶囊 SDF (两端点 a, b, 半径 r)
#[inline]
pub fn sdf_capsule(p: Vec3, a: Vec3, b: Vec3, r: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let denom = ba.dot(ba).max(1e-10);
    let h = (pa.dot(ba) / denom).clamp(0.0, 1.0);
    (pa - ba * h).length() - r
}

/// 平面 SDF (法向 n, 过点 point)
#[inline]
pub fn sdf_plane(p: Vec3, point: Vec3, normal: Vec3) -> f32 {
    (p - point).dot(normal)
}

/// 圆环 SDF (Y 轴, 主半径 radius, 管半径 tube)
#[inline]
pub fn sdf_torus(p: Vec3, center: Vec3, radius: f32, tube: f32) -> f32 {
    let q = Vec2::new(
        (Vec2::new(p.x, p.z) - Vec2::new(center.x, center.z)).length() - radius,
        p.y - center.y,
    );
    q.length() - tube
}

/// 圆锥 SDF (Y 轴, 高度 h, 底面半径 r)
#[inline]
pub fn sdf_cone(p: Vec3, center: Vec3, r: f32, h: f32) -> f32 {
    let q = Vec2::new((p.x - center.x).hypot(p.z - center.z), p.y - center.y);
    let ca = Vec2::new(r, -h);
    let cb = Vec2::new(q.x - r, q.y + h);
    let s = (q.x - ca.x).max(-ca.y).max(q.y + h).max(0.0);
    let d = q.min(ca).dot(cb.normalize_or_zero().min(Vec2::ZERO));
    d.min(0.0).abs() + s
}

// ============================================================
// 组合操作 (CSG)
// ============================================================

#[inline]
pub fn sdf_union(d1: f32, d2: f32) -> f32 { d1.min(d2) }

#[inline]
pub fn sdf_intersection(d1: f32, d2: f32) -> f32 { d1.max(d2) }

/// d1 减去 d2
#[inline]
pub fn sdf_subtraction(d1: f32, d2: f32) -> f32 { d1.max(-d2) }

// ============================================================
// 平滑组合 (指数平滑, k = 平滑半径)
// ============================================================

#[inline]
pub fn sdf_smooth_union(d1: f32, d2: f32, k: f32) -> f32 {
    let h = (0.5 + 0.5 * (d2 - d1) / k).clamp(0.0, 1.0);
    d2 + (d1 - d2) * h - k * h * (1.0 - h)
}

#[inline]
pub fn sdf_smooth_intersection(d1: f32, d2: f32, k: f32) -> f32 {
    let h = (0.5 - 0.5 * (d2 - d1) / k).clamp(0.0, 1.0);
    d2 + (d1 - d2) * h + k * h * (1.0 - h)
}

#[inline]
pub fn sdf_smooth_subtraction(d1: f32, d2: f32, k: f32) -> f32 {
    let h = (0.5 - 0.5 * (d2 + d1) / k).clamp(0.0, 1.0);
    d2 + (-d1 - d2) * h + k * h * (1.0 - h)
}

// ============================================================
// 变换
// ============================================================

#[inline]
pub fn sdf_translate(p: Vec3, offset: Vec3) -> Vec3 { p - offset }

#[inline]
pub fn sdf_rotate(p: Vec3, rotation: Quat) -> Vec3 { rotation.inverse() * p }

#[inline]
pub fn sdf_scale(p: Vec3, scale: f32) -> Vec3 { p / scale }

// ============================================================
// 法向和曲率
// ============================================================

/// SDF 法向 (中心差分, O(eps^2) 精度)
pub fn sdf_normal<F: Fn(Vec3) -> f32>(sdf: &F, p: Vec3, eps: f32) -> Vec3 {
    Vec3::new(
        sdf(Vec3::new(p.x + eps, p.y, p.z)) - sdf(Vec3::new(p.x - eps, p.y, p.z)),
        sdf(Vec3::new(p.x, p.y + eps, p.z)) - sdf(Vec3::new(p.x, p.y - eps, p.z)),
        sdf(Vec3::new(p.x, p.y, p.z + eps)) - sdf(Vec3::new(p.x, p.y, p.z - eps)),
    ).normalize_or_zero()
}

/// SDF 曲率 (Laplacian, 中心差分)
pub fn sdf_curvature<F: Fn(Vec3) -> f32>(sdf: &F, p: Vec3, eps: f32) -> f32 {
    let e = eps;
    let l = sdf(p);
    let dx = sdf(Vec3::new(p.x + e, p.y, p.z)) + sdf(Vec3::new(p.x - e, p.y, p.z)) - 2.0 * l;
    let dy = sdf(Vec3::new(p.x, p.y + e, p.z)) + sdf(Vec3::new(p.x, p.y - e, p.z)) - 2.0 * l;
    let dz = sdf(Vec3::new(p.x, p.y, p.z + e)) + sdf(Vec3::new(p.x, p.y, p.z - e)) - 2.0 * l;
    (dx + dy + dz) / (e * e)
}

// ============================================================
// Sphere Tracing (Ray Marching)
// ============================================================

/// Sphere tracing 求射线与 SDF 的交点
/// 返回 (t, hit): t = 参数, hit = 是否命中
pub fn sphere_trace<F: Fn(Vec3) -> f32>(
    sdf: &F,
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
    max_steps: usize,
    eps: f32,
) -> Option<f32> {
    let dir = dir.normalize_or_zero();
    let mut t = 0.0f32;
    for _ in 0..max_steps {
        let p = origin + dir * t;
        let d = sdf(p);
        if d < eps {
            return Some(t);
        }
        t += d;
        if t > max_dist {
            return None;
        }
    }
    None
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdf_sphere() {
        // 中心点在原点, 半径 1
        let sdf = |p: Vec3| sdf_sphere(p, Vec3::ZERO, 1.0);
        assert!(sdf(Vec3::ZERO) < -0.9, "center should be inside");
        assert!((sdf(Vec3::new(2.0, 0.0, 0.0)) - 1.0).abs() < 1e-4, "outside distance");
        assert!((sdf(Vec3::new(1.0, 0.0, 0.0))).abs() < 1e-4, "on surface");
    }

    #[test]
    fn test_sdf_box() {
        let sdf = |p: Vec3| sdf_box(p, Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        assert!(sdf(Vec3::ZERO) < -0.9, "center inside");
        assert!((sdf(Vec3::new(2.0, 0.0, 0.0)) - 1.0).abs() < 1e-4, "outside 1 unit");
        assert!(sdf(Vec3::new(0.5, 0.5, 0.5)) < 0.0, "corner inside");
    }

    #[test]
    fn test_sdf_cylinder() {
        let sdf = |p: Vec3| sdf_cylinder(p, Vec3::ZERO, 1.0, 1.0);
        assert!(sdf(Vec3::ZERO) < -0.9, "center inside");
        assert!(sdf(Vec3::new(0.0, 2.0, 0.0)) > 0.0, "above top");
        assert!((sdf(Vec3::new(2.0, 0.0, 0.0)) - 1.0).abs() < 1e-4, "outside radius");
    }

    #[test]
    fn test_sdf_capsule() {
        let sdf = |p: Vec3| sdf_capsule(p, Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 1.0, 0.0), 0.5);
        assert!(sdf(Vec3::ZERO) < -0.4, "center inside");
        assert!((sdf(Vec3::new(0.0, 1.5, 0.0))).abs() < 1e-4, "on top cap");
        assert!(sdf(Vec3::new(2.0, 0.0, 0.0)) > 0.0, "outside");
    }

    #[test]
    fn test_sdf_torus() {
        let sdf = |p: Vec3| sdf_torus(p, Vec3::ZERO, 2.0, 0.5);
        // 中心 (远离环) 距离 = 2 - 0.5 = 1.5
        assert!(sdf(Vec3::ZERO) > 1.0, "center outside torus");
        // 环表面一点 (主半径 2 + 管半径 0.5 = 2.5)
        assert!((sdf(Vec3::new(2.5, 0.0, 0.0))).abs() < 1e-3, "on torus surface");
    }

    #[test]
    fn test_sdf_plane() {
        let sdf = |p: Vec3| sdf_plane(p, Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0));
        assert!((sdf(Vec3::new(0.0, 1.0, 0.0)) - 1.0).abs() < 1e-4, "1 above");
        assert!((sdf(Vec3::new(0.0, -2.0, 0.0)) + 2.0).abs() < 1e-4, "2 below");
    }

    #[test]
    fn test_sdf_union() {
        // 两球重叠 (球心 ±0.5, 半径 1), 原点在重叠区
        let d1 = sdf_sphere(Vec3::new(0.0, 0.0, 0.0), Vec3::new(-0.5, 0.0, 0.0), 1.0);
        let d2 = sdf_sphere(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.5, 0.0, 0.0), 1.0);
        let u = sdf_union(d1, d2);
        assert!(u < 0.0, "union should be inside overlapping spheres: u={}", u);
        assert_eq!(u, d1.min(d2));
    }

    #[test]
    fn test_sdf_subtraction() {
        // 大球减小球
        let d_big = sdf_sphere(Vec3::new(0.0, 0.0, 0.0), Vec3::ZERO, 2.0);
        let d_small = sdf_sphere(Vec3::new(0.0, 0.0, 0.0), Vec3::ZERO, 1.0);
        let sub = sdf_subtraction(d_big, d_small);
        // 在小球内部, sub = max(d_big, -d_small) = max(-2, 1) = 1 (被减去)
        assert!(sub > 0.0, "inside small sphere should be removed");
    }

    #[test]
    fn test_sdf_smooth_union() {
        let d1 = 0.0f32; // 表面
        let d2 = 0.0f32; // 表面
        let u = sdf_smooth_union(d1, d2, 0.5);
        // 平滑并应 <= 硬并
        assert!(u <= sdf_union(d1, d2) + 1e-4, "smooth union <= hard union");
        assert!(u < 0.0, "smooth union of two surfaces should be inside");
    }

    #[test]
    fn test_sdf_normal() {
        let sdf = |p: Vec3| sdf_sphere(p, Vec3::ZERO, 1.0);
        let n = sdf_normal(&sdf, Vec3::new(1.0, 0.0, 0.0), 1e-4);
        // 球面 (1,0,0) 处法向应朝 (1,0,0)
        assert!((n - Vec3::new(1.0, 0.0, 0.0)).length() < 0.01, "normal should be +x: {:?}", n);
    }

    #[test]
    fn test_sdf_normal_box() {
        let sdf = |p: Vec3| sdf_box(p, Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        let n = sdf_normal(&sdf, Vec3::new(2.0, 0.0, 0.0), 1e-4);
        // 盒面 (2,0,0) 处法向应朝 (1,0,0)
        assert!((n - Vec3::new(1.0, 0.0, 0.0)).length() < 0.01, "box normal +x: {:?}", n);
    }

    #[test]
    fn test_sphere_trace_hit() {
        let sdf = |p: Vec3| sdf_sphere(p, Vec3::ZERO, 1.0);
        // 射线从 (3,0,0) 朝 -x 方向, 应命中球
        let hit = sphere_trace(&sdf, Vec3::new(3.0, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0), 10.0, 100, 1e-4);
        assert!(hit.is_some(), "should hit sphere");
        let t = hit.unwrap();
        assert!((t - 2.0).abs() < 0.01, "hit distance ~2: {}", t);
    }

    #[test]
    fn test_sphere_trace_miss() {
        let sdf = |p: Vec3| sdf_sphere(p, Vec3::ZERO, 1.0);
        // 射线从 (3,3,0) 朝 -x, 应错过球
        let hit = sphere_trace(&sdf, Vec3::new(3.0, 3.0, 0.0), Vec3::new(-1.0, 0.0, 0.0), 10.0, 100, 1e-4);
        assert!(hit.is_none(), "should miss sphere");
    }

    #[test]
    fn test_sdf_curvature_sphere() {
        let sdf = |p: Vec3| sdf_sphere(p, Vec3::ZERO, 2.0);
        // 球面 SDF Laplacian = 2/r = 1.0 (r=2)
        // f32 精度限制: eps 太小会导致 catastrophic cancellation
        // eps=1e-3 给 0.834 (误差 17%), eps=1e-2 给 1.001 (误差 0.1%)
        let c = sdf_curvature(&sdf, Vec3::new(2.0, 0.0, 0.0), 1e-2);
        assert!((c - 1.0).abs() < 0.05, "sphere Laplacian ~2/r=1.0: {}", c);
    }
}
