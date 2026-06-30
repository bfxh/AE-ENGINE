use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FurnaceType {
    ElectricArc,
    Induction,
    Blast,
    Crucible,
    RotaryKiln,
    Vacuum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Furnace {
    pub id: String,
    pub ftype: FurnaceType,
    pub temperature: f32,
    pub target_temperature: f32,
    pub power: f32,
    pub efficiency: f32,
    pub contents: Vec<FurnaceInput>,
    pub atmosphere: FurnaceAtmosphere,
    pub insulation_quality: f32,
    pub heat_loss: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FurnaceInput {
    pub material_id: String,
    pub mass: f32,
    pub temperature: f32,
    pub melting_point: f32,
    pub specific_heat: f32,
    pub phase: MaterialPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterialPhase {
    Solid,
    Liquid,
    Gas,
    Plasma,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FurnaceAtmosphere {
    Air,
    Argon,
    Nitrogen,
    Vacuum,
    Reducing,
    Oxidizing,
}

impl Default for Furnace {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            ftype: FurnaceType::ElectricArc,
            temperature: 293.0,
            target_temperature: 293.0,
            power: 0.0,
            efficiency: 0.7,
            contents: Vec::new(),
            atmosphere: FurnaceAtmosphere::Air,
            insulation_quality: 0.8,
            heat_loss: 0.0,
        }
    }
}

impl Furnace {
    pub fn max_temperature(&self) -> f32 {
        match self.ftype {
            FurnaceType::ElectricArc => 3500.0,
            FurnaceType::Induction => 2000.0,
            FurnaceType::Blast => 2300.0,
            FurnaceType::Crucible => 1800.0,
            FurnaceType::RotaryKiln => 1700.0,
            FurnaceType::Vacuum => 3000.0,
        }
    }

    pub fn heating_rate(&self) -> f32 {
        match self.ftype {
            FurnaceType::ElectricArc => 100.0,
            FurnaceType::Induction => 50.0,
            FurnaceType::Blast => 20.0,
            FurnaceType::Crucible => 10.0,
            FurnaceType::RotaryKiln => 5.0,
            FurnaceType::Vacuum => 30.0,
        }
    }

    pub fn add_material(
        &mut self,
        material_id: &str,
        mass: f32,
        melting_point: f32,
        specific_heat: f32,
    ) {
        self.contents.push(FurnaceInput {
            material_id: material_id.to_string(),
            mass,
            temperature: self.temperature,
            melting_point,
            specific_heat,
            phase: MaterialPhase::Solid,
        });
    }

    pub fn update(&mut self, dt: f32) {
        let surface_area = 10.0;
        let ambient_temp: f32 = 293.0;
        let stefan_boltzmann: f32 = 5.67e-8;
        let emissivity: f32 = 0.8;
        self.heat_loss = emissivity
            * stefan_boltzmann
            * surface_area
            * (self.temperature.powi(4) - ambient_temp.powi(4))
            * (1.0 - self.insulation_quality);

        let heating = self.power * self.efficiency * dt;
        let temp_diff = self.target_temperature - self.temperature;
        let heating_rate = self.heating_rate();
        let thermal_response = temp_diff * heating_rate * dt * self.efficiency;

        let total_heat_capacity: f32 = self.contents.iter().map(|c| c.mass * c.specific_heat).sum();

        let net_heat = (heating + thermal_response - self.heat_loss * dt).max(0.0);
        if total_heat_capacity > 0.0 {
            self.temperature += net_heat / total_heat_capacity;
        } else {
            self.temperature += (self.target_temperature - self.temperature) * 0.1 * dt;
        }

        self.temperature = self.temperature.clamp(ambient_temp, self.max_temperature());

        for content in &mut self.contents {
            content.temperature = self.temperature;
            if self.temperature >= content.melting_point {
                content.phase = MaterialPhase::Liquid;
            } else {
                content.phase = MaterialPhase::Solid;
            }
        }
    }

    pub fn energy_consumption(&self) -> f32 {
        self.power * 3600.0
    }

    pub fn thermal_efficiency(&self) -> f32 {
        let useful_heat: f32 =
            self.contents.iter().map(|c| c.mass * c.specific_heat * (c.temperature - 293.0)).sum();
        let total_energy = self.power * 3600.0;
        if total_energy > 0.0 { (useful_heat / total_energy).clamp(0.0, 1.0) } else { 0.0 }
    }

    pub fn melt_progress(&self) -> f32 {
        if self.contents.is_empty() {
            return 0.0;
        }
        let melted: f32 =
            self.contents.iter().filter(|c| c.phase == MaterialPhase::Liquid).map(|c| c.mass).sum();
        let total: f32 = self.contents.iter().map(|c| c.mass).sum();
        if total > 0.0 { melted / total } else { 0.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_furnace_creation() {
        let furnace = Furnace::default();
        assert_eq!(furnace.ftype, FurnaceType::ElectricArc);
        assert_eq!(furnace.temperature, 293.0);
        assert_eq!(furnace.efficiency, 0.7);
        assert!(furnace.contents.is_empty());
    }

    #[test]
    fn test_furnace_temperature_change() {
        let mut furnace = Furnace {
            power: 1000.0,
            target_temperature: 1000.0,
            ..Default::default()
        };
        furnace.update(1.0);
        assert!(furnace.temperature > 293.0);
        assert!(furnace.temperature <= furnace.max_temperature());
    }

    #[test]
    fn test_furnace_melting() {
        let mut furnace = Furnace::default();
        furnace.add_material("iron", 10.0, 1800.0, 450.0);
        assert_eq!(furnace.melt_progress(), 0.0);
        furnace.power = 50000.0;
        furnace.target_temperature = 2000.0;
        furnace.update(10.0);
        assert!(furnace.temperature > 293.0);
    }
}
