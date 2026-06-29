//! 曲线编辑器面板：关键帧曲线编辑。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct CurveEditorPanel {
    pub visible: bool,
    pub curves: Vec<Curve>,
    pub selected_curve: Option<usize>,
    pub selected_key: Option<usize>,
    pub loop_mode: LoopMode,
    pub tangent_mode: TangentMode,
    pub show_grid: bool,
    pub snap_to_grid: bool,
    pub time_range: [f32; 2],
    pub value_range: [f32; 2],
    pub preview_time: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoopMode {
    None,
    Repeat,
    PingPong,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TangentMode {
    Auto,
    Linear,
    Constant,
    Bezier,
}

#[derive(Debug, Clone)]
pub struct Curve {
    pub name: String,
    pub color: egui::Color32,
    pub keys: Vec<Keyframe>,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Keyframe {
    pub time: f32,
    pub value: f32,
    pub in_tangent: f32,
    pub out_tangent: f32,
}

impl Default for CurveEditorPanel {
    fn default() -> Self {
        Self {
            visible: false,
            curves: vec![
                Curve {
                    name: "X".into(),
                    color: egui::Color32::RED,
                    visible: true,
                    keys: vec![
                        Keyframe { time: 0.0, value: 0.0, in_tangent: 0.0, out_tangent: 1.0 },
                        Keyframe { time: 1.0, value: 1.0, in_tangent: 1.0, out_tangent: 0.0 },
                    ],
                },
                Curve {
                    name: "Y".into(),
                    color: egui::Color32::GREEN,
                    visible: true,
                    keys: vec![
                        Keyframe { time: 0.0, value: 0.0, in_tangent: 0.0, out_tangent: 0.0 },
                        Keyframe { time: 0.5, value: 0.5, in_tangent: 0.5, out_tangent: 0.5 },
                        Keyframe { time: 1.0, value: 0.0, in_tangent: 0.0, out_tangent: 0.0 },
                    ],
                },
            ],
            selected_curve: Some(0),
            selected_key: Some(0),
            loop_mode: LoopMode::None,
            tangent_mode: TangentMode::Auto,
            show_grid: true,
            snap_to_grid: false,
            time_range: [0.0, 1.0],
            value_range: [0.0, 1.0],
            preview_time: 0.0,
        }
    }
}

impl EditorPanel for CurveEditorPanel {
    fn name(&self) -> &str { "Curve Editor" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(700.0)
            .default_height(500.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Curves:");
                    for (i, c) in self.curves.iter().enumerate() {
                        let selected = self.selected_curve == Some(i);
                        if ui.selectable_label(selected, format!("{}", c.name)).clicked() {
                            self.selected_curve = Some(i);
                        }
                    }
                    ui.separator();
                    if ui.button("Add Curve").clicked() {
                        self.curves.push(Curve { name: format!("C{}", self.curves.len()), color: egui::Color32::BLUE, visible: true, keys: vec![] });
                    }
                    ui.separator();
                    ui.checkbox(&mut self.show_grid, "Grid");
                    ui.checkbox(&mut self.snap_to_grid, "Snap");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Loop:");
                    ui.radio_value(&mut self.loop_mode, LoopMode::None, "None");
                    ui.radio_value(&mut self.loop_mode, LoopMode::Repeat, "Repeat");
                    ui.radio_value(&mut self.loop_mode, LoopMode::PingPong, "PingPong");
                    ui.separator();
                    ui.label("Tangent:");
                    ui.radio_value(&mut self.tangent_mode, TangentMode::Auto, "Auto");
                    ui.radio_value(&mut self.tangent_mode, TangentMode::Linear, "Linear");
                    ui.radio_value(&mut self.tangent_mode, TangentMode::Constant, "Constant");
                    ui.radio_value(&mut self.tangent_mode, TangentMode::Bezier, "Bezier");
                });
                ui.separator();
                ui.label("Canvas:");
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    ui.set_min_size(egui::vec2(650.0, 250.0));
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(650.0, 250.0), egui::Sense::hover());
                    let painter = ui.painter();
                    if self.show_grid {
                        for i in 0..=10 {
                            let x = rect.left() + (i as f32 / 10.0) * rect.width();
                            painter.line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())], egui::Stroke::new(0.5, egui::Color32::DARK_GRAY));
                            let y = rect.top() + (i as f32 / 10.0) * rect.height();
                            painter.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], egui::Stroke::new(0.5, egui::Color32::DARK_GRAY));
                        }
                    }
                    for (ci, curve) in self.curves.iter().enumerate() {
                        if !curve.visible { continue; }
                        let selected = self.selected_curve == Some(ci);
                        let stroke = if selected { egui::Stroke::new(2.0, curve.color) } else { egui::Stroke::new(1.0, curve.color) };
                        for w in curve.keys.windows(2) {
                            let p1 = egui::pos2(
                                rect.left() + (w[0].time - self.time_range[0]) / (self.time_range[1] - self.time_range[0]) * rect.width(),
                                rect.bottom() - (w[0].value - self.value_range[0]) / (self.value_range[1] - self.value_range[0]) * rect.height(),
                            );
                            let p2 = egui::pos2(
                                rect.left() + (w[1].time - self.time_range[0]) / (self.time_range[1] - self.time_range[0]) * rect.width(),
                                rect.bottom() - (w[1].value - self.value_range[0]) / (self.value_range[1] - self.value_range[0]) * rect.height(),
                            );
                            painter.line_segment([p1, p2], stroke);
                        }
                        for (ki, k) in curve.keys.iter().enumerate() {
                            let p = egui::pos2(
                                rect.left() + (k.time - self.time_range[0]) / (self.time_range[1] - self.time_range[0]) * rect.width(),
                                rect.bottom() - (k.value - self.value_range[0]) / (self.value_range[1] - self.value_range[0]) * rect.height(),
                            );
                            let size = if selected && self.selected_key == Some(ki) { 5.0 } else { 3.0 };
                            painter.circle_filled(p, size, curve.color);
                        }
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Preview:");
                    ui.add(egui::Slider::new(&mut self.preview_time, self.time_range[0]..=self.time_range[1]));
                });
                ui.separator();
                if let Some(ci) = self.selected_curve {
                    if ci < self.curves.len() {
                        let curve = &mut self.curves[ci];
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut curve.visible, "Visible");
                            ui.color_edit_button_srgba(&mut curve.color);
                            ui.text_edit_singleline(&mut curve.name);
                            if ui.button("Add Key").clicked() {
                                curve.keys.push(Keyframe { time: self.preview_time, value: 0.5, in_tangent: 0.0, out_tangent: 0.0 });
                            }
                        });
                        if let Some(ki) = self.selected_key {
                            if ki < curve.keys.len() {
                                let k = &mut curve.keys[ki];
                                ui.horizontal(|ui| {
                                    ui.label("Time:");
                                    ui.add(egui::DragValue::new(&mut k.time).speed(0.01));
                                    ui.label("Value:");
                                    ui.add(egui::DragValue::new(&mut k.value).speed(0.01));
                                    ui.label("In Tan:");
                                    ui.add(egui::DragValue::new(&mut k.in_tangent).speed(0.1));
                                    ui.label("Out Tan:");
                                    ui.add(egui::DragValue::new(&mut k.out_tangent).speed(0.1));
                                });
                            }
                        }
                    }
                }
            });
    }
}
