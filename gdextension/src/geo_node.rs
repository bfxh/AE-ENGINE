use crate::WastelandWorld;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandGeo {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    erosion_rate: f32,

    #[var]
    tectonic_activity: f32,

    #[var]
    grid_size: i64,

    geothermal_gradient: f32,
    #[allow(dead_code)]
    plate_boundaries: Vec<(f32, f32, f32)>,
    #[allow(dead_code)]
    mineral_distribution: Vec<(f32, f32, GString, f32)>,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandGeo {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            erosion_rate: 0.01,
            tectonic_activity: 0.1,
            grid_size: 64,
            geothermal_gradient: 25.0,
            plate_boundaries: Vec::new(),
            mineral_distribution: Vec::new(),
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
impl WastelandGeo {
    fn sync_from_world(&mut self) {
        if let Some(ref world) = self.world_ref {
            let data = world.bind().get_stats();
            if let Some(v) = data.get("global_temperature") {
                self.geothermal_gradient = (v.to::<f32>() * 0.01).clamp(10.0, 50.0);
            }
        }
    }

    #[func]
    fn get_erosion_at(&self, x: f32, y: f32) -> f32 {
        self.erosion_rate * (1.0 + (x * 0.01 + y * 0.02).sin() * 0.5)
    }

    #[func]
    fn get_rock_hardness_at(&self, x: f32, y: f32) -> f32 {
        let base = 0.5 + self.tectonic_activity * 0.5;
        base + (x * 0.03 + y * 0.04).cos() * 0.3
    }

    #[func]
    fn get_tectonic_stress(&self) -> Vector3 {
        let sx = (self.tectonic_activity * 10.0).sin() * self.tectonic_activity;
        let sy = (self.tectonic_activity * 8.0).cos() * self.tectonic_activity;
        let sz = (self.tectonic_activity * 6.0).sin() * self.tectonic_activity * 0.5;
        Vector3::new(sx, sy, sz)
    }

    #[func]
    fn get_soil_type_at(&self, x: f32, y: f32) -> GString {
        let h = self.get_rock_hardness_at(x, y);
        let e = self.get_erosion_at(x, y);
        if h > 0.7 {
            GString::from("rock")
        } else if h > 0.5 && e < 0.02 {
            GString::from("clay")
        } else if h > 0.3 && e > 0.015 {
            GString::from("sand")
        } else if e > 0.01 {
            GString::from("silt")
        } else {
            GString::from("loam")
        }
    }

    #[func]
    fn get_mineral_deposit_at(&self, x: f32, y: f32) -> GString {
        let h = self.get_rock_hardness_at(x, y);
        let candidates = ["iron", "copper", "coal", "uranium", "gold", "silver", "none"];
        let idx = ((x * 7.0 + y * 13.0).abs() as usize + (h * 10.0) as usize) % candidates.len();
        GString::from(candidates[idx])
    }

    #[func]
    fn get_geothermal_gradient(&self) -> f32 {
        self.geothermal_gradient
    }

    #[func]
    fn get_terrain_height(&self, x: f32, y: f32) -> f32 {
        let mut height = 0.0;
        let mut amp = 1.0;
        let mut freq = 0.01;
        for i in 0..5 {
            let octave = (x * freq * (i + 1) as f32 + y * freq * 0.7 * (i + 1) as f32).sin()
                * (y * freq * 0.5 * (i + 1) as f32 + x * freq * 0.3 * (i + 1) as f32).cos();
            height += octave * amp;
            amp *= 0.5;
            freq *= 2.0;
        }
        height * 20.0 + (x * 0.001 + y * 0.002).sin() * 50.0
    }

    #[func]
    fn get_plate_boundary(&self, x: f32, y: f32) -> bool {
        let threshold = 0.15;
        let e1 = (x * 0.002 + y * 0.003).sin().abs();
        let e2 = (x * 0.005 - y * 0.001).cos().abs();
        let e3 = (x * 0.001 - y * 0.004).sin().abs();
        e1 < threshold || e2 < threshold || e3 < threshold
    }

    #[func]
    fn get_resource_richness(&self, x: f32, y: f32, mineral: GString) -> f32 {
        let m_str = mineral.to_string();
        let seed = (m_str.len() as f32 * 7.0 + x * 3.0 + y * 5.0).sin() * 0.5 + 0.5;
        let depth = (x * 0.01 + y * 0.02).cos().abs() * 0.5 + 0.3;
        let plate = if self.get_plate_boundary(x, y) { 0.3 } else { 0.0 };
        (seed * 0.4 + depth * 0.3 + plate + self.tectonic_activity * 0.2).min(1.0)
    }

    #[func]
    fn generate_terrain_column(&self, x: f32, y: f32) -> PackedFloat32Array {
        let mut arr = PackedFloat32Array::new();
        let surface = self.get_terrain_height(x, y);
        let depth = 8;
        let step = surface / depth as f32;
        for i in 0..depth {
            let h = surface - step * i as f32;
            let noise = (x * 0.05 + y * 0.03 + i as f32 * 0.5).sin() * 2.0;
            arr.push((h + noise).max(0.0));
        }
        arr.push(0.0);
        arr
    }

    #[func]
    fn get_fault_line_direction(&self, x: f32, y: f32) -> Vector3 {
        let angle = (x * 0.003 + y * 0.005).sin() * std::f32::consts::PI;
        let dx = angle.cos();
        let dy = angle.sin();
        let dz = (x * 0.002 - y * 0.004).cos() * 0.3;
        Vector3::new(dx, dy, dz)
    }

    #[func]
    fn estimate_earthquake_risk(&self, x: f32, y: f32) -> f32 {
        let boundary = if self.get_plate_boundary(x, y) { 0.6 } else { 0.1 };
        let stress = self.get_tectonic_stress();
        let stress_mag = (stress.x * stress.x + stress.y * stress.y + stress.z * stress.z).sqrt();
        let pos_factor = (x * 0.01 + y * 0.02).sin().abs() * 0.3;
        (boundary + stress_mag * 0.2 + pos_factor + self.tectonic_activity * 0.3).min(1.0)
    }
}
