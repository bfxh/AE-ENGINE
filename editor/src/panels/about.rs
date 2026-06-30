//! About dialog: shows editor version and credits.
//!
//! A simple modal dialog displaying version info, tech stack, and links.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

/// About dialog state.
pub struct AboutPanel {
    /// Whether the dialog is visible.
    pub visible: bool,
}

impl Default for AboutPanel {
    fn default() -> Self {
        Self { visible: false }
    }
}

impl EditorPanel for AboutPanel {
    fn name(&self) -> &str {
        "About"
    }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible {
            return;
        }

        egui::Window::new("About Wasteland Editor")
            .default_width(400.0)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);

                    // Logo placeholder.
                    ui.painter().circle_filled(
                        ui.clip_rect().center() + egui::vec2(0.0, -20.0),
                        30.0,
                        egui::Color32::from_rgb(100, 180, 255),
                    );
                    ui.add_space(50.0);

                    ui.heading("Wasteland Editor");
                    ui.label("Version 0.1.0 (Alpha)");
                    ui.add_space(10.0);

                    ui.separator();
                    ui.add_space(5.0);

                    ui.label("A Rust-native 3D game engine editor");
                    ui.label("for the AE-ENGINE.");
                    ui.add_space(10.0);

                    ui.separator();
                    ui.add_space(5.0);

                    ui.label("Built with:");
                    ui.label("  • Rust (stable-x86_64-pc-windows-gnu)");
                    ui.label("  • wgpu 24 + winit 0.30");
                    ui.label("  • egui 0.31 + egui-wgpu 0.31");
                    ui.label("  • glam 0.32 for math");
                    ui.add_space(10.0);

                    ui.separator();
                    ui.add_space(5.0);

                    ui.label("Features:");
                    ui.label("  • Scene graph editor");
                    ui.label("  • Transform gizmo (W/E/R)");
                    ui.label("  • Undo/Redo system");
                    ui.label("  • Console with commands");
                    ui.label("  • Performance stats");
                    ui.label("  • 36+ integrated panels");
                    ui.add_space(10.0);

                    ui.separator();
                    ui.add_space(5.0);

                    ui.horizontal(|ui| {
                        ui.label("GitHub:");
                        ui.hyperlink("https://github.com/ae");
                    });
                    ui.horizontal(|ui| {
                        ui.label("Docs:");
                        ui.hyperlink("https://docs.ae.dev");
                    });

                    ui.add_space(15.0);

                    if ui.button("Close").clicked() {
                        self.visible = false;
                    }

                    ui.add_space(10.0);
                });
            });
    }
}
