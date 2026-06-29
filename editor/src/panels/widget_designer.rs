//! Widget designer panel: widget tree, properties, canvas, animation.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct WidgetDesignerPanel {
    pub visible: bool,
    pub active_tab: WdTab,
    pub widgets: Vec<Widget>,
    pub selected_widget: Option<usize>,
    pub canvas_size: [f32; 2],
    pub show_grid: bool,
    pub grid_snap: bool,
    pub new_widget_name: String,
    pub animations: Vec<WidgetAnimation>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WdTab { Designer, Animations, Events }

#[derive(Debug, Clone)]
pub struct Widget { pub name: String, pub wtype: WidgetType, pub pos: [f32; 2], pub size: [f32; 2], pub visible: bool, pub children: Vec<usize> }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WidgetType { Button, Text, Image, Panel, Slider, CheckBox, TextBox }

#[derive(Debug, Clone)]
pub struct WidgetAnimation { pub name: String, pub duration: f32, pub looping: bool, pub tracks: u32 }

impl Default for WidgetDesignerPanel {
    fn default() -> Self {
        Self {
            visible: false, active_tab: WdTab::Designer,
            widgets: vec![
                Widget { name: "Root".into(), wtype: WidgetType::Panel, pos: [0.0, 0.0], size: [800.0, 600.0], visible: true, children: vec![1, 2] },
                Widget { name: "TitleText".into(), wtype: WidgetType::Text, pos: [50.0, 20.0], size: [300.0, 40.0], visible: true, children: vec![] },
                Widget { name: "StartButton".into(), wtype: WidgetType::Button, pos: [300.0, 300.0], size: [200.0, 60.0], visible: true, children: vec![] },
            ],
            selected_widget: Some(0), canvas_size: [800.0, 600.0], show_grid: true, grid_snap: true, new_widget_name: String::new(),
            animations: vec![WidgetAnimation { name: "FadeIn".into(), duration: 0.5, looping: false, tracks: 2 }],
        }
    }
}

impl EditorPanel for WidgetDesignerPanel {
    fn name(&self) -> &str { "Widget Designer" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(600.0).default_height(450.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, WdTab::Designer, "Designer");
                ui.selectable_value(&mut self.active_tab, WdTab::Animations, "Animations");
                ui.selectable_value(&mut self.active_tab, WdTab::Events, "Events");
            });
            ui.separator();
            match self.active_tab {
                WdTab::Designer => {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label("Widget Tree");
                            ui.separator();
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                for i in 0..self.widgets.len() {
                                    let selected = self.selected_widget == Some(i);
                                    let type_str = match self.widgets[i].wtype { WidgetType::Button => "Btn", WidgetType::Text => "Txt", WidgetType::Image => "Img", WidgetType::Panel => "Pnl", WidgetType::Slider => "Sld", WidgetType::CheckBox => "Chk", WidgetType::TextBox => "Edt" };
                                    if ui.selectable_label(selected, format!("[{}] {}", type_str, self.widgets[i].name)).clicked() { self.selected_widget = Some(i); }
                                }
                            });
                            ui.separator();
                            ui.horizontal(|ui| { ui.text_edit_singleline(&mut self.new_widget_name); if ui.button("Add").clicked() && !self.new_widget_name.is_empty() { self.widgets.push(Widget { name: self.new_widget_name.clone(), wtype: WidgetType::Button, pos: [0.0, 0.0], size: [100.0, 30.0], visible: true, children: vec![] }); self.new_widget_name.clear(); } });
                        });
                        ui.separator();
                        ui.vertical(|ui| {
                            ui.label("Properties");
                            ui.separator();
                            if let Some(idx) = self.selected_widget { if idx < self.widgets.len() {
                                ui.checkbox(&mut self.widgets[idx].visible, "Visible");
                                ui.horizontal(|ui| { ui.label("Pos:"); ui.add(egui::DragValue::new(&mut self.widgets[idx].pos[0]).range(0.0..=2000.0)); ui.add(egui::DragValue::new(&mut self.widgets[idx].pos[1]).range(0.0..=2000.0)); });
                                ui.horizontal(|ui| { ui.label("Size:"); ui.add(egui::DragValue::new(&mut self.widgets[idx].size[0]).range(1.0..=2000.0)); ui.add(egui::DragValue::new(&mut self.widgets[idx].size[1]).range(1.0..=2000.0)); });
                            }}
                            ui.separator();
                            ui.checkbox(&mut self.show_grid, "Show Grid");
                            ui.checkbox(&mut self.grid_snap, "Grid Snap");
                            ui.horizontal(|ui| { ui.label("Canvas:"); ui.add(egui::DragValue::new(&mut self.canvas_size[0]).range(100.0..=4000.0)); ui.add(egui::DragValue::new(&mut self.canvas_size[1]).range(100.0..=4000.0)); });
                        });
                    });
                },
                WdTab::Animations => {
                    ui.heading("Animations");
                    for i in 0..self.animations.len() { ui.horizontal(|ui| { ui.label(&self.animations[i].name); ui.label(format!("{:.1}s", self.animations[i].duration)); ui.checkbox(&mut self.animations[i].looping, "Loop"); ui.label(format!("{} tracks", self.animations[i].tracks)); }); }
                    if ui.button("Add Animation").clicked() { self.animations.push(WidgetAnimation { name: "NewAnim".into(), duration: 1.0, looping: false, tracks: 1 }); }
                },
                WdTab::Events => {
                    ui.heading("Events");
                    ui.label("On Click");
                    ui.label("On Hover");
                    ui.label("On Focus");
                    ui.label("On Unfocus");
                },
            }
        });
    }
}
