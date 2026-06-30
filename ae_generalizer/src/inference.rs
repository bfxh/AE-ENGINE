use serde::{Deserialize, Serialize};
use ae_physics::fixed_point::FixedPoint;

use crate::cache::InferenceCache;
use crate::property_space::PropertyVector;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InferenceMethod {
    ExactMatch,
    NearestNeighbor,
    WeightedInterpolation,
    CosineFallback,
    NoMatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    pub property_vector: PropertyVector,
    pub method: InferenceMethod,
    pub confidence: FixedPoint,
    pub nearest_samples: Vec<(PropertyVector, FixedPoint)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownSample {
    pub input_a: PropertyVector,
    pub input_b: PropertyVector,
    pub output: PropertyVector,
    pub reaction_type: u32,
}

pub struct InferenceEngine {
    pub knowledge_base: Vec<KnownSample>,
    pub cache: InferenceCache,
    pub k_neighbors: usize,
    pub confidence_threshold: FixedPoint,
    pub max_search_samples: usize,
}

impl InferenceEngine {
    pub fn new(k_neighbors: usize, confidence_threshold: FixedPoint) -> Self {
        Self {
            knowledge_base: Vec::new(),
            cache: InferenceCache::new(),
            k_neighbors,
            confidence_threshold,
            max_search_samples: 10000,
        }
    }

    pub fn add_sample(&mut self, sample: KnownSample) {
        self.knowledge_base.push(sample);
    }

    pub fn infer(&mut self, input_a: &PropertyVector, input_b: &PropertyVector) -> InferenceResult {
        let cache_key = self.make_cache_key(input_a, input_b);
        if let Some(result) = self.cache.get(&cache_key) {
            return result;
        }

        let result = self.do_infer(input_a, input_b);
        self.cache.put(cache_key, result.clone());
        result
    }

    fn make_cache_key(&self, a: &PropertyVector, b: &PropertyVector) -> u64 {
        let mut hash: u64 = 0;
        for i in 0..4 {
            hash = hash.wrapping_mul(6364136223846793005).wrapping_add(a.values[i].raw as u64);
            hash = hash.wrapping_mul(6364136223846793005).wrapping_add(b.values[i].raw as u64);
        }
        hash
    }

    fn do_infer(&self, input_a: &PropertyVector, input_b: &PropertyVector) -> InferenceResult {
        let _combined = PropertyVector::interpolate(input_a, input_b, FixedPoint::from_f32(0.5));

        for sample in self.knowledge_base.iter().take(self.max_search_samples) {
            let dist_a = sample.input_a.distance(input_a);
            let dist_b = sample.input_b.distance(input_b);

            if dist_a < FixedPoint::EPSILON && dist_b < FixedPoint::EPSILON {
                return InferenceResult {
                    property_vector: sample.output,
                    method: InferenceMethod::ExactMatch,
                    confidence: FixedPoint::ONE,
                    nearest_samples: vec![(sample.output, FixedPoint::ONE)],
                };
            }
        }

        let mut scored: Vec<(usize, FixedPoint)> = self
            .knowledge_base
            .iter()
            .take(self.max_search_samples)
            .enumerate()
            .map(|(idx, sample)| {
                let dist_a = sample.input_a.distance(input_a);
                let dist_b = sample.input_b.distance(input_b);
                let score = (dist_a + dist_b) / FixedPoint::from_f32(2.0);
                (idx, score)
            })
            .collect();

        scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let k = self.k_neighbors.min(scored.len());
        let nearest: Vec<(PropertyVector, FixedPoint)> = scored[..k]
            .iter()
            .map(|(idx, score)| {
                let sample = &self.knowledge_base[*idx];
                let weight = if *score < FixedPoint::EPSILON {
                    FixedPoint::ONE
                } else {
                    FixedPoint::ONE / (*score + FixedPoint::EPSILON)
                };
                (sample.output, weight)
            })
            .collect();

        let result_vector = PropertyVector::weighted_average(&nearest);

        let confidence = if k > 0 {
            let best_score = scored[0].1;
            if best_score < FixedPoint::EPSILON {
                FixedPoint::ONE
            } else {
                (FixedPoint::ONE / (FixedPoint::ONE + best_score)).max(FixedPoint::from_f32(0.1))
            }
        } else {
            FixedPoint::ZERO
        };

        let method = if confidence > FixedPoint::from_f32(0.9) {
            InferenceMethod::NearestNeighbor
        } else if confidence > FixedPoint::from_f32(0.5) {
            InferenceMethod::WeightedInterpolation
        } else {
            let sim = input_a.cosine_similarity(input_b);
            if sim > FixedPoint::from_f32(0.8) {
                InferenceMethod::CosineFallback
            } else {
                InferenceMethod::NoMatch
            }
        };

        InferenceResult {
            property_vector: result_vector,
            method,
            confidence,
            nearest_samples: nearest,
        }
    }

    pub fn knowledge_size(&self) -> usize {
        self.knowledge_base.len()
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new(5, FixedPoint::from_f32(0.3))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_simple_vector(values: &[f32]) -> PropertyVector {
        let mut vec = PropertyVector::new();
        for (i, &v) in values.iter().enumerate() {
            vec.values[i] = FixedPoint::from_f32(v);
        }
        vec
    }

    #[test]
    fn test_exact_match() {
        let mut engine = InferenceEngine::default();
        let a = make_simple_vector(&[1.0, 0.0]);
        let b = make_simple_vector(&[0.0, 1.0]);
        let c = make_simple_vector(&[1.0, 1.0]);

        engine.add_sample(KnownSample { input_a: a, input_b: b, output: c, reaction_type: 0 });

        let result = engine.infer(&a, &b);
        assert_eq!(result.method, InferenceMethod::ExactMatch);
        assert!((result.confidence.to_f32() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_nearest_neighbor() {
        let mut engine = InferenceEngine::default();
        let a1 = make_simple_vector(&[1.0, 0.0]);
        let b1 = make_simple_vector(&[0.0, 1.0]);
        let c1 = make_simple_vector(&[1.0, 1.0]);

        engine.add_sample(KnownSample { input_a: a1, input_b: b1, output: c1, reaction_type: 0 });

        let a2 = make_simple_vector(&[1.1, 0.0]);
        let b2 = make_simple_vector(&[0.0, 1.1]);
        let result = engine.infer(&a2, &b2);

        assert!(result.confidence > FixedPoint::from_f32(0.5));
    }

    #[test]
    fn test_cache_hit() {
        let mut engine = InferenceEngine::default();
        let a = make_simple_vector(&[1.0, 0.0]);
        let b = make_simple_vector(&[0.0, 1.0]);
        let c = make_simple_vector(&[1.0, 1.0]);

        engine.add_sample(KnownSample { input_a: a, input_b: b, output: c, reaction_type: 0 });

        let r1 = engine.infer(&a, &b);
        let r2 = engine.infer(&a, &b);
        assert_eq!(r1.confidence, r2.confidence);
    }

    #[test]
    fn test_empty_knowledge_base() {
        let mut engine = InferenceEngine::default();
        let a = make_simple_vector(&[1.0, 0.0]);
        let b = make_simple_vector(&[0.0, 1.0]);

        let result = engine.infer(&a, &b);
        assert_eq!(result.method, InferenceMethod::NoMatch);
        assert_eq!(result.confidence, FixedPoint::ZERO);
    }
}
