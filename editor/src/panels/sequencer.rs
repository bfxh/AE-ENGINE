//! 序列器面板：时间轴动画编辑。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct SequencerPanel {
    pub visible: bool,
    pub current_frame: i32,
    pub frame_range_start: i32,
    pub frame_range_end: i32,
    pub fps: f32,
    pub playing: bool,
    pub loop_mode: LoopMode,
    pub tracks: Vec<TrackInfo>,
    pub selected_track: Option<usize>,
    pub snap_enabled: bool,
    pub snap_size: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoopMode {
    None,
    Loop,
    PingPong,
}

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub name: String,
    pub track_type: TrackType,
    pub locked: bool,
    pub muted: bool,
    pub keyframes: Vec<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrackType {
    Object,
    Camera,
    Event,
}

impl Default for SequencerPanel {
    fn default() -> Self {
        Self {
            visible: false,
            current_frame: 0,
            frame_range_start: 0,
            frame_range_end: 120,
            fps: 30.0,
            playing: false,
            loop_mode: LoopMode::Loop,
            tracks: vec![
                TrackInfo { name: "Main Object".into(), track_type: TrackType::Object, locked: false, muted: false, keyframes: vec![0, 30, 60, 90] },
                TrackInfo { name: "Camera 01".into(), track_type: TrackType::Camera, locked: false, muted: false, keyframes: vec![0, 60] },
                TrackInfo { name: "Event Track".into(), track_type: TrackType::Event, locked: false, muted: false, keyframes: vec![10, 50] },
            ],
            selected_track: None,
            snap_enabled: true,
            snap_size: 5,
        }
    }
}

impl EditorPanel for SequencerPanel {
    fn name(&self) -> &str { "Sequencer" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(700.0)
            .default_height(400.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Play").clicked() { self.playing = !self.playing; }
                    if ui.button("Stop").clicked() { self.playing = false; self.current_frame = 0; }
                    if ui.button("|<").clicked() { self.current_frame = self.frame_range_start; }
                    if ui.button(">|").clicked() { self.current_frame = self.frame_range_end; }
                    ui.separator();
                    ui.label("Frame:");
                    ui.add(egui::DragValue::new(&mut self.current_frame).range(self.frame_range_start..=self.frame_range_end));
                    ui.separator();
                    ui.label("FPS:");
                    ui.add(egui::DragValue::new(&mut self.fps).speed(0.1).range(1.0..=120.0));
                    ui.separator();
                    ui.checkbox(&mut self.snap_enabled, "Snap");
                    if self.snap_enabled {
                        ui.add(egui::DragValue::new(&mut self.snap_size).range(1..=60));
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Range:");
                    ui.add(egui::DragValue::new(&mut self.frame_range_start).range(0..=1000));
                    ui.add(egui::DragValue::new(&mut self.frame_range_end).range(0..=10000));
                    ui.separator();
                    ui.label("Loop:");
                    ui.radio_value(&mut self.loop_mode, LoopMode::None, "None");
                    ui.radio_value(&mut self.loop_mode, LoopMode::Loop, "Loop");
                    ui.radio_value(&mut self.loop_mode, LoopMode::PingPong, "PingPong");
                });
                ui.separator();
                ui.label("Tracks:");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut action: Option<(usize, TrackAction)> = None;
                    for (i, track) in self.tracks.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut track.muted, "M");
                            ui.checkbox(&mut track.locked, "L");
                            let selected = self.selected_track == Some(i);
                            if ui.selectable_label(selected, &track.name).clicked() {
                                self.selected_track = Some(i);
                            }
                            ui.label(format!("{:?}", track.track_type));
                            if ui.button("Add Key").clicked() {
                                action = Some((i, TrackAction::AddKey));
                            }
                            if ui.button("Clear").clicked() {
                                action = Some((i, TrackAction::Clear));
                            }
                            ui.label(format!("Keys: {:?}", track.keyframes.len()));
                        });
                    }
                    if let Some((i, act)) = action {
                        if i < self.tracks.len() {
                            match act {
                                TrackAction::AddKey => self.tracks[i].keyframes.push(self.current_frame),
                                TrackAction::Clear => self.tracks[i].keyframes.clear(),
                            }
                        }
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(format!("Frame: {} / {}", self.current_frame, self.frame_range_end));
                    ui.label(format!("Time: {:.2}s", self.current_frame as f32 / self.fps));
                    ui.label(format!("Tracks: {}", self.tracks.len()));
                });
            });
    }
}

enum TrackAction {
    AddKey,
    Clear,
}
