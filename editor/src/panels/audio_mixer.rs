//! 音频混合器面板：音频总线和效果控制。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct AudioMixerPanel {
    pub visible: bool,
    pub buses: Vec<AudioBus>,
    pub selected_bus: Option<usize>,
    pub master_volume: f32,
    pub snapshots: Vec<String>,
    pub current_snapshot: Option<usize>,
    pub show_meters: bool,
    pub show_effects: bool,
}

#[derive(Debug, Clone)]
pub struct AudioBus {
    pub name: String,
    pub volume: f32,
    pub muted: bool,
    pub solo: bool,
    pub parent: Option<usize>,
    pub effects: Vec<AudioEffect>,
    pub meter_level: f32,
}

#[derive(Debug, Clone)]
pub struct AudioEffect {
    pub name: String,
    pub effect_type: EffectType,
    pub enabled: bool,
    pub mix: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EffectType {
    Reverb,
    Delay,
    EQ,
    Compressor,
    Distortion,
}

impl Default for AudioMixerPanel {
    fn default() -> Self {
        Self {
            visible: false,
            buses: vec![
                AudioBus { name: "Master".into(), volume: 1.0, muted: false, solo: false, parent: None, effects: vec![], meter_level: 0.8 },
                AudioBus { name: "Music".into(), volume: 0.7, muted: false, solo: false, parent: Some(0), effects: vec![AudioEffect { name: "Reverb".into(), effect_type: EffectType::Reverb, enabled: true, mix: 0.3 }], meter_level: 0.5 },
                AudioBus { name: "SFX".into(), volume: 0.8, muted: false, solo: false, parent: Some(0), effects: vec![AudioEffect { name: "Compressor".into(), effect_type: EffectType::Compressor, enabled: true, mix: 1.0 }], meter_level: 0.6 },
                AudioBus { name: "Voice".into(), volume: 1.0, muted: false, solo: false, parent: Some(0), effects: vec![], meter_level: 0.4 },
            ],
            selected_bus: Some(0),
            master_volume: 1.0,
            snapshots: vec!["Default".into(), "Combat".into(), "Menu".into()],
            current_snapshot: Some(0),
            show_meters: true,
            show_effects: true,
        }
    }
}

impl EditorPanel for AudioMixerPanel {
    fn name(&self) -> &str { "Audio Mixer" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(700.0)
            .default_height(500.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Master:");
                    ui.add(egui::Slider::new(&mut self.master_volume, 0.0..=1.0));
                    ui.separator();
                    ui.checkbox(&mut self.show_meters, "Meters");
                    ui.checkbox(&mut self.show_effects, "Effects");
                    ui.separator();
                    ui.label("Snapshot:");
                    egui::ComboBox::from_label("").selected_text(self.current_snapshot.and_then(|i| self.snapshots.get(i).cloned()).unwrap_or_default()).show_ui(ui, |ui| {
                        for (i, s) in self.snapshots.iter().enumerate() {
                            ui.selectable_value(&mut self.current_snapshot, Some(i), s);
                        }
                    });
                    if ui.button("Add Snapshot").clicked() {
                        self.snapshots.push(format!("Snapshot {}", self.snapshots.len()));
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Buses");
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for (i, bus) in self.buses.iter_mut().enumerate() {
                                let selected = self.selected_bus == Some(i);
                                let depth = bus.parent.map(|_| 1).unwrap_or(0);
                                let indent = "  ".repeat(depth);
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(selected, format!("{}{}", indent, bus.name)).clicked() {
                                        self.selected_bus = Some(i);
                                    }
                                    ui.checkbox(&mut bus.muted, "M");
                                    ui.checkbox(&mut bus.solo, "S");
                                    ui.add(egui::Slider::new(&mut bus.volume, 0.0..=1.0));
                                    if self.show_meters {
                                        let color = if bus.meter_level > 0.9 { egui::Color32::RED } else if bus.meter_level > 0.7 { egui::Color32::YELLOW } else { egui::Color32::GREEN };
                                        let (rect, _) = ui.allocate_exact_size(egui::vec2(60.0, 10.0), egui::Sense::hover());
                                        ui.painter().rect_filled(rect, 0.0, egui::Color32::DARK_GRAY);
                                        let mut r = rect;
                                        r.set_width(rect.width() * bus.meter_level);
                                        ui.painter().rect_filled(r, 0.0, color);
                                    }
                                });
                            }
                        });
                        if ui.button("Add Bus").clicked() {
                            self.buses.push(AudioBus { name: format!("Bus {}", self.buses.len()), volume: 1.0, muted: false, solo: false, parent: Some(0), effects: vec![], meter_level: 0.0 });
                        }
                    });
                    if self.show_effects {
                        ui.vertical(|ui| {
                            ui.label("Effects Chain");
                            ui.separator();
                            if let Some(bi) = self.selected_bus {
                                if bi < self.buses.len() {
                                    let bus = &mut self.buses[bi];
                                    let mut remove_idx: Option<usize> = None;
                                    for (ei, e) in bus.effects.iter_mut().enumerate() {
                                        ui.horizontal(|ui| {
                                            ui.checkbox(&mut e.enabled, "");
                                            ui.label(format!("{:?}", e.effect_type));
                                            ui.text_edit_singleline(&mut e.name);
                                            ui.add(egui::Slider::new(&mut e.mix, 0.0..=1.0));
                                            if ui.button("X").clicked() { remove_idx = Some(ei); }
                                        });
                                    }
                                    if let Some(i) = remove_idx { bus.effects.remove(i); }
                                    ui.separator();
                                    if ui.button("Add Reverb").clicked() {
                                        bus.effects.push(AudioEffect { name: "Reverb".into(), effect_type: EffectType::Reverb, enabled: true, mix: 0.5 });
                                    }
                                    if ui.button("Add EQ").clicked() {
                                        bus.effects.push(AudioEffect { name: "EQ".into(), effect_type: EffectType::EQ, enabled: true, mix: 1.0 });
                                    }
                                    if ui.button("Add Compressor").clicked() {
                                        bus.effects.push(AudioEffect { name: "Comp".into(), effect_type: EffectType::Compressor, enabled: true, mix: 1.0 });
                                    }
                                }
                            } else {
                                ui.label("Select a bus");
                            }
                        });
                    }
                });
            });
    }
}
