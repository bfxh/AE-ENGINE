use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solute {
    pub id: Uuid,
    pub chemical_formula: String,
    pub concentration: f32,
    pub molar_mass: f32,
    pub charge: i32,
    pub solubility: f32,
    pub max_solubility: f32,
}

impl Solute {
    pub fn new(formula: &str, concentration: f32, molar_mass: f32, charge: i32) -> Self {
        Self {
            id: Uuid::new_v4(),
            chemical_formula: formula.to_string(),
            concentration,
            molar_mass,
            charge,
            solubility: 1.0,
            max_solubility: concentration * 2.0,
        }
    }

    pub fn with_solubility(mut self, max_solubility: f32) -> Self {
        self.max_solubility = max_solubility;
        self
    }

    pub fn moles(&self, volume: f32) -> f32 {
        self.concentration * volume
    }

    pub fn mass_in_solution(&self, volume: f32) -> f32 {
        self.moles(volume) * self.molar_mass
    }

    pub fn is_saturated(&self) -> bool {
        self.concentration >= self.max_solubility
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solvent {
    pub name: String,
    pub density: f32,
    pub dielectric_constant: f32,
    pub boiling_point: f32,
    pub freezing_point: f32,
    pub viscosity: f32,
}

impl Default for Solvent {
    fn default() -> Self {
        Self {
            name: "water".to_string(),
            density: 1.0,
            dielectric_constant: 78.5,
            boiling_point: 373.15,
            freezing_point: 273.15,
            viscosity: 0.001,
        }
    }
}

impl Solvent {
    pub fn water() -> Self {
        Self::default()
    }

    pub fn ethanol() -> Self {
        Self {
            name: "ethanol".to_string(),
            density: 0.789,
            dielectric_constant: 24.3,
            boiling_point: 351.5,
            freezing_point: 159.0,
            viscosity: 0.0012,
        }
    }

    pub fn acetone() -> Self {
        Self {
            name: "acetone".to_string(),
            density: 0.784,
            dielectric_constant: 20.7,
            boiling_point: 329.2,
            freezing_point: 178.2,
            viscosity: 0.00032,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solution {
    pub solutes: Vec<Solute>,
    pub solvent: Solvent,
    pub volume: f32,
    pub temperature: f32,
    pub ph: f32,
    pub agitation: f32,
}

impl Solution {
    pub fn new(volume: f32, temperature: f32) -> Self {
        Self {
            solutes: Vec::new(),
            solvent: Solvent::default(),
            volume,
            temperature,
            ph: 7.0,
            agitation: 0.0,
        }
    }

    pub fn with_solvent(mut self, solvent: Solvent) -> Self {
        self.solvent = solvent;
        self
    }

    pub fn dissolve(&mut self, solute: Solute, amount: f32) -> bool {
        let existing_concentration = self
            .solutes
            .iter()
            .find(|s| s.chemical_formula == solute.chemical_formula)
            .map(|s| s.concentration)
            .unwrap_or(0.0);
        let new_concentration = existing_concentration + amount;
        if new_concentration > solute.max_solubility {
            return false;
        }

        if let Some(existing) =
            self.solutes.iter_mut().find(|s| s.chemical_formula == solute.chemical_formula)
        {
            existing.concentration = new_concentration;
        } else {
            let mut new_solute = solute;
            new_solute.concentration = new_concentration;
            self.solutes.push(new_solute);
        }
        true
    }

    pub fn precipitate(&mut self, solute: &Solute, amount: f32) -> f32 {
        if let Some(existing) =
            self.solutes.iter_mut().find(|s| s.chemical_formula == solute.chemical_formula)
        {
            let actual = amount.min(existing.concentration);
            existing.concentration -= actual;
            if existing.concentration <= 0.0 {
                self.solutes.retain(|s| s.chemical_formula != solute.chemical_formula);
            }
            actual
        } else {
            0.0
        }
    }

    pub fn calculate_ph(&self) -> f32 {
        let mut h_plus = 0.0f32;
        for solute in &self.solutes {
            if solute.charge > 0 {
                h_plus += solute.concentration * solute.charge.abs() as f32 * 0.1;
            } else if solute.charge < 0 {
                h_plus -= solute.concentration * solute.charge.abs() as f32 * 0.1;
            }
        }

        let effective_h = (10.0f32).powi(-self.ph as i32) + h_plus;
        if effective_h <= 0.0 {
            14.0
        } else {
            let ph = -effective_h.log10();
            ph.clamp(0.0, 14.0)
        }
    }

    pub fn calculate_ionic_strength(&self) -> f32 {
        let mut strength = 0.0f32;
        for solute in &self.solutes {
            strength += solute.concentration * (solute.charge.pow(2) as f32);
        }
        strength * 0.5
    }

    pub fn calculate_osmotic_pressure(&self) -> f32 {
        const R: f32 = 0.082057;
        let mut total_concentration = 0.0f32;
        for solute in &self.solutes {
            let vanthoff_factor = 1.0 + (solute.charge.abs() as f32 - 1.0).max(0.0) * 0.8;
            total_concentration += solute.concentration * vanthoff_factor;
        }
        total_concentration * R * self.temperature
    }

    pub fn dilute(&mut self, factor: f32) {
        if factor <= 0.0 {
            return;
        }
        self.volume *= factor;
        for solute in &mut self.solutes {
            solute.concentration /= factor;
        }
    }

    pub fn evaporate(&mut self, amount: f32) {
        let new_volume = (self.volume - amount).max(0.001);
        let concentration_factor = self.volume / new_volume;
        self.volume = new_volume;
        for solute in &mut self.solutes {
            let new_conc = solute.concentration * concentration_factor;
            if new_conc > solute.max_solubility {
                solute.concentration = solute.max_solubility;
            } else {
                solute.concentration = new_conc;
            }
        }
    }

    pub fn total_solute_mass(&self) -> f32 {
        let mut total = 0.0;
        for solute in &self.solutes {
            total += solute.mass_in_solution(self.volume);
        }
        total
    }

    pub fn solute_count(&self) -> usize {
        self.solutes.len()
    }

    pub fn get_solute(&self, formula: &str) -> Option<&Solute> {
        self.solutes.iter().find(|s| s.chemical_formula == formula)
    }

    pub fn get_solute_mut(&mut self, formula: &str) -> Option<&mut Solute> {
        self.solutes.iter_mut().find(|s| s.chemical_formula == formula)
    }

    pub fn is_saturated(&self, formula: &str) -> bool {
        self.get_solute(formula).map(|s| s.is_saturated()).unwrap_or(false)
    }

    pub fn heat(&mut self, energy: f32) {
        let heat_capacity = self.volume * self.solvent.density * 4.184;
        self.temperature += energy / heat_capacity;
    }

    pub fn cool(&mut self, energy: f32) {
        self.temperature = (self.temperature
            - energy / (self.volume * self.solvent.density * 4.184))
            .max(self.solvent.freezing_point);
    }
}

impl Default for Solution {
    fn default() -> Self {
        Self::new(1.0, 298.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dissolve_precipitate() {
        let mut solution = Solution::new(1.0, 298.0);
        let nacl = Solute::new("NaCl", 0.0, 58.44, 0).with_solubility(6.0);

        assert!(solution.dissolve(nacl.clone(), 2.0));
        assert_eq!(solution.solute_count(), 1);

        let precipitated = solution.precipitate(&nacl, 1.0);
        assert!((precipitated - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_dissolve_saturation() {
        let mut solution = Solution::new(1.0, 298.0);
        let solute = Solute::new("KCl", 0.0, 74.55, 0).with_solubility(3.0);

        assert!(solution.dissolve(solute.clone(), 2.0));
        assert!(!solution.dissolve(solute.clone(), 3.0));
    }

    #[test]
    fn test_ph_calculation() {
        let mut solution = Solution::new(1.0, 298.0);
        let acid = Solute::new("HCl", 0.1, 36.46, 1);
        solution.dissolve(acid, 0.1);

        let ph = solution.calculate_ph();
        assert!(ph < 7.0);
    }

    #[test]
    fn test_ionic_strength() {
        let mut solution = Solution::new(1.0, 298.0);
        let nacl = Solute::new("NaCl", 0.1, 58.44, 0);
        solution.dissolve(nacl, 0.1);

        let strength = solution.calculate_ionic_strength();
        assert!((strength - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_dilute() {
        let mut solution = Solution::new(1.0, 298.0);
        let solute = Solute::new("NaCl", 0.0, 58.44, 0).with_solubility(6.0);
        solution.dissolve(solute, 2.0);

        let conc_before = solution.solutes[0].concentration;
        solution.dilute(2.0);
        let conc_after = solution.solutes[0].concentration;

        assert!((conc_after - conc_before / 2.0).abs() < 0.01);
        assert!((solution.volume - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_osmotic_pressure() {
        let mut solution = Solution::new(1.0, 298.0);
        let nacl = Solute::new("NaCl", 0.0, 58.44, 0);
        solution.dissolve(nacl, 0.1);

        let pressure = solution.calculate_osmotic_pressure();
        assert!(pressure >= 0.0);
    }

    #[test]
    fn test_heat_cool() {
        let mut solution = Solution::new(1.0, 298.0);
        let initial_temp = solution.temperature;

        solution.heat(1000.0);
        assert!(solution.temperature > initial_temp);

        solution.cool(1000.0);
        assert!((solution.temperature - initial_temp).abs() < 0.5);
    }
}
