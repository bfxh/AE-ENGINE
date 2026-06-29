use glam::{Mat4, Vec3, Vec4};

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self { origin, direction: direction.normalize_or_zero() }
    }

    pub fn point_at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }

    pub fn intersects_aabb(&self, min: Vec3, max: Vec3) -> Option<f32> {
        let inv_d = Vec3::new(
            if self.direction.x != 0.0 { 1.0 / self.direction.x } else { f32::INFINITY },
            if self.direction.y != 0.0 { 1.0 / self.direction.y } else { f32::INFINITY },
            if self.direction.z != 0.0 { 1.0 / self.direction.z } else { f32::INFINITY },
        );

        let t1 = (min - self.origin) * inv_d;
        let t2 = (max - self.origin) * inv_d;

        let tmin = t1.min(t2);
        let tmax = t1.max(t2);

        let tenter = tmin.x.max(tmin.y).max(tmin.z);
        let texit = tmax.x.min(tmax.y).min(tmax.z);

        if texit < 0.0 || tenter > texit {
            return None;
        }

        if tenter < 0.0 { Some(texit) } else { Some(tenter) }
    }

    pub fn intersects_sphere(&self, center: Vec3, radius: f32) -> Option<f32> {
        let oc = self.origin - center;
        let a = self.direction.dot(self.direction);
        let b = 2.0 * oc.dot(self.direction);
        let c = oc.dot(oc) - radius * radius;
        let discriminant = b * b - 4.0 * a * c;

        if discriminant < 0.0 {
            return None;
        }

        let sqrt_d = discriminant.sqrt();
        let t1 = (-b - sqrt_d) / (2.0 * a);
        let t2 = (-b + sqrt_d) / (2.0 * a);

        if t1 >= 0.0 {
            Some(t1)
        } else if t2 >= 0.0 {
            Some(t2)
        } else {
            None
        }
    }

    pub fn intersects_plane(&self, point: Vec3, normal: Vec3) -> Option<f32> {
        let denom = normal.dot(self.direction);
        if denom.abs() < 1e-6 {
            return None;
        }
        let t = (point - self.origin).dot(normal) / denom;
        if t >= 0.0 { Some(t) } else { None }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PickAxis {
    None,
    X,
    Y,
    Z,
}

pub struct RayPicker {
    pub ray: Ray,
}

impl RayPicker {
    pub fn from_screen(
        screen_x: f32,
        screen_y: f32,
        width: f32,
        height: f32,
        view_proj: Mat4,
    ) -> Self {
        let ndc_x = 2.0 * screen_x / width - 1.0;
        let ndc_y = 1.0 - 2.0 * screen_y / height;

        let inv_vp = view_proj.inverse();

        let near_point = inv_vp * Vec4::new(ndc_x, ndc_y, -1.0, 1.0);
        let far_point = inv_vp * Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

        let near = near_point.truncate() / near_point.w;
        let far = far_point.truncate() / far_point.w;

        let direction = far - near;
        let ray = Ray::new(near, direction);

        RayPicker { ray }
    }

    pub fn pick_aabb(&self, center: Vec3, half_extents: Vec3) -> Option<f32> {
        let min = center - half_extents;
        let max = center + half_extents;
        self.ray.intersects_aabb(min, max)
    }

    pub fn pick_sphere(&self, center: Vec3, radius: f32) -> Option<f32> {
        self.ray.intersects_sphere(center, radius)
    }

    pub fn pick_axis_gizmo(
        &self,
        gizmo_origin: Vec3,
        gizmo_size: f32,
        camera_pos: Vec3,
    ) -> PickAxis {
        let axes = [
            (PickAxis::X, Vec3::new(1.0, 0.0, 0.0)),
            (PickAxis::Y, Vec3::new(0.0, 1.0, 0.0)),
            (PickAxis::Z, Vec3::new(0.0, 0.0, 1.0)),
        ];

        let mut best = PickAxis::None;
        let mut best_dist = f32::INFINITY;

        for (axis, dir) in axes.iter() {
            let axis_center = gizmo_origin + *dir * gizmo_size * 0.5;
            let to_camera = (camera_pos - axis_center).normalize();
            let screen_radius = gizmo_size * 0.15;

            if let Some(t) = self.ray.intersects_sphere(axis_center, screen_radius) {
                if t < best_dist {
                    best_dist = t;
                    best = *axis;
                }
            }

            let perp = dir.cross(&to_camera).normalize();
            let offset = perp * gizmo_size * 0.05;
            let p1 = gizmo_origin + offset;
            let p2 = gizmo_origin + *dir * gizmo_size - offset;
            if let Some(t) = self.ray.intersects_cylinder(p1, p2, gizmo_size * 0.03) {
                if t < best_dist {
                    best_dist = t;
                    best = *axis;
                }
            }
        }

        best
    }

    pub fn pick_ground_plane(&self, plane_y: f32) -> Option<Vec3> {
        let plane_point = Vec3::new(0.0, plane_y, 0.0);
        let plane_normal = Vec3::new(0.0, 1.0, 0.0);
        self.ray.intersects_plane(plane_point, plane_normal).map(|t| self.ray.point_at(t))
    }
}

trait Vec3Ext {
    fn cross(&self, other: &Vec3) -> Vec3;
}

impl Vec3Ext for Vec3 {
    fn cross(&self, other: &Vec3) -> Vec3 {
        Vec3::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }
}

trait RayExt {
    fn intersects_cylinder(&self, p1: Vec3, p2: Vec3, radius: f32) -> Option<f32>;
}

impl RayExt for Ray {
    fn intersects_cylinder(&self, p1: Vec3, p2: Vec3, radius: f32) -> Option<f32> {
        let axis = p2 - p1;
        let axis_len = axis.length();
        if axis_len < 1e-6 {
            return None;
        }
        let axis_dir = axis / axis_len;

        let m = self.origin - p1;
        let d_dot_a = self.direction.dot(axis_dir);
        let m_dot_a = m.dot(axis_dir);

        let d_perp = self.direction - axis_dir * d_dot_a;
        let m_perp = m - axis_dir * m_dot_a;

        let a = d_perp.dot(d_perp);
        let b = 2.0 * d_perp.dot(m_perp);
        let c = m_perp.dot(m_perp) - radius * radius;

        if a.abs() < 1e-8 {
            return None;
        }

        let discriminant = b * b - 4.0 * a * c;
        if discriminant < 0.0 {
            return None;
        }

        let sqrt_d = discriminant.sqrt();
        let t1 = (-b - sqrt_d) / (2.0 * a);
        let t2 = (-b + sqrt_d) / (2.0 * a);

        for t in [t1, t2].iter() {
            if *t < 0.0 {
                continue;
            }
            let hit = self.origin + self.direction * *t;
            let proj = (hit - p1).dot(axis_dir);
            if proj >= 0.0 && proj <= axis_len {
                return Some(*t);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ray_aabb_hit() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0));
        let t = ray.intersects_aabb(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        assert!(t.is_some(), "ray should hit AABB");
        assert!((t.unwrap() - 4.0).abs() < 0.01, "hit distance should be 4.0, got {}", t.unwrap());
    }

    #[test]
    fn test_ray_aabb_miss() {
        let ray = Ray::new(Vec3::new(5.0, 5.0, -5.0), Vec3::new(0.0, 0.0, 1.0));
        let t = ray.intersects_aabb(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        assert!(t.is_none(), "ray should miss AABB");
    }

    #[test]
    fn test_ray_sphere_hit() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0));
        let t = ray.intersects_sphere(Vec3::new(0.0, 0.0, 0.0), 1.0);
        assert!(t.is_some(), "ray should hit sphere");
        assert!((t.unwrap() - 4.0).abs() < 0.01, "hit distance should be 4.0, got {}", t.unwrap());
    }

    #[test]
    fn test_ray_plane_hit() {
        let ray = Ray::new(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
        let t = ray.intersects_plane(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
        assert!(t.is_some(), "ray should hit plane");
        assert!((t.unwrap() - 5.0).abs() < 0.01, "hit distance should be 5.0, got {}", t.unwrap());
    }

    #[test]
    fn test_ray_cylinder_hit() {
        let ray = Ray::new(Vec3::new(0.0, 0.5, -5.0), Vec3::new(0.0, 0.0, 1.0));
        let t = ray.intersects_cylinder(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), 0.1);
        assert!(t.is_some(), "ray should hit cylinder");
    }
}
