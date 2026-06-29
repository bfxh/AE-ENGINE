use crate::WastelandWorld;
use godot::prelude::*;

struct SpeciesEntry {
    name: String,
    count: i64,
    growth_rate: f32,
    death_rate: f32,
}

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandEcology {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    carrying_capacity: f32,

    #[var]
    growth_rate: f32,

    #[var]
    mutation_rate: f32,

    #[var]
    ecosystem_count: i64,

    #[var]
    total_organisms: i64,

    #[var]
    population_count: i64,

    species_list: Vec<SpeciesEntry>,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandEcology {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            carrying_capacity: 1000.0,
            growth_rate: 0.05,
            mutation_rate: 0.001,
            ecosystem_count: 0,
            total_organisms: 0,
            population_count: 0,
            species_list: Vec::new(),
            base,
        }
    }

    fn ready(&mut self) {
        if let Some(parent) = self.base().get_parent() {
            if let Ok(world) = parent.try_cast::<WastelandWorld>() {
                self.world_ref = Some(world);
            }
        }
    }

    fn process(&mut self, _delta: f64) {
        self.sync_from_world();
    }
}

#[godot_api]
impl WastelandEcology {
    fn sync_from_world(&mut self) {
        if let Some(ref world) = self.world_ref {
            let data = world.bind().export_ecology_data();
            if let Some(v) = data.get("ecosystem_count") {
                self.ecosystem_count = v.to::<i64>();
            }
            if let Some(v) = data.get("total_organisms") {
                self.total_organisms = v.to::<i64>();
            }
            if let Some(v) = data.get("population_count") {
                self.population_count = v.to::<i64>();
            }
        }
    }

    #[func]
    fn get_ecosystem_stats(&self) -> Dictionary<Variant, Variant> {
        if let Some(ref world) = self.world_ref {
            let data = world.bind().export_ecology_data();
            return data;
        }
        dict! {
            "ecosystem_count" => self.ecosystem_count,
            "total_organisms" => self.total_organisms,
            "population_count" => self.population_count,
        }
    }

    #[func]
    fn get_all_species(&self) -> PackedStringArray {
        if let Some(ref world) = self.world_ref {
            let data = world.bind().export_ecology_data();
            let mut arr = PackedStringArray::new();
            if let Some(ecosystems) = data.get("ecosystems") {
                if let Ok(arr_var) = ecosystems.try_to::<Array<Variant>>() {
                    for eco in arr_var.iter_shared() {
                        if let Ok(eco_dict) = eco.try_to::<Dictionary<Variant, Variant>>() {
                            if let Some(name) = eco_dict.get("name") {
                                let name_str: GString = name.to();
                                arr.push(&name_str);
                            }
                        }
                    }
                }
            }
            return arr;
        }
        PackedStringArray::new()
    }

    #[func]
    fn add_species(
        &mut self,
        name: GString,
        initial_count: i64,
        growth_rate: f32,
        death_rate: f32,
    ) -> i64 {
        let entry =
            SpeciesEntry { name: name.to_string(), count: initial_count, growth_rate, death_rate };
        self.species_list.push(entry);
        self.population_count = self.species_list.len() as i64;
        (self.species_list.len() - 1) as i64
    }

    #[func]
    fn remove_species(&mut self, name: GString) -> bool {
        let target = name.to_string();
        if let Some(pos) = self.species_list.iter().position(|s| s.name == target) {
            self.species_list.remove(pos);
            self.population_count = self.species_list.len() as i64;
            true
        } else {
            false
        }
    }

    #[func]
    fn get_species_details(&self, name: GString) -> Dictionary<Variant, Variant> {
        let target = name.to_string();
        for s in &self.species_list {
            if s.name == target {
                let trend = s.growth_rate - s.death_rate;
                return dict! {
                    "name" => &name,
                    "count" => s.count,
                    "growth_rate" => s.growth_rate,
                    "death_rate" => s.death_rate,
                    "trend" => trend,
                    "carrying_capacity" => self.carrying_capacity,
                };
            }
        }
        dict! {}
    }

    #[func]
    fn get_population_trends(&self) -> Array<Variant> {
        let mut arr = Array::<Variant>::new();
        for s in &self.species_list {
            let trend = s.growth_rate - s.death_rate;
            let trend_label: &str = if trend > 0.01 {
                "increasing"
            } else if trend < -0.01 {
                "decreasing"
            } else {
                "stable"
            };
            let d: Dictionary<Variant, Variant> = dict! {
                "name" => s.name.clone().as_str(),
                "count" => s.count,
                "trend" => trend,
                "trend_label" => trend_label,
            };
            arr.push(&d);
        }
        arr
    }

    #[func]
    fn get_food_web_level(&self, name: GString) -> i64 {
        let target = name.to_string().to_lowercase();
        match target.as_str() {
            "grass" | "algae" | "plankton" | "tree" | "bush" | "moss" => 1,
            "rabbit" | "deer" | "sheep" | "cow" | "squirrel" | "mouse" | "caterpillar" => 2,
            "wolf" | "bear" | "hawk" | "eagle" | "lion" | "tiger" | "shark" | "human" => 3,
            _ => {
                if target.contains("plant") || target.contains("tree") || target.contains("grass") {
                    1
                } else if target.contains("predator") || target.contains("apex") {
                    3
                } else {
                    2
                }
            },
        }
    }

    #[func]
    fn get_biodiversity_index(&self) -> f32 {
        if self.species_list.is_empty() {
            return 0.0;
        }
        let total: i64 = self.species_list.iter().map(|s| s.count.max(0)).sum();
        if total == 0 {
            return 0.0;
        }
        let n = self.species_list.len() as f32;
        let mut shannon = 0.0f32;
        for s in &self.species_list {
            let p = s.count as f32 / total as f32;
            if p > 0.0 {
                shannon -= p * p.ln();
            }
        }
        let evenness = if n > 1.0 { shannon / n.ln() } else { 1.0 };
        (shannon * evenness).clamp(0.0, 10.0)
    }
}
