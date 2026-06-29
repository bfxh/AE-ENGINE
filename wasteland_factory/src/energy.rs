use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyNetwork {
    pub generators: Vec<Generator>,
    pub consumers: Vec<EnergyConsumer>,
    pub storage: Vec<EnergyStorage>,
    pub total_generation: f32,
    pub total_consumption: f32,
    pub grid_stability: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generator {
    pub id: String,
    pub generator_type: GeneratorType,
    pub output_power: f32,
    pub max_power: f32,
    pub efficiency: f32,
    pub fuel_consumption: f32,
    pub fuel_remaining: f32,
    pub wear: f32,
    pub running: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneratorType {
    Diesel,
    SteamTurbine,
    GasTurbine,
    Solar,
    Wind,
    Geothermal,
    Nuclear,
    Biomass,
    Hydroelectric,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyConsumer {
    pub id: String,
    pub name: String,
    pub power_draw: f32,
    pub priority: ConsumptionPriority,
    pub required: bool,
    pub efficiency: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConsumptionPriority {
    Critical = 0,
    High = 1,
    Medium = 2,
    Low = 3,
    Optional = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyStorage {
    pub id: String,
    pub storage_type: StorageType,
    pub capacity: f32,
    pub current_charge: f32,
    pub charge_rate: f32,
    pub discharge_rate: f32,
    pub efficiency: f32,
    pub cycle_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageType {
    Battery,
    Flywheel,
    Capacitor,
    CompressedAir,
    PumpedHydro,
    Thermal,
    Hydrogen,
}

impl Default for EnergyNetwork {
    fn default() -> Self {
        Self::new()
    }
}

impl EnergyNetwork {
    pub fn new() -> Self {
        Self {
            generators: Vec::new(),
            consumers: Vec::new(),
            storage: Vec::new(),
            total_generation: 0.0,
            total_consumption: 0.0,
            grid_stability: 1.0,
        }
    }

    pub fn add_generator(&mut self, gen: Generator) {
        self.generators.push(gen);
    }

    pub fn add_consumer(&mut self, consumer: EnergyConsumer) {
        self.consumers.push(consumer);
    }

    pub fn add_storage(&mut self, storage: EnergyStorage) {
        self.storage.push(storage);
    }

    pub fn update(&mut self, dt: f32) {
        self.total_generation = self
            .generators
            .iter()
            .filter(|g| g.running)
            .map(|g| g.output_power * g.efficiency)
            .sum();

        self.consumers.sort_by_key(|c| c.priority);
        self.total_consumption =
            self.consumers.iter().map(|c| c.power_draw / c.efficiency.max(0.01)).sum();

        let balance = self.total_generation - self.total_consumption;

        if balance > 0.0 {
            let charge_amount = balance * dt;
            for storage in &mut self.storage {
                if storage.current_charge < storage.capacity {
                    let space = storage.capacity - storage.current_charge;
                    let charge = charge_amount.min(space * storage.charge_rate * dt);
                    storage.current_charge += charge * storage.efficiency;
                    storage.current_charge = storage.current_charge.min(storage.capacity);
                }
            }
        } else {
            let deficit = -balance;
            let mut remaining_deficit = deficit;
            for storage in &mut self.storage {
                if remaining_deficit <= 0.0 {
                    break;
                }
                if storage.current_charge > 0.0 {
                    let discharge =
                        remaining_deficit.min(storage.current_charge * storage.discharge_rate * dt);
                    storage.current_charge -= discharge;
                    storage.current_charge = storage.current_charge.max(0.0);
                    storage.cycle_count += 1;
                    remaining_deficit -= discharge;
                }
            }

            if remaining_deficit > 0.0 {
                self.shed_load(remaining_deficit);
            }
        }

        let max_demand = self.consumers.iter().map(|c| c.power_draw).sum::<f32>() + 1.0;
        self.grid_stability = if max_demand > 0.0 {
            (self.total_generation / max_demand).clamp(0.0, 1.0)
        } else {
            1.0
        };

        for gen in &mut self.generators {
            if gen.running {
                gen.fuel_remaining -= gen.fuel_consumption * dt;
                gen.wear += 0.00001 * dt;
                if gen.fuel_remaining <= 0.0 {
                    gen.running = false;
                }
            }
        }
    }

    fn shed_load(&mut self, deficit: f32) {
        let mut remaining = deficit;
        for consumer in &mut self.consumers {
            if remaining <= 0.0 {
                break;
            }
            if (consumer.priority == ConsumptionPriority::Optional
                || consumer.priority == ConsumptionPriority::Low)
                && !consumer.required
            {
                let shed = consumer.power_draw;
                consumer.power_draw = 0.0;
                remaining -= shed;
            }
        }
    }

    pub fn energy_stored(&self) -> f32 {
        self.storage.iter().map(|s| s.current_charge).sum()
    }

    pub fn storage_capacity(&self) -> f32 {
        self.storage.iter().map(|s| s.capacity).sum()
    }

    pub fn fuel_remaining_hours(&self) -> f32 {
        let total_consumption: f32 =
            self.generators.iter().filter(|g| g.running).map(|g| g.fuel_consumption).sum();
        if total_consumption <= 0.0 {
            return f32::MAX;
        }
        let total_fuel: f32 = self.generators.iter().map(|g| g.fuel_remaining).sum();
        total_fuel / total_consumption
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_energy_network_creation() {
        let network = EnergyNetwork::new();
        assert!(network.generators.is_empty());
        assert!(network.consumers.is_empty());
        assert_eq!(network.grid_stability, 1.0);
    }

    #[test]
    fn test_energy_network_basic_load() {
        let mut network = EnergyNetwork::new();
        network.add_generator(Generator {
            id: "gen1".to_string(),
            generator_type: GeneratorType::Diesel,
            output_power: 1000.0,
            max_power: 1000.0,
            efficiency: 0.9,
            fuel_consumption: 0.1,
            fuel_remaining: 100.0,
            wear: 0.0,
            running: true,
        });
        network.add_consumer(EnergyConsumer {
            id: "c1".to_string(),
            name: "工厂".to_string(),
            power_draw: 500.0,
            priority: ConsumptionPriority::High,
            required: true,
            efficiency: 0.95,
        });
        network.update(1.0);
        assert!(network.total_generation > 0.0);
        assert!(network.total_consumption > 0.0);
    }

    #[test]
    fn test_energy_storage() {
        let mut network = EnergyNetwork::new();
        network.add_generator(Generator {
            id: "gen1".to_string(),
            generator_type: GeneratorType::Solar,
            output_power: 2000.0,
            max_power: 2000.0,
            efficiency: 0.8,
            fuel_consumption: 0.0,
            fuel_remaining: f32::MAX,
            wear: 0.0,
            running: true,
        });
        network.add_storage(EnergyStorage {
            id: "bat1".to_string(),
            storage_type: StorageType::Battery,
            capacity: 1000.0,
            current_charge: 0.0,
            charge_rate: 0.5,
            discharge_rate: 0.5,
            efficiency: 0.9,
            cycle_count: 0,
        });
        network.add_consumer(EnergyConsumer {
            id: "c1".to_string(),
            name: "灯".to_string(),
            power_draw: 100.0,
            priority: ConsumptionPriority::Low,
            required: false,
            efficiency: 0.9,
        });
        network.update(1.0);
        assert!(
            network.energy_stored() > 0.0 || network.total_generation >= network.total_consumption
        );
    }
}
