use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BtStatus {
    Success,
    Failure,
    Running,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtContext {
    pub blackboard: Vec<(String, f32)>,
    pub elapsed: f32,
}

impl BtContext {
    pub fn new() -> Self {
        Self { blackboard: Vec::new(), elapsed: 0.0 }
    }

    pub fn set(&mut self, key: &str, value: f32) {
        if let Some((_, v)) = self.blackboard.iter_mut().find(|(k, _)| k == key) {
            *v = value;
        } else {
            self.blackboard.push((key.to_string(), value));
        }
    }

    pub fn get(&self, key: &str) -> f32 {
        self.blackboard.iter().find(|(k, _)| k == key).map(|(_, v)| *v).unwrap_or(0.0)
    }
}

impl Default for BtContext {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BtNode {
    Sequence { name: String, children: Vec<BtNode>, current: usize },
    Selector { name: String, children: Vec<BtNode> },
    Parallel { name: String, children: Vec<BtNode>, required_successes: usize },
    Condition { name: String, condition_key: String, threshold: f32, operator: ComparisonOp },
    Action { name: String, effect_key: String, effect_value: f32 },
    Wait { name: String, duration: f32, elapsed: f32 },
    Loop { name: String, child: Box<BtNode>, max_iterations: usize, iterations: usize },
    Inverter { name: String, child: Box<BtNode> },
    Succeeder { name: String, child: Box<BtNode> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOp {
    GreaterThan,
    LessThan,
    Equal,
    GreaterOrEqual,
    LessOrEqual,
}

impl ComparisonOp {
    pub fn evaluate(&self, a: f32, b: f32) -> bool {
        match self {
            ComparisonOp::GreaterThan => a > b,
            ComparisonOp::LessThan => a < b,
            ComparisonOp::Equal => (a - b).abs() < 0.001,
            ComparisonOp::GreaterOrEqual => a >= b,
            ComparisonOp::LessOrEqual => a <= b,
        }
    }
}

impl BtNode {
    pub fn tick(&mut self, ctx: &mut BtContext) -> BtStatus {
        match self {
            BtNode::Sequence { children, current, .. } => {
                while *current < children.len() {
                    match children[*current].tick(ctx) {
                        BtStatus::Success => *current += 1,
                        BtStatus::Failure => {
                            *current = 0;
                            return BtStatus::Failure;
                        },
                        BtStatus::Running => return BtStatus::Running,
                    }
                }
                *current = 0;
                BtStatus::Success
            },
            BtNode::Selector { children, .. } => {
                for child in children.iter_mut() {
                    match child.tick(ctx) {
                        BtStatus::Success => return BtStatus::Success,
                        BtStatus::Running => return BtStatus::Running,
                        BtStatus::Failure => continue,
                    }
                }
                BtStatus::Failure
            },
            BtNode::Parallel { children, required_successes, .. } => {
                let mut successes = 0;
                let mut running = false;
                for child in children.iter_mut() {
                    match child.tick(ctx) {
                        BtStatus::Success => successes += 1,
                        BtStatus::Running => running = true,
                        BtStatus::Failure => {},
                    }
                }
                if successes >= *required_successes {
                    BtStatus::Success
                } else if running {
                    BtStatus::Running
                } else {
                    BtStatus::Failure
                }
            },
            BtNode::Condition { condition_key, threshold, operator, .. } => {
                let value = ctx.get(condition_key);
                if operator.evaluate(value, *threshold) {
                    BtStatus::Success
                } else {
                    BtStatus::Failure
                }
            },
            BtNode::Action { effect_key, effect_value, .. } => {
                ctx.set(effect_key, *effect_value);
                BtStatus::Success
            },
            BtNode::Wait { duration, elapsed, .. } => {
                *elapsed += ctx.elapsed;
                if *elapsed >= *duration {
                    *elapsed = 0.0;
                    BtStatus::Success
                } else {
                    BtStatus::Running
                }
            },
            BtNode::Loop { child, max_iterations, iterations, .. } => {
                for _ in *iterations..*max_iterations {
                    match child.tick(ctx) {
                        BtStatus::Success => *iterations += 1,
                        BtStatus::Failure => {
                            *iterations = 0;
                            return BtStatus::Failure;
                        },
                        BtStatus::Running => return BtStatus::Running,
                    }
                }
                *iterations = 0;
                BtStatus::Success
            },
            BtNode::Inverter { child, .. } => match child.tick(ctx) {
                BtStatus::Success => BtStatus::Failure,
                BtStatus::Failure => BtStatus::Success,
                BtStatus::Running => BtStatus::Running,
            },
            BtNode::Succeeder { child, .. } => {
                child.tick(ctx);
                BtStatus::Success
            },
        }
    }

    pub fn sequence(name: &str, children: Vec<BtNode>) -> Self {
        BtNode::Sequence { name: name.to_string(), children, current: 0 }
    }

    pub fn selector(name: &str, children: Vec<BtNode>) -> Self {
        BtNode::Selector { name: name.to_string(), children }
    }

    pub fn condition(name: &str, key: &str, op: ComparisonOp, threshold: f32) -> Self {
        BtNode::Condition {
            name: name.to_string(),
            condition_key: key.to_string(),
            threshold,
            operator: op,
        }
    }

    pub fn action(name: &str, key: &str, value: f32) -> Self {
        BtNode::Action { name: name.to_string(), effect_key: key.to_string(), effect_value: value }
    }

    pub fn inverter(name: &str, child: BtNode) -> Self {
        BtNode::Inverter { name: name.to_string(), child: Box::new(child) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_set_get() {
        let mut ctx = BtContext::new();
        ctx.set("health", 100.0);
        assert!((ctx.get("health") - 100.0).abs() < 0.01);
        ctx.set("health", 50.0);
        assert!((ctx.get("health") - 50.0).abs() < 0.01);
        assert!((ctx.get("nonexistent") - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_comparison_ops() {
        assert!(ComparisonOp::GreaterThan.evaluate(5.0, 3.0));
        assert!(!ComparisonOp::GreaterThan.evaluate(3.0, 5.0));
        assert!(ComparisonOp::LessThan.evaluate(2.0, 5.0));
        assert!(!ComparisonOp::LessThan.evaluate(5.0, 2.0));
        assert!(ComparisonOp::Equal.evaluate(1.0, 1.0));
        assert!(!ComparisonOp::Equal.evaluate(1.0, 1.1));
        assert!(ComparisonOp::GreaterOrEqual.evaluate(5.0, 5.0));
        assert!(ComparisonOp::LessOrEqual.evaluate(3.0, 3.0));
    }

    #[test]
    fn test_sequence_success() {
        let mut seq = BtNode::sequence(
            "test",
            vec![BtNode::action("a1", "x", 1.0), BtNode::action("a2", "y", 2.0)],
        );
        let mut ctx = BtContext::new();
        assert_eq!(seq.tick(&mut ctx), BtStatus::Success);
        assert!((ctx.get("x") - 1.0).abs() < 0.01);
        assert!((ctx.get("y") - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_sequence_failure() {
        let mut seq = BtNode::sequence(
            "test",
            vec![
                BtNode::action("a1", "x", 1.0),
                BtNode::condition("c1", "health", ComparisonOp::GreaterThan, 50.0),
                BtNode::action("a2", "y", 2.0),
            ],
        );
        let mut ctx = BtContext::new();
        ctx.set("health", 10.0);
        assert_eq!(seq.tick(&mut ctx), BtStatus::Failure);
        assert!((ctx.get("x") - 1.0).abs() < 0.01);
        assert!((ctx.get("y") - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_selector_first_succeeds() {
        let mut sel = BtNode::selector(
            "test",
            vec![BtNode::action("a1", "x", 10.0), BtNode::action("a2", "y", 20.0)],
        );
        let mut ctx = BtContext::new();
        assert_eq!(sel.tick(&mut ctx), BtStatus::Success);
        assert!((ctx.get("x") - 10.0).abs() < 0.01);
        assert!((ctx.get("y") - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_selector_fallback() {
        let mut sel = BtNode::selector(
            "test",
            vec![
                BtNode::condition("c1", "health", ComparisonOp::GreaterThan, 50.0),
                BtNode::action("flee", "fleeing", 1.0),
            ],
        );
        let mut ctx = BtContext::new();
        ctx.set("health", 10.0);
        assert_eq!(sel.tick(&mut ctx), BtStatus::Success);
        assert!((ctx.get("fleeing") - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_selector_all_fail() {
        let mut sel = BtNode::selector(
            "test",
            vec![
                BtNode::condition("c1", "health", ComparisonOp::GreaterThan, 50.0),
                BtNode::condition("c2", "ammo", ComparisonOp::GreaterThan, 10.0),
            ],
        );
        let mut ctx = BtContext::new();
        ctx.set("health", 10.0);
        ctx.set("ammo", 2.0);
        assert_eq!(sel.tick(&mut ctx), BtStatus::Failure);
    }

    #[test]
    fn test_parallel_success() {
        let mut par = BtNode::Parallel {
            name: "test".to_string(),
            children: vec![BtNode::action("a1", "x", 1.0), BtNode::action("a2", "y", 2.0)],
            required_successes: 2,
        };
        let mut ctx = BtContext::new();
        assert_eq!(par.tick(&mut ctx), BtStatus::Success);
    }

    #[test]
    fn test_parallel_failure() {
        let mut par = BtNode::Parallel {
            name: "test".to_string(),
            children: vec![
                BtNode::action("a1", "x", 1.0),
                BtNode::condition("c1", "health", ComparisonOp::GreaterThan, 50.0),
            ],
            required_successes: 2,
        };
        let mut ctx = BtContext::new();
        ctx.set("health", 10.0);
        assert_eq!(par.tick(&mut ctx), BtStatus::Failure);
    }

    #[test]
    fn test_condition_true() {
        let mut cond = BtNode::condition("test", "health", ComparisonOp::GreaterThan, 50.0);
        let mut ctx = BtContext::new();
        ctx.set("health", 80.0);
        assert_eq!(cond.tick(&mut ctx), BtStatus::Success);
    }

    #[test]
    fn test_condition_false() {
        let mut cond = BtNode::condition("test", "health", ComparisonOp::LessThan, 20.0);
        let mut ctx = BtContext::new();
        ctx.set("health", 80.0);
        assert_eq!(cond.tick(&mut ctx), BtStatus::Failure);
    }

    #[test]
    fn test_action_sets_value() {
        let mut action = BtNode::action("test", "score", 42.0);
        let mut ctx = BtContext::new();
        assert_eq!(action.tick(&mut ctx), BtStatus::Success);
        assert!((ctx.get("score") - 42.0).abs() < 0.01);
    }

    #[test]
    fn test_wait_not_ready() {
        let mut wait = BtNode::Wait { name: "test".to_string(), duration: 5.0, elapsed: 0.0 };
        let mut ctx = BtContext::new();
        ctx.elapsed = 2.0;
        assert_eq!(wait.tick(&mut ctx), BtStatus::Running);
    }

    #[test]
    fn test_wait_ready() {
        let mut wait = BtNode::Wait { name: "test".to_string(), duration: 3.0, elapsed: 0.0 };
        let mut ctx = BtContext::new();
        ctx.elapsed = 3.5;
        assert_eq!(wait.tick(&mut ctx), BtStatus::Success);
    }

    #[test]
    fn test_loop_completes() {
        let mut loop_node = BtNode::Loop {
            name: "test".to_string(),
            child: Box::new(BtNode::action("inc", "count", 1.0)),
            max_iterations: 3,
            iterations: 0,
        };
        let mut ctx = BtContext::new();
        ctx.set("count", 0.0);
        assert_eq!(loop_node.tick(&mut ctx), BtStatus::Success);
        assert!((ctx.get("count") - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_inverter() {
        let mut inv =
            BtNode::inverter("test", BtNode::condition("c1", "alive", ComparisonOp::Equal, 0.0));
        let mut ctx = BtContext::new();
        ctx.set("alive", 1.0);
        assert_eq!(inv.tick(&mut ctx), BtStatus::Success);
    }

    #[test]
    fn test_inverter_failure() {
        let mut inv = BtNode::inverter("test", BtNode::action("a1", "x", 1.0));
        let mut ctx = BtContext::new();
        assert_eq!(inv.tick(&mut ctx), BtStatus::Failure);
    }

    #[test]
    fn test_succeeder() {
        let mut succ = BtNode::Succeeder {
            name: "test".to_string(),
            child: Box::new(BtNode::condition("c1", "health", ComparisonOp::GreaterThan, 50.0)),
        };
        let mut ctx = BtContext::new();
        ctx.set("health", 10.0);
        assert_eq!(succ.tick(&mut ctx), BtStatus::Success);
    }
}
