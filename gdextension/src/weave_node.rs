use godot::prelude::*;
use slotmap::Key;

use wasteland_physics::fixed_point::FixedPoint;
use wasteland_weave::constraint::{ConstraintGroup, ConstraintInput, ConstraintType, NodeId};
use wasteland_weave::fracture::{FractureConfig, FractureSystem};
use wasteland_weave::network::ConstraintNetwork;
use wasteland_weave::solver::{SolverConfig, WeaveSolver};

#[derive(GodotClass)]
#[class(base=Node)]
pub(crate) struct WastelandWeave {
    #[var]
    solver_iterations: i64,
    #[var]
    damping: f32,
    #[var]
    fracture_enabled: bool,

    network: ConstraintNetwork,
    solver: WeaveSolver,
    fracture: FractureSystem,
    node_count: i64,
    edge_count: i64,
    fracture_count: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandWeave {
    fn init(base: Base<Node>) -> Self {
        let config = SolverConfig { substeps: 10, ..Default::default() };
        Self {
            solver_iterations: 10,
            damping: 0.98,
            fracture_enabled: true,
            network: ConstraintNetwork::new(),
            solver: WeaveSolver::new(config),
            fracture: FractureSystem::new(FractureConfig::default()),
            node_count: 0,
            edge_count: 0,
            fracture_count: 0,
            base,
        }
    }
}

#[godot_api]
impl WastelandWeave {
    #[func]
    fn add_node(&mut self, x: f32, y: f32, z: f32, inv_mass: f32, radius: f32) -> i64 {
        let pos = [FixedPoint::from_f32(x), FixedPoint::from_f32(y), FixedPoint::from_f32(z)];
        let im = FixedPoint::from_f32(inv_mass);
        let r = FixedPoint::from_f32(radius);
        let id = self.network.add_node(pos, im, r);
        self.node_count += 1;
        id.data().as_ffi() as i64
    }

    #[func]
    fn remove_node(&mut self, node_id: i64) {
        let id = NodeId::from(slotmap::KeyData::from_ffi(node_id as u64));
        self.network.remove_node(id);
        self.node_count = (self.node_count - 1).max(0);
    }

    #[func]
    fn add_edge(
        &mut self,
        node_a: i64,
        node_b: i64,
        rest_length: f32,
        stiffness: f32,
        compliance: f32,
        constraint_type: GString,
    ) -> i64 {
        let ct = match constraint_type.to_string().as_str() {
            "fixed" => ConstraintType::Fixed,
            "elastic" => ConstraintType::Elastic,
            "variable" => ConstraintType::Variable,
            "repulsion" => ConstraintType::Repulsion,
            "attraction" => ConstraintType::Attraction,
            "free" => ConstraintType::Free,
            "surface" => ConstraintType::Surface,
            _ => ConstraintType::Elastic,
        };
        let input = ConstraintInput {
            node_a: NodeId::from(slotmap::KeyData::from_ffi(node_a as u64)),
            node_b: NodeId::from(slotmap::KeyData::from_ffi(node_b as u64)),
            constraint_type: ct,
            rest_length: FixedPoint::from_f32(rest_length),
            stiffness: FixedPoint::from_f32(stiffness),
            compliance: FixedPoint::from_f32(compliance),
            group: ConstraintGroup::Structural,
            surface_params: None,
        };
        let id = self.network.add_edge(input);
        self.edge_count += 1;
        id.data().as_ffi() as i64
    }

    #[func]
    fn solve(&mut self, delta_time: f32) {
        let dt = FixedPoint::from_f32(delta_time);
        self.solver.step(&mut self.network, dt);
    }

    #[func]
    fn check_fracture(&mut self) -> i64 {
        if !self.fracture_enabled {
            return 0;
        }
        let mut rng = rand::thread_rng();
        let fractured = self.fracture.process_fracture(&mut self.network, &mut rng);
        let count = fractured.len() as i64;
        self.fracture_count += count;
        count
    }

    #[func]
    fn get_node_position(&self, node_id: i64) -> Vector3 {
        let id = NodeId::from(slotmap::KeyData::from_ffi(node_id as u64));
        if let Some(node) = self.network.nodes.get(id) {
            let p = node.position;
            Vector3::new(p[0].to_f32(), p[1].to_f32(), p[2].to_f32())
        } else {
            Vector3::ZERO
        }
    }

    #[func]
    fn get_node_count(&self) -> i64 {
        self.node_count
    }

    #[func]
    fn get_edge_count(&self) -> i64 {
        self.edge_count
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        dict! {
            "node_count" => self.node_count,
            "edge_count" => self.edge_count,
            "fracture_count" => self.fracture_count,
            "solver_iterations" => self.solver_iterations,
            "damping" => self.damping,
        }
    }
}
