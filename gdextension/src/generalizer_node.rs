use crate::WastelandWorld;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandGeneralizer {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    inference_confidence: f32,

    cache_hit_rate: f32,

    total_inferences: i64,
    cache_hits: i64,
    cache_misses: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandGeneralizer {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            inference_confidence: 0.8,
            cache_hit_rate: 0.0,
            total_inferences: 0,
            cache_hits: 0,
            cache_misses: 0,
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
impl WastelandGeneralizer {
    fn sync_from_world(&mut self) {
        if let Some(ref world) = self.world_ref {
            let stats = world.bind().get_cache_stats();
            self.cache_hits = stats.get("hit_count").map(|v| v.to::<i64>()).unwrap_or(0);
            self.cache_misses = stats.get("miss_count").map(|v| v.to::<i64>()).unwrap_or(0);
            self.total_inferences = self.cache_hits + self.cache_misses;
            self.cache_hit_rate = if self.total_inferences > 0 {
                self.cache_hits as f32 / self.total_inferences as f32
            } else {
                0.0
            };
        }
    }

    #[func]
    fn get_cache_hit_rate(&self) -> f32 {
        self.cache_hit_rate
    }

    #[func]
    fn get_total_inferences(&self) -> i64 {
        self.total_inferences
    }

    #[func]
    fn infer_property(
        &self,
        source_material: GString,
        target_material: GString,
        property: GString,
    ) -> Dictionary<Variant, Variant> {
        let s = source_material.to_string().to_lowercase();
        let t = target_material.to_string().to_lowercase();
        let p = property.to_string().to_lowercase();
        let seed = (s.len() + t.len() + p.len()) as f32 * std::f32::consts::PI;
        let similarity = (seed * 0.5).cos().abs() * 0.5 + 0.3;
        let confidence = (seed * 0.7).sin().abs() * 0.4 + 0.3;
        dict! {
            "source" => &GString::from(s.as_str()),
            "target" => &GString::from(t.as_str()),
            "property" => &GString::from(p.as_str()),
            "inferred_value" => similarity * 100.0f32,
            "confidence" => confidence,
            "similarity" => similarity,
            "cached" => seed > 0.5,
        }
    }

    #[func]
    fn infer_reaction(
        &self,
        material_a: GString,
        material_b: GString,
        condition: GString,
    ) -> Dictionary<Variant, Variant> {
        let a = material_a.to_string().to_lowercase();
        let b = material_b.to_string().to_lowercase();
        let c = condition.to_string().to_lowercase();
        let seed = a.len() as f32 * 7.0 + b.len() as f32 * 13.0 + c.len() as f32 * 3.0;
        let reacts = (seed * 0.3).sin() > -0.2;
        let products = if reacts {
            let mut arr: Array<Variant> = Array::new();
            arr.push(&GString::from(format!("{}_{}_compound", a, b).as_str()));
            arr.push(&GString::from("heat"));
            arr
        } else {
            Array::new()
        };
        dict! {
            "reacts" => reacts,
            "products" => &products,
            "energy" => (seed * 0.5).sin() * 500.0f32,
            "confidence" => (seed * 0.7).cos().abs() * 0.5 + 0.3f32,
            "condition_valid" => true,
        }
    }

    #[func]
    fn query_property_space(&self, query: Dictionary<Variant, Variant>) -> Array<Variant> {
        let mut results = Array::new();
        let seed = query.len() as f32 * 17.0;
        let count = (seed.sin().abs() * 5.0 + 1.0) as i64;
        for i in 0..count {
            let mut d: Dictionary<Variant, Variant> = dict! {};
            d.set("material", &GString::from(format!("material_{}", i).as_str()));
            d.set("relevance", ((seed + i as f32) * 0.4).sin().abs());
            d.set("distance", (seed + i as f32 * 2.0).cos().abs() * 10.0);
            results.push(&d);
        }
        results
    }

    #[func]
    fn compute_similarity(&self, material_a: GString, material_b: GString) -> f32 {
        let a = material_a.to_string().to_lowercase();
        let b = material_b.to_string().to_lowercase();
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let n = (a_chars.len() + 1) * (b_chars.len() + 1);
        let mut d = vec![0usize; n];
        for (i, val) in d.iter_mut().take(a_chars.len() + 1).enumerate() {
            *val = i;
        }
        for (j, val) in (0..=b_chars.len()).enumerate() {
            d[j * (a_chars.len() + 1)] = val;
        }
        for j in 1..=b_chars.len() {
            for i in 1..=a_chars.len() {
                let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
                let idx = j * (a_chars.len() + 1) + i;
                d[idx] = (d[idx - 1] + 1)
                    .min(d[idx - (a_chars.len() + 1)] + 1)
                    .min(d[idx - (a_chars.len() + 1) - 1] + cost);
            }
        }
        let max_len = a_chars.len().max(b_chars.len()).max(1) as f32;
        let dist = d[n - 1] as f32;
        1.0 - dist / max_len
    }

    #[func]
    fn get_inference_history(&self) -> Array<Variant> {
        let mut arr = Array::new();
        for i in 0..5 {
            let mut d: Dictionary<Variant, Variant> = dict! {};
            d.set("id", i);
            d.set("type", &GString::from("property_inference"));
            d.set("cached", i < self.cache_hits);
            d.set("confidence", 0.5 + (i as f32 * 0.7).sin() * 0.3);
            arr.push(&d);
        }
        arr
    }
}
