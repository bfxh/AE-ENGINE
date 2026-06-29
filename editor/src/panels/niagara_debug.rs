//! Niagara debug panel: particle stats, emitter status, performance, bounds.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct NiagaraDebugPanel {
    pub visible: bool,
    pub active_tab: NiagaraDebugTab,
    pub total_particles: u64,
    pub active_emitters: u32,
    pub cpu_time_ms: f32,
    pub gpu_time_ms: f32,
    pub memory_mb: f32,
    pub show_bounds: bool,
    pub show_overdraw: bool,
    pub show_data_interface: bool,
    pub emitters: Vec<EmitterStats>,
    pub selected_emitter: Option<usize>,
    pub max_particles: u64,
    pub sim_rate: f32,
    pub fixed_delta: f32,
    pub use_fixed_delta: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NiagaraDebugTab { Stats, Emitters, Performance, Bounds }

#[derive(Debug, Clone)]
pub struct EmitterStats {
    pub name: String,
    pub particle_count: u32,
    pub alive: bool,
    pub loops: u32,
    pub cpu_ms: f32,
    pub gpu_ms: f32,
    pub color: egui::Color32,
}

impl Default for NiagaraDebugPanel {
    fn default() -> Self {
        Self {
            visible: false,
            active_tab: NiagaraDebugTab::Stats,
            total_particles: 0,
            active_emitters: 0,
            cpu_time_ms: 0.0,
            gpu_time_ms: 0.0,
            memory_mb: 0.0,
            show_bounds: false,
            show_overdraw: false,
            show_data_interface: false,
            emitters: vec![
                EmitterStats { name: "Fire_Main".into(), particle_count: 1500, alive: true, loops: 0, cpu_ms: 0.8, gpu_ms: 1.2, color: egui::Color32::from_rgb(255, 120, 30) },
                EmitterStats { name: "Smoke_Rise".into(), particle_count: 800, alive: true, loops: 0, cpu_ms: 0.4, gpu_ms: 0.6, color: egui::Color32::from_rgb(120, 120, 120) },
                EmitterStats { name: "Spark_Burst".into(), particle_count: 320, alive: false, loops: 3, cpu_ms: 0.2, gpu_ms: 0.3, color: egui::Color32::from_rgb(255, 220, 80) },
            ],
            selected_emitter: Some(0),
            max_particles: 100000,
            sim_rate: 1.0,
            fixed_delta: 0.016,
            use_fixed_delta: false,
        }
    }
}

impl EditorPanel for NiagaraDebugPanel {
    fn name(&self) -> &str { "Niagara Debug" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(500.0).default_height(450.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, NiagaraDebugTab::Stats, "Stats");
                ui.selectable_value(&mut self.active_tab, NiagaraDebugTab::Emitters, "Emitters");
                ui.selectable_value(&mut self.active_tab, NiagaraDebugTab::Performance, "Performance");
                ui.selectable_value(&mut self.active_tab, NiagaraDebugTab::Bounds, "Bounds");
            });
            ui.separator();
            match self.active_tab {
                NiagaraDebugTab::Stats => {
                    ui.heading("Particle Statistics");
                    ui.label(format!("Total Particles: {}", self.total_particles));
                    ui.label(format!("Active Emitters: {}", self.active_emitters));
                    ui.label(format!("Memory Usage: {:.2} MB", self.memory_mb));
                    ui.separator();
                    ui.horizontal(|ui| { ui.label("Max Particles:"); ui.add(egui::DragValue::new(&mut self.max_particles).range(1000..=1000000)); });
                    ui.horizontal(|ui| { ui.label("Sim Rate:"); ui.add(egui::Slider::new(&mut self.sim_rate, 0.0..=4.0)); });
                    ui.checkbox(&mut self.use_fixed_delta, "Use Fixed Delta");
                    if self.use_fixed_delta {
                        ui.horizontal(|ui| { ui.label("Fixed Delta:"); ui.add(egui::Slider::new(&mut self.fixed_delta, 0.001..=0.05)); });
                    }
                },
                NiagaraDebugTab::Emitters => {
                    ui.heading("Emitters");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for i in 0..self.emitters.len() {
                            let selected = self.selected_emitter == Some(i);
                            let ec = self.emitters[i].color;
                            ui.horizontal(|ui| {
                                if ui.selectable_label(selected, &self.emitters[i].name).clicked() { self.selected_emitter = Some(i); }
                                let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 2.0, ec);
                                ui.label(format!("{} particles", self.emitters[i].particle_count));
                                ui.checkbox(&mut self.emitters[i].alive, "Alive");
                            });
                        }
                    });
                    ui.separator();
                    if let Some(idx) = self.selected_emitter { if idx < self.emitters.len() {
                        ui.heading("Emitter Detail");
                        ui.label(format!("Loops: {}", self.emitters[idx].loops));
                        ui.label(format!("CPU: {:.3} ms", self.emitters[idx].cpu_ms));
                        ui.label(format!("GPU: {:.3} ms", self.emitters[idx].gpu_ms));
                        ui.horizontal(|ui| { ui.label("Particle Count:"); ui.add(egui::DragValue::new(&mut self.emitters[idx].particle_count).range(0..=100000)); });
                    }}
                },
                NiagaraDebugTab::Performance => {
                    ui.heading("Performance");
                    ui.horizontal(|ui| { ui.label("CPU Time:"); ui.add(egui::Slider::new(&mut self.cpu_time_ms, 0.0..=20.0)); });
                    ui.horizontal(|ui| { ui.label("GPU Time:"); ui.add(egui::Slider::new(&mut self.gpu_time_ms, 0.0..=20.0)); });
                    ui.label(format!("Total: {:.3} ms", self.cpu_time_ms + self.gpu_time_ms));
                    ui.separator();
                    ui.checkbox(&mut self.show_overdraw, "Show Overdraw");
                    ui.checkbox(&mut self.show_data_interface, "Show Data Interface");
                    if ui.button("Reset Counters").clicked() { self.cpu_time_ms = 0.0; self.gpu_time_ms = 0.0; }
                },
                NiagaraDebugTab::Bounds => {
                    ui.heading("Bounding Volumes");
                    ui.checkbox(&mut self.show_bounds, "Show Bounds");
                    ui.label("Display bounding boxes for each emitter.");
                    ui.separator();
                    for i in 0..self.emitters.len() {
                        ui.horizontal(|ui| {
                            ui.label(&self.emitters[i].name);
                            ui.label(if self.emitters[i].alive { "visible" } else { "hidden" });
                        });
                    }
                },
            }
        });
    }
}
