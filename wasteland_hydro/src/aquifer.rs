use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aquifer {
    pub name: String,
    pub depth: f32,
    pub thickness: f32,
    pub porosity: f32,
    pub hydraulic_conductivity: f32,
    pub storativity: f32,
    pub water_table: f32,
    pub recharge_rate: f32,
    pub transmissivity: f32,
}

impl Aquifer {
    pub fn new(name: &str, depth: f32, thickness: f32, porosity: f32) -> Self {
        let hydraulic_conductivity = 0.00001;
        let transmissivity = hydraulic_conductivity * thickness;
        Self {
            name: name.to_string(),
            depth,
            thickness,
            porosity,
            hydraulic_conductivity,
            storativity: 0.001,
            water_table: depth,
            recharge_rate: 0.0,
            transmissivity,
        }
    }

    pub fn darcy_flow(&self, head_gradient: Vec3, area: f32) -> f32 {
        let gradient_mag = head_gradient.length();
        self.hydraulic_conductivity * gradient_mag * area
    }

    pub fn darcy_velocity(&self, head_gradient: Vec3) -> Vec3 {
        let gradient_mag = head_gradient.length();
        if gradient_mag < 0.0001 {
            return Vec3::ZERO;
        }
        let dir = head_gradient / gradient_mag;
        dir * self.hydraulic_conductivity * gradient_mag
    }

    pub fn specific_discharge(&self, head_gradient: Vec3) -> Vec3 {
        self.darcy_velocity(head_gradient) / self.porosity
    }

    pub fn drawdown(&self, pumping_rate: f32, distance: f32, time: f32) -> f32 {
        if time <= 0.0 || self.transmissivity <= 0.0 {
            return 0.0;
        }
        let u = distance * distance * self.storativity / (4.0 * self.transmissivity * time);
        let wu = if u < 0.01 { -0.5772 - u.ln() + u } else { 0.0 };
        pumping_rate * wu / (4.0 * std::f32::consts::PI * self.transmissivity)
    }

    pub fn update_water_table(&mut self, recharge: f32, extraction: f32, dt: f32) {
        let net = recharge - extraction;
        let change = net * dt / (self.porosity * self.storativity.max(0.0001));
        self.water_table += change;
        self.water_table = self.water_table.clamp(self.depth, self.depth + self.thickness);
        self.recharge_rate = recharge;
    }

    pub fn sustainable_yield(&self) -> f32 {
        self.recharge_rate * 0.7
    }

    pub fn water_volume(&self) -> f32 {
        let saturated_thickness = (self.depth + self.thickness - self.water_table).max(0.0);
        saturated_thickness * self.porosity
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AquiferSystem {
    pub aquifers: Vec<Aquifer>,
    pub confining_layers: Vec<ConfiningLayer>,
    pub springs: Vec<Spring>,
}

impl AquiferSystem {
    pub fn new() -> Self {
        Self { aquifers: Vec::new(), confining_layers: Vec::new(), springs: Vec::new() }
    }

    pub fn add_aquifer(&mut self, aquifer: Aquifer) {
        self.aquifers.push(aquifer);
    }

    pub fn add_confining_layer(&mut self, layer: ConfiningLayer) {
        self.confining_layers.push(layer);
    }

    pub fn update(&mut self, surface_recharge: f32, dt: f32) {
        for aquifer in &mut self.aquifers {
            let confining = self
                .confining_layers
                .iter()
                .find(|c| c.depth >= aquifer.depth && c.depth <= aquifer.depth + aquifer.thickness);
            let effective_recharge = match confining {
                Some(c) => surface_recharge * (1.0 - c.impermeability),
                None => surface_recharge,
            };
            aquifer.update_water_table(effective_recharge, 0.0, dt);
        }

        self.springs.clear();
        for aquifer in &self.aquifers {
            if aquifer.water_table <= aquifer.depth {
                self.springs.push(Spring {
                    position: Vec3::ZERO,
                    flow_rate: aquifer.recharge_rate * 0.1,
                    source_aquifer: aquifer.name.clone(),
                });
            }
        }
    }
}

impl Default for AquiferSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfiningLayer {
    pub depth: f32,
    pub thickness: f32,
    pub impermeability: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spring {
    pub position: Vec3,
    pub flow_rate: f32,
    pub source_aquifer: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_aquifer_creation() {
        let aq = Aquifer::new("砂岩含水层", 50.0, 20.0, 0.3);
        assert_eq!(aq.name, "砂岩含水层");
        assert_eq!(aq.depth, 50.0);
        assert_eq!(aq.thickness, 20.0);
        assert_eq!(aq.porosity, 0.3);
        assert!(aq.transmissivity > 0.0);
    }

    #[test]
    fn test_aquifer_darcy_flow() {
        let aq = Aquifer::new("测试", 30.0, 10.0, 0.25);
        let flow = aq.darcy_flow(Vec3::new(0.1, 0.0, 0.0), 100.0);
        assert!(flow > 0.0);
        let vel = aq.darcy_velocity(Vec3::new(0.1, 0.0, 0.0));
        assert!(vel.length() > 0.0);
    }

    #[test]
    fn test_aquifer_system() {
        let mut system = AquiferSystem::new();
        system.add_aquifer(Aquifer::new("上层", 10.0, 5.0, 0.3));
        system.add_aquifer(Aquifer::new("下层", 30.0, 10.0, 0.2));
        system.add_confining_layer(ConfiningLayer {
            depth: 15.0,
            thickness: 2.0,
            impermeability: 0.9,
        });
        system.update(0.001, 1.0);
        assert_eq!(system.aquifers.len(), 2);
    }
}
