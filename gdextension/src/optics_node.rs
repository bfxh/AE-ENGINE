use crate::WastelandWorld;
use godot::prelude::*;

struct LightSource {
    position: Vector3,
    color: Vector3,
    intensity: f32,
    radius: f32,
    active: bool,
}

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandOptics {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    ambient_light: f32,

    #[var]
    exposure: f32,

    #[var]
    max_bounces: i64,

    #[var]
    active_lights: i64,

    #[var]
    total_luminance: f32,

    light_sources: Vec<LightSource>,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandOptics {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            ambient_light: 0.1,
            exposure: 1.0,
            max_bounces: 4,
            active_lights: 0,
            total_luminance: 0.0,
            light_sources: Vec::new(),
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
impl WastelandOptics {
    fn sync_from_world(&mut self) {
        if let Some(ref world) = self.world_ref {
            let data = world.bind().export_optics_data();
            if let Some(v) = data.get("active_lights") {
                self.active_lights = v.to::<i64>();
            }
            if let Some(v) = data.get("max_bounces") {
                self.max_bounces = v.to::<i64>();
            }
            if let Some(v) = data.get("total_luminance") {
                self.total_luminance = v.to::<f32>();
            }
        }
    }

    #[func]
    fn get_optics_stats(&self) -> Dictionary<Variant, Variant> {
        if let Some(ref world) = self.world_ref {
            return world.bind().export_optics_data();
        }
        dict! {
            "ambient_light" => self.ambient_light,
            "exposure" => self.exposure,
            "active_lights" => self.active_lights,
            "total_luminance" => self.total_luminance,
            "max_bounces" => self.max_bounces,
        }
    }

    #[func]
    fn compute_blackbody(&self, temperature: f32) -> Color {
        let t = temperature.max(100.0);
        let t_k = t / 1000.0;
        let r = if t < 6700.0 { 1.0 } else { (1.0 + (t_k - 6.7) * 0.5).min(1.5) };
        let g = if t < 1000.0 {
            0.0
        } else if t < 6700.0 {
            ((t_k - 1.0) / 5.7).clamp(0.0, 1.0)
        } else {
            (1.0 - (t_k - 6.7) * 0.5).max(0.0)
        };
        let b = if t < 2000.0 {
            0.0
        } else if t < 6700.0 {
            ((t_k - 2.0) / 4.7).clamp(0.0, 1.0)
        } else {
            1.0
        };
        Color::from_rgb(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
    }

    #[func]
    #[allow(clippy::too_many_arguments)]
    fn add_light_source(
        &mut self,
        px: f32,
        py: f32,
        pz: f32,
        r: f32,
        g: f32,
        b: f32,
        intensity: f32,
        radius: f32,
    ) -> i64 {
        let source = LightSource {
            position: Vector3::new(px, py, pz),
            color: Vector3::new(r, g, b),
            intensity,
            radius,
            active: true,
        };
        self.light_sources.push(source);
        self.active_lights = self.light_sources.iter().filter(|l| l.active).count() as i64;
        self.total_luminance =
            self.light_sources.iter().map(|l| l.color.length() * l.intensity).sum();
        (self.light_sources.len() - 1) as i64
    }

    #[func]
    fn remove_light_source(&mut self, index: i64) -> bool {
        let idx = index as usize;
        if idx < self.light_sources.len() {
            self.light_sources.remove(idx);
            self.active_lights = self.light_sources.iter().filter(|l| l.active).count() as i64;
            self.total_luminance =
                self.light_sources.iter().map(|l| l.color.length() * l.intensity).sum();
            true
        } else {
            false
        }
    }

    #[func]
    fn get_light_sources(&self) -> Array<Variant> {
        let mut arr = Array::<Variant>::new();
        for l in &self.light_sources {
            let d: Dictionary<Variant, Variant> = dict! {
                "position" => l.position,
                "color" => Color::from_rgb(l.color.x, l.color.y, l.color.z),
                "intensity" => l.intensity,
                "radius" => l.radius,
                "active" => l.active,
            };
            arr.push(&d);
        }
        arr
    }

    #[func]
    #[allow(clippy::too_many_arguments)]
    fn get_brdf_query(
        &self,
        material: GString,
        normal_x: f32,
        normal_y: f32,
        normal_z: f32,
        view_x: f32,
        view_y: f32,
        view_z: f32,
        light_x: f32,
        light_y: f32,
        light_z: f32,
    ) -> Dictionary<Variant, Variant> {
        let mat_name = material.to_string().to_lowercase();
        let (roughness, specular) = match mat_name.as_str() {
            "metal" => (0.15, 0.9),
            "concrete" => (0.8, 0.05),
            "wood" => (0.5, 0.1),
            "glass" => (0.05, 0.95),
            "plastic" => (0.3, 0.3),
            "sand" => (0.9, 0.02),
            "water" => (0.1, 0.8),
            "rubber" => (0.6, 0.03),
            _ => (0.5, 0.2),
        };
        let normal = Vector3::new(normal_x, normal_y, normal_z).normalized();
        let view = Vector3::new(view_x, view_y, view_z).normalized();
        let light = Vector3::new(light_x, light_y, light_z).normalized();
        let half = (view + light).normalized();
        let n_dot_l = normal.dot(light).max(0.0);
        let n_dot_v = normal.dot(view).max(0.0);
        let n_dot_h = normal.dot(half).max(0.0);
        let v_dot_h = view.dot(half).max(0.0);
        let diffuse = n_dot_l;
        let spec_term = if n_dot_h > 0.0 { n_dot_h.powf(2.0 / (roughness + 0.001)) } else { 0.0 };
        let spec_result = specular * spec_term;
        let f0 = specular;
        let fresnel = f0 + (1.0 - f0) * (1.0 - v_dot_h).powf(5.0);
        dict! {
            "material" => &material,
            "diffuse" => diffuse,
            "specular" => spec_result,
            "roughness" => roughness,
            "fresnel" => fresnel,
            "n_dot_l" => n_dot_l,
            "n_dot_v" => n_dot_v,
        }
    }

    #[func]
    fn compute_spectral(&self, temperature: f32) -> Dictionary<Variant, Variant> {
        let t = temperature.max(100.0);
        let wien_displacement = 2.898e-3 / t;
        let wavelength_nm = wien_displacement * 1e9;
        let color = self.compute_blackbody(t);
        let stefan_boltzmann = 5.67e-8;
        let intensity = stefan_boltzmann * t * t * t * t;
        dict! {
            "temperature" => t,
            "wavelength" => wavelength_nm,
            "rgb" => Vector3::new(color.r, color.g, color.b),
            "intensity" => intensity,
            "wien_displacement" => wien_displacement,
        }
    }

    #[func]
    fn get_visibility_at(
        &self,
        px: f32,
        py: f32,
        pz: f32,
        target_x: f32,
        target_y: f32,
        target_z: f32,
    ) -> f32 {
        let dx = target_x - px;
        let dy = target_y - py;
        let dz = target_z - pz;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        if dist < 0.001 {
            return 1.0;
        }
        let attenuation = (-dist * 0.01).exp();
        (attenuation * self.ambient_light).clamp(0.0, 1.0)
    }
}
