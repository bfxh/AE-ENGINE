//! Transform gizmo for interactive translate/rotate/scale in the 3D viewport.
//!
//! - W key: Translate mode (3 axis arrows)
//! - E key: Rotate mode (3 axis rings)
//! - R key: Scale mode (3 axis handles)

use glam::{Mat4, Quat, Vec3, Vec4};

/// The operation mode of the gizmo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GizmoMode {
    #[default]
    Translate,
    Rotate,
    Scale,
}


impl GizmoMode {
    /// Cycle to the next mode.
    pub fn next(self) -> Self {
        match self {
            GizmoMode::Translate => GizmoMode::Rotate,
            GizmoMode::Rotate => GizmoMode::Scale,
            GizmoMode::Scale => GizmoMode::Translate,
        }
    }

    /// Get a human-readable name.
    pub fn name(self) -> &'static str {
        match self {
            GizmoMode::Translate => "Translate (W)",
            GizmoMode::Rotate => "Rotate (E)",
            GizmoMode::Scale => "Scale (R)",
        }
    }
}

/// State for the interactive transform gizmo.
#[derive(Debug, Default)]
pub struct Gizmo {
    /// Current operation mode.
    pub mode: GizmoMode,
    /// Whether the gizmo is currently being dragged.
    pub dragging: bool,
    /// Which axis is being dragged (0=X, 1=Y, 2=Z, None=no axis).
    pub active_axis: Option<usize>,
    /// Snapshot of the transform before drag started: (translation, scale).
    pub drag_start_transform: Option<(Vec3, Vec3)>,
    /// Snapshot of the rotation before drag started.
    pub drag_start_rotation: Option<Quat>,
    /// For translate/scale: initial parameter t along the axis line at drag start.
    pub drag_start_t: Option<f32>,
    /// For rotate: initial vector from gizmo origin to the hit point on the rotation plane.
    pub drag_start_vec: Option<Vec3>,
}

impl Gizmo {
    pub fn new() -> Self {
        Self::default()
    }

    // ------------------------------------------------------------------
    // Drag interaction
    // ------------------------------------------------------------------

    /// Begin a drag on the given axis (0=X, 1=Y, 2=Z).
    ///
    /// Captures the node's current transform so deltas can be computed
    /// relative to the start state.
    pub fn begin_drag(
        &mut self,
        axis: usize,
        start_translation: Vec3,
        start_scale: Vec3,
        start_rotation: Quat,
    ) {
        self.dragging = true;
        self.active_axis = Some(axis);
        self.drag_start_transform = Some((start_translation, start_scale));
        self.drag_start_rotation = Some(start_rotation);
        self.drag_start_t = None;
        self.drag_start_vec = None;
    }

    /// Capture the initial pick point at drag start.
    ///
    /// Must be called right after `begin_drag` with the mouse ray at
    /// the moment the user pressed the button.
    pub fn capture_drag_start(&mut self, ray_origin: Vec3, ray_dir: Vec3) {
        let (start_pos, _) = match self.drag_start_transform {
            Some(t) => t,
            None => return,
        };
        let axis = match self.active_axis_dir() {
            Some(a) => a,
            None => return,
        };
        match self.mode {
            GizmoMode::Translate | GizmoMode::Scale => {
                self.drag_start_t = ray_ray_closest_t(start_pos, axis, ray_origin, ray_dir);
            },
            GizmoMode::Rotate => {
                // Project the initial mouse ray onto the rotation plane.
                let to_plane = start_pos - ray_origin;
                let denom = axis.dot(ray_dir);
                if denom.abs() > 1e-6 {
                    let t = to_plane.dot(axis) / denom;
                    if t >= 0.0 {
                        let hit = ray_origin + ray_dir * t;
                        self.drag_start_vec = Some(hit - start_pos);
                    }
                }
            },
        }
    }

    /// End the current drag operation.
    pub fn end_drag(&mut self) {
        self.dragging = false;
        self.active_axis = None;
        self.drag_start_transform = None;
        self.drag_start_rotation = None;
        self.drag_start_t = None;
        self.drag_start_vec = None;
    }

    /// Direction vector for the active axis, if any.
    pub fn active_axis_dir(&self) -> Option<Vec3> {
        match self.active_axis {
            Some(0) => Some(Vec3::new(1.0, 0.0, 0.0)),
            Some(1) => Some(Vec3::new(0.0, 1.0, 0.0)),
            Some(2) => Some(Vec3::new(0.0, 0.0, 1.0)),
            _ => None,
        }
    }

    /// During a translate drag, compute the new translation.
    ///
    /// Returns `None` if the drag state is incomplete or the mouse ray
    /// is parallel to the axis.
    pub fn update_translate(&self, ray_origin: Vec3, ray_dir: Vec3) -> Option<Vec3> {
        let (start_pos, _) = self.drag_start_transform?;
        let axis = self.active_axis_dir()?;
        let start_t = self.drag_start_t?;
        let t = ray_ray_closest_t(start_pos, axis, ray_origin, ray_dir)?;
        Some(start_pos + axis * (t - start_t))
    }

    /// During a scale drag, compute the new scale vector.
    pub fn update_scale(&self, ray_origin: Vec3, ray_dir: Vec3) -> Option<Vec3> {
        let (start_pos, start_scale) = self.drag_start_transform?;
        let axis = self.active_axis_dir()?;
        let start_t = self.drag_start_t?;
        let t = ray_ray_closest_t(start_pos, axis, ray_origin, ray_dir)?;
        let delta = (t - start_t) * 0.5;
        let mut new_scale = start_scale;
        match self.active_axis {
            Some(0) => new_scale.x = (start_scale.x + delta).max(0.01),
            Some(1) => new_scale.y = (start_scale.y + delta).max(0.01),
            Some(2) => new_scale.z = (start_scale.z + delta).max(0.01),
            _ => return None,
        }
        Some(new_scale)
    }

    /// During a rotate drag, compute the new rotation quaternion.
    ///
    /// If `snap_angle` is `Some(angle)`, the rotation is snapped to the nearest
    /// multiple of `angle` (in radians).
    pub fn update_rotate(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        snap_angle: Option<f32>,
    ) -> Option<Quat> {
        let (start_pos, _) = self.drag_start_transform?;
        let start_rot = self.drag_start_rotation?;
        let axis = self.active_axis_dir()?;
        let start_vec = self.drag_start_vec?;

        // Intersect mouse ray with the rotation plane.
        let to_plane = start_pos - ray_origin;
        let denom = axis.dot(ray_dir);
        if denom.abs() < 1e-6 {
            return Some(start_rot);
        }
        let t = to_plane.dot(axis) / denom;
        if t < 0.0 {
            return Some(start_rot);
        }
        let hit = ray_origin + ray_dir * t;
        let current_vec = hit - start_pos;

        let start_len = start_vec.length();
        let curr_len = current_vec.length();
        if start_len < 1e-6 || curr_len < 1e-6 {
            return Some(start_rot);
        }

        let cos_a = (start_vec.dot(current_vec) / (start_len * curr_len)).clamp(-1.0, 1.0);
        let cross = start_vec.cross(current_vec);
        let sin_a = cross.dot(axis) / (start_len * curr_len);
        let mut angle = sin_a.atan2(cos_a);

        if let Some(snap) = snap_angle {
            if snap > 1e-6 {
                angle = (angle / snap).round() * snap;
            }
        }

        Some(start_rot * Quat::from_axis_angle(axis, angle))
    }

    /// Render the gizmo for the given world position onto an egui painter.
    ///
    /// `world_pos`: the selected object's world-space translation.
    /// `view_proj`: the camera's view-projection matrix.
    /// `screen_size`: (width, height) of the viewport in pixels.
    /// `screen_offset`: (offset_x, offset_y) of the viewport on screen.
    /// `painter`: the egui painter to use.
    /// `gizmo_size`: visual size of the gizmo in screen pixels.
    pub fn render(
        &self,
        world_pos: Vec3,
        view_proj: Mat4,
        screen_size: (f32, f32),
        screen_offset: (f32, f32),
        painter: &egui::Painter,
        gizmo_size: f32,
    ) {
        let (sx, sy) = screen_size;
        let (ox, oy) = screen_offset;

        // Project world position to screen.
        let clip = view_proj * Vec4::new(world_pos.x, world_pos.y, world_pos.z, 1.0);
        if clip.w.abs() < 0.001 {
            return; // behind camera
        }
        let ndc = Vec3::new(clip.x / clip.w, clip.y / clip.w, clip.z / clip.w);
        let center_x = (ndc.x * 0.5 + 0.5) * sx + ox;
        let center_y = ((1.0 - (ndc.y * 0.5 + 0.5)) * sy) + oy;
        let center = egui::pos2(center_x, center_y);

        // Don't draw if center is off-screen.
        if center_x < ox - gizmo_size
            || center_x > ox + sx + gizmo_size
            || center_y < oy - gizmo_size
            || center_y > oy + sy + gizmo_size
        {
            return;
        }

        match self.mode {
            GizmoMode::Translate => {
                self.render_translate_gizmo(center, painter, gizmo_size);
            },
            GizmoMode::Rotate => {
                self.render_rotate_gizmo(center, painter, gizmo_size);
            },
            GizmoMode::Scale => {
                self.render_scale_gizmo(center, painter, gizmo_size);
            },
        }
    }

    /// Render translation arrows (X=red right, Y=green up, Z=blue diagonal).
    fn render_translate_gizmo(&self, center: egui::Pos2, painter: &egui::Painter, size: f32) {
        let arrow_len = size;

        // X axis — red, right
        let x_end = center + egui::vec2(arrow_len, 0.0);
        self.draw_axis_arrow(painter, center, x_end, egui::Color32::RED, "X");

        // Y axis — green, up
        let y_end = center + egui::vec2(0.0, -arrow_len);
        self.draw_axis_arrow(painter, center, y_end, egui::Color32::GREEN, "Y");

        // Z axis — blue, diagonal (down-right)
        let z_end = center + egui::vec2(arrow_len * 0.7, arrow_len * 0.7);
        self.draw_axis_arrow(painter, center, z_end, egui::Color32::BLUE, "Z");

        // Center dot.
        painter.circle_filled(center, 4.0, egui::Color32::WHITE);
    }

    /// Render rotation rings (colored ellipses around center).
    fn render_rotate_gizmo(&self, center: egui::Pos2, painter: &egui::Painter, size: f32) {
        let radius = size;
        let segments = 48;

        // X rotation ring (red) — somewhat flattened vertically
        self.draw_ring(painter, center, radius, radius * 0.4, egui::Color32::RED, segments);

        // Y rotation ring (green) — nearly circular
        self.draw_ring(painter, center, radius, radius * 0.85, egui::Color32::GREEN, segments);

        // Z rotation ring (blue) — flattened diagonally
        self.draw_ring(painter, center, radius * 0.7, radius * 0.7, egui::Color32::BLUE, segments);

        // Center dot.
        painter.circle_filled(center, 4.0, egui::Color32::WHITE);
    }

    /// Render scale handles (lines with boxes at ends).
    fn render_scale_gizmo(&self, center: egui::Pos2, painter: &egui::Painter, size: f32) {
        let len = size;
        let box_r = 5.0;

        // X axis (red, right)
        self.draw_scale_handle(
            painter,
            center,
            center + egui::vec2(len, 0.0),
            egui::Color32::RED,
            box_r,
        );
        // Y axis (green, up)
        self.draw_scale_handle(
            painter,
            center,
            center + egui::vec2(0.0, -len),
            egui::Color32::GREEN,
            box_r,
        );
        // Z axis (blue, diagonal)
        self.draw_scale_handle(
            painter,
            center,
            center + egui::vec2(len * 0.7, len * 0.7),
            egui::Color32::BLUE,
            box_r,
        );

        // Center box.
        let half = 3.0;
        painter.rect_filled(
            egui::Rect::from_center_size(center, egui::vec2(half * 2.0, half * 2.0)),
            egui::CornerRadius::ZERO,
            egui::Color32::WHITE,
        );
    }

    // ---- Helper drawing methods ----

    fn draw_axis_arrow(
        &self,
        painter: &egui::Painter,
        from: egui::Pos2,
        to: egui::Pos2,
        color: egui::Color32,
        _label: &str,
    ) {
        let dir = (to - from).normalized();
        let head_len = 8.0;
        let head_width = 4.0;
        let shaft_end = to - dir * head_len;

        // Shaft line.
        painter.line_segment([from, shaft_end], egui::Stroke::new(2.0, color));

        // Arrowhead triangle.
        let perp = egui::vec2(-dir.y, dir.x);
        let head_points = [to, shaft_end - perp * head_width, shaft_end + perp * head_width];
        let head_color = color;
        // Draw arrowhead as filled triangle via lines.
        painter.line_segment([head_points[0], head_points[1]], egui::Stroke::new(2.0, head_color));
        painter.line_segment([head_points[0], head_points[2]], egui::Stroke::new(2.0, head_color));
        painter.line_segment([head_points[1], head_points[2]], egui::Stroke::new(2.0, head_color));
    }

    fn draw_ring(
        &self,
        painter: &egui::Painter,
        center: egui::Pos2,
        rx: f32,
        ry: f32,
        color: egui::Color32,
        segments: usize,
    ) {
        let mut points = Vec::with_capacity(segments + 1);
        for i in 0..=segments {
            let angle = (i as f32) / (segments as f32) * std::f32::consts::TAU;
            let x = center.x + angle.cos() * rx;
            let y = center.y + angle.sin() * ry;
            points.push(egui::pos2(x, y));
        }
        for w in points.windows(2) {
            painter.line_segment([w[0], w[1]], egui::Stroke::new(1.5, color));
        }
    }

    fn draw_scale_handle(
        &self,
        painter: &egui::Painter,
        from: egui::Pos2,
        to: egui::Pos2,
        color: egui::Color32,
        box_radius: f32,
    ) {
        // Line.
        painter.line_segment([from, to], egui::Stroke::new(2.0, color));
        // Box at end.
        painter.rect_filled(
            egui::Rect::from_center_size(to, egui::vec2(box_radius * 2.0, box_radius * 2.0)),
            egui::CornerRadius::ZERO,
            color,
        );
    }
}

// ------------------------------------------------------------------
// Free functions
// ------------------------------------------------------------------

/// Compute the parameter `t` along an axis line (point `line_point`,
/// direction `line_dir`) that is closest to a mouse ray (point `ray_origin`,
/// direction `ray_dir`).
///
/// Returns `None` when the two rays are parallel.
fn ray_ray_closest_t(
    line_point: Vec3,
    line_dir: Vec3,
    ray_origin: Vec3,
    ray_dir: Vec3,
) -> Option<f32> {
    let w0 = line_point - ray_origin;
    let a = line_dir.dot(line_dir); // = 1 for unit vectors
    let b = line_dir.dot(ray_dir);
    let c = ray_dir.dot(ray_dir); // = 1 for unit vectors
    let d = line_dir.dot(w0);
    let _e = ray_dir.dot(w0);

    let denom = a * c - b * b;
    if denom.abs() < 1e-8 {
        return None; // rays are parallel
    }

    let t_line = (b * _e - c * d) / denom;
    Some(t_line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ray_ray_closest_t_perpendicular() {
        // Axis along X from origin, mouse ray along Z from (0.5, 0, -5).
        let t = ray_ray_closest_t(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 0.0, -5.0),
            Vec3::new(0.0, 0.0, 1.0),
        );
        // Closest point on axis is (0.5, 0, 0), so t = 0.5.
        assert!(t.is_some());
        assert!((t.unwrap() - 0.5).abs() < 0.001, "expected t=0.5, got {}", t.unwrap());
    }

    #[test]
    fn test_ray_ray_closest_t_parallel() {
        let t = ray_ray_closest_t(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
        );
        assert!(t.is_none(), "parallel rays should return None");
    }

    #[test]
    fn test_gizmo_translate_drag() {
        let mut gizmo = Gizmo::new();
        gizmo.mode = GizmoMode::Translate;
        gizmo.begin_drag(0, Vec3::ZERO, Vec3::ONE, Quat::IDENTITY);
        // Start pick at (0.5, 0, -5) looking +Z: closest point on X axis is t=0.5.
        gizmo.capture_drag_start(Vec3::new(0.5, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(gizmo.drag_start_t, Some(0.5));

        // Move mouse to (2.0, 0, -5): closest point on X axis is t=2.0.
        let new_pos = gizmo.update_translate(Vec3::new(2.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0));
        assert!(new_pos.is_some());
        assert!((new_pos.unwrap().x - 1.5).abs() < 0.001, "delta should be 1.5");
    }

    #[test]
    fn test_gizmo_rotate_drag() {
        let mut gizmo = Gizmo::new();
        gizmo.mode = GizmoMode::Rotate;
        gizmo.begin_drag(2, Vec3::ZERO, Vec3::ONE, Quat::IDENTITY); // Z axis
        // Start pick: ray from (1, 0, -5) looking +Z hits plane at (1, 0, 0).
        gizmo.capture_drag_start(Vec3::new(1.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0));
        assert!(gizmo.drag_start_vec.is_some());

        // Rotate 90 degrees: ray from (0, 1, -5) hits plane at (0, 1, 0).
        let new_rot = gizmo.update_rotate(Vec3::new(0.0, 1.0, -5.0), Vec3::new(0.0, 0.0, 1.0), None);
        assert!(new_rot.is_some());
        // Should be a ~90 degree rotation around Z.
        let rot = new_rot.unwrap();
        let angle = 2.0 * rot.w.acos();
        assert!(angle > 1.4 && angle < 1.7, "expected ~pi/2 rotation, got angle={}", angle);
    }

    #[test]
    fn test_gizmo_rotate_snap_to_45_degrees() {
        let mut gizmo = Gizmo::new();
        gizmo.mode = GizmoMode::Rotate;
        gizmo.begin_drag(2, Vec3::ZERO, Vec3::ONE, Quat::IDENTITY); // Z axis
        gizmo.capture_drag_start(Vec3::new(1.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0));

        // Rotate ~50 degrees: ray from (cos50, sin50, -5) direction +Z.
        let rad = 50.0_f32.to_radians();
        let new_rot = gizmo.update_rotate(
            Vec3::new(rad.cos(), rad.sin(), -5.0),
            Vec3::new(0.0, 0.0, 1.0),
            Some(45.0_f32.to_radians()),
        );
        let rot = new_rot.unwrap();
        let angle = 2.0 * rot.w.acos();
        // Should snap to 45 degrees (pi/4 ≈ 0.785), not 50 degrees.
        let target = 45.0_f32.to_radians();
        assert!(
            (angle - target).abs() < 0.01,
            "expected ~45 degrees after snap, got {} rad",
            angle
        );
    }

    #[test]
    fn test_gizmo_end_drag_clears_state() {
        let mut gizmo = Gizmo::new();
        gizmo.begin_drag(0, Vec3::ZERO, Vec3::ONE, Quat::IDENTITY);
        assert!(gizmo.dragging);
        gizmo.end_drag();
        assert!(!gizmo.dragging);
        assert!(gizmo.active_axis.is_none());
        assert!(gizmo.drag_start_transform.is_none());
    }
}
