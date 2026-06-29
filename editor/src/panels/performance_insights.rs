//! Performance insights panel: CPU/GPU tracing, timeline, hotspots.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct PerformanceInsightsPanel {
    pub visible: bool,
    pub active_tab: PerfTab,
    pub cpu_traces: Vec<TraceEntry>,
    pub gpu_traces: Vec<TraceEntry>,
    pub hotspots: Vec<Hotspot>,
    pub frame_time: f32,
    pub cpu_time: f32,
    pub gpu_time: f32,
    pub draw_calls: u32,
    pub triangles: u32,
    pub recording: bool,
    pub record_duration: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PerfTab { CpuTrace, GpuTrace, Hotspots, Summary }

#[derive(Debug, Clone)]
pub struct TraceEntry { pub name: String, pub duration_ms: f32, pub category: String, pub depth: u32 }

#[derive(Debug, Clone)]
pub struct Hotspot { pub function: String, pub calls: u32, pub total_ms: f32, pub avg_ms: f32 }

impl Default for PerformanceInsightsPanel {
    fn default() -> Self {
        Self {
            visible: false, active_tab: PerfTab::Summary,
            cpu_traces: vec![
                TraceEntry { name: "Update".into(), duration_ms: 2.3, category: "Logic".into(), depth: 0 },
                TraceEntry { name: "Physics".into(), duration_ms: 1.5, category: "Physics".into(), depth: 1 },
                TraceEntry { name: "Render".into(), duration_ms: 8.2, category: "Render".into(), depth: 0 },
            ],
            gpu_traces: vec![
                TraceEntry { name: "Shadow Pass".into(), duration_ms: 2.1, category: "Shadow".into(), depth: 0 },
                TraceEntry { name: "Geometry Pass".into(), duration_ms: 3.5, category: "Geometry".into(), depth: 0 },
                TraceEntry { name: "Post Process".into(), duration_ms: 1.8, category: "PostFX".into(), depth: 0 },
            ],
            hotspots: vec![
                Hotspot { function: "render_mesh".into(), calls: 450, total_ms: 5.2, avg_ms: 0.012 },
                Hotspot { function: "update_transforms".into(), calls: 1200, total_ms: 3.1, avg_ms: 0.003 },
                Hotspot { function: "cull_objects".into(), calls: 60, total_ms: 1.5, avg_ms: 0.025 },
            ],
            frame_time: 16.67, cpu_time: 12.0, gpu_time: 7.4, draw_calls: 450, triangles: 1250000,
            recording: false, record_duration: 0.0,
        }
    }
}

impl EditorPanel for PerformanceInsightsPanel {
    fn name(&self) -> &str { "Performance Insights" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(550.0).default_height(450.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(if self.recording { "Stop" } else { "Record" }).clicked() { self.recording = !self.recording; if !self.recording { self.record_duration = 0.0; } }
                if self.recording { ui.label(format!("Recording: {:.1}s", self.record_duration)); }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, PerfTab::Summary, "Summary");
                ui.selectable_value(&mut self.active_tab, PerfTab::CpuTrace, "CPU Trace");
                ui.selectable_value(&mut self.active_tab, PerfTab::GpuTrace, "GPU Trace");
                ui.selectable_value(&mut self.active_tab, PerfTab::Hotspots, "Hotspots");
            });
            ui.separator();
            match self.active_tab {
                PerfTab::Summary => {
                    ui.heading("Frame Stats");
                    ui.horizontal(|ui| { ui.label("Frame Time:"); ui.label(format!("{:.2} ms", self.frame_time)); });
                    ui.horizontal(|ui| { ui.label("CPU Time:"); ui.label(format!("{:.2} ms", self.cpu_time)); });
                    ui.horizontal(|ui| { ui.label("GPU Time:"); ui.label(format!("{:.2} ms", self.gpu_time)); });
                    ui.separator();
                    ui.horizontal(|ui| { ui.label("Draw Calls:"); ui.label(format!("{}", self.draw_calls)); });
                    ui.horizontal(|ui| { ui.label("Triangles:"); ui.label(format!("{}", self.triangles)); });
                },
                PerfTab::CpuTrace => {
                    ui.heading("CPU Trace");
                    for i in 0..self.cpu_traces.len() {
                        let indent = "  ".repeat(self.cpu_traces[i].depth as usize);
                        ui.horizontal(|ui| { ui.label(format!("{}{}", indent, self.cpu_traces[i].name)); ui.label(&self.cpu_traces[i].category); ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label(format!("{:.2} ms", self.cpu_traces[i].duration_ms)); }); });
                    }
                },
                PerfTab::GpuTrace => {
                    ui.heading("GPU Trace");
                    for i in 0..self.gpu_traces.len() {
                        ui.horizontal(|ui| { ui.label(&self.gpu_traces[i].name); ui.label(&self.gpu_traces[i].category); ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label(format!("{:.2} ms", self.gpu_traces[i].duration_ms)); }); });
                    }
                },
                PerfTab::Hotspots => {
                    ui.heading("Hotspots");
                    for i in 0..self.hotspots.len() {
                        ui.horizontal(|ui| { ui.label(&self.hotspots[i].function); ui.label(format!("{} calls", self.hotspots[i].calls)); ui.label(format!("{:.2} ms total", self.hotspots[i].total_ms)); ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label(format!("{:.3} ms avg", self.hotspots[i].avg_ms)); }); });
                    }
                },
            }
        });
    }
}
