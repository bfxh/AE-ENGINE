use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use ae_physics::fixed_point::FixedPoint;

slotmap::new_key_type! {
    pub struct NodeKey;
}

slotmap::new_key_type! {
    pub struct ConstraintKey;
}

pub type NodeId = NodeKey;
pub type ConstraintId = ConstraintKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintType {
    Fixed,
    Elastic,
    Variable,
    Repulsion,
    Attraction,
    Free,
    Surface,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SurfaceParams {
    pub yield_strength: FixedPoint,
    pub ultimate_strength: FixedPoint,
    pub hardness: FixedPoint,
    pub friction_coefficient: FixedPoint,
    pub surface_roughness: FixedPoint,
    pub contact_area: FixedPoint,
    pub plastic_strain: FixedPoint,
}

impl Default for SurfaceParams {
    fn default() -> Self {
        Self {
            yield_strength: FixedPoint::from_f32(1e7),
            ultimate_strength: FixedPoint::from_f32(2e7),
            hardness: FixedPoint::from_f32(5.0),
            friction_coefficient: FixedPoint::from_f32(0.5),
            surface_roughness: FixedPoint::from_f32(0.1),
            contact_area: FixedPoint::ONE,
            plastic_strain: FixedPoint::ZERO,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConstraintGroup {
    Structural,
    Contact,
    Fluid,
    Thermal,
    Custom(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintNode {
    pub id: NodeId,
    pub position: [FixedPoint; 3],
    pub prev_position: [FixedPoint; 3],
    pub velocity: [FixedPoint; 3],
    pub inv_mass: FixedPoint,
    pub radius: FixedPoint,
    pub pinned: bool,
    pub group: ConstraintGroup,
}

impl ConstraintNode {
    pub fn new(
        id: NodeId,
        position: [FixedPoint; 3],
        inv_mass: FixedPoint,
        radius: FixedPoint,
    ) -> Self {
        Self {
            id,
            position,
            prev_position: position,
            velocity: [FixedPoint::ZERO; 3],
            inv_mass,
            radius,
            pinned: false,
            group: ConstraintGroup::Structural,
        }
    }

    pub fn pin(&mut self) {
        self.pinned = true;
        self.inv_mass = FixedPoint::ZERO;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintEdge {
    pub id: ConstraintId,
    pub node_a: NodeId,
    pub node_b: NodeId,
    pub constraint_type: ConstraintType,
    pub rest_length: FixedPoint,
    pub stiffness: FixedPoint,
    pub damping: FixedPoint,
    pub compliance: FixedPoint,
    pub max_force: FixedPoint,
    pub group: ConstraintGroup,
    pub active: bool,
    pub damage: FixedPoint,
    pub surface_params: Option<SurfaceParams>,
}

impl ConstraintEdge {
    pub fn new(
        id: ConstraintId,
        node_a: NodeId,
        node_b: NodeId,
        constraint_type: ConstraintType,
        rest_length: FixedPoint,
        stiffness: FixedPoint,
        compliance: FixedPoint,
        group: ConstraintGroup,
    ) -> Self {
        Self {
            id,
            node_a,
            node_b,
            constraint_type,
            rest_length,
            stiffness,
            damping: FixedPoint::from_f32(0.1),
            compliance,
            max_force: FixedPoint::MAX,
            group,
            active: true,
            damage: FixedPoint::ZERO,
            surface_params: None,
        }
    }

    pub fn apply_damage(&mut self, amount: FixedPoint) {
        self.damage = (self.damage + amount).min(FixedPoint::ONE);
        if self.damage >= FixedPoint::ONE {
            self.active = false;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintInput {
    pub node_a: NodeId,
    pub node_b: NodeId,
    pub constraint_type: ConstraintType,
    pub rest_length: FixedPoint,
    pub stiffness: FixedPoint,
    pub compliance: FixedPoint,
    pub group: ConstraintGroup,
    pub surface_params: Option<SurfaceParams>,
}

impl ConstraintInput {
    pub fn with_surface(mut self, params: SurfaceParams) -> Self {
        self.surface_params = Some(params);
        self
    }
}

pub type NodeNeighbors = SmallVec<[ConstraintId; 8]>;
