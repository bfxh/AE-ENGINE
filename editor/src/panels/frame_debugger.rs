//! Frame debugger panel: draw call list, state, shaders, textures.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct FrameDebuggerPanel {
    pub visible: bool,
    pub draw_calls: Vec<DrawCall>,
    pub selected_call: Option<usize>,
    pub paused: bool,
    pub frame_number: u64,
    pub total_draw_calls: u32,
    pub total_vertices: u32,
    pub total_triangles: u32,
    pub show_only_visible: bool,
}

#[derive(Debug, Clone)]
pub struct DrawCall { pub id: u32, pub mesh: String, pub material: String, pub shader: String, pub vertices: u32, pub triangles: u32, pub visible: bool, pub sort_key: u64 }

impl Default for FrameDebuggerPanel {
    fn default() -> Self {
        Self {
            visible: false,
            draw_calls: vec![
                DrawCall { id: 1, mesh: "terrain".into(), material: "ground".into(), shader: "PBR".into(), vertices: 50000, triangles: 16000, visible: true, sort_key: 100 },
                DrawCall { id: 2, mesh: "player".into(), material: "skin".into(), shader: "Skin".into(), vertices: 8000, triangles: 3000, visible: true, sort_key: 200 },
                DrawCall { id: 3, mesh: "tree_01".into(), material: "bark".into(), shader: "PBR".into(), vertices: 2000, triangles: 800, visible: true, sort_key: 150 },
                DrawCall { id: 4, mesh: "skybox".into(), material: "sky".into(), shader: "Sky".into(), vertices: 24, triangles: 12, visible: true, sort_key: 0 },
            ],
            selected_call: Some(0), paused: false, frame_number: 0, total_draw_calls: 4, total_vertices: 60024, total_triangles: 19812, show_only_visible: false,
        }
    }
}

impl EditorPanel for FrameDebuggerPanel {
    fn name(&self) -> &str { "Frame Debugger" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(600.0).default_height(450.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(if self.paused { "Resume" } else { "Pause" }).clicked() { self.paused = !self.paused; }
                ui.label(format!("Frame: {}", self.frame_number));
                ui.checkbox(&mut self.show_only_visible, "Visible Only");
            });
            ui.separator();
            ui.horizontal(|ui| { ui.label(format!("Draw Calls: {}", self.total_draw_calls)); ui.label(format!("Vertices: {}", self.total_vertices)); ui.label(format!("Triangles: {}", self.total_triangles)); });
            ui.separator();
            ui.heading("Draw Calls");
            egui::ScrollArea::vertical().show(ui, |ui| {
                for i in 0..self.draw_calls.len() {
                    if self.show_only_visible && !self.draw_calls[i].visible { continue; }
                    let selected = self.selected_call == Some(i);
                    ui.horizontal(|ui| {
                        if ui.selectable_label(selected, format!("#{} {}", self.draw_calls[i].id, self.draw_calls[i].mesh)).clicked() { self.selected_call = Some(i); }
                        ui.label(&self.draw_calls[i].material);
                        ui.label(&self.draw_calls[i].shader);
                        ui.label(format!("{} tris", self.draw_calls[i].triangles));
                    });
                }
            });
            ui.separator();
            if let Some(idx) = self.selected_call { if idx < self.draw_calls.len() {
                ui.heading("Draw Call Details");
                ui.label(format!("ID: {}", self.draw_calls[idx].id));
                ui.label(format!("Mesh: {}", self.draw_calls[idx].mesh));
                ui.label(format!("Material: {}", self.draw_calls[idx].material));
                ui.label(format!("Shader: {}", self.draw_calls[idx].shader));
                ui.label(format!("Vertices: {}", self.draw_calls[idx].vertices));
                ui.label(format!("Triangles: {}", self.draw_calls[idx].triangles));
                ui.label(format!("Sort Key: {}", self.draw_calls[idx].sort_key));
                ui.checkbox(&mut self.draw_calls[idx].visible, "Visible");
            }}
        });
    }
}
