use crate::WastelandWorld;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandAxiom {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    strict_mode: bool,

    #[var]
    auto_resolve: bool,

    axiom_count: i64,
    active_forks: i64,
    axioms: Vec<(GString, GString, f32)>,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandAxiom {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            strict_mode: true,
            auto_resolve: false,
            axiom_count: 0,
            active_forks: 0,
            axioms: Vec::new(),
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
impl WastelandAxiom {
    fn sync_from_world(&mut self) {
        if let Some(ref world) = self.world_ref {
            let data = world.bind().get_stats();
            if let Some(v) = data.get("meta_entity_count") {
                let count = v.to::<i64>();
                self.axiom_count = count;
                self.active_forks = (count / 2).max(0);
            }
        }
    }

    #[func]
    fn propose_axiom(&mut self, name: GString, formula: GString, confidence: f32) -> bool {
        if confidence <= 0.0 || name.to_string().is_empty() {
            return false;
        }
        self.axioms.push((name, formula, confidence));
        self.axiom_count = self.axioms.len() as i64;
        true
    }

    #[func]
    fn revoke_axiom(&mut self, name: GString) -> bool {
        let name_str = name.to_string();
        let idx = self.axioms.iter().position(|(n, _, _)| n.to_string() == name_str);
        if let Some(i) = idx {
            self.axioms.remove(i);
            self.axiom_count = self.axioms.len() as i64;
            return true;
        }
        false
    }

    #[func]
    fn get_axiom_count(&self) -> i64 {
        self.axiom_count
    }

    #[func]
    fn get_all_axioms(&self) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        for (name, _, _) in &self.axioms {
            arr.push(name);
        }
        arr
    }

    #[func]
    fn check_consistency(&self) -> bool {
        if self.strict_mode { self.axioms.len() <= 10 } else { true }
    }

    #[func]
    fn get_active_forks(&self) -> i64 {
        self.active_forks
    }

    #[func]
    fn resolve_conflict(&mut self, fork_id: i64) -> bool {
        if fork_id >= 0 && (fork_id as usize) < self.axioms.len() && self.auto_resolve {
            self.axioms.remove(fork_id as usize);
            self.axiom_count = self.axioms.len() as i64;
            self.active_forks = (self.active_forks - 1).max(0);
            return true;
        }
        false
    }

    #[func]
    fn get_axiom_details(&self, name: GString) -> Dictionary<Variant, Variant> {
        let name_str = name.to_string();
        for (n, f, c) in &self.axioms {
            if n.to_string() == name_str {
                let formula: GString = f.clone();
                return dict! {
                    "name" => &formula,
                    "formula" => n,
                    "confidence" => *c,
                    "active" => true,
                };
            }
        }
        dict! {}
    }

    #[func]
    fn validate_axiom(&self, name: GString) -> Dictionary<Variant, Variant> {
        let name_str = name.to_string();
        let mut conflicts = PackedStringArray::new();
        let mut implications = PackedStringArray::new();
        let mut found = false;
        let mut confidence = 0.0f32;
        for (n, _, c) in &self.axioms {
            if n.to_string() == name_str {
                found = true;
                confidence = *c;
                continue;
            }
            let n_str = n.to_string();
            if n_str.contains(&name_str) || name_str.contains(&n_str) {
                conflicts.push(&GString::from(n_str.as_str()));
            }
            if self.strict_mode && n_str.len() % 2 == name_str.len() % 2 {
                implications.push(&GString::from(n_str.as_str()));
            }
        }
        dict! {
            "valid" => found,
            "confidence" => confidence,
            "conflicts" => &conflicts,
            "implications" => &implications,
        }
    }

    #[func]
    fn get_axiom_dependencies(&self, name: GString) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        let name_str = name.to_string();
        for (n, _, _) in &self.axioms {
            let n_str = n.to_string();
            if n_str != name_str && n_str.contains(&name_str) {
                arr.push(n);
            }
        }
        arr
    }

    #[func]
    fn derive_theorem(&self, hypothesis: GString, steps: i64) -> Dictionary<Variant, Variant> {
        let hyp_str = hypothesis.to_string();
        let h_len = hyp_str.len() as f32;
        let mut derived = PackedStringArray::new();
        let mut grounded = false;
        let max_steps = steps.clamp(1, 20) as usize;
        for (i, (name, formula, _)) in self.axioms.iter().enumerate() {
            if i >= max_steps {
                break;
            }
            let f_str = formula.to_string();
            if f_str.contains(&hyp_str) || hyp_str.contains(&f_str) {
                derived.push(name);
                grounded = true;
            }
        }
        let confidence = if grounded && !derived.is_empty() {
            (0.5 + derived.len() as f32 / max_steps as f32 * 0.5).min(1.0)
        } else {
            (h_len * 0.01).min(0.3)
        };
        dict! {
            "derived" => !derived.is_empty(),
            "theorems" => &derived,
            "confidence" => confidence,
            "grounded" => grounded,
        }
    }

    #[func]
    fn get_axiom_confidence_distribution(&self) -> PackedFloat32Array {
        let mut arr = PackedFloat32Array::new();
        for (_, _, c) in &self.axioms {
            arr.push(*c);
        }
        arr
    }

    #[func]
    fn set_axiom_confidence(&mut self, name: GString, confidence: f32) -> bool {
        let name_str = name.to_string();
        for (n, _, c) in &mut self.axioms {
            if n.to_string() == name_str {
                *c = confidence.clamp(0.0, 1.0);
                return true;
            }
        }
        false
    }

    #[func]
    fn export_axiom_system(&self) -> GString {
        let mut parts: Vec<String> = Vec::new();
        for (name, formula, confidence) in &self.axioms {
            let name_str = name.to_string();
            let formula_str = formula.to_string();
            parts.push(format!(
                r#"{{"name":"{}","formula":"{}","confidence":{:.3}}}"#,
                name_str,
                formula_str,
                confidence
            ));
        }
        GString::from(format!("[{}]", parts.join(",")).as_str())
    }
}
