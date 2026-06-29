use crate::WastelandWorld;
use godot::prelude::*;

struct SoundSource {
    position: Vector3,
    amplitude: f32,
    frequency: f32,
    active: bool,
}

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandAcoustics {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    speed_of_sound: f32,

    #[var]
    master_volume: f32,

    #[var]
    doppler_scale: f32,

    #[var]
    active_sources: i64,

    #[var]
    total_energy: f32,

    #[var]
    grid_resolution: i64,

    sound_sources: Vec<SoundSource>,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandAcoustics {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            speed_of_sound: 343.0,
            master_volume: 1.0,
            doppler_scale: 1.0,
            active_sources: 0,
            total_energy: 0.0,
            grid_resolution: 0,
            sound_sources: Vec::new(),
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
impl WastelandAcoustics {
    fn sync_from_world(&mut self) {
        if let Some(ref world) = self.world_ref {
            let data = world.bind().export_acoustics_data();
            if let Some(v) = data.get("active_sources") {
                self.active_sources = v.to::<i64>();
            }
            if let Some(v) = data.get("total_energy") {
                self.total_energy = v.to::<f32>();
            }
            if let Some(v) = data.get("speed_of_sound") {
                self.speed_of_sound = v.to::<f32>();
            }
            if let Some(v) = data.get("grid_resolution") {
                self.grid_resolution = v.to::<i64>();
            }
        }
    }

    #[func]
    fn get_acoustics_stats(&self) -> Dictionary<Variant, Variant> {
        if let Some(ref world) = self.world_ref {
            return world.bind().export_acoustics_data();
        }
        dict! {
            "active_sources" => self.active_sources,
            "total_energy" => self.total_energy,
            "speed_of_sound" => self.speed_of_sound,
            "master_volume" => self.master_volume,
            "doppler_scale" => self.doppler_scale,
            "grid_resolution" => self.grid_resolution,
        }
    }

    #[func]
    fn set_listener_position(&mut self, x: f32, y: f32, z: f32) {
        godot_print!("[Acoustics] Listener set to ({}, {}, {})", x, y, z);
    }

    #[func]
    fn add_sound_source(
        &mut self,
        px: f32,
        py: f32,
        pz: f32,
        amplitude: f32,
        frequency: f32,
    ) -> i64 {
        let source =
            SoundSource { position: Vector3::new(px, py, pz), amplitude, frequency, active: true };
        self.sound_sources.push(source);
        self.active_sources = self.sound_sources.iter().filter(|s| s.active).count() as i64;
        (self.sound_sources.len() - 1) as i64
    }

    #[func]
    fn remove_sound_source(&mut self, index: i64) -> bool {
        let idx = index as usize;
        if idx < self.sound_sources.len() {
            self.sound_sources.remove(idx);
            self.active_sources = self.sound_sources.iter().filter(|s| s.active).count() as i64;
            true
        } else {
            false
        }
    }

    #[func]
    fn get_sound_sources(&self) -> Array<Variant> {
        let mut arr = Array::<Variant>::new();
        for s in &self.sound_sources {
            let d: Dictionary<Variant, Variant> = dict! {
                "position" => s.position,
                "amplitude" => s.amplitude,
                "frequency" => s.frequency,
                "active" => s.active,
            };
            arr.push(&d);
        }
        arr
    }

    #[func]
    fn compute_hrtf(
        &self,
        source_x: f32,
        source_y: f32,
        source_z: f32,
        listener_x: f32,
        listener_y: f32,
        listener_z: f32,
        head_yaw: f32,
    ) -> Dictionary<Variant, Variant> {
        let dx = source_x - listener_x;
        let dy = source_y - listener_y;
        let dz = source_z - listener_z;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt().max(0.01);
        let azimuth = dz.atan2(dx) - head_yaw;
        let elevation = dy.atan2((dx * dx + dz * dz).sqrt());
        let itd = (azimuth.sin() * 0.08 / dist).clamp(-0.001, 0.001);
        let head_shadow = (azimuth.abs() / std::f32::consts::PI).min(1.0);
        let left_ear = (1.0 / dist) * (1.0 - head_shadow * 0.7) * self.master_volume;
        let right_ear = (1.0 / dist) * (1.0 - (1.0 - head_shadow) * 0.7) * self.master_volume;
        let ild = 20.0 * (right_ear / left_ear.max(0.001)).log10();
        dict! {
            "left_ear" => left_ear,
            "right_ear" => right_ear,
            "itd" => itd,
            "ild" => ild,
            "azimuth" => azimuth,
            "elevation" => elevation,
            "distance" => dist,
        }
    }

    #[func]
    fn get_material_sound(&self, material_name: GString) -> Dictionary<Variant, Variant> {
        let name = material_name.to_string().to_lowercase();
        let (hardness, density, resonant_freq) = match name.as_str() {
            "concrete" => (0.9, 2400.0, 120.0),
            "steel" => (0.95, 7800.0, 440.0),
            "wood" => (0.4, 600.0, 80.0),
            "glass" => (0.85, 2500.0, 300.0),
            "sand" => (0.1, 1600.0, 30.0),
            "water" => (0.0, 1000.0, 20.0),
            "rubber" => (0.05, 1200.0, 15.0),
            "soil" => (0.2, 1800.0, 40.0),
            _ => (0.5, 1500.0, 60.0),
        };
        dict! {
            "material" => &material_name,
            "hardness" => hardness,
            "density" => density,
            "resonant_freq" => resonant_freq,
            "speed_of_sound" => self.speed_of_sound,
        }
    }

    #[func]
    #[allow(clippy::too_many_arguments)]
    fn get_doppler_shift(
        &self,
        source_x: f32,
        source_y: f32,
        source_z: f32,
        source_vx: f32,
        source_vy: f32,
        source_vz: f32,
        listener_vx: f32,
        listener_vy: f32,
        listener_vz: f32,
    ) -> f32 {
        let dx = source_x;
        let dy = source_y;
        let dz = source_z;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt().max(0.01);
        let nx = dx / dist;
        let ny = dy / dist;
        let nz = dz / dist;
        let source_radial = source_vx * nx + source_vy * ny + source_vz * nz;
        let listener_radial = listener_vx * nx + listener_vy * ny + listener_vz * nz;
        let factor = (self.speed_of_sound + listener_radial)
            / (self.speed_of_sound + source_radial).max(0.001);
        factor * self.doppler_scale
    }
}
