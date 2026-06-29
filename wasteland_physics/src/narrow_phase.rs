use glam::Vec3;

const GJK_MAX_ITERATIONS: usize = 64;
const GJK_EPSILON: f32 = 1e-6;
const EPA_MAX_ITERATIONS: usize = 64;
const EPA_EPSILON: f32 = 1e-4;
const EPA_MAX_FACES: usize = 128;

#[derive(Debug, Clone)]
pub struct GjkResult {
    pub colliding: bool,
    pub closest_a: Vec3,
    pub closest_b: Vec3,
    pub separation: f32,
    pub simplex: Vec<Vec3>,
}

#[derive(Debug, Clone, Copy)]
pub struct EpaResult {
    pub penetration_depth: f32,
    pub contact_normal: Vec3,
    pub contact_point_a: Vec3,
    pub contact_point_b: Vec3,
}

pub trait ConvexShape {
    fn support(&self, direction: Vec3) -> Vec3;
    fn center(&self) -> Vec3;
    fn inertia_tensor(&self, mass: f32) -> glam::Mat3;
}

pub struct SphereShape {
    pub radius: f32,
}

impl ConvexShape for SphereShape {
    fn support(&self, direction: Vec3) -> Vec3 {
        let d = direction.normalize_or_zero();
        if d == Vec3::ZERO {
            return Vec3::ZERO;
        }
        d * self.radius
    }

    fn center(&self) -> Vec3 {
        Vec3::ZERO
    }

    fn inertia_tensor(&self, mass: f32) -> glam::Mat3 {
        let i = 0.4 * mass * self.radius * self.radius;
        glam::Mat3::from_diagonal(Vec3::new(i, i, i))
    }
}

pub struct BoxShape {
    pub half_extents: Vec3,
}

impl ConvexShape for BoxShape {
    fn support(&self, direction: Vec3) -> Vec3 {
        Vec3::new(
            self.half_extents.x.copysign(direction.x),
            self.half_extents.y.copysign(direction.y),
            self.half_extents.z.copysign(direction.z),
        )
    }

    fn center(&self) -> Vec3 {
        Vec3::ZERO
    }

    fn inertia_tensor(&self, mass: f32) -> glam::Mat3 {
        let h = self.half_extents;
        let factor = mass / 12.0;
        glam::Mat3::from_diagonal(Vec3::new(
            factor * (h.y * h.y + h.z * h.z),
            factor * (h.x * h.x + h.z * h.z),
            factor * (h.x * h.x + h.y * h.y),
        ))
    }
}

pub struct CapsuleShape {
    pub radius: f32,
    pub half_height: f32,
}

impl ConvexShape for CapsuleShape {
    fn support(&self, direction: Vec3) -> Vec3 {
        let d = direction.normalize_or_zero();
        if d == Vec3::ZERO {
            return Vec3::new(0.0, self.half_height, 0.0);
        }
        let tip = Vec3::new(0.0, self.half_height.copysign(d.y), 0.0);
        tip + d * self.radius
    }

    fn center(&self) -> Vec3 {
        Vec3::ZERO
    }

    fn inertia_tensor(&self, mass: f32) -> glam::Mat3 {
        let h = self.half_height;
        let r = self.radius;
        let cyl_mass = mass * 0.7;
        let cap_mass = mass * 0.3;
        let i_cyl = cyl_mass * (3.0 * r * r + h * h) / 12.0;
        let i_cap = cap_mass * (2.0 * r * r) / 5.0;
        glam::Mat3::from_diagonal(Vec3::new(i_cap, i_cyl, i_cap))
    }
}

pub struct ShapeTransform {
    pub translation: Vec3,
    pub rotation: glam::Quat,
}

impl ShapeTransform {
    pub fn identity() -> Self {
        Self { translation: Vec3::ZERO, rotation: glam::Quat::IDENTITY }
    }

    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        self.rotation.mul_vec3(point) + self.translation
    }

    pub fn inverse_transform_point(&self, point: Vec3) -> Vec3 {
        self.rotation.inverse().mul_vec3(point - self.translation)
    }

    pub fn transform_direction(&self, dir: Vec3) -> Vec3 {
        self.rotation.mul_vec3(dir)
    }
}

#[derive(Debug, Clone, Copy)]
struct MinkowskiVertex {
    point: Vec3,
    _support_a: Vec3,
    _support_b: Vec3,
}

pub fn gjk(
    shape_a: &dyn ConvexShape,
    transform_a: &ShapeTransform,
    shape_b: &dyn ConvexShape,
    transform_b: &ShapeTransform,
) -> GjkResult {
    let mut simplex: Vec<MinkowskiVertex> = Vec::with_capacity(4);
    let mut dir = Vec3::X;

    let v = minkowski_support(shape_a, transform_a, shape_b, transform_b, dir);
    simplex.push(v);
    dir = -v.point;

    for _ in 0..GJK_MAX_ITERATIONS {
        let v = minkowski_support(shape_a, transform_a, shape_b, transform_b, dir);
        if v.point.dot(dir) < GJK_EPSILON {
            return GjkResult {
                colliding: false,
                closest_a: Vec3::ZERO,
                closest_b: Vec3::ZERO,
                separation: 0.0,
                simplex: simplex.iter().map(|s| s.point).collect(),
            };
        }

        simplex.push(v);

        if do_simplex(&mut simplex, &mut dir) {
            return GjkResult {
                colliding: true,
                closest_a: Vec3::ZERO,
                closest_b: Vec3::ZERO,
                separation: 0.0,
                simplex: simplex.iter().map(|s| s.point).collect(),
            };
        }
    }

    GjkResult {
        colliding: false,
        closest_a: Vec3::ZERO,
        closest_b: Vec3::ZERO,
        separation: 0.0,
        simplex: simplex.iter().map(|s| s.point).collect(),
    }
}

fn do_simplex(simplex: &mut Vec<MinkowskiVertex>, dir: &mut Vec3) -> bool {
    let n = simplex.len();
    if n == 2 {
        let b = simplex[0].point;
        let a = simplex[1].point;
        let ao = -a;
        let ab = b - a;
        if ab.dot(ao) > 0.0 {
            *dir = triple_product(ab, ao, ab);
        } else {
            simplex.remove(0);
            *dir = ao;
        }
        return false;
    }

    if n == 3 {
        let c = simplex[0].point;
        let b = simplex[1].point;
        let a = simplex[2].point;
        let ao = -a;
        let ab = b - a;
        let ac = c - a;
        let abc = ab.cross(ac);

        if abc.cross(ac).dot(ao) >= 0.0 {
            if ac.dot(ao) >= 0.0 {
                simplex.remove(1);
                *dir = triple_product(ac, ao, ac);
            } else {
                simplex.remove(0);
                *dir = triple_product(ab, ao, ab);
            }
        } else {
            if ab.cross(abc).dot(ao) >= 0.0 {
                simplex.remove(0);
                *dir = triple_product(ab, ao, ab);
            } else {
                if abc.dot(ao) >= 0.0 {
                    *dir = abc;
                } else {
                    let tmp = simplex[0].point;
                    simplex[0] = MinkowskiVertex {
                        point: simplex[1].point,
                        _support_a: Vec3::ZERO,
                        _support_b: Vec3::ZERO,
                    };
                    simplex[1] = MinkowskiVertex {
                        point: tmp,
                        _support_a: Vec3::ZERO,
                        _support_b: Vec3::ZERO,
                    };
                    *dir = -abc;
                }
            }
        }
        return false;
    }

    if n == 4 {
        let d = simplex[0].point;
        let c = simplex[1].point;
        let b = simplex[2].point;
        let a = simplex[3].point;
        let ao = -a;
        let ab = b - a;
        let ac = c - a;
        let ad = d - a;

        let abc = ab.cross(ac);
        let acd = ac.cross(ad);
        let adb = ad.cross(ab);

        if abc.dot(ao) > 0.0 {
            simplex.remove(0);
            *dir = abc;
            return false;
        }
        if acd.dot(ao) > 0.0 {
            simplex.remove(1);
            *dir = acd;
            return false;
        }
        if adb.dot(ao) > 0.0 {
            simplex.remove(2);
            *dir = adb;
            return false;
        }
        return true;
    }

    false
}

fn triple_product(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let cross = a.cross(b);
    if cross.length_squared() < 1e-12 {
        return perpendicular(a);
    }
    cross.cross(c)
}

fn minkowski_support(
    shape_a: &dyn ConvexShape,
    transform_a: &ShapeTransform,
    shape_b: &dyn ConvexShape,
    transform_b: &ShapeTransform,
    direction: Vec3,
) -> MinkowskiVertex {
    let local_dir_a = transform_a.rotation.inverse().mul_vec3(direction);
    let local_dir_b = transform_b.rotation.inverse().mul_vec3(-direction);

    let support_a = shape_a.support(local_dir_a);
    let support_b = shape_b.support(local_dir_b);

    let world_a = transform_a.transform_point(support_a);
    let world_b = transform_b.transform_point(support_b);

    MinkowskiVertex { point: world_a - world_b, _support_a: world_a, _support_b: world_b }
}

fn perpendicular(v: Vec3) -> Vec3 {
    let abs_x = v.x.abs();
    let abs_y = v.y.abs();
    let abs_z = v.z.abs();

    if abs_x <= abs_y && abs_x <= abs_z {
        Vec3::new(0.0, -v.z, v.y)
    } else if abs_y <= abs_x && abs_y <= abs_z {
        Vec3::new(-v.z, 0.0, v.x)
    } else {
        Vec3::new(-v.y, v.x, 0.0)
    }
}

#[derive(Debug, Clone, Copy)]
struct EpaFace {
    a: usize,
    b: usize,
    c: usize,
    normal: Vec3,
    distance: f32,
}

pub fn epa(
    shape_a: &dyn ConvexShape,
    transform_a: &ShapeTransform,
    shape_b: &dyn ConvexShape,
    transform_b: &ShapeTransform,
    simplex: &[Vec3],
) -> Option<EpaResult> {
    if simplex.len() < 4 {
        return None;
    }

    let mut vertices: Vec<MinkowskiVertex> = Vec::new();
    let mut faces: Vec<EpaFace> = Vec::new();

    let init_verts = [simplex[0], simplex[1], simplex[2], simplex[3]];

    let mut raw_vertices: Vec<Vec3> = Vec::new();
    for &v in &init_verts {
        let mv = MinkowskiVertex { point: v, _support_a: Vec3::ZERO, _support_b: Vec3::ZERO };
        vertices.push(mv);
        raw_vertices.push(v);
    }

    faces.push(EpaFace { a: 0, b: 1, c: 2, normal: Vec3::ZERO, distance: 0.0 });
    faces.push(EpaFace { a: 0, b: 3, c: 1, normal: Vec3::ZERO, distance: 0.0 });
    faces.push(EpaFace { a: 0, b: 2, c: 3, normal: Vec3::ZERO, distance: 0.0 });
    faces.push(EpaFace { a: 1, b: 3, c: 2, normal: Vec3::ZERO, distance: 0.0 });

    for face in &mut faces {
        let ab = raw_vertices[face.b] - raw_vertices[face.a];
        let ac = raw_vertices[face.c] - raw_vertices[face.a];
        let n = ab.cross(ac);
        face.distance = n.length();
        if face.distance > 0.0 {
            face.normal = n / face.distance;
        }
    }

    for _ in 0..EPA_MAX_ITERATIONS {
        if faces.len() > EPA_MAX_FACES {
            break;
        }

        let mut closest_idx = 0;
        let mut closest_dist = f32::MAX;
        for (i, face) in faces.iter().enumerate() {
            let d = face.normal.dot(raw_vertices[face.a]);
            if d < closest_dist {
                closest_dist = d;
                closest_idx = i;
            }
        }

        let closest_face = faces[closest_idx];
        let search_dir = closest_face.normal;

        let v = minkowski_support(shape_a, transform_a, shape_b, transform_b, search_dir);
        let support_dist = v.point.dot(search_dir);

        if support_dist - closest_dist < EPA_EPSILON {
            let p = search_dir * closest_dist;
            let _contact_a = closest_face.normal;
            return Some(EpaResult {
                penetration_depth: closest_dist,
                contact_normal: -search_dir,
                contact_point_a: p,
                contact_point_b: p + search_dir * closest_dist,
            });
        }

        vertices.push(v);
        raw_vertices.push(v.point);
        let new_idx = vertices.len() - 1;

        let mut remove_faces = Vec::new();
        for (i, face) in faces.iter().enumerate() {
            if face.normal.dot(v.point - raw_vertices[face.a]) > 0.0 {
                remove_faces.push(i);
            }
        }

        let mut edge_map: std::collections::HashMap<(usize, usize), usize> =
            std::collections::HashMap::new();
        for &fi in &remove_faces {
            let face = &faces[fi];
            let edges = [
                (face.a.min(face.b), face.a.max(face.b)),
                (face.b.min(face.c), face.b.max(face.c)),
                (face.c.min(face.a), face.c.max(face.a)),
            ];
            for e in &edges {
                *edge_map.entry(*e).or_insert(0) += 1;
            }
        }

        remove_faces.sort_by(|a, b| b.cmp(a));
        for fi in &remove_faces {
            faces.remove(*fi);
        }

        for ((a, b), count) in &edge_map {
            if *count == 1 {
                let mut face =
                    EpaFace { a: *a, b: *b, c: new_idx, normal: Vec3::ZERO, distance: 0.0 };
                let ab = raw_vertices[face.b] - raw_vertices[face.a];
                let ac = raw_vertices[face.c] - raw_vertices[face.a];
                let n = ab.cross(ac);
                face.distance = n.length();
                if face.distance > 0.0 {
                    face.normal = n / face.distance;
                }
                if face.normal.dot(raw_vertices[face.a]) < 0.0 {
                    face.normal = -face.normal;
                    std::mem::swap(&mut face.b, &mut face.c);
                }
                faces.push(face);
            }
        }
    }

    None
}

pub fn sphere_sphere_collision(
    center_a: Vec3,
    radius_a: f32,
    center_b: Vec3,
    radius_b: f32,
) -> Option<(Vec3, f32, f32)> {
    let delta = center_b - center_a;
    let dist_sq = delta.length_squared();
    let sum_radius = radius_a + radius_b;

    if dist_sq > sum_radius * sum_radius {
        return None;
    }

    let dist = dist_sq.sqrt();
    if dist < 1e-6 {
        return Some((Vec3::Y, radius_a.min(radius_b), sum_radius));
    }

    let normal = delta / dist;
    let penetration = sum_radius - dist;
    Some((normal, penetration, dist))
}

pub fn sphere_box_collision(
    sphere_center: Vec3,
    sphere_radius: f32,
    box_center: Vec3,
    box_half_extents: Vec3,
) -> Option<(Vec3, f32)> {
    let local = sphere_center - box_center;
    let clamped = Vec3::new(
        local.x.clamp(-box_half_extents.x, box_half_extents.x),
        local.y.clamp(-box_half_extents.y, box_half_extents.y),
        local.z.clamp(-box_half_extents.z, box_half_extents.z),
    );
    let closest = box_center + clamped;
    let delta = sphere_center - closest;
    let dist_sq = delta.length_squared();

    if dist_sq > sphere_radius * sphere_radius {
        return None;
    }

    let dist = dist_sq.sqrt();
    if dist < 1e-6 {
        let mut max_extent = box_half_extents.x;
        let mut axis = Vec3::X;
        if box_half_extents.y > max_extent {
            max_extent = box_half_extents.y;
            axis = Vec3::Y;
        }
        if box_half_extents.z > max_extent {
            axis = Vec3::Z;
        }
        let local_clamped = Vec3::new(
            local.x.clamp(-box_half_extents.x, box_half_extents.x),
            local.y.clamp(-box_half_extents.y, box_half_extents.y),
            local.z.clamp(-box_half_extents.z, box_half_extents.z),
        );
        let normal = (local - local_clamped).normalize_or_zero();
        if normal == Vec3::ZERO {
            return Some((axis, sphere_radius));
        }
        return Some((normal, sphere_radius));
    }

    let normal = delta / dist;
    let penetration = sphere_radius - dist;
    Some((normal, penetration))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sphere_support() {
        let s = SphereShape { radius: 2.0 };
        let p = s.support(Vec3::X);
        assert!((p - Vec3::new(2.0, 0.0, 0.0)).length() < 0.001);
    }

    #[test]
    fn test_box_support() {
        let b = BoxShape { half_extents: Vec3::new(1.0, 2.0, 3.0) };
        let p = b.support(Vec3::new(1.0, -1.0, 1.0));
        assert!((p - Vec3::new(1.0, -2.0, 3.0)).length() < 0.001);
    }

    #[test]
    fn test_capsule_support() {
        let c = CapsuleShape { radius: 0.5, half_height: 2.0 };
        let p = c.support(Vec3::Y);
        assert!((p - Vec3::new(0.0, 2.5, 0.0)).length() < 0.001);
    }

    #[test]
    fn test_gjk_sphere_sphere_collision() {
        let s = SphereShape { radius: 1.0 };
        let ta = ShapeTransform {
            translation: Vec3::new(0.0, 0.0, 0.0),
            rotation: glam::Quat::IDENTITY,
        };
        let tb = ShapeTransform {
            translation: Vec3::new(1.5, 0.0, 0.0),
            rotation: glam::Quat::IDENTITY,
        };
        let result = gjk(&s, &ta, &s, &tb);
        assert!(result.colliding);
    }

    #[test]
    fn test_gjk_sphere_sphere_separated() {
        let s = SphereShape { radius: 1.0 };
        let ta = ShapeTransform {
            translation: Vec3::new(0.0, 0.0, 0.0),
            rotation: glam::Quat::IDENTITY,
        };
        let tb = ShapeTransform {
            translation: Vec3::new(5.0, 0.0, 0.0),
            rotation: glam::Quat::IDENTITY,
        };
        let result = gjk(&s, &ta, &s, &tb);
        assert!(!result.colliding);
    }

    #[test]
    fn test_gjk_box_box_collision() {
        let b = BoxShape { half_extents: Vec3::new(1.0, 1.0, 1.0) };
        let ta = ShapeTransform {
            translation: Vec3::new(0.0, 0.0, 0.0),
            rotation: glam::Quat::IDENTITY,
        };
        let tb = ShapeTransform {
            translation: Vec3::new(1.5, 0.0, 0.0),
            rotation: glam::Quat::IDENTITY,
        };
        let result = gjk(&b, &ta, &b, &tb);
        assert!(result.colliding);
    }

    #[test]
    fn test_sphere_sphere_collision_direct() {
        let result =
            sphere_sphere_collision(Vec3::new(0.0, 0.0, 0.0), 1.0, Vec3::new(1.5, 0.0, 0.0), 1.0);
        assert!(result.is_some());
        let (normal, penetration, _) = result.unwrap();
        assert!((penetration - 0.5).abs() < 0.001);
        assert!(normal.x > 0.0);
    }

    #[test]
    fn test_sphere_sphere_no_collision() {
        let result =
            sphere_sphere_collision(Vec3::new(0.0, 0.0, 0.0), 1.0, Vec3::new(5.0, 0.0, 0.0), 1.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_sphere_box_collision() {
        let result = sphere_box_collision(
            Vec3::new(2.0, 0.0, 0.0),
            1.0,
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
        );
        assert!(result.is_some());
    }

    #[test]
    fn test_sphere_box_no_collision() {
        let result = sphere_box_collision(
            Vec3::new(5.0, 0.0, 0.0),
            1.0,
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_inertia_tensor_sphere() {
        let s = SphereShape { radius: 2.0 };
        let i = s.inertia_tensor(10.0);
        let expected = 0.4 * 10.0 * 4.0;
        assert!((i.x_axis.x - expected).abs() < 0.001);
    }

    #[test]
    fn test_inertia_tensor_box() {
        let b = BoxShape { half_extents: Vec3::new(1.0, 2.0, 3.0) };
        let i = b.inertia_tensor(12.0);
        let expected_x = 4.0 + 9.0;
        let expected_y = 1.0 + 9.0;
        let expected_z = 1.0 + 4.0;
        assert!((i.x_axis.x - expected_x).abs() < 0.001);
        assert!((i.y_axis.y - expected_y).abs() < 0.001);
        assert!((i.z_axis.z - expected_z).abs() < 0.001);
    }
}
