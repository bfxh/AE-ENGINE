//! Measure Tool panel: measure distances, angles, and areas in 3D space.
//!
//! Provides point-to-point measurement, angle measurement, and area calculation.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

/// Measurement unit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Unit {
    Meters,
    Centimeters,
    Feet,
    Inches,
}

impl Unit {
    fn label(&self) -> &'static str {
        match self {
            Unit::Meters => "m",
            Unit::Centimeters => "cm",
            Unit::Feet => "ft",
            Unit::Inches => "in",
        }
    }

    fn from_meters(&self, m: f32) -> f32 {
        match self {
            Unit::Meters => m,
            Unit::Centimeters => m * 100.0,
            Unit::Feet => m * 3.28084,
            Unit::Inches => m * 39.3701,
        }
    }
}

/// A measurement record.
#[derive(Clone, Debug)]
pub struct Measurement {
    pub name: String,
    pub kind: MeasurementKind,
    pub points: Vec<[f32; 3]>,
    pub value: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MeasurementKind {
    Distance,
    Angle,
    Area,
    Perimeter,
}

impl MeasurementKind {
    fn label(&self) -> &'static str {
        match self {
            MeasurementKind::Distance => "Distance",
            MeasurementKind::Angle => "Angle",
            MeasurementKind::Area => "Area",
            MeasurementKind::Perimeter => "Perimeter",
        }
    }

    #[allow(dead_code)]
    fn unit(&self, u: Unit) -> String {
        match self {
            MeasurementKind::Distance => u.label().to_string(),
            MeasurementKind::Angle => "deg".to_string(),
            MeasurementKind::Area => format!("{}^2", u.label()),
            MeasurementKind::Perimeter => u.label().to_string(),
        }
    }
}

/// Measure Tool panel state.
pub struct MeasureToolPanel {
    pub visible: bool,
    pub points: Vec<[f32; 3]>,
    pub unit: Unit,
    pub measurements: Vec<Measurement>,
    pub selected: Option<usize>,
    pub snap_to_grid: bool,
    pub show_labels: bool,
}

impl Default for MeasureToolPanel {
    fn default() -> Self {
        Self {
            visible: false,
            points: Vec::new(),
            unit: Unit::Meters,
            measurements: Vec::new(),
            selected: None,
            snap_to_grid: true,
            show_labels: true,
        }
    }
}

impl MeasureToolPanel {
    fn add_point(&mut self, pos: glam::Vec3) {
        let p = if self.snap_to_grid {
            [pos.x.round(), pos.y.round(), pos.z.round()]
        } else {
            [pos.x, pos.y, pos.z]
        };
        self.points.push(p);
    }

    fn clear_points(&mut self) {
        self.points.clear();
    }

    fn total_distance(&self) -> f32 {
        let mut total = 0.0f32;
        for i in 1..self.points.len() {
            let a = self.points[i - 1];
            let b = self.points[i];
            total += ((a[0]-b[0]).powi(2) + (a[1]-b[1]).powi(2) + (a[2]-b[2]).powi(2)).sqrt();
        }
        total
    }

    fn last_segment_distance(&self) -> Option<f32> {
        if self.points.len() < 2 { return None; }
        let a = self.points[self.points.len() - 2];
        let b = self.points[self.points.len() - 1];
        Some(((a[0]-b[0]).powi(2) + (a[1]-b[1]).powi(2) + (a[2]-b[2]).powi(2)).sqrt())
    }

    fn angle_at_last(&self) -> Option<f32> {
        if self.points.len() < 3 { return None; }
        let n = self.points.len();
        let a = self.points[n - 3];
        let b = self.points[n - 2];
        let c = self.points[n - 1];

        let v1 = glam::Vec3::new(b[0]-a[0], b[1]-a[1], b[2]-a[2]);
        let v2 = glam::Vec3::new(c[0]-b[0], c[1]-b[1], c[2]-b[2]);

        let dot = v1.dot(v2);
        let mag = v1.length() * v2.length();
        if mag < 0.0001 { return None; }
        let cos_angle = (dot / mag).clamp(-1.0, 1.0);
        Some(cos_angle.acos().to_degrees())
    }

    fn polygon_area(&self) -> Option<f32> {
        if self.points.len() < 3 { return None; }
        // Newell's method for polygon area.
        let mut normal = glam::Vec3::ZERO;
        for i in 0..self.points.len() {
            let curr = self.points[i];
            let next = self.points[(i + 1) % self.points.len()];
            normal.x += (curr[1] - next[1]) * (curr[2] + next[2]);
            normal.y += (curr[2] - next[2]) * (curr[0] + next[0]);
            normal.z += (curr[0] - next[0]) * (curr[1] + next[1]);
        }
        Some(normal.length() * 0.5)
    }
}

impl EditorPanel for MeasureToolPanel {
    fn name(&self) -> &str { "Measure Tool" }

    fn render(&mut self, ctx: &egui::Context, app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new("Measure Tool").default_width(360.0).default_height(450.0).resizable(true).show(ctx, |ui| {
            // Toolbar.
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Unit:").small().color(egui::Color32::from_rgb(150, 150, 160)));
                egui::ComboBox::from_id_salt("measure_unit")
                    .selected_text(self.unit.label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.unit, Unit::Meters, "Meters (m)");
                        ui.selectable_value(&mut self.unit, Unit::Centimeters, "Centimeters (cm)");
                        ui.selectable_value(&mut self.unit, Unit::Feet, "Feet (ft)");
                        ui.selectable_value(&mut self.unit, Unit::Inches, "Inches (in)");
                    });

                ui.separator();

                ui.checkbox(&mut self.snap_to_grid, "Snap to Grid");
                ui.checkbox(&mut self.show_labels, "Labels");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Clear Points").clicked() { self.clear_points(); }
                });
            });

            ui.separator();

            // Point input.
            ui.label(egui::RichText::new("Points").strong().color(egui::Color32::from_rgb(180, 200, 220)));
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("+ Add Camera Target").clicked() {
                    self.add_point(app.camera.target);
                }
                if ui.button("+ Add Camera Position").clicked() {
                    self.add_point(app.camera.position);
                }
                if let Some(id) = app.selection.selected_id {
                    if let Some(node) = app.scene.find_node(id) {
                        if ui.button("+ Add Selected Node").clicked() {
                            self.add_point(node.transform.translation);
                        }
                    }
                }
            });

            ui.add_space(4.0);

            // Point list.
            egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                if self.points.is_empty() {
                    ui.label(egui::RichText::new("No points added. Use buttons above to add points.").small().color(egui::Color32::from_rgb(120, 120, 120)));
                } else {
                    let mut to_remove: Option<usize> = None;
                    for (i, p) in self.points.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("P{}:", i)).color(egui::Color32::from_rgb(255, 220, 100)).strong().family(egui::FontFamily::Monospace));
                            ui.label(egui::RichText::new(format!("({:.2}, {:.2}, {:.2})", p[0], p[1], p[2])).family(egui::FontFamily::Monospace).small());
                            if ui.small_button("X").clicked() { to_remove = Some(i); }
                        });
                    }
                    if let Some(i) = to_remove { self.points.remove(i); }
                }
            });

            ui.separator();

            // Measurements display.
            ui.label(egui::RichText::new("Live Measurements").strong().color(egui::Color32::from_rgb(180, 200, 220)));
            ui.separator();

            egui::Grid::new("live_measurements").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                // Point count.
                ui.label("Points:");
                ui.label(format!("{}", self.points.len()));
                ui.end_row();

                // Last segment distance.
                if let Some(dist) = self.last_segment_distance() {
                    ui.label("Last Segment:");
                    ui.label(egui::RichText::new(format!("{:.3} {}", self.unit.from_meters(dist), self.unit.label())).color(egui::Color32::from_rgb(100, 255, 100)).strong().family(egui::FontFamily::Monospace));
                    ui.end_row();
                }

                // Total distance.
                if self.points.len() >= 2 {
                    let total = self.total_distance();
                    ui.label("Total Path:");
                    ui.label(egui::RichText::new(format!("{:.3} {}", self.unit.from_meters(total), self.unit.label())).color(egui::Color32::from_rgb(100, 200, 255)).strong().family(egui::FontFamily::Monospace));
                    ui.end_row();
                }

                // Angle.
                if let Some(angle) = self.angle_at_last() {
                    ui.label("Angle at last:");
                    ui.label(egui::RichText::new(format!("{:.1} deg", angle)).color(egui::Color32::from_rgb(255, 200, 100)).strong().family(egui::FontFamily::Monospace));
                    ui.end_row();
                }

                // Area.
                if let Some(area) = self.polygon_area() {
                    ui.label("Polygon Area:");
                    ui.label(egui::RichText::new(format!("{:.3} {}^2", self.unit.from_meters(area), self.unit.label())).color(egui::Color32::from_rgb(255, 150, 200)).strong().family(egui::FontFamily::Monospace));
                    ui.end_row();
                }
            });

            ui.add_space(4.0);

            // Save measurement.
            ui.horizontal(|ui| {
                if self.points.len() >= 2 {
                    if ui.button("Save as Distance").clicked() && self.points.len() >= 2 {
                        let dist = self.total_distance();
                        self.measurements.push(Measurement {
                            name: format!("D{}", self.measurements.len()),
                            kind: MeasurementKind::Distance,
                            points: self.points.clone(),
                            value: dist,
                        });
                    }
                    if self.points.len() >= 3 {
                        if let Some(angle) = self.angle_at_last() {
                            if ui.button("Save Angle").clicked() {
                                self.measurements.push(Measurement {
                                    name: format!("A{}", self.measurements.len()),
                                    kind: MeasurementKind::Angle,
                                    points: self.points.clone(),
                                    value: angle,
                                });
                            }
                        }
                        if let Some(area) = self.polygon_area() {
                            if ui.button("Save Area").clicked() {
                                self.measurements.push(Measurement {
                                    name: format!("S{}", self.measurements.len()),
                                    kind: MeasurementKind::Area,
                                    points: self.points.clone(),
                                    value: area,
                                });
                            }
                        }
                    }
                }
            });

            ui.separator();

            // Saved measurements.
            ui.label(egui::RichText::new(format!("Saved Measurements ({})", self.measurements.len())).strong().color(egui::Color32::from_rgb(180, 200, 220)));
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut to_remove: Option<usize> = None;
                for (i, m) in self.measurements.iter().enumerate() {
                    let is_sel = self.selected == Some(i);
                    let value_str = match m.kind {
                        MeasurementKind::Angle => format!("{:.2} deg", m.value),
                        MeasurementKind::Area => format!("{:.3} {}^2", self.unit.from_meters(m.value), self.unit.label()),
                        _ => format!("{:.3} {}", self.unit.from_meters(m.value), self.unit.label()),
                    };
                    let kind_color = match m.kind {
                        MeasurementKind::Distance => egui::Color32::from_rgb(100, 200, 255),
                        MeasurementKind::Angle => egui::Color32::from_rgb(255, 200, 100),
                        MeasurementKind::Area => egui::Color32::from_rgb(255, 150, 200),
                        MeasurementKind::Perimeter => egui::Color32::from_rgb(150, 255, 150),
                    };
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("[{}]", m.kind.label())).color(kind_color).small());
                        let response = ui.selectable_label(is_sel, egui::RichText::new(&m.name).strong());
                        if response.clicked() { self.selected = Some(i); }
                        ui.label(egui::RichText::new(value_str).family(egui::FontFamily::Monospace).color(kind_color));
                        if ui.small_button("X").clicked() { to_remove = Some(i); }
                    });
                }
                if let Some(i) = to_remove { self.measurements.remove(i); if self.selected == Some(i) { self.selected = None; } }

                if self.measurements.is_empty() {
                    ui.label(egui::RichText::new("No saved measurements").small().color(egui::Color32::from_rgb(120, 120, 120)));
                }
            });

            ui.separator();

            // Export.
            ui.horizontal(|ui| {
                if ui.button("Export to Clipboard").clicked() && !self.measurements.is_empty() {
                    let mut text = String::from("Measurements:\n");
                    for m in &self.measurements {
                        text.push_str(&format!("  {} ({}): {:.4}\n", m.name, m.kind.label(), m.value));
                    }
                    ui.ctx().copy_text(text);
                }
                if ui.button("Clear All").clicked() {
                    self.points.clear();
                    self.measurements.clear();
                    self.selected = None;
                }
            });
        });
    }
}
