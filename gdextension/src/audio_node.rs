use godot::prelude::*;

use ae_audio::source::{DirectivityPattern, PropagationMode, SoundSource};
use ae_audio::spatial::SpatialAudio;
use ae_audio::synthesis::{FractureSound, FrictionSound, ImpactSound};

#[derive(GodotClass)]
#[class(base=Node)]
pub(crate) struct WastelandAudio {
    #[var]
    master_volume: f32,
    #[var]
    doppler_factor: f32,
    #[var]
    speed_of_sound: f32,
    source_count: i64,
    active_sources: i64,

    #[allow(dead_code)]
    spatial: SpatialAudio,
    #[allow(dead_code)]
    impact_count: i64,
    #[allow(dead_code)]
    friction_count: i64,
    #[allow(dead_code)]
    fracture_count: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandAudio {
    fn init(base: Base<Node>) -> Self {
        Self {
            master_volume: 1.0,
            doppler_factor: 1.0,
            speed_of_sound: 343.0,
            source_count: 0,
            active_sources: 0,
            spatial: SpatialAudio::default(),
            impact_count: 0,
            friction_count: 0,
            fracture_count: 0,
            base,
        }
    }
}

#[godot_api]
impl WastelandAudio {
    #[func]
    fn create_point_source(
        &mut self,
        x: f32,
        y: f32,
        z: f32,
        volume_db: f32,
    ) -> Dictionary<Variant, Variant> {
        let source = SoundSource::new_point(glam::Vec3::new(x, y, z), volume_db);
        let id = self.source_count;
        self.source_count += 1;
        self.active_sources += 1;
        dict! {
            "source_id" => id,
            "volume_db" => source.volume_db,
            "max_distance" => source.max_distance,
            "occlusion" => source.occlusion,
        }
    }

    #[func]
    fn set_source_directivity(&mut self, _source_id: i64, pattern: GString) {
        let _p = match pattern.to_string().as_str() {
            "cardioid" => DirectivityPattern::Cardioid,
            "hypercardioid" => DirectivityPattern::Hypercardioid,
            "figure8" => DirectivityPattern::Figure8,
            "shotgun" => DirectivityPattern::Shotgun,
            _ => DirectivityPattern::Omnidirectional,
        };
    }

    #[func]
    fn set_source_propagation(&mut self, _source_id: i64, mode: GString) {
        let _m = match mode.to_string().as_str() {
            "solid" => PropagationMode::Solid,
            "liquid" => PropagationMode::Liquid,
            _ => PropagationMode::Air,
        };
    }

    #[func]
    fn compute_spatial_attenuation(
        &self,
        source_x: f32,
        source_y: f32,
        source_z: f32,
        listener_x: f32,
        listener_y: f32,
        listener_z: f32,
    ) -> f32 {
        let src = glam::Vec3::new(source_x, source_y, source_z);
        let lst = glam::Vec3::new(listener_x, listener_y, listener_z);
        let dist = src.distance(lst).max(0.01);
        let attenuation = 1.0 / (1.0 + dist * 0.1);
        attenuation * self.master_volume
    }

    #[func]
    fn compute_doppler_shift(
        &self,
        source_velocity_x: f32,
        source_velocity_y: f32,
        source_velocity_z: f32,
        listener_velocity_x: f32,
        listener_velocity_y: f32,
        listener_velocity_z: f32,
        source_freq: f32,
    ) -> f32 {
        let sv = glam::Vec3::new(source_velocity_x, source_velocity_y, source_velocity_z);
        let lv = glam::Vec3::new(listener_velocity_x, listener_velocity_y, listener_velocity_z);
        let rel_vel = sv - lv;
        let shift = self.speed_of_sound / (self.speed_of_sound + rel_vel.length());
        source_freq * shift * self.doppler_factor
    }

    #[func]
    fn synthesize_impact(
        &self,
        material_a: GString,
        material_b: GString,
        force: f32,
        velocity: f32,
    ) -> Dictionary<Variant, Variant> {
        let ma = material_a.to_string();
        let mb = material_b.to_string();
        let mut rng = rand::thread_rng();
        let impact = ImpactSound::from_materials(&ma, &mb, velocity, force, &mut rng);
        dict! {
            "amplitude" => impact.amplitude,
            "frequency" => impact.frequency,
            "decay" => impact.decay,
            "mode_count" => impact.material_modes.len() as i64,
        }
    }

    #[func]
    fn synthesize_friction(
        &self,
        _material_a: GString,
        _material_b: GString,
        normal_force: f32,
        velocity: f32,
    ) -> Dictionary<Variant, Variant> {
        let friction = FrictionSound::new(0.5, normal_force, velocity);
        dict! {
            "base_frequency" => friction.base_frequency,
            "roughness" => friction.roughness,
            "velocity" => friction.velocity,
        }
    }

    #[func]
    fn synthesize_fracture(
        &self,
        _material: GString,
        stress: f32,
        _volume: f32,
    ) -> Dictionary<Variant, Variant> {
        let fracture = FractureSound::new(5, 2.0, stress);
        dict! {
            "crack_count" => fracture.crack_count as i64,
            "material_density" => fracture.material_density,
            "stress_level" => fracture.stress_level,
        }
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        dict! {
            "source_count" => self.source_count,
            "active_sources" => self.active_sources,
            "master_volume" => self.master_volume,
            "speed_of_sound" => self.speed_of_sound,
        }
    }
}
