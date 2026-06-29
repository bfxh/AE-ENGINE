//! Tilemap editor panel: tileset, brush, layers, collision.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct TilemapEditorPanel {
    pub visible: bool,
    pub tileset_name: String,
    pub tile_size: [u32; 2],
    pub map_size: [u32; 2],
    pub layers: Vec<TileLayer>,
    pub selected_layer: Option<usize>,
    pub active_brush: BrushType,
    pub brush_size: u32,
    pub show_grid: bool,
    pub show_collision: bool,
    pub selected_tile: Option<(u32, u32)>,
}

#[derive(Debug, Clone)]
pub struct TileLayer { pub name: String, pub visible: bool, pub opacity: f32, pub locked: bool }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BrushType { Paint, Erase, Fill, Rectangle, Line, Bucket }

impl Default for TilemapEditorPanel {
    fn default() -> Self {
        Self {
            visible: false, tileset_name: "tiles.png".into(), tile_size: [32, 32], map_size: [64, 64],
            layers: vec![
                TileLayer { name: "Background".into(), visible: true, opacity: 1.0, locked: false },
                TileLayer { name: "Foreground".into(), visible: true, opacity: 1.0, locked: false },
                TileLayer { name: "Collision".into(), visible: false, opacity: 0.5, locked: true },
            ],
            selected_layer: Some(0), active_brush: BrushType::Paint, brush_size: 1, show_grid: true, show_collision: false, selected_tile: Some((0, 0)),
        }
    }
}

impl EditorPanel for TilemapEditorPanel {
    fn name(&self) -> &str { "Tilemap Editor" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(500.0).default_height(450.0).show(ctx, |ui| {
            ui.heading("Tilemap");
            ui.horizontal(|ui| { ui.label("Tileset:"); ui.text_edit_singleline(&mut self.tileset_name); });
            ui.horizontal(|ui| { ui.label("Tile Size:"); ui.add(egui::DragValue::new(&mut self.tile_size[0]).range(8..=256)); ui.add(egui::DragValue::new(&mut self.tile_size[1]).range(8..=256)); });
            ui.horizontal(|ui| { ui.label("Map Size:"); ui.add(egui::DragValue::new(&mut self.map_size[0]).range(1..=1024)); ui.add(egui::DragValue::new(&mut self.map_size[1]).range(1..=1024)); });
            ui.separator();
            ui.heading("Brush");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_brush, BrushType::Paint, "Paint");
                ui.selectable_value(&mut self.active_brush, BrushType::Erase, "Erase");
                ui.selectable_value(&mut self.active_brush, BrushType::Fill, "Fill");
                ui.selectable_value(&mut self.active_brush, BrushType::Rectangle, "Rect");
                ui.selectable_value(&mut self.active_brush, BrushType::Line, "Line");
                ui.selectable_value(&mut self.active_brush, BrushType::Bucket, "Bucket");
            });
            ui.horizontal(|ui| { ui.label("Brush Size:"); ui.add(egui::DragValue::new(&mut self.brush_size).range(1..=20)); });
            ui.separator();
            ui.checkbox(&mut self.show_grid, "Show Grid");
            ui.checkbox(&mut self.show_collision, "Show Collision");
            ui.separator();
            ui.heading("Layers");
            for i in 0..self.layers.len() {
                let selected = self.selected_layer == Some(i);
                ui.horizontal(|ui| {
                    if ui.selectable_label(selected, &self.layers[i].name).clicked() { self.selected_layer = Some(i); }
                    ui.checkbox(&mut self.layers[i].visible, "Vis");
                    ui.checkbox(&mut self.layers[i].locked, "Lock");
                    ui.add(egui::Slider::new(&mut self.layers[i].opacity, 0.0..=1.0));
                });
            }
            ui.separator();
            ui.horizontal(|ui| { if ui.button("Add Layer").clicked() { self.layers.push(TileLayer { name: format!("Layer {}", self.layers.len() + 1), visible: true, opacity: 1.0, locked: false }); } if ui.button("Remove Layer").clicked() { if let Some(idx) = self.selected_layer { if idx < self.layers.len() && self.layers.len() > 1 { self.layers.remove(idx); self.selected_layer = None; } } } });
            ui.separator();
            ui.label("Selected Tile:");
            if let Some((tx, ty)) = self.selected_tile { ui.label(format!("({}, {})", tx, ty)); }
        });
    }
}
