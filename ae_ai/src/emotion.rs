use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityTraits {
    pub openness: f32,
    pub conscientiousness: f32,
    pub extraversion: f32,
    pub agreeableness: f32,
    pub neuroticism: f32,
    pub aggression: f32,
    pub curiosity: f32,
    pub loyalty: f32,
}

impl Default for PersonalityTraits {
    fn default() -> Self {
        Self {
            openness: 0.5,
            conscientiousness: 0.5,
            extraversion: 0.5,
            agreeableness: 0.5,
            neuroticism: 0.5,
            aggression: 0.5,
            curiosity: 0.5,
            loyalty: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmotionType {
    Joy,
    Sadness,
    Fear,
    Anger,
    Disgust,
    Surprise,
    Trust,
    Anticipation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionState {
    pub emotion_type: EmotionType,
    pub intensity: f32,
    pub decay_rate: f32,
    pub source: String,
}

impl EmotionState {
    pub fn new(emotion_type: EmotionType, intensity: f32, source: &str) -> Self {
        Self {
            emotion_type,
            intensity: intensity.clamp(0.0, 1.0),
            decay_rate: 0.1,
            source: source.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionEngine {
    pub emotions: Vec<EmotionState>,
    pub personality: PersonalityTraits,
    pub mood: f32,
    pub arousal: f32,
    pub last_event_time: f32,
    pub event_history: Vec<String>,
    pub max_history: usize,
}

impl EmotionEngine {
    pub fn new(personality: PersonalityTraits) -> Self {
        Self {
            emotions: Vec::new(),
            personality,
            mood: 0.5,
            arousal: 0.3,
            last_event_time: 0.0,
            event_history: Vec::new(),
            max_history: 50,
        }
    }

    pub fn trigger_event(&mut self, event_type: &str, intensity: f32, current_time: f32) {
        self.last_event_time = current_time;
        self.event_history.push(event_type.to_string());
        if self.event_history.len() > self.max_history {
            self.event_history.remove(0);
        }

        match event_type {
            "damage_taken" => {
                self.add_emotion(EmotionType::Fear, intensity * 0.8);
                self.add_emotion(
                    EmotionType::Anger,
                    intensity * (1.0 - self.personality.agreeableness),
                );
                self.arousal = (self.arousal + intensity * 0.5).min(1.0);
            },
            "damage_dealt" => {
                self.add_emotion(EmotionType::Joy, intensity * 0.3);
                self.arousal = (self.arousal + intensity * 0.3).min(1.0);
            },
            "ally_died" => {
                self.add_emotion(
                    EmotionType::Sadness,
                    intensity * (1.0 + self.personality.neuroticism * 0.5),
                );
                self.add_emotion(EmotionType::Anger, intensity * self.personality.aggression);
            },
            "enemy_died" => {
                self.add_emotion(EmotionType::Joy, intensity * 0.5);
                self.add_emotion(EmotionType::Trust, intensity * 0.1);
            },
            "reward_received" => {
                self.add_emotion(EmotionType::Joy, intensity * 0.6);
                self.add_emotion(EmotionType::Trust, intensity * 0.3);
            },
            "betrayal" => {
                self.add_emotion(
                    EmotionType::Anger,
                    intensity * (1.0 + self.personality.aggression),
                );
                self.add_emotion(EmotionType::Sadness, intensity * 0.7);
            },
            "discovery" => {
                self.add_emotion(EmotionType::Surprise, intensity * 0.5);
                self.add_emotion(EmotionType::Anticipation, intensity * self.personality.curiosity);
            },
            "threat_detected" => {
                self.add_emotion(
                    EmotionType::Fear,
                    intensity * (1.0 + self.personality.neuroticism * 0.5),
                );
                self.arousal = (self.arousal + intensity * 0.4).min(1.0);
            },
            _ => {
                self.add_emotion(EmotionType::Surprise, intensity * 0.2);
            },
        }
    }

    fn add_emotion(&mut self, emotion_type: EmotionType, intensity: f32) {
        let clamped = intensity.clamp(0.0, 1.0);
        if let Some(existing) = self.emotions.iter_mut().find(|e| {
            matches!(e.emotion_type, _)
                && std::mem::discriminant(&e.emotion_type) == std::mem::discriminant(&emotion_type)
        }) {
            existing.intensity = (existing.intensity + clamped).min(1.0);
            return;
        }
        self.emotions.push(EmotionState::new(emotion_type, clamped, "event"));
    }

    pub fn update(&mut self, dt: f32) {
        for emotion in &mut self.emotions {
            let personality_modifier = match emotion.emotion_type {
                EmotionType::Fear => 1.0 + self.personality.neuroticism * 0.5,
                EmotionType::Anger => 1.0 + self.personality.aggression * 0.5,
                EmotionType::Joy => 1.0 + self.personality.extraversion * 0.3,
                EmotionType::Sadness => 1.0 + self.personality.neuroticism * 0.3,
                _ => 1.0,
            };
            emotion.intensity -= emotion.decay_rate * personality_modifier * dt;
            emotion.intensity = emotion.intensity.max(0.0);
        }
        self.emotions.retain(|e| e.intensity > 0.01);

        self.arousal -= 0.05 * dt;
        self.arousal = self.arousal.clamp(0.0, 1.0);

        self.update_mood();
    }

    fn update_mood(&mut self) {
        let joy = self.get_emotion_intensity(&EmotionType::Joy);
        let sadness = self.get_emotion_intensity(&EmotionType::Sadness);
        let fear = self.get_emotion_intensity(&EmotionType::Fear);
        let anger = self.get_emotion_intensity(&EmotionType::Anger);
        let trust = self.get_emotion_intensity(&EmotionType::Trust);

        let raw_mood = joy * 0.4 + trust * 0.2 - sadness * 0.3 - fear * 0.2 - anger * 0.1 + 0.5;
        self.mood = (self.mood * 0.9 + raw_mood * 0.1).clamp(0.0, 1.0);
    }

    pub fn get_emotion_intensity(&self, emotion_type: &EmotionType) -> f32 {
        self.emotions
            .iter()
            .filter(|e| {
                std::mem::discriminant(&e.emotion_type) == std::mem::discriminant(emotion_type)
            })
            .map(|e| e.intensity)
            .sum::<f32>()
            .min(1.0)
    }

    pub fn dominant_emotion(&self) -> Option<&EmotionState> {
        self.emotions.iter().max_by(|a, b| a.intensity.partial_cmp(&b.intensity).unwrap())
    }

    pub fn is_afraid(&self) -> bool {
        self.get_emotion_intensity(&EmotionType::Fear) > 0.5
    }

    pub fn is_angry(&self) -> bool {
        self.get_emotion_intensity(&EmotionType::Anger) > 0.5
    }

    pub fn is_happy(&self) -> bool {
        self.mood > 0.7
    }

    pub fn is_depressed(&self) -> bool {
        self.mood < 0.3 && self.get_emotion_intensity(&EmotionType::Sadness) > 0.4
    }

    pub fn emotion_count(&self) -> usize {
        self.emotions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emotion_trigger() {
        let traits = PersonalityTraits::default();
        let mut engine = EmotionEngine::new(traits);
        engine.trigger_event("damage_taken", 0.8, 0.0);
        assert!(engine.emotion_count() > 0);
        assert!(engine.get_emotion_intensity(&EmotionType::Fear) > 0.0);
    }

    #[test]
    fn test_emotion_decay() {
        let traits = PersonalityTraits::default();
        let mut engine = EmotionEngine::new(traits);
        engine.trigger_event("reward_received", 1.0, 0.0);
        engine.update(10.0);
        assert!(engine.get_emotion_intensity(&EmotionType::Joy) < 1.0);
    }

    #[test]
    fn test_mood_calculation() {
        let traits = PersonalityTraits::default();
        let mut engine = EmotionEngine::new(traits);
        engine.trigger_event("reward_received", 1.0, 0.0);
        engine.update(1.0);
        assert!(engine.mood > 0.5);
    }

    #[test]
    fn test_dominant_emotion() {
        let traits = PersonalityTraits::default();
        let mut engine = EmotionEngine::new(traits);
        engine.trigger_event("damage_taken", 0.9, 0.0);
        engine.trigger_event("reward_received", 0.2, 0.1);
        let _dominant = engine.dominant_emotion().unwrap();
        assert!(engine.get_emotion_intensity(&EmotionType::Fear) > 0.0);
    }

    #[test]
    fn test_is_afraid() {
        let traits = PersonalityTraits {
            neuroticism: 0.8,
            ..Default::default()
        };
        let mut engine = EmotionEngine::new(traits);
        engine.trigger_event("threat_detected", 1.0, 0.0);
        assert!(engine.is_afraid());
    }

    #[test]
    fn test_is_angry() {
        let traits = PersonalityTraits {
            aggression: 0.9,
            ..Default::default()
        };
        let mut engine = EmotionEngine::new(traits);
        engine.trigger_event("betrayal", 1.0, 0.0);
        assert!(engine.is_angry());
    }

    #[test]
    fn test_is_happy() {
        let traits = PersonalityTraits::default();
        let mut engine = EmotionEngine::new(traits);
        for _ in 0..20 {
            engine.trigger_event("reward_received", 0.5, 0.0);
            engine.update(0.1);
        }
        assert!(engine.is_happy());
    }

    #[test]
    fn test_is_depressed() {
        let traits = PersonalityTraits {
            neuroticism: 1.0,
            ..Default::default()
        };
        let mut engine = EmotionEngine::new(traits);
        for _ in 0..20 {
            engine.trigger_event("ally_died", 0.5, 0.0);
            engine.update(0.1);
        }
        assert!(engine.is_depressed());
    }

    #[test]
    fn test_damage_taken_personality() {
        let traits = PersonalityTraits {
            agreeableness: 0.1,
            neuroticism: 0.9,
            ..Default::default()
        };
        let mut engine = EmotionEngine::new(traits);
        engine.trigger_event("damage_taken", 1.0, 0.0);
        let anger = engine.get_emotion_intensity(&EmotionType::Anger);
        assert!(anger > 0.5);
        assert!(engine.arousal > 0.3);
    }

    #[test]
    fn test_damage_dealt() {
        let traits = PersonalityTraits::default();
        let mut engine = EmotionEngine::new(traits);
        engine.trigger_event("damage_dealt", 1.0, 0.0);
        assert!(engine.get_emotion_intensity(&EmotionType::Joy) > 0.0);
        assert!(engine.arousal > 0.3);
    }

    #[test]
    fn test_ally_died() {
        let traits = PersonalityTraits {
            neuroticism: 0.8,
            aggression: 0.7,
            ..Default::default()
        };
        let mut engine = EmotionEngine::new(traits);
        engine.trigger_event("ally_died", 1.0, 0.0);
        assert!(engine.get_emotion_intensity(&EmotionType::Sadness) > 0.0);
        assert!(engine.get_emotion_intensity(&EmotionType::Anger) > 0.0);
    }

    #[test]
    fn test_discovery() {
        let traits = PersonalityTraits {
            curiosity: 0.9,
            ..Default::default()
        };
        let mut engine = EmotionEngine::new(traits);
        engine.trigger_event("discovery", 1.0, 0.0);
        assert!(engine.get_emotion_intensity(&EmotionType::Surprise) > 0.0);
        assert!(engine.get_emotion_intensity(&EmotionType::Anticipation) > 0.0);
    }

    #[test]
    fn test_unknown_event() {
        let traits = PersonalityTraits::default();
        let mut engine = EmotionEngine::new(traits);
        engine.trigger_event("something_weird", 0.5, 0.0);
        assert!(engine.get_emotion_intensity(&EmotionType::Surprise) > 0.0);
    }

    #[test]
    fn test_emotion_intensity_capped() {
        let traits = PersonalityTraits::default();
        let mut engine = EmotionEngine::new(traits);
        engine.trigger_event("reward_received", 1.0, 0.0);
        engine.trigger_event("reward_received", 1.0, 0.1);
        engine.trigger_event("reward_received", 1.0, 0.2);
        assert!(engine.get_emotion_intensity(&EmotionType::Joy) <= 1.0);
    }

    #[test]
    fn test_arousal_decay() {
        let traits = PersonalityTraits::default();
        let mut engine = EmotionEngine::new(traits);
        engine.trigger_event("damage_taken", 1.0, 0.0);
        let after_trigger = engine.arousal;
        engine.update(20.0);
        assert!(engine.arousal < after_trigger);
    }

    #[test]
    fn test_event_history() {
        let traits = PersonalityTraits::default();
        let mut engine = EmotionEngine::new(traits);
        engine.trigger_event("damage_taken", 0.5, 0.0);
        engine.trigger_event("reward_received", 0.5, 0.1);
        assert_eq!(engine.event_history.len(), 2);
        assert_eq!(engine.event_history[0], "damage_taken");
        assert_eq!(engine.event_history[1], "reward_received");
    }

    #[test]
    fn test_event_history_max() {
        let traits = PersonalityTraits::default();
        let mut engine = EmotionEngine::new(traits);
        engine.max_history = 3;
        engine.trigger_event("a", 0.5, 0.0);
        engine.trigger_event("b", 0.5, 0.1);
        engine.trigger_event("c", 0.5, 0.2);
        engine.trigger_event("d", 0.5, 0.3);
        assert_eq!(engine.event_history.len(), 3);
        assert_eq!(engine.event_history[0], "b");
        assert_eq!(engine.event_history[2], "d");
    }

    #[test]
    fn test_personality_defaults() {
        let traits = PersonalityTraits::default();
        assert!((traits.openness - 0.5).abs() < 0.01);
        assert!((traits.conscientiousness - 0.5).abs() < 0.01);
        assert!((traits.extraversion - 0.5).abs() < 0.01);
        assert!((traits.agreeableness - 0.5).abs() < 0.01);
        assert!((traits.neuroticism - 0.5).abs() < 0.01);
        assert!((traits.aggression - 0.5).abs() < 0.01);
        assert!((traits.curiosity - 0.5).abs() < 0.01);
        assert!((traits.loyalty - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_emotion_state_new() {
        let state = EmotionState::new(EmotionType::Joy, 1.5, "test");
        assert!((state.intensity - 1.0).abs() < 0.01);
        let state2 = EmotionState::new(EmotionType::Sadness, -0.5, "test");
        assert!((state2.intensity - 0.0).abs() < 0.01);
    }
}
