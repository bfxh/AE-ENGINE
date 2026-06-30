use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct River {
    pub name: String,
    pub discharge: f32,
    pub width: f32,
    pub depth: f32,
    pub slope: f32,
    pub manning_n: f32,
    pub sediment_load: f32,
    pub water_temperature: f32,
    pub pollution: f32,
    pub velocity: f32,
}

impl River {
    pub fn new(name: &str, discharge: f32, slope: f32, width: f32) -> Self {
        let depth = if width > 0.0 { discharge / (width * 0.5) } else { 0.0 };
        Self {
            name: name.to_string(),
            discharge,
            width,
            depth,
            slope,
            manning_n: 0.035,
            sediment_load: 0.0,
            water_temperature: 288.0,
            pollution: 0.0,
            velocity: 0.0,
        }
    }

    pub fn update_hydraulics(&mut self) {
        let wetted_perimeter = self.width + 2.0 * self.depth;
        let hydraulic_radius =
            if wetted_perimeter > 0.0 { self.width * self.depth / wetted_perimeter } else { 0.0 };

        self.velocity = if self.manning_n > 0.0 {
            (1.0 / self.manning_n) * hydraulic_radius.powf(2.0 / 3.0) * self.slope.abs().sqrt()
        } else {
            0.0
        };

        self.discharge = self.width * self.depth * self.velocity;
    }

    pub fn sediment_transport_capacity(&self, grain_size: f32) -> f32 {
        const GRAVITY: f32 = 9.81;
        const WATER_DENSITY: f32 = 1000.0;
        const SEDIMENT_DENSITY: f32 = 2650.0;

        let shear = WATER_DENSITY * GRAVITY * self.depth * self.slope;
        let critical_shear = 0.03 * (SEDIMENT_DENSITY - WATER_DENSITY) * GRAVITY * grain_size;

        if shear > critical_shear { 0.01 * (shear - critical_shear).powf(1.5) } else { 0.0 }
    }

    pub fn erode_bank(&mut self, bank_material_resistance: f32, dt: f32) {
        let shear = 1000.0 * 9.81 * self.depth * self.slope;
        if shear > bank_material_resistance {
            let erosion = (shear - bank_material_resistance) * 0.0001 * dt;
            self.width += erosion;
            self.sediment_load += erosion * 0.5;
            self.update_hydraulics();
        }
    }

    pub fn deposit(&mut self, dt: f32) {
        let capacity = self.sediment_transport_capacity(0.001);
        if self.sediment_load > capacity {
            let deposit = (self.sediment_load - capacity) * 0.01 * dt;
            self.sediment_load -= deposit;
            self.depth -= deposit * 0.001;
        }
    }

    pub fn mix_pollution(&mut self, upstream_pollution: f32, flow_rate: f32, dt: f32) {
        let mixing = (upstream_pollution - self.pollution) * flow_rate * dt;
        self.pollution += mixing;
        self.pollution = self.pollution.clamp(0.0, 1.0);
        let decay = self.pollution * 0.0001 * dt;
        self.pollution -= decay;
    }

    pub fn heat_exchange(&mut self, air_temperature: f32, solar_radiation: f32, dt: f32) {
        let temp_diff = air_temperature - self.water_temperature;
        let exchange_rate = 0.001 + solar_radiation * 0.00001;
        self.water_temperature += temp_diff * exchange_rate * dt;
        self.water_temperature = self.water_temperature.clamp(273.0, 310.0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiverNetwork {
    pub rivers: Vec<River>,
    pub connections: Vec<(usize, usize)>,
    pub total_discharge: f32,
}

impl RiverNetwork {
    pub fn new() -> Self {
        Self { rivers: Vec::new(), connections: Vec::new(), total_discharge: 0.0 }
    }

    pub fn add_river(&mut self, river: River) -> usize {
        let idx = self.rivers.len();
        self.rivers.push(river);
        self.update_total_discharge();
        idx
    }

    pub fn connect(&mut self, upstream: usize, downstream: usize) {
        if upstream < self.rivers.len() && downstream < self.rivers.len() {
            self.connections.push((upstream, downstream));
        }
    }

    pub fn update_total_discharge(&mut self) {
        self.total_discharge = self.rivers.iter().map(|r| r.discharge).sum();
    }

    pub fn downstream_flow(&self, river_idx: usize) -> f32 {
        let mut flow = self.rivers[river_idx].discharge;
        for &(up, down) in &self.connections {
            if down == river_idx && up < self.rivers.len() {
                flow += self.rivers[up].discharge;
            }
        }
        flow
    }

    pub fn update_all(&mut self, dt: f32) {
        for river in &mut self.rivers {
            river.update_hydraulics();
            river.erode_bank(50.0, dt);
            river.deposit(dt);
        }

        for &(up, down) in &self.connections {
            if up < self.rivers.len() && down < self.rivers.len() {
                let upstream_pollution = self.rivers[up].pollution;
                let flow_rate = self.rivers[up].discharge / self.rivers[down].discharge.max(0.001);
                self.rivers[down].mix_pollution(upstream_pollution, flow_rate.min(1.0), dt);
            }
        }

        self.update_total_discharge();
    }
}

impl Default for RiverNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_river_creation() {
        let river = River::new("黄河", 100.0, 0.001, 50.0);
        assert_eq!(river.name, "黄河");
        assert_eq!(river.discharge, 100.0);
        assert_eq!(river.width, 50.0);
        assert!(river.depth > 0.0);
    }

    #[test]
    fn test_river_hydraulics() {
        let mut river = River::new("测试河", 50.0, 0.002, 30.0);
        river.update_hydraulics();
        assert!(river.velocity > 0.0);
        assert!(river.discharge > 0.0);
    }

    #[test]
    fn test_river_network() {
        let mut network = RiverNetwork::new();
        let r1 = River::new("上游", 100.0, 0.002, 40.0);
        let r2 = River::new("下游", 50.0, 0.001, 30.0);
        let idx1 = network.add_river(r1);
        let idx2 = network.add_river(r2);
        network.connect(idx1, idx2);
        network.update_all(1.0);
        assert!(network.total_discharge > 0.0);
    }
}
