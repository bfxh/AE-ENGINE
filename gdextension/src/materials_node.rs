use crate::WastelandWorld;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandMaterials {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    default_hardness: f32,

    #[var]
    corrosion_factor: f32,

    material_count: i64,
    #[allow(dead_code)]
    active_corrosion: f32,
    #[allow(dead_code)]
    fatigue_level: f32,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandMaterials {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            default_hardness: 0.5,
            corrosion_factor: 1.0,
            material_count: 0,
            active_corrosion: 0.0,
            fatigue_level: 0.0,
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
impl WastelandMaterials {
    fn sync_from_world(&mut self) {
        if let Some(ref world) = self.world_ref {
            let stats = world.bind().get_stats();
            self.material_count = stats.get("material_count").map(|v| v.to::<i64>()).unwrap_or(0);
        }
    }

    #[func]
    fn get_material_properties(&self, material: GString) -> Dictionary<Variant, Variant> {
        let m = material.to_string().to_lowercase();
        match m.as_str() {
            "iron" => dict! {
                "hardness" => 200.0f32,
                "density" => 7.874f32,
                "tensile_strength" => 540.0f32,
                "yield_strength" => 370.0f32,
                "elastic_modulus" => 210.0f32,
                "poisson_ratio" => 0.29f32,
                "fatigue_limit" => 250.0f32,
                "corrosion_resistance" => 0.3f32,
            },
            "steel" => dict! {
                "hardness" => 250.0f32,
                "density" => 7.85f32,
                "tensile_strength" => 760.0f32,
                "yield_strength" => 520.0f32,
                "elastic_modulus" => 200.0f32,
                "poisson_ratio" => 0.3f32,
                "fatigue_limit" => 350.0f32,
                "corrosion_resistance" => 0.5f32,
            },
            "concrete" => dict! {
                "hardness" => 50.0f32,
                "density" => 2.4f32,
                "tensile_strength" => 3.0f32,
                "compressive_strength" => 40.0f32,
                "elastic_modulus" => 30.0f32,
                "poisson_ratio" => 0.2f32,
                "fatigue_limit" => 15.0f32,
                "corrosion_resistance" => 0.7f32,
            },
            "wood" => dict! {
                "hardness" => 15.0f32,
                "density" => 0.7f32,
                "tensile_strength" => 100.0f32,
                "yield_strength" => 50.0f32,
                "elastic_modulus" => 12.0f32,
                "poisson_ratio" => 0.35f32,
                "fatigue_limit" => 30.0f32,
                "corrosion_resistance" => 0.1f32,
            },
            _ => dict! {
                "hardness" => self.default_hardness * 100.0f32,
                "density" => 2.0f32,
                "tensile_strength" => 100.0f32,
                "yield_strength" => 50.0f32,
                "elastic_modulus" => 50.0f32,
                "poisson_ratio" => 0.3f32,
                "fatigue_limit" => 50.0f32,
                "corrosion_resistance" => 0.5f32,
            },
        }
    }

    #[func]
    fn compute_fatigue_damage(&self, stress: f32, cycles: i64, fatigue_limit: f32) -> f32 {
        if stress < fatigue_limit {
            return 0.0;
        }
        let stress_range = stress - fatigue_limit;
        let damage_per_cycle = (stress_range / fatigue_limit).powi(3) * 1e-6;
        (damage_per_cycle * cycles as f32).min(1.0)
    }

    #[func]
    fn compute_creep_strain(
        &self,
        stress: f32,
        temperature: f32,
        time: f32,
        elastic_modulus: f32,
    ) -> f32 {
        let melting_point = if temperature > 1500.0 {
            1800.0f32
        } else if temperature > 800.0 {
            1200.0f32
        } else {
            800.0f32
        };
        let homologous_temp = temperature / melting_point;
        if homologous_temp < 0.4 {
            return 0.0;
        }
        let creep_rate =
            (stress / elastic_modulus.max(1.0)).powi(3) * homologous_temp.powi(5) * 1e-6;
        creep_rate * time
    }

    #[func]
    fn compute_corrosion_depth(&self, material: GString, time: f32, ph: f32) -> f32 {
        let props = self.get_material_properties(material);
        let resistance = props.get("corrosion_resistance").map(|v| v.to::<f32>()).unwrap_or(0.5);
        let corrosion_rate = (1.0 - resistance) * self.corrosion_factor;
        let ph_factor = if ph < 5.0 {
            (5.0 - ph) * 0.5 + 1.0
        } else if ph > 9.0 {
            (ph - 9.0) * 0.5 + 1.0
        } else {
            1.0
        };
        corrosion_rate * time * ph_factor * 0.001
    }

    #[func]
    fn compute_hardness_from_composition(&self, composition: Dictionary<Variant, Variant>) -> f32 {
        let mut total = 0.0f32;
        let mut weight = 0.0f32;
        for (key, val) in composition.iter_shared() {
            let mat = GString::from(key.to_string().as_str());
            let fraction = val.to::<f32>();
            let props = self.get_material_properties(mat);
            let h = props.get("hardness").map(|v| v.to::<f32>()).unwrap_or(100.0);
            total += h * fraction;
            weight += fraction;
        }
        if weight > 0.0 { total / weight } else { self.default_hardness * 100.0 }
    }

    #[func]
    fn compute_structural_integrity(
        &self,
        material: GString,
        damage: f32,
        fatigue: f32,
        corrosion: f32,
    ) -> f32 {
        let props = self.get_material_properties(material);
        let tensile = props.get("tensile_strength").map(|v| v.to::<f32>()).unwrap_or(100.0);
        let effective = tensile * (1.0 - damage) * (1.0 - fatigue) * (1.0 - corrosion);
        effective.max(0.0)
    }

    #[func]
    fn get_material_count(&self) -> i64 {
        self.material_count
    }
}
