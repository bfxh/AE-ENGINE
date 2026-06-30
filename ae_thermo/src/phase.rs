use serde::{Deserialize, Serialize};

use crate::properties::ThermalProperties;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Solid,
    Liquid,
    Gas,
    Plasma,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhaseTransition {
    None,
    Melting,
    Solidification,
    Vaporization,
    Condensation,
    Sublimation,
    Deposition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseState {
    pub current_phase: Phase,
    pub temperature: f32,
    pub latent_heat_absorbed: f32,
    pub transition: PhaseTransition,
    pub mass: f32,
    pub props: ThermalProperties,
}

impl PhaseState {
    pub fn new(mass: f32, temperature: f32, props: ThermalProperties) -> Self {
        let phase = Self::determine_phase(temperature, &props);
        Self {
            current_phase: phase,
            temperature,
            latent_heat_absorbed: 0.0,
            transition: PhaseTransition::None,
            mass,
            props,
        }
    }

    fn determine_phase(temp: f32, props: &ThermalProperties) -> Phase {
        if temp >= props.boiling_point
            && props.boiling_point > 0.0
            && props.latent_heat_vaporization > 0.0
        {
            Phase::Gas
        } else if temp >= props.melting_point
            && props.melting_point < f32::MAX
            && props.latent_heat_fusion > 0.0
        {
            Phase::Liquid
        } else {
            Phase::Solid
        }
    }

    pub fn total_latent_heat_fusion(&self) -> f32 {
        self.mass * self.props.latent_heat_fusion
    }

    pub fn total_latent_heat_vaporization(&self) -> f32 {
        self.mass * self.props.latent_heat_vaporization
    }

    pub fn heat_capacity(&self) -> f32 {
        self.props.heat_capacity(self.mass / self.props.density)
    }

    pub fn transition_progress(&self) -> f32 {
        match self.transition {
            PhaseTransition::None => 0.0,
            PhaseTransition::Melting | PhaseTransition::Solidification => {
                let total = self.total_latent_heat_fusion();
                if total > 0.0 { (self.latent_heat_absorbed / total).clamp(0.0, 1.0) } else { 1.0 }
            },
            PhaseTransition::Vaporization | PhaseTransition::Condensation => {
                let total = self.total_latent_heat_vaporization();
                if total > 0.0 { (self.latent_heat_absorbed / total).clamp(0.0, 1.0) } else { 1.0 }
            },
            PhaseTransition::Sublimation | PhaseTransition::Deposition => {
                let total = self.total_latent_heat_fusion() + self.total_latent_heat_vaporization();
                if total > 0.0 { (self.latent_heat_absorbed / total).clamp(0.0, 1.0) } else { 1.0 }
            },
        }
    }

    pub fn is_transitioning(&self) -> bool {
        self.transition != PhaseTransition::None
    }

    fn check_transition(&mut self) {
        if self.transition != PhaseTransition::None {
            return;
        }

        match self.current_phase {
            Phase::Solid => {
                if self.temperature >= self.props.melting_point
                    && self.props.melting_point < f32::MAX
                    && self.props.latent_heat_fusion > 0.0
                {
                    self.transition = PhaseTransition::Melting;
                    self.temperature = self.props.melting_point;
                    self.latent_heat_absorbed = 0.0;
                } else if self.props.boiling_point > 0.0
                    && self.temperature >= self.props.boiling_point
                    && self.props.latent_heat_vaporization > 0.0
                    && self.props.melting_point >= f32::MAX
                {
                    self.transition = PhaseTransition::Sublimation;
                    self.temperature = self.props.boiling_point;
                    self.latent_heat_absorbed = 0.0;
                }
            },
            Phase::Liquid => {
                if self.temperature >= self.props.boiling_point
                    && self.props.boiling_point > 0.0
                    && self.props.latent_heat_vaporization > 0.0
                {
                    self.transition = PhaseTransition::Vaporization;
                    self.temperature = self.props.boiling_point;
                    self.latent_heat_absorbed = 0.0;
                } else if self.temperature < self.props.melting_point
                    && self.props.latent_heat_fusion > 0.0
                {
                    self.transition = PhaseTransition::Solidification;
                    self.temperature = self.props.melting_point;
                    self.latent_heat_absorbed = 0.0;
                }
            },
            Phase::Gas => {
                if self.temperature < self.props.boiling_point
                    && self.props.boiling_point > 0.0
                    && self.props.latent_heat_vaporization > 0.0
                {
                    self.transition = PhaseTransition::Condensation;
                    self.temperature = self.props.boiling_point;
                    self.latent_heat_absorbed = 0.0;
                } else if self.temperature < self.props.melting_point
                    && self.props.melting_point < f32::MAX
                    && self.props.latent_heat_fusion > 0.0
                    && self.props.boiling_point <= 0.0
                {
                    self.transition = PhaseTransition::Deposition;
                    self.temperature = self.props.melting_point;
                    self.latent_heat_absorbed = 0.0;
                }
            },
            Phase::Plasma => {},
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseSolver {
    pub time_step: f32,
}

impl Default for PhaseSolver {
    fn default() -> Self {
        Self { time_step: 1.0 / 60.0 }
    }
}

impl PhaseSolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_heat(&self, state: &mut PhaseState, delta_temp: f32) -> f32 {
        let heat_capacity = state.heat_capacity();
        let energy = delta_temp * heat_capacity;

        if state.is_transitioning() {
            let remaining = self.advance_transition_energy(state, energy);
            if remaining.abs() < 1e-9 {
                return 0.0;
            }
            return remaining / heat_capacity;
        }

        let new_temp = state.temperature + delta_temp;
        state.temperature = new_temp;
        state.check_transition();

        if state.is_transitioning() {
            let heat_capacity = state.heat_capacity();
            let excess_energy = (new_temp - state.temperature) * heat_capacity;
            let remaining = self.advance_transition_energy(state, excess_energy);
            if remaining.abs() < 1e-9 {
                return 0.0;
            }
            return remaining / heat_capacity;
        }

        delta_temp
    }

    fn advance_transition_energy(&self, state: &mut PhaseState, energy: f32) -> f32 {
        let total_needed = match state.transition {
            PhaseTransition::Melting | PhaseTransition::Solidification => {
                state.total_latent_heat_fusion()
            },
            PhaseTransition::Vaporization | PhaseTransition::Condensation => {
                state.total_latent_heat_vaporization()
            },
            PhaseTransition::Sublimation | PhaseTransition::Deposition => {
                state.total_latent_heat_fusion() + state.total_latent_heat_vaporization()
            },
            PhaseTransition::None => return energy,
        };

        if total_needed <= 0.0 {
            let old_transition = state.transition;
            state.transition = PhaseTransition::None;
            state.latent_heat_absorbed = 0.0;
            match old_transition {
                PhaseTransition::Melting => state.current_phase = Phase::Liquid,
                PhaseTransition::Solidification => state.current_phase = Phase::Solid,
                PhaseTransition::Vaporization => state.current_phase = Phase::Gas,
                PhaseTransition::Condensation => state.current_phase = Phase::Liquid,
                PhaseTransition::Sublimation => state.current_phase = Phase::Gas,
                PhaseTransition::Deposition => state.current_phase = Phase::Solid,
                _ => {},
            }
            return energy;
        }

        let energy_abs = energy.abs();
        let remaining = total_needed - state.latent_heat_absorbed;

        if energy_abs >= remaining {
            let sign = energy.signum();
            state.latent_heat_absorbed = total_needed;
            let excess = energy - remaining * sign;

            let old_transition = state.transition;
            match old_transition {
                PhaseTransition::Melting => {
                    state.current_phase = Phase::Liquid;
                    state.temperature = state.props.melting_point;
                },
                PhaseTransition::Solidification => {
                    state.current_phase = Phase::Solid;
                    state.temperature = state.props.melting_point;
                },
                PhaseTransition::Vaporization => {
                    state.current_phase = Phase::Gas;
                    state.temperature = state.props.boiling_point;
                },
                PhaseTransition::Condensation => {
                    state.current_phase = Phase::Liquid;
                    state.temperature = state.props.boiling_point;
                },
                PhaseTransition::Sublimation => {
                    state.current_phase = Phase::Gas;
                    state.temperature = state.props.boiling_point.max(state.props.melting_point);
                },
                PhaseTransition::Deposition => {
                    state.current_phase = Phase::Solid;
                    state.temperature = state.props.melting_point.min(state.props.boiling_point);
                },
                PhaseTransition::None => {},
            }
            state.transition = PhaseTransition::None;
            state.latent_heat_absorbed = 0.0;

            state.check_transition();
            if state.is_transitioning() {
                return self.advance_transition_energy(state, excess);
            }

            excess
        } else {
            state.latent_heat_absorbed += energy_abs;
            0.0
        }
    }

    pub fn force_phase(&self, state: &mut PhaseState, target: Phase) {
        state.transition = match (state.current_phase, target) {
            (Phase::Solid, Phase::Liquid) => PhaseTransition::Melting,
            (Phase::Liquid, Phase::Solid) => PhaseTransition::Solidification,
            (Phase::Liquid, Phase::Gas) => PhaseTransition::Vaporization,
            (Phase::Gas, Phase::Liquid) => PhaseTransition::Condensation,
            (Phase::Solid, Phase::Gas) => PhaseTransition::Sublimation,
            (Phase::Gas, Phase::Solid) => PhaseTransition::Deposition,
            _ => {
                state.current_phase = target;
                return;
            },
        };
        state.latent_heat_absorbed = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::properties::{THERMAL_IRON, THERMAL_WATER, THERMAL_WOOD};

    #[test]
    fn test_water_freezes_at_273k() {
        let props = THERMAL_WATER;
        let state = PhaseState::new(1.0, 270.0, props);
        assert_eq!(state.current_phase, Phase::Solid);
    }

    #[test]
    fn test_water_liquid_at_300k() {
        let props = THERMAL_WATER;
        let state = PhaseState::new(1.0, 300.0, props);
        assert_eq!(state.current_phase, Phase::Liquid);
    }

    #[test]
    fn test_water_boils_at_400k() {
        let props = THERMAL_WATER;
        let state = PhaseState::new(1.0, 400.0, props);
        assert_eq!(state.current_phase, Phase::Gas);
    }

    #[test]
    fn test_iron_solid_at_300k() {
        let props = THERMAL_IRON;
        let state = PhaseState::new(1.0, 300.0, props);
        assert_eq!(state.current_phase, Phase::Solid);
    }

    #[test]
    fn test_latent_heat_melting_triggers_transition() {
        let solver = PhaseSolver::default();
        let props = THERMAL_WATER;
        let mut state = PhaseState::new(1.0, 270.0, props);

        let _delta = solver.apply_heat(&mut state, 10.0);
        assert!(state.temperature >= state.props.melting_point);
        assert!(state.is_transitioning() || state.current_phase == Phase::Liquid);
    }

    #[test]
    fn test_latent_heat_freezes_water() {
        let solver = PhaseSolver::default();
        let props = THERMAL_WATER;
        let mut state = PhaseState::new(1.0, 280.0, props);
        assert_eq!(state.current_phase, Phase::Liquid);

        let _delta = solver.apply_heat(&mut state, -20.0);
        assert!(state.is_transitioning() || state.current_phase == Phase::Solid);
    }

    #[test]
    fn test_full_melt_cycle() {
        let solver = PhaseSolver::default();
        let props = THERMAL_WATER;
        let mut state = PhaseState::new(1.0, 250.0, props);
        assert_eq!(state.current_phase, Phase::Solid);

        let mut max_iter = 10000;
        while state.current_phase != Phase::Gas && max_iter > 0 {
            solver.apply_heat(&mut state, 5.0);
            max_iter -= 1;
        }
        assert!(max_iter > 0, "phase transition did not complete within iteration limit");
        assert_eq!(state.current_phase, Phase::Gas);
    }

    #[test]
    fn test_wood_no_boiling() {
        let props = THERMAL_WOOD;
        let state = PhaseState::new(1.0, 600.0, props);
        assert_eq!(state.current_phase, Phase::Solid);
    }

    #[test]
    fn test_transition_progress() {
        let solver = PhaseSolver::default();
        let props = THERMAL_WATER;
        let mut state = PhaseState::new(1.0, 273.15, props);

        solver.apply_heat(&mut state, 0.1);
        if state.is_transitioning() {
            let progress = state.transition_progress();
            assert!((0.0..=1.0).contains(&progress));
        }
    }
}
