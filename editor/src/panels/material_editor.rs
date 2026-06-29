//! 材质编辑器面板：PBR 材质属性、纹理槽位、预设与实时预览。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

/// 渲染模式。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderMode {
    Opaque,
    Masked,
    Translucent,
    Additive,
}

impl RenderMode {
    fn label(&self) -> &'static str {
        match self {
            RenderMode::Opaque => "Opaque",
            RenderMode::Masked => "Masked",
            RenderMode::Translucent => "Translucent",
            RenderMode::Additive => "Additive",
        }
    }
}

/// 材质预设。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MaterialPreset {
    Metal,
    Plastic,
    Wood,
    Stone,
    Glass,
    Emissive,
}

impl MaterialPreset {
    fn label(&self) -> &'static str {
        match self {
            MaterialPreset::Metal => "Metal",
            MaterialPreset::Plastic => "Plastic",
            MaterialPreset::Wood => "Wood",
            MaterialPreset::Stone => "Stone",
            MaterialPreset::Glass => "Glass",
            MaterialPreset::Emissive => "Emissive",
        }
    }
}

/// 纹理槽位类型。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextureSlot {
    Albedo,
    Normal,
    ORM,
    Emissive,
    Roughness,
}

impl TextureSlot {
    fn label(&self) -> &'static str {
        match self {
            TextureSlot::Albedo => "Albedo",
            TextureSlot::Normal => "Normal",
            TextureSlot::ORM => "ORM",
            TextureSlot::Emissive => "Emissive",
            TextureSlot::Roughness => "Roughness",
        }
    }
}

pub struct MaterialEditorPanel {
    pub visible: bool,
    pub material_name: String,
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
    pub emissive_strength: f32,
    pub opacity: f32,
    pub ior: f32,
    pub subsurface: f32,
    pub anisotropy: f32,
    pub render_mode: RenderMode,
    pub double_sided: bool,
    pub wireframe: bool,
    pub use_vertex_colors: bool,
    pub texture_paths: [String; 5],
    pub texture_enabled: [bool; 5],
    pub uv_tiling: [f32; 2],
    pub uv_offset: [f32; 2],
    pub preview_size: f32,
    pub preview_background: bool,
    pub current_preset: MaterialPreset,
    pub status_message: String,
    pub dirty: bool,
}

impl Default for MaterialEditorPanel {
    fn default() -> Self {
        Self {
            visible: false,
            material_name: "NewMaterial".to_string(),
            base_color: [0.8, 0.8, 0.8, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            emissive: [0.0, 0.0, 0.0],
            emissive_strength: 0.0,
            opacity: 1.0,
            ior: 1.5,
            subsurface: 0.0,
            anisotropy: 0.0,
            render_mode: RenderMode::Opaque,
            double_sided: false,
            wireframe: false,
            use_vertex_colors: false,
            texture_paths: [
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            texture_enabled: [false; 5],
            uv_tiling: [1.0, 1.0],
            uv_offset: [0.0, 0.0],
            preview_size: 128.0,
            preview_background: true,
            current_preset: MaterialPreset::Plastic,
            status_message: "Ready".to_string(),
            dirty: false,
        }
    }
}

impl MaterialEditorPanel {
    fn apply_preset(&mut self, preset: MaterialPreset) {
        self.current_preset = preset;
        match preset {
            MaterialPreset::Metal => {
                self.base_color = [0.75, 0.75, 0.78, 1.0];
                self.metallic = 1.0;
                self.roughness = 0.25;
                self.emissive = [0.0, 0.0, 0.0];
                self.emissive_strength = 0.0;
                self.opacity = 1.0;
                self.render_mode = RenderMode::Opaque;
            }
            MaterialPreset::Plastic => {
                self.base_color = [0.7, 0.7, 0.7, 1.0];
                self.metallic = 0.0;
                self.roughness = 0.45;
                self.emissive = [0.0, 0.0, 0.0];
                self.emissive_strength = 0.0;
                self.opacity = 1.0;
                self.render_mode = RenderMode::Opaque;
            }
            MaterialPreset::Wood => {
                self.base_color = [0.45, 0.28, 0.12, 1.0];
                self.metallic = 0.0;
                self.roughness = 0.75;
                self.emissive = [0.0, 0.0, 0.0];
                self.emissive_strength = 0.0;
                self.opacity = 1.0;
                self.render_mode = RenderMode::Opaque;
            }
            MaterialPreset::Stone => {
                self.base_color = [0.55, 0.55, 0.52, 1.0];
                self.metallic = 0.0;
                self.roughness = 0.9;
                self.emissive = [0.0, 0.0, 0.0];
                self.emissive_strength = 0.0;
                self.opacity = 1.0;
                self.render_mode = RenderMode::Opaque;
            }
            MaterialPreset::Glass => {
                self.base_color = [0.9, 0.95, 1.0, 0.3];
                self.metallic = 0.0;
                self.roughness = 0.05;
                self.emissive = [0.0, 0.0, 0.0];
                self.emissive_strength = 0.0;
                self.opacity = 0.3;
                self.render_mode = RenderMode::Translucent;
                self.ior = 1.52;
            }
            MaterialPreset::Emissive => {
                self.base_color = [0.1, 0.1, 0.1, 1.0];
                self.metallic = 0.0;
                self.roughness = 0.5;
                self.emissive = [1.0, 0.85, 0.4];
                self.emissive_strength = 3.0;
                self.opacity = 1.0;
                self.render_mode = RenderMode::Opaque;
            }
        }
        self.dirty = true;
        self.status_message = format!("Preset applied: {}", preset.label());
    }

    fn render_preview(&self, ui: &mut egui::Ui) {
        let size = self.preview_size;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
        let painter = ui.painter();
        if self.preview_background {
            painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(30, 30, 34));
        }
        let center = rect.center();
        let radius = size * 0.42;
        let bc = self.base_color;
        let base = egui::Color32::from_rgba_premultiplied(
            (bc[0] * 255.0).clamp(0.0, 255.0) as u8,
            (bc[1] * 255.0).clamp(0.0, 255.0) as u8,
            (bc[2] * 255.0).clamp(0.0, 255.0) as u8,
            (bc[3] * 255.0).clamp(0.0, 255.0) as u8,
        );
        let steps = 24;
        for i in 0..steps {
            let t = i as f32 / steps as f32;
            let r = radius * (1.0 - t * 0.95);
            let offset = egui::vec2(-radius * 0.25 * t, -radius * 0.3 * t);
            let highlight = (1.0 - self.roughness) * (1.0 - t) * if self.metallic > 0.5 { 1.5 } else { 1.0 };
            let shade = 0.5 + 0.5 * (1.0 - t);
            let mr = (base.r() as f32 * shade + 255.0 * highlight * self.metallic).clamp(0.0, 255.0) as u8;
            let mg = (base.g() as f32 * shade + 255.0 * highlight * self.metallic).clamp(0.0, 255.0) as u8;
            let mb = (base.b() as f32 * shade + 255.0 * highlight * self.metallic).clamp(0.0, 255.0) as u8;
            let col = egui::Color32::from_rgba_premultiplied(mr, mg, mb, base.a());
            painter.circle_filled(center + offset, r, col);
        }
        if self.roughness < 0.7 {
            let hl_size = radius * 0.18 * (1.0 - self.roughness);
            let hl_pos = center + egui::vec2(-radius * 0.3, -radius * 0.35);
            painter.circle_filled(hl_pos, hl_size, egui::Color32::from_rgba_premultiplied(255, 255, 255, 180));
        }
        if self.emissive_strength > 0.0 {
            let em = self.emissive;
            let intensity = self.emissive_strength.min(1.0);
            let em_color = egui::Color32::from_rgba_premultiplied(
                (em[0] * 255.0 * intensity) as u8,
                (em[1] * 255.0 * intensity) as u8,
                (em[2] * 255.0 * intensity) as u8,
                120,
            );
            painter.circle_filled(center, radius * 1.05, em_color);
        }
        painter.circle_stroke(center, radius, egui::Stroke::new(1.5, egui::Color32::from_rgb(90, 90, 100)));
        if self.wireframe {
            painter.text(
                rect.left_bottom() + egui::vec2(4.0, -14.0),
                egui::Align2::LEFT_BOTTOM,
                "WIRE",
                egui::FontId::proportional(10.0),
                egui::Color32::from_rgb(255, 200, 80),
            );
        }
    }

    fn render_texture_slot(&mut self, ui: &mut egui::Ui, slot: TextureSlot) {
        let idx = slot as usize;
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.texture_enabled[idx], "");
            ui.label(format!("{}:", slot.label()));
            let display: String = if self.texture_paths[idx].is_empty() {
                "(none)".to_string()
            } else {
                self.texture_paths[idx].clone()
            };
            ui.monospace(display.as_str());
            if ui.button("Browse").clicked() {
                self.status_message = format!("Browse for {} texture", slot.label());
            }
            if ui.button("Clear").clicked() {
                self.texture_paths[idx].clear();
                self.texture_enabled[idx] = false;
                self.dirty = true;
                self.status_message = format!("Cleared {} texture", slot.label());
            }
        });
    }
}

impl EditorPanel for MaterialEditorPanel {
    fn name(&self) -> &str { "Material Editor" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(480.0)
            .default_height(680.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Material:");
                    ui.text_edit_singleline(&mut self.material_name);
                    ui.separator();
                    if ui.button("Save").clicked() {
                        self.status_message = format!("Saved '{}'", self.material_name.as_str());
                        self.dirty = false;
                    }
                    if ui.button("Load").clicked() {
                        self.status_message = "Load dialog".to_string();
                    }
                    if ui.button("Reset").clicked() {
                        *self = Self::default();
                        self.visible = true;
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Preview");
                        self.render_preview(ui);
                        ui.horizontal(|ui| {
                            ui.label("Size:");
                            ui.add(egui::Slider::new(&mut self.preview_size, 64.0..=256.0));
                        });
                        ui.checkbox(&mut self.preview_background, "Background");
                    });
                    ui.vertical(|ui| {
                        ui.label("Presets");
                        ui.separator();
                        let presets = [
                            MaterialPreset::Metal,
                            MaterialPreset::Plastic,
                            MaterialPreset::Wood,
                            MaterialPreset::Stone,
                            MaterialPreset::Glass,
                            MaterialPreset::Emissive,
                        ];
                        for preset in presets {
                            let selected = self.current_preset == preset;
                            if ui.selectable_label(selected, preset.label()).clicked() {
                                self.apply_preset(preset);
                            }
                        }
                    });
                });
                ui.separator();
                ui.collapsing("PBR Properties", |ui| {
                    egui::Grid::new("pbr_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                        ui.label("Base Color:");
                        ui.color_edit_button_rgba_premultiplied(&mut self.base_color);
                        ui.end_row();
                        ui.label("Metallic:");
                        ui.add(egui::Slider::new(&mut self.metallic, 0.0..=1.0));
                        ui.end_row();
                        ui.label("Roughness:");
                        ui.add(egui::Slider::new(&mut self.roughness, 0.0..=1.0));
                        ui.end_row();
                        ui.label("Opacity:");
                        ui.add(egui::Slider::new(&mut self.opacity, 0.0..=1.0));
                        ui.end_row();
                    });
                });
                ui.collapsing("Emissive", |ui| {
                    egui::Grid::new("emissive_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                        ui.label("Color:");
                        ui.color_edit_button_rgb(&mut self.emissive);
                        ui.end_row();
                        ui.label("Strength:");
                        ui.add(egui::Slider::new(&mut self.emissive_strength, 0.0..=10.0));
                        ui.end_row();
                    });
                });
                ui.collapsing("Advanced Parameters", |ui| {
                    egui::Grid::new("adv_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                        ui.label("IOR:");
                        ui.add(egui::DragValue::new(&mut self.ior).range(1.0..=3.0).speed(0.01));
                        ui.end_row();
                        ui.label("Subsurface:");
                        ui.add(egui::Slider::new(&mut self.subsurface, 0.0..=1.0));
                        ui.end_row();
                        ui.label("Anisotropy:");
                        ui.add(egui::Slider::new(&mut self.anisotropy, 0.0..=1.0));
                        ui.end_row();
                    });
                });
                ui.collapsing("Texture Slots", |ui| {
                    let slots = [
                        TextureSlot::Albedo,
                        TextureSlot::Normal,
                        TextureSlot::ORM,
                        TextureSlot::Emissive,
                        TextureSlot::Roughness,
                    ];
                    for slot in slots {
                        self.render_texture_slot(ui, slot);
                    }
                    ui.separator();
                    egui::Grid::new("uv_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                        ui.label("UV Tiling X:");
                        ui.add(egui::DragValue::new(&mut self.uv_tiling[0]).range(0.01..=32.0).speed(0.1));
                        ui.end_row();
                        ui.label("UV Tiling Y:");
                        ui.add(egui::DragValue::new(&mut self.uv_tiling[1]).range(0.01..=32.0).speed(0.1));
                        ui.end_row();
                        ui.label("UV Offset X:");
                        ui.add(egui::DragValue::new(&mut self.uv_offset[0]).range(-10.0..=10.0).speed(0.05));
                        ui.end_row();
                        ui.label("UV Offset Y:");
                        ui.add(egui::DragValue::new(&mut self.uv_offset[1]).range(-10.0..=10.0).speed(0.05));
                        ui.end_row();
                    });
                });
                ui.collapsing("Rendering", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Mode:");
                        let modes = [
                            RenderMode::Opaque,
                            RenderMode::Masked,
                            RenderMode::Translucent,
                            RenderMode::Additive,
                        ];
                        egui::ComboBox::from_id_salt("render_mode_combo")
                            .selected_text(self.render_mode.label())
                            .show_ui(ui, |ui| {
                                for m in modes {
                                    ui.selectable_value(&mut self.render_mode, m, m.label());
                                }
                            });
                    });
                    ui.checkbox(&mut self.double_sided, "Double Sided");
                    ui.checkbox(&mut self.wireframe, "Wireframe");
                    ui.checkbox(&mut self.use_vertex_colors, "Use Vertex Colors");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    let status_color = if self.dirty {
                        egui::Color32::from_rgb(255, 200, 80)
                    } else {
                        egui::Color32::from_rgb(120, 200, 120)
                    };
                    ui.colored_label(status_color, format!("* {}", self.status_message.as_str()));
                    if self.dirty {
                        ui.label("(unsaved changes)");
                    }
                });
            });
    }
}
