//! Status bar panel: displays editor state information at the bottom.
//!
//! Shows scene path, selection info, dirty flag, camera position, FPS, and gizmo mode.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct StatusBarPanel;

impl EditorPanel for StatusBarPanel {
    fn name(&self) -> &str { "Status Bar" }

    fn render(&mut self, ctx: &egui::Context, app: &mut EditorApp) {
        egui::TopBottomPanel::bottom("status_bar").exact_height(22.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.set_height(20.0);

                // Scene name and path.
                let scene_label = if let Some(ref path) = app.scene_path {
                    let name = std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path.clone());
                    format!(" {}", name)
                } else {
                    format!(" {} (unsaved)", app.scene.name)
                };
                ui.label(egui::RichText::new(&scene_label).small().color(egui::Color32::from_rgb(200, 220, 255)));

                // Dirty indicator.
                if app.dirty {
                    ui.label(egui::RichText::new("*").small().color(egui::Color32::from_rgb(255, 200, 80)).strong());
                }

                ui.separator();

                // Selection info.
                if let Some(id) = app.selection.selected_id {
                    if let Some(node) = app.scene.find_node(id) {
                        let type_icon = match &node.node_type {
                            crate::scene::NodeType::Empty => "[E]",
                            crate::scene::NodeType::Mesh { .. } => "[M]",
                            crate::scene::NodeType::Light { .. } => "[L]",
                            crate::scene::NodeType::Camera { .. } => "[C]",
                        };
                        ui.label(egui::RichText::new(format!("{} {} #{}", type_icon, node.name, id)).small().color(egui::Color32::from_rgb(255, 220, 150)));

                        // Selected node position.
                        let p = node.transform.translation;
                        ui.label(egui::RichText::new(format!("({:.1},{:.1},{:.1})", p.x, p.y, p.z)).small().color(egui::Color32::from_rgb(140, 140, 150)).family(egui::FontFamily::Monospace));
                    }
                } else {
                    ui.label(egui::RichText::new("No selection").small().color(egui::Color32::from_rgb(100, 100, 100)));
                }

                ui.separator();

                // Node count.
                ui.label(egui::RichText::new(format!("Nodes: {}", app.scene.nodes.len())).small().color(egui::Color32::from_rgb(150, 180, 150)));

                ui.separator();

                // Gizmo mode.
                let gizmo_label = match app.gizmo.mode {
                    crate::gizmo::GizmoMode::Translate => "Translate",
                    crate::gizmo::GizmoMode::Rotate => "Rotate",
                    crate::gizmo::GizmoMode::Scale => "Scale",
                };
                let gizmo_color = match app.gizmo.mode {
                    crate::gizmo::GizmoMode::Translate => egui::Color32::from_rgb(255, 100, 100),
                    crate::gizmo::GizmoMode::Rotate => egui::Color32::from_rgb(100, 255, 100),
                    crate::gizmo::GizmoMode::Scale => egui::Color32::from_rgb(100, 150, 255),
                };
                ui.label(egui::RichText::new(format!("Gizmo: {}", gizmo_label)).small().color(gizmo_color));

                ui.separator();

                // Camera position.
                let cam = &app.camera;
                ui.label(egui::RichText::new(format!("Cam: ({:.1},{:.1},{:.1})", cam.position.x, cam.position.y, cam.position.z)).small().color(egui::Color32::from_rgb(140, 140, 150)).family(egui::FontFamily::Monospace));

                // Right side: FPS and version.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Version.
                    ui.label(egui::RichText::new("v0.1").small().color(egui::Color32::from_rgb(100, 100, 110)));

                    ui.separator();

                    // Frame counter.
                    ui.label(egui::RichText::new(format!("Frame: {}", app.frame_counter)).small().color(egui::Color32::from_rgb(100, 100, 110)).family(egui::FontFamily::Monospace));

                    ui.separator();

                    // FPS (from stats panel if available).
                    if let Some(ref stats) = app.stats_panel {
                        let fps = stats.current_fps;
                        let fps_color = if fps >= 55.0 {
                            egui::Color32::from_rgb(100, 255, 100)
                        } else if fps >= 30.0 {
                            egui::Color32::from_rgb(255, 255, 100)
                        } else {
                            egui::Color32::from_rgb(255, 100, 100)
                        };
                        ui.label(egui::RichText::new(format!("{:.0} FPS", fps)).small().color(fps_color).strong().family(egui::FontFamily::Monospace));
                    } else {
                        ui.label(egui::RichText::new("-- FPS").small().color(egui::Color32::from_rgb(100, 100, 100)));
                    }
                });
            });
        });
    }
}
