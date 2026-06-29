//! 战斗系统
//!
//! 游戏层战斗逻辑：实体/武器/伤害/投射物/区域效果

pub mod optimizer;

use glam::Vec3;
use serde::{Deserialize, Serialize};
use slotmap::SlotMap;

slotmap::new_key_type! { pub struct CombatEntityId; }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CombatState {
    Idle,
    Engaging,
    Attacking,
    Defending,
    Retreating,
    Dead,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatEntity {
    pub id: CombatEntityId,
    pub position: Vec3,
    pub velocity: Vec3,
    pub health: f32,
    pub max_health: f32,
    pub faction: u8,
    pub damage_multiplier: f32,
    pub armor: f32,
    pub attack_range: f32,
    pub attack_damage: f32,
    pub attack_cooldown: f32,
    pub current_cooldown: f32,
    pub state: CombatState,
    pub target: Option<CombatEntityId>,
}

impl CombatEntity {
    pub fn new(position: Vec3, faction: u8, max_health: f32) -> Self {
        Self {
            id: CombatEntityId::default(),
            position,
            velocity: Vec3::ZERO,
            health: max_health,
            max_health,
            faction,
            damage_multiplier: 1.0,
            armor: 0.0,
            attack_range: 5.0,
            attack_damage: 10.0,
            attack_cooldown: 1.0,
            current_cooldown: 0.0,
            state: CombatState::Idle,
            target: None,
        }
    }

    pub fn apply_damage(&mut self, raw_damage: f32) -> f32 {
        if self.state == CombatState::Dead {
            return 0.0;
        }
        let mitigated = raw_damage * (1.0 - self.armor.clamp(0.0, 0.95));
        self.health = (self.health - mitigated).max(0.0);
        if self.health <= 0.0 {
            self.state = CombatState::Dead;
        }
        mitigated
    }

    pub fn heal(&mut self, amount: f32) {
        if self.state == CombatState::Dead {
            return;
        }
        self.health = (self.health + amount).min(self.max_health);
    }

    pub fn is_alive(&self) -> bool {
        self.state != CombatState::Dead && self.health > 0.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Projectile {
    pub id: u64,
    pub position: Vec3,
    pub velocity: Vec3,
    pub damage: f32,
    pub faction: u8,
    pub lifetime: f32,
    pub max_lifetime: f32,
}

impl Projectile {
    pub fn new(id: u64, origin: Vec3, velocity: Vec3, damage: f32, faction: u8) -> Self {
        Self { id, position: origin, velocity, damage, faction, lifetime: 0.0, max_lifetime: 5.0 }
    }

    pub fn update(&mut self, dt: f32) -> bool {
        self.position += self.velocity * dt;
        self.lifetime += dt;
        self.lifetime < self.max_lifetime
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageZone {
    pub center: Vec3,
    pub radius: f32,
    pub damage_per_second: f32,
    pub faction: u8,
    pub duration: f32,
    pub elapsed: f32,
}

impl DamageZone {
    pub fn new(
        center: Vec3,
        radius: f32,
        damage_per_second: f32,
        faction: u8,
        duration: f32,
    ) -> Self {
        Self { center, radius, damage_per_second, faction, duration, elapsed: 0.0 }
    }

    pub fn update(&mut self, dt: f32) -> bool {
        self.elapsed += dt;
        self.elapsed < self.duration
    }

    pub fn contains(&self, pos: Vec3) -> bool {
        (pos - self.center).length_squared() <= self.radius * self.radius
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CombatEvent {
    EntityDamaged { id: CombatEntityId, damage: f32, source: Option<CombatEntityId> },
    EntityKilled { id: CombatEntityId, killer: Option<CombatEntityId> },
    ProjectileFired { id: u64, faction: u8 },
    ProjectileHit { id: u64, target: CombatEntityId },
}

pub struct CombatSystem {
    pub entities: SlotMap<CombatEntityId, CombatEntity>,
    pub projectiles: Vec<Projectile>,
    pub damage_zones: Vec<DamageZone>,
    pub events: Vec<CombatEvent>,
    pub next_projectile_id: u64,
}

impl Default for CombatSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl CombatSystem {
    pub fn new() -> Self {
        Self {
            entities: SlotMap::with_key(),
            projectiles: Vec::new(),
            damage_zones: Vec::new(),
            events: Vec::new(),
            next_projectile_id: 0,
        }
    }

    pub fn spawn_entity(&mut self, entity: CombatEntity) -> CombatEntityId {
        self.entities.insert(entity)
    }

    pub fn remove_entity(&mut self, id: CombatEntityId) {
        self.entities.remove(id);
    }

    pub fn get_entity(&self, id: CombatEntityId) -> Option<&CombatEntity> {
        self.entities.get(id)
    }

    pub fn get_entity_mut(&mut self, id: CombatEntityId) -> Option<&mut CombatEntity> {
        self.entities.get_mut(id)
    }

    pub fn fire_projectile(
        &mut self,
        origin: Vec3,
        velocity: Vec3,
        damage: f32,
        faction: u8,
    ) -> u64 {
        let id = self.next_projectile_id;
        self.next_projectile_id += 1;
        self.projectiles.push(Projectile::new(id, origin, velocity, damage, faction));
        self.events.push(CombatEvent::ProjectileFired { id, faction });
        id
    }

    pub fn create_damage_zone(&mut self, zone: DamageZone) {
        self.damage_zones.push(zone);
    }

    pub fn update(&mut self, dt: f32) {
        let mut dead_projectiles = Vec::new();
        for (i, proj) in self.projectiles.iter_mut().enumerate() {
            if !proj.update(dt) {
                dead_projectiles.push(i);
            }
        }
        for &i in dead_projectiles.iter().rev() {
            self.projectiles.swap_remove(i);
        }

        let mut expired_zones = Vec::new();
        for (i, zone) in self.damage_zones.iter_mut().enumerate() {
            if !zone.update(dt) {
                expired_zones.push(i);
            }
        }
        for &i in expired_zones.iter().rev() {
            self.damage_zones.swap_remove(i);
        }

        for entity in self.entities.values_mut() {
            if entity.current_cooldown > 0.0 {
                entity.current_cooldown -= dt;
            }
            entity.position += entity.velocity * dt;
        }

        for zone in &self.damage_zones {
            let zone_damage = zone.damage_per_second * dt;
            for (id, entity) in &mut self.entities {
                if entity.faction == zone.faction || !entity.is_alive() {
                    continue;
                }
                if zone.contains(entity.position) {
                    let actual = entity.apply_damage(zone_damage);
                    if actual > 0.0 {
                        self.events.push(CombatEvent::EntityDamaged {
                            id,
                            damage: actual,
                            source: None,
                        });
                    }
                    if !entity.is_alive() {
                        self.events.push(CombatEvent::EntityKilled { id, killer: None });
                    }
                }
            }
        }

        let mut projectile_hits = Vec::new();
        for (pi, proj) in self.projectiles.iter().enumerate() {
            for (eid, entity) in &self.entities {
                if entity.faction == proj.faction || !entity.is_alive() {
                    continue;
                }
                let dist = (entity.position - proj.position).length();
                if dist < 1.0 {
                    projectile_hits.push((pi, eid));
                    break;
                }
            }
        }
        for &(pi, eid) in &projectile_hits {
            if let Some(proj) = self.projectiles.get(pi) {
                let damage = proj.damage;
                if let Some(entity) = self.entities.get_mut(eid) {
                    let actual = entity.apply_damage(damage);
                    if actual > 0.0 {
                        self.events.push(CombatEvent::EntityDamaged {
                            id: eid,
                            damage: actual,
                            source: None,
                        });
                    }
                    if !entity.is_alive() {
                        self.events.push(CombatEvent::EntityKilled { id: eid, killer: None });
                    }
                }
                self.events
                    .push(CombatEvent::ProjectileHit { id: self.projectiles[pi].id, target: eid });
            }
        }
        let mut to_remove = Vec::new();
        for &(pi, _) in &projectile_hits {
            to_remove.push(pi);
        }
        to_remove.sort_unstable_by(|a, b| b.cmp(a));
        for i in to_remove {
            if i < self.projectiles.len() {
                self.projectiles.swap_remove(i);
            }
        }
    }

    pub fn drain_events(&mut self) -> Vec<CombatEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    pub fn projectile_count(&self) -> usize {
        self.projectiles.len()
    }

    pub fn damage_zone_count(&self) -> usize {
        self.damage_zones.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combat_entity_creation() {
        let entity = CombatEntity::new(Vec3::ZERO, 0, 100.0);
        assert_eq!(entity.health, 100.0);
        assert!(entity.is_alive());
        assert_eq!(entity.state, CombatState::Idle);
    }

    #[test]
    fn test_damage_and_death() {
        let mut entity = CombatEntity::new(Vec3::ZERO, 0, 100.0);
        let dmg = entity.apply_damage(30.0);
        assert_eq!(dmg, 30.0);
        assert_eq!(entity.health, 70.0);
        assert!(entity.is_alive());

        let _final = entity.apply_damage(70.0);
        assert_eq!(entity.state, CombatState::Dead);
        assert!(!entity.is_alive());
    }

    #[test]
    fn test_armor_mitigation() {
        let mut entity = CombatEntity::new(Vec3::ZERO, 0, 100.0);
        entity.armor = 0.5;
        let dmg = entity.apply_damage(100.0);
        assert_eq!(dmg, 50.0);
        assert_eq!(entity.health, 50.0);
    }

    #[test]
    fn test_heal() {
        let mut entity = CombatEntity::new(Vec3::ZERO, 0, 100.0);
        entity.apply_damage(50.0);
        entity.heal(30.0);
        assert_eq!(entity.health, 80.0);

        entity.heal(100.0);
        assert_eq!(entity.health, 100.0);
    }

    #[test]
    fn test_projectile_lifecycle() {
        let mut proj = Projectile::new(1, Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), 10.0, 0);
        assert!(proj.update(0.5));
        assert_eq!(proj.position.x, 0.5);
    }

    #[test]
    fn test_damage_zone() {
        let mut zone = DamageZone::new(Vec3::ZERO, 5.0, 10.0, 0, 2.0);
        assert!(zone.contains(Vec3::new(3.0, 0.0, 0.0)));
        assert!(!zone.contains(Vec3::new(6.0, 0.0, 0.0)));
        assert!(zone.update(1.0));
        assert!(!zone.update(2.0));
    }

    #[test]
    fn test_combat_system_spawn() {
        let mut system = CombatSystem::new();
        let entity = CombatEntity::new(Vec3::ZERO, 0, 100.0);
        let id = system.spawn_entity(entity);
        assert_eq!(system.entity_count(), 1);
        assert!(system.get_entity(id).is_some());
        system.remove_entity(id);
        assert_eq!(system.entity_count(), 0);
    }

    #[test]
    fn test_combat_system_projectile_fire() {
        let mut system = CombatSystem::new();
        let id = system.fire_projectile(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), 10.0, 0);
        assert_eq!(id, 0);
        assert_eq!(system.projectile_count(), 1);
        assert!(!system.drain_events().is_empty());
    }

    #[test]
    fn test_combat_system_damage_zone() {
        let mut system = CombatSystem::new();
        let entity = CombatEntity::new(Vec3::ZERO, 1, 100.0);
        system.spawn_entity(entity);
        system.create_damage_zone(DamageZone::new(Vec3::ZERO, 5.0, 50.0, 0, 1.0));
        system.update(0.5);
        let events = system.drain_events();
        assert!(!events.is_empty());
    }
}
