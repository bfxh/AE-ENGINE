use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtilityScore {
    pub action_name: String,
    pub score: f32,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtilityAction {
    pub name: String,
    pub base_utility: f32,
    pub considerations: Vec<UtilityConsideration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtilityConsideration {
    pub input_key: String,
    pub curve: ResponseCurve,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseCurve {
    Linear { slope: f32, intercept: f32 },
    Quadratic { a: f32, b: f32, c: f32, min: f32, max: f32 },
    Logistic { midpoint: f32, steepness: f32 },
    Threshold { threshold: f32, above: f32, below: f32 },
    Inverse { threshold: f32, steepness: f32 },
}

impl ResponseCurve {
    pub fn evaluate(&self, input: f32) -> f32 {
        match self {
            ResponseCurve::Linear { slope, intercept } => {
                (slope * input + intercept).clamp(0.0, 1.0)
            },
            ResponseCurve::Quadratic { a, b, c, min, max } => {
                let val = a * input * input + b * input + c;
                val.clamp(*min, *max)
            },
            ResponseCurve::Logistic { midpoint, steepness } => {
                1.0 / (1.0 + (-steepness * (input - midpoint)).exp())
            },
            ResponseCurve::Threshold { threshold, above, below } => {
                if input >= *threshold {
                    *above
                } else {
                    *below
                }
            },
            ResponseCurve::Inverse { threshold, steepness } => {
                1.0 - 1.0 / (1.0 + (-steepness * (input - threshold)).exp())
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtilityAI {
    pub actions: Vec<UtilityAction>,
    pub state: Vec<(String, f32)>,
}

impl UtilityAI {
    pub fn new() -> Self {
        Self { actions: Vec::new(), state: Vec::new() }
    }

    pub fn add_action(&mut self, action: UtilityAction) {
        self.actions.push(action);
    }

    pub fn set_state(&mut self, key: &str, value: f32) {
        if let Some((_, v)) = self.state.iter_mut().find(|(k, _)| k == key) {
            *v = value;
        } else {
            self.state.push((key.to_string(), value));
        }
    }

    pub fn get_state(&self, key: &str) -> f32 {
        self.state.iter().find(|(k, _)| k == key).map(|(_, v)| *v).unwrap_or(0.0)
    }

    pub fn score_action(&self, action: &UtilityAction) -> UtilityScore {
        let mut total_score = action.base_utility;
        let mut reasons = Vec::new();

        for consideration in &action.considerations {
            let input = self.get_state(&consideration.input_key);
            let contribution = consideration.curve.evaluate(input) * consideration.weight;
            total_score *= contribution;
            reasons.push(format!("{}: {:.2}", consideration.input_key, contribution));
        }

        UtilityScore {
            action_name: action.name.clone(),
            score: total_score.clamp(0.0, 1.0),
            reason: reasons.join(", "),
        }
    }

    pub fn select_best_action(&self) -> Option<UtilityScore> {
        self.actions
            .iter()
            .map(|a| self.score_action(a))
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
    }

    pub fn select_top_n(&self, n: usize) -> Vec<UtilityScore> {
        let mut scores: Vec<UtilityScore> =
            self.actions.iter().map(|a| self.score_action(a)).collect();
        scores.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        scores.truncate(n);
        scores
    }
}

impl Default for UtilityAI {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_curve() {
        let curve = ResponseCurve::Linear { slope: 0.5, intercept: 0.0 };
        assert!((curve.evaluate(1.0) - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_linear_curve_clamped() {
        let curve = ResponseCurve::Linear { slope: 2.0, intercept: 0.0 };
        assert!((curve.evaluate(2.0) - 1.0).abs() < 0.01);
        assert!((curve.evaluate(-1.0) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_quadratic_curve() {
        let curve = ResponseCurve::Quadratic { a: 1.0, b: 0.0, c: 0.0, min: 0.0, max: 2.0 };
        let val = curve.evaluate(1.0);
        assert!((val - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_logistic_curve() {
        let curve = ResponseCurve::Logistic { midpoint: 50.0, steepness: 0.1 };
        let val = curve.evaluate(50.0);
        assert!((val - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_logistic_curve_high() {
        let curve = ResponseCurve::Logistic { midpoint: 50.0, steepness: 0.5 };
        let val = curve.evaluate(100.0);
        assert!(val > 0.9);
    }

    #[test]
    fn test_threshold_curve() {
        let curve = ResponseCurve::Threshold { threshold: 50.0, above: 1.0, below: 0.0 };
        assert!((curve.evaluate(60.0) - 1.0).abs() < 0.01);
        assert!((curve.evaluate(40.0) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_inverse_curve() {
        let curve = ResponseCurve::Inverse { threshold: 50.0, steepness: 0.1 };
        let val = curve.evaluate(50.0);
        assert!((val - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_inverse_curve_low_input() {
        let curve = ResponseCurve::Inverse { threshold: 50.0, steepness: 0.5 };
        let val = curve.evaluate(0.0);
        assert!(val > 0.9);
    }

    #[test]
    fn test_utility_selection() {
        let mut ai = UtilityAI::new();
        ai.set_state("health", 30.0);
        ai.set_state("enemy_distance", 10.0);

        ai.add_action(UtilityAction {
            name: "flee".to_string(),
            base_utility: 0.8,
            considerations: vec![UtilityConsideration {
                input_key: "health".to_string(),
                curve: ResponseCurve::Inverse { threshold: 50.0, steepness: 0.2 },
                weight: 1.0,
            }],
        });

        ai.add_action(UtilityAction {
            name: "fight".to_string(),
            base_utility: 0.6,
            considerations: vec![UtilityConsideration {
                input_key: "health".to_string(),
                curve: ResponseCurve::Logistic { midpoint: 50.0, steepness: 0.1 },
                weight: 1.0,
            }],
        });

        let best = ai.select_best_action().unwrap();
        assert_eq!(best.action_name, "flee");
    }

    #[test]
    fn test_select_top_n() {
        let mut ai = UtilityAI::new();
        ai.set_state("health", 80.0);

        ai.add_action(UtilityAction {
            name: "fight".to_string(),
            base_utility: 0.9,
            considerations: vec![],
        });
        ai.add_action(UtilityAction {
            name: "explore".to_string(),
            base_utility: 0.5,
            considerations: vec![],
        });
        ai.add_action(UtilityAction {
            name: "rest".to_string(),
            base_utility: 0.3,
            considerations: vec![],
        });

        let top = ai.select_top_n(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].action_name, "fight");
        assert_eq!(top[1].action_name, "explore");
    }

    #[test]
    fn test_no_actions() {
        let ai = UtilityAI::new();
        assert!(ai.select_best_action().is_none());
    }

    #[test]
    fn test_utility_score_clamped() {
        let mut ai = UtilityAI::new();
        ai.set_state("health", 100.0);
        ai.add_action(UtilityAction {
            name: "overpowered".to_string(),
            base_utility: 2.0,
            considerations: vec![],
        });
        let score = ai.select_best_action().unwrap();
        assert!(score.score <= 1.0);
    }

    #[test]
    fn test_state_management() {
        let mut ai = UtilityAI::new();
        ai.set_state("a", 1.0);
        ai.set_state("b", 2.0);
        ai.set_state("a", 10.0);
        assert!((ai.get_state("a") - 10.0).abs() < 0.01);
        assert!((ai.get_state("b") - 2.0).abs() < 0.01);
        assert!((ai.get_state("c") - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_multiple_considerations() {
        let mut ai = UtilityAI::new();
        ai.set_state("health", 20.0);
        ai.set_state("ammo", 80.0);
        ai.add_action(UtilityAction {
            name: "retreat".to_string(),
            base_utility: 0.7,
            considerations: vec![
                UtilityConsideration {
                    input_key: "health".to_string(),
                    curve: ResponseCurve::Inverse { threshold: 50.0, steepness: 0.2 },
                    weight: 1.0,
                },
                UtilityConsideration {
                    input_key: "ammo".to_string(),
                    curve: ResponseCurve::Logistic { midpoint: 50.0, steepness: 0.1 },
                    weight: 0.5,
                },
            ],
        });
        let score = ai.score_action(&ai.actions[0]);
        assert!(score.score > 0.0);
        assert!(score.score <= 1.0);
    }
}
