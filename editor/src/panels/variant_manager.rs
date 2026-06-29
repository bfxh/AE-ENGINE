//! 变体管理器面板：管理资产变体和属性绑定。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct VariantManagerPanel {
    pub visible: bool,
    pub variant_sets: Vec<VariantSet>,
    pub selected_set: Option<usize>,
    pub selected_variant: Option<usize>,
    pub capture_enabled: bool,
    pub auto_apply: bool,
    pub new_set_name: String,
}

#[derive(Debug, Clone)]
pub struct VariantSet {
    pub name: String,
    pub variants: Vec<Variant>,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub bindings: Vec<PropertyBinding>,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct PropertyBinding {
    pub object: String,
    pub property: String,
    pub value: String,
}

impl Default for VariantManagerPanel {
    fn default() -> Self {
        Self {
            visible: false,
            variant_sets: vec![
                VariantSet {
                    name: "Materials".into(),
                    variants: vec![
                        Variant { name: "Red".into(), bindings: vec![PropertyBinding { object: "Cube".into(), property: "Color".into(), value: "1,0,0,1".into() }], active: true },
                        Variant { name: "Blue".into(), bindings: vec![PropertyBinding { object: "Cube".into(), property: "Color".into(), value: "0,0,1,1".into() }], active: false },
                    ],
                },
            ],
            selected_set: Some(0),
            selected_variant: Some(0),
            capture_enabled: true,
            auto_apply: false,
            new_set_name: String::new(),
        }
    }
}

impl EditorPanel for VariantManagerPanel {
    fn name(&self) -> &str { "Variant Manager" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(700.0)
            .default_height(500.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("New Set:");
                    ui.text_edit_singleline(&mut self.new_set_name);
                    if ui.button("Add Set").clicked() && !self.new_set_name.is_empty() {
                        self.variant_sets.push(VariantSet { name: self.new_set_name.clone(), variants: vec![] });
                        self.new_set_name.clear();
                    }
                    ui.separator();
                    ui.checkbox(&mut self.capture_enabled, "Capture");
                    ui.checkbox(&mut self.auto_apply, "Auto Apply");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Variant Sets");
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for (i, set) in self.variant_sets.iter().enumerate() {
                                let selected = self.selected_set == Some(i);
                                if ui.selectable_label(selected, &set.name).clicked() {
                                    self.selected_set = Some(i);
                                    self.selected_variant = None;
                                }
                            }
                        });
                    });
                    ui.vertical(|ui| {
                        ui.label("Variants");
                        if let Some(set_idx) = self.selected_set {
                            if set_idx < self.variant_sets.len() {
                                ui.horizontal(|ui| {
                                    if ui.button("Add Variant").clicked() {
                                        let vlen = self.variant_sets[set_idx].variants.len();
                                        self.variant_sets[set_idx].variants.push(Variant { name: format!("Variant {}", vlen), bindings: vec![], active: false });
                                    }
                                });
                                let mut remove_idx: Option<usize> = None;
                                let mut apply_idx: Option<usize> = None;
                                let mut select_idx: Option<usize> = None;
                                let vcount = self.variant_sets[set_idx].variants.len();
                                for i in 0..vcount {
                                    let selected = self.selected_variant == Some(i);
                                    let label = format!("{} {}", if self.variant_sets[set_idx].variants[i].active { "*" } else { " " }, self.variant_sets[set_idx].variants[i].name);
                                    ui.horizontal(|ui| {
                                        if ui.selectable_label(selected, &label).clicked() {
                                            select_idx = Some(i);
                                        }
                                        if ui.button("Apply").clicked() {
                                            apply_idx = Some(i);
                                        }
                                        if ui.button("X").clicked() { remove_idx = Some(i); }
                                    });
                                }
                                if let Some(i) = select_idx { self.selected_variant = Some(i); }
                                if let Some(i) = apply_idx {
                                    for v in self.variant_sets[set_idx].variants.iter_mut() {
                                        v.active = false;
                                    }
                                    self.variant_sets[set_idx].variants[i].active = true;
                                }
                                if let Some(i) = remove_idx { self.variant_sets[set_idx].variants.remove(i); }
                            }
                        } else {
                            ui.label("Select a set");
                        }
                    });
                    ui.vertical(|ui| {
                        ui.label("Property Bindings");
                        if let (Some(set_idx), Some(var_idx)) = (self.selected_set, self.selected_variant) {
                            if set_idx < self.variant_sets.len() && var_idx < self.variant_sets[set_idx].variants.len() {
                                let variant = &mut self.variant_sets[set_idx].variants[var_idx];
                                ui.horizontal(|ui| {
                                    if ui.button("Add Binding").clicked() {
                                        variant.bindings.push(PropertyBinding { object: String::new(), property: String::new(), value: String::new() });
                                    }
                                });
                                let mut remove_idx: Option<usize> = None;
                                for (i, b) in variant.bindings.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.text_edit_singleline(&mut b.object);
                                        ui.text_edit_singleline(&mut b.property);
                                        ui.text_edit_singleline(&mut b.value);
                                        if ui.button("X").clicked() { remove_idx = Some(i); }
                                    });
                                }
                                if let Some(i) = remove_idx { variant.bindings.remove(i); }
                            }
                        } else {
                            ui.label("Select a variant");
                        }
                    });
                });
            });
    }
}
