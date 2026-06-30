use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistortionModel {
    pub distortion_rate: f32,
    pub mutation_probability: f32,
    pub amplification_factor: f32,
}

impl Default for DistortionModel {
    fn default() -> Self {
        Self { distortion_rate: 0.05, mutation_probability: 0.1, amplification_factor: 1.0 }
    }
}

impl DistortionModel {
    pub fn distort(&self, message: &Message, rng: &mut impl rand::Rng) -> Message {
        let mut distorted = message.clone();
        distorted.generation += 1;

        for _ in 0..((self.distortion_rate * message.content.len() as f32) as usize)
            .min(message.content.len())
        {
            if rng.gen::<f32>() < self.mutation_probability {
                let pos = rng.gen_range(0..distorted.content.len());
                distorted.content.replace_range(pos..pos + 1, &Self::random_char(rng).to_string());
            }
        }

        let bias = rng.gen_range(-0.2..0.2) * self.amplification_factor;
        distorted.accuracy = (message.accuracy * (1.0 + bias)).clamp(0.0, 1.0);

        distorted
    }

    fn random_char(rng: &mut impl rand::Rng) -> char {
        let chars = "abcdefghijklmnopqrstuvwxyz";
        chars.chars().nth(rng.gen_range(0..chars.len())).unwrap_or('a')
    }

    pub fn rumor_amplify(&self, message: &Message, emotional_charge: f32) -> Message {
        let mut amplified = message.clone();
        amplified.accuracy *= 1.0 - emotional_charge * 0.3;
        amplified.accuracy = amplified.accuracy.clamp(0.0, 1.0);
        amplified.sensationalism += emotional_charge * 0.1;
        amplified.sensationalism = amplified.sensationalism.clamp(0.0, 1.0);
        amplified
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub content: String,
    pub origin: String,
    pub topic: String,
    pub accuracy: f32,
    pub generation: u32,
    pub sensationalism: f32,
    pub version_history: Vec<MessageVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageVersion {
    pub generation: u32,
    pub content: String,
    pub accuracy: f32,
    pub changed_by: String,
}

impl Message {
    pub fn new(content: &str, origin: &str, topic: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content: content.to_string(),
            origin: origin.to_string(),
            topic: topic.to_string(),
            accuracy: 1.0,
            generation: 0,
            sensationalism: 0.0,
            version_history: Vec::new(),
        }
    }

    pub fn is_reliable(&self) -> bool {
        self.accuracy > 0.7 && self.sensationalism < 0.3
    }

    pub fn is_rumor(&self) -> bool {
        self.accuracy < 0.4 || self.sensationalism > 0.7
    }

    pub fn compare_with_source(&self, source: &Message) -> f32 {
        let same_words =
            self.content.split_whitespace().filter(|w| source.content.contains(w)).count() as f32;
        let total_words = self.content.split_whitespace().count().max(1) as f32;
        same_words / total_words
    }
}
