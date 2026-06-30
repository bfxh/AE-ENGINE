use glam::Vec3;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::meta_entity::{
    BiologyAttributes, ChemistryAttributes, MetaEntity, MetaEntityState, PhysicsAttributes,
};

/// 功能推导引擎 — 实时分析几何结构与材料属性，输出功能置信度
///
/// 规则取代配方：不查表，根据几何特征+材料属性+质量分布实时推导物品功能
#[derive(Debug, Clone)]
pub struct FunctionalDerivationEngine {
    pub confidence_threshold: f32,
    pub analysis_cache: HashMap<Uuid, FunctionalAnalysis>,
    pub recent_blueprints: Vec<Blueprint>,
    pub tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionalAnalysis {
    pub entity_id: Uuid,
    pub functions: Vec<FunctionConfidence>,
    pub derived_properties: DerivedProperties,
    pub socket_points: Vec<SocketPoint>,
    pub analysis_tick: u64,
    pub geometry_hash: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionConfidence {
    pub function: Function,
    pub confidence: f32,
    pub reasoning: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Function {
    Cutting,
    Piercing,
    Bludgeoning,
    Chopping,
    Slicing,
    Scraping,
    Digging,
    Hammering,
    Prying,
    Holding,
    Containing,
    Filtering,
    Covering,
    Supporting,
    Connecting,
    Hinging,
    Rolling,
    Sliding,
    Springing,
    Absorbing,
    Conducting,
    Insulating,
    Reflecting,
    Focusing,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedProperties {
    pub damage: f32,
    pub durability: f32,
    pub center_of_mass: Vec3,
    pub attack_speed: f32,
    pub reach: f32,
    pub weight: f32,
    pub balance: f32,
    pub edge_retention: f32,
    pub heat_capacity: f32,
    pub electrical_resistance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketPoint {
    pub position: Vec3,
    pub normal: Vec3,
    pub socket_type: SocketType,
    pub max_force: f32,
    pub compatibility: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SocketType {
    Grip,
    Blade,
    Pommel,
    Crossguard,
    Haft,
    Head,
    Binding,
    Hinge,
    Snap,
    Threaded,
    Universal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub id: Uuid,
    pub name: String,
    pub parts: Vec<BlueprintPart>,
    pub assembly_instructions: Vec<AssemblyStep>,
    pub resulting_functions: Vec<FunctionConfidence>,
    pub author: String,
    pub license: String,
    pub created_tick: u64,
    pub verified: bool,
    pub usage_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintPart {
    pub material_category: String,
    pub approximate_shape: ShapeDescriptor,
    pub socket_types: Vec<SocketType>,
    pub quantity: u32,
    pub alternatives: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeDescriptor {
    pub length: f32,
    pub width: f32,
    pub height: f32,
    pub volume: f32,
    pub has_sharp_edge: bool,
    pub has_point: bool,
    pub has_cavity: bool,
    pub has_flat_surface: bool,
    pub curvature: f32,
    pub edge_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssemblyStep {
    pub step_number: u32,
    pub part_a_index: u32,
    pub part_b_index: u32,
    pub socket_a: SocketType,
    pub socket_b: SocketType,
    pub required_force: f32,
    pub description: String,
}

impl FunctionalDerivationEngine {
    pub fn new() -> Self {
        Self {
            confidence_threshold: 0.5,
            analysis_cache: HashMap::new(),
            recent_blueprints: Vec::new(),
            tick: 0,
        }
    }

    /// 分析单个元体的功能
    pub fn analyze_entity(
        &mut self,
        entity: &MetaEntity,
        geometry: &EntityGeometry,
    ) -> FunctionalAnalysis {
        let mut functions = Vec::new();
        let _reasoning: Vec<String> = Vec::new();

        // 切割功能：锐边 + 高硬度
        if geometry.has_sharp_edge && entity.physics.hardness > 3.0 {
            let confidence = (geometry.edge_sharpness * 0.4
                + (entity.physics.hardness / 10.0) * 0.3
                + (geometry.length / geometry.width).min(10.0) * 0.03
                + (entity.physics.toughness / 100.0) * 0.2)
                .min(1.0);
            functions.push(FunctionConfidence {
                function: Function::Cutting,
                confidence,
                reasoning: vec![
                    format!("sharp_edge: {:.2}", geometry.edge_sharpness),
                    format!("hardness: {:.1}", entity.physics.hardness),
                    format!("aspect_ratio: {:.1}", geometry.length / geometry.width.max(0.01)),
                ],
            });
        }

        // 穿刺功能：尖端 + 高硬度 + 细长
        if geometry.has_point && entity.physics.hardness > 3.0 {
            let confidence = (geometry.point_sharpness * 0.4
                + (entity.physics.hardness / 10.0) * 0.3
                + (1.0 - geometry.width / geometry.length.max(1.0)) * 0.3)
                .min(1.0);
            functions.push(FunctionConfidence {
                function: Function::Piercing,
                confidence,
                reasoning: vec![
                    format!("point_sharpness: {:.2}", geometry.point_sharpness),
                    format!("hardness: {:.1}", entity.physics.hardness),
                ],
            });
        }

        // 钝击功能：大质量 + 低硬度（或任意形状）
        if entity.physics.mass > 0.5 {
            let confidence = ((entity.physics.mass / 20.0) * 0.5
                + (entity.physics.density / 10000.0) * 0.3
                + (geometry.volume / 0.1) * 0.2)
                .min(1.0);
            if confidence > 0.3 {
                functions.push(FunctionConfidence {
                    function: Function::Bludgeoning,
                    confidence,
                    reasoning: vec![
                        format!("mass: {:.2}kg", entity.physics.mass),
                        format!("density: {:.0}", entity.physics.density),
                    ],
                });
            }
        }

        // 容器功能：空腔
        if geometry.has_cavity {
            let confidence = (geometry.cavity_volume * 10.0).min(0.9) + 0.1;
            functions.push(FunctionConfidence {
                function: Function::Containing,
                confidence,
                reasoning: vec![format!("cavity_volume: {:.4}", geometry.cavity_volume)],
            });
        }

        // 支撑功能：平坦表面 + 高强度
        if geometry.has_flat_surface && entity.physics.yield_strength > 1e6 {
            let confidence = (geometry.flatness * 0.3
                + (entity.physics.yield_strength / 1e8).min(1.0) * 0.4
                + (geometry.surface_area / 10.0).min(1.0) * 0.3)
                .min(1.0);
            functions.push(FunctionConfidence {
                function: Function::Supporting,
                confidence,
                reasoning: vec![
                    format!("flatness: {:.2}", geometry.flatness),
                    format!("yield_strength: {:.2e}", entity.physics.yield_strength),
                ],
            });
        }

        // 连接功能：Socket点数量
        if !geometry.socket_candidates.is_empty() {
            let confidence = (geometry.socket_candidates.len() as f32 / 5.0).min(1.0);
            functions.push(FunctionConfidence {
                function: Function::Connecting,
                confidence,
                reasoning: vec![format!("socket_count: {}", geometry.socket_candidates.len())],
            });
        }

        // 导出属性
        let damage = functions
            .iter()
            .map(|f| {
                f.confidence
                    * match f.function {
                        Function::Cutting | Function::Piercing | Function::Chopping => 100.0,
                        Function::Bludgeoning => 50.0,
                        _ => 10.0,
                    }
            })
            .sum::<f32>();

        let durability = entity.physics.toughness * entity.physics.yield_strength / 1e7;
        let attack_speed = 1.0 / (entity.physics.mass * 0.1 + 0.5).sqrt();
        let reach = geometry.length;
        let balance = 1.0
            - (geometry.center_of_mass - Vec3::new(0.0, 0.0, geometry.length * 0.3)).length()
                / geometry.length.max(0.01);

        let analysis = FunctionalAnalysis {
            entity_id: entity.id,
            functions,
            derived_properties: DerivedProperties {
                damage,
                durability,
                center_of_mass: geometry.center_of_mass,
                attack_speed,
                reach,
                weight: entity.physics.mass,
                balance,
                edge_retention: entity.physics.hardness / 10.0,
                heat_capacity: entity.physics.specific_heat_capacity * entity.physics.mass,
                electrical_resistance: if entity.physics.electrical_conductivity > 0.0 {
                    1.0 / entity.physics.electrical_conductivity
                } else {
                    f32::MAX
                },
            },
            socket_points: geometry.socket_candidates.clone(),
            analysis_tick: self.tick,
            geometry_hash: geometry.compute_hash(),
        };

        self.analysis_cache.insert(entity.id, analysis.clone());
        analysis
    }

    /// 分析组合体（多个元体装配在一起）的功能
    pub fn analyze_assembly(
        &mut self,
        entities: &[&MetaEntity],
        geometries: &[EntityGeometry],
        connections: &[(usize, usize, SocketType, SocketType)],
    ) -> FunctionalAnalysis {
        let assembly_id = Uuid::new_v4();
        let mut combined_geometry = EntityGeometry::default();

        for geo in geometries {
            combined_geometry.merge(geo);
        }

        let mut combined_physics = PhysicsAttributes::default();
        for entity in entities {
            combined_physics.mass += entity.physics.mass;
            combined_physics.density = (combined_physics.density + entity.physics.density) / 2.0;
            combined_physics.hardness = combined_physics.hardness.max(entity.physics.hardness);
            combined_physics.toughness = combined_physics.toughness.max(entity.physics.toughness);
            combined_physics.yield_strength =
                combined_physics.yield_strength.max(entity.physics.yield_strength);
        }

        let dummy_entity = MetaEntity {
            id: assembly_id,
            version: 0,
            position: Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            physics: combined_physics,
            chemistry: ChemistryAttributes::default(),
            biology: BiologyAttributes::default(),
            state: MetaEntityState::Active,
            structural_field: None,
            spawn_tick: 0,
            parent_id: None,
            children: smallvec::SmallVec::new(),
            extensions: hashbrown::HashMap::new(),
            mpss_index: None,
        };

        let mut analysis = self.analyze_entity(&dummy_entity, &combined_geometry);

        if !connections.is_empty() {
            analysis.functions.push(FunctionConfidence {
                function: Function::Hinging,
                confidence: 0.8,
                reasoning: vec![format!("{} connections", connections.len())],
            });
        }

        analysis
    }

    /// 记录蓝图
    pub fn record_blueprint(
        &mut self,
        analysis: &FunctionalAnalysis,
        parts: Vec<BlueprintPart>,
        assembly: Vec<AssemblyStep>,
        author: String,
    ) -> Uuid {
        let blueprint = Blueprint {
            id: Uuid::new_v4(),
            name: format!("Blueprint_{}", self.recent_blueprints.len()),
            parts,
            assembly_instructions: assembly,
            resulting_functions: analysis.functions.clone(),
            author,
            license: "CC BY 4.0".into(),
            created_tick: self.tick,
            verified: false,
            usage_count: 0,
        };
        let id = blueprint.id;
        self.recent_blueprints.push(blueprint);
        if self.recent_blueprints.len() > 1000 {
            self.recent_blueprints.remove(0);
        }
        id
    }

    /// 搜索蓝图中最佳匹配
    pub fn find_similar_blueprint(&self, functions: &[FunctionConfidence]) -> Option<&Blueprint> {
        self.recent_blueprints.iter().max_by(|a, b| {
            let score_a = Self::blueprint_match_score(a, functions);
            let score_b = Self::blueprint_match_score(b, functions);
            score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    fn blueprint_match_score(blueprint: &Blueprint, target: &[FunctionConfidence]) -> f32 {
        let mut score = 0.0f32;
        for tf in target {
            for bf in &blueprint.resulting_functions {
                if bf.function == tf.function {
                    score += (1.0 - (tf.confidence - bf.confidence).abs()) * tf.confidence;
                }
            }
        }
        score
    }

    pub fn tick(&mut self) {
        self.tick += 1;
    }

    pub fn stats(&self) -> DerivationStats {
        DerivationStats {
            cached_analyses: self.analysis_cache.len(),
            total_blueprints: self.recent_blueprints.len(),
            verified_blueprints: self.recent_blueprints.iter().filter(|b| b.verified).count(),
            tick: self.tick,
        }
    }
}

impl Default for FunctionalDerivationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct DerivationStats {
    pub cached_analyses: usize,
    pub total_blueprints: usize,
    pub verified_blueprints: usize,
    pub tick: u64,
}

/// 实体几何特征描述
#[derive(Debug, Clone, Default)]
pub struct EntityGeometry {
    pub length: f32,
    pub width: f32,
    pub height: f32,
    pub volume: f32,
    pub surface_area: f32,
    pub has_sharp_edge: bool,
    pub edge_sharpness: f32,
    pub has_point: bool,
    pub point_sharpness: f32,
    pub has_cavity: bool,
    pub cavity_volume: f32,
    pub has_flat_surface: bool,
    pub flatness: f32,
    pub curvature: f32,
    pub edge_count: u32,
    pub center_of_mass: Vec3,
    pub socket_candidates: Vec<SocketPoint>,
    pub bounding_box: [Vec3; 2],
}

impl EntityGeometry {
    pub fn compute_hash(&self) -> u64 {
        let mut hash: u64 = 0;
        hash ^= (self.length as u64).wrapping_mul(0x9E3779B97F4A7C15);
        hash ^= (self.width as u64).wrapping_mul(0xC6A4A7935BD1E995);
        hash ^= (self.volume as u64).wrapping_mul(0xBF58476D1CE4E5B9);
        if self.has_sharp_edge {
            hash ^= 0x94D049BB133111EB;
        }
        if self.has_point {
            hash ^= 0x27D4EB2F165667C5;
        }
        if self.has_cavity {
            hash ^= 0x517CC1B727220A95;
        }
        hash
    }

    pub fn merge(&mut self, other: &EntityGeometry) {
        self.length = self.length.max(other.length);
        self.width = self.width.max(other.width);
        self.height += other.height;
        self.volume += other.volume;
        self.surface_area += other.surface_area;
        self.has_sharp_edge = self.has_sharp_edge || other.has_sharp_edge;
        self.has_point = self.has_point || other.has_point;
        self.has_cavity = self.has_cavity || other.has_cavity;
        self.has_flat_surface = self.has_flat_surface || other.has_flat_surface;
        self.socket_candidates.extend(other.socket_candidates.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta_entity::*;

    #[test]
    fn test_analyze_sword_like() {
        let mut engine = FunctionalDerivationEngine::new();
        let entity = MetaEntity::iron(glam::Vec3::ZERO, 0);
        let geometry = EntityGeometry {
            length: 1.0,
            width: 0.05,
            height: 0.02,
            volume: 0.001,
            has_sharp_edge: true,
            edge_sharpness: 0.8,
            has_point: true,
            point_sharpness: 0.7,
            has_flat_surface: true,
            flatness: 0.9,
            center_of_mass: glam::Vec3::new(0.0, 0.0, 0.3),
            ..Default::default()
        };

        let analysis = engine.analyze_entity(&entity, &geometry);
        assert!(!analysis.functions.is_empty());
        let has_cutting = analysis.functions.iter().any(|f| f.function == Function::Cutting);
        assert!(has_cutting);
    }
}
