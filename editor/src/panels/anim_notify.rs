//! Anim notify panel: notify list, trigger time, event type, state.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct AnimNotifyPanel {
    pub visible: bool,
    pub animation_name: String,
    pub duration: f32,
    pub notifies: Vec<AnimNotify>,
    pub selected_notify: Option<usize>,
    pub new_notify_name: String,
    pub new_notify_time: f32,
    pub show_tracks: bool,
}

#[derive(Debug, Clone)]
pub struct AnimNotify { pub name: String, pub notify_type: NotifyType, pub time: f32, pub duration: f32, pub enabled: bool, pub color: egui::Color32 }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NotifyType { Event, Sound, Particle, State, Footstep, Hit }

impl Default for AnimNotifyPanel {
    fn default() -> Self {
        Self {
            visible: false, animation_name: "Walk".into(), duration: 2.0,
            notifies: vec![
                AnimNotify { name: "Footstep_L".into(), notify_type: NotifyType::Footstep, time: 0.2, duration: 0.0, enabled: true, color: egui::Color32::from_rgb(100, 200, 100) },
                AnimNotify { name: "Footstep_R".into(), notify_type: NotifyType::Footstep, time: 0.7, duration: 0.0, enabled: true, color: egui::Color32::from_rgb(100, 200, 100) },
                AnimNotify { name: "Swing".into(), notify_type: NotifyType::Sound, time: 0.5, duration: 0.3, enabled: true, color: egui::Color32::from_rgb(200, 200, 100) },
                AnimNotify { name: "Hit".into(), notify_type: NotifyType::Hit, time: 1.0, duration: 0.1, enabled: false, color: egui::Color32::from_rgb(200, 100, 100) },
            ],
            selected_notify: Some(0), new_notify_name: String::new(), new_notify_time: 0.0, show_tracks: true,
        }
    }
}

impl EditorPanel for AnimNotifyPanel {
    fn name(&self) -> &str { "Anim Notify" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(500.0).default_height(400.0).show(ctx, |ui| {
            ui.horizontal(|ui| { ui.label("Animation:"); ui.text_edit_singleline(&mut self.animation_name); ui.label(format!("Duration: {:.2}s", self.duration)); });
            ui.separator();
            ui.checkbox(&mut self.show_tracks, "Show Tracks");
            ui.separator();
            if self.show_tracks {
                ui.label("Timeline");
                let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 2.0, egui::Color32::from_rgb(30, 30, 35));
                for i in 0..self.notifies.len() {
                    if !self.notifies[i].enabled { continue; }
                    let x = rect.min.x + (self.notifies[i].time / self.duration) * rect.width();
                    let w = if self.notifies[i].duration > 0.0 { (self.notifies[i].duration / self.duration) * rect.width() } else { 3.0 };
                    let nr = egui::Rect::from_min_size(egui::pos2(x, rect.min.y + 5.0), egui::vec2(w.max(3.0), 30.0));
                    ui.painter().rect_filled(nr, 2.0, self.notifies[i].color);
                }
                ui.separator();
            }
            ui.heading("Notifies");
            egui::ScrollArea::vertical().show(ui, |ui| {
                for i in 0..self.notifies.len() {
                    let selected = self.selected_notify == Some(i);
                    let type_str = match self.notifies[i].notify_type { NotifyType::Event => "Event", NotifyType::Sound => "Sound", NotifyType::Particle => "Particle", NotifyType::State => "State", NotifyType::Footstep => "Footstep", NotifyType::Hit => "Hit" };
                    ui.horizontal(|ui| {
                        if ui.selectable_label(selected, &self.notifies[i].name).clicked() { self.selected_notify = Some(i); }
                        ui.label(type_str);
                        ui.label(format!("{:.2}s", self.notifies[i].time));
                        ui.checkbox(&mut self.notifies[i].enabled, "");
                    });
                }
            });
            ui.separator();
            if let Some(idx) = self.selected_notify { if idx < self.notifies.len() {
                ui.heading("Properties");
                ui.horizontal(|ui| { ui.label("Time:"); ui.add(egui::Slider::new(&mut self.notifies[idx].time, 0.0..=self.duration)); });
                ui.horizontal(|ui| { ui.label("Duration:"); ui.add(egui::Slider::new(&mut self.notifies[idx].duration, 0.0..=1.0)); });
                ui.horizontal(|ui| {
                    ui.label("Type:");
                    egui::ComboBox::from_id_source("notify_type").selected_text(format!("{:?}", self.notifies[idx].notify_type)).show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.notifies[idx].notify_type, NotifyType::Event, "Event");
                        ui.selectable_value(&mut self.notifies[idx].notify_type, NotifyType::Sound, "Sound");
                        ui.selectable_value(&mut self.notifies[idx].notify_type, NotifyType::Particle, "Particle");
                        ui.selectable_value(&mut self.notifies[idx].notify_type, NotifyType::Hit, "Hit");
                    });
                });
            }}
            ui.separator();
            ui.horizontal(|ui| { ui.label("New:"); ui.text_edit_singleline(&mut self.new_notify_name); ui.label("Time:"); ui.add(egui::DragValue::new(&mut self.new_notify_time).range(0.0..=10.0)); if ui.button("Add").clicked() && !self.new_notify_name.is_empty() { self.notifies.push(AnimNotify { name: self.new_notify_name.clone(), notify_type: NotifyType::Event, time: self.new_notify_time, duration: 0.0, enabled: true, color: egui::Color32::from_rgb(150, 150, 200) }); self.new_notify_name.clear(); } });
        });
    }
}
