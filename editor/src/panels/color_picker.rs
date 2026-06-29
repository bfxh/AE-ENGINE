//! 颜色选择器面板：HSV/RGB、色轮、色板、历史、取色器。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct ColorPickerPanel {
    pub visible: bool,
    pub current_color: egui::Color32,
    pub hsv: [f32; 3],
    pub rgb: [u8; 3],
    pub alpha: f32,
    pub history: Vec<egui::Color32>,
    pub palette: Vec<egui::Color32>,
    pub mode: ColorMode,
    pub hex_input: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorMode {
    Rgb,
    Hsv,
}

impl Default for ColorPickerPanel {
    fn default() -> Self {
        Self {
            visible: false,
            current_color: egui::Color32::from_rgb(200, 100, 50),
            hsv: [0.05, 0.75, 0.78],
            rgb: [200, 100, 50],
            alpha: 1.0,
            history: vec![
                egui::Color32::from_rgb(255, 255, 255),
                egui::Color32::from_rgb(0, 0, 0),
                egui::Color32::from_rgb(255, 0, 0),
                egui::Color32::from_rgb(0, 255, 0),
                egui::Color32::from_rgb(0, 0, 255),
            ],
            palette: vec![
                egui::Color32::from_rgb(255, 255, 255), egui::Color32::from_rgb(200, 200, 200),
                egui::Color32::from_rgb(150, 150, 150), egui::Color32::from_rgb(100, 100, 100),
                egui::Color32::from_rgb(50, 50, 50), egui::Color32::from_rgb(0, 0, 0),
                egui::Color32::from_rgb(255, 0, 0), egui::Color32::from_rgb(255, 128, 0),
                egui::Color32::from_rgb(255, 255, 0), egui::Color32::from_rgb(128, 255, 0),
                egui::Color32::from_rgb(0, 255, 0), egui::Color32::from_rgb(0, 255, 128),
            ],
            mode: ColorMode::Rgb,
            hex_input: "#C86432".into(),
        }
    }
}

impl ColorPickerPanel {
    fn update_from_rgb(&mut self) {
        self.current_color = egui::Color32::from_rgba_unmultiplied(
            self.rgb[0], self.rgb[1], self.rgb[2],
            (self.alpha * 255.0) as u8,
        );
        self.hex_input = format!("#{:02X}{:02X}{:02X}", self.rgb[0], self.rgb[1], self.rgb[2]);
    }
}

impl EditorPanel for ColorPickerPanel {
    fn name(&self) -> &str { "Color Picker" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(350.0)
            .default_height(450.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Mode:");
                    ui.selectable_value(&mut self.mode, ColorMode::Rgb, "RGB");
                    ui.selectable_value(&mut self.mode, ColorMode::Hsv, "HSV");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    let size = egui::vec2(60.0, 60.0);
                    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                    ui.painter().rect_filled(rect, 4.0, self.current_color);
                    ui.vertical(|ui| {
                        ui.label("Current Color");
                        ui.label(&self.hex_input.as_str());
                        if ui.button("Pick from Screen").clicked() {}
                    });
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.color_picker_button_rgb(&mut self.current_color);
                    ui.label("Color Picker");
                });
                ui.separator();
                match self.mode {
                    ColorMode::Rgb => {
                        ui.label("RGB");
                        ui.horizontal(|ui| {
                            ui.label("R:");
                            if ui.add(egui::Slider::new(&mut self.rgb[0], 0..=255)).changed() {
                                self.update_from_rgb();
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("G:");
                            if ui.add(egui::Slider::new(&mut self.rgb[1], 0..=255)).changed() {
                                self.update_from_rgb();
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("B:");
                            if ui.add(egui::Slider::new(&mut self.rgb[2], 0..=255)).changed() {
                                self.update_from_rgb();
                            }
                        });
                    },
                    ColorMode::Hsv => {
                        ui.label("HSV");
                        ui.horizontal(|ui| { ui.label("H:"); ui.add(egui::Slider::new(&mut self.hsv[0], 0.0..=1.0)); });
                        ui.horizontal(|ui| { ui.label("S:"); ui.add(egui::Slider::new(&mut self.hsv[1], 0.0..=1.0)); });
                        ui.horizontal(|ui| { ui.label("V:"); ui.add(egui::Slider::new(&mut self.hsv[2], 0.0..=1.0)); });
                    },
                }
                ui.horizontal(|ui| {
                    ui.label("A:");
                    ui.add(egui::Slider::new(&mut self.alpha, 0.0..=1.0));
                });
                ui.separator();
                ui.label("Hex:");
                ui.text_edit_singleline(&mut self.hex_input);
                ui.separator();
                ui.label("Palette");
                ui.horizontal_wrapped(|ui| {
                    for i in 0..self.palette.len() {
                        if ui.color_button(self.palette[i], egui::Color32::from_rgb(80,80,80)).clicked() {
                            let c = self.palette[i];
                            self.rgb = [c.r(), c.g(), c.b()];
                            self.update_from_rgb();
                        }
                    }
                });
                ui.separator();
                ui.label("History");
                ui.horizontal_wrapped(|ui| {
                    for i in 0..self.history.len() {
                        if ui.color_button(self.history[i], egui::Color32::from_rgb(80,80,80)).clicked() {
                            let c = self.history[i];
                            self.rgb = [c.r(), c.g(), c.b()];
                            self.update_from_rgb();
                        }
                    }
                });
                ui.separator();
                if ui.button("Save to History").clicked() {
                    self.history.push(self.current_color);
                    if self.history.len() > 20 { self.history.remove(0); }
                }
            });
    }
}
