//! Session frontend panel: device list, session management, multi-device testing.

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct SessionFrontendPanel {
    pub visible: bool,
    pub devices: Vec<Device>,
    pub selected_device: Option<usize>,
    pub sessions: Vec<Session>,
    pub auto_connect: bool,
    pub session_name: String,
    pub max_players: i32,
    pub current_session: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Device { pub name: String, pub address: String, pub platform: String, pub status: DeviceStatus, pub ping: f32 }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceStatus { Online, Offline, Busy, Connecting }

#[derive(Debug, Clone)]
pub struct Session { pub name: String, pub device: String, pub players: i32, pub max_players: i32, pub active: bool }

impl Default for SessionFrontendPanel {
    fn default() -> Self {
        Self {
            visible: false,
            devices: vec![
                Device { name: "Windows PC".into(), address: "192.168.1.10".into(), platform: "Windows".into(), status: DeviceStatus::Online, ping: 5.2 },
                Device { name: "MacBook".into(), address: "192.168.1.20".into(), platform: "macOS".into(), status: DeviceStatus::Online, ping: 12.8 },
                Device { name: "Android Phone".into(), address: "192.168.1.30".into(), platform: "Android".into(), status: DeviceStatus::Offline, ping: 0.0 },
            ],
            selected_device: Some(0), sessions: vec![], auto_connect: false, session_name: String::new(), max_players: 4, current_session: None,
        }
    }
}

impl EditorPanel for SessionFrontendPanel {
    fn name(&self) -> &str { "Session Frontend" }
    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name()).default_width(500.0).default_height(400.0).show(ctx, |ui| {
            ui.heading("Devices");
            ui.separator();
            ui.checkbox(&mut self.auto_connect, "Auto-connect");
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for i in 0..self.devices.len() {
                    let selected = self.selected_device == Some(i);
                    let status_str = match self.devices[i].status { DeviceStatus::Online => "Online", DeviceStatus::Offline => "Offline", DeviceStatus::Busy => "Busy", DeviceStatus::Connecting => "Connecting..." };
                    ui.horizontal(|ui| {
                        if ui.selectable_label(selected, &self.devices[i].name).clicked() { self.selected_device = Some(i); }
                        ui.label(&self.devices[i].platform);
                        ui.label(&self.devices[i].address);
                        ui.label(status_str);
                        if self.devices[i].status == DeviceStatus::Online { ui.label(format!("{:.0}ms", self.devices[i].ping)); }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if self.devices[i].status == DeviceStatus::Online { if ui.button("Connect").clicked() {} }
                            else if self.devices[i].status == DeviceStatus::Offline { if ui.button("Wake").clicked() { self.devices[i].status = DeviceStatus::Connecting; } }
                        });
                    });
                }
            });
            ui.separator();
            ui.heading("New Session");
            ui.horizontal(|ui| { ui.label("Name:"); ui.text_edit_singleline(&mut self.session_name); });
            ui.horizontal(|ui| { ui.label("Max Players:"); ui.add(egui::DragValue::new(&mut self.max_players).range(1..=32)); });
            if ui.button("Create Session").clicked() && !self.session_name.is_empty() {
                let dev = self.selected_device.map(|i| self.devices.get(i).map(|d| d.name.clone()).unwrap_or_default()).unwrap_or_default();
                self.sessions.push(Session { name: self.session_name.clone(), device: dev, players: 1, max_players: self.max_players, active: true });
                self.current_session = Some(self.sessions.len() - 1);
                self.session_name.clear();
            }
            ui.separator();
            ui.heading("Active Sessions");
            for i in 0..self.sessions.len() {
                ui.horizontal(|ui| {
                    ui.label(&self.sessions[i].name);
                    ui.label(format!("{}/{} players", self.sessions[i].players, self.sessions[i].max_players));
                    ui.label(&self.sessions[i].device);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { if ui.button("End").clicked() { self.sessions[i].active = false; } });
                });
            }
        });
    }
}
