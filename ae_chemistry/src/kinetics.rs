use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ReactionOrder {
    ZeroOrder,
    FirstOrder,
    SecondOrder,
    MixedOrder { order_a: f32, order_b: f32 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RateConstant {
    pub pre_exponential: f32,
    pub activation_energy: f32,
    pub temperature: f32,
}

impl RateConstant {
    pub fn new(pre_exponential: f32, activation_energy: f32, temperature: f32) -> Self {
        Self { pre_exponential, activation_energy, temperature }
    }

    pub fn value(&self) -> f32 {
        const R: f32 = 8.314;
        if self.temperature <= 0.0 {
            return 0.0;
        }
        self.pre_exponential * (-self.activation_energy * 1000.0 / (R * self.temperature)).exp()
    }
}

impl Default for RateConstant {
    fn default() -> Self {
        Self { pre_exponential: 1.0e10, activation_energy: 50.0, temperature: 298.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KineticModel {
    pub order: ReactionOrder,
    pub rate_constant: RateConstant,
    pub catalyst_factor: f32,
    pub inhibitor_factor: f32,
    pub chain_reaction: bool,
    pub chain_branching: f32,
    pub chain_termination: f32,
}

impl KineticModel {
    pub fn new(order: ReactionOrder, rate_constant: RateConstant) -> Self {
        Self {
            order,
            rate_constant,
            catalyst_factor: 1.0,
            inhibitor_factor: 1.0,
            chain_reaction: false,
            chain_branching: 0.0,
            chain_termination: 0.0,
        }
    }

    pub fn calculate_rate(&self, concentrations: &[f32], temperature: f32) -> f32 {
        let k = self.arrhenius_rate(
            temperature,
            self.rate_constant.pre_exponential,
            self.rate_constant.activation_energy,
        );
        let k_eff = k * self.catalyst_factor * self.inhibitor_factor;

        match self.order {
            ReactionOrder::ZeroOrder => k_eff,
            ReactionOrder::FirstOrder => {
                if concentrations.is_empty() {
                    k_eff
                } else {
                    k_eff * concentrations[0]
                }
            },
            ReactionOrder::SecondOrder => {
                if concentrations.len() < 2 {
                    k_eff * concentrations.first().copied().unwrap_or(1.0).powi(2)
                } else {
                    k_eff * concentrations[0] * concentrations[1]
                }
            },
            ReactionOrder::MixedOrder { order_a, order_b } => {
                let conc_a = concentrations.first().copied().unwrap_or(1.0);
                let conc_b = concentrations.get(1).copied().unwrap_or(1.0);
                k_eff * conc_a.powf(order_a) * conc_b.powf(order_b)
            },
        }
    }

    pub fn arrhenius_rate(
        &self,
        temperature: f32,
        pre_exponential: f32,
        activation_energy: f32,
    ) -> f32 {
        const R: f32 = 8.314;
        if temperature <= 0.0 {
            return 0.0;
        }
        pre_exponential * (-activation_energy * 1000.0 / (R * temperature)).exp()
    }

    pub fn progress_to_completion(&self, rate: f32, dt: f32, initial_concentration: f32) -> f32 {
        let consumed = match self.order {
            ReactionOrder::ZeroOrder => rate * dt,
            ReactionOrder::FirstOrder => initial_concentration * (1.0 - (-rate * dt).exp()),
            ReactionOrder::SecondOrder => {
                let denom = 1.0 + rate * dt * initial_concentration;
                if denom.abs() < 1e-10 {
                    initial_concentration
                } else {
                    initial_concentration - initial_concentration / denom
                }
            },
            ReactionOrder::MixedOrder { .. } => rate * dt,
        };

        consumed.min(initial_concentration).max(0.0)
    }

    pub fn half_life(&self, initial_concentration: f32) -> f32 {
        let k = self.rate_constant.value() * self.catalyst_factor * self.inhibitor_factor;
        match self.order {
            ReactionOrder::ZeroOrder => initial_concentration / (2.0 * k),
            ReactionOrder::FirstOrder => (2.0f32).ln() / k,
            ReactionOrder::SecondOrder => 1.0 / (k * initial_concentration),
            ReactionOrder::MixedOrder { .. } => (2.0f32).ln() / k,
        }
    }

    pub fn with_catalyst(mut self, factor: f32) -> Self {
        self.catalyst_factor = factor;
        self
    }

    pub fn with_inhibitor(mut self, factor: f32) -> Self {
        self.inhibitor_factor = factor;
        self
    }

    pub fn with_chain_reaction(mut self, branching: f32, termination: f32) -> Self {
        self.chain_reaction = true;
        self.chain_branching = branching;
        self.chain_termination = termination;
        self
    }

    pub fn chain_propagation_rate(
        &self,
        radical_concentration: f32,
        monomer_concentration: f32,
    ) -> f32 {
        if !self.chain_reaction {
            return 0.0;
        }
        let k_p = self.rate_constant.value() * self.catalyst_factor;
        let net_growth = k_p
            * radical_concentration
            * monomer_concentration
            * (self.chain_branching - self.chain_termination);
        net_growth.max(0.0)
    }

    pub fn estimate_reaction_time(
        &self,
        initial_concentration: f32,
        target_conversion: f32,
    ) -> f32 {
        let k = self.rate_constant.value() * self.catalyst_factor * self.inhibitor_factor;
        match self.order {
            ReactionOrder::ZeroOrder => (initial_concentration * target_conversion) / k,
            ReactionOrder::FirstOrder => -(1.0 - target_conversion).ln() / k,
            ReactionOrder::SecondOrder => {
                target_conversion / (k * initial_concentration * (1.0 - target_conversion))
            },
            ReactionOrder::MixedOrder { .. } => -(1.0 - target_conversion).ln() / k,
        }
    }
}

impl Default for KineticModel {
    fn default() -> Self {
        Self {
            order: ReactionOrder::FirstOrder,
            rate_constant: RateConstant::default(),
            catalyst_factor: 1.0,
            inhibitor_factor: 1.0,
            chain_reaction: false,
            chain_branching: 0.0,
            chain_termination: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_order_rate() {
        let model =
            KineticModel::new(ReactionOrder::ZeroOrder, RateConstant::new(1.0e10, 50.0, 298.0));
        let rate = model.calculate_rate(&[1.0], 298.0);
        assert!(rate > 0.0);
    }

    #[test]
    fn test_first_order_rate() {
        let model =
            KineticModel::new(ReactionOrder::FirstOrder, RateConstant::new(1.0e10, 50.0, 298.0));
        let rate = model.calculate_rate(&[0.5], 298.0);
        assert!(rate > 0.0);
    }

    #[test]
    fn test_arrhenius_temperature_dependence() {
        let model =
            KineticModel::new(ReactionOrder::FirstOrder, RateConstant::new(1.0e10, 50.0, 298.0));
        let rate_low = model.calculate_rate(&[1.0], 250.0);
        let rate_high = model.calculate_rate(&[1.0], 350.0);
        assert!(rate_high > rate_low);
    }

    #[test]
    fn test_progress_to_completion() {
        let model =
            KineticModel::new(ReactionOrder::FirstOrder, RateConstant::new(1.0e10, 50.0, 298.0));
        let rate = model.calculate_rate(&[1.0], 298.0);
        let consumed = model.progress_to_completion(rate, 0.1, 1.0);
        assert!(consumed > 0.0);
        assert!(consumed <= 1.0);
    }

    #[test]
    fn test_half_life() {
        let model =
            KineticModel::new(ReactionOrder::FirstOrder, RateConstant::new(1.0e10, 50.0, 298.0));
        let hl = model.half_life(1.0);
        assert!(hl > 0.0);
    }

    #[test]
    fn test_catalyst_factor() {
        let model =
            KineticModel::new(ReactionOrder::FirstOrder, RateConstant::new(1.0e10, 50.0, 298.0))
                .with_catalyst(10.0);

        let rate = model.calculate_rate(&[1.0], 298.0);
        assert!(rate > 0.0);
        assert_eq!(model.catalyst_factor, 10.0);
    }

    #[test]
    fn test_chain_reaction() {
        let model =
            KineticModel::new(ReactionOrder::FirstOrder, RateConstant::new(1.0e10, 50.0, 298.0))
                .with_chain_reaction(1.5, 0.5);

        let rate = model.chain_propagation_rate(0.1, 1.0);
        assert!(rate > 0.0);
    }
}
