use crate::WastelandWorld;
use godot::prelude::*;

struct Conveyor {
    start: Vector3,
    end: Vector3,
    speed: f32,
}

struct Machine {
    machine_type: String,
    position: Vector3,
    power_draw: f32,
    active: bool,
}

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandFactory {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    tick_rate: f32,

    #[var]
    power_available: f32,

    #[var]
    conveyor_count: i64,

    #[var]
    sensor_count: i64,

    #[var]
    energy_consumption: f32,

    #[var]
    grid_stability: f32,

    conveyors: Vec<Conveyor>,
    machines: Vec<Machine>,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandFactory {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            tick_rate: 1.0,
            power_available: 0.0,
            conveyor_count: 0,
            sensor_count: 0,
            energy_consumption: 0.0,
            grid_stability: 1.0,
            conveyors: Vec::new(),
            machines: Vec::new(),
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
impl WastelandFactory {
    fn sync_from_world(&mut self) {
        if let Some(ref world) = self.world_ref {
            let data = world.bind().export_factory_data();
            if let Some(v) = data.get("tick_rate") {
                self.tick_rate = v.to::<f32>();
            }
            if let Some(v) = data.get("energy_generation") {
                self.power_available = v.to::<f32>();
            }
            if let Some(v) = data.get("energy_consumption") {
                self.energy_consumption = v.to::<f32>();
            }
            if let Some(v) = data.get("conveyor_count") {
                self.conveyor_count = v.to::<i64>();
            }
            if let Some(v) = data.get("sensor_count") {
                self.sensor_count = v.to::<i64>();
            }
            if let Some(v) = data.get("grid_stability") {
                self.grid_stability = v.to::<f32>();
            }
        }
    }

    #[func]
    fn get_factory_stats(&self) -> Dictionary<Variant, Variant> {
        if let Some(ref world) = self.world_ref {
            return world.bind().export_factory_data();
        }
        dict! {
            "tick_rate" => self.tick_rate,
            "energy_generation" => self.power_available,
            "energy_consumption" => self.energy_consumption,
            "conveyor_count" => self.conveyor_count,
            "sensor_count" => self.sensor_count,
            "grid_stability" => self.grid_stability,
        }
    }

    #[func]
    fn add_conveyor(
        &mut self,
        start_x: f32,
        start_y: f32,
        start_z: f32,
        end_x: f32,
        end_y: f32,
        end_z: f32,
        speed: f32,
    ) -> i64 {
        let conv = Conveyor {
            start: Vector3::new(start_x, start_y, start_z),
            end: Vector3::new(end_x, end_y, end_z),
            speed,
        };
        self.conveyors.push(conv);
        self.conveyor_count = self.conveyors.len() as i64;
        (self.conveyors.len() - 1) as i64
    }

    #[func]
    fn remove_conveyor(&mut self, index: i64) -> bool {
        let idx = index as usize;
        if idx < self.conveyors.len() {
            self.conveyors.remove(idx);
            self.conveyor_count = self.conveyors.len() as i64;
            true
        } else {
            false
        }
    }

    #[func]
    fn get_conveyors(&self) -> Array<Variant> {
        let mut arr = Array::<Variant>::new();
        for c in &self.conveyors {
            let length = (c.end - c.start).length();
            let d: Dictionary<Variant, Variant> = dict! {
                "start" => c.start,
                "end" => c.end,
                "speed" => c.speed,
                "length" => length,
            };
            arr.push(&d);
        }
        arr
    }

    #[func]
    fn add_machine(
        &mut self,
        machine_type: GString,
        px: f32,
        py: f32,
        pz: f32,
        power_draw: f32,
    ) -> i64 {
        let m = Machine {
            machine_type: machine_type.to_string(),
            position: Vector3::new(px, py, pz),
            power_draw,
            active: true,
        };
        self.machines.push(m);
        self.energy_consumption =
            self.machines.iter().map(|m| if m.active { m.power_draw } else { 0.0 }).sum();
        (self.machines.len() - 1) as i64
    }

    #[func]
    fn remove_machine(&mut self, index: i64) -> bool {
        let idx = index as usize;
        if idx < self.machines.len() {
            self.machines.remove(idx);
            self.energy_consumption =
                self.machines.iter().map(|m| if m.active { m.power_draw } else { 0.0 }).sum();
            true
        } else {
            false
        }
    }

    #[func]
    fn get_machines(&self) -> Array<Variant> {
        let mut arr = Array::<Variant>::new();
        for m in &self.machines {
            let d: Dictionary<Variant, Variant> = dict! {
                "machine_type" => m.machine_type.clone().as_str(),
                "position" => m.position,
                "power_draw" => m.power_draw,
                "active" => m.active,
            };
            arr.push(&d);
        }
        arr
    }

    #[func]
    fn get_power_grid_status(&self) -> Dictionary<Variant, Variant> {
        let generation = self.power_available;
        let consumption = self.energy_consumption;
        let stability = if generation > 0.0 {
            (1.0 - (consumption / generation).min(1.0)) * self.grid_stability
        } else {
            0.0
        };
        let overload = consumption > generation && generation > 0.0;
        dict! {
            "generation" => generation,
            "consumption" => consumption,
            "stability" => stability,
            "overload" => overload,
            "surplus" => (generation - consumption).max(0.0),
            "tick_rate" => self.tick_rate,
        }
    }
}
