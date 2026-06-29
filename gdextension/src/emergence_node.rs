use crate::WastelandWorld;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandEmergence {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    emergence_threshold: f32,

    complexity_level: f32,

    active_patterns: i64,
    crack_count: i64,
    rust_count: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandEmergence {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            emergence_threshold: 0.5,
            complexity_level: 0.0,
            active_patterns: 0,
            crack_count: 0,
            rust_count: 0,
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
impl WastelandEmergence {
    fn sync_from_world(&mut self) {
        if let Some(ref world) = self.world_ref {
            self.crack_count = world.bind().get_crack_count();
            self.rust_count = world.bind().get_rust_sizes().len() as i64;
            self.active_patterns = self.crack_count + self.rust_count;
            self.complexity_level = (self.active_patterns as f32).sqrt() * 0.1;
        }
    }

    #[func]
    fn get_crack_positions(&self) -> PackedVector3Array {
        if let Some(ref world) = self.world_ref {
            return world.bind().get_crack_positions();
        }
        PackedVector3Array::new()
    }

    #[func]
    fn get_crack_count(&self) -> i64 {
        self.crack_count
    }

    #[func]
    fn get_rust_positions(&self) -> PackedVector3Array {
        if let Some(ref world) = self.world_ref {
            return world.bind().get_rust_positions();
        }
        PackedVector3Array::new()
    }

    #[func]
    fn get_rust_sizes(&self) -> PackedFloat32Array {
        if let Some(ref world) = self.world_ref {
            return world.bind().get_rust_sizes();
        }
        PackedFloat32Array::new()
    }

    #[func]
    fn get_bark_fissure_positions(&self) -> PackedVector3Array {
        if let Some(ref world) = self.world_ref {
            return world.bind().get_bark_fissure_positions();
        }
        PackedVector3Array::new()
    }

    #[func]
    fn get_growth_ring_years(&self) -> PackedInt32Array {
        if let Some(ref world) = self.world_ref {
            return world.bind().get_growth_ring_years();
        }
        PackedInt32Array::new()
    }

    #[func]
    fn get_growth_ring_thicknesses(&self) -> PackedFloat32Array {
        if let Some(ref world) = self.world_ref {
            return world.bind().get_growth_ring_thicknesses();
        }
        PackedFloat32Array::new()
    }

    #[func]
    fn compute_morphogenesis(
        &self,
        activator: f32,
        inhibitor: f32,
        iterations: i64,
    ) -> PackedFloat32Array {
        let mut arr = PackedFloat32Array::new();
        let iterations = iterations.clamp(1, 1000) as usize;
        let mut u = activator;
        let mut v = inhibitor;
        let du = 0.2;
        let dv = 0.1;
        let feed_rate = 0.055;
        let kill_rate = 0.062;
        for _ in 0..iterations {
            let reaction = u * u * v;
            let laplacian_u = -u;
            let laplacian_v = -v;
            u += du * laplacian_u - reaction + feed_rate * (1.0 - u);
            v += dv * laplacian_v + reaction - (feed_rate + kill_rate) * v;
            u = u.clamp(0.0, 10.0);
            v = v.clamp(0.0, 10.0);
            arr.push(u);
            arr.push(v);
        }
        arr
    }

    #[func]
    fn compute_holographic_material(
        &self,
        wavelength: f32,
        angle: f32,
        intensity: f32,
    ) -> Dictionary<Variant, Variant> {
        let phase = (angle * wavelength * 0.1).cos();
        let response = intensity * phase.abs();
        let color_shift = (wavelength * 0.01).sin() * 0.5 + 0.5;
        dict! {
            "response" => response,
            "phase" => phase,
            "color_shift" => color_shift,
            "wavelength_nm" => wavelength,
            "visible" => wavelength > 380.0 && wavelength < 750.0,
        }
    }

    #[func]
    fn compute_time_surface(
        &self,
        x: f32,
        y: f32,
        z: f32,
        time: f32,
    ) -> Dictionary<Variant, Variant> {
        let seed = x * 0.1 + y * 0.15 + z * 0.05;
        let evolution = (seed + time * 0.01).sin();
        let stability = (seed + time * 0.005).cos().abs();
        dict! {
            "evolution" => evolution,
            "stability" => stability,
            "age" => time,
            "state" => &GString::from(if evolution > 0.5 { "growing" } else if evolution < -0.5 { "decaying" } else { "stable" }),
        }
    }

    #[func]
    fn get_active_patterns(&self) -> i64 {
        self.active_patterns
    }

    #[func]
    fn get_complexity_level(&self) -> f32 {
        self.complexity_level
    }
}
