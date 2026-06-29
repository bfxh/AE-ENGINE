use crate::WastelandWorld;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandParticle {
    world_ref: Option<Gd<WastelandWorld>>,

    particle_count: i64,

    #[var]
    max_particles: i64,

    #[var]
    interaction_radius: f32,

    #[allow(dead_code)]
    phase_state: GString,
    #[allow(dead_code)]
    emergence_level: f32,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandParticle {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            particle_count: 0,
            max_particles: 10000,
            interaction_radius: 1.0,
            phase_state: GString::from("gas"),
            emergence_level: 0.0,
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
impl WastelandParticle {
    fn sync_from_world(&mut self) {
        if let Some(ref world) = self.world_ref {
            self.particle_count = world.bind().get_particle_count();
        }
    }

    #[func]
    fn get_particle_count(&self) -> i64 {
        self.particle_count
    }

    #[func]
    fn get_particle_positions(&self) -> Array<Variant> {
        let mut arr: Array<Variant> = Array::new();
        if let Some(ref world) = self.world_ref {
            let pos_arr = world.bind().get_particle_positions();
            let count = pos_arr.len();
            for i in 0..count {
                if let Some(pos) = pos_arr.get(i) {
                    arr.push(Vector3::new(pos.x, pos.y, pos.z));
                }
            }
        }
        arr
    }

    #[func]
    fn spawn_particles(&mut self, x: f32, y: f32, z: f32, count: i64, spread: f32) {
        if count <= 0 {
            return;
        }
        if let Some(ref mut world) = self.world_ref {
            world.bind_mut().spawn_iron_particles(x, y, z, count, spread, false);
        }
    }

    #[func]
    fn get_phase_transition(&self, temperature: f32, pressure: f32) -> GString {
        if temperature > 500.0 && pressure < 0.1 {
            GString::from("plasma")
        } else if temperature > 373.0 && pressure < 1.0 {
            GString::from("gas")
        } else if temperature > 273.0 {
            GString::from("liquid")
        } else if pressure > 100.0 {
            GString::from("metallic")
        } else {
            GString::from("solid")
        }
    }

    #[func]
    fn compute_self_organization(&self, positions: PackedFloat32Array) -> f32 {
        let n = positions.len() / 3;
        if n < 2 {
            return 0.0;
        }
        let limit = n.min(100);
        if n > 100 {
            godot_print!("compute_self_organization: truncating from {} to {} particles", n, limit);
        }
        let mut total_dist = 0.0f32;
        let mut pair_count = 0;
        for i in 0..(n.min(100)) {
            let xi = positions.get(i * 3).unwrap_or(0.0);
            let yi = positions.get(i * 3 + 1).unwrap_or(0.0);
            let zi = positions.get(i * 3 + 2).unwrap_or(0.0);
            for j in (i + 1)..(n.min(100)) {
                let dx = xi - positions.get(j * 3).unwrap_or(0.0);
                let dy = yi - positions.get(j * 3 + 1).unwrap_or(0.0);
                let dz = zi - positions.get(j * 3 + 2).unwrap_or(0.0);
                total_dist += (dx * dx + dy * dy + dz * dz).sqrt();
                pair_count += 1;
            }
        }
        if pair_count == 0 {
            return 0.0;
        }
        let avg_dist = total_dist / pair_count as f32;
        (1.0 / (avg_dist + 0.01)).min(1.0)
    }

    #[func]
    fn get_emergent_patterns(&self) -> Array<Variant> {
        let mut arr = Array::new();
        let patterns = ["cluster", "chain", "ring", "lattice", "vortex", "wave"];
        for (i, p) in patterns.iter().enumerate() {
            let mut d: Dictionary<Variant, Variant> = dict! {};
            d.set("pattern", &GString::from(*p));
            d.set("probability", (i as f32 * 0.7).sin().abs() * 0.3 + 0.1);
            d.set("stability", (i as f32 * 1.3).cos().abs() * 0.5 + 0.3);
            arr.push(&d);
        }
        arr
    }

    #[func]
    fn get_interaction_matrix(&self) -> Dictionary<Variant, Variant> {
        dict! {
            "attract_attract" => 1.0f32,
            "attract_repel" => -1.0f32,
            "repel_repel" => 0.5f32,
            "neutral_neutral" => 0.0f32,
            "alignment_strength" => 0.3f32,
            "cohesion_strength" => 0.5f32,
            "separation_strength" => 0.7f32,
        }
    }
}
