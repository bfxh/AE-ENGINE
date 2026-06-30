use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationController {
    pub id: String,
    pub sensors: Vec<Sensor>,
    pub actuators: Vec<Actuator>,
    pub rules: Vec<AutomationRule>,
    pub program: Option<AutomationProgram>,
    pub tick_rate: f32,
    pub tick_counter: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sensor {
    pub id: String,
    pub sensor_type: SensorType,
    pub target_id: String,
    pub current_value: f32,
    pub precision: f32,
    pub calibration: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensorType {
    Temperature,
    Pressure,
    FlowRate,
    Mass,
    Hardness,
    Purity,
    Position,
    Velocity,
    Proximity,
    Optical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actuator {
    pub id: String,
    pub actuator_type: ActuatorType,
    pub target_id: String,
    pub current_state: f32,
    pub min_value: f32,
    pub max_value: f32,
    pub response_time: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActuatorType {
    Valve,
    Motor,
    Heater,
    Cooler,
    Pump,
    Switch,
    Gripper,
    Injector,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRule {
    pub condition: Condition,
    pub actions: Vec<Action>,
    pub priority: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub sensor_id: String,
    pub operator: ComparisonOperator,
    pub threshold: f32,
    pub hysteresis: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOperator {
    GreaterThan,
    LessThan,
    Equal,
    Between,
    OutsideRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub actuator_id: String,
    pub target_value: f32,
    pub ramp_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationProgram {
    pub steps: Vec<ProgramStep>,
    pub current_step: usize,
    pub loop_mode: LoopMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramStep {
    pub actions: Vec<Action>,
    pub duration: f32,
    pub transition_condition: Option<Condition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopMode {
    Once,
    Repeat,
    PingPong,
}

impl Default for AutomationController {
    fn default() -> Self {
        Self::new()
    }
}

impl AutomationController {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            sensors: Vec::new(),
            actuators: Vec::new(),
            rules: Vec::new(),
            program: None,
            tick_rate: 10.0,
            tick_counter: 0.0,
        }
    }

    pub fn add_sensor(&mut self, sensor: Sensor) {
        self.sensors.push(sensor);
    }

    pub fn add_actuator(&mut self, actuator: Actuator) {
        self.actuators.push(actuator);
    }

    pub fn add_rule(&mut self, rule: AutomationRule) {
        self.rules.push(rule);
    }

    pub fn update_sensor(&mut self, sensor_id: &str, value: f32) {
        if let Some(sensor) = self.sensors.iter_mut().find(|s| s.id == sensor_id) {
            sensor.current_value = value * sensor.calibration;
        }
    }

    pub fn read_sensor(&self, sensor_id: &str) -> Option<f32> {
        self.sensors.iter().find(|s| s.id == sensor_id).map(|s| s.current_value)
    }

    pub fn evaluate_condition(&self, condition: &Condition) -> bool {
        let value = match self.read_sensor(&condition.sensor_id) {
            Some(v) => v,
            None => return false,
        };

        match condition.operator {
            ComparisonOperator::GreaterThan => value > condition.threshold,
            ComparisonOperator::LessThan => value < condition.threshold,
            ComparisonOperator::Equal => (value - condition.threshold).abs() < condition.hysteresis,
            ComparisonOperator::Between => {
                value > condition.threshold && value < condition.threshold + condition.hysteresis
            },
            ComparisonOperator::OutsideRange => {
                value < condition.threshold || value > condition.threshold + condition.hysteresis
            },
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.tick_counter += dt;
        if self.tick_counter < 1.0 / self.tick_rate {
            return;
        }
        self.tick_counter = 0.0;

        self.rules.sort_by_key(|r| r.priority);
        let mut pending_actions: Vec<Action> = Vec::new();
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }
            if self.evaluate_condition(&rule.condition) {
                pending_actions.extend(rule.actions.clone());
            }
        }
        for action in &pending_actions {
            self.execute_action(action, dt);
        }

        if let Some(program) = &mut self.program {
            if program.current_step < program.steps.len() {
                let actions: Vec<Action> = program.steps[program.current_step].actions.clone();
                for action in &actions {
                    self.execute_action(action, dt);
                }
            }
        }
    }

    fn execute_action(&mut self, action: &Action, dt: f32) {
        if let Some(actuator) = self.actuators.iter_mut().find(|a| a.id == action.actuator_id) {
            let diff = action.target_value - actuator.current_state;
            let step = action.ramp_rate * dt;
            if diff.abs() <= step {
                actuator.current_state = action.target_value;
            } else {
                actuator.current_state += diff.signum() * step;
            }
            actuator.current_state =
                actuator.current_state.clamp(actuator.min_value, actuator.max_value);
        }
    }

    pub fn actuator_state(&self, actuator_id: &str) -> Option<f32> {
        self.actuators.iter().find(|a| a.id == actuator_id).map(|a| a.current_state)
    }
}
