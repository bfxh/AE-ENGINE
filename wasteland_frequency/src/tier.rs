use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FrequencyTier {
    Critical = 0,
    High = 1,
    Medium = 2,
    Low = 3,
    Background = 4,
}

impl FrequencyTier {
    pub fn hz(self) -> f32 {
        match self {
            FrequencyTier::Critical => 60.0,
            FrequencyTier::High => 30.0,
            FrequencyTier::Medium => 10.0,
            FrequencyTier::Low => 1.0,
            FrequencyTier::Background => 0.1,
        }
    }

    pub fn interval_ticks(self) -> u64 {
        match self {
            FrequencyTier::Critical => 1,
            FrequencyTier::High => 2,
            FrequencyTier::Medium => 6,
            FrequencyTier::Low => 60,
            FrequencyTier::Background => 600,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FrequencyTier::Critical => "critical",
            FrequencyTier::High => "high",
            FrequencyTier::Medium => "medium",
            FrequencyTier::Low => "low",
            FrequencyTier::Background => "background",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Urgency {
    pub tier: FrequencyTier,
    pub score: f32,
}

impl Urgency {
    pub fn new(tier: FrequencyTier, score: f32) -> Self {
        Self { tier, score }
    }

    pub fn from_distance(distance: f32, max_distance: f32) -> Self {
        let normalized = (distance / max_distance.max(1.0)).clamp(0.0, 1.0);
        let (tier, score) = if normalized < 0.01 {
            (FrequencyTier::Critical, 1.0 - normalized * 10.0)
        } else if normalized < 0.1 {
            (FrequencyTier::High, 1.0 - normalized * 10.0)
        } else if normalized < 0.3 {
            (FrequencyTier::Medium, 1.0 - normalized * 3.0)
        } else if normalized < 0.7 {
            (FrequencyTier::Low, 1.0 - normalized)
        } else {
            (FrequencyTier::Background, 0.1)
        };
        Self { tier, score }
    }

    pub fn from_velocity(velocity: f32, threshold: f32) -> Self {
        let ratio = (velocity / threshold.max(0.001)).clamp(0.0, 1.0);
        let (tier, score) = if ratio > 0.8 {
            (FrequencyTier::Critical, ratio)
        } else if ratio > 0.4 {
            (FrequencyTier::High, ratio)
        } else if ratio > 0.1 {
            (FrequencyTier::Medium, ratio)
        } else if ratio > 0.01 {
            (FrequencyTier::Low, ratio)
        } else {
            (FrequencyTier::Background, 0.1)
        };
        Self { tier, score }
    }

    pub fn from_interaction(is_player_involved: bool, is_combat: bool) -> Self {
        if is_combat {
            Self::new(FrequencyTier::Critical, 1.0)
        } else if is_player_involved {
            Self::new(FrequencyTier::High, 0.8)
        } else {
            Self::new(FrequencyTier::Medium, 0.5)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_hz() {
        assert_eq!(FrequencyTier::Critical.hz(), 60.0);
        assert_eq!(FrequencyTier::High.hz(), 30.0);
        assert_eq!(FrequencyTier::Medium.hz(), 10.0);
        assert_eq!(FrequencyTier::Low.hz(), 1.0);
        assert_eq!(FrequencyTier::Background.hz(), 0.1);
    }

    #[test]
    fn test_tier_ordering() {
        assert!(FrequencyTier::Critical < FrequencyTier::High);
        assert!(FrequencyTier::High < FrequencyTier::Medium);
        assert!(FrequencyTier::Background > FrequencyTier::Low);
    }

    #[test]
    fn test_interval_ticks() {
        assert_eq!(FrequencyTier::Critical.interval_ticks(), 1);
        assert_eq!(FrequencyTier::Background.interval_ticks(), 600);
    }

    #[test]
    fn test_urgency_distance_near() {
        let u = Urgency::from_distance(0.5, 100.0);
        assert_eq!(u.tier, FrequencyTier::Critical);
        assert!(u.score > 0.9);
    }

    #[test]
    fn test_urgency_distance_far() {
        let u = Urgency::from_distance(80.0, 100.0);
        assert_eq!(u.tier, FrequencyTier::Background);
    }

    #[test]
    fn test_urgency_velocity_high() {
        let u = Urgency::from_velocity(100.0, 100.0);
        assert_eq!(u.tier, FrequencyTier::Critical);
    }

    #[test]
    fn test_urgency_velocity_static() {
        let u = Urgency::from_velocity(0.0, 100.0);
        assert_eq!(u.tier, FrequencyTier::Background);
    }

    #[test]
    fn test_urgency_combat() {
        let u = Urgency::from_interaction(false, true);
        assert_eq!(u.tier, FrequencyTier::Critical);
    }
}
