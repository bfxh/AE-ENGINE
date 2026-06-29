//! 插件浏览器面板：已安装插件列表、启用/禁用、版本、搜索、安装。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct PluginBrowserPanel {
    pub visible: bool,
    pub plugins: Vec<Plugin>,
    pub search_query: String,
    pub selected_plugin: Option<usize>,
    pub filter_enabled_only: bool,
    pub auto_update: bool,
    pub install_path: String,
}

#[derive(Debug, Clone)]
pub struct Plugin {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub enabled: bool,
    pub installed: bool,
    pub size_mb: f32,
}

impl Default for PluginBrowserPanel {
    fn default() -> Self {
        Self {
            visible: false,
            plugins: vec![
                Plugin { name: "RustScript".into(), version: "1.2.0".into(), author: "WastelandTeam".into(), description: "Scripting runtime".into(), enabled: true, installed: true, size_mb: 12.5 },
                Plugin { name: "ProBuilder".into(), version: "0.8.3".into(), author: "MeshTools".into(), description: "Brush-based modeling".into(), enabled: false, installed: true, size_mb: 8.2 },
                Plugin { name: "NoiseGen".into(), version: "2.0.1".into(), author: "Procedural".into(), description: "Procedural noise generator".into(), enabled: true, installed: true, size_mb: 3.4 },
                Plugin { name: "PathTracer".into(), version: "0.1.0".into(), author: "RenderLab".into(), description: "Offline path tracer".into(), enabled: false, installed: false, size_mb: 45.0 },
            ],
            search_query: String::new(),
            selected_plugin: Some(0),
            filter_enabled_only: false,
            auto_update: true,
            install_path: "plugins/".into(),
        }
    }
}

impl EditorPanel for PluginBrowserPanel {
    fn name(&self) -> &str { "Plugin Browser" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(500.0)
            .default_height(400.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.search_query);
                    ui.checkbox(&mut self.filter_enabled_only, "Enabled only");
                    ui.checkbox(&mut self.auto_update, "Auto-update");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Install path:");
                    ui.text_edit_singleline(&mut self.install_path);
                });
                ui.separator();
                ui.heading("Installed Plugins");
                ui.separator();
                let mut action: Option<(usize, bool)> = None;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for i in 0..self.plugins.len() {
                        let matches_search = self.search_query.is_empty()
                            || self.plugins[i].name.to_lowercase().contains(&self.search_query.to_lowercase());
                        let matches_filter = !self.filter_enabled_only || self.plugins[i].enabled;
                        if !matches_search || !matches_filter { continue; }
                        let selected = self.selected_plugin == Some(i);
                        ui.horizontal(|ui| {
                            if ui.selectable_label(selected, &self.plugins[i].name).clicked() {
                                self.selected_plugin = Some(i);
                            }
                            ui.label(format!("v{}", self.plugins[i].version));
                            ui.label(format!("- {}", self.plugins[i].author));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if self.plugins[i].installed {
                                    let label = if self.plugins[i].enabled { "Disable" } else { "Enable" };
                                    if ui.button(label).clicked() {
                                        action = Some((i, !self.plugins[i].enabled));
                                    }
                                } else {
                                    if ui.button("Install").clicked() {
                                        self.plugins[i].installed = true;
                                        self.plugins[i].enabled = true;
                                    }
                                }
                            });
                        });
                        ui.label(format!("  {}", self.plugins[i].description));
                        ui.label(format!("  Size: {:.1} MB", self.plugins[i].size_mb));
                        ui.separator();
                    }
                });
                if let Some((idx, new_enabled)) = action {
                    if idx < self.plugins.len() {
                        self.plugins[idx].enabled = new_enabled;
                    }
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Refresh").clicked() {}
                    if ui.button("Install from File...").clicked() {}
                    if ui.button("Uninstall Selected").clicked() {
                        if let Some(idx) = self.selected_plugin {
                            if idx < self.plugins.len() && self.plugins[idx].installed {
                                self.plugins[idx].installed = false;
                                self.plugins[idx].enabled = false;
                            }
                        }
                    }
                });
            });
    }
}
