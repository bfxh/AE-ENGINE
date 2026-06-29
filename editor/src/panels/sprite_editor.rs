//! Sprite editor panel: slice, 9-slice, bones, animation.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct SpriteEditorPanel {
    pub visible: bool,
    pub sprite_name: String,
    pub texture_size: [u32; 2],
    pub slices: Vec<SpriteSlice>,
    pub selected_slice: Option<usize>,
    pub nine_slice: NineSlice,
    pub use_nine_slice: bool,
    pub bones: Vec<SpriteBone>,
    pub animations: Vec<SpriteAnimation>,
    pub show_slices: bool,
    pub show_bones: bool,
    pub new_slice_name: String,
}

#[derive(Debug, Clone)]
pub struct SpriteSlice { pub name: String, pub x: u32, pub y: u32, pub w: u32, pub h: u32 }

#[derive(Debug, Clone)]
pub struct NineSlice { pub left: u32, pub right: u32, pub top: u32, pub bottom: u32 }

#[derive(Debug, Clone)]
pub struct SpriteBone { pub name: String, pub parent: i32, pub pos: [f32; 2], pub rot: f32 }

#[derive(Debug, Clone)]
pub struct SpriteAnimation { pub name: String, pub fps: f32, pub looped: bool, pub frames: u32 }

impl Default for SpriteEditorPanel {
    fn default() -> Self {
        Self {
            visible: false, sprite_name: "character.png".into(), texture_size: [512, 512],
            slices: vec![
                SpriteSlice { name: "idle_0".into(), x: 0, y: 0, w: 64, h: 64 },
                SpriteSlice { name: "idle_1".into(), x: 64, y: 0, w: 64, h: 64 },
                SpriteSlice { name: "walk_0".into(), x: 0, y: 64, w: 64, h: 64 },
            ],
            selected_slice: Some(0),
            nine_slice: NineSlice { left: 16, right: 16, top: 16, bottom: 16 }, use_nine_slice: false,
            bones: vec![SpriteBone { name: "root".into(), parent: -1, pos: [32.0, 32.0], rot: 0.0 }],
            animations: vec![SpriteAnimation { name: "idle".into(), fps: 8.0, looped: true, frames: 2 }],
            show_slices: true, show_bones: false, new_slice_name: String::new(),
        }
    }
}

impl EditorPanel for SpriteEditorPanel {
    fn name(&self) -> &str { "Sprite Editor" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(500.0).default_height(450.0).show(ctx, |ui| {
            ui.horizontal(|ui| { ui.label("Sprite:"); ui.label(&self.sprite_name); ui.label(format!("{}x{}", self.texture_size[0], self.texture_size[1])); });
            ui.separator();
            ui.checkbox(&mut self.show_slices, "Show Slices");
            ui.checkbox(&mut self.show_bones, "Show Bones");
            ui.checkbox(&mut self.use_nine_slice, "9-Slice");
            ui.separator();
            if self.use_nine_slice {
                ui.heading("9-Slice");
                ui.horizontal(|ui| { ui.label("Left:"); ui.add(egui::DragValue::new(&mut self.nine_slice.left).range(0..=256)); ui.label("Right:"); ui.add(egui::DragValue::new(&mut self.nine_slice.right).range(0..=256)); });
                ui.horizontal(|ui| { ui.label("Top:"); ui.add(egui::DragValue::new(&mut self.nine_slice.top).range(0..=256)); ui.label("Bottom:"); ui.add(egui::DragValue::new(&mut self.nine_slice.bottom).range(0..=256)); });
                ui.separator();
            }
            ui.heading("Slices");
            egui::ScrollArea::vertical().show(ui, |ui| {
                for i in 0..self.slices.len() {
                    let selected = self.selected_slice == Some(i);
                    ui.horizontal(|ui| {
                        if ui.selectable_label(selected, &self.slices[i].name).clicked() { self.selected_slice = Some(i); }
                        ui.label(format!("{}x{} @ {},{}", self.slices[i].w, self.slices[i].h, self.slices[i].x, self.slices[i].y));
                    });
                }
            });
            ui.separator();
            if let Some(idx) = self.selected_slice { if idx < self.slices.len() {
                ui.heading("Slice Properties");
                ui.horizontal(|ui| { ui.label("X:"); ui.add(egui::DragValue::new(&mut self.slices[idx].x).range(0..=4096)); ui.label("Y:"); ui.add(egui::DragValue::new(&mut self.slices[idx].y).range(0..=4096)); });
                ui.horizontal(|ui| { ui.label("W:"); ui.add(egui::DragValue::new(&mut self.slices[idx].w).range(1..=4096)); ui.label("H:"); ui.add(egui::DragValue::new(&mut self.slices[idx].h).range(1..=4096)); });
            }}
            ui.separator();
            ui.heading("Bones");
            for i in 0..self.bones.len() { ui.horizontal(|ui| { ui.label(&self.bones[i].name); ui.label(format!("parent:{}", self.bones[i].parent)); }); }
            ui.separator();
            ui.heading("Animations");
            for i in 0..self.animations.len() { ui.horizontal(|ui| { ui.label(&self.animations[i].name); ui.label(format!("{}fps", self.animations[i].fps)); ui.checkbox(&mut self.animations[i].looped, "Loop"); ui.label(format!("{} frames", self.animations[i].frames)); }); }
        });
    }
}
