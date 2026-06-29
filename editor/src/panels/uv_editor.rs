//! UV编辑器面板：UV坐标编辑。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct UvEditorPanel {
    pub visible: bool,
    pub uv_islands: Vec<UvIsland>,
    pub selected_island: Option<usize>,
    pub transform_mode: UvTransform,
    pub uv_scale: f32,
    pub uv_rotation: f32,
    pub uv_offset: [f32; 2],
    pub show_grid: bool,
    pub show_overlaps: bool,
    pub pack_margin: f32,
    pub texture_preview: String,
    pub symmetry_axis: SymmetryAxis,
    pub check_mode: UvCheck,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UvTransform {
    Move,
    Rotate,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymmetryAxis {
    None,
    X,
    Y,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UvCheck {
    None,
    Overlapping,
    Stretching,
    Orientation,
}

#[derive(Debug, Clone)]
pub struct UvIsland {
    pub name: String,
    pub selected: bool,
    pub u_min: f32,
    pub v_min: f32,
    pub u_max: f32,
    pub v_max: f32,
}

impl Default for UvEditorPanel {
    fn default() -> Self {
        Self {
            visible: false,
            uv_islands: vec![
                UvIsland { name: "Island0".into(), selected: true, u_min: 0.0, v_min: 0.0, u_max: 0.5, v_max: 0.5 },
                UvIsland { name: "Island1".into(), selected: false, u_min: 0.5, v_min: 0.5, u_max: 1.0, v_max: 1.0 },
            ],
            selected_island: Some(0),
            transform_mode: UvTransform::Move,
            uv_scale: 1.0,
            uv_rotation: 0.0,
            uv_offset: [0.0, 0.0],
            show_grid: true,
            show_overlaps: false,
            pack_margin: 0.01,
            texture_preview: String::new(),
            symmetry_axis: SymmetryAxis::None,
            check_mode: UvCheck::None,
        }
    }
}

impl EditorPanel for UvEditorPanel {
    fn name(&self) -> &str { "UV Editor" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(700.0)
            .default_height(550.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.transform_mode, UvTransform::Move, "Move");
                    ui.radio_value(&mut self.transform_mode, UvTransform::Rotate, "Rotate");
                    ui.radio_value(&mut self.transform_mode, UvTransform::Scale, "Scale");
                    ui.separator();
                    if ui.button("Pack").clicked() {}
                    if ui.button("Unwrap").clicked() {}
                    if ui.button("Mirror").clicked() {}
                    ui.separator();
                    ui.checkbox(&mut self.show_grid, "Grid");
                    ui.checkbox(&mut self.show_overlaps, "Overlaps");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("UV Canvas");
                        egui::Frame::canvas(ui.style()).show(ui, |ui| {
                            ui.set_min_size(egui::vec2(400.0, 400.0));
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(400.0, 400.0), egui::Sense::hover());
                            let painter = ui.painter();
                            if self.show_grid {
                                for i in 0..=10 {
                                    let p = i as f32 / 10.0;
                                    let x = rect.left() + p * rect.width();
                                    painter.line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())], egui::Stroke::new(0.5, egui::Color32::DARK_GRAY));
                                    let y = rect.top() + p * rect.height();
                                    painter.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], egui::Stroke::new(0.5, egui::Color32::DARK_GRAY));
                                }
                            }
                            for (i, island) in self.uv_islands.iter().enumerate() {
                                let selected = self.selected_island == Some(i);
                                let color = if selected { egui::Color32::YELLOW } else { egui::Color32::LIGHT_BLUE };
                                let x0 = rect.left() + island.u_min * rect.width();
                                let y0 = rect.top() + (1.0 - island.v_max) * rect.height();
                                let x1 = rect.left() + island.u_max * rect.width();
                                let y1 = rect.top() + (1.0 - island.v_min) * rect.height();
                                let island_rect = egui::Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y1));
                                painter.rect_stroke(island_rect, 0.0, egui::Stroke::new(2.0, color), egui::StrokeKind::Middle);
                                painter.rect_filled(island_rect, 0.0, egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 30));
                            }
                        });
                    });
                    ui.vertical(|ui| {
                        ui.label("Transform");
                        ui.horizontal(|ui| {
                            ui.label("Scale:");
                            ui.add(egui::Slider::new(&mut self.uv_scale, 0.01..=10.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Rotation:");
                            ui.add(egui::Slider::new(&mut self.uv_rotation, 0.0..=360.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Offset:");
                            ui.add(egui::DragValue::new(&mut self.uv_offset[0]).speed(0.01));
                            ui.add(egui::DragValue::new(&mut self.uv_offset[1]).speed(0.01));
                        });
                        ui.separator();
                        ui.label("Symmetry");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.symmetry_axis, SymmetryAxis::None, "None");
                            ui.radio_value(&mut self.symmetry_axis, SymmetryAxis::X, "X");
                            ui.radio_value(&mut self.symmetry_axis, SymmetryAxis::Y, "Y");
                            ui.radio_value(&mut self.symmetry_axis, SymmetryAxis::Both, "Both");
                        });
                        ui.separator();
                        ui.label("Check Mode");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.check_mode, UvCheck::None, "None");
                            ui.radio_value(&mut self.check_mode, UvCheck::Overlapping, "Overlap");
                            ui.radio_value(&mut self.check_mode, UvCheck::Stretching, "Stretch");
                            ui.radio_value(&mut self.check_mode, UvCheck::Orientation, "Orient");
                        });
                        ui.separator();
                        ui.label("Pack Margin:");
                        ui.add(egui::Slider::new(&mut self.pack_margin, 0.0..=0.1));
                        ui.separator();
                        ui.label("Texture:");
                        ui.text_edit_singleline(&mut self.texture_preview);
                    });
                });
                ui.separator();
                ui.label("UV Islands:");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (i, island) in self.uv_islands.iter_mut().enumerate() {
                        let selected = self.selected_island == Some(i);
                        if ui.selectable_label(selected, &island.name).clicked() {
                            self.selected_island = Some(i);
                        }
                    }
                });
            });
    }
}
