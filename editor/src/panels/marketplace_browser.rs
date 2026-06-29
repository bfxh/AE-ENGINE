//! 市场浏览器面板：分类、资产列表、预览、下载、搜索。

use crate::app::EditorApp;
use crate::panels::EditorPanel;

pub struct MarketplaceBrowserPanel {
    pub visible: bool,
    pub categories: Vec<String>,
    pub selected_category: usize,
    pub assets: Vec<MarketAsset>,
    pub search_query: String,
    pub selected_asset: Option<usize>,
    pub sort_by: SortBy,
    pub free_only: bool,
    pub download_progress: f32,
    pub downloading: bool,
}

#[derive(Debug, Clone)]
pub struct MarketAsset {
    pub name: String,
    pub category: String,
    pub author: String,
    pub rating: f32,
    pub price: f32,
    pub downloads: u32,
    pub description: String,
    pub downloaded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortBy {
    Popularity,
    Rating,
    PriceLow,
    PriceHigh,
    Newest,
}

impl Default for MarketplaceBrowserPanel {
    fn default() -> Self {
        Self {
            visible: false,
            categories: vec!["All".into(), "3D Models".into(), "Materials".into(), "Audio".into(), "Scripts".into(), "VFX".into()],
            selected_category: 0,
            assets: vec![
                MarketAsset { name: "Sci-Fi Pack".into(), category: "3D Models".into(), author: "ArtistA".into(), rating: 4.8, price: 29.99, downloads: 15234, description: "200+ sci-fi meshes".into(), downloaded: false },
                MarketAsset { name: "PBR Metal".into(), category: "Materials".into(), author: "MatLab".into(), rating: 4.5, price: 0.0, downloads: 45123, description: "50 metal materials".into(), downloaded: true },
                MarketAsset { name: "Footsteps Vol1".into(), category: "Audio".into(), author: "SoundCo".into(), rating: 4.2, price: 9.99, downloads: 8932, description: "300 footstep sounds".into(), downloaded: false },
                MarketAsset { name: "Fire VFX".into(), category: "VFX".into(), author: "FxHouse".into(), rating: 4.9, price: 14.99, downloads: 23456, description: "Realistic fire effects".into(), downloaded: false },
            ],
            search_query: String::new(),
            selected_asset: Some(0),
            sort_by: SortBy::Popularity,
            free_only: false,
            download_progress: 0.0,
            downloading: false,
        }
    }
}

impl EditorPanel for MarketplaceBrowserPanel {
    fn name(&self) -> &str { "Marketplace Browser" }

    fn render(&mut self, ctx: &egui::Context, _app: &mut EditorApp) {
        if !self.visible { return; }
        egui::Window::new(self.name())
            .default_width(600.0)
            .default_height(450.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.search_query);
                    ui.checkbox(&mut self.free_only, "Free only");
                    ui.label("Sort:");
                    egui::ComboBox::from_id_source("sort_combo")
                        .selected_text(format!("{:?}", self.sort_by))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.sort_by, SortBy::Popularity, "Popularity");
                            ui.selectable_value(&mut self.sort_by, SortBy::Rating, "Rating");
                            ui.selectable_value(&mut self.sort_by, SortBy::PriceLow, "Price: Low");
                            ui.selectable_value(&mut self.sort_by, SortBy::PriceHigh, "Price: High");
                            ui.selectable_value(&mut self.sort_by, SortBy::Newest, "Newest");
                        });
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Categories");
                        ui.separator();
                        for i in 0..self.categories.len() {
                            let selected = self.selected_category == i;
                            if ui.selectable_label(selected, &self.categories[i]).clicked() {
                                self.selected_category = i;
                            }
                        }
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.label("Assets");
                        ui.separator();
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for i in 0..self.assets.len() {
                                let cat_match = self.selected_category == 0
                                    || self.assets[i].category == self.categories[self.selected_category];
                                let search_match = self.search_query.is_empty()
                                    || self.assets[i].name.to_lowercase().contains(&self.search_query.to_lowercase());
                                let free_match = !self.free_only || self.assets[i].price == 0.0;
                                if !cat_match || !search_match || !free_match { continue; }
                                let selected = self.selected_asset == Some(i);
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(selected, &self.assets[i].name).clicked() {
                                        self.selected_asset = Some(i);
                                    }
                                    ui.label(format!("${:.2}", self.assets[i].price));
                                    ui.label(format!("({:.1}*)", self.assets[i].rating));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if self.assets[i].downloaded {
                                            ui.label("Downloaded");
                                        } else if ui.button("Download").clicked() {
                                            self.assets[i].downloaded = true;
                                        }
                                    });
                                });
                                ui.label(format!("  by {} - {} downloads", self.assets[i].author, self.assets[i].downloads));
                                ui.label(format!("  {}", self.assets[i].description));
                                ui.separator();
                            }
                        });
                    });
                });
                if self.downloading {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Downloading...");
                        ui.add(egui::ProgressBar::new(self.download_progress).show_percentage());
                    });
                }
            });
    }
}
