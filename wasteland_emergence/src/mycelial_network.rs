use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::morphogenesis::{
    ActivationCondition, ChemicalGradient, ConditionType, GeneToken, GeneTokenType,
    MorphogeneticOrganism,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MycelialNetwork {
    pub id: Uuid,
    pub name: String,
    pub organism: MorphogeneticOrganism,
    pub hyphae: Vec<Hypha>,
    pub nodes: Vec<MycelialNode>,
    pub fruiting_bodies: Vec<FruitingBody>,
    pub nutrient_map: Vec<f32>,
    pub total_biomass: f32,
    pub spread_rate: f32,
    pub tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypha {
    pub id: Uuid,
    pub start_node: usize,
    pub end_node: usize,
    pub length: f32,
    pub radius: f32,
    pub growth_rate: f32,
    pub nutrient_flow: f32,
    pub age: f32,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MycelialNode {
    pub id: Uuid,
    pub position: Vec3,
    pub node_type: NodeType,
    pub nutrient_storage: f32,
    pub chemical_signals: Vec<(String, f32)>,
    pub connections: Vec<usize>,
    pub age: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    Root,
    Branch,
    Tip,
    Anastomosis,
    FruitingBodyPrimordium,
    Senescent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FruitingBody {
    pub id: Uuid,
    pub position: Vec3,
    pub stage: FruitingBodyStage,
    pub size: f32,
    pub spore_count: u32,
    pub toxin_level: f32,
    pub nutrient_content: f32,
    pub age: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FruitingBodyStage {
    Primordium,
    Emerging,
    Expanding,
    Mature,
    Sporulating,
    Senescent,
}

impl MycelialNetwork {
    pub fn new(name: &str, resolution: [u32; 3], origin: Vec3, cell_size: f32) -> Self {
        let total = (resolution[0] * resolution[1] * resolution[2]) as usize;
        let mut organism = MorphogeneticOrganism::new(name, resolution, origin, cell_size);

        organism.field.add_gene_token(GeneToken {
            token_id: 0,
            token_type: GeneTokenType::FormBranch,
            parameters: vec![0.5, 2.0, 0.3],
            expression_level: 0.8,
            activation_conditions: vec![
                ActivationCondition {
                    condition_type: ConditionType::ChemicalAbove,
                    threshold: 0.3,
                    chemical_id: Some(0),
                },
                ActivationCondition {
                    condition_type: ConditionType::NeighborCountBelow,
                    threshold: 5.0,
                    chemical_id: None,
                },
            ],
        });

        organism.field.add_gene_token(GeneToken {
            token_id: 1,
            token_type: GeneTokenType::FormFruitingBody,
            parameters: vec![1.0, 3.0],
            expression_level: 0.6,
            activation_conditions: vec![
                ActivationCondition {
                    condition_type: ConditionType::ChemicalAbove,
                    threshold: 0.7,
                    chemical_id: Some(0),
                },
                ActivationCondition {
                    condition_type: ConditionType::AgeAbove,
                    threshold: 20.0,
                    chemical_id: None,
                },
            ],
        });

        organism.field.add_gene_token(GeneToken {
            token_id: 2,
            token_type: GeneTokenType::SecreteChemical,
            parameters: vec![0.0, 0.5, 0.1],
            expression_level: 0.9,
            activation_conditions: vec![ActivationCondition {
                condition_type: ConditionType::DensityAbove,
                threshold: 0.3,
                chemical_id: None,
            }],
        });

        organism.field.add_chemical_gradient(ChemicalGradient {
            chemical_id: 0,
            source_positions: Vec::new(),
            source_strengths: Vec::new(),
            diffusivity: 0.15,
            decay_rate: 0.02,
            current_pattern: vec![0.0; total],
        });

        organism.field.add_chemical_gradient(ChemicalGradient {
            chemical_id: 1,
            source_positions: Vec::new(),
            source_strengths: Vec::new(),
            diffusivity: 0.08,
            decay_rate: 0.05,
            current_pattern: vec![0.0; total],
        });

        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            organism,
            hyphae: Vec::new(),
            nodes: Vec::new(),
            fruiting_bodies: Vec::new(),
            nutrient_map: vec![0.5; total],
            total_biomass: 0.0,
            spread_rate: 0.5,
            tick: 0,
        }
    }

    pub fn spawn_root_node(&mut self, position: Vec3) -> usize {
        let node = MycelialNode {
            id: Uuid::new_v4(),
            position,
            node_type: NodeType::Root,
            nutrient_storage: 10.0,
            chemical_signals: vec![
                ("growth_factor".into(), 1.0),
                ("nutrient_attractant".into(), 0.5),
            ],
            connections: Vec::new(),
            age: 0.0,
        };
        self.nodes.push(node);

        self.organism.field.set_source(0, position, 1.0);

        self.nodes.len() - 1
    }

    pub fn grow(&mut self, dt: f32) {
        self.organism.grow(dt);
        self.tick += 1;

        self.transport_nutrients(dt);

        self.grow_tips(dt);

        self.check_anastomosis();

        if self.tick.is_multiple_of(30) {
            self.try_form_fruiting_body();
        }

        self.update_fruiting_bodies(dt);

        self.prune_senescent();

        self.total_biomass =
            self.nodes.iter().filter(|n| n.node_type != NodeType::Senescent).count() as f32
                + self.hyphae.iter().filter(|h| h.active).count() as f32 * 0.5;
    }

    fn transport_nutrients(&mut self, dt: f32) {
        let node_count = self.nodes.len();
        let mut flows = vec![0.0f32; node_count];

        for hypha in &self.hyphae {
            if !hypha.active {
                continue;
            }
            let start = hypha.start_node;
            let end = hypha.end_node;
            if start >= node_count || end >= node_count {
                continue;
            }

            let delta = self.nodes[end].nutrient_storage - self.nodes[start].nutrient_storage;
            let flow = delta * hypha.nutrient_flow * dt * 0.1;
            flows[start] -= flow;
            flows[end] += flow;
        }

        for (i, node) in &mut self.nodes.iter_mut().enumerate() {
            node.nutrient_storage = (node.nutrient_storage + flows[i]).clamp(0.0, 100.0);

            let pos = node.position;
            let local = pos - self.organism.field.origin;
            let cx = (local.x / self.organism.field.cell_size).round() as u32;
            let cy = (local.y / self.organism.field.cell_size).round() as u32;
            let cz = (local.z / self.organism.field.cell_size).round() as u32;

            let res = self.organism.field.resolution;
            if cx < res[0] && cy < res[1] && cz < res[2] {
                let idx = (cz * res[1] * res[0] + cy * res[0] + cx) as usize;
                if idx < self.nutrient_map.len() {
                    let absorption = self.nutrient_map[idx] * 0.01 * dt;
                    node.nutrient_storage = (node.nutrient_storage + absorption).min(100.0);
                    self.nutrient_map[idx] = (self.nutrient_map[idx] - absorption).max(0.0);
                }
            }
        }
    }

    fn grow_tips(&mut self, dt: f32) {
        let mut new_nodes = Vec::new();
        let mut new_hyphae = Vec::new();

        let node_count = self.nodes.len();
        for i in 0..node_count {
            if self.nodes[i].node_type != NodeType::Tip && self.nodes[i].node_type != NodeType::Root
            {
                continue;
            }
            if self.nodes[i].nutrient_storage < 1.0 {
                continue;
            }

            let pos = self.nodes[i].position;
            let local = pos - self.organism.field.origin;
            let cx = (local.x / self.organism.field.cell_size).round() as u32;
            let cy = (local.y / self.organism.field.cell_size).round() as u32;
            let cz = (local.z / self.organism.field.cell_size).round() as u32;

            let chem = self.organism.field.get_chemical_at(0, cx, cy, cz);
            let neighbor_count = self
                .nodes
                .iter()
                .filter(|n| (n.position - pos).length() < self.organism.field.cell_size * 3.0)
                .count();

            let can_grow =
                chem > 0.2 && neighbor_count < 8 && rand::random::<f32>() < self.spread_rate * dt;

            if !can_grow {
                continue;
            }

            let num_branches = if chem > 0.6 && neighbor_count < 4 { 2 } else { 1 };

            for _b in 0..num_branches {
                let angle = rand::random::<f32>() * std::f32::consts::TAU;
                let pitch = (rand::random::<f32>() - 0.5) * std::f32::consts::PI * 0.5;
                let direction =
                    Vec3::new(angle.cos() * pitch.cos(), pitch.sin(), angle.sin() * pitch.cos());

                let step = self.organism.field.cell_size * (1.0 + rand::random::<f32>());
                let new_pos = pos + direction * step;

                let new_idx = self.nodes.len() + new_nodes.len();
                new_nodes.push(MycelialNode {
                    id: Uuid::new_v4(),
                    position: new_pos,
                    node_type: NodeType::Tip,
                    nutrient_storage: self.nodes[i].nutrient_storage * 0.3,
                    chemical_signals: vec![
                        ("growth_factor".into(), chem),
                        ("nutrient_attractant".into(), 0.3),
                    ],
                    connections: vec![i],
                    age: 0.0,
                });

                new_hyphae.push(Hypha {
                    id: Uuid::new_v4(),
                    start_node: i,
                    end_node: new_idx,
                    length: step,
                    radius: 0.02,
                    growth_rate: self.spread_rate,
                    nutrient_flow: 0.5,
                    age: 0.0,
                    active: true,
                });

                self.nodes[i].nutrient_storage -= 0.5;

                self.organism.field.set_source(0, new_pos, chem * 0.7);
                self.organism.field.set_source(1, new_pos, chem * 0.3);
            }
        }

        self.nodes.extend(new_nodes);
        self.hyphae.extend(new_hyphae);
    }

    fn check_anastomosis(&mut self) {
        let n = self.nodes.len();
        let threshold = self.organism.field.cell_size * 1.5;

        for i in 0..n {
            if self.nodes[i].node_type == NodeType::Senescent {
                continue;
            }
            for j in (i + 1)..n {
                if self.nodes[j].node_type == NodeType::Senescent {
                    continue;
                }
                if self.nodes[i].connections.contains(&j) || self.nodes[j].connections.contains(&i)
                {
                    continue;
                }

                let dist = (self.nodes[i].position - self.nodes[j].position).length();
                if dist < threshold {
                    self.nodes[i].connections.push(j);
                    self.nodes[j].connections.push(i);
                    self.nodes[i].node_type = NodeType::Anastomosis;
                    self.nodes[j].node_type = NodeType::Anastomosis;

                    self.hyphae.push(Hypha {
                        id: Uuid::new_v4(),
                        start_node: i,
                        end_node: j,
                        length: dist,
                        radius: 0.015,
                        growth_rate: 0.0,
                        nutrient_flow: 0.8,
                        age: 0.0,
                        active: true,
                    });
                }
            }
        }
    }

    fn try_form_fruiting_body(&mut self) {
        let mut to_upgrade = Vec::new();

        for (i, node) in self.nodes.iter().enumerate() {
            if node.node_type == NodeType::Senescent
                || node.node_type == NodeType::FruitingBodyPrimordium
            {
                continue;
            }
            if node.nutrient_storage < 20.0 {
                continue;
            }

            let chem = {
                let pos = node.position;
                let local = pos - self.organism.field.origin;
                let cx = (local.x / self.organism.field.cell_size).round() as u32;
                let cy = (local.y / self.organism.field.cell_size).round() as u32;
                let cz = (local.z / self.organism.field.cell_size).round() as u32;
                self.organism.field.get_chemical_at(0, cx, cy, cz)
            };

            if chem > 0.7 && rand::random::<f32>() < 0.1 {
                to_upgrade.push((i, node.position));
            }
        }

        for (i, pos) in to_upgrade {
            self.nodes[i].node_type = NodeType::FruitingBodyPrimordium;
            self.nodes[i].nutrient_storage -= 15.0;

            self.fruiting_bodies.push(FruitingBody {
                id: Uuid::new_v4(),
                position: pos,
                stage: FruitingBodyStage::Primordium,
                size: 0.1,
                spore_count: 0,
                toxin_level: 0.0,
                nutrient_content: 15.0,
                age: 0.0,
            });
        }
    }

    fn update_fruiting_bodies(&mut self, dt: f32) {
        for fb in &mut self.fruiting_bodies {
            fb.age += dt;

            match fb.stage {
                FruitingBodyStage::Primordium => {
                    fb.size += dt * 0.05;
                    if fb.size > 0.5 {
                        fb.stage = FruitingBodyStage::Emerging;
                    }
                },
                FruitingBodyStage::Emerging => {
                    fb.size += dt * 0.1;
                    if fb.size > 1.5 {
                        fb.stage = FruitingBodyStage::Expanding;
                    }
                },
                FruitingBodyStage::Expanding => {
                    fb.size += dt * 0.15;
                    if fb.size > 3.0 {
                        fb.stage = FruitingBodyStage::Mature;
                    }
                },
                FruitingBodyStage::Mature => {
                    fb.spore_count += (dt * 100.0) as u32;
                    if fb.spore_count > 1000 {
                        fb.stage = FruitingBodyStage::Sporulating;
                    }
                },
                FruitingBodyStage::Sporulating => {
                    fb.spore_count -= (dt * 200.0) as u32;
                    if fb.spore_count < 100 {
                        fb.stage = FruitingBodyStage::Senescent;
                    }
                },
                FruitingBodyStage::Senescent => {
                    fb.size -= dt * 0.05;
                },
            }

            fb.toxin_level += dt * 0.01;
        }

        self.fruiting_bodies.retain(|fb| fb.stage != FruitingBodyStage::Senescent || fb.size > 0.1);
    }

    fn prune_senescent(&mut self) {
        for node in &mut self.nodes {
            node.age += 1.0;
            if node.nutrient_storage < 0.1 && node.node_type != NodeType::Root {
                node.node_type = NodeType::Senescent;
            }
        }
    }

    pub fn active_hyphae(&self) -> usize {
        self.hyphae
            .iter()
            .filter(|h| {
                h.active && {
                    h.start_node < self.nodes.len()
                        && self.nodes[h.start_node].node_type != NodeType::Senescent
                }
            })
            .count()
    }

    pub fn network_stats(&self) -> NetworkStats {
        let total_nodes = self.nodes.len();
        let tips = self.nodes.iter().filter(|n| n.node_type == NodeType::Tip).count();
        let anastomoses =
            self.nodes.iter().filter(|n| n.node_type == NodeType::Anastomosis).count();
        let avg_connections = if total_nodes > 0 {
            self.nodes.iter().map(|n| n.connections.len()).sum::<usize>() as f32
                / total_nodes as f32
        } else {
            0.0
        };

        NetworkStats {
            total_nodes,
            active_tips: tips,
            anastomosis_count: anastomoses,
            total_hyphae: self.hyphae.len(),
            active_hyphae: self.active_hyphae(),
            fruiting_bodies: self.fruiting_bodies.len(),
            total_biomass: self.total_biomass,
            average_connectivity: avg_connections,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub total_nodes: usize,
    pub active_tips: usize,
    pub anastomosis_count: usize,
    pub total_hyphae: usize,
    pub active_hyphae: usize,
    pub fruiting_bodies: usize,
    pub total_biomass: f32,
    pub average_connectivity: f32,
}

impl Default for MycelialNetwork {
    fn default() -> Self {
        Self::new("DefaultMycelium", [32, 32, 32], Vec3::ZERO, 1.0)
    }
}
