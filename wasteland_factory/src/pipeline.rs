use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub diameter: f32,
    pub length: f32,
    pub roughness: f32,
    pub flow_rate: f32,
    pub pressure: f32,
    pub fluid: Option<FluidContent>,
    pub valves: Vec<Valve>,
    pub connections: Vec<PipelineConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluidContent {
    pub fluid_id: String,
    pub density: f32,
    pub viscosity: f32,
    pub temperature: f32,
    pub volume: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Valve {
    pub position: f32,
    pub open_fraction: f32,
    pub valve_type: ValveType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValveType {
    Gate,
    Ball,
    Butterfly,
    Check,
    PressureRelief,
    FlowControl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConnection {
    pub target_id: String,
    pub position: f32,
    pub connection_type: ConnectionType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionType {
    Inline,
    Branch,
    Return,
    Bypass,
}

impl Pipeline {
    pub fn new(diameter: f32, length: f32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            diameter,
            length,
            roughness: 0.000045,
            flow_rate: 0.0,
            pressure: 101325.0,
            fluid: None,
            valves: Vec::new(),
            connections: Vec::new(),
        }
    }

    pub fn add_valve(&mut self, position: f32, valve_type: ValveType) {
        self.valves.push(Valve {
            position: position.clamp(0.0, self.length),
            open_fraction: 1.0,
            valve_type,
        });
    }

    pub fn set_valve(&mut self, valve_index: usize, open_fraction: f32) {
        if let Some(valve) = self.valves.get_mut(valve_index) {
            valve.open_fraction = open_fraction.clamp(0.0, 1.0);
        }
    }

    pub fn darcy_weisbach_head_loss(&self) -> f32 {
        if self.flow_rate <= 0.0 {
            return 0.0;
        }
        let fluid = match &self.fluid {
            Some(f) => f,
            None => return 0.0,
        };
        let area = std::f32::consts::PI * (self.diameter / 2.0).powi(2);
        let velocity = self.flow_rate / area;
        let reynolds = fluid.density * velocity * self.diameter / fluid.viscosity.max(0.000001);

        let friction_factor = if reynolds < 2300.0 {
            if reynolds > 0.0 { 64.0 / reynolds } else { 0.0 }
        } else {
            let relative_roughness = self.roughness / self.diameter;
            let a = (relative_roughness / 3.7).powf(1.11) + 6.9 / reynolds;
            if a > 0.0 { 0.25 / (a.log10() * a.log10()) } else { 0.0 }
        };

        let valve_loss: f32 = self
            .valves
            .iter()
            .map(|v| {
                let k = match v.valve_type {
                    ValveType::Gate => 0.15 * (1.0 - v.open_fraction),
                    ValveType::Ball => 0.05 * (1.0 - v.open_fraction),
                    ValveType::Butterfly => 0.5 * (1.0 - v.open_fraction),
                    ValveType::Check => 2.0,
                    ValveType::PressureRelief => 5.0,
                    ValveType::FlowControl => 1.0 * (1.0 - v.open_fraction),
                };
                k * velocity * velocity / (2.0 * 9.81)
            })
            .sum();

        friction_factor * (self.length / self.diameter) * (velocity * velocity / (2.0 * 9.81))
            + valve_loss
    }

    pub fn update_flow(&mut self, pressure_difference: f32, dt: f32) {
        let head_loss = self.darcy_weisbach_head_loss();
        let fluid = match &self.fluid {
            Some(f) => f,
            None => {
                self.flow_rate = 0.0;
                return;
            },
        };

        let driving_pressure = pressure_difference - head_loss * fluid.density * 9.81;
        let area = std::f32::consts::PI * (self.diameter / 2.0).powi(2);

        let effective_flow = if driving_pressure > 0.0 {
            (driving_pressure / (fluid.density * 0.5)).sqrt() * area
        } else {
            0.0
        };

        self.flow_rate += (effective_flow - self.flow_rate) * 0.1 * dt;
        self.flow_rate = self.flow_rate.max(0.0);
    }

    pub fn volumetric_flow(&self) -> f32 {
        self.flow_rate
    }

    pub fn mass_flow(&self) -> f32 {
        match &self.fluid {
            Some(f) => self.flow_rate * f.density,
            None => 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tank {
    pub id: String,
    pub capacity: f32,
    pub current_volume: f32,
    pub fluid: Option<FluidContent>,
    pub pressure: f32,
    pub temperature: f32,
    pub insulation: f32,
}

impl Tank {
    pub fn new(capacity: f32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            capacity,
            current_volume: 0.0,
            fluid: None,
            pressure: 101325.0,
            temperature: 293.0,
            insulation: 0.5,
        }
    }

    pub fn fill_level(&self) -> f32 {
        if self.capacity > 0.0 { self.current_volume / self.capacity } else { 0.0 }
    }

    pub fn add_fluid(&mut self, volume: f32, fluid: FluidContent) -> f32 {
        let available = self.capacity - self.current_volume;
        let added = volume.min(available);
        self.current_volume += added;
        if self.fluid.is_none() {
            self.fluid = Some(fluid);
        }
        added
    }

    pub fn remove_fluid(&mut self, volume: f32) -> f32 {
        let removed = volume.min(self.current_volume);
        self.current_volume -= removed;
        if self.current_volume < 0.001 {
            self.fluid = None;
        }
        removed
    }

    pub fn update_temperature(&mut self, ambient_temp: f32, dt: f32) {
        let heat_loss = (self.temperature - ambient_temp) * (1.0 - self.insulation) * 0.01;
        self.temperature -= heat_loss * dt;
    }
}
