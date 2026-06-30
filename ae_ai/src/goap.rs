use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    pub facts: Vec<(String, f32)>,
}

impl WorldState {
    pub fn new() -> Self {
        Self { facts: Vec::new() }
    }

    pub fn set(&mut self, key: &str, value: f32) {
        if let Some((_, v)) = self.facts.iter_mut().find(|(k, _)| k == key) {
            *v = value;
        } else {
            self.facts.push((key.to_string(), value));
        }
    }

    pub fn get(&self, key: &str) -> f32 {
        self.facts.iter().find(|(k, _)| k == key).map(|(_, v)| *v).unwrap_or(0.0)
    }

    pub fn satisfies(&self, precondition: &(String, f32, f32)) -> bool {
        let value = self.get(&precondition.0);
        let (_, min, max) = precondition;
        value >= *min && value <= *max
    }

    pub fn merge(&self, effects: &[(String, f32)]) -> Self {
        let mut new_state = self.clone();
        for (key, value) in effects {
            new_state.set(key, *value);
        }
        new_state
    }
}

impl Default for WorldState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoapAction {
    pub name: String,
    pub cost: f32,
    pub preconditions: Vec<(String, f32, f32)>,
    pub effects: Vec<(String, f32)>,
    pub duration: f32,
    pub requires_target: bool,
    pub target_position: Option<Vec3>,
}

impl GoapAction {
    pub fn new(name: &str, cost: f32) -> Self {
        Self {
            name: name.to_string(),
            cost,
            preconditions: Vec::new(),
            effects: Vec::new(),
            duration: 1.0,
            requires_target: false,
            target_position: None,
        }
    }

    pub fn with_precondition(mut self, key: &str, min: f32, max: f32) -> Self {
        self.preconditions.push((key.to_string(), min, max));
        self
    }

    pub fn with_effect(mut self, key: &str, value: f32) -> Self {
        self.effects.push((key.to_string(), value));
        self
    }

    pub fn is_valid(&self, state: &WorldState) -> bool {
        self.preconditions.iter().all(|p| state.satisfies(p))
    }

    pub fn apply(&self, state: &WorldState) -> WorldState {
        state.merge(&self.effects)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoapGoal {
    pub name: String,
    pub priority: f32,
    pub desired_state: Vec<(String, f32, f32)>,
    pub is_persistent: bool,
}

impl GoapGoal {
    pub fn new(name: &str, priority: f32) -> Self {
        Self { name: name.to_string(), priority, desired_state: Vec::new(), is_persistent: false }
    }

    pub fn with_desired(mut self, key: &str, min: f32, max: f32) -> Self {
        self.desired_state.push((key.to_string(), min, max));
        self
    }

    pub fn is_satisfied(&self, state: &WorldState) -> bool {
        self.desired_state.iter().all(|p| state.satisfies(p))
    }

    pub fn urgency(&self, state: &WorldState) -> f32 {
        if self.is_satisfied(state) {
            return 0.0;
        }
        let dissatisfaction: f32 = self
            .desired_state
            .iter()
            .map(|(key, min, max)| {
                let value = state.get(key);
                if value < *min {
                    *min - value
                } else if value > *max {
                    value - *max
                } else {
                    0.0
                }
            })
            .sum();
        self.priority * dissatisfaction
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoapPlan {
    pub goal: GoapGoal,
    pub actions: Vec<GoapAction>,
    pub total_cost: f32,
    pub expected_utility: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoapPlanner {
    pub available_actions: Vec<GoapAction>,
    pub max_plan_depth: usize,
    pub max_plans: usize,
}

impl GoapPlanner {
    pub fn new() -> Self {
        Self { available_actions: Vec::new(), max_plan_depth: 8, max_plans: 5 }
    }

    pub fn add_action(&mut self, action: GoapAction) {
        self.available_actions.push(action);
    }

    pub fn plan(&self, state: &WorldState, goals: &[GoapGoal]) -> Option<GoapPlan> {
        let mut active_goals: Vec<&GoapGoal> =
            goals.iter().filter(|g| !g.is_satisfied(state)).collect();
        active_goals.sort_by(|a, b| b.urgency(state).partial_cmp(&a.urgency(state)).unwrap());

        let goal = active_goals.first()?;

        let mut best_plan: Option<GoapPlan> = None;
        let mut best_cost = f32::MAX;

        self.build_plan(state, goal, &[], 0, &mut best_plan, &mut best_cost);

        best_plan
    }

    fn build_plan(
        &self,
        current_state: &WorldState,
        goal: &GoapGoal,
        current_actions: &[GoapAction],
        depth: usize,
        best_plan: &mut Option<GoapPlan>,
        best_cost: &mut f32,
    ) {
        if depth >= self.max_plan_depth {
            return;
        }

        if goal.is_satisfied(current_state) {
            let total_cost: f32 = current_actions.iter().map(|a| a.cost).sum();
            if total_cost < *best_cost {
                *best_cost = total_cost;
                *best_plan = Some(GoapPlan {
                    goal: goal.clone(),
                    actions: current_actions.to_vec(),
                    total_cost,
                    expected_utility: goal.priority - total_cost * 0.1,
                });
            }
            return;
        }

        for action in &self.available_actions {
            if !action.is_valid(current_state) {
                continue;
            }
            let new_state = action.apply(current_state);
            let mut new_actions = current_actions.to_vec();
            new_actions.push(action.clone());
            self.build_plan(&new_state, goal, &new_actions, depth + 1, best_plan, best_cost);
        }
    }
}

impl Default for GoapPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_state_set_get() {
        let mut state = WorldState::new();
        state.set("hunger", 50.0);
        assert!((state.get("hunger") - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_world_state_overwrite() {
        let mut state = WorldState::new();
        state.set("x", 10.0);
        state.set("x", 20.0);
        assert!((state.get("x") - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_world_state_satisfies() {
        let mut state = WorldState::new();
        state.set("hunger", 50.0);
        let precond = ("hunger".to_string(), 30.0, 70.0);
        assert!(state.satisfies(&precond));
        let precond2 = ("hunger".to_string(), 60.0, 80.0);
        assert!(!state.satisfies(&precond2));
    }

    #[test]
    fn test_world_state_merge() {
        let mut state = WorldState::new();
        state.set("a", 1.0);
        state.set("b", 2.0);
        let effects = vec![("a".to_string(), 10.0), ("c".to_string(), 3.0)];
        let merged = state.merge(&effects);
        assert!((merged.get("a") - 10.0).abs() < 0.01);
        assert!((merged.get("b") - 2.0).abs() < 0.01);
        assert!((merged.get("c") - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_goap_action_validity() {
        let mut state = WorldState::new();
        state.set("has_weapon", 1.0);
        let action = GoapAction::new("attack", 5.0)
            .with_precondition("has_weapon", 1.0, 1.0)
            .with_effect("enemy_alive", 0.0);
        assert!(action.is_valid(&state));
    }

    #[test]
    fn test_goap_action_invalid() {
        let state = WorldState::new();
        let action = GoapAction::new("attack", 5.0).with_precondition("has_weapon", 1.0, 1.0);
        assert!(!action.is_valid(&state));
    }

    #[test]
    fn test_goap_action_apply() {
        let state = WorldState::new();
        let action =
            GoapAction::new("eat", 2.0).with_effect("hunger", 0.0).with_effect("energy", 100.0);
        let new_state = action.apply(&state);
        assert!((new_state.get("hunger") - 0.0).abs() < 0.01);
        assert!((new_state.get("energy") - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_goal_satisfied() {
        let mut state = WorldState::new();
        state.set("hunger", 10.0);
        let goal = GoapGoal::new("survive", 10.0).with_desired("hunger", 0.0, 30.0);
        assert!(goal.is_satisfied(&state));
    }

    #[test]
    fn test_goal_not_satisfied() {
        let mut state = WorldState::new();
        state.set("hunger", 80.0);
        let goal = GoapGoal::new("survive", 10.0).with_desired("hunger", 0.0, 30.0);
        assert!(!goal.is_satisfied(&state));
    }

    #[test]
    fn test_goal_urgency() {
        let mut state = WorldState::new();
        state.set("hunger", 90.0);
        let goal = GoapGoal::new("survive", 10.0).with_desired("hunger", 0.0, 30.0);
        let urgency = goal.urgency(&state);
        assert!(urgency > 0.0);
        assert!((urgency - 600.0).abs() < 0.01);
    }

    #[test]
    fn test_goal_urgency_zero_when_satisfied() {
        let mut state = WorldState::new();
        state.set("hunger", 10.0);
        let goal = GoapGoal::new("survive", 10.0).with_desired("hunger", 0.0, 30.0);
        assert!((goal.urgency(&state) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_goap_planning() {
        let mut planner = GoapPlanner::new();
        planner.add_action(
            GoapAction::new("eat_food", 2.0)
                .with_precondition("has_food", 1.0, 1.0)
                .with_effect("hunger", 0.0),
        );
        planner.add_action(GoapAction::new("find_food", 1.0).with_effect("has_food", 1.0));

        let mut state = WorldState::new();
        state.set("hunger", 80.0);
        let goal = GoapGoal::new("survive", 10.0).with_desired("hunger", 0.0, 30.0);

        let plan = planner.plan(&state, &[goal]);
        assert!(plan.is_some());
        assert!(plan.unwrap().actions.len() >= 2);
    }

    #[test]
    fn test_goap_no_plan_possible() {
        let mut planner = GoapPlanner::new();
        planner.add_action(
            GoapAction::new("eat", 2.0)
                .with_precondition("has_food", 1.0, 1.0)
                .with_effect("hunger", 0.0),
        );

        let mut state = WorldState::new();
        state.set("hunger", 80.0);
        let goal = GoapGoal::new("survive", 10.0).with_desired("hunger", 0.0, 30.0);

        let plan = planner.plan(&state, &[goal]);
        assert!(plan.is_none());
    }

    #[test]
    fn test_goap_multiple_goals_priority() {
        let mut planner = GoapPlanner::new();
        planner.add_action(GoapAction::new("eat", 2.0).with_effect("hunger", 0.0));
        planner.add_action(GoapAction::new("sleep", 3.0).with_effect("energy", 100.0));

        let mut state = WorldState::new();
        state.set("hunger", 90.0);
        state.set("energy", 10.0);

        let survive = GoapGoal::new("survive", 10.0).with_desired("hunger", 0.0, 30.0);
        let rest = GoapGoal::new("rest", 5.0).with_desired("energy", 50.0, 100.0);

        let plan = planner.plan(&state, &[survive, rest]);
        assert!(plan.is_some());
        assert_eq!(plan.unwrap().goal.name, "survive");
    }

    #[test]
    fn test_goap_plan_expected_utility() {
        let mut planner = GoapPlanner::new();
        planner.add_action(GoapAction::new("eat", 2.0).with_effect("hunger", 0.0));

        let mut state = WorldState::new();
        state.set("hunger", 80.0);
        let goal = GoapGoal::new("survive", 10.0).with_desired("hunger", 0.0, 30.0);

        let plan = planner.plan(&state, &[goal]).unwrap();
        assert!((plan.expected_utility - (10.0 - 2.0 * 0.1)).abs() < 0.01);
        assert!((plan.total_cost - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_goap_persistent_goal() {
        let state = WorldState::new();
        let goal = GoapGoal {
            name: "always".to_string(),
            priority: 5.0,
            desired_state: vec![("x".to_string(), 50.0, 100.0)],
            is_persistent: true,
        };
        assert!(goal.is_persistent);
        assert!(!goal.is_satisfied(&state));
    }
}
