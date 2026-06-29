use crate::WastelandWorld;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandThermo {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    ambient_temperature: f32,

    #[var]
    heat_transfer_rate: f32,

    #[var]
    grid_resolution: i64,

    thermal_conductivity: f32,
    #[allow(dead_code)]
    specific_heat: f32,
    #[allow(dead_code)]
    phase_state: GString,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandThermo {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            ambient_temperature: 293.15,
            heat_transfer_rate: 1.0,
            grid_resolution: 32,
            thermal_conductivity: 0.0,
            specific_heat: 0.0,
            phase_state: GString::from("unknown"),
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
impl WastelandThermo {
    fn sync_from_world(&mut self) {
        if let Some(ref world) = self.world_ref {
            let data = world.bind().get_stats();
            if let Some(v) = data.get("global_temperature") {
                self.ambient_temperature = v.to::<f32>();
            }
        }
    }

    #[func]
    fn get_temperature_at(&self, x: f32, y: f32, z: f32) -> f32 {
        if let Some(ref world) = self.world_ref {
            let field_val = world.bind().get_field_value(GString::from("temperature"), x, y, z);
            if field_val != 0.0 {
                return field_val;
            }
        }
        self.ambient_temperature + (x * 0.01 + y * 0.02 + z * 0.005).sin() * 5.0
    }

    #[func]
    fn compute_heat_flow(&self, t1: f32, t2: f32, distance: f32, area: f32) -> f32 {
        let conductivity = self.thermal_conductivity.max(0.001);
        conductivity * area * (t1 - t2) / distance.max(0.001)
    }

    #[func]
    fn get_thermal_conductivity(&self, material: GString) -> f32 {
        let m = material.to_string().to_lowercase();
        match m.as_str() {
            "iron" => 80.0,
            "copper" => 401.0,
            "aluminum" => 237.0,
            "steel" => 50.0,
            "concrete" => 1.7,
            "wood" => 0.15,
            "water" => 0.6,
            "air" => 0.026,
            "glass" => 1.0,
            _ => 1.0,
        }
    }

    #[func]
    fn get_specific_heat(&self, material: GString) -> f32 {
        let m = material.to_string().to_lowercase();
        match m.as_str() {
            "iron" => 450.0,
            "copper" => 385.0,
            "aluminum" => 900.0,
            "steel" => 500.0,
            "concrete" => 880.0,
            "wood" => 2000.0,
            "water" => 4184.0,
            "air" => 1005.0,
            "glass" => 840.0,
            _ => 1000.0,
        }
    }

    #[func]
    fn get_phase_at(&self, material: GString, temperature: f32) -> GString {
        let m = material.to_string().to_lowercase();
        match m.as_str() {
            "water" => {
                if temperature < 273.15 {
                    GString::from("solid")
                } else if temperature > 373.15 {
                    GString::from("gas")
                } else {
                    GString::from("liquid")
                }
            },
            "iron" => {
                if temperature > 1811.0 {
                    GString::from("liquid")
                } else {
                    GString::from("solid")
                }
            },
            _ => {
                if temperature > 2000.0 {
                    GString::from("gas")
                } else if temperature > 1000.0 {
                    GString::from("liquid")
                } else {
                    GString::from("solid")
                }
            },
        }
    }

    #[func]
    fn compute_convective_heat_transfer(
        &self,
        surface_temp: f32,
        fluid_temp: f32,
        area: f32,
        velocity: f32,
    ) -> f32 {
        let h = 5.0 + 3.8 * velocity;
        h * area * (surface_temp - fluid_temp)
    }

    #[func]
    fn compute_radiative_heat_transfer(
        &self,
        temp_a: f32,
        temp_b: f32,
        area: f32,
        emissivity: f32,
    ) -> f32 {
        if !temp_a.is_finite() || !temp_b.is_finite() {
            return 0.0;
        }
        let sigma = 5.670367e-8;
        sigma * emissivity * area * (temp_a.powi(4) - temp_b.powi(4))
    }

    #[func]
    fn compute_latent_heat(
        &self,
        material: GString,
        phase_from: GString,
        phase_to: GString,
    ) -> f32 {
        let m = material.to_string().to_lowercase();
        let from = phase_from.to_string().to_lowercase();
        let to = phase_to.to_string().to_lowercase();
        match (m.as_str(), from.as_str(), to.as_str()) {
            ("water", "solid", "liquid") => 334000.0,
            ("water", "liquid", "gas") => 2260000.0,
            ("iron", "solid", "liquid") => 247000.0,
            _ => 100000.0,
        }
    }
}
