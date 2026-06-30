use glam::Vec3;
use hashbrown::HashMap;
use uuid::Uuid;

use crate::event::{Event, EventData, EventType};

#[derive(Debug, Clone)]
pub struct AggregationConfig {
    pub collision_merge_radius: f32,
    pub damage_merge_window: f32,
    pub force_merge_threshold: f32,
    pub max_aggregated_in_frame: usize,
}

impl Default for AggregationConfig {
    fn default() -> Self {
        Self {
            collision_merge_radius: 0.5,
            damage_merge_window: 0.1,
            force_merge_threshold: 0.01,
            max_aggregated_in_frame: 1000,
        }
    }
}

#[derive(Debug)]
pub struct EventAggregator {
    config: AggregationConfig,
    collision_buffer: Vec<CollisionEntry>,
    damage_buffer: HashMap<(Uuid, Uuid), DamageEntry>,
    force_buffer: HashMap<(Uuid, Uuid), ForceEntry>,
}

#[derive(Debug, Clone)]
struct CollisionEntry {
    source: Uuid,
    target: Uuid,
    contact_normal: Vec3,
    penetration_depth: f32,
    relative_velocity: Vec3,
    count: u32,
}

#[derive(Debug, Clone)]
struct DamageEntry {
    total_damage: f32,
    damage_types: Vec<String>,
    source_position: Vec3,
    count: u32,
}

#[derive(Debug, Clone)]
struct ForceEntry {
    total_force: Vec3,
    total_torque: Vec3,
    application_point: Vec3,
    count: u32,
}

impl EventAggregator {
    pub fn new(config: AggregationConfig) -> Self {
        Self {
            config,
            collision_buffer: Vec::new(),
            damage_buffer: HashMap::new(),
            force_buffer: HashMap::new(),
        }
    }

    pub fn aggregate(&mut self, events: &[Event]) -> Vec<Event> {
        if events.len() > self.config.max_aggregated_in_frame {
            return events.to_vec();
        }

        self.collision_buffer.clear();
        self.damage_buffer.clear();
        self.force_buffer.clear();

        let mut other = Vec::new();

        for event in events {
            match &event.data {
                EventData::Collision { contact_normal, penetration_depth, relative_velocity } => {
                    if let (Some(src), Some(tgt)) = (event.source_entity, event.target_entity) {
                        let normal = Vec3::from_array(*contact_normal);
                        let rel_vel = Vec3::from_array(*relative_velocity);
                        self.merge_collision(
                            src,
                            tgt,
                            normal,
                            *penetration_depth,
                            rel_vel,
                            event.intensity,
                        );
                    } else {
                        other.push(event.clone());
                    }
                },
                EventData::Damage { amount, damage_type, source_position } => {
                    if let (Some(src), Some(tgt)) = (event.source_entity, event.target_entity) {
                        self.merge_damage(
                            src,
                            tgt,
                            *amount,
                            damage_type.clone(),
                            Vec3::from_array(*source_position),
                            event.intensity,
                        );
                    } else {
                        other.push(event.clone());
                    }
                },
                EventData::Force { force_vector, torque, application_point } => {
                    if let (Some(src), Some(tgt)) = (event.source_entity, event.target_entity) {
                        self.merge_force(
                            src,
                            tgt,
                            Vec3::from_array(*force_vector),
                            Vec3::from_array(*torque),
                            Vec3::from_array(*application_point),
                            event.intensity,
                        );
                    } else {
                        other.push(event.clone());
                    }
                },
                _ => other.push(event.clone()),
            }
        }

        let mut result = other;

        for entry in &self.collision_buffer {
            let avg_normal = entry.contact_normal / entry.count as f32;
            let avg_penetration = entry.penetration_depth / entry.count as f32;
            let avg_velocity = entry.relative_velocity / entry.count as f32;
            result.push(Event::new(
                EventType::CollisionDetected,
                Some(entry.source),
                Some(entry.target),
                [0.0, 0.0, 0.0],
                entry.count as f32,
                EventData::Collision {
                    contact_normal: avg_normal.to_array(),
                    penetration_depth: avg_penetration,
                    relative_velocity: avg_velocity.to_array(),
                },
                0,
            ));
        }

        for ((src, tgt), entry) in &self.damage_buffer {
            let avg_pos = entry.source_position / entry.count as f32;
            result.push(Event::new(
                EventType::DamageReceived,
                Some(*src),
                Some(*tgt),
                avg_pos.to_array(),
                entry.total_damage,
                EventData::Damage {
                    amount: entry.total_damage,
                    damage_type: entry.damage_types.join("+"),
                    source_position: avg_pos.to_array(),
                },
                0,
            ));
        }

        for ((src, tgt), entry) in &self.force_buffer {
            let avg_force = entry.total_force / entry.count as f32;
            let avg_torque = entry.total_torque / entry.count as f32;
            let avg_point = entry.application_point / entry.count as f32;
            result.push(Event::new(
                EventType::ForceApplied,
                Some(*src),
                Some(*tgt),
                avg_point.to_array(),
                avg_force.length(),
                EventData::Force {
                    force_vector: avg_force.to_array(),
                    torque: avg_torque.to_array(),
                    application_point: avg_point.to_array(),
                },
                0,
            ));
        }

        result
    }

    fn merge_collision(
        &mut self,
        src: Uuid,
        tgt: Uuid,
        normal: Vec3,
        penetration: f32,
        rel_vel: Vec3,
        _intensity: f32,
    ) {
        for entry in &mut self.collision_buffer {
            if entry.source == src && entry.target == tgt {
                let dist = normal.dot(entry.contact_normal / entry.count as f32);
                if dist > 1.0 - self.config.collision_merge_radius {
                    entry.contact_normal += normal;
                    entry.penetration_depth += penetration;
                    entry.relative_velocity += rel_vel;
                    entry.count += 1;
                    return;
                }
            }
        }
        self.collision_buffer.push(CollisionEntry {
            source: src,
            target: tgt,
            contact_normal: normal,
            penetration_depth: penetration,
            relative_velocity: rel_vel,
            count: 1,
        });
    }

    fn merge_damage(
        &mut self,
        src: Uuid,
        tgt: Uuid,
        amount: f32,
        damage_type: String,
        source_pos: Vec3,
        _intensity: f32,
    ) {
        let key = (src, tgt);
        let entry = self.damage_buffer.entry(key).or_insert(DamageEntry {
            total_damage: 0.0,
            damage_types: Vec::new(),
            source_position: Vec3::ZERO,
            count: 0,
        });
        entry.total_damage += amount;
        if !entry.damage_types.contains(&damage_type) {
            entry.damage_types.push(damage_type);
        }
        entry.source_position += source_pos;
        entry.count += 1;
    }

    fn merge_force(
        &mut self,
        src: Uuid,
        tgt: Uuid,
        force: Vec3,
        torque: Vec3,
        app_point: Vec3,
        _intensity: f32,
    ) {
        let key = (src, tgt);
        let entry = self.force_buffer.entry(key).or_insert(ForceEntry {
            total_force: Vec3::ZERO,
            total_torque: Vec3::ZERO,
            application_point: Vec3::ZERO,
            count: 0,
        });
        entry.total_force += force;
        entry.total_torque += torque;
        entry.application_point += app_point;
        entry.count += 1;
    }

    pub fn clear(&mut self) {
        self.collision_buffer.clear();
        self.damage_buffer.clear();
        self.force_buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_collision_event(src: Uuid, tgt: Uuid, intensity: f32) -> Event {
        Event::new(
            EventType::CollisionDetected,
            Some(src),
            Some(tgt),
            [0.0, 0.0, 0.0],
            intensity,
            EventData::Collision {
                contact_normal: [0.0, 1.0, 0.0],
                penetration_depth: 0.1,
                relative_velocity: [1.0, 0.0, 0.0],
            },
            0,
        )
    }

    fn make_damage_event(src: Uuid, tgt: Uuid, amount: f32) -> Event {
        Event::new(
            EventType::DamageReceived,
            Some(src),
            Some(tgt),
            [0.0, 0.0, 0.0],
            amount,
            EventData::Damage {
                amount,
                damage_type: "slashing".to_string(),
                source_position: [1.0, 0.0, 0.0],
            },
            0,
        )
    }

    #[test]
    fn test_aggregate_collisions() {
        let mut agg = EventAggregator::new(AggregationConfig::default());
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let events = vec![
            make_collision_event(a, b, 1.0),
            make_collision_event(a, b, 2.0),
            make_collision_event(a, b, 0.5),
        ];
        let result = agg.aggregate(&events);
        assert!(result.len() < 3);
    }

    #[test]
    fn test_aggregate_damage() {
        let mut agg = EventAggregator::new(AggregationConfig::default());
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let events = vec![make_damage_event(a, b, 10.0), make_damage_event(a, b, 5.0)];
        let result = agg.aggregate(&events);
        let total_damage: f32 = result
            .iter()
            .filter_map(|e| {
                if let EventData::Damage { amount, .. } = &e.data { Some(*amount) } else { None }
            })
            .sum();
        assert!(total_damage > 0.0);
    }
}
