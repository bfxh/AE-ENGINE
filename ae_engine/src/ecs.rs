use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentType {
    Position,
    Velocity,
    Mass,
    Material,
    Temperature,
    Health,
    ChemicalState,
    BiologicalState,
    RadiationLevel,
    ReactionProgress,
    FieldValue,
    Stress,
    Strain,
    CorrosionDepth,
    // 游戏实体组件（P0/P1 优先级，用于统一 NPC/CombatEntity/Projectile 查询）
    Faction,           // u8 — 阵营（敌我判定）
    MaxHealth,         // f32 — 最大生命
    Lifetime,          // f32 — 寿命上限（投射物/粒子）
    Age,               // f32 — 已存活时间
    AnimationState,    // u8 — 动画状态（映射 AnimationState::as_u8）
    AttackCooldown,    // f32 — 攻击冷却剩余
    Radius,            // f32 — 碰撞/感知半径
    MaxSpeed,          // f32 — 最大移动速度
    Target,            // Uuid — 当前目标实体
    EntityKind,        // u8 — 实体种类（NPC=0, CombatEntity=1, Projectile=2, DamageZone=3, Particle=4）
    Custom(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentValue {
    Vec3(Vec3),
    Float32(f32),
    Float64(f64),
    Int32(i32),
    Uint32(u32),
    Uint8(u8),
    Bool(bool),
    Uuid(Uuid),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub component_type: ComponentType,
    pub value: ComponentValue,
    pub last_modified_by: SystemId,
    pub last_modified_tick: u64,
    pub version: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SystemId {
    Physics,
    Chemistry,
    Biology,
    Field,
    Particle,
    Emergence,
    Player,
    External(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: Uuid,
    pub components: HashMap<ComponentType, Component>,
    pub active: bool,
    pub spawn_tick: u64,
}

impl Entity {
    pub fn new(id: Uuid, spawn_tick: u64) -> Self {
        Self { id, components: HashMap::new(), active: true, spawn_tick }
    }

    pub fn get_f32(&self, component_type: ComponentType) -> Option<f32> {
        self.components
            .get(&component_type)
            .and_then(|c| if let ComponentValue::Float32(v) = c.value { Some(v) } else { None })
    }

    pub fn get_vec3(&self, component_type: ComponentType) -> Option<Vec3> {
        self.components
            .get(&component_type)
            .and_then(|c| if let ComponentValue::Vec3(v) = c.value { Some(v) } else { None })
    }

    pub fn get_u8(&self, component_type: ComponentType) -> Option<u8> {
        self.components
            .get(&component_type)
            .and_then(|c| if let ComponentValue::Uint8(v) = c.value { Some(v) } else { None })
    }

    pub fn get_uuid(&self, component_type: ComponentType) -> Option<Uuid> {
        self.components
            .get(&component_type)
            .and_then(|c| if let ComponentValue::Uuid(v) = c.value { Some(v) } else { None })
    }

    pub fn set_f32(
        &mut self,
        component_type: ComponentType,
        value: f32,
        system: SystemId,
        tick: u64,
    ) {
        let version = self.components.get(&component_type).map(|c| c.version + 1).unwrap_or(1);
        self.components.insert(
            component_type,
            Component {
                component_type,
                value: ComponentValue::Float32(value),
                last_modified_by: system,
                last_modified_tick: tick,
                version,
            },
        );
    }

    pub fn set_vec3(
        &mut self,
        component_type: ComponentType,
        value: Vec3,
        system: SystemId,
        tick: u64,
    ) {
        let version = self.components.get(&component_type).map(|c| c.version + 1).unwrap_or(1);
        self.components.insert(
            component_type,
            Component {
                component_type,
                value: ComponentValue::Vec3(value),
                last_modified_by: system,
                last_modified_tick: tick,
                version,
            },
        );
    }

    pub fn set_f64(
        &mut self,
        component_type: ComponentType,
        value: f64,
        system: SystemId,
        tick: u64,
    ) {
        let version = self.components.get(&component_type).map(|c| c.version + 1).unwrap_or(1);
        self.components.insert(
            component_type,
            Component {
                component_type,
                value: ComponentValue::Float64(value),
                last_modified_by: system,
                last_modified_tick: tick,
                version,
            },
        );
    }

    pub fn set_bool(
        &mut self,
        component_type: ComponentType,
        value: bool,
        system: SystemId,
        tick: u64,
    ) {
        let version = self.components.get(&component_type).map(|c| c.version + 1).unwrap_or(1);
        self.components.insert(
            component_type,
            Component {
                component_type,
                value: ComponentValue::Bool(value),
                last_modified_by: system,
                last_modified_tick: tick,
                version,
            },
        );
    }

    pub fn set_u8(
        &mut self,
        component_type: ComponentType,
        value: u8,
        system: SystemId,
        tick: u64,
    ) {
        let version = self.components.get(&component_type).map(|c| c.version + 1).unwrap_or(1);
        self.components.insert(
            component_type,
            Component {
                component_type,
                value: ComponentValue::Uint8(value),
                last_modified_by: system,
                last_modified_tick: tick,
                version,
            },
        );
    }

    pub fn set_uuid(
        &mut self,
        component_type: ComponentType,
        value: Uuid,
        system: SystemId,
        tick: u64,
    ) {
        let version = self.components.get(&component_type).map(|c| c.version + 1).unwrap_or(1);
        self.components.insert(
            component_type,
            Component {
                component_type,
                value: ComponentValue::Uuid(value),
                last_modified_by: system,
                last_modified_tick: tick,
                version,
            },
        );
    }

    pub fn last_modified_by(&self, component_type: ComponentType) -> Option<SystemId> {
        self.components.get(&component_type).map(|c| c.last_modified_by)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcsWorld {
    pub entities: HashMap<Uuid, Entity>,
    pub component_index: HashMap<ComponentType, Vec<Uuid>>,
    pub tick: u64,
}

impl EcsWorld {
    pub fn new() -> Self {
        Self { entities: HashMap::new(), component_index: HashMap::new(), tick: 0 }
    }

    pub fn spawn(&mut self) -> Uuid {
        let id = Uuid::new_v4();
        self.entities.insert(id, Entity::new(id, self.tick));
        id
    }

    pub fn despawn(&mut self, id: Uuid) {
        if let Some(entity) = self.entities.remove(&id) {
            for component_type in entity.components.keys() {
                if let Some(ids) = self.component_index.get_mut(component_type) {
                    ids.retain(|eid| *eid != id);
                }
            }
        }
    }

    pub fn entity(&self, id: Uuid) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn entity_mut(&mut self, id: Uuid) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    pub fn set_component(
        &mut self,
        entity_id: Uuid,
        component_type: ComponentType,
        value: ComponentValue,
        system: SystemId,
    ) {
        if let Some(entity) = self.entities.get_mut(&entity_id) {
            let version =
                entity.components.get(&component_type).map(|c| c.version + 1).unwrap_or(1);
            entity.components.insert(
                component_type,
                Component {
                    component_type,
                    value,
                    last_modified_by: system,
                    last_modified_tick: self.tick,
                    version,
                },
            );
            // Fix: Only add entity_id to component_index if not already present.
            // set_component is called repeatedly for updates (e.g., Transform every frame);
            // without this check, component_index grows unboundedly with duplicate UUIDs,
            // causing query_entities_with to return duplicate results.
            let idx = self.component_index.entry(component_type).or_default();
            if !idx.contains(&entity_id) {
                idx.push(entity_id);
            }
        }
    }

    pub fn get_component(
        &self,
        entity_id: Uuid,
        component_type: ComponentType,
    ) -> Option<&ComponentValue> {
        self.entities
            .get(&entity_id)
            .and_then(|e| e.components.get(&component_type))
            .map(|c| &c.value)
    }

    pub fn query_entities_with(&self, component_type: ComponentType) -> Vec<Uuid> {
        self.component_index.get(&component_type).cloned().unwrap_or_default()
    }

    pub fn query_entities_with_all(&self, component_types: &[ComponentType]) -> Vec<Uuid> {
        self.entities
            .iter()
            .filter(|(_, e)| component_types.iter().all(|ct| e.components.contains_key(ct)))
            .map(|(id, _)| *id)
            .collect()
    }

    /// 按阵营查询实体（敌我判定的统一入口）。
    pub fn query_by_faction(&self, faction: u8) -> Vec<Uuid> {
        self.entities
            .iter()
            .filter(|(_, e)| e.get_u8(ComponentType::Faction) == Some(faction))
            .map(|(id, _)| *id)
            .collect()
    }

    /// 按实体种类查询（NPC=0, CombatEntity=1, Projectile=2, DamageZone=3, Particle=4）。
    pub fn query_by_kind(&self, kind: u8) -> Vec<Uuid> {
        self.entities
            .iter()
            .filter(|(_, e)| e.get_u8(ComponentType::EntityKind) == Some(kind))
            .map(|(id, _)| *id)
            .collect()
    }

    /// 空间范围查询（球形区域内的实体）。
    /// 性能说明：线性扫描所有实体，适合 <1000 实体；大规模空间查询应用 NpcManager.spatial_hash。
    pub fn query_in_range(&self, center: Vec3, radius: f32) -> Vec<Uuid> {
        let r_sq = radius * radius;
        self.entities
            .iter()
            .filter(|(_, e)| {
                e.get_vec3(ComponentType::Position)
                    .map(|pos| (pos - center).length_squared() <= r_sq)
                    .unwrap_or(false)
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// 查询某阵营在球形区域内的实体（query_by_faction + query_in_range 组合）。
    pub fn query_faction_in_range(&self, faction: u8, center: Vec3, radius: f32) -> Vec<Uuid> {
        let r_sq = radius * radius;
        self.entities
            .iter()
            .filter(|(_, e)| {
                e.get_u8(ComponentType::Faction) == Some(faction)
                    && e.get_vec3(ComponentType::Position)
                        .map(|pos| (pos - center).length_squared() <= r_sq)
                        .unwrap_or(false)
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// 按实体种类批量注销（用于短生命周期实体如 Projectile/DamageZone 的每帧重建）。
    pub fn despawn_by_kind(&mut self, kind: u8) {
        let ids: Vec<Uuid> = self
            .entities
            .iter()
            .filter(|(_, e)| e.get_u8(ComponentType::EntityKind) == Some(kind))
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            self.despawn(id);
        }
    }

    pub fn advance_tick(&mut self) {
        self.tick += 1;
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    pub fn stats(&self) -> EcsStats {
        let total_components: usize = self.entities.values().map(|e| e.components.len()).sum();
        EcsStats {
            entity_count: self.entities.len(),
            component_count: total_components,
            indexed_types: self.component_index.len(),
            tick: self.tick,
        }
    }
}

impl Default for EcsWorld {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct EcsStats {
    pub entity_count: usize,
    pub component_count: usize,
    pub indexed_types: usize,
    pub tick: u64,
}

pub struct EcsReader<'a> {
    pub world: &'a EcsWorld,
}

impl<'a> EcsReader<'a> {
    pub fn new(world: &'a EcsWorld) -> Self {
        Self { world }
    }

    pub fn read_f32(&self, entity_id: Uuid, component_type: ComponentType) -> Option<f32> {
        self.world.entity(entity_id).and_then(|e| e.get_f32(component_type))
    }

    pub fn read_vec3(&self, entity_id: Uuid, component_type: ComponentType) -> Option<Vec3> {
        self.world.entity(entity_id).and_then(|e| e.get_vec3(component_type))
    }

    pub fn read_u8(&self, entity_id: Uuid, component_type: ComponentType) -> Option<u8> {
        self.world.entity(entity_id).and_then(|e| e.get_u8(component_type))
    }

    pub fn read_uuid(&self, entity_id: Uuid, component_type: ComponentType) -> Option<Uuid> {
        self.world.entity(entity_id).and_then(|e| e.get_uuid(component_type))
    }
}

pub struct EcsWriter<'a> {
    pub world: &'a mut EcsWorld,
}

impl<'a> EcsWriter<'a> {
    pub fn new(world: &'a mut EcsWorld) -> Self {
        Self { world }
    }

    pub fn write_f32(
        &mut self,
        entity_id: Uuid,
        component_type: ComponentType,
        value: f32,
        system: SystemId,
    ) {
        self.world.set_component(entity_id, component_type, ComponentValue::Float32(value), system);
    }

    pub fn write_vec3(
        &mut self,
        entity_id: Uuid,
        component_type: ComponentType,
        value: Vec3,
        system: SystemId,
    ) {
        self.world.set_component(entity_id, component_type, ComponentValue::Vec3(value), system);
    }

    pub fn write_u8(
        &mut self,
        entity_id: Uuid,
        component_type: ComponentType,
        value: u8,
        system: SystemId,
    ) {
        self.world.set_component(entity_id, component_type, ComponentValue::Uint8(value), system);
    }

    pub fn write_uuid(
        &mut self,
        entity_id: Uuid,
        component_type: ComponentType,
        value: Uuid,
        system: SystemId,
    ) {
        self.world.set_component(entity_id, component_type, ComponentValue::Uuid(value), system);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecs_spawn_and_query() {
        let mut world = EcsWorld::new();
        let entity_id = world.spawn();
        world.set_component(
            entity_id,
            ComponentType::Temperature,
            ComponentValue::Float32(300.0),
            SystemId::Physics,
        );
        world.set_component(
            entity_id,
            ComponentType::Position,
            ComponentValue::Vec3(Vec3::new(1.0, 2.0, 3.0)),
            SystemId::Physics,
        );

        let temp = world.entity(entity_id).unwrap().get_f32(ComponentType::Temperature);
        assert_eq!(temp, Some(300.0));

        let entities = world.query_entities_with(ComponentType::Temperature);
        assert_eq!(entities.len(), 1);
    }

    #[test]
    fn test_game_entity_faction_query() {
        let mut world = EcsWorld::new();
        // 生成 3 个实体：2 个阵营 0，1 个阵营 1
        let e1 = world.spawn();
        world.set_component(e1, ComponentType::Faction, ComponentValue::Uint8(0), SystemId::External(0));
        world.set_component(e1, ComponentType::Position, ComponentValue::Vec3(Vec3::new(0.0, 0.0, 0.0)), SystemId::External(0));
        world.set_component(e1, ComponentType::EntityKind, ComponentValue::Uint8(0), SystemId::External(0)); // NPC

        let e2 = world.spawn();
        world.set_component(e2, ComponentType::Faction, ComponentValue::Uint8(0), SystemId::External(0));
        world.set_component(e2, ComponentType::Position, ComponentValue::Vec3(Vec3::new(5.0, 0.0, 0.0)), SystemId::External(0));
        world.set_component(e2, ComponentType::EntityKind, ComponentValue::Uint8(1), SystemId::External(0)); // CombatEntity

        let e3 = world.spawn();
        world.set_component(e3, ComponentType::Faction, ComponentValue::Uint8(1), SystemId::External(0));
        world.set_component(e3, ComponentType::Position, ComponentValue::Vec3(Vec3::new(100.0, 0.0, 0.0)), SystemId::External(0));
        world.set_component(e3, ComponentType::EntityKind, ComponentValue::Uint8(0), SystemId::External(0)); // NPC

        // 按阵营查询
        let faction_0 = world.query_by_faction(0);
        assert_eq!(faction_0.len(), 2);
        let faction_1 = world.query_by_faction(1);
        assert_eq!(faction_1.len(), 1);

        // 按种类查询
        let npcs = world.query_by_kind(0);
        assert_eq!(npcs.len(), 2);
        let combat_entities = world.query_by_kind(1);
        assert_eq!(combat_entities.len(), 1);

        // 空间范围查询（半径 10m 内的实体）
        let nearby = world.query_in_range(Vec3::ZERO, 10.0);
        assert_eq!(nearby.len(), 2); // e1 和 e2

        // 组合查询：阵营 0 在 10m 内
        let friendly_nearby = world.query_faction_in_range(0, Vec3::ZERO, 10.0);
        assert_eq!(friendly_nearby.len(), 2);

        // 组合查询：阵营 1 在 10m 内（应为空，e3 在 100m 外）
        let enemy_nearby = world.query_faction_in_range(1, Vec3::ZERO, 10.0);
        assert_eq!(enemy_nearby.len(), 0);
    }

    #[test]
    fn test_u8_and_uuid_components() {
        let mut world = EcsWorld::new();
        let entity_id = world.spawn();

        // u8 组件（Faction）
        world.set_component(entity_id, ComponentType::Faction, ComponentValue::Uint8(5), SystemId::External(0));
        assert_eq!(world.entity(entity_id).unwrap().get_u8(ComponentType::Faction), Some(5));

        // Uuid 组件（Target）
        let target_id = Uuid::new_v4();
        world.set_component(entity_id, ComponentType::Target, ComponentValue::Uuid(target_id), SystemId::External(0));
        assert_eq!(world.entity(entity_id).unwrap().get_uuid(ComponentType::Target), Some(target_id));
    }
}
