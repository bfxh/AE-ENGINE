use crate::app::EditorApp;
use crate::panels::EditorPanel;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TerrainTool {
    Sculpt,
    Smooth,
    Paint,
    Flatten,
    Noise,
    Erode,
}

impl TerrainTool {
    fn label(self) -> &'static str {
        match self {
            TerrainTool::Sculpt => "Sculpt",
            TerrainTool::Smooth => "Smooth",
            TerrainTool::Paint => "Paint",
            TerrainTool::Flatten => "Flatten",
            TerrainTool::Noise => "Noise",
            TerrainTool::Erode => "Erode",
        }
    }
    fn icon(self) -> &'static str {
        match self {
            TerrainTool::Sculpt => "\u{25B2}",
            TerrainTool::Smooth => "\u{223F}",
            TerrainTool::Paint => "\u{25A0}",
            TerrainTool::Flatten => "\u{25AC}",
            TerrainTool::Noise => "\u{2726}",
            TerrainTool::Erode => "\u{2248}",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BrushShape {
    Circle,
    Square,
    Diamond,
}

impl BrushShape {
    fn label(self) -> &'static str {
        match self {
            BrushShape::Circle => "Circle",
            BrushShape::Square => "Square",
            BrushShape::Diamond => "Diamond",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerrainLayer {
    pub name: String,
    pub color: [f32; 4],
    pub visible: bool,
    pub blend_weight: f32,
}

pub struct TerrainEditorPanel {
    pub visible: bool,
    pub tool: TerrainTool,
    pub brush_size: f32,
    pub brush_strength: f32,
    pub brush_falloff: f32,
    pub brush_shape: BrushShape,
    pub resolution: u32,
    pub height_scale: f32,
    pub world_size: f32,
    pub layers: Vec<TerrainLayer>,
    pub selected_layer: usize,
    pub max_height: f32,
    pub min_height: f32,
    pub avg_height: f32,
    pub undo_count: usize,
    pub redo_count: usize,
    pub flatten_target: f32,
    pub noise_scale: f32,
    pub heightmap_format: u32,
}

impl Default for TerrainEditorPanel {
    fn default() -> Self {
        Self {
            visible: false,
            tool: TerrainTool::Sculpt,
            brush_size: 25.0,
            brush_strength: 0.5,
            brush_falloff: 0.5,
            brush_shape: BrushShape::Circle,
            resolution: 256,
            height_scale: 100.0,
            world_size: 1000.0,
            layers: vec![
                TerrainLayer { name: "Base".into(), color: [0.5, 0.4, 0.3, 1.0], visible: true, blend_weight: 1.0 },
                TerrainLayer { name: "Grass".into(), color: [0.3, 0.6, 0.2, 1.0], visible: true, blend_weight: 0.7 },
                TerrainLayer { name: "Rock".into(), color: [0.5, 0.5, 0.5, 1.0], visible: true, blend_weight: 0.5 },
                TerrainLayer { name: "Snow".into(), color: [0.9, 0.9, 0.95, 1.0], visible: false, blend_weight: 0.3 },
                TerrainLayer { name: "Dirt".into(), color: [0.4, 0.3, 0.2, 1.0], visible: true, blend_weight: 0.4 },
            ],
            selected_layer: 0,
            max_height: 95.2,
            min_height: 0.0,
            avg_height: 42.7,
            undo_count: 0,
            redo_count: 0,
            flatten_target: 50.0,
            noise_scale: 1.0,
            heightmap_format: 0,
        }
    }
}

impl EditorPanel for TerrainEditorPanel {
    fn name(&self) -> &str { "Terrain Editor" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new("Terrain Editor")
            .default_width(380.0)
            .default_height(600.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.add_enabled(self.undo_count > 0, egui::Button::new("Undo")).clicked() {
                        self.undo_count -= 1;
                        self.redo_count += 1;
                    }
                    if ui.add_enabled(self.redo_count > 0, egui::Button::new("Redo")).clicked() {
                        self.redo_count -= 1;
                        self.undo_count += 1;
                    }
                    ui.separator();
                    ui.label(format!("History: {} / {}", self.undo_count, self.redo_count));
                });
                ui.separator();

                ui.collapsing("Brush Tools", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for &t in &[TerrainTool::Sculpt, TerrainTool::Smooth, TerrainTool::Paint, TerrainTool::Flatten, TerrainTool::Noise, TerrainTool::Erode] {
                            let label = format!("{} {}", t.icon(), t.label());
                            if ui.selectable_label(self.tool == t, label).clicked() {
                                self.tool = t;
                            }
                        }
                    });
                });

                ui.collapsing("Brush Parameters", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Size:");
                        ui.add(egui::Slider::new(&mut self.brush_size, 1.0..=200.0).suffix(" m"));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Strength:");
                        ui.add(egui::Slider::new(&mut self.brush_strength, 0.0..=1.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Falloff:");
                        ui.add(egui::Slider::new(&mut self.brush_falloff, 0.0..=1.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Shape:");
                        egui::ComboBox::from_id_salt("brush_shape")
                            .selected_text(self.brush_shape.label())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.brush_shape, BrushShape::Circle, "Circle");
                                ui.selectable_value(&mut self.brush_shape, BrushShape::Square, "Square");
                                ui.selectable_value(&mut self.brush_shape, BrushShape::Diamond, "Diamond");
                            });
                    });
                    match self.tool {
                        TerrainTool::Flatten => {
                            ui.horizontal(|ui| {
                                ui.label("Target Height:");
                                ui.add(egui::Slider::new(&mut self.flatten_target, 0.0..=self.height_scale));
                            });
                        }
                        TerrainTool::Noise => {
                            ui.horizontal(|ui| {
                                ui.label("Noise Scale:");
                                ui.add(egui::Slider::new(&mut self.noise_scale, 0.1..=10.0));
                            });
                        }
                        _ => {}
                    }
                });

                ui.collapsing("Brush Preview", |ui| {
                    egui::Frame::canvas(ui.style()).show(ui, |ui| {
                        ui.set_min_size(egui::vec2(340.0, 160.0));
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(340.0, 160.0), egui::Sense::hover());
                        let painter = ui.painter();
                        let left_rect = egui::Rect::from_min_size(rect.min, egui::vec2(150.0, 150.0));
                        let right_rect = egui::Rect::from_min_size(egui::pos2(rect.min.x + 160.0, rect.min.y), egui::vec2(170.0, 150.0));

                        painter.rect_filled(left_rect, 2.0, egui::Color32::from_rgb(30, 30, 35));
                        painter.rect_filled(right_rect, 2.0, egui::Color32::from_rgb(30, 30, 35));

                        let center = left_rect.center();
                        let radius = (self.brush_size / 200.0 * 60.0).clamp(10.0, 65.0);
                        for i in 0..10usize {
                            let t = i as f32 / 10.0;
                            let r = radius * (1.0 - t * 0.1);
                            let alpha = ((1.0 - t) * (1.0 - self.brush_falloff * 0.5) * 200.0).clamp(0.0, 255.0) as u8;
                            let color = egui::Color32::from_rgba_unmultiplied(100, 180, 255, alpha);
                            painter.circle_filled(center, r, color);
                        }
                        let stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 255));
                        match self.brush_shape {
                            BrushShape::Circle => {
                                painter.circle_stroke(center, radius, stroke);
                            }
                            BrushShape::Square => {
                                let r = egui::Rect::from_center_size(center, egui::vec2(radius * 2.0, radius * 2.0));
                                painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Middle);
                            }
                            BrushShape::Diamond => {
                                let p1 = egui::pos2(center.x, center.y - radius);
                                let p2 = egui::pos2(center.x + radius, center.y);
                                let p3 = egui::pos2(center.x, center.y + radius);
                                let p4 = egui::pos2(center.x - radius, center.y);
                                painter.line_segment([p1, p2], stroke);
                                painter.line_segment([p2, p3], stroke);
                                painter.line_segment([p3, p4], stroke);
                                painter.line_segment([p4, p1], stroke);
                            }
                        }
                        painter.text(left_rect.left_top() + egui::vec2(4.0, 2.0), egui::Align2::LEFT_TOP, "Shape", egui::FontId::proportional(10.0), egui::Color32::LIGHT_GRAY);

                        let curve_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 200, 100));
                        let mut prev: Option<egui::Pos2> = None;
                        for i in 0..=40usize {
                            let t = i as f32 / 40.0;
                            let strength = (1.0 - t).powf(1.0 + self.brush_falloff * 4.0) * self.brush_strength;
                            let x = right_rect.left() + t * right_rect.width();
                            let y = right_rect.bottom() - strength * right_rect.height();
                            let p = egui::pos2(x, y);
                            if let Some(p0) = prev {
                                painter.line_segment([p0, p], curve_stroke);
                            }
                            prev = Some(p);
                        }
                        painter.line_segment([right_rect.left_bottom(), right_rect.right_bottom()], egui::Stroke::new(1.0, egui::Color32::GRAY));
                        painter.line_segment([right_rect.left_bottom(), right_rect.left_top()], egui::Stroke::new(1.0, egui::Color32::GRAY));
                        painter.text(right_rect.left_top() + egui::vec2(4.0, 2.0), egui::Align2::LEFT_TOP, "Falloff", egui::FontId::proportional(10.0), egui::Color32::LIGHT_GRAY);
                    });
                });

                ui.collapsing("Terrain Settings", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Resolution:");
                        ui.add(egui::DragValue::new(&mut self.resolution).range(32..=2048).suffix(" px"));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Height Scale:");
                        ui.add(egui::DragValue::new(&mut self.height_scale).speed(1.0).range(1.0..=10000.0).suffix(" m"));
                    });
                    ui.horizontal(|ui| {
                        ui.label("World Size:");
                        ui.add(egui::DragValue::new(&mut self.world_size).speed(10.0).range(100.0..=100000.0).suffix(" m"));
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Generate Terrain").clicked() {
                            self.max_height = self.height_scale * 0.95;
                            self.min_height = 0.0;
                            self.avg_height = self.height_scale * 0.42;
                            self.undo_count = 0;
                            self.redo_count = 0;
                        }
                        if ui.button("Clear").clicked() {
                            self.max_height = 0.0;
                            self.min_height = 0.0;
                            self.avg_height = 0.0;
                        }
                    });
                });

                ui.collapsing("Layers", |ui| {
                    let mut remove_idx: Option<usize> = None;
                    for (i, layer) in self.layers.iter_mut().enumerate() {
                        let selected = self.selected_layer == i;
                        ui.horizontal(|ui| {
                            if ui.selectable_label(selected, layer.name.as_str()).clicked() {
                                self.selected_layer = i;
                            }
                            ui.color_edit_button_rgba_unmultiplied(&mut layer.color);
                            ui.checkbox(&mut layer.visible, "Vis");
                            ui.add(egui::Slider::new(&mut layer.blend_weight, 0.0..=1.0));
                            if ui.button("X").clicked() { remove_idx = Some(i); }
                        });
                        if selected {
                            ui.horizontal(|ui| {
                                ui.label("Rename:");
                                ui.text_edit_singleline(&mut layer.name);
                            });
                        }
                    }
                    if let Some(i) = remove_idx {
                        if self.layers.len() > 1 {
                            self.layers.remove(i);
                            if self.selected_layer >= self.layers.len() {
                                self.selected_layer = self.layers.len() - 1;
                            }
                        }
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Add Layer").clicked() {
                            self.layers.push(TerrainLayer {
                                name: format!("Layer {}", self.layers.len()),
                                color: [0.5, 0.5, 0.5, 1.0],
                                visible: true,
                                blend_weight: 0.5,
                            });
                        }
                    });
                });

                ui.collapsing("Heightmap", |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Import Heightmap...").clicked() {}
                        if ui.button("Export Heightmap...").clicked() {}
                    });
                    ui.horizontal(|ui| {
                        ui.label("Format:");
                        egui::ComboBox::from_id_salt("heightmap_format")
                            .selected_text(match self.heightmap_format {
                                0 => "RAW 16-bit",
                                1 => "PNG 8-bit",
                                2 => "EXR 32-bit",
                                _ => "?",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.heightmap_format, 0u32, "RAW 16-bit");
                                ui.selectable_value(&mut self.heightmap_format, 1u32, "PNG 8-bit");
                                ui.selectable_value(&mut self.heightmap_format, 2u32, "EXR 32-bit");
                            });
                    });
                });

                ui.collapsing("Statistics", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Max Height:");
                        ui.label(format!("{:.1} m", self.max_height));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Min Height:");
                        ui.label(format!("{:.1} m", self.min_height));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Avg Height:");
                        ui.label(format!("{:.1} m", self.avg_height));
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Resolution:");
                        ui.label(format!("{}x{}", self.resolution, self.resolution));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Triangles:");
                        let tris = self.resolution.saturating_sub(1) * self.resolution.saturating_sub(1) * 2;
                        ui.label(format!("{}", tris));
                    });
                });
            });
    }
}
