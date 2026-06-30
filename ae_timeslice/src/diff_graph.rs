use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(u64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffNode {
    pub id: NodeId,
    pub name: String,
    pub dependencies: Vec<NodeId>,
    pub dependents: Vec<NodeId>,
    pub dirty: bool,
    pub enabled: bool,
    pub update_count: u64,
}

type UpdateFn = Box<dyn FnMut(f32) + Send>;

pub struct NodeEntry {
    pub node: DiffNode,
    pub callback: UpdateFn,
}

impl std::fmt::Debug for NodeEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeEntry").field("node", &self.node).finish()
    }
}

pub struct DiffUpdateGraph {
    nodes: HashMap<NodeId, NodeEntry>,
    next_id: u64,
    pub propagation_enabled: bool,
}

impl std::fmt::Debug for DiffUpdateGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiffUpdateGraph")
            .field("node_count", &self.nodes.len())
            .field("propagation_enabled", &self.propagation_enabled)
            .finish()
    }
}

impl DiffUpdateGraph {
    pub fn new() -> Self {
        Self { nodes: HashMap::new(), next_id: 0, propagation_enabled: true }
    }

    pub fn register<F>(&mut self, name: &str, dependencies: Vec<NodeId>, callback: F) -> NodeId
    where
        F: FnMut(f32) + Send + 'static,
    {
        let id = NodeId(self.next_id);
        self.next_id += 1;

        let node = DiffNode {
            id,
            name: name.to_string(),
            dependencies: dependencies.clone(),
            dependents: Vec::new(),
            dirty: true,
            enabled: true,
            update_count: 0,
        };

        for dep_id in &dependencies {
            if let Some(entry) = self.nodes.get_mut(dep_id) {
                entry.node.dependents.push(id);
            }
        }

        self.nodes.insert(id, NodeEntry { node, callback: Box::new(callback) });

        id
    }

    pub fn mark_dirty(&mut self, node_id: NodeId) {
        if let Some(entry) = self.nodes.get_mut(&node_id) {
            entry.node.dirty = true;
            if self.propagation_enabled {
                self.propagate_dirty(node_id);
            }
        }
    }

    pub fn mark_dirty_by_name(&mut self, name: &str) {
        let ids: Vec<NodeId> =
            self.nodes.iter().filter(|(_, e)| e.node.name == name).map(|(id, _)| *id).collect();
        for id in ids {
            self.mark_dirty(id);
        }
    }

    fn propagate_dirty(&mut self, node_id: NodeId) {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(node_id);

        while let Some(current_id) = queue.pop_front() {
            if !visited.insert(current_id) {
                continue;
            }

            let dependents: Vec<NodeId> =
                self.nodes.get(&current_id).map(|e| e.node.dependents.clone()).unwrap_or_default();

            for dep_id in dependents {
                if let Some(entry) = self.nodes.get_mut(&dep_id) {
                    entry.node.dirty = true;
                    queue.push_back(dep_id);
                }
            }
        }
    }

    pub fn topological_order(&self) -> Vec<NodeId> {
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
        let mut graph: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

        for (id, entry) in &self.nodes {
            in_degree.entry(*id).or_insert(0);
            for dep_id in &entry.node.dependencies {
                graph.entry(*dep_id).or_default().push(*id);
                *in_degree.entry(*id).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<NodeId> =
            in_degree.iter().filter(|(_, &deg)| deg == 0).map(|(&id, _)| id).collect();

        let mut order = Vec::new();

        while let Some(id) = queue.pop_front() {
            order.push(id);
            if let Some(dependents) = graph.get(&id) {
                for &dep_id in dependents {
                    if let Some(deg) = in_degree.get_mut(&dep_id) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(dep_id);
                        }
                    }
                }
            }
        }

        order
    }

    pub fn update(&mut self, dt: f32) {
        let order = self.topological_order();

        for node_id in order {
            let should_update =
                self.nodes.get(&node_id).map(|e| e.node.dirty && e.node.enabled).unwrap_or(false);

            if should_update {
                if let Some(entry) = self.nodes.get_mut(&node_id) {
                    (entry.callback)(dt);
                    entry.node.dirty = false;
                    entry.node.update_count += 1;
                }
            }
        }
    }

    pub fn dirty_nodes(&self) -> Vec<&DiffNode> {
        self.nodes.values().filter(|e| e.node.dirty).map(|e| &e.node).collect()
    }

    pub fn stats(&self) -> DiffGraphStats {
        let total = self.nodes.len();
        let dirty = self.nodes.values().filter(|e| e.node.dirty).count();
        let total_updates: u64 = self.nodes.values().map(|e| e.node.update_count).sum();

        DiffGraphStats {
            total_nodes: total,
            dirty_nodes: dirty,
            clean_nodes: total - dirty,
            total_updates,
            update_ratio: if total > 0 { dirty as f64 / total as f64 } else { 0.0 },
        }
    }

    pub fn node(&self, id: NodeId) -> Option<&DiffNode> {
        self.nodes.get(&id).map(|e| &e.node)
    }

    pub fn node_by_name(&self, name: &str) -> Option<&DiffNode> {
        self.nodes.values().find(|e| e.node.name == name).map(|e| &e.node)
    }
}

impl Default for DiffUpdateGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffGraphStats {
    pub total_nodes: usize,
    pub dirty_nodes: usize,
    pub clean_nodes: usize,
    pub total_updates: u64,
    pub update_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registration_and_update() {
        let mut graph = DiffUpdateGraph::new();
        let counter = std::sync::Arc::new(std::sync::Mutex::new(0));
        let c = counter.clone();
        graph.register("test", vec![], move |_dt| {
            *c.lock().unwrap() += 1;
        });
        graph.update(0.016);
        assert_eq!(*counter.lock().unwrap(), 1);

        graph.update(0.016);
        assert_eq!(*counter.lock().unwrap(), 1);
    }

    #[test]
    fn test_dirty_propagation() {
        let mut graph = DiffUpdateGraph::new();
        let _a_updated = false;
        let _b_updated = false;

        let a = graph.register("A", vec![], move |_dt| {});
        let _b = graph.register("B", vec![a], move |_dt| {});

        graph.mark_dirty(a);
        let dirty = graph.dirty_nodes();
        assert_eq!(dirty.len(), 2);
    }

    #[test]
    fn test_topological_order() {
        let mut graph = DiffUpdateGraph::new();
        let a = graph.register("A", vec![], |_| {});
        let b = graph.register("B", vec![a], |_| {});
        let c = graph.register("C", vec![a], |_| {});
        let _d = graph.register("D", vec![b, c], |_| {});

        let order = graph.topological_order();
        let pos: HashMap<NodeId, usize> =
            order.iter().enumerate().map(|(i, id)| (*id, i)).collect();

        assert!(pos[&a] < pos[&b]);
        assert!(pos[&a] < pos[&c]);
        assert!(pos[&b] < pos[&order[3]]);
        assert!(pos[&c] < pos[&order[3]]);
    }
}
