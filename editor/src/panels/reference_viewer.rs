//! 引用查看器面板：显示资产依赖关系。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct ReferenceViewerPanel {
    pub visible: bool,
    pub current_asset: String,
    pub references: Vec<RefEntry>,
    pub dependents: Vec<RefEntry>,
    pub search_filter: String,
    pub ref_type_filter: RefType,
    pub show_size: bool,
    pub show_path: bool,
    pub graph_depth: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RefType {
    All,
    Texture,
    Mesh,
    Material,
    Sound,
}

#[derive(Debug, Clone)]
pub struct RefEntry {
    pub path: String,
    pub ref_type: RefType,
    pub size_kb: f32,
    pub reference_count: u32,
}

impl Default for ReferenceViewerPanel {
    fn default() -> Self {
        Self {
            visible: false,
            current_asset: "/game/props/cube".to_string(),
            references: vec![
                RefEntry { path: "/game/textures/cube_diff".into(), ref_type: RefType::Texture, size_kb: 1024.0, reference_count: 1 },
                RefEntry { path: "/game/materials/cube_mat".into(), ref_type: RefType::Material, size_kb: 16.0, reference_count: 3 },
                RefEntry { path: "/game/meshes/cube_mesh".into(), ref_type: RefType::Mesh, size_kb: 256.0, reference_count: 1 },
            ],
            dependents: vec![
                RefEntry { path: "/game/levels/test".into(), ref_type: RefType::All, size_kb: 8192.0, reference_count: 2 },
                RefEntry { path: "/game/blueprints/item".into(), ref_type: RefType::All, size_kb: 64.0, reference_count: 1 },
            ],
            search_filter: String::new(),
            ref_type_filter: RefType::All,
            show_size: true,
            show_path: true,
            graph_depth: 3,
        }
    }
}

impl EditorPanel for ReferenceViewerPanel {
    fn name(&self) -> &str { "Reference Viewer" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(700.0)
            .default_height(500.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Asset:");
                    ui.text_edit_singleline(&mut self.current_asset);
                    if ui.button("Refresh").clicked() {}
                    ui.separator();
                    ui.text_edit_singleline(&mut self.search_filter);
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Filter:");
                    ui.radio_value(&mut self.ref_type_filter, RefType::All, "All");
                    ui.radio_value(&mut self.ref_type_filter, RefType::Texture, "Texture");
                    ui.radio_value(&mut self.ref_type_filter, RefType::Mesh, "Mesh");
                    ui.radio_value(&mut self.ref_type_filter, RefType::Material, "Material");
                    ui.radio_value(&mut self.ref_type_filter, RefType::Sound, "Sound");
                    ui.separator();
                    ui.checkbox(&mut self.show_size, "Size");
                    ui.checkbox(&mut self.show_path, "Path");
                    ui.label("Depth:");
                    ui.add(egui::DragValue::new(&mut self.graph_depth).range(1..=10));
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading("References (depends on)");
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for r in &self.references {
                                if self.ref_type_filter != RefType::All && r.ref_type != self.ref_type_filter { continue; }
                                if !self.search_filter.is_empty() && !r.path.contains(&self.search_filter) { continue; }
                                ui.horizontal(|ui| {
                                    ui.label(&r.path);
                                    if self.show_size { ui.label(format!("{:.1}KB", r.size_kb)); }
                                    ui.label(format!("x{}", r.reference_count));
                                });
                            }
                        });
                    });
                    ui.vertical(|ui| {
                        ui.heading("Dependents (referenced by)");
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for r in &self.dependents {
                                if !self.search_filter.is_empty() && !r.path.contains(&self.search_filter) { continue; }
                                ui.horizontal(|ui| {
                                    ui.label(&r.path);
                                    if self.show_size { ui.label(format!("{:.1}KB", r.size_kb)); }
                                    ui.label(format!("x{}", r.reference_count));
                                });
                            }
                        });
                    });
                });
                ui.separator();
                ui.label("Dependency Graph:");
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    ui.set_min_size(egui::vec2(600.0, 150.0));
                    ui.label("(Graph visualization - nodes and edges)");
                });
            });
    }
}
