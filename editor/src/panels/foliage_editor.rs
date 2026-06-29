//! 植被编辑器面板：场景植被绘制。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct FoliageEditorPanel {
    pub visible: bool,
    pub brush_size: f32,
    pub brush_falloff: f32,
    pub density: f32,
    pub min_scale: f32,
    pub max_scale: f32,
    pub random_rotation: bool,
    pub random_tilt: f32,
    pub foliage_types: Vec<FoliageType>,
    pub selected_type: Option<usize>,
    pub paint_mode: PaintMode,
    pub collision_enabled: bool,
    pub cast_shadow: bool,
    pub cull_distance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PaintMode {
    Paint,
    Erase,
    Select,
    Reveal,
}

#[derive(Debug, Clone)]
pub struct FoliageType {
    pub name: String,
    pub mesh_path: String,
    pub enabled: bool,
    pub density_scale: f32,
}

impl Default for FoliageEditorPanel {
    fn default() -> Self {
        Self {
            visible: false,
            brush_size: 100.0,
            brush_falloff: 0.5,
            density: 1.0,
            min_scale: 0.8,
            max_scale: 1.2,
            random_rotation: true,
            random_tilt: 5.0,
            foliage_types: vec![
                FoliageType { name: "Grass".into(), mesh_path: "meshes/grass.fbx".into(), enabled: true, density_scale: 1.0 },
                FoliageType { name: "Tree".into(), mesh_path: "meshes/tree.fbx".into(), enabled: true, density_scale: 0.3 },
                FoliageType { name: "Bush".into(), mesh_path: "meshes/bush.fbx".into(), enabled: false, density_scale: 0.5 },
            ],
            selected_type: Some(0),
            paint_mode: PaintMode::Paint,
            collision_enabled: false,
            cast_shadow: true,
            cull_distance: 5000.0,
        }
    }
}

impl EditorPanel for FoliageEditorPanel {
    fn name(&self) -> &str { "Foliage Editor" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(450.0)
            .default_height(550.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.paint_mode, PaintMode::Paint, "Paint");
                    ui.radio_value(&mut self.paint_mode, PaintMode::Erase, "Erase");
                    ui.radio_value(&mut self.paint_mode, PaintMode::Select, "Select");
                    ui.radio_value(&mut self.paint_mode, PaintMode::Reveal, "Reveal");
                });
                ui.separator();
                ui.heading("Brush");
                ui.horizontal(|ui| {
                    ui.label("Size:");
                    ui.add(egui::Slider::new(&mut self.brush_size, 1.0..=1000.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Falloff:");
                    ui.add(egui::Slider::new(&mut self.brush_falloff, 0.0..=1.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Density:");
                    ui.add(egui::Slider::new(&mut self.density, 0.0..=10.0));
                });
                ui.separator();
                ui.heading("Scale");
                ui.horizontal(|ui| {
                    ui.label("Min:");
                    ui.add(egui::Slider::new(&mut self.min_scale, 0.01..=5.0));
                    ui.label("Max:");
                    ui.add(egui::Slider::new(&mut self.max_scale, 0.01..=10.0));
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.random_rotation, "Random Rotation");
                    ui.label("Tilt:");
                    ui.add(egui::Slider::new(&mut self.random_tilt, 0.0..=45.0));
                });
                ui.separator();
                ui.heading("Foliage Types");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut remove_idx: Option<usize> = None;
                    for (i, ft) in self.foliage_types.iter_mut().enumerate() {
                        let selected = self.selected_type == Some(i);
                        ui.horizontal(|ui| {
                            if ui.selectable_label(selected, &ft.name).clicked() {
                                self.selected_type = Some(i);
                            }
                            ui.checkbox(&mut ft.enabled, "On");
                            ui.add(egui::DragValue::new(&mut ft.density_scale).speed(0.1).range(0.0..=5.0));
                            if ui.button("X").clicked() { remove_idx = Some(i); }
                        });
                    }
                    if let Some(i) = remove_idx { self.foliage_types.remove(i); }
                });
                ui.horizontal(|ui| {
                    if ui.button("Add Type").clicked() {
                        self.foliage_types.push(FoliageType { name: format!("Type {}", self.foliage_types.len()), mesh_path: String::new(), enabled: true, density_scale: 1.0 });
                    }
                });
                ui.separator();
                ui.heading("Settings");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.collision_enabled, "Collision");
                    ui.checkbox(&mut self.cast_shadow, "Cast Shadow");
                });
                ui.horizontal(|ui| {
                    ui.label("Cull Distance:");
                    ui.add(egui::DragValue::new(&mut self.cull_distance).speed(10.0).range(100.0..=20000.0));
                });
            });
    }
}
