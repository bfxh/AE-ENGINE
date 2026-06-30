//! Asset Browser panel — file browser for managing project assets.
//!
//! Provides a bottom panel that lists files in a selected project directory.
//! Supports opening a directory via `rfd` file dialog, search filter, and view modes.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

/// View mode for the asset browser.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewMode {
    List,
    Grid,
}

/// File type filter.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileTypeFilter {
    All,
    Scenes,
    Models,
    Textures,
    Audio,
    Scripts,
    Other,
}

impl FileTypeFilter {
    fn label(&self) -> &'static str {
        match self {
            FileTypeFilter::All => "All",
            FileTypeFilter::Scenes => "Scenes",
            FileTypeFilter::Models => "Models",
            FileTypeFilter::Textures => "Textures",
            FileTypeFilter::Audio => "Audio",
            FileTypeFilter::Scripts => "Scripts",
            FileTypeFilter::Other => "Other",
        }
    }

    fn matches(&self, ext: &str) -> bool {
        match self {
            FileTypeFilter::All => true,
            FileTypeFilter::Scenes => matches!(ext, "ae" | "json" | "scene"),
            FileTypeFilter::Models => matches!(ext, "gltf" | "glb" | "obj" | "fbx" | "mesh"),
            FileTypeFilter::Textures => matches!(ext, "png" | "jpg" | "jpeg" | "bmp" | "tga" | "hdr" | "exr"),
            FileTypeFilter::Audio => matches!(ext, "wav" | "mp3" | "ogg" | "flac"),
            FileTypeFilter::Scripts => matches!(ext, "rs" | "py" | "lua" | "js" | "ts"),
            FileTypeFilter::Other => !matches!(ext, "ae" | "json" | "scene" | "gltf" | "glb" | "obj" | "fbx" | "mesh" | "png" | "jpg" | "jpeg" | "bmp" | "tga" | "hdr" | "exr" | "wav" | "mp3" | "ogg" | "flac" | "rs" | "py" | "lua" | "js" | "ts"),
        }
    }
}

/// Asset Browser panel state.
pub struct AssetBrowserPanel {
    pub visible: bool,
    pub current_dir: Option<std::path::PathBuf>,
    pub entries: Vec<AssetEntry>,
    pub filter_text: String,
    pub view_mode: ViewMode,
    pub file_type_filter: FileTypeFilter,
    pub show_hidden: bool,
    pub selected_entry: Option<usize>,
}

impl Default for AssetBrowserPanel {
    fn default() -> Self {
        Self {
            visible: true,
            current_dir: None,
            entries: Vec::new(),
            filter_text: String::new(),
            view_mode: ViewMode::List,
            file_type_filter: FileTypeFilter::All,
            show_hidden: false,
            selected_entry: None,
        }
    }
}

/// A file or directory entry in the asset browser.
#[derive(Debug, Clone)]
pub struct AssetEntry {
    pub name: String,
    pub path: std::path::PathBuf,
    pub is_dir: bool,
    pub extension: String,
    pub size: u64,
}

impl AssetEntry {
    fn icon(&self) -> &'static str {
        if self.is_dir {
            return "[D]";
        }
        match self.extension.as_str() {
            "ae" | "json" | "scene" => "[S]",
            "gltf" | "glb" | "obj" | "fbx" | "mesh" => "[M]",
            "png" | "jpg" | "jpeg" | "bmp" | "tga" | "hdr" | "exr" => "[T]",
            "wav" | "mp3" | "ogg" | "flac" => "[A]",
            "rs" | "py" | "lua" | "js" | "ts" => "[C]",
            _ => "[F]",
        }
    }

    fn color(&self) -> egui::Color32 {
        if self.is_dir {
            return egui::Color32::from_rgb(255, 220, 100);
        }
        match self.extension.as_str() {
            "ae" | "json" | "scene" => egui::Color32::from_rgb(180, 220, 255),
            "gltf" | "glb" | "obj" | "fbx" | "mesh" => egui::Color32::from_rgb(100, 200, 255),
            "png" | "jpg" | "jpeg" | "bmp" | "tga" | "hdr" | "exr" => egui::Color32::from_rgb(255, 180, 220),
            "wav" | "mp3" | "ogg" | "flac" => egui::Color32::from_rgb(200, 255, 150),
            "rs" | "py" | "lua" | "js" | "ts" => egui::Color32::from_rgb(255, 200, 120),
            _ => egui::Color32::from_rgb(180, 180, 180),
        }
    }

    fn size_str(&self) -> String {
        if self.is_dir {
            return "-".to_string();
        }
        if self.size < 1024 {
            format!("{} B", self.size)
        } else if self.size < 1024 * 1024 {
            format!("{:.1} KB", self.size as f64 / 1024.0)
        } else if self.size < 1024 * 1024 * 1024 {
            format!("{:.1} MB", self.size as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", self.size as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

impl AssetBrowserPanel {
    fn refresh_entries(&mut self) {
        self.entries.clear();
        let dir = match &self.current_dir {
            Some(d) => d.clone(),
            None => return,
        };

        let entries = match std::fs::read_dir(&dir) {
            Ok(iter) => iter,
            Err(e) => {
                log::warn!("Cannot read directory {:?}: {}", dir, e);
                return;
            },
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "???".to_string());

            // Skip hidden files unless enabled.
            if !self.show_hidden && name.starts_with('.') {
                continue;
            }

            let is_dir = path.is_dir();
            let extension = path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

            self.entries.push(AssetEntry { name, path, is_dir, extension, size });
        }

        self.entries.sort_by(|a, b| {
            b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
    }

    fn navigate_to(&mut self, path: std::path::PathBuf) {
        self.current_dir = Some(path);
        self.refresh_entries();
        self.selected_entry = None;
    }

    fn go_up(&mut self) {
        if let Some(ref dir) = self.current_dir {
            if let Some(parent) = dir.parent() {
                self.navigate_to(parent.to_path_buf());
            }
        }
    }
}

impl EditorPanel for AssetBrowserPanel {
    fn name(&self) -> &str { "Asset Browser" }
    fn visible(&self) -> bool { self.visible }
    fn render(&mut self, _ctx: &egui::Context, _app: &mut EditorApp) {}
}

/// Render the asset browser as a bottom side panel.
pub fn render_asset_browser_panel(
    ctx: &egui::Context,
    _app: &mut EditorApp,
    panel: &mut AssetBrowserPanel,
) {
    if !panel.visible { return; }

    egui::TopBottomPanel::bottom("asset_browser_panel").resizable(true).default_height(200.0).show(
        ctx,
        |ui| {
            // Toolbar.
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Asset Browser").strong());

                ui.separator();

                // Navigation buttons.
                if ui.button("Up").clicked() {
                    panel.go_up();
                }
                if ui.button("Open Folder").clicked() {
                    let dir = rfd::FileDialog::new().pick_folder();
                    if let Some(path) = dir {
                        panel.navigate_to(path);
                    }
                }
                if panel.current_dir.is_some() && ui.button("Refresh").clicked() {
                    panel.refresh_entries();
                }

                ui.separator();

                // View mode toggle.
                ui.label(egui::RichText::new("View:").small().color(egui::Color32::from_rgb(150, 150, 160)));
                if ui.selectable_label(panel.view_mode == ViewMode::List, "List").clicked() {
                    panel.view_mode = ViewMode::List;
                }
                if ui.selectable_label(panel.view_mode == ViewMode::Grid, "Grid").clicked() {
                    panel.view_mode = ViewMode::Grid;
                }

                ui.separator();

                // File type filter.
                ui.label(egui::RichText::new("Type:").small().color(egui::Color32::from_rgb(150, 150, 160)));
                let filters = [FileTypeFilter::All, FileTypeFilter::Scenes, FileTypeFilter::Models, FileTypeFilter::Textures, FileTypeFilter::Audio, FileTypeFilter::Scripts, FileTypeFilter::Other];
                for filter in filters {
                    if ui.selectable_label(panel.file_type_filter == filter, filter.label()).clicked() {
                        panel.file_type_filter = filter;
                    }
                }

                ui.separator();

                // Search filter.
                ui.label(egui::RichText::new("Filter:").small().color(egui::Color32::from_rgb(150, 150, 160)));
                ui.add(egui::TextEdit::singleline(&mut panel.filter_text).hint_text("Search files...").desired_width(120.0));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.checkbox(&mut panel.show_hidden, "Hidden");
                    if ui.button("x").clicked() {
                        panel.visible = false;
                    }
                });
            });

            ui.separator();

            // Current path breadcrumb.
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Path:").small().color(egui::Color32::from_rgb(150, 150, 160)));
                let path_str = match &panel.current_dir {
                    Some(p) => p.display().to_string(),
                    None => "(No folder selected)".to_string(),
                };
                ui.label(egui::RichText::new(&path_str).color(egui::Color32::from_rgb(200, 220, 255)).family(egui::FontFamily::Monospace).small());
            });

            ui.separator();

            // File listing.
            let filter = panel.filter_text.to_lowercase();
            let file_filter = panel.file_type_filter;
            let view_mode = panel.view_mode;

            let filtered_indices: Vec<usize> = panel.entries.iter().enumerate().filter(|(_, e)| {
                if !filter.is_empty() && !e.name.to_lowercase().contains(&filter) {
                    return false;
                }
                if !e.is_dir && !file_filter.matches(&e.extension) {
                    return false;
                }
                true
            }).map(|(i, _)| i).collect();

            let entry_count = filtered_indices.len();

            egui::ScrollArea::vertical().show(ui, |ui| {
                if panel.entries.is_empty() {
                    if panel.current_dir.is_some() {
                        ui.label(egui::RichText::new("(empty directory)").color(egui::Color32::from_rgb(120, 120, 120)));
                    } else {
                        ui.label(egui::RichText::new("Click 'Open Folder' to browse assets").color(egui::Color32::from_rgb(120, 120, 120)));
                    }
                } else if entry_count == 0 {
                    ui.label(egui::RichText::new("No files match filter").color(egui::Color32::from_rgb(120, 120, 120)));
                } else {
                    match view_mode {
                        ViewMode::List => {
                            egui::Grid::new("asset_list_grid").striped(true).num_columns(3).spacing([8.0, 2.0]).min_col_width(80.0).show(ui, |ui| {
                                // Header.
                                ui.label(egui::RichText::new("Name").strong().color(egui::Color32::from_rgb(180, 180, 190)));
                                ui.label(egui::RichText::new("Size").strong().color(egui::Color32::from_rgb(180, 180, 190)));
                                ui.label(egui::RichText::new("Type").strong().color(egui::Color32::from_rgb(180, 180, 190)));
                                ui.end_row();

                                for idx in &filtered_indices {
                                    let entry = &panel.entries[*idx];
                                    let is_selected = panel.selected_entry == Some(*idx);

                                    let icon = entry.icon();
                                    let color = entry.color();
                                    let label = format!("{} {}", icon, entry.name);

                                    let response = ui.selectable_label(is_selected, egui::RichText::new(&label).color(color));
                                    ui.label(egui::RichText::new(entry.size_str()).small().color(egui::Color32::from_rgb(140, 140, 140)).family(egui::FontFamily::Monospace));
                                    ui.label(egui::RichText::new(if entry.is_dir { "Directory" } else { &entry.extension }).small().color(egui::Color32::from_rgb(140, 140, 140)));

                                    if response.clicked() {
                                        panel.selected_entry = Some(*idx);
                                    }
                                    if response.double_clicked() {
                                        if entry.is_dir {
                                            panel.navigate_to(entry.path.clone());
                                        }
                                    }
                                    ui.end_row();
                                }
                            });
                        },
                        ViewMode::Grid => {
                            let available_width = ui.available_width();
                            let item_width = 100.0;
                            let cols = (available_width / item_width).max(1.0) as usize;

                            let mut col = 0usize;
                            ui.horizontal_wrapped(|ui| {
                                for idx in &filtered_indices {
                                    let entry = &panel.entries[*idx];
                                    let is_selected = panel.selected_entry == Some(*idx);

                                    let icon = entry.icon();
                                    let color = entry.color();

                                    let frame = egui::Frame::group(ui.style())
                                        .fill(if is_selected {
                                            egui::Color32::from_rgba_premultiplied(80, 120, 200, 80)
                                        } else {
                                            egui::Color32::from_rgba_premultiplied(30, 32, 38, 160)
                                        })
                                        .stroke(if is_selected {
                                            egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 160, 255))
                                        } else {
                                            egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 70))
                                        })
                                        .inner_margin(egui::Margin::symmetric(4, 4))
                                        .corner_radius(3);

                                    let response = frame.show(ui, |ui| {
                                        ui.set_min_width(80.0);
                                        ui.vertical_centered(|ui| {
                                            ui.add_space(4.0);
                                            ui.label(egui::RichText::new(icon).color(color).size(20.0));
                                            ui.add_space(2.0);
                                            ui.label(egui::RichText::new(&entry.name).small().color(egui::Color32::from_rgb(200, 200, 210)));
                                            if !entry.is_dir {
                                                ui.label(egui::RichText::new(entry.size_str()).small().color(egui::Color32::from_rgb(120, 120, 120)));
                                            }
                                        });
                                    }).response;

                                    if response.clicked() {
                                        panel.selected_entry = Some(*idx);
                                    }
                                    if response.double_clicked() {
                                        if entry.is_dir {
                                            panel.navigate_to(entry.path.clone());
                                        }
                                    }

                                    col += 1;
                                    if col >= cols {
                                        ui.end_row();
                                        col = 0;
                                    }
                                }
                            });
                        },
                    }
                }
            });

            // Status bar.
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{} items ({} shown)", panel.entries.len(), entry_count)).small().color(egui::Color32::from_rgb(120, 120, 120)));
                if let Some(idx) = panel.selected_entry {
                    if let Some(entry) = panel.entries.get(idx) {
                        ui.separator();
                        ui.label(egui::RichText::new(format!("Selected: {}", entry.name)).small().color(egui::Color32::from_rgb(200, 220, 255)));
                    }
                }
            });
        },
    );
}
