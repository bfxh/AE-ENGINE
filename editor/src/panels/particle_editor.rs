use crate::app::EditorApp;
use crate::panels::EditorPanel;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EmitterShape {
    Point,
    Box,
    Sphere,
    Cone,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderMode {
    Billboard,
    Stretched,
    Mesh,
}

pub struct ParticleEditorPanel {
    pub visible: bool,
    pub emission_rate: f32,
    pub duration: f32,
    pub loop_enabled: bool,
    pub burst_count: u32,
    pub burst_interval: f32,
    pub max_particles: u32,
    pub lifetime_min: f32,
    pub lifetime_max: f32,
    pub start_speed: f32,
    pub start_size: f32,
    pub end_size: f32,
    pub start_color: [f32; 4],
    pub end_color: [f32; 4],
    pub start_rotation: f32,
    pub angular_velocity: f32,
    pub gravity: f32,
    pub gravity_scale: f32,
    pub drag: f32,
    pub shape_mode: EmitterShape,
    pub shape_radius: f32,
    pub shape_angle: f32,
    pub shape_size: [f32; 3],
    pub render_mode: RenderMode,
    pub length_scale: f32,
    pub texture_path: String,
    pub tiles_x: u32,
    pub tiles_y: u32,
    pub frame_index: u32,
    pub preview_playing: bool,
    pub simulation_time: f32,
}

impl Default for ParticleEditorPanel {
    fn default() -> Self {
        Self {
            visible: false,
            emission_rate: 10.0,
            duration: 5.0,
            loop_enabled: true,
            burst_count: 0,
            burst_interval: 1.0,
            max_particles: 1000,
            lifetime_min: 2.0,
            lifetime_max: 4.0,
            start_speed: 1.0,
            start_size: 0.2,
            end_size: 0.0,
            start_color: [1.0, 0.8, 0.2, 1.0],
            end_color: [1.0, 0.2, 0.1, 0.0],
            start_rotation: 0.0,
            angular_velocity: 0.0,
            gravity: -9.81,
            gravity_scale: 1.0,
            drag: 0.1,
            shape_mode: EmitterShape::Point,
            shape_radius: 0.5,
            shape_angle: 45.0,
            shape_size: [1.0, 1.0, 1.0],
            render_mode: RenderMode::Billboard,
            length_scale: 2.0,
            texture_path: "None".to_string(),
            tiles_x: 1,
            tiles_y: 1,
            frame_index: 0,
            preview_playing: true,
            simulation_time: 0.0,
        }
    }
}

impl EditorPanel for ParticleEditorPanel {
    fn name(&self) -> &str { "Particle Editor" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }

        if self.preview_playing {
            let dt = ctx.input(|i| i.unstable_dt).min(0.05);
            self.simulation_time += dt;
            if self.loop_enabled && self.simulation_time > self.duration {
                self.simulation_time = self.simulation_time % self.duration.max(0.1);
            }
        }

        egui::Window::new("Particle Editor")
            .default_width(420.0)
            .default_height(620.0)
            .show(ctx, |ui| {
                ui.collapsing("Emitter", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Rate:");
                        ui.add(egui::Slider::new(&mut self.emission_rate, 0.0..=1000.0).suffix("/s"));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Duration:");
                        ui.add(egui::Slider::new(&mut self.duration, 0.1..=60.0).suffix("s"));
                    });
                    ui.checkbox(&mut self.loop_enabled, "Loop");
                    ui.horizontal(|ui| {
                        ui.label("Max Particles:");
                        ui.add(egui::DragValue::new(&mut self.max_particles).range(1..=100000));
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Burst Count:");
                        ui.add(egui::DragValue::new(&mut self.burst_count).range(0..=10000));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Burst Interval:");
                        ui.add(egui::Slider::new(&mut self.burst_interval, 0.0..=10.0).suffix("s"));
                    });
                });

                ui.collapsing("Lifecycle", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Lifetime Min:");
                        ui.add(egui::Slider::new(&mut self.lifetime_min, 0.1..=30.0).suffix("s"));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Lifetime Max:");
                        ui.add(egui::Slider::new(&mut self.lifetime_max, 0.1..=30.0).suffix("s"));
                    });
                    if self.lifetime_max < self.lifetime_min {
                        self.lifetime_max = self.lifetime_min;
                    }
                    ui.horizontal(|ui| {
                        ui.label("Start Speed:");
                        ui.add(egui::Slider::new(&mut self.start_speed, 0.0..=50.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Start Size:");
                        ui.add(egui::Slider::new(&mut self.start_size, 0.01..=10.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("End Size:");
                        ui.add(egui::Slider::new(&mut self.end_size, 0.0..=10.0));
                    });
                });

                ui.collapsing("Color Gradient", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Start Color:");
                        ui.color_edit_button_rgba_premultiplied(&mut self.start_color);
                    });
                    ui.horizontal(|ui| {
                        ui.label("End Color:");
                        ui.color_edit_button_rgba_premultiplied(&mut self.end_color);
                    });
                    ui.separator();
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(340.0, 22.0), egui::Sense::hover());
                    let painter = ui.painter();
                    let steps = 48;
                    for i in 0..steps {
                        let t = i as f32 / steps as f32;
                        let r = self.start_color[0] * (1.0 - t) + self.end_color[0] * t;
                        let g = self.start_color[1] * (1.0 - t) + self.end_color[1] * t;
                        let b = self.start_color[2] * (1.0 - t) + self.end_color[2] * t;
                        let a = self.start_color[3] * (1.0 - t) + self.end_color[3] * t;
                        let x0 = rect.left() + (i as f32 / steps as f32) * rect.width();
                        let x1 = rect.left() + ((i + 1) as f32 / steps as f32) * rect.width();
                        let sub = egui::Rect::from_min_max(egui::pos2(x0, rect.top()), egui::pos2(x1, rect.bottom()));
                        painter.rect_filled(sub, 0.0, egui::Rgba::from_rgba_premultiplied(r, g, b, a));
                    }
                    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::DARK_GRAY), egui::StrokeKind::Middle);
                });

                ui.collapsing("Rotation", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Start Rotation:");
                        ui.add(egui::Slider::new(&mut self.start_rotation, 0.0..=360.0).suffix(" deg"));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Angular Velocity:");
                        ui.add(egui::Slider::new(&mut self.angular_velocity, -360.0..=360.0).suffix(" deg/s"));
                    });
                });

                ui.collapsing("Physics", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Gravity:");
                        ui.add(egui::DragValue::new(&mut self.gravity).speed(0.1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Gravity Scale:");
                        ui.add(egui::Slider::new(&mut self.gravity_scale, -5.0..=5.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Drag:");
                        ui.add(egui::Slider::new(&mut self.drag, 0.0..=1.0));
                    });
                });

                ui.collapsing("Shape Emitter", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Shape:");
                        egui::ComboBox::from_id_salt("shape_combo")
                            .selected_text(match self.shape_mode {
                                EmitterShape::Point => "Point",
                                EmitterShape::Box => "Box",
                                EmitterShape::Sphere => "Sphere",
                                EmitterShape::Cone => "Cone",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.shape_mode, EmitterShape::Point, "Point");
                                ui.selectable_value(&mut self.shape_mode, EmitterShape::Box, "Box");
                                ui.selectable_value(&mut self.shape_mode, EmitterShape::Sphere, "Sphere");
                                ui.selectable_value(&mut self.shape_mode, EmitterShape::Cone, "Cone");
                            });
                    });
                    match self.shape_mode {
                        EmitterShape::Point => {
                            ui.label("Point emitter: particles spawn at origin");
                        }
                        EmitterShape::Box => {
                            ui.horizontal(|ui| {
                                ui.label("Size X:");
                                ui.add(egui::DragValue::new(&mut self.shape_size[0]).speed(0.1).range(0.0..=100.0));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Size Y:");
                                ui.add(egui::DragValue::new(&mut self.shape_size[1]).speed(0.1).range(0.0..=100.0));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Size Z:");
                                ui.add(egui::DragValue::new(&mut self.shape_size[2]).speed(0.1).range(0.0..=100.0));
                            });
                        }
                        EmitterShape::Sphere => {
                            ui.horizontal(|ui| {
                                ui.label("Radius:");
                                ui.add(egui::DragValue::new(&mut self.shape_radius).speed(0.1).range(0.0..=100.0));
                            });
                        }
                        EmitterShape::Cone => {
                            ui.horizontal(|ui| {
                                ui.label("Radius:");
                                ui.add(egui::DragValue::new(&mut self.shape_radius).speed(0.1).range(0.0..=100.0));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Angle:");
                                ui.add(egui::Slider::new(&mut self.shape_angle, 0.0..=180.0).suffix(" deg"));
                            });
                        }
                    }
                });

                ui.collapsing("Rendering", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Render Mode:");
                        egui::ComboBox::from_id_salt("render_mode_combo")
                            .selected_text(match self.render_mode {
                                RenderMode::Billboard => "Billboard",
                                RenderMode::Stretched => "Stretched Billboard",
                                RenderMode::Mesh => "Mesh",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.render_mode, RenderMode::Billboard, "Billboard");
                                ui.selectable_value(&mut self.render_mode, RenderMode::Stretched, "Stretched Billboard");
                                ui.selectable_value(&mut self.render_mode, RenderMode::Mesh, "Mesh");
                            });
                    });
                    if self.render_mode == RenderMode::Stretched {
                        ui.horizontal(|ui| {
                            ui.label("Length Scale:");
                            ui.add(egui::Slider::new(&mut self.length_scale, 0.0..=10.0));
                        });
                    }
                });

                ui.collapsing("Texture", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Texture:");
                        ui.add(egui::TextEdit::singleline(&mut self.texture_path).desired_width(160.0));
                        if ui.button("Browse...").clicked() {}
                        if ui.button("Clear").clicked() {
                            self.texture_path = "None".to_string();
                        }
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Tiles X:");
                        ui.add(egui::DragValue::new(&mut self.tiles_x).range(1..=32));
                        ui.label("Tiles Y:");
                        ui.add(egui::DragValue::new(&mut self.tiles_y).range(1..=32));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Frame:");
                        ui.add(egui::DragValue::new(&mut self.frame_index).range(0..=1024));
                    });
                });

                ui.collapsing("Preview", |ui| {
                    ui.horizontal(|ui| {
                        if ui.button(if self.preview_playing { "Pause" } else { "Play" }).clicked() {
                            self.preview_playing = !self.preview_playing;
                        }
                        if ui.button("Reset").clicked() {
                            self.simulation_time = 0.0;
                        }
                        ui.label(format!("t = {:.2} s", self.simulation_time));
                    });
                    ui.separator();
                    egui::Frame::canvas(ui.style()).show(ui, |ui| {
                        ui.set_min_size(egui::vec2(380.0, 220.0));
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(380.0, 220.0), egui::Sense::hover());
                        let painter = ui.painter();
                        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(18, 18, 26));
                        for i in 0..=10 {
                            let x = rect.left() + (i as f32 / 10.0) * rect.width();
                            painter.line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())], egui::Stroke::new(0.5, egui::Color32::from_rgb(38, 38, 48)));
                            let y = rect.top() + (i as f32 / 10.0) * rect.height();
                            painter.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], egui::Stroke::new(0.5, egui::Color32::from_rgb(38, 38, 48)));
                        }
                        let origin = egui::pos2(rect.center().x, rect.bottom() - 20.0);
                        painter.circle_filled(origin, 4.0, egui::Color32::YELLOW);
                        let max_preview = 120usize;
                        let preview_rate = self.emission_rate.min(60.0).max(1.0);
                        let spawn_interval = 1.0 / preview_rate;
                        for i in 0..max_preview {
                            let spawn_time = self.simulation_time - i as f32 * spawn_interval;
                            if spawn_time < 0.0 { break; }
                            let age = i as f32 * spawn_interval;
                            let life = self.lifetime_min + (self.lifetime_max - self.lifetime_min) * ((spawn_time * 0.7).sin().abs());
                            if age > life { continue; }
                            let life_t = if life > 0.0 { age / life } else { 1.0 };
                            let seed = spawn_time * 13.7;
                            let spread = seed.sin() * 0.5;
                            let vx = self.start_speed * spread.sin();
                            let vy = self.start_speed * spread.cos();
                            let px = vx * age;
                            let py = vy * age + 0.5 * self.gravity * self.gravity_scale * age * age;
                            let cx = origin.x + px * 10.0;
                            let cy = origin.y - py * 10.0;
                            if cx < rect.left() || cx > rect.right() || cy < rect.top() || cy > rect.bottom() { continue; }
                            let size = self.start_size * (1.0 - life_t) + self.end_size * life_t;
                            let radius = (size * 20.0).max(0.5);
                            let r = self.start_color[0] * (1.0 - life_t) + self.end_color[0] * life_t;
                            let g = self.start_color[1] * (1.0 - life_t) + self.end_color[1] * life_t;
                            let b = self.start_color[2] * (1.0 - life_t) + self.end_color[2] * life_t;
                            let a = self.start_color[3] * (1.0 - life_t) + self.end_color[3] * life_t;
                            let color = egui::Color32::from_rgba_premultiplied(
                                (r * 255.0).clamp(0.0, 255.0) as u8,
                                (g * 255.0).clamp(0.0, 255.0) as u8,
                                (b * 255.0).clamp(0.0, 255.0) as u8,
                                (a * 255.0).clamp(0.0, 255.0) as u8,
                            );
                            painter.circle_filled(egui::pos2(cx, cy), radius, color);
                        }
                    });
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Save Preset").clicked() {}
                    if ui.button("Load Preset").clicked() {}
                    if ui.button("Reset to Default").clicked() {
                        let vis = self.visible;
                        *self = Self::default();
                        self.visible = vis;
                    }
                });
            });
    }
}
