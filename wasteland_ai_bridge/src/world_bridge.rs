use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSpawnRequest {
    pub entity_type: String,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub properties: Vec<EntityProperty>,
    pub components: Vec<ComponentDef>,
    pub spawn_priority: u32,
    pub chunk_id: Option<[i32; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityProperty {
    pub name: String,
    pub value: PropertyValue,
    pub mutable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyValue {
    Float(f32),
    Int(i32),
    Bool(bool),
    String(String),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Array(Vec<PropertyValue>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDef {
    pub component_type: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSpawnResult {
    pub entity_id: u64,
    pub success: bool,
    pub chunk_id: [i32; 2],
    pub spawned_at: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeTransition {
    pub from_biome: String,
    pub to_biome: String,
    pub position: [f32; 2],
    pub transition_width: f32,
    pub blend_factor: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    pub time_of_day: f32,
    pub weather: String,
    pub temperature: f32,
    pub humidity: f32,
    pub wind: [f32; 3],
    pub radiation_level: f32,
    pub active_events: Vec<String>,
}

pub struct WorldBridge {
    pub config: WorldBridgeConfig,
    entity_counter: u64,
    chunks: Vec<ChunkInfo>,
    active_entities: Vec<ActiveEntity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldBridgeConfig {
    pub chunk_size: f32,
    pub max_entities_per_chunk: u32,
    pub spawn_radius: f32,
    pub despawn_radius: f32,
    pub lod_distance: [f32; 4],
}

impl Default for WorldBridgeConfig {
    fn default() -> Self {
        WorldBridgeConfig {
            chunk_size: 64.0,
            max_entities_per_chunk: 256,
            spawn_radius: 200.0,
            despawn_radius: 300.0,
            lod_distance: [50.0, 100.0, 200.0, 400.0],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChunkInfo {
    pub id: [i32; 2],
    pub entity_count: u32,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveEntity {
    pub id: u64,
    pub entity_type: String,
    pub position: [f32; 3],
    pub chunk_id: [i32; 2],
    pub lod_level: u32,
}

impl WorldBridge {
    pub fn new(config: WorldBridgeConfig) -> Self {
        WorldBridge { config, entity_counter: 0, chunks: Vec::new(), active_entities: Vec::new() }
    }

    pub fn spawn_entity(&mut self, request: &WorldSpawnRequest) -> WorldSpawnResult {
        let chunk_id = request.chunk_id.unwrap_or([
            (request.position[0] / self.config.chunk_size).floor() as i32,
            (request.position[2] / self.config.chunk_size).floor() as i32,
        ]);
        if let Some(chunk) = self.chunks.iter().find(|c| c.id == chunk_id) {
            if chunk.entity_count >= self.config.max_entities_per_chunk {
                return WorldSpawnResult {
                    entity_id: 0,
                    success: false,
                    chunk_id,
                    spawned_at: 0,
                    error: Some(format!("chunk {:?} is full", chunk_id)),
                };
            }
        }
        self.entity_counter += 1;
        let entity_id = self.entity_counter;
        if let Some(chunk) = self.chunks.iter_mut().find(|c| c.id == chunk_id) {
            chunk.entity_count += 1;
        } else {
            self.chunks.push(ChunkInfo { id: chunk_id, entity_count: 1, active: true });
        }
        self.active_entities.push(ActiveEntity {
            id: entity_id,
            entity_type: request.entity_type.clone(),
            position: request.position,
            chunk_id,
            lod_level: 0,
        });
        WorldSpawnResult { entity_id, success: true, chunk_id, spawned_at: 0, error: None }
    }

    pub fn despawn_entity(&mut self, entity_id: u64) -> bool {
        if let Some(pos) = self.active_entities.iter().position(|e| e.id == entity_id) {
            let entity = self.active_entities.remove(pos);
            if let Some(chunk) = self.chunks.iter_mut().find(|c| c.id == entity.chunk_id) {
                chunk.entity_count = chunk.entity_count.saturating_sub(1);
            }
            true
        } else {
            false
        }
    }

    pub fn update_lod(&mut self, entity_id: u64, viewer_position: [f32; 3]) -> Option<u32> {
        let entity = self.active_entities.iter_mut().find(|e| e.id == entity_id)?;
        let dx = entity.position[0] - viewer_position[0];
        let dy = entity.position[1] - viewer_position[1];
        let dz = entity.position[2] - viewer_position[2];
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();
        let new_lod = if distance < self.config.lod_distance[0] {
            0
        } else if distance < self.config.lod_distance[1] {
            1
        } else if distance < self.config.lod_distance[2] {
            2
        } else {
            3
        };
        entity.lod_level = new_lod;
        Some(new_lod)
    }

    pub fn query_nearby(
        &self,
        position: [f32; 3],
        radius: f32,
        entity_type: Option<&str>,
    ) -> Vec<&ActiveEntity> {
        let r2 = radius * radius;
        self.active_entities
            .iter()
            .filter(|e| {
                let dx = e.position[0] - position[0];
                let dy = e.position[1] - position[1];
                let dz = e.position[2] - position[2];
                let in_range = dx * dx + dy * dy + dz * dz <= r2;
                let type_match = entity_type.is_none_or(|t| e.entity_type == t);
                in_range && type_match
            })
            .collect()
    }

    pub fn chunk_stats(&self) -> Vec<ChunkStats> {
        self.chunks
            .iter()
            .map(|c| ChunkStats { chunk_id: c.id, entity_count: c.entity_count, active: c.active })
            .collect()
    }

    pub fn total_entities(&self) -> usize {
        self.active_entities.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkStats {
    pub chunk_id: [i32; 2],
    pub entity_count: u32,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentUpdate {
    pub time_delta: f32,
    pub weather_change: Option<WeatherChange>,
    pub temperature_delta: f32,
    pub humidity_delta: f32,
    pub radiation_delta: f32,
    pub events: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherChange {
    pub from: String,
    pub to: String,
    pub transition_progress: f32,
    pub intensity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSpawnRequest {
    pub resource_type: String,
    pub position: [f32; 3],
    pub quantity: f32,
    pub quality: f32,
    pub respawn_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructurePlacement {
    pub structure_type: String,
    pub position: [f32; 3],
    pub rotation: f32,
    pub scale: [f32; 3],
    pub integrity: f32,
    pub blueprint_data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_entity() {
        let mut bridge = WorldBridge::new(WorldBridgeConfig::default());
        let request = WorldSpawnRequest {
            entity_type: "npc_scavenger".into(),
            position: [10.0, 0.0, 20.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            properties: vec![],
            components: vec![],
            spawn_priority: 1,
            chunk_id: None,
        };
        let result = bridge.spawn_entity(&request);
        assert!(result.success);
        assert_eq!(result.entity_id, 1);
        assert_eq!(bridge.total_entities(), 1);
    }

    #[test]
    fn test_despawn_entity() {
        let mut bridge = WorldBridge::new(WorldBridgeConfig::default());
        let request = WorldSpawnRequest {
            entity_type: "test".into(),
            position: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            properties: vec![],
            components: vec![],
            spawn_priority: 1,
            chunk_id: None,
        };
        let result = bridge.spawn_entity(&request);
        assert!(bridge.despawn_entity(result.entity_id));
        assert_eq!(bridge.total_entities(), 0);
    }

    #[test]
    fn test_update_lod() {
        let mut bridge = WorldBridge::new(WorldBridgeConfig::default());
        let request = WorldSpawnRequest {
            entity_type: "test".into(),
            position: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            properties: vec![],
            components: vec![],
            spawn_priority: 1,
            chunk_id: None,
        };
        let result = bridge.spawn_entity(&request);
        let lod = bridge.update_lod(result.entity_id, [150.0, 0.0, 0.0]);
        assert_eq!(lod, Some(2));
    }

    #[test]
    fn test_query_nearby() {
        let mut bridge = WorldBridge::new(WorldBridgeConfig::default());
        bridge.spawn_entity(&WorldSpawnRequest {
            entity_type: "npc".into(),
            position: [10.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            properties: vec![],
            components: vec![],
            spawn_priority: 1,
            chunk_id: None,
        });
        bridge.spawn_entity(&WorldSpawnRequest {
            entity_type: "creature".into(),
            position: [100.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            properties: vec![],
            components: vec![],
            spawn_priority: 1,
            chunk_id: None,
        });
        let nearby = bridge.query_nearby([0.0, 0.0, 0.0], 20.0, None);
        assert_eq!(nearby.len(), 1);
        let filtered = bridge.query_nearby([0.0, 0.0, 0.0], 200.0, Some("creature"));
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_chunk_full() {
        let mut bridge =
            WorldBridge::new(WorldBridgeConfig { max_entities_per_chunk: 2, ..Default::default() });
        let spawn = |i: f32| WorldSpawnRequest {
            entity_type: "test".into(),
            position: [i, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            properties: vec![],
            components: vec![],
            spawn_priority: 1,
            chunk_id: None,
        };
        assert!(bridge.spawn_entity(&spawn(0.0)).success);
        assert!(bridge.spawn_entity(&spawn(1.0)).success);
        let third = bridge.spawn_entity(&spawn(2.0));
        assert!(!third.success);
    }
}
