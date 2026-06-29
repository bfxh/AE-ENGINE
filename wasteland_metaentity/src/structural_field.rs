use glam::Vec3;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::HashMap;
use uuid::Uuid;

/// 结构场约束图 — 分析元体间约束关系，优化应力传播路径
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralField {
    pub constraint_graph: HashMap<Uuid, ConstraintNode>,
    pub structure_groups: Vec<StructureGroup>,
    pub stress_propagation_paths: HashMap<Uuid, Vec<StressPath>>,
    pub total_entities: usize,
    pub max_depth: u32,
    pub built_tick: u64,
    pub ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintNode {
    pub entity_id: Uuid,
    pub depth: u32,
    pub group_id: u32,
    pub is_critical: bool,
    pub upstream: Vec<Uuid>,
    pub downstream: Vec<Uuid>,
    pub constraint_type: ConstraintType,
    pub max_stress: f32,
    pub current_stress: f32,
    pub position: Vec3,
    pub mass: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintType {
    FixedJoint,
    HingeJoint,
    SliderJoint,
    BallJoint,
    WeldJoint,
    ContactConstraint,
    GravitySupport,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureGroup {
    pub id: u32,
    pub nodes: Vec<Uuid>,
    pub root_node: Uuid,
    pub total_mass: f32,
    pub center_of_mass: Vec3,
    pub bounding_box: [Vec3; 2],
    pub is_stable: bool,
    pub processing_order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressPath {
    pub from: Uuid,
    pub to: Uuid,
    pub path_nodes: Vec<Uuid>,
    pub path_length: u32,
    pub stress_multiplier: f32,
    pub damping_factor: f32,
}

impl StructuralField {
    pub fn new(tick: u64) -> Self {
        Self {
            constraint_graph: HashMap::new(),
            structure_groups: Vec::new(),
            stress_propagation_paths: HashMap::new(),
            total_entities: 0,
            max_depth: 0,
            built_tick: tick,
            ready: false,
        }
    }

    /// 从元体列表构建结构场约束图
    pub fn build_from_entities(
        &mut self,
        entities: &[(Uuid, Vec3, f32, Vec<ConstraintEdge>)],
        tick: u64,
    ) {
        self.constraint_graph.clear();
        self.structure_groups.clear();
        self.stress_propagation_paths.clear();
        self.total_entities = entities.len();
        self.built_tick = tick;

        // 第一步：构建约束节点
        for (id, pos, mass, edges) in entities {
            let node = ConstraintNode {
                entity_id: *id,
                depth: u32::MAX,
                group_id: 0,
                is_critical: false,
                upstream: edges.iter().map(|e| e.target_id).collect(),
                downstream: Vec::new(),
                constraint_type: ConstraintType::Unknown,
                max_stress: 1e8,
                current_stress: 0.0,
                position: *pos,
                mass: *mass,
            };
            self.constraint_graph.insert(*id, node);
        }

        // 第二步：填充下游引用
        let mut downstream_map: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for (id, _, _, edges) in entities {
            for edge in edges {
                downstream_map.entry(edge.target_id).or_default().push(*id);
            }
        }
        for (id, children) in &downstream_map {
            if let Some(node) = self.constraint_graph.get_mut(id) {
                node.downstream = children.clone();
            }
        }

        // 第三步：识别根节点（无上游的节点为根）
        let roots: Vec<Uuid> = self
            .constraint_graph
            .iter()
            .filter(|(_, node)| node.upstream.is_empty())
            .map(|(id, _)| *id)
            .collect();

        // 第四步：BFS计算深度和分组
        for (group_id, root) in roots.iter().enumerate() {
            self.assign_depths_and_groups(*root, 0, group_id as u32);
        }
        self.max_depth = self.constraint_graph.values().map(|n| n.depth).max().unwrap_or(0);

        // 第五步：识别关键节点（深度最大或下游最多的节点）
        for node in self.constraint_graph.values_mut() {
            node.is_critical = node.depth as f32 > self.max_depth as f32 * 0.7
                || node.downstream.len() > 3
                || node.upstream.is_empty();
            node.constraint_type = Self::infer_constraint_type(node);
        }

        // 第六步：构建结构分组
        let mut groups: HashMap<u32, Vec<Uuid>> = HashMap::new();
        for (id, node) in &self.constraint_graph {
            groups.entry(node.group_id).or_default().push(*id);
        }
        for (gid, node_ids) in groups {
            let total_mass: f32 = node_ids
                .iter()
                .filter_map(|id| self.constraint_graph.get(id))
                .map(|n| n.mass)
                .sum();
            let com: Vec3 = node_ids
                .iter()
                .filter_map(|id| self.constraint_graph.get(id))
                .map(|n| n.position * n.mass)
                .sum::<Vec3>()
                / total_mass.max(0.001);

            let mut min_bound = Vec3::splat(f32::MAX);
            let mut max_bound = Vec3::splat(f32::MIN);
            for id in &node_ids {
                if let Some(n) = self.constraint_graph.get(id) {
                    min_bound = min_bound.min(n.position);
                    max_bound = max_bound.max(n.position);
                }
            }

            let group_root = node_ids
                .iter()
                .filter_map(|id| self.constraint_graph.get(id))
                .min_by_key(|n| n.depth)
                .map(|n| n.entity_id)
                .unwrap_or(node_ids[0]);

            self.structure_groups.push(StructureGroup {
                id: gid,
                nodes: node_ids,
                root_node: group_root,
                total_mass,
                center_of_mass: com,
                bounding_box: [min_bound, max_bound],
                is_stable: true,
                processing_order: gid,
            });
        }
        self.structure_groups.sort_by_key(|g| g.processing_order);

        // 第七步：预计算应力传播路径
        self.precompute_stress_paths();

        self.ready = true;
    }

    fn assign_depths_and_groups(&mut self, node_id: Uuid, depth: u32, group_id: u32) {
        if let Some(node) = self.constraint_graph.get_mut(&node_id) {
            if node.depth != u32::MAX {
                return;
            }
            node.depth = depth;
            node.group_id = group_id;
        }

        let children: Vec<Uuid> =
            self.constraint_graph.get(&node_id).map(|n| n.downstream.clone()).unwrap_or_default();

        for child in children {
            self.assign_depths_and_groups(child, depth + 1, group_id);
        }
    }

    fn infer_constraint_type(node: &ConstraintNode) -> ConstraintType {
        if node.upstream.is_empty() {
            ConstraintType::GravitySupport
        } else if node.upstream.len() == 1 && node.downstream.is_empty() {
            ConstraintType::FixedJoint
        } else if node.upstream.len() >= 3 {
            ConstraintType::ContactConstraint
        } else {
            ConstraintType::WeldJoint
        }
    }

    fn precompute_stress_paths(&mut self) {
        self.stress_propagation_paths.clear();

        for (id, node) in &self.constraint_graph {
            if node.is_critical {
                let mut paths = Vec::new();

                for upstream_id in &node.upstream {
                    let path = self.compute_path_to_root(*upstream_id);
                    paths.push(StressPath {
                        from: *upstream_id,
                        to: *id,
                        path_nodes: path,
                        path_length: 0,
                        stress_multiplier: 1.0,
                        damping_factor: 0.95,
                    });
                }

                for downstream_id in &node.downstream {
                    let path = self.compute_path_from_node(*downstream_id);
                    paths.push(StressPath {
                        from: *id,
                        to: *downstream_id,
                        path_nodes: path,
                        path_length: 0,
                        stress_multiplier: 0.8,
                        damping_factor: 0.9,
                    });
                }

                self.stress_propagation_paths.insert(*id, paths);
            }
        }
    }

    fn compute_path_to_root(&self, start: Uuid) -> Vec<Uuid> {
        let mut path = Vec::new();
        let mut current = start;
        let mut visited = hashbrown::HashSet::new();

        while let Some(node) = self.constraint_graph.get(&current) {
            if !visited.insert(current) {
                break;
            }
            path.push(current);
            if let Some(upstream) = node.upstream.first() {
                current = *upstream;
            } else {
                break;
            }
        }
        path
    }

    fn compute_path_from_node(&self, start: Uuid) -> Vec<Uuid> {
        let mut path = Vec::new();
        let mut stack = vec![start];
        let mut visited = hashbrown::HashSet::new();

        while let Some(current) = stack.pop() {
            if !visited.insert(current) {
                continue;
            }
            path.push(current);
            if let Some(node) = self.constraint_graph.get(&current) {
                for child in node.downstream.iter().rev() {
                    stack.push(*child);
                }
            }
        }
        path
    }

    /// 沿结构场传播应力
    pub fn propagate_stress(
        &mut self,
        _impact_point: Vec3,
        force: Vec3,
        entity_id: Uuid,
        max_depth: u32,
        remaining_force: &mut f32,
    ) -> Vec<(Uuid, f32)> {
        let mut affected = Vec::new();

        if max_depth == 0 || *remaining_force <= 0.01 {
            return affected;
        }

        if let Some(node) = self.constraint_graph.get(&entity_id) {
            let stress = force.length() * 0.1;
            affected.push((entity_id, stress));

            *remaining_force *= 0.7;

            let downstream_ids: SmallVec<[Uuid; 8]> = node.downstream.iter().copied().collect();
            for downstream_id in downstream_ids {
                let sub_affected = self.propagate_stress(
                    _impact_point,
                    force * 0.5,
                    downstream_id,
                    max_depth - 1,
                    remaining_force,
                );
                affected.extend(sub_affected);
            }
        }

        affected
    }

    /// 获取受影响的结构分组（按应力传播顺序）
    pub fn get_affected_groups(&self, impact_point: Vec3, radius: f32) -> Vec<&StructureGroup> {
        self.structure_groups
            .iter()
            .filter(|g| {
                let com_dist = (g.center_of_mass - impact_point).length();
                com_dist < radius * 2.0
            })
            .collect()
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }

    pub fn stats(&self) -> StructuralFieldStats {
        StructuralFieldStats {
            total_nodes: self.constraint_graph.len(),
            total_groups: self.structure_groups.len(),
            max_depth: self.max_depth,
            critical_nodes: self.constraint_graph.values().filter(|n| n.is_critical).count(),
            total_stress_paths: self.stress_propagation_paths.len(),
            ready: self.ready,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConstraintEdge {
    pub target_id: Uuid,
    pub constraint_type: ConstraintType,
    pub max_force: f32,
}

#[derive(Debug, Clone)]
pub struct StructuralFieldStats {
    pub total_nodes: usize,
    pub total_groups: usize,
    pub max_depth: u32,
    pub critical_nodes: usize,
    pub total_stress_paths: usize,
    pub ready: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_simple_structure() {
        let mut sf = StructuralField::new(0);
        let entities = vec![
            (
                Uuid::new_v4(),
                Vec3::new(0.0, 0.0, 0.0),
                10.0,
                vec![ConstraintEdge {
                    target_id: Uuid::nil(),
                    constraint_type: ConstraintType::FixedJoint,
                    max_force: 1000.0,
                }],
            ),
            (
                Uuid::new_v4(),
                Vec3::new(0.0, 1.0, 0.0),
                5.0,
                vec![ConstraintEdge {
                    target_id: Uuid::nil(),
                    constraint_type: ConstraintType::FixedJoint,
                    max_force: 500.0,
                }],
            ),
        ];
        sf.build_from_entities(&entities, 0);
        assert!(sf.is_ready());
        assert_eq!(sf.stats().total_nodes, 2);
    }

    #[test]
    fn test_stress_propagation() {
        let mut sf = StructuralField::new(0);
        let root = Uuid::new_v4();
        let child = Uuid::new_v4();
        let entities = vec![
            (root, Vec3::ZERO, 10.0, Vec::new()),
            (
                child,
                Vec3::new(0.0, 1.0, 0.0),
                5.0,
                vec![ConstraintEdge {
                    target_id: root,
                    constraint_type: ConstraintType::FixedJoint,
                    max_force: 500.0,
                }],
            ),
        ];
        sf.build_from_entities(&entities, 0);

        let mut remaining = 100.0;
        let affected = sf.propagate_stress(
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, -10.0, 0.0),
            child,
            5,
            &mut remaining,
        );
        assert!(!affected.is_empty());
    }
}
