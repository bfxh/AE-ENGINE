use glam::Vec3;
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::tier::FrequencyTier;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityGroup {
    pub id: Uuid,
    pub entities: Vec<Uuid>,
    pub tier: FrequencyTier,
    pub is_batch: bool,
    pub batch_size: usize,
    pub position: Vec3,
    pub radius: f32,
    pub dependencies: Vec<Uuid>,
}

impl EntityGroup {
    pub fn new(id: Uuid, tier: FrequencyTier, position: Vec3, radius: f32) -> Self {
        Self {
            id,
            entities: Vec::new(),
            tier,
            is_batch: false,
            batch_size: 16,
            position,
            radius,
            dependencies: Vec::new(),
        }
    }

    pub fn add_entity(&mut self, entity_id: Uuid) {
        if !self.entities.contains(&entity_id) {
            self.entities.push(entity_id);
        }
    }

    pub fn remove_entity(&mut self, entity_id: &Uuid) {
        self.entities.retain(|e| e != entity_id);
    }

    pub fn contains(&self, entity_id: &Uuid) -> bool {
        self.entities.contains(entity_id)
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    edges: HashMap<Uuid, Vec<Uuid>>,
    reverse: HashMap<Uuid, Vec<Uuid>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self { edges: HashMap::new(), reverse: HashMap::new() }
    }

    pub fn add_dependency(&mut self, from: Uuid, to: Uuid) {
        self.edges.entry(from).or_default().push(to);
        self.reverse.entry(to).or_default().push(from);
    }

    pub fn remove_dependency(&mut self, from: &Uuid, to: &Uuid) {
        if let Some(edges) = self.edges.get_mut(from) {
            edges.retain(|t| t != to);
        }
        if let Some(rev) = self.reverse.get_mut(to) {
            rev.retain(|f| f != from);
        }
    }

    pub fn dependents_of(&self, entity: &Uuid) -> Vec<Uuid> {
        self.reverse.get(entity).cloned().unwrap_or_default()
    }

    pub fn dependencies_of(&self, entity: &Uuid) -> Vec<Uuid> {
        self.edges.get(entity).cloned().unwrap_or_default()
    }

    pub fn propagate_tier(
        &self,
        entity: &Uuid,
        tier: FrequencyTier,
        tiers: &HashMap<Uuid, FrequencyTier>,
    ) -> HashMap<Uuid, FrequencyTier> {
        let mut result = HashMap::new();
        let mut visited = HashSet::new();
        let mut stack = vec![(entity, tier)];

        while let Some((current, current_tier)) = stack.pop() {
            if !visited.insert(current) {
                continue;
            }

            if let Some(existing) = tiers.get(current) {
                if *existing <= current_tier {
                    continue;
                }
            }
            result.insert(*current, current_tier);

            if let Some(dependents) = self.reverse.get(current) {
                for dep in dependents {
                    if !visited.contains(dep) {
                        stack.push((dep, current_tier));
                    }
                }
            }
        }

        result
    }

    pub fn topological_order(&self, entities: &[Uuid]) -> Vec<Uuid> {
        let mut in_degree: HashMap<Uuid, usize> = HashMap::new();
        let mut queue: Vec<Uuid> = Vec::new();
        let mut result = Vec::new();

        for &e in entities {
            in_degree.entry(e).or_insert(0);
        }

        for (&from, tos) in &self.edges {
            if entities.contains(&from) {
                for to in tos {
                    if entities.contains(to) {
                        *in_degree.entry(*to).or_insert(0) += 1;
                    }
                }
            }
        }

        for &e in entities {
            if in_degree.get(&e).copied().unwrap_or(0) == 0 {
                queue.push(e);
            }
        }

        while let Some(node) = queue.pop() {
            result.push(node);
            if let Some(deps) = self.edges.get(&node) {
                for dep in deps {
                    if let Some(degree) = in_degree.get_mut(dep) {
                        *degree = degree.saturating_sub(1);
                        if *degree == 0 {
                            queue.push(*dep);
                        }
                    }
                }
            }
        }

        result
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct GroupScheduler {
    groups: HashMap<Uuid, EntityGroup>,
    entity_to_group: HashMap<Uuid, Uuid>,
    dependencies: DependencyGraph,
    spatial_grid: HashMap<(i32, i32, i32), Vec<Uuid>>,
    grid_cell_size: f32,
}

impl GroupScheduler {
    pub fn new(grid_cell_size: f32) -> Self {
        Self {
            groups: HashMap::new(),
            entity_to_group: HashMap::new(),
            dependencies: DependencyGraph::new(),
            spatial_grid: HashMap::new(),
            grid_cell_size,
        }
    }

    pub fn create_group(&mut self, id: Uuid, tier: FrequencyTier, position: Vec3, radius: f32) {
        let group = EntityGroup::new(id, tier, position, radius);
        self.groups.insert(id, group);
        self.update_spatial(id);
    }

    pub fn add_to_group(&mut self, entity_id: Uuid, group_id: Uuid) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.add_entity(entity_id);
            self.entity_to_group.insert(entity_id, group_id);
        }
    }

    pub fn remove_entity(&mut self, entity_id: &Uuid) {
        if let Some(group_id) = self.entity_to_group.remove(entity_id) {
            if let Some(group) = self.groups.get_mut(&group_id) {
                group.remove_entity(entity_id);
                if group.is_empty() {
                    self.groups.remove(&group_id);
                }
            }
        }
    }

    pub fn get_group(&self, entity_id: &Uuid) -> Option<&EntityGroup> {
        self.entity_to_group.get(entity_id).and_then(|gid| self.groups.get(gid))
    }

    pub fn get_group_tier(&self, entity_id: &Uuid) -> Option<FrequencyTier> {
        self.get_group(entity_id).map(|g| g.tier)
    }

    pub fn set_group_tier(&mut self, group_id: &Uuid, tier: FrequencyTier) {
        if let Some(group) = self.groups.get_mut(group_id) {
            group.tier = tier;
        }
    }

    pub fn add_dependency(&mut self, from: Uuid, to: Uuid) {
        self.dependencies.add_dependency(from, to);
    }

    pub fn get_dependents(&self, entity: &Uuid) -> Vec<Uuid> {
        self.dependencies.dependents_of(entity)
    }

    pub fn propagate_tier_upgrade(&mut self, entity: &Uuid, new_tier: FrequencyTier) {
        let mut entity_tiers: HashMap<Uuid, FrequencyTier> = HashMap::new();
        for (eid, gid) in &self.entity_to_group {
            if let Some(g) = self.groups.get(gid) {
                entity_tiers.insert(*eid, g.tier);
            }
        }
        let upgrades = self.dependencies.propagate_tier(entity, new_tier, &entity_tiers);
        for (eid, tier) in upgrades {
            if let Some(gid) = self.entity_to_group.get(&eid) {
                if let Some(group) = self.groups.get_mut(gid) {
                    if group.tier > tier {
                        group.tier = tier;
                    }
                }
            }
        }
    }

    pub fn query_spatial(&self, position: Vec3, radius: f32) -> Vec<Uuid> {
        let cell_size = self.grid_cell_size;
        let min_cell = (
            ((position.x - radius) / cell_size).floor() as i32,
            ((position.y - radius) / cell_size).floor() as i32,
            ((position.z - radius) / cell_size).floor() as i32,
        );
        let max_cell = (
            ((position.x + radius) / cell_size).ceil() as i32,
            ((position.y + radius) / cell_size).ceil() as i32,
            ((position.z + radius) / cell_size).ceil() as i32,
        );

        let mut result = Vec::new();
        for cx in min_cell.0..=max_cell.0 {
            for cy in min_cell.1..=max_cell.1 {
                for cz in min_cell.2..=max_cell.2 {
                    if let Some(groups) = self.spatial_grid.get(&(cx, cy, cz)) {
                        for gid in groups {
                            if !result.contains(gid) {
                                result.push(*gid);
                            }
                        }
                    }
                }
            }
        }
        result
    }

    fn update_spatial(&mut self, _group_id: Uuid) {
        self.spatial_grid.clear();
        for (gid, group) in &self.groups {
            let cx = (group.position.x / self.grid_cell_size).floor() as i32;
            let cy = (group.position.y / self.grid_cell_size).floor() as i32;
            let cz = (group.position.z / self.grid_cell_size).floor() as i32;
            self.spatial_grid.entry((cx, cy, cz)).or_default().push(*gid);
        }
    }

    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    pub fn entity_count(&self) -> usize {
        self.entity_to_group.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_add_remove() {
        let mut g = EntityGroup::new(Uuid::new_v4(), FrequencyTier::Medium, Vec3::ZERO, 10.0);
        let eid = Uuid::new_v4();
        g.add_entity(eid);
        assert!(g.contains(&eid));
        g.remove_entity(&eid);
        assert!(!g.contains(&eid));
    }

    #[test]
    fn test_dependency_propagation() {
        let mut graph = DependencyGraph::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        graph.add_dependency(a, b);
        graph.add_dependency(b, c);

        let mut tiers = HashMap::new();
        tiers.insert(a, FrequencyTier::Low);
        tiers.insert(b, FrequencyTier::Low);
        tiers.insert(c, FrequencyTier::Low);

        let upgrades = graph.propagate_tier(&a, FrequencyTier::Critical, &tiers);
        assert!(upgrades.contains_key(&a));
        assert_eq!(upgrades.get(&a), Some(&FrequencyTier::Critical));
    }

    #[test]
    fn test_topological_order() {
        let mut graph = DependencyGraph::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        graph.add_dependency(a, b);
        graph.add_dependency(b, c);

        let order = graph.topological_order(&[a, b, c]);
        assert_eq!(order.len(), 3);
        assert_eq!(order[0], a);
        assert_eq!(order[1], b);
        assert_eq!(order[2], c);
    }

    #[test]
    fn test_group_scheduler_spatial() {
        let mut s = GroupScheduler::new(50.0);
        let gid = Uuid::new_v4();
        s.create_group(gid, FrequencyTier::Medium, Vec3::new(10.0, 0.0, 0.0), 5.0);
        let nearby = s.query_spatial(Vec3::ZERO, 20.0);
        assert!(nearby.contains(&gid));
    }
}
