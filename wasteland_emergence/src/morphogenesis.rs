use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorphogeneticField {
    pub id: Uuid,
    pub name: String,
    pub resolution: [u32; 3],
    pub origin: Vec3,
    pub cell_size: f32,
    pub chemical_gradients: Vec<ChemicalGradient>,
    pub gene_tokens: Vec<GeneToken>,
    pub target_shape: Option<Vec<TargetShapeComponent>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemicalGradient {
    pub chemical_id: u32,
    pub source_positions: Vec<Vec3>,
    pub source_strengths: Vec<f32>,
    pub diffusivity: f32,
    pub decay_rate: f32,
    pub current_pattern: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneToken {
    pub token_id: u32,
    pub token_type: GeneTokenType,
    pub parameters: Vec<f32>,
    pub expression_level: f32,
    pub activation_conditions: Vec<ActivationCondition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneTokenType {
    Proliferate,
    Migrate,
    Differentiate,
    Apoptosis,
    SecreteChemical,
    FormBranch,
    FormLeaf,
    FormRoot,
    FormSpore,
    FormTendril,
    FormThorn,
    FormFruitingBody,
    Custom(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationCondition {
    pub condition_type: ConditionType,
    pub threshold: f32,
    pub chemical_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionType {
    ChemicalAbove,
    ChemicalBelow,
    DensityAbove,
    DensityBelow,
    DistanceFromSource,
    RandomProbability,
    AgeAbove,
    NeighborCountAbove,
    NeighborCountBelow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetShapeComponent {
    pub shape_type: ShapeComponentType,
    pub position: Vec3,
    pub orientation: Vec3,
    pub size: Vec3,
    pub weight: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShapeComponentType {
    Sphere,
    Ellipsoid,
    Cylinder,
    TaperedCylinder,
    Branch,
    Leaf,
    Ribbed,
    Spiky,
    Flat,
    Curved,
}

impl MorphogeneticField {
    pub fn new(name: &str, resolution: [u32; 3], origin: Vec3, cell_size: f32) -> Self {
        let _total = (resolution[0] * resolution[1] * resolution[2]) as usize;
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            resolution,
            origin,
            cell_size,
            chemical_gradients: Vec::new(),
            gene_tokens: Vec::new(),
            target_shape: None,
        }
    }

    fn index(&self, x: u32, y: u32, z: u32) -> usize {
        (z * self.resolution[1] * self.resolution[0] + y * self.resolution[0] + x) as usize
    }

    pub fn add_gene_token(&mut self, token: GeneToken) {
        self.gene_tokens.push(token);
    }

    pub fn add_chemical_gradient(&mut self, gradient: ChemicalGradient) {
        let total = (self.resolution[0] * self.resolution[1] * self.resolution[2]) as usize;
        let mut grad = gradient;
        if grad.current_pattern.is_empty() {
            grad.current_pattern = vec![0.0; total];
        }
        self.chemical_gradients.push(grad);
    }

    pub fn set_source(&mut self, chemical_id: u32, position: Vec3, strength: f32) {
        for grad in &mut self.chemical_gradients {
            if grad.chemical_id == chemical_id {
                grad.source_positions.push(position);
                grad.source_strengths.push(strength);
                return;
            }
        }
    }

    pub fn diffuse_chemicals(&mut self, dt: f32) {
        let resolution = self.resolution;
        let cell_size = self.cell_size;
        let origin = self.origin;
        let idx_fn = |x: u32, y: u32, z: u32| -> usize {
            (z * resolution[1] * resolution[0] + y * resolution[0] + x) as usize
        };

        for grad in &mut self.chemical_gradients {
            let mut new_pattern = vec![0.0f32; grad.current_pattern.len()];

            for z in 0..resolution[2] {
                for y in 0..resolution[1] {
                    for x in 0..resolution[0] {
                        let idx = idx_fn(x, y, z);

                        let mut laplacian = 0.0f32;
                        let center = grad.current_pattern[idx];

                        if x > 0 {
                            laplacian += grad.current_pattern[idx_fn(x - 1, y, z)] - center;
                        }
                        if x + 1 < resolution[0] {
                            laplacian += grad.current_pattern[idx_fn(x + 1, y, z)] - center;
                        }
                        if y > 0 {
                            laplacian += grad.current_pattern[idx_fn(x, y - 1, z)] - center;
                        }
                        if y + 1 < resolution[1] {
                            laplacian += grad.current_pattern[idx_fn(x, y + 1, z)] - center;
                        }
                        if z > 0 {
                            laplacian += grad.current_pattern[idx_fn(x, y, z - 1)] - center;
                        }
                        if z + 1 < resolution[2] {
                            laplacian += grad.current_pattern[idx_fn(x, y, z + 1)] - center;
                        }

                        let diffusion = grad.diffusivity * laplacian;
                        let decay = grad.decay_rate * center;

                        new_pattern[idx] = center + (diffusion - decay) * dt;
                        new_pattern[idx] = new_pattern[idx].max(0.0);
                    }
                }
            }

            for (pos, strength) in grad.source_positions.iter().zip(grad.source_strengths.iter()) {
                let local = *pos - origin;
                let sx = (local.x / cell_size).round() as i32;
                let sy = (local.y / cell_size).round() as i32;
                let sz = (local.z / cell_size).round() as i32;

                let radius = 3i32;
                for dx in -radius..=radius {
                    for dy in -radius..=radius {
                        for dz in -radius..=radius {
                            let nx = sx + dx;
                            let ny = sy + dy;
                            let nz = sz + dz;
                            if nx >= 0
                                && nx < resolution[0] as i32
                                && ny >= 0
                                && ny < resolution[1] as i32
                                && nz >= 0
                                && nz < resolution[2] as i32
                            {
                                let dist = ((dx * dx + dy * dy + dz * dz) as f32).sqrt();
                                let factor = (-dist * dist / 4.0).exp();
                                let idx = idx_fn(nx as u32, ny as u32, nz as u32);
                                new_pattern[idx] += strength * factor * dt;
                            }
                        }
                    }
                }
            }

            grad.current_pattern = new_pattern;
        }
    }

    pub fn evaluate_expression(
        &self,
        x: u32,
        y: u32,
        z: u32,
        age: Option<f32>,
        neighbor_count: Option<usize>,
    ) -> Vec<(&GeneToken, f32)> {
        let idx = self.index(x, y, z);
        let mut results = Vec::new();

        for token in &self.gene_tokens {
            let mut all_conditions_met = true;

            for condition in &token.activation_conditions {
                let met = match condition.condition_type {
                    ConditionType::ChemicalAbove => {
                        if let Some(cid) = condition.chemical_id {
                            if let Some(grad) =
                                self.chemical_gradients.iter().find(|g| g.chemical_id == cid)
                            {
                                grad.current_pattern[idx] > condition.threshold
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    },
                    ConditionType::ChemicalBelow => {
                        if let Some(cid) = condition.chemical_id {
                            if let Some(grad) =
                                self.chemical_gradients.iter().find(|g| g.chemical_id == cid)
                            {
                                grad.current_pattern[idx] < condition.threshold
                            } else {
                                true
                            }
                        } else {
                            true
                        }
                    },
                    ConditionType::RandomProbability => rand::random::<f32>() < condition.threshold,
                    ConditionType::AgeAbove => age.unwrap_or(0.0) > condition.threshold,
                    ConditionType::NeighborCountAbove => {
                        neighbor_count.unwrap_or(0) as f32 > condition.threshold
                    },
                    ConditionType::NeighborCountBelow => {
                        (neighbor_count.unwrap_or(usize::MAX) as f32) < condition.threshold
                    },
                    _ => true,
                };

                if !met {
                    all_conditions_met = false;
                    break;
                }
            }

            if all_conditions_met {
                results.push((token, token.expression_level));
            }
        }

        results
    }

    pub fn get_chemical_at(&self, chemical_id: u32, x: u32, y: u32, z: u32) -> f32 {
        if x >= self.resolution[0] || y >= self.resolution[1] || z >= self.resolution[2] {
            return 0.0;
        }
        let idx = self.index(x, y, z);
        for grad in &self.chemical_gradients {
            if grad.chemical_id == chemical_id {
                return grad.current_pattern[idx];
            }
        }
        0.0
    }

    pub fn step(&mut self, dt: f32) {
        self.diffuse_chemicals(dt);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorphogeneticOrganism {
    pub id: Uuid,
    pub field: MorphogeneticField,
    pub cells: Vec<OrganismCell>,
    pub growth_stage: GrowthStage,
    pub age: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganismCell {
    pub position: Vec3,
    pub cell_type: CellType,
    pub division_timer: f32,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellType {
    StemCell,
    BoneCell,
    MuscleCell,
    SkinCell,
    NerveCell,
    GlandCell,
    SporeCell,
    HyphaCell,
    FruitingBodyCell,
    Custom(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrowthStage {
    Embryo,
    Larval,
    Juvenile,
    Adult,
    Senescent,
    Sporulating,
}

impl MorphogeneticOrganism {
    pub fn new(name: &str, resolution: [u32; 3], origin: Vec3, cell_size: f32) -> Self {
        let field = MorphogeneticField::new(name, resolution, origin, cell_size);
        Self {
            id: Uuid::new_v4(),
            field,
            cells: Vec::new(),
            growth_stage: GrowthStage::Embryo,
            age: 0.0,
        }
    }

    pub fn seed_cell(&mut self, position: Vec3, cell_type: CellType) {
        self.cells.push(OrganismCell { position, cell_type, division_timer: 10.0, active: true });
    }

    pub fn grow(&mut self, dt: f32) {
        self.field.step(dt);
        self.age += dt;

        if self.age > 100.0 && self.growth_stage == GrowthStage::Adult {
            self.growth_stage = GrowthStage::Senescent;
        } else if self.age > 50.0 && self.growth_stage == GrowthStage::Juvenile {
            self.growth_stage = GrowthStage::Adult;
        } else if self.age > 10.0 && self.growth_stage == GrowthStage::Embryo {
            self.growth_stage = GrowthStage::Juvenile;
        }

        let mut new_cells = Vec::new();
        let cell_count = self.cells.len();

        for i in 0..cell_count {
            if !self.cells[i].active {
                continue;
            }

            let pos = self.cells[i].position;
            let local = pos - self.field.origin;
            let cx = (local.x / self.field.cell_size).round() as u32;
            let cy = (local.y / self.field.cell_size).round() as u32;
            let cz = (local.z / self.field.cell_size).round() as u32;

            let mut neighbor_count = 0usize;
            for j in 0..cell_count {
                if i != j && self.cells[j].active {
                    let dist = (self.cells[i].position - self.cells[j].position).length();
                    if dist < self.field.cell_size * 2.0 {
                        neighbor_count += 1;
                    }
                }
            }

            let expressions =
                self.field.evaluate_expression(cx, cy, cz, Some(self.age), Some(neighbor_count));

            for (token, level) in &expressions {
                match token.token_type {
                    GeneTokenType::Proliferate
                        if self.cells[i].division_timer <= 0.0 && level > &0.5 =>
                    {
                        let offset = Vec3::new(
                            (rand::random::<f32>() - 0.5) * self.field.cell_size * 2.0,
                            (rand::random::<f32>() - 0.5) * self.field.cell_size * 2.0,
                            (rand::random::<f32>() - 0.5) * self.field.cell_size * 2.0,
                        );
                        new_cells.push(OrganismCell {
                            position: pos + offset,
                            cell_type: self.cells[i].cell_type,
                            division_timer: 10.0,
                            active: true,
                        });
                        self.cells[i].division_timer = 10.0;
                    },
                    GeneTokenType::Migrate => {
                        let grad = self.field.get_chemical_at(
                            token.parameters.first().copied().unwrap_or(0.0) as u32,
                            cx,
                            cy,
                            cz,
                        );
                        let dir = if grad > 0.5 {
                            Vec3::new(
                                (rand::random::<f32>() - 0.5) * 2.0,
                                (rand::random::<f32>() - 0.5) * 2.0,
                                (rand::random::<f32>() - 0.5) * 2.0,
                            )
                            .normalize()
                        } else {
                            Vec3::ZERO
                        };
                        self.cells[i].position += dir * self.field.cell_size * level * dt;
                    },
                    GeneTokenType::Differentiate if level > &0.7 => {
                        self.cells[i].cell_type = CellType::MuscleCell;
                    },
                    GeneTokenType::Apoptosis if level > &0.9 => {
                        self.cells[i].active = false;
                    },
                    _ => {},
                }
            }

            self.cells[i].division_timer -= dt;
        }

        self.cells.extend(new_cells);
    }

    pub fn active_cells(&self) -> Vec<&OrganismCell> {
        self.cells.iter().filter(|c| c.active).collect()
    }
}
