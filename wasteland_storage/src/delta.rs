use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntityAction {
    Created,
    Removed,
    Modified { component_ids: Vec<u32> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityDiff {
    pub entity_id: u64,
    pub action: EntityAction,
    pub component_data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldDelta {
    pub frame_from: u64,
    pub frame_to: u64,
    pub entities: Vec<EntityDiff>,
    pub global_state: Vec<u8>,
}

impl WorldDelta {
    pub fn new(frame_from: u64, frame_to: u64) -> Self {
        WorldDelta { frame_from, frame_to, entities: Vec::new(), global_state: Vec::new() }
    }

    pub fn add_entity(&mut self, diff: EntityDiff) {
        self.entities.push(diff);
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty() && self.global_state.is_empty()
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
}

pub fn compute_entity_diff(
    before: &HashMap<u64, Vec<u8>>,
    after: &HashMap<u64, Vec<u8>>,
) -> Vec<EntityDiff> {
    let mut diffs = Vec::new();
    for (id, data) in after {
        if !before.contains_key(id) {
            diffs.push(EntityDiff {
                entity_id: *id,
                action: EntityAction::Created,
                component_data: data.clone(),
            });
        }
    }
    for id in before.keys() {
        if !after.contains_key(id) {
            diffs.push(EntityDiff {
                entity_id: *id,
                action: EntityAction::Removed,
                component_data: Vec::new(),
            });
        }
    }
    for (id, data) in after {
        if let Some(before_data) = before.get(id) {
            if before_data != data {
                let mut changed_ids = Vec::new();
                let min_len = data.len().min(before_data.len());
                for i in 0..min_len {
                    if data[i] != before_data[i] {
                        changed_ids.push(i as u32);
                    }
                }
                if data.len() != before_data.len() {
                    changed_ids.push(u32::MAX);
                }
                diffs.push(EntityDiff {
                    entity_id: *id,
                    action: EntityAction::Modified { component_ids: changed_ids },
                    component_data: data.clone(),
                });
            }
        }
    }
    diffs
}

pub fn apply_delta(state: &mut HashMap<u64, Vec<u8>>, delta: &WorldDelta) {
    for diff in &delta.entities {
        match &diff.action {
            EntityAction::Created => {
                state.insert(diff.entity_id, diff.component_data.clone());
            },
            EntityAction::Removed => {
                state.remove(&diff.entity_id);
            },
            EntityAction::Modified { .. } => {
                state.insert(diff.entity_id, diff.component_data.clone());
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(entries: &[(u64, Vec<u8>)]) -> HashMap<u64, Vec<u8>> {
        entries.iter().map(|(k, v)| (*k, v.clone())).collect()
    }

    #[test]
    fn test_entity_created() {
        let before = make_state(&[]);
        let after = make_state(&[(1, vec![10, 20])]);
        let diffs = compute_entity_diff(&before, &after);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].action, EntityAction::Created);
    }

    #[test]
    fn test_entity_removed() {
        let before = make_state(&[(1, vec![10, 20])]);
        let after = make_state(&[]);
        let diffs = compute_entity_diff(&before, &after);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].action, EntityAction::Removed);
    }

    #[test]
    fn test_entity_modified() {
        let before = make_state(&[(1, vec![10, 20])]);
        let after = make_state(&[(1, vec![10, 99])]);
        let diffs = compute_entity_diff(&before, &after);
        assert_eq!(diffs.len(), 1);
        assert!(matches!(diffs[0].action, EntityAction::Modified { .. }));
    }

    #[test]
    fn test_entity_unchanged() {
        let before = make_state(&[(1, vec![10, 20])]);
        let after = make_state(&[(1, vec![10, 20])]);
        let diffs = compute_entity_diff(&before, &after);
        assert_eq!(diffs.len(), 0);
    }

    #[test]
    fn test_apply_delta() {
        let mut state = make_state(&[(1, vec![10, 20])]);
        let mut delta = WorldDelta::new(0, 1);
        delta.add_entity(EntityDiff {
            entity_id: 2,
            action: EntityAction::Created,
            component_data: vec![30, 40],
        });
        apply_delta(&mut state, &delta);
        assert!(state.contains_key(&2));
        assert_eq!(state[&2], vec![30, 40]);
    }

    #[test]
    fn test_apply_delta_remove() {
        let mut state = make_state(&[(1, vec![10, 20])]);
        let mut delta = WorldDelta::new(0, 1);
        delta.add_entity(EntityDiff {
            entity_id: 1,
            action: EntityAction::Removed,
            component_data: vec![],
        });
        apply_delta(&mut state, &delta);
        assert!(!state.contains_key(&1));
    }
}
