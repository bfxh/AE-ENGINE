use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsAction {
    pub action_type: PhysicsActionType,
    pub target_entity: Option<String>,
    pub force: [f32; 3],
    pub torque: [f32; 3],
    pub impulse: [f32; 3],
    pub constraint_params: Option<ConstraintParams>,
    pub duration_ms: u64,
    pub priority: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhysicsActionType {
    ApplyForce,
    ApplyTorque,
    ApplyImpulse,
    AddConstraint,
    RemoveConstraint,
    ModifyMaterial,
    TriggerCollision,
    SetVelocity,
    SetPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintParams {
    pub constraint_type: BridgeConstraintType,
    pub anchor: [f32; 3],
    pub axis: Option<[f32; 3]>,
    pub limits: Option<[f32; 2]>,
    pub stiffness: f32,
    pub damping: f32,
    pub break_force: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BridgeConstraintType {
    Fixed,
    Hinge,
    Slider,
    Spring,
    Distance,
    BallJoint,
    Contact,
    Surface,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsResponse {
    pub action_id: u64,
    pub success: bool,
    pub energy_consumed: f32,
    pub resulting_velocity: [f32; 3],
    pub resulting_angular_velocity: [f32; 3],
    pub contacts_generated: u32,
    pub constraints_broken: u32,
    pub simulation_time_ms: u64,
}

pub struct PhysicsBridge {
    pub config: PhysicsBridgeConfig,
    action_counter: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsBridgeConfig {
    pub default_stiffness: f32,
    pub default_damping: f32,
    pub max_force: f32,
    pub max_torque: f32,
    pub gravity: [f32; 3],
    pub time_step: f32,
}

impl Default for PhysicsBridgeConfig {
    fn default() -> Self {
        PhysicsBridgeConfig {
            default_stiffness: 1000.0,
            default_damping: 10.0,
            max_force: 100000.0,
            max_torque: 50000.0,
            gravity: [0.0, -9.81, 0.0],
            time_step: 1.0 / 60.0,
        }
    }
}

impl PhysicsBridge {
    pub fn new(config: PhysicsBridgeConfig) -> Self {
        PhysicsBridge { config, action_counter: 0 }
    }

    pub fn translate_action(&mut self, action: &PhysicsAction) -> TranslatedPhysicsAction {
        self.action_counter += 1;
        let clamped_force = [
            action.force[0].clamp(-self.config.max_force, self.config.max_force),
            action.force[1].clamp(-self.config.max_force, self.config.max_force),
            action.force[2].clamp(-self.config.max_force, self.config.max_force),
        ];
        let clamped_torque = [
            action.torque[0].clamp(-self.config.max_torque, self.config.max_torque),
            action.torque[1].clamp(-self.config.max_torque, self.config.max_torque),
            action.torque[2].clamp(-self.config.max_torque, self.config.max_torque),
        ];
        TranslatedPhysicsAction {
            id: self.action_counter,
            action_type: action.action_type,
            target_entity: action.target_entity.clone(),
            force: clamped_force,
            torque: clamped_torque,
            impulse: action.impulse,
            constraint: action.constraint_params.as_ref().map(|c| TranslatedConstraint {
                constraint_type: c.constraint_type,
                anchor: c.anchor,
                axis: c.axis.unwrap_or([0.0, 1.0, 0.0]),
                limits: c.limits.unwrap_or([-1.0, 1.0]),
                stiffness: c.stiffness.max(self.config.default_stiffness * 0.1),
                damping: c.damping.max(self.config.default_damping * 0.1),
                break_force: c.break_force,
            }),
            duration_ms: action.duration_ms,
            priority: action.priority,
        }
    }

    pub fn simulate_response(&self, action: &TranslatedPhysicsAction) -> PhysicsResponse {
        let force_mag =
            (action.force[0].powi(2) + action.force[1].powi(2) + action.force[2].powi(2)).sqrt();
        let mass = 1.0;
        let accel = force_mag / mass;
        let velocity = accel * self.config.time_step;
        PhysicsResponse {
            action_id: action.id,
            success: true,
            energy_consumed: force_mag * velocity * self.config.time_step,
            resulting_velocity: [
                action.force[0] / mass * self.config.time_step,
                action.force[1] / mass * self.config.time_step,
                action.force[2] / mass * self.config.time_step,
            ],
            resulting_angular_velocity: [
                action.torque[0] * self.config.time_step,
                action.torque[1] * self.config.time_step,
                action.torque[2] * self.config.time_step,
            ],
            contacts_generated: 0,
            constraints_broken: 0,
            simulation_time_ms: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslatedPhysicsAction {
    pub id: u64,
    pub action_type: PhysicsActionType,
    pub target_entity: Option<String>,
    pub force: [f32; 3],
    pub torque: [f32; 3],
    pub impulse: [f32; 3],
    pub constraint: Option<TranslatedConstraint>,
    pub duration_ms: u64,
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslatedConstraint {
    pub constraint_type: BridgeConstraintType,
    pub anchor: [f32; 3],
    pub axis: [f32; 3],
    pub limits: [f32; 2],
    pub stiffness: f32,
    pub damping: f32,
    pub break_force: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionEvent {
    pub entity_a: String,
    pub entity_b: String,
    pub contact_point: [f32; 3],
    pub contact_normal: [f32; 3],
    pub penetration_depth: f32,
    pub relative_velocity: [f32; 3],
    pub impulse_applied: f32,
    pub material_a: String,
    pub material_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsQuery {
    pub query_type: PhysicsQueryType,
    pub origin: [f32; 3],
    pub direction: Option<[f32; 3]>,
    pub radius: Option<f32>,
    pub max_distance: f32,
    pub filter_mask: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhysicsQueryType {
    Raycast,
    SphereCast,
    OverlapSphere,
    OverlapBox,
    Sweep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsQueryResult {
    pub hit: bool,
    pub entity: Option<String>,
    pub point: [f32; 3],
    pub normal: [f32; 3],
    pub distance: f32,
    pub material: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_force_action() {
        let mut bridge = PhysicsBridge::new(PhysicsBridgeConfig::default());
        let action = PhysicsAction {
            action_type: PhysicsActionType::ApplyForce,
            target_entity: Some("entity_1".into()),
            force: [10.0, 0.0, 0.0],
            torque: [0.0; 3],
            impulse: [0.0; 3],
            constraint_params: None,
            duration_ms: 1000,
            priority: 1,
        };
        let translated = bridge.translate_action(&action);
        assert_eq!(translated.action_type, PhysicsActionType::ApplyForce);
        assert_eq!(translated.force, [10.0, 0.0, 0.0]);
        assert_eq!(translated.target_entity.as_deref(), Some("entity_1"));
    }

    #[test]
    fn test_translate_constraint_action() {
        let mut bridge = PhysicsBridge::new(PhysicsBridgeConfig::default());
        let action = PhysicsAction {
            action_type: PhysicsActionType::AddConstraint,
            target_entity: Some("entity_1".into()),
            force: [0.0; 3],
            torque: [0.0; 3],
            impulse: [0.0; 3],
            constraint_params: Some(ConstraintParams {
                constraint_type: BridgeConstraintType::Hinge,
                anchor: [0.0, 1.0, 0.0],
                axis: Some([0.0, 1.0, 0.0]),
                limits: Some([-0.5, 0.5]),
                stiffness: 500.0,
                damping: 5.0,
                break_force: 1000.0,
            }),
            duration_ms: 500,
            priority: 2,
        };
        let translated = bridge.translate_action(&action);
        assert!(translated.constraint.is_some());
        let c = translated.constraint.unwrap();
        assert_eq!(c.constraint_type, BridgeConstraintType::Hinge);
        assert_eq!(c.limits, [-0.5, 0.5]);
    }

    #[test]
    fn test_force_clamping() {
        let mut bridge =
            PhysicsBridge::new(PhysicsBridgeConfig { max_force: 100.0, ..Default::default() });
        let action = PhysicsAction {
            action_type: PhysicsActionType::ApplyForce,
            target_entity: None,
            force: [500.0, 0.0, 0.0],
            torque: [0.0; 3],
            impulse: [0.0; 3],
            constraint_params: None,
            duration_ms: 100,
            priority: 1,
        };
        let translated = bridge.translate_action(&action);
        assert_eq!(translated.force, [100.0, 0.0, 0.0]);
    }

    #[test]
    fn test_simulate_response() {
        let bridge = PhysicsBridge::new(PhysicsBridgeConfig::default());
        let action = TranslatedPhysicsAction {
            id: 1,
            action_type: PhysicsActionType::ApplyForce,
            target_entity: None,
            force: [10.0, 0.0, 0.0],
            torque: [0.0; 3],
            impulse: [0.0; 3],
            constraint: None,
            duration_ms: 100,
            priority: 1,
        };
        let response = bridge.simulate_response(&action);
        assert!(response.success);
        assert!(response.resulting_velocity[0] > 0.0);
    }
}
