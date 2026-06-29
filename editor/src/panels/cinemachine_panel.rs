//! Cinemachine panel: virtual cameras, follow, aim, collision.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct CinemachinePanel {
    pub visible: bool,
    pub virtual_cameras: Vec<VirtualCamera>,
    pub selected_camera: Option<usize>,
    pub active_camera: Option<usize>,
    pub blend_time: f32,
    pub blend_mode: BlendMode,
    pub global_damping: f32,
    pub new_camera_name: String,
}

#[derive(Debug, Clone)]
pub struct VirtualCamera {
    pub name: String, pub body: BodyType, pub aim: AimType, pub follow_target: String, pub look_at_target: String,
    pub damping: [f32; 3], pub fov: f32, pub near_clip: f32, pub far_clip: f32, pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BodyType { Fixed, Follow, HardLock, FramingTransposer }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AimType { DoNothing, Composer, POV, GroupComposer }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlendMode { Cut, EaseInOut, Hard, Linear }

impl Default for CinemachinePanel {
    fn default() -> Self {
        Self {
            visible: false,
            virtual_cameras: vec![
                VirtualCamera { name: "PlayerFollow".into(), body: BodyType::FramingTransposer, aim: AimType::Composer, follow_target: "Player".into(), look_at_target: "Player".into(), damping: [2.0, 2.0, 2.0], fov: 60.0, near_clip: 0.1, far_clip: 1000.0, enabled: true },
                VirtualCamera { name: "Overview".into(), body: BodyType::Fixed, aim: AimType::DoNothing, follow_target: "".into(), look_at_target: "".into(), damping: [1.0, 1.0, 1.0], fov: 45.0, near_clip: 0.1, far_clip: 2000.0, enabled: true },
                VirtualCamera { name: "Cutscene1".into(), body: BodyType::Follow, aim: AimType::POV, follow_target: "Actor1".into(), look_at_target: "Actor2".into(), damping: [3.0, 3.0, 3.0], fov: 35.0, near_clip: 0.1, far_clip: 500.0, enabled: false },
            ],
            selected_camera: Some(0), active_camera: Some(0), blend_time: 2.0, blend_mode: BlendMode::EaseInOut, global_damping: 1.0, new_camera_name: String::new(),
        }
    }
}

impl EditorPanel for CinemachinePanel {
    fn name(&self) -> &str { "Cinemachine" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(500.0).default_height(450.0).show(ctx, |ui| {
            ui.heading("Global Settings");
            ui.horizontal(|ui| { ui.label("Blend Time:"); ui.add(egui::Slider::new(&mut self.blend_time, 0.0..=10.0)); });
            ui.horizontal(|ui| {
                ui.label("Blend Mode:");
                egui::ComboBox::from_id_source("blend_mode").selected_text(format!("{:?}", self.blend_mode)).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.blend_mode, BlendMode::Cut, "Cut");
                    ui.selectable_value(&mut self.blend_mode, BlendMode::EaseInOut, "EaseInOut");
                    ui.selectable_value(&mut self.blend_mode, BlendMode::Linear, "Linear");
                });
            });
            ui.horizontal(|ui| { ui.label("Global Damping:"); ui.add(egui::Slider::new(&mut self.global_damping, 0.1..=5.0)); });
            ui.separator();
            ui.heading("Virtual Cameras");
            egui::ScrollArea::vertical().show(ui, |ui| {
                for i in 0..self.virtual_cameras.len() {
                    let selected = self.selected_camera == Some(i);
                    let active = self.active_camera == Some(i);
                    ui.horizontal(|ui| {
                        if ui.selectable_label(selected, &self.virtual_cameras[i].name).clicked() { self.selected_camera = Some(i); }
                        if active { ui.label("(Active)"); }
                        ui.checkbox(&mut self.virtual_cameras[i].enabled, "");
                        if !active && ui.button("Activate").clicked() { self.active_camera = Some(i); }
                    });
                }
            });
            ui.separator();
            ui.horizontal(|ui| { ui.text_edit_singleline(&mut self.new_camera_name); if ui.button("Add Camera").clicked() && !self.new_camera_name.is_empty() { self.virtual_cameras.push(VirtualCamera { name: self.new_camera_name.clone(), body: BodyType::Fixed, aim: AimType::DoNothing, follow_target: "".into(), look_at_target: "".into(), damping: [1.0, 1.0, 1.0], fov: 60.0, near_clip: 0.1, far_clip: 1000.0, enabled: true }); self.new_camera_name.clear(); } });
            ui.separator();
            if let Some(idx) = self.selected_camera { if idx < self.virtual_cameras.len() {
                ui.heading("Camera Properties");
                ui.horizontal(|ui| { ui.label("Follow:"); ui.text_edit_singleline(&mut self.virtual_cameras[idx].follow_target); });
                ui.horizontal(|ui| { ui.label("Look At:"); ui.text_edit_singleline(&mut self.virtual_cameras[idx].look_at_target); });
                ui.horizontal(|ui| { ui.label("FOV:"); ui.add(egui::Slider::new(&mut self.virtual_cameras[idx].fov, 10.0..=120.0)); });
                ui.horizontal(|ui| { ui.label("Near:"); ui.add(egui::Slider::new(&mut self.virtual_cameras[idx].near_clip, 0.01..=5.0)); });
                ui.horizontal(|ui| { ui.label("Far:"); ui.add(egui::Slider::new(&mut self.virtual_cameras[idx].far_clip, 100.0..=10000.0)); });
                ui.horizontal(|ui| { ui.label("Damping X:"); ui.add(egui::Slider::new(&mut self.virtual_cameras[idx].damping[0], 0.0..=10.0)); });
                ui.horizontal(|ui| { ui.label("Damping Y:"); ui.add(egui::Slider::new(&mut self.virtual_cameras[idx].damping[1], 0.0..=10.0)); });
                ui.horizontal(|ui| { ui.label("Damping Z:"); ui.add(egui::Slider::new(&mut self.virtual_cameras[idx].damping[2], 0.0..=10.0)); });
            }}
        });
    }
}
