use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RockType {
    Granite,
    Basalt,
    Limestone,
    Sandstone,
    Shale,
    Marble,
    Quartzite,
    Slate,
    Gneiss,
    Schist,
    Obsidian,
    Pumice,
    Conglomerate,
    Breccia,
}

impl RockType {
    pub fn hardness(&self) -> f32 {
        match self {
            RockType::Granite => 6.0,
            RockType::Basalt => 6.0,
            RockType::Limestone => 3.0,
            RockType::Sandstone => 3.5,
            RockType::Shale => 2.0,
            RockType::Marble => 3.5,
            RockType::Quartzite => 7.0,
            RockType::Slate => 4.0,
            RockType::Gneiss => 6.5,
            RockType::Schist => 4.5,
            RockType::Obsidian => 5.5,
            RockType::Pumice => 1.0,
            RockType::Conglomerate => 3.0,
            RockType::Breccia => 4.0,
        }
    }

    pub fn density(&self) -> f32 {
        match self {
            RockType::Granite => 2750.0,
            RockType::Basalt => 3000.0,
            RockType::Limestone => 2500.0,
            RockType::Sandstone => 2300.0,
            RockType::Shale => 2600.0,
            RockType::Marble => 2700.0,
            RockType::Quartzite => 2650.0,
            RockType::Slate => 2800.0,
            RockType::Gneiss => 2700.0,
            RockType::Schist => 2500.0,
            RockType::Obsidian => 2400.0,
            RockType::Pumice => 700.0,
            RockType::Conglomerate => 2400.0,
            RockType::Breccia => 2500.0,
        }
    }

    pub fn erosion_resistance(&self) -> f32 {
        match self {
            RockType::Granite => 0.9,
            RockType::Basalt => 0.85,
            RockType::Limestone => 0.3,
            RockType::Sandstone => 0.4,
            RockType::Shale => 0.15,
            RockType::Marble => 0.5,
            RockType::Quartzite => 0.95,
            RockType::Slate => 0.6,
            RockType::Gneiss => 0.85,
            RockType::Schist => 0.45,
            RockType::Obsidian => 0.7,
            RockType::Pumice => 0.05,
            RockType::Conglomerate => 0.35,
            RockType::Breccia => 0.25,
        }
    }

    pub fn porosity(&self) -> f32 {
        match self {
            RockType::Sandstone => 0.25,
            RockType::Limestone => 0.15,
            RockType::Pumice => 0.8,
            RockType::Shale => 0.1,
            RockType::Conglomerate => 0.2,
            RockType::Breccia => 0.18,
            _ => 0.02,
        }
    }

    pub fn category(&self) -> RockCategory {
        match self {
            RockType::Granite | RockType::Basalt | RockType::Obsidian | RockType::Pumice => {
                RockCategory::Igneous
            },
            RockType::Limestone
            | RockType::Sandstone
            | RockType::Shale
            | RockType::Conglomerate
            | RockType::Breccia => RockCategory::Sedimentary,
            RockType::Marble
            | RockType::Quartzite
            | RockType::Slate
            | RockType::Gneiss
            | RockType::Schist => RockCategory::Metamorphic,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RockCategory {
    Igneous,
    Sedimentary,
    Metamorphic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RockFormation {
    pub rock_type: RockType,
    pub thickness: f32,
    pub age_ma: f32,
    pub fractures: f32,
    pub weathering: f32,
}

impl RockFormation {
    pub fn new(rock_type: RockType, thickness: f32) -> Self {
        Self { rock_type, thickness, age_ma: 0.0, fractures: 0.0, weathering: 0.0 }
    }

    pub fn effective_hardness(&self) -> f32 {
        self.rock_type.hardness() * (1.0 - self.weathering) * (1.0 - self.fractures * 0.5)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratigraphicColumn {
    pub layers: Vec<RockFormation>,
    pub base_depth: f32,
}

impl StratigraphicColumn {
    pub fn new() -> Self {
        Self { layers: Vec::new(), base_depth: 0.0 }
    }

    pub fn add_layer(&mut self, formation: RockFormation) {
        self.base_depth += formation.thickness;
        self.layers.push(formation);
    }

    pub fn rock_at_depth(&self, depth: f32) -> Option<&RockFormation> {
        let mut cumulative = 0.0;
        for layer in &self.layers {
            cumulative += layer.thickness;
            if depth <= cumulative {
                return Some(layer);
            }
        }
        self.layers.last()
    }
}

impl Default for StratigraphicColumn {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rock_type_properties() {
        assert_eq!(RockType::Granite.hardness(), 6.0);
        assert_eq!(RockType::Granite.density(), 2750.0);
        assert_eq!(RockType::Granite.category(), RockCategory::Igneous);
        assert!(RockType::Granite.erosion_resistance() > 0.0);
    }

    #[test]
    fn test_rock_formation_creation() {
        let formation = RockFormation::new(RockType::Basalt, 100.0);
        assert_eq!(formation.rock_type, RockType::Basalt);
        assert_eq!(formation.thickness, 100.0);
        assert_eq!(formation.age_ma, 0.0);
        assert!(formation.effective_hardness() > 0.0);
    }

    #[test]
    fn test_stratigraphic_column() {
        let mut column = StratigraphicColumn::new();
        column.add_layer(RockFormation::new(RockType::Sandstone, 50.0));
        column.add_layer(RockFormation::new(RockType::Granite, 100.0));
        assert_eq!(column.layers.len(), 2);
        let rock = column.rock_at_depth(30.0);
        assert!(rock.is_some());
        assert_eq!(rock.unwrap().rock_type, RockType::Sandstone);
        let deep = column.rock_at_depth(200.0);
        assert!(deep.is_some());
        assert_eq!(deep.unwrap().rock_type, RockType::Granite);
    }
}
