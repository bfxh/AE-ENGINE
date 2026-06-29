use crate::WastelandWorld;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandBiology {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    mutation_rate: f32,

    #[var]
    generation: i64,

    species_count: i64,
    organism_count: i64,
    dominant_species: GString,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandBiology {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            mutation_rate: 0.001,
            generation: 0,
            species_count: 0,
            organism_count: 0,
            dominant_species: GString::from("none"),
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
impl WastelandBiology {
    fn sync_from_world(&mut self) {
        if let Some(ref world) = self.world_ref {
            let stats = world.bind().get_stats();
            self.species_count = stats.get("ecosystem_count").map(|v| v.to::<i64>()).unwrap_or(0);
            self.organism_count = stats.get("total_organisms").map(|v| v.to::<i64>()).unwrap_or(0);
            if self.species_count > 0 {
                let eco_data = world.bind().export_ecology_data();
                self.dominant_species = eco_data
                    .get("dominant_species")
                    .map(|v| GString::from(v.to_string().as_str()))
                    .unwrap_or(GString::from("unknown"));
            }
        }
    }

    #[func]
    fn get_species_count(&self) -> i64 {
        self.species_count
    }

    #[func]
    fn get_organism_count(&self) -> i64 {
        self.organism_count
    }

    #[func]
    fn get_dominant_species(&self) -> GString {
        self.dominant_species.clone()
    }

    #[func]
    fn get_species_info(&self, index: i64) -> Dictionary<Variant, Variant> {
        if index < 0 || index >= self.species_count {
            return dict! {};
        }
        let species_names =
            ["human", "wolf", "deer", "rabbit", "fox", "bear", "eagle", "salmon", "oak", "pine"];
        let name = species_names.get(index as usize).unwrap_or(&"unknown");
        let pop = if self.species_count > 0 {
            self.organism_count / self.species_count.max(1)
        } else {
            0
        };
        let name_str = GString::from(*name);
        dict! {
            "species" => &name_str,
            "population" => pop,
            "positions" => &Array::<Variant>::new(),
        }
    }

    #[func]
    fn get_genome_sequence(&self, species: GString) -> GString {
        let s = species.to_string().to_lowercase();
        let seed = s.len() as u64 * 7 + s.bytes().fold(0u64, |a, b| a + b as u64);
        let bases = ['A', 'T', 'G', 'C'];
        let mut seq = String::with_capacity(64);
        for i in 0..64 {
            let idx = ((seed.wrapping_mul(13 + i as u64).wrapping_add(i as u64 * 7)) % 4) as usize;
            seq.push(bases[idx]);
        }
        GString::from(seq.as_str())
    }

    #[func]
    fn compute_genetic_distance(&self, species_a: GString, species_b: GString) -> f32 {
        let seq_a = self.get_genome_sequence(species_a).to_string();
        let seq_b = self.get_genome_sequence(species_b).to_string();
        let mut diff = 0;
        let len = seq_a.len().min(seq_b.len());
        if len == 0 {
            return 1.0;
        }
        for i in 0..len {
            if seq_a.as_bytes()[i] != seq_b.as_bytes()[i] {
                diff += 1;
            }
        }
        diff as f32 / len as f32
    }

    #[func]
    fn get_metabolic_rate(&self, species: GString) -> f32 {
        let s = species.to_string().to_lowercase();
        match s.as_str() {
            "human" => 100.0,
            "mutant" => 150.0,
            "ghoul" => 50.0,
            "animal" => 80.0,
            "plant" => 10.0,
            "bacteria" => 200.0,
            _ => {
                let seed = s.len() as f32 * 7.0 + s.bytes().fold(0.0f32, |a, b| a + b as f32);
                50.0 + (seed * 0.1).sin() * 30.0
            },
        }
    }

    #[func]
    fn get_disease_resistance(&self, species: GString) -> f32 {
        let s = species.to_string().to_lowercase();
        let seed = s.len() as f32 * 13.0 + s.bytes().fold(0.0f32, |a, b| a + b as f32);
        let base = match s.as_str() {
            "human" => 0.5,
            "mutant" => 0.8,
            "ghoul" => 0.95,
            "robot" => 1.0,
            _ => 0.4,
        };
        (base + (seed * 0.2).sin() * 0.3).clamp(0.0, 1.0)
    }

    #[func]
    fn get_organ_health(&self, species: GString, organ: GString) -> f32 {
        let s = species.to_string().to_lowercase();
        let o = organ.to_string().to_lowercase();
        let seed = s.len() as f32 * 3.0 + o.len() as f32 * 7.0;
        let base = match s.as_str() {
            "human" => 0.8,
            "mutant" => 0.7,
            "ghoul" => 0.4,
            "robot" => 0.95,
            _ => 0.6,
        };
        (base + (seed * 0.5).sin() * 0.2).clamp(0.0, 1.0)
    }
}
