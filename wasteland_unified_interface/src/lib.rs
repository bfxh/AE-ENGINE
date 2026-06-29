use glam::Vec3;
use slotmap::{SlotMap, new_key_type};
use uuid::Uuid;
use wasteland_metaentity::meta_entity::{EntityChanges, MetaEntity};

new_key_type! {
    pub struct EntityKey;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldId {
    Position,
    Velocity,
    AngularVelocity,
    Mass,
    Density,
    Hardness,
    Toughness,
    ElasticModulus,
    YieldStrength,
    UltimateStrength,
    Temperature,
    Reactivity,
    Ph,
    OxidationState,
    CorrosionDepth,
    Health,
    MetabolicRate,
    GrowthRate,
    State,
    Extension(u64),
}

#[derive(Debug, Clone)]
pub enum WriteError {
    EntityNotFound,
    VersionMismatch { expected: u64, actual: u64 },
    InvalidField,
    StorageFull,
}

pub trait UnifiedWorld: Send + Sync {
    fn read_entity(&self, id: Uuid) -> Option<&MetaEntity>;
    fn read_entity_mut(&mut self, id: Uuid) -> Option<&mut MetaEntity>;
    fn write_entity(&mut self, id: Uuid, changes: EntityChanges) -> Result<u64, WriteError>;
    fn query_entities(&self, predicate: &dyn Fn(&MetaEntity) -> bool) -> Vec<&MetaEntity>;
    fn query_entities_mut(
        &mut self,
        predicate: &dyn Fn(&MetaEntity) -> bool,
    ) -> Vec<&mut MetaEntity>;
    fn spawn_entity(&mut self, entity: MetaEntity) -> Uuid;
    fn despawn_entity(&mut self, id: Uuid);
    fn entity_count(&self) -> usize;
    fn spatial_query(&self, center: Vec3, radius: f32) -> Vec<&MetaEntity>;
}

pub struct WorldStorage {
    entities: SlotMap<EntityKey, MetaEntity>,
    id_to_key: hashbrown::HashMap<Uuid, EntityKey>,
}

impl WorldStorage {
    pub fn new() -> Self {
        Self { entities: SlotMap::with_key(), id_to_key: hashbrown::HashMap::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entities: SlotMap::with_capacity_and_key(capacity),
            id_to_key: hashbrown::HashMap::with_capacity(capacity),
        }
    }

    pub fn get(&self, id: Uuid) -> Option<&MetaEntity> {
        self.id_to_key.get(&id).and_then(|key| self.entities.get(*key))
    }

    pub fn get_mut(&mut self, id: Uuid) -> Option<&mut MetaEntity> {
        self.id_to_key.get(&id).and_then(|key| self.entities.get_mut(*key))
    }

    pub fn insert(&mut self, entity: MetaEntity) -> Uuid {
        let id = entity.id;
        let key = self.entities.insert(entity);
        self.id_to_key.insert(id, key);
        id
    }

    pub fn remove(&mut self, id: Uuid) -> Option<MetaEntity> {
        if let Some(key) = self.id_to_key.remove(&id) { self.entities.remove(key) } else { None }
    }

    pub fn iter(&self) -> impl Iterator<Item = &MetaEntity> {
        self.entities.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut MetaEntity> {
        self.entities.values_mut()
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn spatial_query(&self, center: Vec3, radius: f32) -> Vec<&MetaEntity> {
        let r2 = radius * radius;
        self.entities.values().filter(|e| (e.position - center).length_squared() <= r2).collect()
    }
}

impl Default for WorldStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl UnifiedWorld for WorldStorage {
    fn read_entity(&self, id: Uuid) -> Option<&MetaEntity> {
        self.get(id)
    }

    fn read_entity_mut(&mut self, id: Uuid) -> Option<&mut MetaEntity> {
        self.get_mut(id)
    }

    fn write_entity(&mut self, id: Uuid, changes: EntityChanges) -> Result<u64, WriteError> {
        let expected_version = changes.expected_version;
        let entity = self.get_mut(id).ok_or(WriteError::EntityNotFound)?;
        entity.apply_changes(changes).map_err(|_| WriteError::VersionMismatch {
            expected: expected_version,
            actual: entity.version,
        })
    }

    fn query_entities(&self, predicate: &dyn Fn(&MetaEntity) -> bool) -> Vec<&MetaEntity> {
        self.entities.values().filter(|e| predicate(e)).collect()
    }

    fn query_entities_mut(
        &mut self,
        predicate: &dyn Fn(&MetaEntity) -> bool,
    ) -> Vec<&mut MetaEntity> {
        self.entities.values_mut().filter(|e| predicate(e)).collect()
    }

    fn spawn_entity(&mut self, entity: MetaEntity) -> Uuid {
        self.insert(entity)
    }

    fn despawn_entity(&mut self, id: Uuid) {
        self.remove(id);
    }

    fn entity_count(&self) -> usize {
        self.len()
    }

    fn spatial_query(&self, center: Vec3, radius: f32) -> Vec<&MetaEntity> {
        WorldStorage::spatial_query(self, center, radius)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_storage_spawn() {
        let mut world = WorldStorage::new();
        let iron = MetaEntity::iron(Vec3::ZERO, 0);
        let id = world.spawn_entity(iron);
        assert_eq!(world.entity_count(), 1);
        assert!(world.read_entity(id).is_some());
    }

    #[test]
    fn test_world_storage_write() {
        let mut world = WorldStorage::new();
        let iron = MetaEntity::iron(Vec3::ZERO, 0);
        let id = world.spawn_entity(iron);

        let changes = EntityChanges {
            entity_id: id,
            expected_version: 0,
            position: Some(Vec3::new(10.0, 0.0, 0.0)),
            velocity: None,
            angular_velocity: None,
            physics_changes: hashbrown::HashMap::new(),
            chemistry_changes: hashbrown::HashMap::new(),
            biology_changes: hashbrown::HashMap::new(),
            state_change: None,
            extension_changes: hashbrown::HashMap::new(),
            spawn_children: vec![],
            despawn: false,
        };
        let result = world.write_entity(id, changes);
        assert!(result.is_ok());
        assert_eq!(world.read_entity(id).unwrap().position.x, 10.0);
    }

    #[test]
    fn test_world_storage_despawn() {
        let mut world = WorldStorage::new();
        let iron = MetaEntity::iron(Vec3::ZERO, 0);
        let id = world.spawn_entity(iron);
        world.despawn_entity(id);
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn test_spatial_query() {
        let mut world = WorldStorage::new();
        world.spawn_entity(MetaEntity::iron(Vec3::ZERO, 0));
        world.spawn_entity(MetaEntity::iron(Vec3::new(100.0, 0.0, 0.0), 0));
        let nearby = world.spatial_query(Vec3::ZERO, 10.0);
        assert_eq!(nearby.len(), 1);
    }

    #[test]
    fn test_query_entities() {
        let mut world = WorldStorage::new();
        world.spawn_entity(MetaEntity::iron(Vec3::ZERO, 0));
        world.spawn_entity(MetaEntity::water(Vec3::new(1.0, 0.0, 0.0), 0));
        let irons = world.query_entities(&|e| e.physics.density > 5000.0);
        assert_eq!(irons.len(), 1);
    }

    #[test]
    fn test_version_mismatch() {
        let mut world = WorldStorage::new();
        let mut iron = MetaEntity::iron(Vec3::ZERO, 0);
        iron.version = 5;
        let id = world.spawn_entity(iron);

        let changes = EntityChanges {
            entity_id: id,
            expected_version: 0,
            position: None,
            velocity: None,
            angular_velocity: None,
            physics_changes: hashbrown::HashMap::new(),
            chemistry_changes: hashbrown::HashMap::new(),
            biology_changes: hashbrown::HashMap::new(),
            state_change: None,
            extension_changes: hashbrown::HashMap::new(),
            spawn_children: vec![],
            despawn: false,
        };
        assert!(world.write_entity(id, changes).is_err());
    }
}
