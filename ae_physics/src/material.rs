use serde::{Deserialize, Serialize};

use crate::fixed_point::{FixedPoint, FixedVec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MaterialCategory {
    Metal,
    Wood,
    Stone,
    Concrete,
    Glass,
    Plastic,
    Organic,
    Fabric,
    Rubber,
    Ceramic,
    Composite,
    Liquid,
    Gas,
    Energy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MaterialProperties {
    pub density: FixedPoint,
    pub hardness: FixedPoint,
    pub toughness: FixedPoint,
    pub elasticity: FixedPoint,
    pub friction: FixedPoint,
    pub restitution: FixedPoint,
    pub tensile_strength: FixedPoint,
    pub compressive_strength: FixedPoint,
    pub shear_strength: FixedPoint,
    pub melting_point: FixedPoint,
    pub thermal_conductivity: FixedPoint,
    pub electrical_conductivity: FixedPoint,
    pub radiation_resistance: FixedPoint,
    pub corrosion_resistance: FixedPoint,
    pub flammability: FixedPoint,
    pub category: MaterialCategory,
}

impl Default for MaterialProperties {
    fn default() -> Self {
        Self {
            density: FixedPoint::from_f32(1000.0),
            hardness: FixedPoint::from_f32(50.0),
            toughness: FixedPoint::from_f32(50.0),
            elasticity: FixedPoint::from_f32(0.5),
            friction: FixedPoint::from_f32(0.5),
            restitution: FixedPoint::from_f32(0.3),
            tensile_strength: FixedPoint::from_f32(100.0),
            compressive_strength: FixedPoint::from_f32(100.0),
            shear_strength: FixedPoint::from_f32(50.0),
            melting_point: FixedPoint::from_f32(1000.0),
            thermal_conductivity: FixedPoint::from_f32(1.0),
            electrical_conductivity: FixedPoint::ZERO,
            radiation_resistance: FixedPoint::from_f32(0.5),
            corrosion_resistance: FixedPoint::from_f32(0.5),
            flammability: FixedPoint::ZERO,
            category: MaterialCategory::Organic,
        }
    }
}

impl MaterialProperties {
    pub fn steel() -> Self {
        Self {
            density: FixedPoint::from_f32(7850.0),
            hardness: FixedPoint::from_f32(85.0),
            toughness: FixedPoint::from_f32(90.0),
            elasticity: FixedPoint::from_f32(0.3),
            friction: FixedPoint::from_f32(0.6),
            restitution: FixedPoint::from_f32(0.1),
            tensile_strength: FixedPoint::from_f32(500.0),
            compressive_strength: FixedPoint::from_f32(500.0),
            shear_strength: FixedPoint::from_f32(300.0),
            melting_point: FixedPoint::from_f32(1500.0),
            thermal_conductivity: FixedPoint::from_f32(50.0),
            electrical_conductivity: FixedPoint::ONE,
            radiation_resistance: FixedPoint::from_f32(0.8),
            corrosion_resistance: FixedPoint::from_f32(0.4),
            flammability: FixedPoint::ZERO,
            category: MaterialCategory::Metal,
        }
    }

    pub fn wood() -> Self {
        Self {
            density: FixedPoint::from_f32(600.0),
            hardness: FixedPoint::from_f32(30.0),
            toughness: FixedPoint::from_f32(40.0),
            elasticity: FixedPoint::from_f32(0.7),
            friction: FixedPoint::from_f32(0.7),
            restitution: FixedPoint::from_f32(0.2),
            tensile_strength: FixedPoint::from_f32(80.0),
            compressive_strength: FixedPoint::from_f32(40.0),
            shear_strength: FixedPoint::from_f32(20.0),
            melting_point: FixedPoint::from_f32(300.0),
            thermal_conductivity: FixedPoint::from_f32(0.15),
            electrical_conductivity: FixedPoint::ZERO,
            radiation_resistance: FixedPoint::from_f32(0.2),
            corrosion_resistance: FixedPoint::from_f32(0.2),
            flammability: FixedPoint::from_f32(0.8),
            category: MaterialCategory::Wood,
        }
    }

    pub fn stone() -> Self {
        Self {
            density: FixedPoint::from_f32(2700.0),
            hardness: FixedPoint::from_f32(90.0),
            toughness: FixedPoint::from_f32(30.0),
            elasticity: FixedPoint::from_f32(0.1),
            friction: FixedPoint::from_f32(0.8),
            restitution: FixedPoint::from_f32(0.05),
            tensile_strength: FixedPoint::from_f32(30.0),
            compressive_strength: FixedPoint::from_f32(200.0),
            shear_strength: FixedPoint::from_f32(40.0),
            melting_point: FixedPoint::from_f32(1200.0),
            thermal_conductivity: FixedPoint::from_f32(2.0),
            electrical_conductivity: FixedPoint::ZERO,
            radiation_resistance: FixedPoint::from_f32(0.9),
            corrosion_resistance: FixedPoint::from_f32(0.9),
            flammability: FixedPoint::ZERO,
            category: MaterialCategory::Stone,
        }
    }

    pub fn concrete() -> Self {
        Self {
            density: FixedPoint::from_f32(2400.0),
            hardness: FixedPoint::from_f32(80.0),
            toughness: FixedPoint::from_f32(25.0),
            elasticity: FixedPoint::from_f32(0.15),
            friction: FixedPoint::from_f32(0.7),
            restitution: FixedPoint::from_f32(0.05),
            tensile_strength: FixedPoint::from_f32(20.0),
            compressive_strength: FixedPoint::from_f32(250.0),
            shear_strength: FixedPoint::from_f32(35.0),
            melting_point: FixedPoint::from_f32(1400.0),
            thermal_conductivity: FixedPoint::from_f32(1.5),
            electrical_conductivity: FixedPoint::ZERO,
            radiation_resistance: FixedPoint::from_f32(0.85),
            corrosion_resistance: FixedPoint::from_f32(0.75),
            flammability: FixedPoint::ZERO,
            category: MaterialCategory::Concrete,
        }
    }

    pub fn glass() -> Self {
        Self {
            density: FixedPoint::from_f32(2500.0),
            hardness: FixedPoint::from_f32(70.0),
            toughness: FixedPoint::from_f32(5.0),
            elasticity: FixedPoint::from_f32(0.05),
            friction: FixedPoint::from_f32(0.4),
            restitution: FixedPoint::from_f32(0.1),
            tensile_strength: FixedPoint::from_f32(30.0),
            compressive_strength: FixedPoint::from_f32(100.0),
            shear_strength: FixedPoint::from_f32(15.0),
            melting_point: FixedPoint::from_f32(800.0),
            thermal_conductivity: FixedPoint::from_f32(1.0),
            electrical_conductivity: FixedPoint::ZERO,
            radiation_resistance: FixedPoint::from_f32(0.3),
            corrosion_resistance: FixedPoint::from_f32(0.95),
            flammability: FixedPoint::ZERO,
            category: MaterialCategory::Glass,
        }
    }

    pub fn flesh() -> Self {
        Self {
            density: FixedPoint::from_f32(1050.0),
            hardness: FixedPoint::from_f32(5.0),
            toughness: FixedPoint::from_f32(15.0),
            elasticity: FixedPoint::from_f32(0.8),
            friction: FixedPoint::from_f32(0.3),
            restitution: FixedPoint::from_f32(0.4),
            tensile_strength: FixedPoint::from_f32(5.0),
            compressive_strength: FixedPoint::from_f32(10.0),
            shear_strength: FixedPoint::from_f32(3.0),
            melting_point: FixedPoint::from_f32(60.0),
            thermal_conductivity: FixedPoint::from_f32(0.5),
            electrical_conductivity: FixedPoint::from_f32(0.3),
            radiation_resistance: FixedPoint::from_f32(0.1),
            corrosion_resistance: FixedPoint::ZERO,
            flammability: FixedPoint::from_f32(0.3),
            category: MaterialCategory::Organic,
        }
    }

    pub fn scrap_metal() -> Self {
        Self {
            density: FixedPoint::from_f32(5000.0),
            hardness: FixedPoint::from_f32(60.0),
            toughness: FixedPoint::from_f32(70.0),
            elasticity: FixedPoint::from_f32(0.35),
            friction: FixedPoint::from_f32(0.55),
            restitution: FixedPoint::from_f32(0.15),
            tensile_strength: FixedPoint::from_f32(300.0),
            compressive_strength: FixedPoint::from_f32(350.0),
            shear_strength: FixedPoint::from_f32(200.0),
            melting_point: FixedPoint::from_f32(1300.0),
            thermal_conductivity: FixedPoint::from_f32(40.0),
            electrical_conductivity: FixedPoint::from_f32(0.8),
            radiation_resistance: FixedPoint::from_f32(0.7),
            corrosion_resistance: FixedPoint::from_f32(0.3),
            flammability: FixedPoint::ZERO,
            category: MaterialCategory::Metal,
        }
    }

    pub fn radiation_glass() -> Self {
        Self {
            density: FixedPoint::from_f32(3500.0),
            hardness: FixedPoint::from_f32(75.0),
            toughness: FixedPoint::from_f32(10.0),
            elasticity: FixedPoint::from_f32(0.08),
            friction: FixedPoint::from_f32(0.35),
            restitution: FixedPoint::from_f32(0.08),
            tensile_strength: FixedPoint::from_f32(40.0),
            compressive_strength: FixedPoint::from_f32(120.0),
            shear_strength: FixedPoint::from_f32(20.0),
            melting_point: FixedPoint::from_f32(950.0),
            thermal_conductivity: FixedPoint::from_f32(0.8),
            electrical_conductivity: FixedPoint::ZERO,
            radiation_resistance: FixedPoint::from_f32(0.99),
            corrosion_resistance: FixedPoint::from_f32(0.98),
            flammability: FixedPoint::ZERO,
            category: MaterialCategory::Glass,
        }
    }

    pub fn rusted_steel() -> Self {
        Self {
            density: FixedPoint::from_f32(7200.0),
            hardness: FixedPoint::from_f32(70.0),
            toughness: FixedPoint::from_f32(75.0),
            elasticity: FixedPoint::from_f32(0.32),
            friction: FixedPoint::from_f32(0.65),
            restitution: FixedPoint::from_f32(0.12),
            tensile_strength: FixedPoint::from_f32(350.0),
            compressive_strength: FixedPoint::from_f32(400.0),
            shear_strength: FixedPoint::from_f32(220.0),
            melting_point: FixedPoint::from_f32(1400.0),
            thermal_conductivity: FixedPoint::from_f32(35.0),
            electrical_conductivity: FixedPoint::from_f32(0.5),
            radiation_resistance: FixedPoint::from_f32(0.75),
            corrosion_resistance: FixedPoint::from_f32(0.15),
            flammability: FixedPoint::ZERO,
            category: MaterialCategory::Metal,
        }
    }

    pub fn irradiated_concrete() -> Self {
        Self {
            density: FixedPoint::from_f32(2600.0),
            hardness: FixedPoint::from_f32(75.0),
            toughness: FixedPoint::from_f32(20.0),
            elasticity: FixedPoint::from_f32(0.12),
            friction: FixedPoint::from_f32(0.65),
            restitution: FixedPoint::from_f32(0.03),
            tensile_strength: FixedPoint::from_f32(15.0),
            compressive_strength: FixedPoint::from_f32(180.0),
            shear_strength: FixedPoint::from_f32(25.0),
            melting_point: FixedPoint::from_f32(1300.0),
            thermal_conductivity: FixedPoint::from_f32(1.2),
            electrical_conductivity: FixedPoint::ZERO,
            radiation_resistance: FixedPoint::from_f32(0.6),
            corrosion_resistance: FixedPoint::from_f32(0.5),
            flammability: FixedPoint::ZERO,
            category: MaterialCategory::Concrete,
        }
    }

    pub fn bone() -> Self {
        Self {
            density: FixedPoint::from_f32(1900.0),
            hardness: FixedPoint::from_f32(60.0),
            toughness: FixedPoint::from_f32(35.0),
            elasticity: FixedPoint::from_f32(0.25),
            friction: FixedPoint::from_f32(0.4),
            restitution: FixedPoint::from_f32(0.15),
            tensile_strength: FixedPoint::from_f32(120.0),
            compressive_strength: FixedPoint::from_f32(170.0),
            shear_strength: FixedPoint::from_f32(50.0),
            melting_point: FixedPoint::from_f32(200.0),
            thermal_conductivity: FixedPoint::from_f32(0.4),
            electrical_conductivity: FixedPoint::ZERO,
            radiation_resistance: FixedPoint::from_f32(0.3),
            corrosion_resistance: FixedPoint::from_f32(0.2),
            flammability: FixedPoint::from_f32(0.4),
            category: MaterialCategory::Organic,
        }
    }

    pub fn chitin() -> Self {
        Self {
            density: FixedPoint::from_f32(1400.0),
            hardness: FixedPoint::from_f32(55.0),
            toughness: FixedPoint::from_f32(45.0),
            elasticity: FixedPoint::from_f32(0.35),
            friction: FixedPoint::from_f32(0.5),
            restitution: FixedPoint::from_f32(0.2),
            tensile_strength: FixedPoint::from_f32(80.0),
            compressive_strength: FixedPoint::from_f32(100.0),
            shear_strength: FixedPoint::from_f32(40.0),
            melting_point: FixedPoint::from_f32(250.0),
            thermal_conductivity: FixedPoint::from_f32(0.3),
            electrical_conductivity: FixedPoint::ZERO,
            radiation_resistance: FixedPoint::from_f32(0.4),
            corrosion_resistance: FixedPoint::from_f32(0.5),
            flammability: FixedPoint::from_f32(0.5),
            category: MaterialCategory::Organic,
        }
    }

    pub fn mutant_flesh() -> Self {
        Self {
            density: FixedPoint::from_f32(1200.0),
            hardness: FixedPoint::from_f32(8.0),
            toughness: FixedPoint::from_f32(25.0),
            elasticity: FixedPoint::from_f32(0.7),
            friction: FixedPoint::from_f32(0.25),
            restitution: FixedPoint::from_f32(0.35),
            tensile_strength: FixedPoint::from_f32(8.0),
            compressive_strength: FixedPoint::from_f32(15.0),
            shear_strength: FixedPoint::from_f32(5.0),
            melting_point: FixedPoint::from_f32(65.0),
            thermal_conductivity: FixedPoint::from_f32(0.45),
            electrical_conductivity: FixedPoint::from_f32(0.25),
            radiation_resistance: FixedPoint::from_f32(0.3),
            corrosion_resistance: FixedPoint::from_f32(0.05),
            flammability: FixedPoint::from_f32(0.35),
            category: MaterialCategory::Organic,
        }
    }

    pub fn rubber() -> Self {
        Self {
            density: FixedPoint::from_f32(1100.0),
            hardness: FixedPoint::from_f32(20.0),
            toughness: FixedPoint::from_f32(60.0),
            elasticity: FixedPoint::from_f32(0.95),
            friction: FixedPoint::from_f32(0.9),
            restitution: FixedPoint::from_f32(0.7),
            tensile_strength: FixedPoint::from_f32(25.0),
            compressive_strength: FixedPoint::from_f32(30.0),
            shear_strength: FixedPoint::from_f32(15.0),
            melting_point: FixedPoint::from_f32(180.0),
            thermal_conductivity: FixedPoint::from_f32(0.15),
            electrical_conductivity: FixedPoint::ZERO,
            radiation_resistance: FixedPoint::from_f32(0.2),
            corrosion_resistance: FixedPoint::from_f32(0.6),
            flammability: FixedPoint::from_f32(0.7),
            category: MaterialCategory::Rubber,
        }
    }

    pub fn ceramic_armor() -> Self {
        Self {
            density: FixedPoint::from_f32(3800.0),
            hardness: FixedPoint::from_f32(95.0),
            toughness: FixedPoint::from_f32(15.0),
            elasticity: FixedPoint::from_f32(0.05),
            friction: FixedPoint::from_f32(0.3),
            restitution: FixedPoint::from_f32(0.02),
            tensile_strength: FixedPoint::from_f32(50.0),
            compressive_strength: FixedPoint::from_f32(500.0),
            shear_strength: FixedPoint::from_f32(30.0),
            melting_point: FixedPoint::from_f32(2000.0),
            thermal_conductivity: FixedPoint::from_f32(30.0),
            electrical_conductivity: FixedPoint::ZERO,
            radiation_resistance: FixedPoint::from_f32(0.85),
            corrosion_resistance: FixedPoint::from_f32(0.95),
            flammability: FixedPoint::ZERO,
            category: MaterialCategory::Ceramic,
        }
    }

    pub fn composite_armor() -> Self {
        Self {
            density: FixedPoint::from_f32(2500.0),
            hardness: FixedPoint::from_f32(85.0),
            toughness: FixedPoint::from_f32(50.0),
            elasticity: FixedPoint::from_f32(0.2),
            friction: FixedPoint::from_f32(0.4),
            restitution: FixedPoint::from_f32(0.05),
            tensile_strength: FixedPoint::from_f32(200.0),
            compressive_strength: FixedPoint::from_f32(300.0),
            shear_strength: FixedPoint::from_f32(100.0),
            melting_point: FixedPoint::from_f32(1600.0),
            thermal_conductivity: FixedPoint::from_f32(5.0),
            electrical_conductivity: FixedPoint::ZERO,
            radiation_resistance: FixedPoint::from_f32(0.9),
            corrosion_resistance: FixedPoint::from_f32(0.8),
            flammability: FixedPoint::from_f32(0.1),
            category: MaterialCategory::Composite,
        }
    }

    pub fn soil() -> Self {
        Self {
            density: FixedPoint::from_f32(1600.0),
            hardness: FixedPoint::from_f32(10.0),
            toughness: FixedPoint::from_f32(5.0),
            elasticity: FixedPoint::from_f32(0.3),
            friction: FixedPoint::from_f32(0.8),
            restitution: FixedPoint::from_f32(0.01),
            tensile_strength: FixedPoint::from_f32(2.0),
            compressive_strength: FixedPoint::from_f32(15.0),
            shear_strength: FixedPoint::from_f32(5.0),
            melting_point: FixedPoint::from_f32(1200.0),
            thermal_conductivity: FixedPoint::from_f32(0.8),
            electrical_conductivity: FixedPoint::ZERO,
            radiation_resistance: FixedPoint::from_f32(0.5),
            corrosion_resistance: FixedPoint::from_f32(0.8),
            flammability: FixedPoint::ZERO,
            category: MaterialCategory::Stone,
        }
    }

    pub fn radioactive_slag() -> Self {
        Self {
            density: FixedPoint::from_f32(3200.0),
            hardness: FixedPoint::from_f32(65.0),
            toughness: FixedPoint::from_f32(10.0),
            elasticity: FixedPoint::from_f32(0.08),
            friction: FixedPoint::from_f32(0.5),
            restitution: FixedPoint::from_f32(0.02),
            tensile_strength: FixedPoint::from_f32(10.0),
            compressive_strength: FixedPoint::from_f32(80.0),
            shear_strength: FixedPoint::from_f32(15.0),
            melting_point: FixedPoint::from_f32(900.0),
            thermal_conductivity: FixedPoint::from_f32(1.5),
            electrical_conductivity: FixedPoint::from_f32(0.1),
            radiation_resistance: FixedPoint::from_f32(0.1),
            corrosion_resistance: FixedPoint::from_f32(0.3),
            flammability: FixedPoint::ZERO,
            category: MaterialCategory::Composite,
        }
    }

    pub fn damage_at_point(
        &self,
        impact_force: FixedPoint,
        impact_velocity: FixedVec3,
    ) -> FixedPoint {
        let kinetic_energy =
            FixedPoint::from_f32(0.5) * self.density * impact_velocity.length_squared();
        let strain_rate = impact_velocity.length() / FixedPoint::from_f32(0.01);
        let strain_rate_ln =
            if strain_rate > FixedPoint::ONE { strain_rate.ln() } else { FixedPoint::ZERO };
        let dynamic_strength =
            self.tensile_strength * (FixedPoint::ONE + FixedPoint::from_f32(0.1) * strain_rate_ln);
        let damage = (kinetic_energy / (dynamic_strength * self.toughness)).min(FixedPoint::ONE);
        let force = if impact_force > FixedPoint::ZERO { impact_force } else { FixedPoint::ZERO };
        damage * force
    }

    pub fn thermal_damage(&self, temperature: FixedPoint, duration: FixedPoint) -> FixedPoint {
        let half_melt = self.melting_point * FixedPoint::from_f32(0.5);
        if temperature < half_melt {
            return FixedPoint::ZERO;
        }
        let temp_ratio = (temperature / self.melting_point).min(FixedPoint::from_f32(2.0));
        let conductivity_factor = self.thermal_conductivity / FixedPoint::from_f32(50.0);
        (temp_ratio * conductivity_factor * duration * FixedPoint::from_f32(0.1))
            .min(FixedPoint::ONE)
    }

    pub fn radiation_damage(
        &self,
        rads_per_second: FixedPoint,
        duration: FixedPoint,
    ) -> FixedPoint {
        let total_rads = rads_per_second * duration;
        let resistance_factor = FixedPoint::ONE - self.radiation_resistance;
        (total_rads * resistance_factor * FixedPoint::from_f32(0.001)).min(FixedPoint::ONE)
    }

    pub fn corrosion_damage(&self, acidity: FixedPoint, duration: FixedPoint) -> FixedPoint {
        let resistance_factor = FixedPoint::ONE - self.corrosion_resistance;
        (acidity * resistance_factor * duration * FixedPoint::from_f32(0.01)).min(FixedPoint::ONE)
    }
}
