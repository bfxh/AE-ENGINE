//! 相机：透视/正交投影，视图矩阵

use bytemuck::{Pod, Zeroable};

/// 投影类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraProjection {
    Perspective { fov_y: f32, aspect: f32, near: f32, far: f32 },
    Orthographic { left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32 },
}

impl CameraProjection {
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        Self::Perspective { fov_y, aspect, near, far }
    }

    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        Self::Orthographic { left, right, bottom, top, near, far }
    }

    pub fn projection_matrix(&self) -> [[f32; 4]; 4] {
        match self {
            Self::Perspective { fov_y, aspect, near, far } => {
                let f = 1.0 / (fov_y * 0.5).tan();
                let nf = 1.0 / (near - far);
                [
                    [f / aspect, 0.0, 0.0, 0.0],
                    [0.0, f, 0.0, 0.0],
                    [0.0, 0.0, (far + near) * nf, -1.0],
                    [0.0, 0.0, 2.0 * far * near * nf, 0.0],
                ]
            },
            Self::Orthographic { left, right, bottom, top, near, far } => {
                let rml = right - left;
                let rpl = right + left;
                let tmb = top - bottom;
                let tpb = top + bottom;
                let fmn = far - near;
                let fpn = far + near;
                [
                    [2.0 / rml, 0.0, 0.0, 0.0],
                    [0.0, 2.0 / tmb, 0.0, 0.0],
                    [0.0, 0.0, -2.0 / fmn, 0.0],
                    [-rpl / rml, -tpb / tmb, -fpn / fmn, 1.0],
                ]
            },
        }
    }

    pub fn aspect(&self) -> f32 {
        match self {
            Self::Perspective { aspect, .. } => *aspect,
            Self::Orthographic { left, right, .. } => (right - left).abs(),
        }
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        if let Self::Perspective { aspect: a, .. } = self {
            *a = aspect;
        }
    }
}

/// 相机：位置 + 朝向 + 投影
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub position: [f32; 3],
    pub forward: [f32; 3],
    pub up: [f32; 3],
    pub projection: CameraProjection,
}

impl Camera {
    pub fn new(position: [f32; 3], target: [f32; 3], projection: CameraProjection) -> Self {
        let forward =
            normalize([target[0] - position[0], target[1] - position[1], target[2] - position[2]]);
        Self { position, forward, up: [0.0, 1.0, 0.0], projection }
    }

    pub fn view_matrix(&self) -> [[f32; 4]; 4] {
        let f = self.forward;
        let s = normalize(cross(f, self.up));
        let u = cross(s, f);
        let p = self.position;
        [
            [s[0], u[0], -f[0], 0.0],
            [s[1], u[1], -f[1], 0.0],
            [s[2], u[2], -f[2], 0.0],
            [-dot(s, p), -dot(u, p), dot(f, p), 1.0],
        ]
    }

    pub fn view_projection(&self) -> [[f32; 4]; 4] {
        let view = self.view_matrix();
        let proj = self.projection.projection_matrix();
        mul_mat4(proj, view)
    }

    pub fn look_at(&mut self, target: [f32; 3]) {
        self.forward = normalize([
            target[0] - self.position[0],
            target[1] - self.position[1],
            target[2] - self.position[2],
        ]);
    }

    pub fn move_by(&mut self, delta: [f32; 3]) {
        self.position[0] += delta[0];
        self.position[1] += delta[1];
        self.position[2] += delta[2];
    }
}

/// GPU 上传用的相机 uniform
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub position: [f32; 4],
}

impl CameraUniform {
    pub fn from_camera(camera: &Camera) -> Self {
        Self {
            view_proj: camera.view_projection(),
            view: camera.view_matrix(),
            proj: camera.projection.projection_matrix(),
            position: [camera.position[0], camera.position[1], camera.position[2], 1.0],
        }
    }
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-10 {
        return [0.0, 0.0, 0.0];
    }
    [v[0] / len, v[1] / len, v[2] / len]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn mul_mat4(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut r = [[0.0f32; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += a[i][k] * b[k][j];
            }
            r[i][j] = sum;
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perspective_projection_matrix() {
        let p = CameraProjection::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
        let m = p.projection_matrix();
        assert!((m[0][0] - 1.0).abs() < 1e-5);
        assert!((m[1][1] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn orthographic_projection_matrix() {
        let p = CameraProjection::orthographic(-1.0, 1.0, -1.0, 1.0, 0.0, 10.0);
        let m = p.projection_matrix();
        assert!((m[0][0] - 1.0).abs() < 1e-5);
        assert!((m[1][1] - 1.0).abs() < 1e-5);
        assert!((m[2][2] - (-0.2)).abs() < 1e-5);
    }

    #[test]
    fn camera_view_matrix_basic() {
        let cam = Camera::new(
            [0.0, 0.0, 5.0],
            [0.0, 0.0, 0.0],
            CameraProjection::perspective(std::f32::consts::FRAC_PI_4, 1.0, 0.1, 100.0),
        );
        let v = cam.view_matrix();
        // 相机沿 -Z 看，所以 forward = [0,0,-1]
        assert!((cam.forward[0]).abs() < 1e-5);
        assert!((cam.forward[1]).abs() < 1e-5);
        assert!((cam.forward[2] + 1.0).abs() < 1e-5);
        // view 矩阵的平移部分应该把相机移到原点
        assert!((v[3][2] + 5.0).abs() < 1e-5);
    }

    #[test]
    fn camera_look_at_updates_forward() {
        let mut cam = Camera::new(
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            CameraProjection::perspective(std::f32::consts::FRAC_PI_4, 1.0, 0.1, 100.0),
        );
        assert!((cam.forward[0] - 1.0).abs() < 1e-5);
        cam.look_at([0.0, 1.0, 0.0]);
        assert!((cam.forward[1] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn camera_uniform_from_camera() {
        let cam = Camera::new(
            [0.0, 0.0, 5.0],
            [0.0, 0.0, 0.0],
            CameraProjection::perspective(std::f32::consts::FRAC_PI_4, 1.0, 0.1, 100.0),
        );
        let u = CameraUniform::from_camera(&cam);
        assert_eq!(u.position, [0.0, 0.0, 5.0, 1.0]);
    }

    #[test]
    fn normalize_zero_vector() {
        let v = normalize([0.0, 0.0, 0.0]);
        assert_eq!(v, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn cross_product_basic() {
        let c = cross([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        assert!((c[0]).abs() < 1e-5);
        assert!((c[1]).abs() < 1e-5);
        assert!((c[2] - 1.0).abs() < 1e-5);
    }
}
