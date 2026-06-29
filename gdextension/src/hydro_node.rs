use crate::WastelandWorld;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandHydro {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    water_table_depth: f32,

    #[var]
    infiltration_rate: f32,

    #[var]
    evaporation_rate: f32,

    river_flow_avg: f32,
    drought_index: f32,
    #[allow(dead_code)]
    aquifer_map: Vec<(f32, f32, f32)>,
    #[allow(dead_code)]
    water_quality_map: Vec<(f32, f32, f32)>,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandHydro {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            water_table_depth: 10.0,
            infiltration_rate: 0.3,
            evaporation_rate: 0.1,
            river_flow_avg: 0.5,
            drought_index: 0.0,
            aquifer_map: Vec::new(),
            water_quality_map: Vec::new(),
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
impl WastelandHydro {
    fn sync_from_world(&mut self) {
        if let Some(ref world) = self.world_ref {
            let data = world.bind().get_stats();
            if let Some(v) = data.get("global_temperature") {
                self.evaporation_rate = (v.to::<f32>() * 0.0001).clamp(0.01, 0.5);
                self.drought_index = (1.0 - self.evaporation_rate * 2.0).max(0.0);
            }
        }
    }

    #[func]
    fn get_runoff_at(&self, x: f32, y: f32) -> f32 {
        let precipitation_factor = (x * 0.01 + y * 0.02).sin().abs();
        (1.0 - self.infiltration_rate) * precipitation_factor * (1.0 - self.evaporation_rate)
    }

    #[func]
    fn get_infiltration_at(&self, x: f32, y: f32) -> f32 {
        let soil_factor = (x * 0.02 + y * 0.01).cos().abs() * 0.5 + 0.5;
        self.infiltration_rate * soil_factor
    }

    #[func]
    fn get_water_table_at(&self, x: f32, y: f32) -> f32 {
        let variation = (x * 0.005 + y * 0.008).sin() * 5.0;
        self.water_table_depth + variation
    }

    #[func]
    fn get_river_flow_at(&self, x: f32, y: f32) -> f32 {
        let runoff = self.get_runoff_at(x, y);
        let gradient = (x * 0.01 + y * 0.02).sin().abs();
        runoff * gradient * self.river_flow_avg * 10.0
    }

    #[func]
    fn get_flood_risk_at(&self, x: f32, y: f32) -> f32 {
        let runoff = self.get_runoff_at(x, y);
        let wt = self.get_water_table_at(x, y);
        let risk = runoff * (1.0 / (wt.max(0.1))) * 10.0;
        risk.min(1.0)
    }

    #[func]
    fn get_drought_index(&self) -> f32 {
        self.drought_index
    }

    #[func]
    fn get_aquifer_capacity(&self, x: f32, y: f32) -> f32 {
        let porosity = (x * 0.01 + y * 0.02).cos().abs() * 0.3 + 0.2;
        let thickness = 20.0 + (x * 0.005 + y * 0.008).sin() * 10.0;
        let area = 100.0;
        porosity * thickness * area
    }

    #[func]
    fn get_water_quality(&self, x: f32, y: f32) -> f32 {
        let natural_purity = (x * 0.02 + y * 0.03).cos().abs() * 0.5 + 0.5;
        let contamination = (x * 0.01 - y * 0.01).sin().abs() * 0.2;
        let depth_factor = (self.get_water_table_at(x, y) / 20.0).min(1.0);
        (natural_purity * (1.0 - contamination) + depth_factor * 0.2).min(1.0)
    }

    #[func]
    fn get_watershed_area(&self, x: f32, y: f32) -> f32 {
        let elevation = (x * 0.01 + y * 0.02).sin().abs() * 100.0;
        let slope = (x * 0.02 - y * 0.01).cos().abs() * 0.5 + 0.1;
        let base_area = 10000.0;
        base_area * (1.0 + elevation * 0.5) * slope
    }

    #[func]
    fn get_sediment_load(&self, x: f32, y: f32) -> f32 {
        let flow = self.get_river_flow_at(x, y);
        let erosion = (x * 0.03 + y * 0.04).sin().abs() * 0.5 + 0.2;
        (flow * erosion * 0.05).min(100.0)
    }

    #[func]
    fn get_groundwater_flow_direction(&self, x: f32, y: f32) -> Vector3 {
        let grad_x = -(x * 0.01 + y * 0.02).cos() * 0.5;
        let grad_y = -(x * 0.02 + y * 0.01).cos() * 0.5;
        let grad_z = -(x * 0.005 + y * 0.005).sin() * 0.1;
        let mag = (grad_x * grad_x + grad_y * grad_y + grad_z * grad_z).sqrt().max(0.001);
        Vector3::new(grad_x / mag, grad_y / mag, grad_z / mag)
    }

    #[func]
    fn simulate_rainfall(
        &self,
        x: f32,
        y: f32,
        intensity: f32,
        duration: f32,
    ) -> Dictionary<Variant, Variant> {
        let total_rain = intensity * duration;
        let infiltration = self.get_infiltration_at(x, y);
        let runoff = (1.0 - infiltration) * total_rain;
        let infiltrated = infiltration * total_rain;
        let base_flood = if self.get_flood_risk_at(x, y) > 0.5 { 0.3 } else { 0.1 };
        let flood_risk = (base_flood + runoff * 0.1).min(1.0);
        dict! {
            "runoff" => runoff,
            "infiltration" => infiltrated,
            "flood_risk" => flood_risk,
        }
    }
}
