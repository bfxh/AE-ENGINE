use godot::prelude::*;
use std::sync::Mutex;
use ae_ai::{
    BtContext, BtNode, BtStatus, ComparisonOp, EmotionEngine, EmotionType, GoapAction, GoapGoal,
    GoapPlanner, PersonalityTraits, WorldState,
};

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandAI {
    planner: Mutex<GoapPlanner>,
    state: Mutex<WorldState>,
    goals: Mutex<Vec<GoapGoal>>,
    current_plan: Mutex<Vec<String>>,

    #[var]
    max_plan_depth: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandAI {
    fn init(base: Base<Node>) -> Self {
        Self {
            planner: Mutex::new(GoapPlanner::new()),
            state: Mutex::new(WorldState::new()),
            goals: Mutex::new(Vec::new()),
            current_plan: Mutex::new(Vec::new()),
            max_plan_depth: 8,
            base,
        }
    }
}

#[godot_api]
impl WastelandAI {
    #[func]
    fn set_world_state(&mut self, state_dict: Dictionary<Variant, Variant>) {
        if let Ok(mut state) = self.state.lock() {
            for (key, value) in state_dict.iter_shared() {
                let k = key.to::<GString>();
                let v = value.to::<f32>();
                state.set(&k.to_string(), v);
            }
        }
    }

    #[func]
    fn set_fact(&mut self, key: GString, value: f32) {
        if let Ok(mut state) = self.state.lock() {
            state.set(&key.to_string(), value);
        }
    }

    #[func]
    fn get_fact(&self, key: GString) -> f32 {
        if let Ok(state) = self.state.lock() {
            return state.get(&key.to_string());
        }
        0.0
    }

    #[func]
    fn add_goal(
        &mut self,
        name: GString,
        priority: f32,
        desired_fact: GString,
        min_val: f32,
        max_val: f32,
    ) {
        if let Ok(mut goals) = self.goals.lock() {
            let goal = GoapGoal {
                name: name.to_string(),
                priority,
                desired_state: vec![(desired_fact.to_string(), min_val, max_val)],
                is_persistent: false,
            };
            goals.push(goal);
        }
    }

    #[func]
    fn add_action(
        &mut self,
        name: GString,
        cost: f32,
        duration: f32,
        preconditions: Dictionary<Variant, Variant>,
        effects: Dictionary<Variant, Variant>,
    ) {
        if let Ok(mut planner) = self.planner.lock() {
            let precond: Vec<(String, f32, f32)> = preconditions
                .iter_shared()
                .filter_map(|(k, v)| {
                    let key = k.to::<GString>().to_string();
                    let arr = v.to::<Array<Variant>>();
                    let min = arr.get(0)?.to::<f32>();
                    let max = arr.get(1)?.to::<f32>();
                    Some((key, min, max))
                })
                .collect();
            let eff: Vec<(String, f32)> = effects
                .iter_shared()
                .map(|(k, v)| {
                    let key = k.to::<GString>().to_string();
                    let val = v.to::<f32>();
                    (key, val)
                })
                .collect();
            let action = GoapAction {
                name: name.to_string(),
                cost,
                preconditions: precond,
                effects: eff,
                duration,
                requires_target: false,
                target_position: None,
            };
            planner.add_action(action);
        }
    }

    #[func]
    fn plan(&mut self) -> bool {
        if let Ok(state) = self.state.lock() {
            if let Ok(goals) = self.goals.lock() {
                if let Ok(planner) = self.planner.lock() {
                    if let Some(plan) = planner.plan(&state, &goals) {
                        let names: Vec<String> =
                            plan.actions.iter().map(|a| a.name.clone()).collect();
                        if let Ok(mut current) = self.current_plan.lock() {
                            *current = names;
                        }
                        return true;
                    }
                }
            }
        }
        false
    }

    #[func]
    fn get_plan(&self) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        if let Ok(plan) = self.current_plan.lock() {
            for step in plan.iter() {
                arr.push(step.as_str());
            }
        }
        arr
    }

    #[func]
    fn clear_goals(&mut self) {
        if let Ok(mut goals) = self.goals.lock() {
            goals.clear();
        }
    }

    #[func]
    fn clear_actions(&mut self) {
        if let Ok(mut planner) = self.planner.lock() {
            planner.available_actions.clear();
        }
    }

    #[func]
    fn reset(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            *state = WorldState::new();
        }
        if let Ok(mut goals) = self.goals.lock() {
            goals.clear();
        }
        if let Ok(mut planner) = self.planner.lock() {
            planner.available_actions.clear();
        }
        if let Ok(mut plan) = self.current_plan.lock() {
            plan.clear();
        }
    }

    #[func]
    #[allow(clippy::too_many_arguments)]
    fn add_conditional_action(
        &mut self,
        name: GString,
        cost: f32,
        duration: f32,
        preconditions_dict: Dictionary<Variant, Variant>,
        effects_dict: Dictionary<Variant, Variant>,
        condition_key: GString,
        condition_op: i64,
        condition_threshold: f32,
    ) {
        if let Ok(mut planner) = self.planner.lock() {
            let precond: Vec<(String, f32, f32)> = preconditions_dict
                .iter_shared()
                .filter_map(|(k, v)| {
                    let key = k.to::<GString>().to_string();
                    let arr = v.to::<Array<Variant>>();
                    let min = arr.get(0)?.to::<f32>();
                    let max = arr.get(1)?.to::<f32>();
                    Some((key, min, max))
                })
                .collect();
            let eff: Vec<(String, f32)> = effects_dict
                .iter_shared()
                .map(|(k, v)| {
                    let key = k.to::<GString>().to_string();
                    let val = v.to::<f32>();
                    (key, val)
                })
                .collect();
            let _cmp_op = match condition_op {
                0 => ComparisonOp::GreaterThan,
                1 => ComparisonOp::LessThan,
                2 => ComparisonOp::Equal,
                3 => ComparisonOp::GreaterOrEqual,
                _ => ComparisonOp::LessOrEqual,
            };
            let mut action = GoapAction {
                name: name.to_string(),
                cost,
                preconditions: precond,
                effects: eff,
                duration,
                requires_target: false,
                target_position: None,
            };
            action.preconditions.push((
                condition_key.to_string(),
                condition_threshold * 0.99,
                condition_threshold * 1.01,
            ));
            planner.add_action(action);
        }
    }

    #[func]
    fn get_all_actions(&self) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        if let Ok(planner) = self.planner.lock() {
            for action in &planner.available_actions {
                arr.push(action.name.as_str());
            }
        }
        arr
    }

    #[func]
    fn save_plan(&self, path: GString) -> bool {
        let path_str = path.to_string();
        if let Ok(plan) = self.current_plan.lock() {
            let content = plan.join("\n");
            let bytes = content.as_bytes();
            let mut packed = PackedByteArray::new();
            for b in bytes {
                packed.push(*b);
            }
            return std::fs::write(&path_str, packed.as_slice()).is_ok();
        }
        false
    }

    #[func]
    fn load_plan(&mut self, path: GString) -> bool {
        let path_str = path.to_string();
        if let Ok(data) = std::fs::read(&path_str) {
            if let Ok(content) = String::from_utf8(data) {
                let steps: Vec<String> =
                    content.lines().map(|l| l.to_string()).filter(|l| !l.is_empty()).collect();
                if let Ok(mut plan) = self.current_plan.lock() {
                    *plan = steps;
                    return true;
                }
            }
        }
        false
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandBehaviorTree {
    tree: Mutex<Option<BtNode>>,
    ctx: Mutex<BtContext>,
    status: Mutex<BtStatus>,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandBehaviorTree {
    fn init(base: Base<Node>) -> Self {
        Self {
            tree: Mutex::new(None),
            ctx: Mutex::new(BtContext::new()),
            status: Mutex::new(BtStatus::Success),
            base,
        }
    }
}

#[godot_api]
impl WastelandBehaviorTree {
    #[func]
    fn set_blackboard(&mut self, key: GString, value: f32) {
        if let Ok(mut ctx) = self.ctx.lock() {
            ctx.set(&key.to_string(), value);
        }
    }

    #[func]
    fn get_blackboard(&self, key: GString) -> f32 {
        if let Ok(ctx) = self.ctx.lock() {
            return ctx.get(&key.to_string());
        }
        0.0
    }

    #[func]
    fn create_sequence(&mut self, name: GString, action_keys: PackedStringArray) {
        let children: Vec<BtNode> = action_keys
            .as_slice()
            .iter()
            .map(|s| BtNode::action(&s.to_string(), &s.to_string(), 1.0))
            .collect();
        if let Ok(mut tree) = self.tree.lock() {
            *tree = Some(BtNode::sequence(&name.to_string(), children));
        }
    }

    #[func]
    fn create_selector(&mut self, name: GString, action_keys: PackedStringArray) {
        let children: Vec<BtNode> = action_keys
            .as_slice()
            .iter()
            .map(|s| BtNode::action(&s.to_string(), &s.to_string(), 1.0))
            .collect();
        if let Ok(mut tree) = self.tree.lock() {
            *tree = Some(BtNode::selector(&name.to_string(), children));
        }
    }

    #[func]
    fn add_condition(&mut self, name: GString, key: GString, op: i64, threshold: f32) {
        let cmp_op = match op {
            0 => ComparisonOp::GreaterThan,
            1 => ComparisonOp::LessThan,
            2 => ComparisonOp::Equal,
            3 => ComparisonOp::GreaterOrEqual,
            _ => ComparisonOp::LessOrEqual,
        };
        let cond = BtNode::condition(&name.to_string(), &key.to_string(), cmp_op, threshold);
        if let Ok(mut tree) = self.tree.lock() {
            *tree = Some(cond);
        }
    }

    #[func]
    fn tick(&mut self, dt: f32) -> i64 {
        if let Ok(mut ctx) = self.ctx.lock() {
            ctx.elapsed = dt;
            if let Ok(mut tree) = self.tree.lock() {
                if let Some(ref mut t) = *tree {
                    let status = t.tick(&mut ctx);
                    if let Ok(mut s) = self.status.lock() {
                        *s = status;
                    }
                    return match status {
                        BtStatus::Success => 0,
                        BtStatus::Failure => 1,
                        BtStatus::Running => 2,
                    };
                }
            }
        }
        1
    }

    #[func]
    fn reset_tree(&mut self) {
        if let Ok(mut tree) = self.tree.lock() {
            *tree = None;
        }
        if let Ok(mut ctx) = self.ctx.lock() {
            *ctx = BtContext::new();
        }
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandEmotion {
    engine: Mutex<EmotionEngine>,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandEmotion {
    fn init(base: Base<Node>) -> Self {
        Self { engine: Mutex::new(EmotionEngine::new(PersonalityTraits::default())), base }
    }
}

#[godot_api]
impl WastelandEmotion {
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn set_personality(
        &mut self,
        openness: f32,
        conscientiousness: f32,
        extraversion: f32,
        agreeableness: f32,
        neuroticism: f32,
        aggression: f32,
        curiosity: f32,
        loyalty: f32,
    ) {
        if let Ok(mut engine) = self.engine.lock() {
            engine.personality = PersonalityTraits {
                openness,
                conscientiousness,
                extraversion,
                agreeableness,
                neuroticism,
                aggression,
                curiosity,
                loyalty,
            };
        }
    }

    #[func]
    fn trigger_event(&mut self, event_type: GString, intensity: f32, time: f32) {
        if let Ok(mut engine) = self.engine.lock() {
            engine.trigger_event(&event_type.to_string(), intensity, time);
        }
    }

    #[func]
    fn update(&mut self, dt: f32) {
        if let Ok(mut engine) = self.engine.lock() {
            engine.update(dt);
        }
    }

    #[func]
    fn get_mood(&self) -> f32 {
        if let Ok(engine) = self.engine.lock() {
            return engine.mood;
        }
        0.5
    }

    #[func]
    fn get_arousal(&self) -> f32 {
        if let Ok(engine) = self.engine.lock() {
            return engine.arousal;
        }
        0.0
    }

    #[func]
    fn is_afraid(&self) -> bool {
        if let Ok(engine) = self.engine.lock() {
            return engine.is_afraid();
        }
        false
    }

    #[func]
    fn is_angry(&self) -> bool {
        if let Ok(engine) = self.engine.lock() {
            return engine.is_angry();
        }
        false
    }

    #[func]
    fn is_happy(&self) -> bool {
        if let Ok(engine) = self.engine.lock() {
            return engine.is_happy();
        }
        false
    }

    #[func]
    fn emotion_count(&self) -> i64 {
        if let Ok(engine) = self.engine.lock() {
            return engine.emotion_count() as i64;
        }
        0
    }

    #[func]
    fn get_emotion_intensities(&self) -> Dictionary<Variant, Variant> {
        if let Ok(engine) = self.engine.lock() {
            return dict! {
                "joy" => engine.get_emotion_intensity(&EmotionType::Joy),
                "sadness" => engine.get_emotion_intensity(&EmotionType::Sadness),
                "fear" => engine.get_emotion_intensity(&EmotionType::Fear),
                "anger" => engine.get_emotion_intensity(&EmotionType::Anger),
                "disgust" => engine.get_emotion_intensity(&EmotionType::Disgust),
                "surprise" => engine.get_emotion_intensity(&EmotionType::Surprise),
                "trust" => engine.get_emotion_intensity(&EmotionType::Trust),
                "anticipation" => engine.get_emotion_intensity(&EmotionType::Anticipation),
            };
        }
        dict! {}
    }
}

#[derive(GodotClass)]
#[class(base=Node, rename=WastelandMemoryAI)]
struct WastelandMemoryAI {
    memories: Mutex<Vec<(String, f32, String)>>,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandMemoryAI {
    fn init(base: Base<Node>) -> Self {
        Self { memories: Mutex::new(Vec::new()), base }
    }
}

#[godot_api]
impl WastelandMemoryAI {
    #[func]
    fn add_memory(&mut self, content: GString, importance: f32, emotion_tag: GString) {
        if let Ok(mut mem) = self.memories.lock() {
            mem.push((content.to_string(), importance, emotion_tag.to_string()));
        }
    }

    #[func]
    fn recall_by_emotion(&self, tag: GString) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        let tag_str = tag.to_string();
        if let Ok(mem) = self.memories.lock() {
            for (content, _, emotion) in mem.iter() {
                if emotion == &tag_str {
                    arr.push(content.as_str());
                }
            }
        }
        arr
    }

    #[func]
    fn recall_recent(&self, n: i64) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        if let Ok(mem) = self.memories.lock() {
            let start = if mem.len() > n as usize { mem.len() - n as usize } else { 0 };
            for (content, _, _) in &mem[start..] {
                arr.push(content.as_str());
            }
        }
        arr
    }

    #[func]
    fn memory_count(&self) -> i64 {
        if let Ok(mem) = self.memories.lock() {
            return mem.len() as i64;
        }
        0
    }

    #[func]
    fn forget_all(&mut self) {
        if let Ok(mut mem) = self.memories.lock() {
            mem.clear();
        }
    }
}
