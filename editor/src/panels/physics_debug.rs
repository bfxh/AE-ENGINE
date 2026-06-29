use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct PhysicsDebugPanel {
    pub visible: bool,
    pub paused: bool,
    pub step_mode: bool,
    pub simulation_speed: f32,
    pub step_request: u32,
    pub gravity: [f32; 3],
    pub gravity_scale: f32,
    pub body_count: u32,
    pub active_body_count: u32,
    pub sleeping_body_count: u32,
    pub show_colliders: bool,
    pub show_box_colliders: bool,
    pub show_sphere_colliders: bool,
    pub show_capsule_colliders: bool,
    pub show_mesh_colliders: bool,
    pub show_contacts: bool,
    pub contact_count: u32,
    pub show_velocities: bool,
    pub velocity_scale: f32,
    pub show_aabbs: bool,
    pub show_constraints: bool,
    pub constraint_count: u32,
    pub show_grid: bool,
    pub friction: f32,
    pub restitution: f32,
    pub physics_frame_time: f32,
    pub collision_detection_time: f32,
    pub physics_fps: f32,
    pub max_frame_time: f32,
    pub collider_color: [f32; 4],
    pub contact_color: [f32; 4],
    pub velocity_color: [f32; 4],
    pub aabb_color: [f32; 4],
    pub constraint_color: [f32; 4],
}

impl Default for PhysicsDebugPanel {
    fn default() -> Self {
        Self {
            visible: false,
            paused: false,
            step_mode: false,
            simulation_speed: 1.0,
            step_request: 0,
            gravity: [0.0, -9.81, 0.0],
            gravity_scale: 1.0,
            body_count: 0,
            active_body_count: 0,
            sleeping_body_count: 0,
            show_colliders: true,
            show_box_colliders: true,
            show_sphere_colliders: true,
            show_capsule_colliders: true,
            show_mesh_colliders: false,
            show_contacts: true,
            contact_count: 0,
            show_velocities: false,
            velocity_scale: 1.0,
            show_aabbs: false,
            show_constraints: false,
            constraint_count: 0,
            show_grid: true,
            friction: 0.5,
            restitution: 0.3,
            physics_frame_time: 0.0,
            collision_detection_time: 0.0,
            physics_fps: 60.0,
            max_frame_time: 16.67,
            collider_color: [0.0, 1.0, 0.0, 1.0],
            contact_color: [1.0, 0.0, 0.0, 1.0],
            velocity_color: [0.0, 0.5, 1.0, 1.0],
            aabb_color: [1.0, 1.0, 0.0, 0.5],
            constraint_color: [0.5, 0.0, 1.0, 1.0],
        }
    }
}

impl EditorPanel for PhysicsDebugPanel {
    fn name(&self) -> &str { "Physics Debug" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new("Physics Debug")
            .default_width(360.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.collapsing("Simulation", |ui| {
                    ui.horizontal(|ui| {
                        let play_label = if self.paused { "Play" } else { "Pause" };
                        if ui.button(play_label).clicked() {
                            self.paused = !self.paused;
                        }
                        if ui.button("Step").clicked() {
                            self.step_request = self.step_request.saturating_add(1);
                        }
                        if ui.button("Reset").clicked() {
                            self.step_request = 0;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Speed:");
                        ui.add(egui::Slider::new(&mut self.simulation_speed, 0.0..=5.0).suffix("x"));
                    });
                    ui.checkbox(&mut self.step_mode, "Step Mode");
                    ui.horizontal(|ui| {
                        ui.label("Status:");
                        let status = if self.paused { "Paused" } else { "Running" };
                        let color = if self.paused {
                            egui::Color32::from_rgb(255, 150, 150)
                        } else {
                            egui::Color32::from_rgb(150, 255, 150)
                        };
                        ui.label(egui::RichText::new(status).color(color).strong());
                    });
                });

                ui.collapsing("Gravity", |ui| {
                    egui::Grid::new("physics_gravity_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                        ui.label("X:");
                        ui.add(egui::DragValue::new(&mut self.gravity[0]).speed(0.1).range(-50.0..=50.0));
                        ui.end_row();
                        ui.label("Y:");
                        ui.add(egui::DragValue::new(&mut self.gravity[1]).speed(0.1).range(-50.0..=50.0));
                        ui.end_row();
                        ui.label("Z:");
                        ui.add(egui::DragValue::new(&mut self.gravity[2]).speed(0.1).range(-50.0..=50.0));
                        ui.end_row();
                        ui.label("Scale:");
                        ui.add(egui::DragValue::new(&mut self.gravity_scale).speed(0.05).range(0.0..=5.0));
                        ui.end_row();
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Earth (9.81)").clicked() {
                            self.gravity = [0.0, -9.81, 0.0];
                        }
                        if ui.button("Moon (1.62)").clicked() {
                            self.gravity = [0.0, -1.62, 0.0];
                        }
                        if ui.button("Zero").clicked() {
                            self.gravity = [0.0, 0.0, 0.0];
                        }
                    });
                });

                ui.collapsing("Rigid Body Stats", |ui| {
                    egui::Grid::new("physics_body_stats_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                        ui.label("Total:");
                        ui.label(egui::RichText::new(format!("{}", self.body_count)).family(egui::FontFamily::Monospace).strong());
                        ui.end_row();
                        ui.label("Active:");
                        let active_color = if self.active_body_count > 0 {
                            egui::Color32::from_rgb(150, 255, 150)
                        } else {
                            egui::Color32::from_rgb(150, 150, 150)
                        };
                        ui.label(egui::RichText::new(format!("{}", self.active_body_count)).family(egui::FontFamily::Monospace).color(active_color));
                        ui.end_row();
                        ui.label("Sleeping:");
                        ui.label(egui::RichText::new(format!("{}", self.sleeping_body_count)).family(egui::FontFamily::Monospace).color(egui::Color32::from_rgb(150, 150, 200)));
                        ui.end_row();
                    });
                    let total = self.body_count.max(1);
                    let active_pct = (self.active_body_count as f32 / total as f32) * 100.0;
                    let sleeping_pct = (self.sleeping_body_count as f32 / total as f32) * 100.0;
                    ui.add_space(4.0);
                    ui.label(format!("Active: {:.1}%  Sleeping: {:.1}%", active_pct, sleeping_pct));
                    let bar_size = egui::vec2(ui.available_width(), 12.0);
                    let (bar_rect, _) = ui.allocate_exact_size(bar_size, egui::Sense::hover());
                    ui.painter().rect_filled(bar_rect, 2.0, egui::Color32::from_rgb(40, 40, 50));
                    if self.body_count > 0 {
                        let active_w = bar_rect.width() * (active_pct / 100.0);
                        let active_rect = egui::Rect::from_min_size(bar_rect.min, egui::vec2(active_w, bar_rect.height()));
                        ui.painter().rect_filled(active_rect, 2.0, egui::Color32::from_rgb(100, 200, 100));
                    }
                });

                ui.collapsing("Collider Visualization", |ui| {
                    ui.checkbox(&mut self.show_colliders, "Show All Colliders");
                    ui.separator();
                    ui.add_enabled_ui(self.show_colliders, |ui| {
                        ui.checkbox(&mut self.show_box_colliders, "Box");
                        ui.checkbox(&mut self.show_sphere_colliders, "Sphere");
                        ui.checkbox(&mut self.show_capsule_colliders, "Capsule");
                        ui.checkbox(&mut self.show_mesh_colliders, "Mesh");
                    });
                });

                ui.collapsing("Contact Points", |ui| {
                    ui.checkbox(&mut self.show_contacts, "Show Contacts");
                    ui.horizontal(|ui| {
                        ui.label("Contact Count:");
                        ui.label(egui::RichText::new(format!("{}", self.contact_count)).family(egui::FontFamily::Monospace).strong());
                    });
                });

                ui.collapsing("Velocity Vectors", |ui| {
                    ui.checkbox(&mut self.show_velocities, "Show Velocities");
                    ui.horizontal(|ui| {
                        ui.label("Scale:");
                        ui.add(egui::Slider::new(&mut self.velocity_scale, 0.0..=10.0).suffix("x"));
                    });
                });

                ui.collapsing("AABBs", |ui| {
                    ui.checkbox(&mut self.show_aabbs, "Show AABBs");
                });

                ui.collapsing("Constraints", |ui| {
                    ui.checkbox(&mut self.show_constraints, "Show Constraints");
                    ui.horizontal(|ui| {
                        ui.label("Count:");
                        ui.label(egui::RichText::new(format!("{}", self.constraint_count)).family(egui::FontFamily::Monospace).strong());
                    });
                });

                ui.collapsing("Physics Material", |ui| {
                    egui::Grid::new("physics_material_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                        ui.label("Friction:");
                        ui.add(egui::Slider::new(&mut self.friction, 0.0..=1.0));
                        ui.end_row();
                        ui.label("Restitution:");
                        ui.add(egui::Slider::new(&mut self.restitution, 0.0..=1.0));
                        ui.end_row();
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Ice").clicked() {
                            self.friction = 0.02;
                            self.restitution = 0.1;
                        }
                        if ui.button("Rubber").clicked() {
                            self.friction = 0.9;
                            self.restitution = 0.8;
                        }
                        if ui.button("Metal").clicked() {
                            self.friction = 0.3;
                            self.restitution = 0.2;
                        }
                        if ui.button("Default").clicked() {
                            self.friction = 0.5;
                            self.restitution = 0.3;
                        }
                    });
                });

                ui.collapsing("Performance", |ui| {
                    egui::Grid::new("physics_perf_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                        ui.label("Frame Time:");
                        let ft_color = if self.physics_frame_time > 16.67 {
                            egui::Color32::from_rgb(255, 150, 150)
                        } else if self.physics_frame_time > 8.0 {
                            egui::Color32::from_rgb(255, 220, 120)
                        } else {
                            egui::Color32::from_rgb(150, 255, 150)
                        };
                        ui.label(egui::RichText::new(format!("{:.3} ms", self.physics_frame_time)).family(egui::FontFamily::Monospace).color(ft_color));
                        ui.end_row();
                        ui.label("Collision Time:");
                        ui.label(egui::RichText::new(format!("{:.3} ms", self.collision_detection_time)).family(egui::FontFamily::Monospace));
                        ui.end_row();
                        ui.label("Physics FPS:");
                        ui.label(egui::RichText::new(format!("{:.1}", self.physics_fps)).family(egui::FontFamily::Monospace).strong());
                        ui.end_row();
                        ui.label("Max Frame Time:");
                        ui.label(egui::RichText::new(format!("{:.3} ms", self.max_frame_time)).family(egui::FontFamily::Monospace));
                        ui.end_row();
                    });
                    if self.physics_frame_time > 0.0 {
                        ui.add_space(4.0);
                        ui.label("Frame Time Budget:");
                        let budget_pct = (self.physics_frame_time / 16.67) * 100.0;
                        let bar_size = egui::vec2(ui.available_width(), 10.0);
                        let (bar_rect, _) = ui.allocate_exact_size(bar_size, egui::Sense::hover());
                        ui.painter().rect_filled(bar_rect, 2.0, egui::Color32::from_rgb(40, 40, 50));
                        let fill_pct = budget_pct.min(100.0) / 100.0;
                        let fill_rect = egui::Rect::from_min_size(bar_rect.min, egui::vec2(bar_rect.width() * fill_pct, bar_rect.height()));
                        let fill_color = if budget_pct > 100.0 {
                            egui::Color32::from_rgb(255, 80, 80)
                        } else if budget_pct > 60.0 {
                            egui::Color32::from_rgb(255, 200, 80)
                        } else {
                            egui::Color32::from_rgb(100, 200, 100)
                        };
                        ui.painter().rect_filled(fill_rect, 2.0, fill_color);
                    }
                });

                ui.collapsing("Overlay Colors", |ui| {
                    egui::Grid::new("physics_color_grid").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                        ui.label("Colliders:");
                        ui.color_edit_button_rgba_premultiplied(&mut self.collider_color);
                        ui.end_row();
                        ui.label("Contacts:");
                        ui.color_edit_button_rgba_premultiplied(&mut self.contact_color);
                        ui.end_row();
                        ui.label("Velocities:");
                        ui.color_edit_button_rgba_premultiplied(&mut self.velocity_color);
                        ui.end_row();
                        ui.label("AABBs:");
                        ui.color_edit_button_rgba_premultiplied(&mut self.aabb_color);
                        ui.end_row();
                        ui.label("Constraints:");
                        ui.color_edit_button_rgba_premultiplied(&mut self.constraint_color);
                        ui.end_row();
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Reset Colors").clicked() {
                            self.collider_color = [0.0, 1.0, 0.0, 1.0];
                            self.contact_color = [1.0, 0.0, 0.0, 1.0];
                            self.velocity_color = [0.0, 0.5, 1.0, 1.0];
                            self.aabb_color = [1.0, 1.0, 0.0, 0.5];
                            self.constraint_color = [0.5, 0.0, 1.0, 1.0];
                        }
                    });
                });

                ui.collapsing("Grid", |ui| {
                    ui.checkbox(&mut self.show_grid, "Show Grid");
                });
            });
    }
}
