use hashbrown::HashMap;
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq)]
pub enum TransitionCondition {
    Bool { name: String, value: bool },
    FloatGreater { name: String, threshold: f32 },
    FloatLess { name: String, threshold: f32 },
    Trigger { name: String },
    Elapsed { duration: f32 },
    Always,
}

#[derive(Debug, Clone)]
pub struct Transition<S: Clone + Eq + Hash> {
    pub from: S,
    pub to: S,
    pub conditions: Vec<TransitionCondition>,
    pub priority: i32,
    pub blend_duration: f32,
}

#[derive(Debug, Clone)]
pub struct AnimationState<S: Clone + Eq + Hash> {
    pub state: S,
    pub animation_name: String,
    pub speed: f32,
    pub loop_animation: bool,
    pub transitions: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct AnimationStateMachine<S: Clone + Eq + Hash> {
    pub states: HashMap<S, AnimationState<S>>,
    pub transitions: Vec<Transition<S>>,
    pub current_state: S,
    pub blend_progress: f32,
    pub active_blend_duration: f32,
    pub elapsed: f32,
    pub bools: HashMap<String, bool>,
    pub floats: HashMap<String, f32>,
    pub triggers: HashMap<String, bool>,
}

impl<S: Clone + Eq + Hash> AnimationStateMachine<S> {
    pub fn new(initial_state: S) -> Self {
        Self {
            states: HashMap::new(),
            transitions: Vec::new(),
            current_state: initial_state,
            blend_progress: 0.0,
            active_blend_duration: 0.0,
            elapsed: 0.0,
            bools: HashMap::new(),
            floats: HashMap::new(),
            triggers: HashMap::new(),
        }
    }

    pub fn add_state(&mut self, state: S, animation: &str, speed: f32, loop_anim: bool) {
        self.states.insert(
            state.clone(),
            AnimationState {
                state,
                animation_name: animation.to_string(),
                speed,
                loop_animation: loop_anim,
                transitions: Vec::new(),
            },
        );
    }

    pub fn add_transition(
        &mut self,
        from: S,
        to: S,
        conditions: Vec<TransitionCondition>,
        priority: i32,
        blend_duration: f32,
    ) -> usize {
        let idx = self.transitions.len();
        let trans =
            Transition { from: from.clone(), to: to.clone(), conditions, priority, blend_duration };
        self.transitions.push(trans);
        if let Some(state) = self.states.get_mut(&from) {
            state.transitions.push(idx);
        }
        idx
    }

    pub fn set_bool(&mut self, name: &str, value: bool) {
        self.bools.insert(name.to_string(), value);
    }

    pub fn set_float(&mut self, name: &str, value: f32) {
        self.floats.insert(name.to_string(), value);
    }

    pub fn trigger(&mut self, name: &str) {
        self.triggers.insert(name.to_string(), true);
    }

    pub fn update(&mut self, dt: f32) {
        self.elapsed += dt;

        if let Some(state) = self.states.get(&self.current_state) {
            let transition_indices: Vec<usize> = state.transitions.clone();
            let mut matching: Vec<(usize, i32)> = Vec::new();

            for &t_idx in &transition_indices {
                let trans = &self.transitions[t_idx];
                if self.evaluate_conditions(&trans.conditions) {
                    matching.push((t_idx, trans.priority));
                }
            }

            if !matching.is_empty() {
                matching.sort_by_key(|(_, p)| -*p);
                let (best_idx, _) = matching[0];
                let trans = &self.transitions[best_idx];
                self.current_state = trans.to.clone();
                self.blend_progress = 0.0;
                self.active_blend_duration = trans.blend_duration;
                self.elapsed = 0.0;
            }
        }

        self.blend_progress = if self.active_blend_duration > 0.0 {
            (self.blend_progress + dt / self.active_blend_duration).min(1.0)
        } else {
            1.0
        };

        for (_, v) in self.triggers.iter_mut() {
            *v = false;
        }
    }

    fn evaluate_conditions(&self, conditions: &[TransitionCondition]) -> bool {
        conditions.iter().all(|c| match c {
            TransitionCondition::Bool { name, value } => {
                self.bools.get(name).copied().unwrap_or(false) == *value
            },
            TransitionCondition::FloatGreater { name, threshold } => {
                self.floats.get(name).copied().unwrap_or(0.0) > *threshold
            },
            TransitionCondition::FloatLess { name, threshold } => {
                self.floats.get(name).copied().unwrap_or(f32::MAX) < *threshold
            },
            TransitionCondition::Trigger { name } => {
                self.triggers.get(name).copied().unwrap_or(false)
            },
            TransitionCondition::Elapsed { duration } => self.elapsed >= *duration,
            TransitionCondition::Always => true,
        })
    }

    pub fn current_animation(&self) -> Option<&str> {
        self.states.get(&self.current_state).map(|s| s.animation_name.as_str())
    }

    pub fn current_speed(&self) -> f32 {
        self.states.get(&self.current_state).map(|s| s.speed).unwrap_or(1.0)
    }

    pub fn is_blending(&self) -> bool {
        self.blend_progress < 1.0
    }

    pub fn blend_factor(&self) -> f32 {
        self.blend_progress
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum TestState {
        Idle,
        Walk,
        Run,
    }

    #[test]
    fn test_simple_transition() {
        let mut sm = AnimationStateMachine::new(TestState::Idle);
        sm.add_state(TestState::Idle, "idle_anim", 1.0, true);
        sm.add_state(TestState::Walk, "walk_anim", 1.0, true);

        sm.add_transition(
            TestState::Idle,
            TestState::Walk,
            vec![TransitionCondition::Bool { name: "moving".into(), value: true }],
            0,
            0.2,
        );

        sm.update(0.016);
        assert_eq!(sm.current_state, TestState::Idle);

        sm.set_bool("moving", true);
        sm.update(0.016);
        assert_eq!(sm.current_state, TestState::Walk);
    }

    #[test]
    fn test_trigger_transition() {
        let mut sm = AnimationStateMachine::new(TestState::Idle);
        sm.add_state(TestState::Idle, "idle", 1.0, true);
        sm.add_state(TestState::Run, "run", 1.5, true);

        sm.add_transition(
            TestState::Idle,
            TestState::Run,
            vec![TransitionCondition::Trigger { name: "sprint".into() }],
            0,
            0.1,
        );

        sm.update(0.016);
        assert_eq!(sm.current_state, TestState::Idle);

        sm.trigger("sprint");
        sm.update(0.016);
        assert_eq!(sm.current_state, TestState::Run);
    }

    #[test]
    fn test_float_threshold() {
        let mut sm = AnimationStateMachine::new(TestState::Idle);
        sm.add_state(TestState::Idle, "idle", 1.0, true);
        sm.add_state(TestState::Walk, "walk", 1.0, true);

        sm.add_transition(
            TestState::Idle,
            TestState::Walk,
            vec![TransitionCondition::FloatGreater { name: "speed".into(), threshold: 0.5 }],
            0,
            0.2,
        );

        sm.set_float("speed", 0.3);
        sm.update(0.016);
        assert_eq!(sm.current_state, TestState::Idle);

        sm.set_float("speed", 0.8);
        sm.update(0.016);
        assert_eq!(sm.current_state, TestState::Walk);
    }

    #[test]
    fn test_blend_progress() {
        let mut sm = AnimationStateMachine::new(TestState::Idle);
        sm.add_state(TestState::Idle, "idle", 1.0, true);
        sm.add_state(TestState::Walk, "walk", 1.0, true);

        sm.add_transition(
            TestState::Idle,
            TestState::Walk,
            vec![TransitionCondition::Always],
            0,
            1.0,
        );

        sm.update(0.016);
        assert!(sm.is_blending());
        sm.update(1.0);
        assert!(!sm.is_blending());
    }
}
