use crate::WastelandWorld;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandCrafting {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    creativity_factor: f32,

    #[var]
    precision: f32,

    known_derivations: i64,
    cache_hits: i64,
    cache_misses: i64,
    recipes: Vec<(GString, PackedStringArray, GString, f32)>,
    #[allow(dead_code)]
    material_pairs: Vec<(GString, GString, f32)>,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandCrafting {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            creativity_factor: 1.0,
            precision: 0.9,
            known_derivations: 0,
            cache_hits: 0,
            cache_misses: 0,
            recipes: Vec::new(),
            material_pairs: Vec::new(),
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
impl WastelandCrafting {
    fn sync_from_world(&mut self) {
        if let Some(ref world) = self.world_ref {
            let data = world.bind().get_cache_stats();
            if let Some(v) = data.get("hit_count") {
                self.cache_hits = v.to::<i64>();
            }
            if let Some(v) = data.get("miss_count") {
                self.cache_misses = v.to::<i64>();
            }
        }
    }

    #[func]
    fn derive_function(
        &mut self,
        material: GString,
        shape: GString,
        mass: f32,
    ) -> Dictionary<Variant, Variant> {
        let mat_str = material.to_string();
        let shape_str = shape.to_string();
        let hash = mat_str.len() as u64 + shape_str.len() as u64 + (mass * 1000.0) as u64;
        let function_name = GString::from(format!("fn_{}", hash % 10000).as_str());
        let confidence = (self.precision * self.creativity_factor * 0.5 + 0.5).min(1.0);
        let mut possible_uses = PackedStringArray::new();
        possible_uses.push(&GString::from("structural"));
        possible_uses.push(&GString::from("decorative"));
        if mass > 5.0 {
            possible_uses.push(&GString::from("heavy_duty"));
        }
        if self.precision > 0.8 {
            possible_uses.push(&GString::from("precision_tool"));
        }
        self.known_derivations += 1;
        self.cache_misses += 1;
        dict! {
            "function_name" => &function_name,
            "confidence" => confidence,
            "possible_uses" => &possible_uses,
        }
    }

    #[func]
    fn evaluate_craft(
        &self,
        material_a: GString,
        material_b: GString,
        connection_type: GString,
    ) -> Dictionary<Variant, Variant> {
        let a_len = material_a.to_string().len() as f32;
        let b_len = material_b.to_string().len() as f32;
        let c_len = connection_type.to_string().len() as f32;
        let feasibility =
            (self.precision * 0.7 + 0.3 * (a_len + b_len) / (a_len + b_len + c_len + 1.0)).min(1.0);
        let strength = (self.creativity_factor * 0.5 + self.precision * 0.3 + 0.2).min(1.0);
        let durability = (self.precision * 0.6 + 0.4).min(1.0);
        dict! {
            "feasibility" => feasibility,
            "strength" => strength,
            "durability" => durability,
        }
    }

    #[func]
    fn get_known_derivations(&self) -> i64 {
        self.known_derivations
    }

    #[func]
    fn get_cache_stats(&self) -> Dictionary<Variant, Variant> {
        let total = self.cache_hits + self.cache_misses;
        let hit_rate = if total > 0 { self.cache_hits as f32 / total as f32 } else { 0.0 };
        dict! {
            "hits" => self.cache_hits,
            "misses" => self.cache_misses,
            "hit_rate" => hit_rate,
        }
    }

    #[func]
    fn suggest_improvement(&self, item_description: GString) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        let desc = item_description.to_string().to_lowercase();
        if desc.contains("wood") || desc.contains("木") {
            arr.push(&GString::from("apply_waterproof_coating"));
        }
        if desc.contains("metal") || desc.contains("金属") {
            arr.push(&GString::from("temper_treatment"));
            arr.push(&GString::from("rust_proofing"));
        }
        if self.creativity_factor > 0.8 {
            arr.push(&GString::from("experimental_alloy_mix"));
        }
        if arr.is_empty() {
            arr.push(&GString::from("reinforce_structure"));
        }
        arr
    }

    #[func]
    fn add_recipe(
        &mut self,
        name: GString,
        inputs_array: PackedStringArray,
        output: GString,
        difficulty: f32,
    ) -> bool {
        let name_str = name.to_string();
        if name_str.is_empty() || inputs_array.is_empty() {
            return false;
        }
        if self.recipes.iter().any(|(n, _, _, _)| n.to_string() == name_str) {
            return false;
        }
        self.recipes.push((name, inputs_array, output, difficulty.clamp(0.0, 1.0)));
        true
    }

    #[func]
    fn remove_recipe(&mut self, name: GString) -> bool {
        let name_str = name.to_string();
        let idx = self.recipes.iter().position(|(n, _, _, _)| n.to_string() == name_str);
        if let Some(i) = idx {
            self.recipes.remove(i);
            return true;
        }
        false
    }

    #[func]
    fn get_all_recipes(&self) -> Array<Variant> {
        let mut arr = Array::<Variant>::new();
        for (name, inputs, output, difficulty) in &self.recipes {
            let d: Dictionary<Variant, Variant> = dict! {
                "name" => name,
                "inputs" => inputs,
                "output" => output,
                "difficulty" => *difficulty,
            };
            arr.push(&Variant::from(d));
        }
        arr
    }

    #[func]
    fn discover_recipe(
        &mut self,
        material_a: GString,
        material_b: GString,
        tool: GString,
    ) -> Dictionary<Variant, Variant> {
        let a_str = material_a.to_string();
        let b_str = material_b.to_string();
        let t_str = tool.to_string();
        let combo_seed = (a_str.len() + b_str.len() + t_str.len()) as f32;
        let discovered = combo_seed.sin() * self.creativity_factor > 0.3;
        let recipe_name = GString::from(format!("{}_{}_craft", a_str, b_str).as_str());
        let qual = self.precision * self.creativity_factor * 0.5 + 0.3;
        if discovered {
            let mut inputs = PackedStringArray::new();
            inputs.push(&material_a);
            inputs.push(&material_b);
            self.recipes.push((
                recipe_name.clone(),
                inputs,
                GString::from("discovered_item"),
                qual,
            ));
            self.known_derivations += 1;
        }
        dict! {
            "discovered" => discovered,
            "recipe_name" => &recipe_name,
            "quality" => qual,
            "tool_used" => &tool,
        }
    }

    #[func]
    fn get_recipe_quality(
        &self,
        material_quality: f32,
        tool_quality: f32,
        skill_level: f32,
    ) -> f32 {
        let mat = material_quality.clamp(0.0, 1.0);
        let tool = tool_quality.clamp(0.0, 1.0);
        let skill = skill_level.clamp(0.0, 1.0);
        (mat * 0.4 + tool * 0.3 + skill * 0.2 + self.precision * 0.1).min(1.0)
    }

    #[func]
    fn get_material_compatibility(
        &self,
        material_a: GString,
        material_b: GString,
    ) -> Dictionary<Variant, Variant> {
        let a_str = material_a.to_string();
        let b_str = material_b.to_string();
        let len_diff = (a_str.len() as f32 - b_str.len() as f32).abs();
        let sim = 1.0 / (1.0 + len_diff);
        let bond = (a_str.len() as f32 * 0.1 + b_str.len() as f32 * 0.1).sin().abs();
        let stability = (sim * 0.5 + bond * 0.3 + self.precision * 0.2).min(1.0);
        let mut suggestions = PackedStringArray::new();
        if stability > 0.7 {
            suggestions.push(&GString::from("direct_bonding"));
        }
        if stability > 0.4 {
            suggestions.push(&GString::from("adhesive_join"));
        }
        if stability < 0.3 {
            suggestions.push(&GString::from("mechanical_fastening"));
        }
        dict! {
            "compatibility" => stability,
            "similarity" => sim,
            "bond_strength" => bond,
            "suggestions" => &suggestions,
        }
    }
}
