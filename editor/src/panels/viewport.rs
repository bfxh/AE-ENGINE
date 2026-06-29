//! Viewport panel: the central 3D rendering area.
//!
//! Renders the scene using wgpu and displays the result as an egui image.
//! Handles mouse input for camera orbit/pan/zoom and object picking.

use crate::app::{EditorAction, EditorApp};
use crate::gizmo::GizmoMode;
use crate::panels::EditorPanel;
use crate::render::picking::{PickAxis, RayPicker};

/// Viewport projection mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewMode {
    Perspective,
    Top,
    Front,
    Side,
}

impl ViewMode {
    fn label(&self) -> &'static str {
        match self {
            ViewMode::Perspective => "Persp",
            ViewMode::Top => "Top",
            ViewMode::Front => "Front",
            ViewMode::Side => "Side",
        }
    }
}

pub struct ViewportPanel {
    /// Whether to show the ground grid overlay.
    pub show_grid: bool,
    /// Whether to show the stats overlay (FPS, coords).
    pub show_stats_overlay: bool,
    /// Current view projection mode.
    pub view_mode: ViewMode,
    /// Whether to show node name labels.
    pub show_labels: bool,
}

impl Default for ViewportPanel {
    fn default() -> Self {
        Self {
            show_grid: true,
            show_stats_overlay: true,
            view_mode: ViewMode::Perspective,
            show_labels: true,
        }
    }
}

impl EditorPanel for ViewportPanel {
    fn name(&self) -> &str {
        "Viewport"
    }

    fn render(&mut self, ctx: &egui::Context, app: &mut EditorApp) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Get the available rectangle for the viewport.
            let rect = ui.available_rect_before_wrap();

            // Update viewport rect in app state.
            app.viewport_rect = Some((rect.min.x, rect.min.y, rect.width(), rect.height()));

            // Track whether the mouse is over the viewport.
            let pointer_pos = ui.input(|i| i.pointer.hover_pos());
            app.viewport_hovered = pointer_pos.is_some_and(|p| rect.contains(p));

            // Allocate the full area for interaction.
            let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

            // Handle gizmo mode switching (W/E/R) — only when viewport is hovered
            // and no modifier keys are pressed (to avoid conflicting with text input).
            if app.viewport_hovered {
                let mods = ui.input(|i| i.modifiers);
                if !mods.ctrl && !mods.alt && !mods.shift {
                    ui.input(|i| {
                        for event in &i.events {
                            if let egui::Event::Key { key: egui::Key::W, pressed: true, .. } = event
                            {
                                app.pending_action = Some(EditorAction::GizmoTranslate);
                            }
                            if let egui::Event::Key { key: egui::Key::E, pressed: true, .. } = event
                            {
                                app.pending_action = Some(EditorAction::GizmoRotate);
                            }
                            if let egui::Event::Key { key: egui::Key::R, pressed: true, .. } = event
                            {
                                app.pending_action = Some(EditorAction::GizmoScale);
                            }
                        }
                    });
                }
            }

            // Handle gizmo drag interaction, camera input, and picking.
            if app.gizmo.dragging {
                // Drag in progress: update the selected node's transform.
                self.handle_gizmo_drag(ui, app, &rect);
                // End drag when the primary button is released.
                if ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) {
                    app.gizmo.end_drag();
                    app.commit_gizmo_drag();
                }
            } else {
                // Try to grab a gizmo axis on primary press.
                let gizmo_grabbed =
                    if ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary))
                        && app.viewport_hovered
                    {
                        self.try_start_gizmo_drag(ui, app, &rect)
                    } else {
                        false
                    };

                if !gizmo_grabbed {
                    // Normal camera orbit/pan/zoom (middle/right mouse).
                    self.handle_camera_input(ui, app, &response);
                    // Click-to-select (left click without dragging).
                    if response.clicked() && app.viewport_hovered {
                        self.handle_picking(ui, app, &rect);
                    }
                }
            }

            // Render the 3D scene into a texture and display it.
            self.render_viewport(ui, app, &rect);

            // Floating view mode selector + display toggles.
            let ctx2 = ui.ctx().clone();
            self.draw_viewport_toolbar(&ctx2, app, &rect);
        });
    }
}

impl ViewportPanel {
    /// Handle camera orbit/pan/zoom from mouse input.
    fn handle_camera_input(&self, ui: &egui::Ui, app: &mut EditorApp, response: &egui::Response) {
        if !app.viewport_hovered {
            return;
        }

        // Scroll for zoom.
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.01 {
            app.camera.zoom(scroll * 0.5);
        }

        // Middle mouse button or Alt+Left for orbit.
        let middle_down = ui.input(|i| i.pointer.button_down(egui::PointerButton::Middle));
        let alt_left = {
            let mods = ui.input(|i| i.modifiers);
            mods.alt && ui.input(|i| i.pointer.button_down(egui::PointerButton::Primary))
        };

        if (middle_down || alt_left) && response.dragged() {
            let delta = response.drag_delta();
            app.camera.orbit(delta.x, delta.y);
        }

        // Shift+Middle or middle for pan.
        let shift_middle = {
            let mods = ui.input(|i| i.modifiers);
            mods.shift && ui.input(|i| i.pointer.button_down(egui::PointerButton::Middle))
        };

        if shift_middle && response.dragged() {
            let delta = response.drag_delta();
            app.camera.pan(-delta.x, delta.y);
        }

        // Right mouse button for fly (orbit).
        let right_down = ui.input(|i| i.pointer.button_down(egui::PointerButton::Secondary));
        if right_down && response.dragged() {
            let delta = response.drag_delta();
            app.camera.orbit(delta.x, delta.y);
        }

        // WASD-equivalent fly using arrow keys + Space/Shift.
        // (W/E/R are reserved for gizmo mode switching.)
        let (dt, fwd, back, left, right_fly, up_fly, down_fly) = ui.input(|i| {
            (
                i.stable_dt,
                i.key_down(egui::Key::ArrowUp),
                i.key_down(egui::Key::ArrowDown),
                i.key_down(egui::Key::ArrowLeft),
                i.key_down(egui::Key::ArrowRight),
                i.key_down(egui::Key::Space),
                i.modifiers.shift,
            )
        });
        if dt > 0.0 && (fwd || back || left || right_fly || up_fly || down_fly) {
            let f = if fwd { 1.0 } else { 0.0 } - if back { 1.0 } else { 0.0 };
            let r = if right_fly { 1.0 } else { 0.0 } - if left { 1.0 } else { 0.0 };
            let u = if up_fly { 1.0 } else { 0.0 } - if down_fly { 1.0 } else { 0.0 };
            app.camera.fly(f, r, u, dt);
        }
    }

    /// Handle left-click picking in the viewport.
    ///
    /// Uses BVH ray_query against per-node AABBs (unit cube transformed by model matrix).
    /// Replaces the previous O(n) ray-sphere scan for API consistency with the frustum
    /// culling path in `SceneRenderer::render_scene`. Returns the closest AABB hit.
    fn handle_picking(&self, ui: &egui::Ui, app: &mut EditorApp, rect: &egui::Rect) {
        let pointer_pos = match ui.input(|i| i.pointer.hover_pos()) {
            Some(p) => p,
            None => return,
        };

        let w = rect.width();
        let h = rect.height();
        if w < 1.0 || h < 1.0 {
            return;
        }

        // Convert to normalized device coordinates (-1 to 1).
        let ndc_x = ((pointer_pos.x - rect.min.x) / w) * 2.0 - 1.0;
        let ndc_y = -(((pointer_pos.y - rect.min.y) / h) * 2.0 - 1.0);
        let aspect = w / h;

        let (origin, direction) = app.camera.screen_to_ray(ndc_x, ndc_y, aspect);

        // BVH ray query: closest AABB hit wins. Hits are sorted by t ascending.
        let bvh = crate::render::scene_renderer::build_scene_bvh(&app.scene);
        let hits = bvh.ray_query(origin, direction, f32::MAX);

        if let Some((id_u32, _t)) = hits.first() {
            // BVH stores u32 ids; scene nodes use u64. Round-trip via as u64.
            app.selection.select(*id_u32 as u64);
        } else {
            app.selection.clear();
        }
    }

    /// Try to start a gizmo drag by hit-testing the gizmo axes.
    ///
    /// Returns `true` if a gizmo axis was grabbed and drag started.
    fn try_start_gizmo_drag(
        &self,
        ui: &egui::Ui,
        app: &mut EditorApp,
        rect: &egui::Rect,
    ) -> bool {
        let sel_id = match app.selection.selected_id {
            Some(id) => id,
            None => return false,
        };
        // Clone the node so we don't hold an immutable borrow of app.scene
        // when we later mutably borrow app.gizmo.
        let node = match app.scene.find_node(sel_id) {
            Some(n) => n.clone(),
            None => return false,
        };

        let pointer = match ui.input(|i| i.pointer.hover_pos()) {
            Some(p) => p,
            None => return false,
        };

        let w = rect.width();
        let h = rect.height();
        if w < 1.0 || h < 1.0 {
            return false;
        }

        let aspect = w / h;
        let view_proj = app.camera.view_projection_matrix(aspect);
        let cam_pos = app.camera.position;
        let gizmo_origin = node.transform.translation;

        // Screen coordinates relative to the viewport rect.
        let sx = pointer.x - rect.min.x;
        let sy = pointer.y - rect.min.y;

        let picker = RayPicker::from_screen(sx, sy, w, h, view_proj);
        let pick_axis = picker.pick_axis_gizmo(gizmo_origin, 1.5, cam_pos);

        let axis = match pick_axis {
            PickAxis::X => 0,
            PickAxis::Y => 1,
            PickAxis::Z => 2,
            PickAxis::None => return false,
        };

        // Compute the mouse ray for the drag-start capture.
        let ndc_x = 2.0 * sx / w - 1.0;
        let ndc_y = 1.0 - 2.0 * sy / h;
        let (ray_origin, ray_dir) = app.camera.screen_to_ray(ndc_x, ndc_y, aspect);

        app.gizmo.begin_drag(
            axis,
            node.transform.translation,
            node.transform.scale,
            node.transform.rotation,
        );
        app.gizmo.capture_drag_start(ray_origin, ray_dir);
        app.gizmo_drag_start = Some((sel_id, node.transform.clone()));
        log::debug!("Gizmo drag started: axis={}, mode={:?}", axis, app.gizmo.mode);
        true
    }

    /// Update the selected node's transform during a gizmo drag.
    fn handle_gizmo_drag(&self, ui: &egui::Ui, app: &mut EditorApp, rect: &egui::Rect) {
        let sel_id = match app.selection.selected_id {
            Some(id) => id,
            None => {
                app.gizmo.end_drag();
                app.commit_gizmo_drag();
                return;
            }
        };

        let pointer = match ui.input(|i| i.pointer.hover_pos()) {
            Some(p) => p,
            None => return,
        };

        let w = rect.width();
        let h = rect.height();
        if w < 1.0 || h < 1.0 {
            return;
        }

        let aspect = w / h;
        let sx = pointer.x - rect.min.x;
        let sy = pointer.y - rect.min.y;
        let ndc_x = 2.0 * sx / w - 1.0;
        let ndc_y = 1.0 - 2.0 * sy / h;
        let (ray_origin, ray_dir) = app.camera.screen_to_ray(ndc_x, ndc_y, aspect);

        let (grid_snap, snap_dist, rot_snap, snap_deg) = app
            .settings_panel
            .as_ref()
            .map(|s| (s.grid_snapping, s.snap_distance, s.rotation_snapping, s.snap_angle_deg))
            .unwrap_or((false, 0.25, false, 15.0));

        let mode = app.gizmo.mode;
        match mode {
            GizmoMode::Translate => {
                if let Some(mut new_pos) = app.gizmo.update_translate(ray_origin, ray_dir) {
                    if grid_snap && snap_dist > 0.0 {
                        new_pos.x = (new_pos.x / snap_dist).round() * snap_dist;
                        new_pos.y = (new_pos.y / snap_dist).round() * snap_dist;
                        new_pos.z = (new_pos.z / snap_dist).round() * snap_dist;
                    }
                    if let Some(node) = app.scene.find_node_mut(sel_id) {
                        node.transform.translation = new_pos;
                        app.dirty = true;
                    }
                }
            },
            GizmoMode::Scale => {
                if let Some(new_scale) = app.gizmo.update_scale(ray_origin, ray_dir) {
                    if let Some(node) = app.scene.find_node_mut(sel_id) {
                        node.transform.scale = new_scale;
                        app.dirty = true;
                    }
                }
            },
            GizmoMode::Rotate => {
                let snap_rad = if rot_snap {
                    Some(snap_deg.to_radians())
                } else {
                    None
                };
                if let Some(new_rot) = app.gizmo.update_rotate(ray_origin, ray_dir, snap_rad) {
                    if let Some(node) = app.scene.find_node_mut(sel_id) {
                        node.transform.rotation = new_rot;
                        app.dirty = true;
                    }
                }
            },
        }
    }

    /// Render the 3D viewport content.
    fn render_viewport(&self, ui: &mut egui::Ui, app: &mut EditorApp, rect: &egui::Rect) {
        let size = rect.size();

        // If a real wgpu-rendered 3D texture is available, display it.
        if let Some(tex_id) = app.viewport_texture_id {
            let (tw, th) = app.viewport_texture_size;
            if tw > 0 && th > 0 {
                ui.painter().rect_filled(
                    *rect,
                    egui::CornerRadius::ZERO,
                    egui::Color32::from_rgb(20, 22, 28),
                );
                let tex = egui::load::SizedTexture::new(tex_id, egui::Vec2::new(tw as f32, th as f32));
                ui.put(*rect, egui::Image::from_texture(tex).fit_to_exact_size(size));
                if self.show_stats_overlay {
                    self.draw_viewport_overlay(ui, app, rect);
                }
                return;
            }
        }

        // Fallback: 2D simulated viewport (used until wgpu 3D renderer is wired).
        let bg_color = egui::Color32::from_rgb(40, 42, 48);
        let grid_color = egui::Color32::from_rgb(60, 62, 68);

        // Draw background rect to fill area.
        ui.painter().rect_filled(
            *rect,
            egui::CornerRadius::ZERO,
            bg_color,
        );

        // Draw a simple grid overlay to represent the ground plane.
        let grid_rect = *rect;
        let cell_size = 48.0;

        // Draw grid lines.
        let mut x = grid_rect.min.x;
        while x < grid_rect.max.x {
            ui.painter().line_segment(
                [egui::pos2(x, grid_rect.min.y), egui::pos2(x, grid_rect.min.y + size.y)],
                egui::Stroke::new(0.5, grid_color),
            );
            x += cell_size;
        }

        let mut y = grid_rect.min.y;
        while y < grid_rect.max.y {
            ui.painter().line_segment(
                [egui::pos2(grid_rect.min.x, y), egui::pos2(grid_rect.min.x + size.x, y)],
                egui::Stroke::new(0.5, grid_color),
            );
            y += cell_size;
        }

        // ---- Animated Elements ----
        let t = app.frame_counter as f32 * 0.02; // Animation time

        // Draw animated rotating ring around origin.
        let origin = egui::pos2(grid_rect.min.x + size.x / 2.0, grid_rect.min.y + size.y / 2.0);
        let ring_radius = 60.0;
        let num_dots = 8;
        for i in 0..num_dots {
            let angle = t + (i as f32) * std::f32::consts::TAU / num_dots as f32;
            let dot_x = origin.x + angle.cos() * ring_radius;
            let dot_y = origin.y + angle.sin() * ring_radius * 0.6; // squash to simulate perspective
            let alpha = ((angle.sin() * 0.5 + 0.5) * 200.0) as u8 + 55;
            ui.painter().circle_filled(
                egui::pos2(dot_x, dot_y),
                3.0,
                egui::Color32::from_rgba_premultiplied(0, 200, 255, alpha),
            );
        }

        // Draw a small axis indicator in the corner.
        let axis_origin = egui::pos2(grid_rect.min.x + 48.0, grid_rect.min.y + size.y - 48.0);
        let axis_len = 30.0;
        let x_color = egui::Color32::RED;
        let y_color = egui::Color32::GREEN;
        let z_color = egui::Color32::BLUE;

        ui.painter().arrow(axis_origin, egui::vec2(axis_len, 0.0), egui::Stroke::new(2.0, x_color));
        ui.painter().arrow(
            axis_origin,
            egui::vec2(0.0, -axis_len),
            egui::Stroke::new(2.0, y_color),
        );
        // Z axis drawn diagonally.
        let z_end = axis_origin + egui::vec2(-axis_len * 0.7, axis_len * 0.7);
        ui.painter().arrow(axis_origin, z_end - axis_origin, egui::Stroke::new(2.0, z_color));

        // Selection highlight: project selected node to screen and draw a ring.
        if let Some(id) = app.selection.selected_id {
            if let Some(node) = app.scene.find_node(id) {
                let pos = node.transform.translation;
                let view_proj = app.camera.view_projection_matrix(size.x / size.y);
                let clip = view_proj * glam::Vec4::new(pos.x, pos.y, pos.z, 1.0);
                if clip.w.abs() > 0.001 {
                    let ndc_x = clip.x / clip.w;
                    let ndc_y = clip.y / clip.w;
                    let ndc_z = clip.z / clip.w;
                    if ndc_z < 1.0 {
                        let sx = (ndc_x * 0.5 + 0.5) * size.x + rect.min.x;
                        let sy = ((1.0 - (ndc_y * 0.5 + 0.5)) * size.y) + rect.min.y;
                        ui.painter().circle_stroke(
                            egui::pos2(sx, sy),
                            10.0,
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 200, 0)),
                        );
                    }
                }
            }
        }

        // Draw scene node markers.
        for node in &app.scene.nodes {
            if node.id == 0 {
                continue; // skip root
            }
            let pos = node.transform.translation;

            // Project world position to screen using camera.
            let view_proj = app.camera.view_projection_matrix(size.x / size.y);
            let world_pos = glam::Vec4::new(pos.x, pos.y, pos.z, 1.0);
            let clip_pos = view_proj * world_pos;

            if clip_pos.w.abs() < 0.001 {
                continue; // behind camera or at infinity
            }

            let ndc = glam::Vec2::new(clip_pos.x / clip_pos.w, clip_pos.y / clip_pos.w);

            // Convert NDC to screen coordinates.
            let screen_x = (ndc.x * 0.5 + 0.5) * size.x + grid_rect.min.x;
            let screen_y = ((1.0 - (ndc.y * 0.5 + 0.5)) * size.y) + grid_rect.min.y;

            if screen_x < grid_rect.min.x
                || screen_x > grid_rect.max.x
                || screen_y < grid_rect.min.y
                || screen_y > grid_rect.max.y
            {
                continue; // off-screen
            }

            let marker_pos = egui::pos2(screen_x, screen_y);

            // Draw selection highlight.
            if app.selection.is_selected(node.id) {
                // Animated selection ring.
                let pulse = (t * 3.0).sin() * 0.3 + 0.7; // 0.4..1.0
                let outer_r = 12.0 * pulse;
                let inner_r = 8.0;
                ui.painter().circle_filled(
                    marker_pos,
                    outer_r,
                    egui::Color32::from_rgba_premultiplied(255, 200, 50, (pulse * 128.0) as u8),
                );
                ui.painter().circle(
                    marker_pos,
                    inner_r,
                    egui::Color32::from_rgb(255, 200, 50),
                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                );
            } else {
                let node_color = match &node.node_type {
                    crate::scene::NodeType::Empty => egui::Color32::from_rgb(150, 150, 150),
                    crate::scene::NodeType::Mesh { .. } => egui::Color32::from_rgb(100, 180, 255),
                    crate::scene::NodeType::Light { .. } => {
                        // Light nodes gently pulse.
                        let glow = (t * 2.0 + node.id as f32).sin() * 0.3 + 0.7;
                        let r = (255.0 * glow) as u8;
                        let g = (255.0 * glow) as u8;
                        let b = (100.0 * glow) as u8;
                        egui::Color32::from_rgb(r, g, b)
                    },
                    crate::scene::NodeType::Camera { .. } => egui::Color32::from_rgb(100, 255, 100),
                };
                ui.painter().circle(marker_pos, 5.0, node_color, egui::Stroke::NONE);
            }

            // Draw name label (conditional on show_labels).
            if self.show_labels {
                let label_pos = marker_pos + egui::vec2(8.0, -8.0);
                ui.painter().text(
                    label_pos,
                    egui::Align2::LEFT_TOP,
                    &node.name,
                    egui::FontId::proportional(10.0),
                    egui::Color32::WHITE,
                );
            }
        }

        // ---- Render Gizmo ----
        if let Some(selected_id) = app.selection.selected_id {
            if let Some(node) = app.scene.find_node(selected_id) {
                let world_pos = node.transform.translation;
                let view_proj = app.camera.view_projection_matrix(size.x / size.y);
                let gizmo_sz = app
                    .settings_panel
                    .as_ref()
                    .map(|s| s.gizmo_size)
                    .unwrap_or(60.0);
                app.gizmo.render(
                    world_pos,
                    view_proj,
                    (size.x, size.y),
                    (grid_rect.min.x, grid_rect.min.y),
                    ui.painter(),
                    gizmo_sz,
                );
            }
        }

        // Gizmo mode indicator.
        let mode_label = app.gizmo.mode.name();
        let indicator_pos = egui::pos2(grid_rect.min.x + 12.0, grid_rect.min.y + 12.0);
        ui.painter().text(
            indicator_pos,
            egui::Align2::LEFT_TOP,
            format!("Gizmo: {}", mode_label),
            egui::FontId::proportional(11.0),
            egui::Color32::from_rgba_premultiplied(200, 200, 200, 180),
        );

        // Display help text if the scene is nearly empty.
        if app.scene.nodes.len() <= 1 {
            let help_alpha = if t.sin() > 0.0 { 160 } else { 128 };
            ui.painter().text(
                grid_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Right-click to orbit | Scroll to zoom | Middle-click to pan\nUse File > New to create a scene or File > Open to load one",
                egui::FontId::proportional(14.0),
                egui::Color32::from_rgba_premultiplied(180, 180, 180, help_alpha),
            );
        }
    }

    /// Apply the current view_mode to reposition the camera.
    fn apply_view_mode(&self, app: &mut EditorApp) {
        let (yaw, pitch) = match self.view_mode {
            ViewMode::Perspective => (-std::f32::consts::FRAC_PI_4, -std::f32::consts::FRAC_PI_4),
            ViewMode::Top => (0.0, -(std::f32::consts::FRAC_PI_2 - 0.01)),
            ViewMode::Front => (std::f32::consts::FRAC_PI_2, 0.0),
            ViewMode::Side => (0.0, 0.0),
        };
        app.camera.set_orbit_angles(yaw, pitch);
    }

    /// Draw the floating view mode selector and display toggle toolbar.
    fn draw_viewport_toolbar(&mut self, ctx: &egui::Context, app: &mut EditorApp, rect: &egui::Rect) {
        egui::Area::new(egui::Id::new("viewport_toolbar"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(rect.min.x + 8.0, rect.min.y + 8.0))
            .show(ctx, |ui| {
                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgba_premultiplied(30, 30, 35, 200))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(80, 80, 90, 180)))
                    .inner_margin(egui::Margin::symmetric(6, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("View:").small().color(egui::Color32::from_rgb(200, 200, 200)));
                            for mode in [ViewMode::Perspective, ViewMode::Top, ViewMode::Front, ViewMode::Side] {
                                let active = self.view_mode == mode;
                                let text = egui::RichText::new(mode.label()).small();
                                let text = if active {
                                    text.color(egui::Color32::from_rgb(100, 180, 255)).strong()
                                } else {
                                    text.color(egui::Color32::from_rgb(180, 180, 180))
                                };
                                if ui.selectable_label(active, text).clicked() {
                                    self.view_mode = mode;
                                    self.apply_view_mode(app);
                                }
                            }
                            ui.separator();
                            ui.checkbox(&mut self.show_grid, "Grid");
                            ui.checkbox(&mut self.show_stats_overlay, "Stats");
                            ui.checkbox(&mut self.show_labels, "Labels");
                        });
                    });
            });
    }

    /// Draw overlay on top of the 3D viewport (axis gizmo, selection outline).
    fn draw_viewport_overlay(&self, ui: &mut egui::Ui, _app: &mut EditorApp, rect: &egui::Rect) {
        let size = rect.size();
        if size.x < 1.0 || size.y < 1.0 {
            return;
        }

        // Draw a small axis gizmo in the bottom-left corner.
        let axis_origin = egui::pos2(rect.min.x + 48.0, rect.max.y - 48.0);
        let axis_len = 30.0;
        ui.painter().arrow(axis_origin, egui::vec2(axis_len, 0.0), egui::Stroke::new(2.0, egui::Color32::RED));
        ui.painter().arrow(axis_origin, egui::vec2(0.0, -axis_len), egui::Stroke::new(2.0, egui::Color32::GREEN));
        let z_end = axis_origin + egui::vec2(-axis_len * 0.7, axis_len * 0.7);
        ui.painter().arrow(axis_origin, z_end - axis_origin, egui::Stroke::new(2.0, egui::Color32::BLUE));

        // View mode label in top-right.
        let label = format!("{} [{}x{}]", self.view_mode.label(), size.x as u32, size.y as u32);
        ui.painter().text(
            egui::pos2(rect.max.x - 8.0, rect.min.y + 8.0),
            egui::Align2::RIGHT_TOP,
            label,
            egui::FontId::proportional(11.0),
            egui::Color32::from_rgba_premultiplied(200, 200, 200, 180),
        );
    }
}
