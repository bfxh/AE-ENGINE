//! Statistics panel: displays real-time performance and scene statistics.
//!
//! Shows FPS, frame time, memory usage, draw calls, and scene node counts.

use crate::app::EditorApp;
use crate::panels::EditorPanel;
use std::collections::VecDeque;

/// Statistics panel state.
pub struct StatsPanel {
    pub visible: bool,
    pub fps_history: VecDeque<f32>,
    pub frame_time_history: VecDeque<f32>,
    pub last_frame_time: f32,
    pub fps_accumulator: f32,
    pub fps_frame_count: u32,
    pub current_fps: f32,
    pub max_history: usize,
    pub detailed: bool,
    pub show_fps_graph: bool,
    pub show_frame_time_graph: bool,
}

impl Default for StatsPanel {
    fn default() -> Self {
        Self {
            visible: false,
            fps_history: VecDeque::new(),
            frame_time_history: VecDeque::new(),
            last_frame_time: 16.67,
            fps_accumulator: 0.0,
            fps_frame_count: 0,
            current_fps: 60.0,
            max_history: 120,
            detailed: false,
            show_fps_graph: true,
            show_frame_time_graph: true,
        }
    }
}

impl StatsPanel {
    pub fn update(&mut self, dt: f32, frame: u64) {
        self.last_frame_time = dt * 1000.0;
        self.fps_accumulator += dt;
        self.fps_frame_count += 1;

        if self.fps_accumulator >= 0.25 {
            self.current_fps = self.fps_frame_count as f32 / self.fps_accumulator;
            self.fps_accumulator = 0.0;
            self.fps_frame_count = 0;

            self.fps_history.push_back(self.current_fps);
            if self.fps_history.len() > self.max_history {
                self.fps_history.pop_front();
            }

            self.frame_time_history.push_back(self.last_frame_time);
            if self.frame_time_history.len() > self.max_history {
                self.frame_time_history.pop_front();
            }
        }
        let _ = frame;
    }

    fn draw_graph(&self, ui: &mut egui::Ui, data: &VecDeque<f32>, color: egui::Color32, label: &str, unit: &str, target: f32, warn: f32) {
        let size = egui::vec2(ui.available_width(), 70.0);
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());

        // Background.
        ui.painter().rect_filled(rect, 3, egui::Color32::from_rgb(25, 27, 32));
        ui.painter().rect_stroke(rect, 3, egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 70)), egui::StrokeKind::Middle);

        if data.len() < 2 {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Collecting data...",
                egui::FontId::proportional(10.0),
                egui::Color32::from_rgb(100, 100, 100),
            );
            return;
        }

        let max_val = data.iter().cloned().fold(0.0f32, f32::max).max(1.0);
        let _min_val = 0.0f32;
        let range = max_val.max(1.0);

        // Draw target line (green).
        if target > 0.0 && target <= max_val {
            let target_y = rect.max.y - (target / range) * rect.height();
            ui.painter().line_segment(
                [egui::pos2(rect.min.x, target_y), egui::pos2(rect.max.x, target_y)],
                egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(100, 255, 100, 80)),
            );
        }

        // Draw warning line (red).
        if warn > 0.0 && warn <= max_val {
            let warn_y = rect.max.y - (warn / range) * rect.height();
            ui.painter().line_segment(
                [egui::pos2(rect.min.x, warn_y), egui::pos2(rect.max.x, warn_y)],
                egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(255, 100, 100, 80)),
            );
        }

        // Draw filled area under curve.
        let points: Vec<egui::Pos2> = data
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let x = rect.min.x + (i as f32 / (data.len() - 1) as f32) * rect.width();
                let y = rect.max.y - (*v / range) * rect.height();
                egui::pos2(x, y)
            })
            .collect();

        if points.len() >= 2 {
            // Fill area.
            let mut fill_points = points.clone();
            fill_points.insert(0, egui::pos2(points[0].x, rect.max.y));
            fill_points.push(egui::pos2(points[points.len() - 1].x, rect.max.y));
            let fill_color = egui::Color32::from_rgba_premultiplied(
                color.r(),
                color.g(),
                color.b(),
                40,
            );
            ui.painter().add(egui::Shape::convex_polygon(
                fill_points,
                fill_color,
                egui::Stroke::NONE,
            ));

            // Draw line.
            for i in 0..points.len() - 1 {
                ui.painter().line_segment([points[i], points[i + 1]], egui::Stroke::new(1.5, color));
            }
        }

        // Label.
        let last = *data.back().unwrap();
        let label_color = if last < warn && warn > 0.0 {
            egui::Color32::from_rgb(255, 150, 150)
        } else if last >= target && target > 0.0 {
            egui::Color32::from_rgb(150, 255, 150)
        } else {
            egui::Color32::WHITE
        };
        ui.painter().text(
            rect.min + egui::vec2(4.0, 2.0),
            egui::Align2::LEFT_TOP,
            format!("{}: {:.1}{}", label, last, unit),
            egui::FontId::proportional(10.0),
            label_color,
        );

        // Min/Max labels.
        let avg: f32 = data.iter().sum::<f32>() / data.len() as f32;
        ui.painter().text(
            rect.max - egui::vec2(4.0, 2.0),
            egui::Align2::RIGHT_TOP,
            format!("avg: {:.1}{}  max: {:.1}{}", avg, unit, max_val, unit),
            egui::FontId::proportional(9.0),
            egui::Color32::from_rgb(140, 140, 150),
        );
    }
}

impl EditorPanel for StatsPanel {
    fn name(&self) -> &str { "Statistics" }

    fn render(&mut self, ctx: &egui::Context, app: &mut EditorApp) {
        if !self.visible { return; }

        egui::Window::new("Statistics")
            .default_width(320.0).resizable(true)
            .show(ctx, |ui| {
                // Performance overview.
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Performance").strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.checkbox(&mut self.detailed, "Detailed");
                    });
                });
                ui.separator();

                // FPS display with color coding.
                ui.horizontal(|ui| {
                    ui.label("FPS:");
                    let fps_color = if self.current_fps >= 55.0 {
                        egui::Color32::from_rgb(100, 255, 100)
                    } else if self.current_fps >= 30.0 {
                        egui::Color32::from_rgb(255, 255, 100)
                    } else {
                        egui::Color32::from_rgb(255, 100, 100)
                    };
                    ui.label(egui::RichText::new(format!("{:.1}", self.current_fps)).color(fps_color).strong().size(16.0));

                    let fps_status = if self.current_fps >= 55.0 { "Excellent" }
                        else if self.current_fps >= 30.0 { "Good" }
                        else if self.current_fps >= 20.0 { "Fair" }
                        else { "Poor" };
                    ui.label(egui::RichText::new(fps_status).small().color(fps_color));
                });

                // Frame time.
                ui.horizontal(|ui| {
                    ui.label("Frame Time:");
                    ui.label(egui::RichText::new(format!("{:.2} ms", self.last_frame_time)).family(egui::FontFamily::Monospace));
                    let budget_pct = (self.last_frame_time / 16.67) * 100.0;
                    ui.label(egui::RichText::new(format!("({:.0}% of 60fps budget)", budget_pct)).small().color(egui::Color32::from_rgb(120, 120, 120)));
                });

                ui.horizontal(|ui| {
                    ui.label("Frame:");
                    ui.label(format!("{}", app.frame_counter));
                });

                ui.separator();

                // FPS graph.
                if self.show_fps_graph {
                    ui.checkbox(&mut self.show_fps_graph, "FPS Graph");
                    self.draw_graph(ui, &self.fps_history, egui::Color32::from_rgb(100, 200, 255), "FPS", "", 60.0, 30.0);
                } else {
                    ui.checkbox(&mut self.show_fps_graph, "FPS Graph");
                }

                ui.add_space(4.0);

                // Frame time graph.
                if self.show_frame_time_graph {
                    ui.checkbox(&mut self.show_frame_time_graph, "Frame Time Graph");
                    self.draw_graph(ui, &self.frame_time_history, egui::Color32::from_rgb(255, 180, 100), "Frame", "ms", 16.67, 33.33);
                } else {
                    ui.checkbox(&mut self.show_frame_time_graph, "Frame Time Graph");
                }

                ui.separator();

                // Scene stats.
                ui.label(egui::RichText::new("Scene").strong());
                ui.separator();

                let mut node_counts = std::collections::HashMap::new();
                let mut total_nodes = 0u32;

                for node in &app.scene.nodes {
                    total_nodes += 1;
                    let type_name = match &node.node_type {
                        crate::scene::NodeType::Empty => "Empty",
                        crate::scene::NodeType::Mesh { .. } => "Mesh",
                        crate::scene::NodeType::Light { .. } => "Light",
                        crate::scene::NodeType::Camera { .. } => "Camera",
                    };
                    *node_counts.entry(type_name).or_insert(0u32) += 1;
                }

                egui::Grid::new("scene_stats_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                    ui.label("Total Nodes:");
                    ui.label(format!("{}", total_nodes));
                    ui.end_row();

                    for (type_name, count) in &node_counts {
                        ui.label(format!("  {}:", type_name));
                        ui.label(format!("{}", count));
                        ui.end_row();
                    }
                });

                ui.separator();

                // Selection info.
                ui.label(egui::RichText::new("Selection").strong());
                ui.separator();
                if let Some(id) = app.selection.selected_id {
                    if let Some(node) = app.scene.find_node(id) {
                        egui::Grid::new("selection_grid").num_columns(2).spacing([8.0, 2.0]).show(ui, |ui| {
                            ui.label("ID:"); ui.label(format!("{}", node.id)); ui.end_row();
                            ui.label("Name:"); ui.label(&node.name); ui.end_row();
                            ui.label("Children:"); ui.label(format!("{}", node.children.len())); ui.end_row();
                        });
                    }
                } else {
                    ui.label(egui::RichText::new("No selection").color(egui::Color32::from_rgb(120, 120, 120)));
                }

                ui.separator();

                // Camera info.
                ui.label(egui::RichText::new("Camera").strong());
                ui.separator();
                let cam = &app.camera;
                egui::Grid::new("camera_grid").num_columns(2).spacing([8.0, 2.0]).show(ui, |ui| {
                    ui.label("Position:");
                    ui.label(egui::RichText::new(format!("({:.2}, {:.2}, {:.2})", cam.position.x, cam.position.y, cam.position.z)).family(egui::FontFamily::Monospace));
                    ui.end_row();

                    if self.detailed {
                        ui.label("Target:");
                        ui.label(egui::RichText::new(format!("({:.2}, {:.2}, {:.2})", cam.target.x, cam.target.y, cam.target.z)).family(egui::FontFamily::Monospace));
                        ui.end_row();

                        ui.label("Distance:");
                        ui.label(format!("{:.2}", (cam.position - cam.target).length()));
                        ui.end_row();
                    }
                });
            });
    }
}

