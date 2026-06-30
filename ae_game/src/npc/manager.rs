use super::ai_optimizer::{NpcAiConfig, NpcAiOptimizer};
use super::NpcId;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use slotmap::SlotMap;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NpcRole {
    Combatant,
    Civilian,
    Wildlife,
    Vendor,
    Guard,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NpcBehaviorState {
    Idle,
    Patrol,
    Investigate,
    Combat,
    Flee,
    Dead,
    Wounded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcEntity {
    pub id: NpcId,
    pub role: NpcRole,
    pub state: NpcBehaviorState,
    pub position: Vec3,
    pub velocity: Vec3,
    pub target_position: Option<Vec3>,
    pub patrol_points: Vec<Vec3>,
    pub patrol_index: usize,
    pub health: f32,
    pub max_health: f32,
    pub faction: u8,
    pub perception_range: f32,
    pub detection_level: f32,
    pub last_known_enemy_pos: Option<Vec3>,
    pub alert_timer: f32,
    pub attack_cooldown: f32,
    pub move_speed: f32,
    pub run_speed: f32,
    pub name: String,
    pub dialogue_lines: Vec<String>,
    pub current_dialogue: Option<usize>,
    pub animation_state: u8,
    pub animation_time: f32,
}

impl NpcEntity {
    pub fn new_combatant(pos: Vec3, faction: u8) -> Self {
        NpcEntity {
            id: NpcId::default(),
            role: NpcRole::Combatant,
            state: NpcBehaviorState::Patrol,
            position: pos,
            velocity: Vec3::ZERO,
            target_position: None,
            patrol_points: Vec::new(),
            patrol_index: 0,
            health: 100.0,
            max_health: 100.0,
            faction,
            perception_range: 30.0,
            detection_level: 0.0,
            last_known_enemy_pos: None,
            alert_timer: 0.0,
            attack_cooldown: 0.0,
            move_speed: 3.0,
            run_speed: 6.0,
            name: String::new(),
            dialogue_lines: Vec::new(),
            current_dialogue: None,
            animation_state: 0,
            animation_time: 0.0,
        }
    }

    pub fn new_civilian(pos: Vec3) -> Self {
        NpcEntity {
            id: NpcId::default(),
            role: NpcRole::Civilian,
            state: NpcBehaviorState::Idle,
            position: pos,
            velocity: Vec3::ZERO,
            target_position: None,
            patrol_points: Vec::new(),
            patrol_index: 0,
            health: 50.0,
            max_health: 50.0,
            faction: 0,
            perception_range: 15.0,
            detection_level: 0.0,
            last_known_enemy_pos: None,
            alert_timer: 0.0,
            attack_cooldown: 0.0,
            move_speed: 2.0,
            run_speed: 4.0,
            name: String::new(),
            dialogue_lines: Vec::new(),
            current_dialogue: None,
            animation_state: 0,
            animation_time: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcPerception {
    pub npc_id: NpcId,
    pub visible_enemies: Vec<NpcId>,
    pub visible_allies: Vec<NpcId>,
    pub heard_events: Vec<PerceivedEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceivedEvent {
    pub event_type: PerceptionEventType,
    pub position: Vec3,
    pub intensity: f32,
    pub timestamp: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PerceptionEventType {
    Footstep,
    Gunshot,
    Explosion,
    Scream,
    Combat,
}

/// 轻量级 3D 空间哈希（内部使用，避免循环依赖 ae_engine）。
/// cell_size 必须 >= 最大 perception_range，这样 27 邻域查询覆盖感知范围。
struct SpatialHash {
    cell_size: f32,
    cells: HashMap<(i32, i32, i32), Vec<usize>>,
}

impl SpatialHash {
    fn new(cell_size: f32) -> Self {
        Self { cell_size, cells: HashMap::new() }
    }

    fn clear(&mut self) {
        self.cells.clear();
    }

    fn build(&mut self, positions: &[[f32; 3]]) {
        self.clear();
        for (i, pos) in positions.iter().enumerate() {
            let key = self.cell_key(*pos);
            self.cells.entry(key).or_default().push(i);
        }
    }

    #[inline]
    fn cell_key(&self, pos: [f32; 3]) -> (i32, i32, i32) {
        (
            (pos[0] / self.cell_size).floor() as i32,
            (pos[1] / self.cell_size).floor() as i32,
            (pos[2] / self.cell_size).floor() as i32,
        )
    }

    /// 查询 27 邻域内所有候选索引（调用者需再做精确距离过滤）。
    fn query_neighbors(&self, pos: [f32; 3]) -> SmallVecIndices {
        let (cx, cy, cz) = self.cell_key(pos);
        let mut result = SmallVecIndices::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if let Some(cell) = self.cells.get(&(cx + dx, cy + dy, cz + dz)) {
                        result.extend_from_slice(cell);
                    }
                }
            }
        }
        result
    }
}

/// 避免每次查询都堆分配的小型索引集合。
/// 大多数情况下 27 邻域的候选数 < 64，用栈上缓冲。
type SmallVecIndices = smallvec::SmallVec<[usize; 64]>;

pub struct NpcManager {
    pub npcs: SlotMap<NpcId, NpcEntity>,
    pub perceptions: Vec<NpcPerception>,
    pub max_npcs: usize,
    pub global_alert_level: f32,
    spatial_hash: SpatialHash,
    /// AI 节流器：基于到玩家距离的 LOD 思考/感知频率控制。
    pub ai_optimizer: NpcAiOptimizer,
}

impl NpcManager {
    pub fn new(max_npcs: usize) -> Self {
        NpcManager {
            npcs: SlotMap::with_key(),
            perceptions: Vec::with_capacity(max_npcs),
            max_npcs,
            global_alert_level: 0.0,
            // cell_size = 32 覆盖最大 perception_range = 30
            spatial_hash: SpatialHash::new(32.0),
            ai_optimizer: NpcAiOptimizer::new(NpcAiConfig::default()),
        }
    }

    pub fn spawn_npc(&mut self, npc: NpcEntity) -> NpcId {
        let id = self.npcs.insert(npc);
        if let Some(n) = self.npcs.get_mut(id) {
            n.id = id;
        }
        id
    }

    pub fn spawn_combatant(&mut self, pos: Vec3, faction: u8) -> NpcId {
        self.spawn_npc(NpcEntity::new_combatant(pos, faction))
    }

    pub fn spawn_civilian(&mut self, pos: Vec3) -> NpcId {
        self.spawn_npc(NpcEntity::new_civilian(pos))
    }

    pub fn remove_npc(&mut self, id: NpcId) {
        self.npcs.remove(id);
    }

    /// NPC 行为更新。
    ///
    /// 优化点：
    /// 1. 用 spatial hash 把感知 O(n²) 降为 O(n·k)，k=邻域候选数
    /// 2. 第一阶段只收集 (NpcId, Option<nearest_enemy_pos>)，避免 perceptions 整体深拷贝
    /// 3. 第二阶段用紧凑数据驱动 NPC 状态机
    pub fn step(&mut self, dt: f32, player_position: Vec3, _enemy_positions: &[(NpcId, Vec3, u8)]) {
        self.perceptions.clear();
        self.global_alert_level = (self.global_alert_level - dt * 0.1).max(0.0);

        // 构建活跃 NPC 索引表 + 位置数组
        let npc_table: Vec<(NpcId, Vec3, u8, f32)> = self
            .npcs
            .iter()
            .filter(|(_, n)| n.state != NpcBehaviorState::Dead)
            .map(|(id, n)| (id, n.position, n.faction, n.perception_range))
            .collect();

        let positions: Vec<[f32; 3]> = npc_table.iter().map(|(_, p, _, _)| (*p).into()).collect();
        self.spatial_hash.build(&positions);

        // 感知 + 最近敌人位置（单遍，避免后续 clone）
        let mut nearest_enemy: Vec<(NpcId, Option<Vec3>)> = Vec::with_capacity(npc_table.len());

        for (id, pos, faction, perception_range) in &npc_table {
            let dist_to_player = pos.distance(player_position);

            // AI 节流：远距离 NPC 降低感知频率，跳过扫描但仍保留 last_known_enemy_pos
            if !self.ai_optimizer.should_perceive(*id, dt, dist_to_player) {
                let enemy_pos = self
                    .npcs
                    .get(*id)
                    .and_then(|n| n.last_known_enemy_pos);
                nearest_enemy.push((*id, enemy_pos));
                self.perceptions.push(NpcPerception {
                    npc_id: *id,
                    visible_enemies: Vec::new(),
                    visible_allies: Vec::new(),
                    heard_events: Vec::new(),
                });
                continue;
            }

            let mut perception = NpcPerception {
                npc_id: *id,
                visible_enemies: Vec::new(),
                visible_allies: Vec::new(),
                heard_events: Vec::new(),
            };

            let candidates = self.spatial_hash.query_neighbors((*pos).into());
            let mut best_enemy: Option<(Vec3, f32)> = None; // (pos, dist_sq)

            for idx in candidates.iter() {
                let (other_id, other_pos, other_faction, _) = &npc_table[*idx];
                if *other_id == *id {
                    continue;
                }
                let diff = *pos - *other_pos;
                let dist_sq = diff.length_squared();
                if dist_sq < perception_range * perception_range {
                    if *other_faction != *faction {
                        perception.visible_enemies.push(*other_id);
                        if best_enemy.map_or(true, |(_, d)| dist_sq < d) {
                            best_enemy = Some((*other_pos, dist_sq));
                        }
                    } else {
                        perception.visible_allies.push(*other_id);
                    }
                }
            }

            if !perception.visible_enemies.is_empty() {
                self.global_alert_level = (self.global_alert_level + dt * 0.5).min(1.0);
            }
            nearest_enemy.push((*id, best_enemy.map(|(p, _)| p)));
            self.perceptions.push(perception);
        }

        // 第二阶段：用紧凑的 nearest_enemy 数据驱动状态机，无需 clone perceptions
        for (npc_id, enemy_pos) in &nearest_enemy {
            let npc_pos = match self.npcs.get(*npc_id) {
                Some(n) if n.state != NpcBehaviorState::Dead => n.position,
                _ => continue,
            };

            // AI 节流：远距离 NPC 降低思考频率
            let in_combat = self
                .npcs
                .get(*npc_id)
                .map_or(false, |n| n.state == NpcBehaviorState::Combat);
            let dist_to_player = npc_pos.distance(player_position);
            let should_think = self
                .ai_optimizer
                .should_think(*npc_id, dt, in_combat, dist_to_player);

            let npc = match self.npcs.get_mut(*npc_id) {
                Some(n) => n,
                None => continue,
            };

            if npc.state == NpcBehaviorState::Dead {
                continue;
            }

            npc.alert_timer = (npc.alert_timer - dt).max(0.0);
            npc.attack_cooldown = (npc.attack_cooldown - dt).max(0.0);
            npc.animation_time += dt;

            if should_think {
                if let Some(ep) = enemy_pos {
                    npc.last_known_enemy_pos = Some(*ep);
                    npc.detection_level = (npc.detection_level + dt * 2.0).min(1.0);

                    if npc.detection_level > 0.5 {
                        let dist = (*ep - npc_pos).length();
                        if dist < 5.0 {
                            npc.state = NpcBehaviorState::Combat;
                            npc.velocity = Vec3::ZERO;
                            npc.animation_state = 2;
                        } else if npc.health < npc.max_health * 0.2 {
                            npc.state = NpcBehaviorState::Flee;
                            let dir = (npc_pos - *ep).normalize_or_zero();
                            npc.velocity = dir * npc.run_speed;
                            npc.animation_state = 3;
                        } else {
                            npc.state = NpcBehaviorState::Combat;
                            let dir = (*ep - npc_pos).normalize_or_zero();
                            npc.velocity = dir * npc.run_speed;
                            npc.animation_state = 3;
                        }
                    }
                } else {
                    npc.detection_level = (npc.detection_level - dt * 0.5).max(0.0);

                    if npc.detection_level < 0.1 && npc.state == NpcBehaviorState::Combat {
                        if !npc.patrol_points.is_empty() {
                            npc.state = NpcBehaviorState::Patrol;
                            npc.animation_state = 1;
                        } else {
                            npc.state = NpcBehaviorState::Idle;
                            npc.velocity = Vec3::ZERO;
                            npc.animation_state = 0;
                        }
                    }

                    if npc.state == NpcBehaviorState::Patrol && !npc.patrol_points.is_empty() {
                        let target = npc.patrol_points[npc.patrol_index];
                        let dir = target - npc_pos;
                        let dist = dir.length();
                        if dist < 1.0 {
                            npc.patrol_index = (npc.patrol_index + 1) % npc.patrol_points.len();
                        } else {
                            npc.velocity = dir.normalize_or_zero() * npc.move_speed;
                            npc.animation_state = 1;
                        }
                    }
                }
            }

            npc.position += npc.velocity * dt;

            if npc.health <= 0.0 {
                npc.state = NpcBehaviorState::Dead;
                npc.velocity = Vec3::ZERO;
                npc.animation_state = 4;
            }
        }
    }

    pub fn get_dialogue(&self, npc_id: NpcId) -> Option<&str> {
        self.npcs.get(npc_id).and_then(|n| {
            n.current_dialogue
                .and_then(|i| n.dialogue_lines.get(i).map(|s| s.as_str()))
        })
    }

    pub fn trigger_dialogue(&mut self, npc_id: NpcId) {
        if let Some(npc) = self.npcs.get_mut(npc_id) {
            if npc.dialogue_lines.is_empty() {
                return;
            }
            npc.current_dialogue = Some(match npc.current_dialogue {
                Some(i) => (i + 1) % npc.dialogue_lines.len(),
                None => 0,
            });
        }
    }

    pub fn active_count(&self) -> usize {
        self.npcs
            .values()
            .filter(|n| n.state != NpcBehaviorState::Dead)
            .count()
    }

    pub fn combatant_count(&self) -> usize {
        self.npcs
            .values()
            .filter(|n| n.role == NpcRole::Combatant && n.state != NpcBehaviorState::Dead)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npc_manager_spawn_and_remove() {
        let mut mgr = NpcManager::new(100);
        let id = mgr.spawn_combatant(Vec3::new(0.0, 0.0, 0.0), 1);
        assert_eq!(mgr.active_count(), 1);
        mgr.remove_npc(id);
        assert_eq!(mgr.active_count(), 0);
    }

    #[test]
    fn spatial_hash_reduces_comparisons() {
        // 300 NPC 均匀分布，spatial hash 应大幅减少候选数
        let mut mgr = NpcManager::new(300);
        for i in 0..300u32 {
            let x = (i % 30) as f32 * 10.0;
            let z = (i / 30) as f32 * 10.0;
            mgr.spawn_combatant(Vec3::new(x, 0.0, z), (i % 2) as u8);
        }
        mgr.step(0.016, Vec3::ZERO, &[]);
        // 感知应非空（有敌人）
        assert!(mgr.perceptions.iter().any(|p| !p.visible_enemies.is_empty()));
    }

    #[test]
    fn dead_npcs_excluded() {
        let mut mgr = NpcManager::new(10);
        let id1 = mgr.spawn_combatant(Vec3::new(0.0, 0.0, 0.0), 1);
        let _id2 = mgr.spawn_combatant(Vec3::new(1.0, 0.0, 0.0), 2);
        if let Some(n) = mgr.npcs.get_mut(id1) {
            n.state = NpcBehaviorState::Dead;
        }
        mgr.step(0.016, Vec3::ZERO, &[]);
        // Dead NPC 不应产生感知
        assert!(mgr.perceptions.iter().all(|p| p.npc_id != id1));
    }

    #[test]
    fn global_alert_rises_with_enemies() {
        let mut mgr = NpcManager::new(10);
        let _ = mgr.spawn_combatant(Vec3::new(0.0, 0.0, 0.0), 1);
        let _ = mgr.spawn_combatant(Vec3::new(1.0, 0.0, 0.0), 2);
        let before = mgr.global_alert_level;
        mgr.step(0.016, Vec3::ZERO, &[]);
        assert!(mgr.global_alert_level > before);
    }

    #[test]
    fn no_friendly_fire_in_perception() {
        let mut mgr = NpcManager::new(10);
        let _ = mgr.spawn_combatant(Vec3::new(0.0, 0.0, 0.0), 1);
        let _ = mgr.spawn_combatant(Vec3::new(1.0, 0.0, 0.0), 1); // 同阵营
        mgr.step(0.016, Vec3::ZERO, &[]);
        // 同阵营应归入 visible_allies 而非 visible_enemies
        for p in &mgr.perceptions {
            assert!(p.visible_enemies.is_empty());
        }
    }
}
