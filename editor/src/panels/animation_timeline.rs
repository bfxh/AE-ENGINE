//! 动画时间轴面板：专业动画时间轴编辑。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct AnimationTimelinePanel {
    pub visible: bool,
    pub current_time: f32,
    pub duration: f32,
    pub fps: f32,
    pub playing: bool,
    pub loop_mode: bool,
    pub tracks: Vec<Track>,
    pub selected_track: usize,
    pub selected_keyframe: Option<(usize, usize)>,
    pub zoom: f32,
    pub onion_skin: bool,
    pub onion_skin_frames: u32,
    pub curve_mode: bool,
    pub snap_enabled: bool,
    pub show_grid: bool,
}

#[derive(Debug, Clone)]
pub struct Track {
    pub name: String,
    pub track_type: TrackType,
    pub expanded: bool,
    pub locked: bool,
    pub muted: bool,
    pub visible: bool,
    pub keyframes: Vec<Keyframe>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrackType {
    Transform,
    Rotation,
    Scale,
    Visibility,
    Event,
}

#[derive(Debug, Clone, Copy)]
pub struct Keyframe {
    pub time: f32,
    pub value: f32,
}

impl TrackType {
    fn color(self) -> egui::Color32 {
        match self {
            TrackType::Transform => egui::Color32::from_rgb(100, 180, 255),
            TrackType::Rotation => egui::Color32::from_rgb(255, 180, 100),
            TrackType::Scale => egui::Color32::from_rgb(180, 255, 100),
            TrackType::Visibility => egui::Color32::from_rgb(255, 100, 200),
            TrackType::Event => egui::Color32::from_rgb(255, 220, 100),
        }
    }
    fn label(self) -> &'static str {
        match self {
            TrackType::Transform => "Transform",
            TrackType::Rotation => "Rotation",
            TrackType::Scale => "Scale",
            TrackType::Visibility => "Visibility",
            TrackType::Event => "Event",
        }
    }
}

impl Default for AnimationTimelinePanel {
    fn default() -> Self {
        Self {
            visible: false,
            current_time: 0.0,
            duration: 5.0,
            fps: 30.0,
            playing: false,
            loop_mode: true,
            tracks: vec![
                Track {
                    name: "Transform".into(),
                    track_type: TrackType::Transform,
                    expanded: true,
                    locked: false,
                    muted: false,
                    visible: true,
                    keyframes: vec![
                        Keyframe { time: 0.0, value: 0.0 },
                        Keyframe { time: 1.0, value: 1.0 },
                        Keyframe { time: 2.5, value: 0.5 },
                        Keyframe { time: 4.0, value: 1.0 },
                    ],
                },
                Track {
                    name: "Rotation".into(),
                    track_type: TrackType::Rotation,
                    expanded: true,
                    locked: false,
                    muted: false,
                    visible: true,
                    keyframes: vec![
                        Keyframe { time: 0.0, value: 0.0 },
                        Keyframe { time: 2.0, value: 90.0 },
                        Keyframe { time: 5.0, value: 360.0 },
                    ],
                },
                Track {
                    name: "Scale".into(),
                    track_type: TrackType::Scale,
                    expanded: false,
                    locked: false,
                    muted: false,
                    visible: true,
                    keyframes: vec![
                        Keyframe { time: 0.0, value: 1.0 },
                        Keyframe { time: 3.0, value: 2.0 },
                    ],
                },
                Track {
                    name: "Visibility".into(),
                    track_type: TrackType::Visibility,
                    expanded: false,
                    locked: false,
                    muted: false,
                    visible: true,
                    keyframes: vec![
                        Keyframe { time: 0.0, value: 1.0 },
                        Keyframe { time: 4.0, value: 0.0 },
                    ],
                },
                Track {
                    name: "Event".into(),
                    track_type: TrackType::Event,
                    expanded: false,
                    locked: false,
                    muted: false,
                    visible: true,
                    keyframes: vec![
                        Keyframe { time: 1.0, value: 0.0 },
                        Keyframe { time: 2.0, value: 0.0 },
                        Keyframe { time: 3.5, value: 0.0 },
                    ],
                },
            ],
            selected_track: 0,
            selected_keyframe: None,
            zoom: 1.0,
            onion_skin: false,
            onion_skin_frames: 3,
            curve_mode: false,
            snap_enabled: true,
            show_grid: true,
        }
    }
}

impl EditorPanel for AnimationTimelinePanel {
    fn name(&self) -> &str { "Animation Timeline" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }

        // Playback update
        if self.playing {
            let dt = ctx.input(|i| i.unstable_dt);
            self.current_time += dt;
            if self.current_time >= self.duration {
                if self.loop_mode {
                    self.current_time = self.current_time % self.duration;
                } else {
                    self.current_time = self.duration;
                    self.playing = false;
                }
            }
        }

        egui::Window::new("Animation Timeline")
            .default_width(820.0)
            .default_height(420.0)
            .show(ctx, |ui| {
                self.render_toolbar(ui);
                ui.separator();
                self.render_time_slider(ui);
                ui.separator();
                self.render_track_list(ui);
                ui.separator();
                self.render_timeline(ui);
                ui.separator();
                self.render_status_bar(ui);
            });
    }
}

impl AnimationTimelinePanel {
    fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            // Playback controls
            if ui.button(if self.playing { "⏸ Pause" } else { "▶ Play" }).clicked() {
                self.playing = !self.playing;
            }
            if ui.button("⏹ Stop").clicked() {
                self.playing = false;
                self.current_time = 0.0;
            }
            if ui.button("⏮ Start").clicked() {
                self.current_time = 0.0;
            }
            if ui.button("⏭ End").clicked() {
                self.current_time = self.duration;
            }
            ui.separator();

            // Loop toggle
            ui.checkbox(&mut self.loop_mode, "🔁 Loop");
            ui.separator();

            // FPS presets
            ui.label("FPS:");
            for &fps in &[24.0, 30.0, 60.0, 120.0] {
                if ui.selectable_label(self.fps == fps, format!("{}", fps)).clicked() {
                    self.fps = fps;
                }
            }
            ui.separator();

            // Zoom
            ui.label("Zoom:");
            ui.add(egui::Slider::new(&mut self.zoom, 0.25..=4.0).step_by(0.25));
            ui.separator();

            // Onion skin
            ui.checkbox(&mut self.onion_skin, "🧅 Onion Skin");
            if self.onion_skin {
                ui.add(egui::DragValue::new(&mut self.onion_skin_frames).range(1..=10));
            }
            ui.separator();

            // Curve mode and helpers
            ui.checkbox(&mut self.curve_mode, "📈 Curve Mode");
            ui.checkbox(&mut self.snap_enabled, "🧲 Snap");
            ui.checkbox(&mut self.show_grid, "▦ Grid");
        });
    }

    fn render_time_slider(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Time:");
            ui.add(
                egui::Slider::new(&mut self.current_time, 0.0..=self.duration)
                    .suffix("s")
                    .text("current"),
            );
            ui.separator();
            ui.label("Duration:");
            ui.add(
                egui::DragValue::new(&mut self.duration)
                    .range(0.1..=600.0)
                    .speed(0.1)
                    .suffix("s"),
            );
            ui.separator();
            let frame = (self.current_time * self.fps).round() as i32;
            let total_frames = (self.duration * self.fps).round() as i32;
            ui.label(format!("Frame: {} / {}", frame, total_frames));
        });
    }

    fn render_track_list(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Tracks:");
            if ui.button("+ Add Track").clicked() {
                self.tracks.push(Track {
                    name: format!("Track {}", self.tracks.len() + 1),
                    track_type: TrackType::Transform,
                    expanded: true,
                    locked: false,
                    muted: false,
                    visible: true,
                    keyframes: vec![],
                });
            }
            if ui.button("Clear All Keys").clicked() {
                for t in &mut self.tracks { t.keyframes.clear(); }
            }
        });

        egui::ScrollArea::vertical()
            .max_height(160.0)
            .show(ui, |ui| {
                let mut add_key_action: Option<usize> = None;
                let mut delete_track: Option<usize> = None;

                for (i, track) in self.tracks.iter_mut().enumerate() {
                    let selected = self.selected_track == i;
                    let header_color = track.track_type.color();

                    ui.horizontal(|ui| {
                        let arrow = if track.expanded { "▼" } else { "▶" };
                        if ui.button(arrow).clicked() {
                            track.expanded = !track.expanded;
                        }

                        let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 16.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, header_color);

                        if ui.selectable_label(selected, track.name.as_str()).clicked() {
                            self.selected_track = i;
                        }

                        ui.label(track.track_type.label());

                        ui.checkbox(&mut track.muted, "M");
                        ui.checkbox(&mut track.locked, "L");
                        ui.checkbox(&mut track.visible, "V");

                        if !track.locked {
                            if ui.button("Add Key").clicked() {
                                add_key_action = Some(i);
                            }
                            if ui.button("✕").clicked() {
                                delete_track = Some(i);
                            }
                        }

                        ui.label(format!("Keys: {}", track.keyframes.len()));
                    });

                    if track.expanded {
                        ui.indent(format!("track_indent_{}", i), |ui| {
                            if track.keyframes.is_empty() {
                                ui.label("No keyframes. Click 'Add Key' to create one at current time.");
                            } else {
                                let mut del_key: Option<usize> = None;
                                let mut sel_key: Option<usize> = None;
                                for (ki, kf) in track.keyframes.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        let is_sel = self.selected_keyframe == Some((i, ki));
                                        if ui.selectable_label(
                                            is_sel,
                                            format!("Key {}: t={:.2}s, v={:.3}", ki, kf.time, kf.value),
                                        ).clicked() {
                                            sel_key = Some(ki);
                                        }
                                        ui.label("t:");
                                        ui.add(
                                            egui::DragValue::new(&mut kf.time)
                                                .range(0.0..=self.duration)
                                                .speed(0.01)
                                                .suffix("s"),
                                        );
                                        ui.label("v:");
                                        ui.add(egui::DragValue::new(&mut kf.value).speed(0.01));
                                        if ui.button("Del").clicked() {
                                            del_key = Some(ki);
                                        }
                                    });
                                }
                                if let Some(ki) = del_key {
                                    track.keyframes.remove(ki);
                                    if self.selected_keyframe == Some((i, ki)) {
                                        self.selected_keyframe = None;
                                    }
                                }
                                if let Some(ki) = sel_key {
                                    self.selected_keyframe = Some((i, ki));
                                }
                            }
                        });
                    }
                }

                if let Some(i) = add_key_action {
                    if i < self.tracks.len() && !self.tracks[i].locked {
                        self.tracks[i].keyframes.push(Keyframe {
                            time: self.current_time,
                            value: 0.0,
                        });
                    }
                }
                if let Some(i) = delete_track {
                    if i < self.tracks.len() {
                        self.tracks.remove(i);
                        if self.selected_track >= self.tracks.len() && !self.tracks.is_empty() {
                            self.selected_track = self.tracks.len() - 1;
                        }
                        self.selected_keyframe = None;
                    }
                }
            });
    }

    fn render_timeline(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Timeline:");
            ui.label(format!("(zoom {:.2}x — click/drag to scrub)", self.zoom));
        });

        egui::Frame::canvas(ui.style()).show(ui, |ui| {
            let header_width = 120.0;
            let ruler_height = 28.0;
            let track_height = 26.0;

            // Compute visible time window
            let visible_duration = (self.duration / self.zoom).max(0.1);
            let time_start = if visible_duration >= self.duration {
                0.0
            } else {
                (self.current_time - visible_duration * 0.5)
                    .clamp(0.0, (self.duration - visible_duration).max(0.0))
            };
            let time_end = (time_start + visible_duration).min(self.duration);
            let time_range = (time_end - time_start).max(0.001);

            let available_width = ui.available_width();
            let total_height = (self.tracks.len() as f32 * track_height) + ruler_height;
            let size = egui::vec2(available_width.max(500.0), total_height.max(120.0));
            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
            let painter = ui.painter();

            // Background
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(25, 25, 30));

            let timeline_rect = egui::Rect::from_min_size(
                egui::pos2(rect.min.x + header_width, rect.min.y + ruler_height),
                egui::vec2((rect.width() - header_width).max(1.0), rect.height() - ruler_height),
            );

            // Grid + ruler
            if self.show_grid {
                let divisions = 10;
                for i in 0..=divisions {
                    let frac = i as f32 / divisions as f32;
                    let t = time_start + frac * time_range;
                    let x = timeline_rect.min.x + frac * timeline_rect.width();
                    painter.line_segment(
                        [egui::pos2(x, timeline_rect.min.y), egui::pos2(x, timeline_rect.max.y)],
                        egui::Stroke::new(0.5, egui::Color32::from_rgb(50, 50, 60)),
                    );
                    painter.text(
                        egui::pos2(x, rect.min.y + 4.0),
                        egui::Align2::CENTER_TOP,
                        format!("{:.2}s", t),
                        egui::FontId::proportional(10.0),
                        egui::Color32::from_rgb(150, 150, 160),
                    );
                }
                for i in 0..=self.tracks.len() {
                    let y = timeline_rect.min.y + (i as f32 * track_height);
                    painter.line_segment(
                        [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                        egui::Stroke::new(0.5, egui::Color32::from_rgb(50, 50, 60)),
                    );
                }
            }

            // Header separator
            painter.line_segment(
                [
                    egui::pos2(rect.min.x, timeline_rect.min.y),
                    egui::pos2(rect.max.x, timeline_rect.min.y),
                ],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 90)),
            );
            painter.line_segment(
                [
                    egui::pos2(timeline_rect.min.x, rect.min.y),
                    egui::pos2(timeline_rect.min.x, rect.max.y),
                ],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 90)),
            );

            // Tracks: labels + keyframes
            for (ti, track) in self.tracks.iter().enumerate() {
                let y = timeline_rect.min.y + (ti as f32 * track_height) + track_height * 0.5;

                // Track label in header
                let label_color = if track.muted {
                    egui::Color32::from_rgb(100, 100, 100)
                } else {
                    track.track_type.color()
                };
                painter.text(
                    egui::pos2(rect.min.x + 6.0, y),
                    egui::Align2::LEFT_CENTER,
                    track.name.as_str(),
                    egui::FontId::proportional(11.0),
                    label_color,
                );

                if track.muted || !track.visible {
                    continue;
                }

                // Keyframe diamonds
                for (ki, kf) in track.keyframes.iter().enumerate() {
                    if kf.time < time_start || kf.time > time_end {
                        continue;
                    }
                    let x = timeline_rect.min.x + ((kf.time - time_start) / time_range) * timeline_rect.width();
                    let is_selected = self.selected_keyframe == Some((ti, ki));
                    let size = if is_selected { 6.0 } else { 4.0 };
                    let color = if is_selected {
                        egui::Color32::WHITE
                    } else {
                        track.track_type.color()
                    };

                    let points = vec![
                        egui::pos2(x, y - size),
                        egui::pos2(x + size, y),
                        egui::pos2(x, y + size),
                        egui::pos2(x - size, y),
                    ];
                    painter.add(egui::Shape::convex_polygon(
                        points,
                        color,
                        egui::Stroke::new(1.0, egui::Color32::BLACK),
                    ));
                }
            }

            // Onion skin markers
            if self.onion_skin {
                let frame_time = 1.0 / self.fps;
                for offset in 1..=self.onion_skin_frames {
                    let alpha = 1.0 - (offset as f32 / self.onion_skin_frames as f32) * 0.7;
                    let color = egui::Color32::from_rgba_unmultiplied(
                        255,
                        200,
                        100,
                        (alpha * 255.0) as u8,
                    );
                    let t_past = self.current_time - (offset as f32 * frame_time);
                    if t_past >= time_start && t_past <= time_end {
                        let x = timeline_rect.min.x
                            + ((t_past - time_start) / time_range) * timeline_rect.width();
                        painter.line_segment(
                            [egui::pos2(x, timeline_rect.min.y), egui::pos2(x, timeline_rect.max.y)],
                            egui::Stroke::new(1.0, color),
                        );
                    }
                    let t_future = self.current_time + (offset as f32 * frame_time);
                    if t_future >= time_start && t_future <= time_end {
                        let x = timeline_rect.min.x
                            + ((t_future - time_start) / time_range) * timeline_rect.width();
                        painter.line_segment(
                            [egui::pos2(x, timeline_rect.min.y), egui::pos2(x, timeline_rect.max.y)],
                            egui::Stroke::new(1.0, color),
                        );
                    }
                }
            }

            // Playhead
            if self.current_time >= time_start && self.current_time <= time_end {
                let playhead_x = timeline_rect.min.x
                    + ((self.current_time - time_start) / time_range) * timeline_rect.width();
                painter.line_segment(
                    [egui::pos2(playhead_x, rect.min.y), egui::pos2(playhead_x, rect.max.y)],
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 200, 50)),
                );
                painter.rect_filled(
                    egui::Rect::from_center_size(
                        egui::pos2(playhead_x, rect.min.y + 12.0),
                        egui::vec2(12.0, 16.0),
                    ),
                    2.0,
                    egui::Color32::from_rgb(255, 200, 50),
                );
            }

            // Scrub interaction
            if response.clicked() || response.dragged() {
                if let Some(pos) = response.interact_pointer_pos() {
                    if pos.x >= timeline_rect.min.x && pos.x <= timeline_rect.max.x {
                        let t = time_start
                            + ((pos.x - timeline_rect.min.x) / timeline_rect.width()) * time_range;
                        let snapped = if self.snap_enabled {
                            (t * self.fps).round() / self.fps
                        } else {
                            t
                        };
                        self.current_time = snapped.clamp(0.0, self.duration);
                    }
                }
            }
        });
    }

    fn render_status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(format!("Time: {:.3}s / {:.3}s", self.current_time, self.duration));
            ui.separator();
            let frame = (self.current_time * self.fps).round() as i32;
            let total_frames = (self.duration * self.fps).round() as i32;
            ui.label(format!("Frame: {} / {}", frame, total_frames));
            ui.separator();
            ui.label(format!("FPS: {:.0}", self.fps));
            ui.separator();
            ui.label(format!("Tracks: {}", self.tracks.len()));
            ui.separator();
            ui.label(format!("Zoom: {:.2}x", self.zoom));
            ui.separator();
            let total_keys: usize = self.tracks.iter().map(|t| t.keyframes.len()).sum();
            ui.label(format!("Keyframes: {}", total_keys));
            ui.separator();
            if let Some((ti, ki)) = self.selected_keyframe {
                if ti < self.tracks.len() && ki < self.tracks[ti].keyframes.len() {
                    let kf = self.tracks[ti].keyframes[ki];
                    ui.label(format!(
                        "Selected: {}[{}] t={:.2}s v={:.3}",
                        self.tracks[ti].name, ki, kf.time, kf.value
                    ));
                }
            } else {
                ui.label("No keyframe selected");
            }
        });
    }
}
