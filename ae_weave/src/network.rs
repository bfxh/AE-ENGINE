use hashbrown::HashMap;
use slotmap::{SecondaryMap, SlotMap};
use ae_physics::fixed_point::FixedPoint;

use crate::constraint::{
    ConstraintEdge, ConstraintGroup, ConstraintId, ConstraintInput, ConstraintNode, NodeId,
    NodeNeighbors,
};

pub struct ConstraintNetwork {
    pub nodes: SlotMap<NodeId, ConstraintNode>,
    pub edges: SlotMap<ConstraintId, ConstraintEdge>,
    pub adjacency: SecondaryMap<NodeId, NodeNeighbors>,
    pub node_groups: HashMap<ConstraintGroup, Vec<NodeId>>,
    pub edge_groups: HashMap<ConstraintGroup, Vec<ConstraintId>>,
}

impl ConstraintNetwork {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            edges: SlotMap::with_key(),
            adjacency: SecondaryMap::new(),
            node_groups: HashMap::new(),
            edge_groups: HashMap::new(),
        }
    }

    pub fn add_node(
        &mut self,
        position: [FixedPoint; 3],
        inv_mass: FixedPoint,
        radius: FixedPoint,
    ) -> NodeId {
        let id =
            self.nodes.insert_with_key(|key| ConstraintNode::new(key, position, inv_mass, radius));
        self.adjacency.insert(id, NodeNeighbors::new());
        id
    }

    pub fn remove_node(&mut self, node_id: NodeId) {
        if let Some(neighbors) = self.adjacency.get(node_id) {
            let edge_ids: Vec<ConstraintId> = neighbors.iter().copied().collect();
            for edge_id in edge_ids {
                self.remove_edge(edge_id);
            }
        }
        self.nodes.remove(node_id);
        self.adjacency.remove(node_id);
    }

    pub fn add_edge(&mut self, input: ConstraintInput) -> ConstraintId {
        let surface_params = input.surface_params;
        let id = self.edges.insert_with_key(|key| {
            let mut edge = ConstraintEdge::new(
                key,
                input.node_a,
                input.node_b,
                input.constraint_type,
                input.rest_length,
                input.stiffness,
                input.compliance,
                input.group,
            );
            edge.surface_params = surface_params;
            edge
        });

        if let Some(neighbors) = self.adjacency.get_mut(input.node_a) {
            neighbors.push(id);
        }
        if let Some(neighbors) = self.adjacency.get_mut(input.node_b) {
            neighbors.push(id);
        }

        self.edge_groups.entry(input.group).or_default().push(id);

        id
    }

    pub fn remove_edge(&mut self, edge_id: ConstraintId) {
        if let Some(edge) = self.edges.get(edge_id) {
            let (na, nb) = (edge.node_a, edge.node_b);
            if let Some(neighbors) = self.adjacency.get_mut(na) {
                neighbors.retain(|e| *e != edge_id);
            }
            if let Some(neighbors) = self.adjacency.get_mut(nb) {
                neighbors.retain(|e| *e != edge_id);
            }
        }
        self.edges.remove(edge_id);
    }

    pub fn get_neighbors(&self, node_id: NodeId) -> &[ConstraintId] {
        self.adjacency.get(node_id).map(|n| n.as_slice()).unwrap_or(&[])
    }

    pub fn pin_node(&mut self, node_id: NodeId) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.pin();
        }
    }

    pub fn apply_gravity(&mut self, gravity: FixedPoint) {
        for node in self.nodes.values_mut() {
            if !node.pinned {
                node.velocity[1] -= gravity;
            }
        }
    }

    pub fn predict_positions(&mut self, dt: FixedPoint) {
        for node in self.nodes.values_mut() {
            if !node.pinned {
                node.prev_position = node.position;
                for i in 0..3 {
                    node.position[i] += node.velocity[i] * dt;
                }
            }
        }
    }

    pub fn update_velocities(&mut self, inv_dt: FixedPoint) {
        for node in self.nodes.values_mut() {
            if !node.pinned {
                for i in 0..3 {
                    node.velocity[i] = (node.position[i] - node.prev_position[i]) * inv_dt;
                }
            }
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn active_edge_count(&self) -> usize {
        self.edges.values().filter(|e| e.active).count()
    }
}

impl Default for ConstraintNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::ConstraintType;

    #[test]
    fn test_create_network() {
        let network = ConstraintNetwork::new();
        assert_eq!(network.node_count(), 0);
        assert_eq!(network.edge_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut network = ConstraintNetwork::new();
        let pos = [FixedPoint::ZERO; 3];
        let id = network.add_node(pos, FixedPoint::ONE, FixedPoint::from_f32(0.1));
        assert_eq!(network.node_count(), 1);
        assert!(network.nodes.contains_key(id));
    }

    #[test]
    fn test_add_edge() {
        let mut network = ConstraintNetwork::new();
        let pos = [FixedPoint::ZERO; 3];
        let na = network.add_node(pos, FixedPoint::ONE, FixedPoint::from_f32(0.1));
        let nb = network.add_node(
            [FixedPoint::ONE, FixedPoint::ZERO, FixedPoint::ZERO],
            FixedPoint::ONE,
            FixedPoint::from_f32(0.1),
        );

        let edge_id = network.add_edge(ConstraintInput {
            node_a: na,
            node_b: nb,
            constraint_type: ConstraintType::Elastic,
            rest_length: FixedPoint::ONE,
            stiffness: FixedPoint::from_f32(100.0),
            compliance: FixedPoint::from_f32(0.001),
            group: ConstraintGroup::Structural,
            surface_params: None,
        });

        assert_eq!(network.edge_count(), 1);
        assert!(network.edges.contains_key(edge_id));
        assert_eq!(network.active_edge_count(), 1);
    }

    #[test]
    fn test_remove_node_clears_edges() {
        let mut network = ConstraintNetwork::new();
        let pos = [FixedPoint::ZERO; 3];
        let na = network.add_node(pos, FixedPoint::ONE, FixedPoint::from_f32(0.1));
        let nb = network.add_node(
            [FixedPoint::ONE, FixedPoint::ZERO, FixedPoint::ZERO],
            FixedPoint::ONE,
            FixedPoint::from_f32(0.1),
        );
        network.add_edge(ConstraintInput {
            node_a: na,
            node_b: nb,
            constraint_type: ConstraintType::Elastic,
            rest_length: FixedPoint::ONE,
            stiffness: FixedPoint::from_f32(100.0),
            compliance: FixedPoint::from_f32(0.001),
            group: ConstraintGroup::Structural,
            surface_params: None,
        });

        network.remove_node(na);
        assert_eq!(network.node_count(), 1);
        assert_eq!(network.edge_count(), 0);
    }

    #[test]
    fn test_pin_node() {
        let mut network = ConstraintNetwork::new();
        let pos = [FixedPoint::ZERO; 3];
        let id = network.add_node(pos, FixedPoint::ONE, FixedPoint::from_f32(0.1));
        network.pin_node(id);

        let node = network.nodes.get(id).unwrap();
        assert!(node.pinned);
        assert_eq!(node.inv_mass, FixedPoint::ZERO);
    }

    #[test]
    fn test_edge_damage() {
        let mut network = ConstraintNetwork::new();
        let pos = [FixedPoint::ZERO; 3];
        let na = network.add_node(pos, FixedPoint::ONE, FixedPoint::from_f32(0.1));
        let nb = network.add_node(
            [FixedPoint::ONE, FixedPoint::ZERO, FixedPoint::ZERO],
            FixedPoint::ONE,
            FixedPoint::from_f32(0.1),
        );
        let edge_id = network.add_edge(ConstraintInput {
            node_a: na,
            node_b: nb,
            constraint_type: ConstraintType::Elastic,
            rest_length: FixedPoint::ONE,
            stiffness: FixedPoint::from_f32(100.0),
            compliance: FixedPoint::from_f32(0.001),
            group: ConstraintGroup::Structural,
            surface_params: None,
        });

        network.edges[edge_id].apply_damage(FixedPoint::from_f32(0.5));
        assert_eq!(network.edges[edge_id].damage, FixedPoint::from_f32(0.5));
        assert!(network.edges[edge_id].active);

        network.edges[edge_id].apply_damage(FixedPoint::from_f32(0.6));
        assert!(!network.edges[edge_id].active);
        assert_eq!(network.active_edge_count(), 0);
    }
}
