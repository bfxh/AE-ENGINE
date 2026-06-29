//! Editor camera controller with orbit, pan, and zoom.
//!
//! Provides a free-fly / orbit camera suitable for a 3D scene editor viewport.

use glam::{Mat4, Vec3};

/// Editor camera supporting orbit, pan, and zoom operations.
#[derive(Debug, Clone)]
pub struct EditorCamera {
    /// Camera position in world space.
    pub position: Vec3,
    /// Target point the camera orbits around.
    pub target: Vec3,
    /// Up vector.
    pub up: Vec3,
    /// Field of view in degrees.
    pub fov: f32,
    /// Near clipping plane.
    pub near: f32,
    /// Far clipping plane.
    pub far: f32,
    /// Yaw angle in radians (rotation around world up).
    pub yaw: f32,
    /// Pitch angle in radians (rotation around local right).
    pub pitch: f32,
    /// Distance from target (for orbit mode).
    pub distance: f32,
    /// Sensitivity for rotation.
    pub sensitivity: f32,
    /// Sensitivity for panning.
    pub pan_sensitivity: f32,
    /// Sensitivity for zoom.
    pub zoom_sensitivity: f32,
    /// Movement speed for WASD fly mode (units per second).
    pub move_speed: f32,
    /// Whether to invert Y axis on orbit (flight-sim style).
    pub invert_y: bool,
}

impl Default for EditorCamera {
    fn default() -> Self {
        Self {
            position: Vec3::new(5.0, 5.0, 5.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov: 60.0,
            near: 0.1,
            far: 1000.0,
            yaw: -std::f32::consts::FRAC_PI_4,
            pitch: -std::f32::consts::FRAC_PI_4,
            distance: 8.66,
            sensitivity: 0.005,
            pan_sensitivity: 0.005,
            zoom_sensitivity: 0.1,
            move_speed: 5.0,
            invert_y: false,
        }
    }
}

impl EditorCamera {
    /// Compute the view matrix (world → camera).
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    /// Compute the projection matrix.
    pub fn projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov.to_radians(), aspect_ratio, self.near, self.far)
    }

    /// Compute combined view-projection matrix.
    pub fn view_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        self.projection_matrix(aspect_ratio) * self.view_matrix()
    }

    /// Get the forward direction (from position toward target).
    pub fn forward(&self) -> Vec3 {
        (self.target - self.position).normalize()
    }

    /// Get the right direction.
    pub fn right(&self) -> Vec3 {
        self.forward().cross(self.up).normalize()
    }

    /// Get the camera up direction.
    pub fn camera_up(&self) -> Vec3 {
        self.right().cross(self.forward()).normalize()
    }

    /// Orbit the camera around the target by delta mouse movement.
    /// `delta_x`: horizontal mouse delta in pixels.
    /// `delta_y`: vertical mouse delta in pixels.
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        let dy = if self.invert_y { delta_y } else { -delta_y };
        self.yaw -= delta_x * self.sensitivity;
        self.pitch -= dy * self.sensitivity;

        // Clamp pitch to avoid flipping.
        let max_pitch = std::f32::consts::FRAC_PI_2 - 0.01;
        self.pitch = self.pitch.clamp(-max_pitch, max_pitch);

        self.update_position_from_orbit();
    }

    /// Pan the camera in the view plane.
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let right = self.right();
        let up = self.camera_up();
        let pan_offset = (right * delta_x + up * delta_y) * self.pan_sensitivity * self.distance;
        self.target += pan_offset;
        self.position += pan_offset;
    }

    /// Zoom the camera in/out.
    pub fn zoom(&mut self, delta: f32) {
        self.distance -= delta * self.zoom_sensitivity * self.distance;
        self.distance = self.distance.clamp(0.1, 1000.0);
        self.update_position_from_orbit();
    }

    /// Fly the camera using WASD-style inputs.
    /// `forward`: +1 = forward (W), -1 = backward (S).
    /// `right`: +1 = right (D), -1 = left (A).
    /// `up`: +1 = up (Space), -1 = down (Shift/Ctrl).
    /// `dt`: delta time in seconds.
    pub fn fly(&mut self, forward: f32, right: f32, up: f32, dt: f32) {
        let f = self.forward() * forward;
        let r = self.right() * right;
        let u = self.up * up;
        let offset = (f + r + u) * self.move_speed * dt;
        self.position += offset;
        self.target += offset;
    }

    /// Focus the camera on a specific point.
    pub fn focus_on(&mut self, point: Vec3) {
        self.target = point;
        self.update_position_from_orbit();
    }

    /// Set orbit angles (yaw, pitch) directly and recompute position.
    pub fn set_orbit_angles(&mut self, yaw: f32, pitch: f32) {
        self.yaw = yaw;
        let max_pitch = std::f32::consts::FRAC_PI_2 - 0.01;
        self.pitch = pitch.clamp(-max_pitch, max_pitch);
        self.update_position_from_orbit();
    }

    /// Update position based on yaw, pitch, and distance (orbit mode).
    fn update_position_from_orbit(&mut self) {
        let direction = Vec3::new(
            self.pitch.cos() * self.yaw.cos(),
            self.pitch.sin(),
            self.pitch.cos() * self.yaw.sin(),
        );
        self.position = self.target - direction * self.distance;
    }

    /// Get the forward ray from the camera through a screen point.
    /// `screen_x`, `screen_y`: normalized device coordinates (-1 to 1).
    /// `aspect_ratio`: viewport width / height.
    pub fn screen_to_ray(&self, screen_x: f32, screen_y: f32, aspect_ratio: f32) -> (Vec3, Vec3) {
        let proj = self.projection_matrix(aspect_ratio);
        let inv_proj = proj.inverse();
        let inv_view = self.view_matrix().inverse();

        // Unproject near and far points.
        let near_point = glam::Vec4::new(screen_x, screen_y, 0.0, 1.0);
        let far_point = glam::Vec4::new(screen_x, screen_y, 1.0, 1.0);

        let near_world = inv_view * inv_proj * near_point;
        let far_world = inv_view * inv_proj * far_point;

        let origin = near_world.truncate() / near_world.w;
        let far = far_world.truncate() / far_world.w;
        let direction = (far - origin).normalize();

        (origin, direction)
    }
}
