use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConveyorType {
    Belt,
    Roller,
    Chain,
    Screw,
    Pneumatic,
    Magnetic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conveyor {
    pub id: String,
    pub ctype: ConveyorType,
    pub speed: f32,
    pub length: f32,
    pub direction: Vec3,
    pub items: Vec<ConveyorItem>,
    pub power_consumption: f32,
    pub wear: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConveyorItem {
    pub item_id: String,
    pub position: f32,
    pub mass: f32,
    pub friction: f32,
}

impl Conveyor {
    pub fn new(ctype: ConveyorType, speed: f32, length: f32, direction: Vec3) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            ctype,
            speed,
            length,
            direction: direction.normalize(),
            items: Vec::new(),
            power_consumption: 0.0,
            wear: 0.0,
        }
    }

    pub fn max_speed(&self) -> f32 {
        match self.ctype {
            ConveyorType::Belt => 3.0,
            ConveyorType::Roller => 5.0,
            ConveyorType::Chain => 2.0,
            ConveyorType::Screw => 1.5,
            ConveyorType::Pneumatic => 20.0,
            ConveyorType::Magnetic => 10.0,
        }
    }

    pub fn add_item(&mut self, item_id: &str, mass: f32) {
        self.items.push(ConveyorItem {
            item_id: item_id.to_string(),
            position: 0.0,
            mass,
            friction: 0.3,
        });
    }

    pub fn update(&mut self, dt: f32) {
        let effective_speed = self.speed.min(self.max_speed()) * (1.0 - self.wear * 0.5);

        for item in &mut self.items {
            let slip = 1.0 - item.friction * 0.1;
            item.position += effective_speed * slip * dt;
        }

        self.items.retain(|item| item.position < self.length);

        let total_mass: f32 = self.items.iter().map(|i| i.mass).sum();
        self.power_consumption = total_mass * effective_speed * 10.0;

        self.wear += self.items.len() as f32 * 0.0001 * dt;
        self.wear = self.wear.min(1.0);
    }

    pub fn throughput(&self) -> f32 {
        if self.items.is_empty() {
            return 0.0;
        }
        self.items.iter().map(|i| i.mass).sum::<f32>() * self.speed / self.length
    }

    pub fn item_at_end(&self) -> Option<&ConveyorItem> {
        self.items.iter().find(|item| item.position >= self.length)
    }

    pub fn remove_arrived(&mut self) -> Vec<ConveyorItem> {
        let mut arrived = Vec::new();
        let mut i = 0;
        while i < self.items.len() {
            if self.items[i].position >= self.length {
                arrived.push(self.items.remove(i));
            } else {
                i += 1;
            }
        }
        arrived
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConveyorNetwork {
    pub conveyors: Vec<Conveyor>,
    pub connections: Vec<(usize, usize)>,
    pub total_power: f32,
}

impl Default for ConveyorNetwork {
    fn default() -> Self {
        Self::new()
    }
}

impl ConveyorNetwork {
    pub fn new() -> Self {
        Self { conveyors: Vec::new(), connections: Vec::new(), total_power: 0.0 }
    }

    pub fn add_conveyor(&mut self, conveyor: Conveyor) -> usize {
        let idx = self.conveyors.len();
        self.conveyors.push(conveyor);
        idx
    }

    pub fn connect(&mut self, from: usize, to: usize) {
        if from < self.conveyors.len() && to < self.conveyors.len() {
            self.connections.push((from, to));
        }
    }

    pub fn update_all(&mut self, dt: f32) {
        for conveyor in &mut self.conveyors {
            conveyor.update(dt);
        }

        for &(from, to) in &self.connections {
            let (from_len, to_len) = {
                let from_conv = &self.conveyors[from];
                let to_conv = &self.conveyors[to];
                (from_conv.items.len(), to_conv.items.len())
            };

            if from_len > 0 && to_len < 10 {
                if let Some(item) = self.conveyors[from].items.last() {
                    let item_id = item.item_id.clone();
                    let mass = item.mass;
                    self.conveyors[from].items.pop();
                    self.conveyors[to].add_item(&item_id, mass);
                }
            }
        }

        self.total_power = self.conveyors.iter().map(|c| c.power_consumption).sum();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_conveyor_creation() {
        let conv = Conveyor::new(ConveyorType::Belt, 2.0, 10.0, Vec3::X);
        assert_eq!(conv.ctype, ConveyorType::Belt);
        assert_eq!(conv.speed, 2.0);
        assert_eq!(conv.length, 10.0);
        assert!(conv.items.is_empty());
    }

    #[test]
    fn test_conveyor_item_movement() {
        let mut conv = Conveyor::new(ConveyorType::Roller, 3.0, 10.0, Vec3::X);
        conv.add_item("iron_ore", 5.0);
        conv.add_item("coal", 3.0);
        assert_eq!(conv.items.len(), 2);
        conv.update(1.0);
        assert!(conv.items[0].position > 0.0);
        assert!(conv.power_consumption > 0.0);
    }

    #[test]
    fn test_conveyor_network() {
        let mut network = ConveyorNetwork::new();
        let c1 = Conveyor::new(ConveyorType::Belt, 2.0, 5.0, Vec3::X);
        let c2 = Conveyor::new(ConveyorType::Belt, 2.0, 5.0, Vec3::X);
        let idx1 = network.add_conveyor(c1);
        let idx2 = network.add_conveyor(c2);
        network.connect(idx1, idx2);
        network.conveyors[idx1].add_item("ore", 10.0);
        network.update_all(1.0);
        assert!(network.total_power >= 0.0);
    }
}
