use serde::{Deserialize, Serialize};
use ae_physics::fixed_point::FixedPoint;

pub const PROPERTY_VECTOR_SIZE: usize = 32;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PropertyVector {
    pub values: [FixedPoint; PROPERTY_VECTOR_SIZE],
}

impl PropertyVector {
    pub fn new() -> Self {
        Self { values: [FixedPoint::ZERO; PROPERTY_VECTOR_SIZE] }
    }

    pub fn from_slice(slice: &[FixedPoint]) -> Self {
        let mut values = [FixedPoint::ZERO; PROPERTY_VECTOR_SIZE];
        let len = slice.len().min(PROPERTY_VECTOR_SIZE);
        values[..len].copy_from_slice(&slice[..len]);
        Self { values }
    }

    pub fn distance_sq(&self, other: &PropertyVector) -> FixedPoint {
        let mut sum = FixedPoint::ZERO;
        for i in 0..PROPERTY_VECTOR_SIZE {
            let diff = self.values[i] - other.values[i];
            sum += diff * diff;
        }
        sum
    }

    pub fn distance(&self, other: &PropertyVector) -> FixedPoint {
        self.distance_sq(other).sqrt()
    }

    pub fn cosine_similarity(&self, other: &PropertyVector) -> FixedPoint {
        let mut dot = FixedPoint::ZERO;
        let mut mag_a = FixedPoint::ZERO;
        let mut mag_b = FixedPoint::ZERO;

        for i in 0..PROPERTY_VECTOR_SIZE {
            dot += self.values[i] * other.values[i];
            mag_a += self.values[i] * self.values[i];
            mag_b += other.values[i] * other.values[i];
        }

        let mag = mag_a.sqrt() * mag_b.sqrt();
        if mag < FixedPoint::EPSILON {
            return FixedPoint::ZERO;
        }
        dot / mag
    }

    pub fn interpolate(&self, other: &PropertyVector, t: FixedPoint) -> PropertyVector {
        let mut result = PropertyVector::new();
        let one_minus_t = FixedPoint::ONE - t;
        for i in 0..PROPERTY_VECTOR_SIZE {
            result.values[i] = self.values[i] * one_minus_t + other.values[i] * t;
        }
        result
    }

    pub fn weighted_average(vectors: &[(PropertyVector, FixedPoint)]) -> PropertyVector {
        let mut result = PropertyVector::new();
        let mut total_weight = FixedPoint::ZERO;

        for (vec, weight) in vectors {
            let w = *weight;
            for i in 0..PROPERTY_VECTOR_SIZE {
                result.values[i] += vec.values[i] * w;
            }
            total_weight += w;
        }

        if total_weight > FixedPoint::EPSILON {
            let inv_total = FixedPoint::ONE / total_weight;
            for i in 0..PROPERTY_VECTOR_SIZE {
                result.values[i] *= inv_total;
            }
        }

        result
    }

    pub fn normalize(&self) -> PropertyVector {
        let mut mag_sq = FixedPoint::ZERO;
        for i in 0..PROPERTY_VECTOR_SIZE {
            mag_sq += self.values[i] * self.values[i];
        }
        let mag = mag_sq.sqrt();
        if mag < FixedPoint::EPSILON {
            return *self;
        }

        let inv_mag = FixedPoint::ONE / mag;
        let mut result = PropertyVector::new();
        for i in 0..PROPERTY_VECTOR_SIZE {
            result.values[i] = self.values[i] * inv_mag;
        }
        result
    }
}

impl Default for PropertyVector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_zero() {
        let a = PropertyVector::new();
        let b = PropertyVector::new();
        assert_eq!(a.distance(&b), FixedPoint::ZERO);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let mut a = PropertyVector::new();
        a.values[0] = FixedPoint::ONE;
        let sim = a.cosine_similarity(&a);
        assert!((sim.to_f32() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_interpolate() {
        let mut a = PropertyVector::new();
        a.values[0] = FixedPoint::ZERO;
        let mut b = PropertyVector::new();
        b.values[0] = FixedPoint::from_f32(10.0);

        let mid = a.interpolate(&b, FixedPoint::from_f32(0.5));
        assert!((mid.values[0].to_f32() - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_weighted_average() {
        let mut a = PropertyVector::new();
        a.values[0] = FixedPoint::from_f32(2.0);
        let mut b = PropertyVector::new();
        b.values[0] = FixedPoint::from_f32(8.0);

        let avg = PropertyVector::weighted_average(&[(a, FixedPoint::ONE), (b, FixedPoint::ONE)]);
        assert!((avg.values[0].to_f32() - 5.0).abs() < 0.01);
    }
}
