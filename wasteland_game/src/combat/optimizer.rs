use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialHashGrid {
    cell_size: f32,
    cells: HashMap<(i32, i32, i32), Vec<u64>>,
}

impl SpatialHashGrid {
    pub fn new(cell_size: f32) -> Self {
        SpatialHashGrid {
            cell_size,
            cells: HashMap::new(),
        }
    }

    pub fn insert(&mut self, entity_id: u64, position: Vec3) {
        let cell = self.cell_coord(position);
        self.cells.entry(cell).or_default().push(entity_id);
    }

    pub fn remove(&mut self, entity_id: u64, position: Vec3) {
        let cell = self.cell_coord(position);
        if let Some(entities) = self.cells.get_mut(&cell) {
            entities.retain(|&e| e != entity_id);
            if entities.is_empty() {
                self.cells.remove(&cell);
            }
        }
    }

    pub fn update(&mut self, entity_id: u64, old_pos: Vec3, new_pos: Vec3) {
        let old_cell = self.cell_coord(old_pos);
        let new_cell = self.cell_coord(new_pos);
        if old_cell != new_cell {
            self.remove(entity_id, old_pos);
            self.insert(entity_id, new_pos);
        }
    }

    pub fn query_radius(&self, center: Vec3, radius: f32) -> Vec<u64> {
        let mut result = Vec::new();
        let min_cell = self.cell_coord(center - Vec3::splat(radius));
        let max_cell = self.cell_coord(center + Vec3::splat(radius));

        for x in min_cell.0..=max_cell.0 {
            for y in min_cell.1..=max_cell.1 {
                for z in min_cell.2..=max_cell.2 {
                    if let Some(entities) = self.cells.get(&(x, y, z)) {
                        result.extend(entities.iter().copied());
                    }
                }
            }
        }
        result
    }

    fn cell_coord(&self, pos: Vec3) -> (i32, i32, i32) {
        (
            (pos.x / self.cell_size).floor() as i32,
            (pos.y / self.cell_size).floor() as i32,
            (pos.z / self.cell_size).floor() as i32,
        )
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageZoneOptimizer {
    pub grid: SpatialHashGrid,
    pub active_zones: Vec<OptimizedDamageZone>,
    pub max_zones: usize,
    pub update_interval: f32,
    pub timer: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedDamageZone {
    pub id: u64,
    pub center: Vec3,
    pub radius: f32,
    pub damage_per_second: f32,
    pub remaining: f32,
    pub faction: u8,
    pub affected_entities: Vec<u64>,
    pub last_update: f32,
}

impl DamageZoneOptimizer {
    pub fn new(cell_size: f32, max_zones: usize) -> Self {
        DamageZoneOptimizer {
            grid: SpatialHashGrid::new(cell_size),
            active_zones: Vec::new(),
            max_zones,
            update_interval: 0.1,
            timer: 0.0,
        }
    }

    pub fn add_zone(&mut self, center: Vec3, radius: f32, dps: f32, duration: f32, faction: u8) -> u64 {
        if self.active_zones.len() >= self.max_zones {
            self.active_zones.remove(0);
        }
        let id = self.active_zones.len() as u64 + 1;
        self.active_zones.push(OptimizedDamageZone {
            id,
            center,
            radius,
            damage_per_second: dps,
            remaining: duration,
            faction,
            affected_entities: Vec::new(),
            last_update: 0.0,
        });
        id
    }

    pub fn step(&mut self, dt: f32, entity_positions: &[(u64, Vec3, u8)]) -> Vec<(u64, f32)> {
        self.timer += dt;
        let should_update = self.timer >= self.update_interval;
        if should_update {
            self.timer = 0.0;
        }

        let mut damage_events = Vec::new();

        self.grid.clear();
        for &(id, pos, _faction) in entity_positions {
            self.grid.insert(id, pos);
        }

        for zone in &mut self.active_zones {
            zone.remaining -= dt;
            if zone.remaining <= 0.0 {
                continue;
            }

            if should_update {
                zone.affected_entities.clear();
                let nearby = self.grid.query_radius(zone.center, zone.radius);
                for &(id, pos, faction) in entity_positions {
                    if !nearby.contains(&id) {
                        continue;
                    }
                    if faction == zone.faction {
                        continue;
                    }
                    let dist = (pos - zone.center).length();
                    if dist < zone.radius {
                        zone.affected_entities.push(id);
                    }
                }
                zone.last_update = 0.0;
            } else {
                zone.last_update += dt;
            }

            let effective_dt = if should_update { self.update_interval } else { dt };
            for &entity_id in &zone.affected_entities {
                if let Some(&(_, _, _)) = entity_positions.iter().find(|(id, _, _)| *id == entity_id) {
                    let falloff = 1.0 - (zone.radius - 0.0).max(0.1).recip() * 0.0;
                    let damage = zone.damage_per_second * effective_dt * falloff;
                    damage_events.push((entity_id, damage));
                }
            }
        }

        self.active_zones.retain(|z| z.remaining > 0.0);
        damage_events
    }

    pub fn active_zone_count(&self) -> usize {
        self.active_zones.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReinforcementSystem {
    pub pending_calls: Vec<ReinforcementCall>,
    pub arrived_reinforcements: Vec<ArrivedReinforcement>,
    pub max_concurrent_calls: usize,
    pub spawn_cooldown: f32,
    pub timer: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReinforcementCall {
    pub caller_id: u64,
    pub caller_position: Vec3,
    pub faction: u8,
    pub requested_count: u8,
    pub arrival_time: f32,
    pub remaining_time: f32,
    pub spawn_positions: Vec<Vec3>,
    pub urgency: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrivedReinforcement {
    pub caller_id: u64,
    pub npc_ids: Vec<u64>,
    pub faction: u8,
}

impl ReinforcementSystem {
    pub fn new(max_concurrent_calls: usize) -> Self {
        ReinforcementSystem {
            pending_calls: Vec::new(),
            arrived_reinforcements: Vec::new(),
            max_concurrent_calls,
            spawn_cooldown: 2.0,
            timer: 0.0,
        }
    }

    pub fn request_reinforcements(
        &mut self,
        caller_id: u64,
        caller_position: Vec3,
        faction: u8,
        count: u8,
        urgency: f32,
    ) -> bool {
        if self.pending_calls.len() >= self.max_concurrent_calls {
            return false;
        }

        let arrival_time = if urgency > 0.8 {
            3.0
        } else if urgency > 0.5 {
            8.0
        } else {
            15.0
        };

        let mut spawn_positions = Vec::new();
        for i in 0..count {
            let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
            let offset = Vec3::new(angle.cos(), 0.0, angle.sin()) * 20.0;
            spawn_positions.push(caller_position + offset);
        }

        self.pending_calls.push(ReinforcementCall {
            caller_id,
            caller_position,
            faction,
            requested_count: count,
            arrival_time,
            remaining_time: arrival_time,
            spawn_positions,
            urgency,
        });
        true
    }

    pub fn step(&mut self, dt: f32) -> Vec<ReinforcementCall> {
        self.timer += dt;
        let mut arrived = Vec::new();

        for call in &mut self.pending_calls {
            call.remaining_time -= dt;
            if call.remaining_time <= 0.0 {
                arrived.push(call.clone());
            }
        }

        self.pending_calls.retain(|c| c.remaining_time > 0.0);
        arrived
    }

    pub fn pending_count(&self) -> usize {
        self.pending_calls.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatPerformanceMonitor {
    pub frame_combat_time: f32,
    pub frame_damage_events: u32,
    pub frame_projectile_checks: u32,
    pub frame_zone_checks: u32,
    pub avg_combat_time: f32,
    pub max_combat_time: f32,
    pub sample_count: u32,
}

impl CombatPerformanceMonitor {
    pub fn new() -> Self {
        CombatPerformanceMonitor {
            frame_combat_time: 0.0,
            frame_damage_events: 0,
            frame_projectile_checks: 0,
            frame_zone_checks: 0,
            avg_combat_time: 0.0,
            max_combat_time: 0.0,
            sample_count: 0,
        }
    }

    pub fn begin_frame(&mut self) {
        self.frame_combat_time = 0.0;
        self.frame_damage_events = 0;
        self.frame_projectile_checks = 0;
        self.frame_zone_checks = 0;
    }

    pub fn end_frame(&mut self, frame_time: f32) {
        self.frame_combat_time = frame_time;
        self.sample_count += 1;
        let alpha = 0.95;
        self.avg_combat_time = self.avg_combat_time * alpha + frame_time * (1.0 - alpha);
        if frame_time > self.max_combat_time {
            self.max_combat_time = frame_time;
        }
    }

    pub fn should_optimize(&self) -> bool {
        self.avg_combat_time > 2.0
    }

    pub fn report(&self) -> String {
        format!(
            "Combat: {:.2}ms avg, {:.2}ms max, {} dmg events, {} proj checks, {} zone checks",
            self.avg_combat_time,
            self.max_combat_time,
            self.frame_damage_events,
            self.frame_projectile_checks,
            self.frame_zone_checks
        )
    }
}

impl Default for CombatPerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}
