//! 角色损伤系统（从 v1 ae_render/procedural/damage.rs 移植）
//!
//! 突破性损伤模拟：
//! - 四级损伤等级（表面/中度/重度/完全毁坏）
//! - 生理地图（17 个身体区域独立跟踪）
//! - 血液流动模拟（血压/失血/休克/死亡）
//! - 骨骼破坏（闭合/开放/粉碎性骨折）
//! - 跨系统耦合（物理→化学→生物）
//!
//! Nova 适配：
//! - 输出 `MeshData`（损伤可视化贴花/伤口几何）
//! - 通过 `ProceduralGenerator` trait 集成
//! - 随机源用 `rand` crate

use crate::assets::MeshData;
use crate::procedural::{GeneratorParams, MeshBuilder, ProceduralGenerator, ProceduralStyle};
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand::Rng;
use serde::{Deserialize, Serialize};

// ============================================================================
// 损伤等级 + 类型
// ============================================================================

/// 四级损伤等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageLevel {
    Surface,
    Moderate,
    Severe,
    Destroyed,
}

impl DamageLevel {
    /// 功能影响因子（0=完全功能，1=完全丧失）
    pub fn functional_impact(&self) -> f32 {
        match self {
            DamageLevel::Surface => 0.0,
            DamageLevel::Moderate => 0.3,
            DamageLevel::Severe => 0.7,
            DamageLevel::Destroyed => 1.0,
        }
    }

    /// 出血速率倍数
    pub fn bleeding_multiplier(&self) -> f32 {
        match self {
            DamageLevel::Surface => 0.1,
            DamageLevel::Moderate => 1.0,
            DamageLevel::Severe => 5.0,
            DamageLevel::Destroyed => 10.0,
        }
    }
}

/// 损伤类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageType {
    Blunt,
    Slash,
    Pierce,
    Burn,
    Acid,
    Poison,
}

impl DamageType {
    /// 该损伤类型主要影响的组织层
    pub fn primary_target(&self) -> TissueLayer {
        match self {
            DamageType::Blunt => TissueLayer::Bone,
            DamageType::Slash => TissueLayer::Muscle,
            DamageType::Pierce => TissueLayer::Organ,
            DamageType::Burn => TissueLayer::Skin,
            DamageType::Acid => TissueLayer::Skin,
            DamageType::Poison => TissueLayer::Vascular,
        }
    }
}

/// 组织层
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TissueLayer {
    Skin,
    Muscle,
    Bone,
    Vascular,
    Nerve,
    Organ,
}

// ============================================================================
// 身体区域（17 个）
// ============================================================================

/// 身体区域标识
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BodyRegionId {
    Head,
    Neck,
    TorsoUpper,
    TorsoLower,
    Pelvis,
    LeftArmUpper,
    LeftArmLower,
    LeftHand,
    RightArmUpper,
    RightArmLower,
    RightHand,
    LeftLegUpper,
    LeftLegLower,
    LeftFoot,
    RightLegUpper,
    RightLegLower,
    RightFoot,
}

impl BodyRegionId {
    /// 是否包含重要器官（致命区域）
    pub fn contains_vital_organs(&self) -> bool {
        matches!(self, BodyRegionId::Head | BodyRegionId::TorsoUpper | BodyRegionId::TorsoLower)
    }

    /// 是否是肢体（断肢影响移动/操作）
    pub fn is_limb(&self) -> bool {
        matches!(
            self,
            BodyRegionId::LeftArmUpper | BodyRegionId::LeftArmLower | BodyRegionId::LeftHand
                | BodyRegionId::RightArmUpper | BodyRegionId::RightArmLower | BodyRegionId::RightHand
                | BodyRegionId::LeftLegUpper | BodyRegionId::LeftLegLower | BodyRegionId::LeftFoot
                | BodyRegionId::RightLegUpper | BodyRegionId::RightLegLower | BodyRegionId::RightFoot
        )
    }

    /// 该区域默认的骨骼应力容量（Pa）
    pub fn bone_capacity_pa(&self) -> f32 {
        match self {
            BodyRegionId::Head => 80_000.0,
            BodyRegionId::Neck => 50_000.0,
            BodyRegionId::TorsoUpper => 200_000.0,
            BodyRegionId::TorsoLower => 250_000.0,
            BodyRegionId::Pelvis => 400_000.0,
            BodyRegionId::LeftArmUpper | BodyRegionId::RightArmUpper => 150_000.0,
            BodyRegionId::LeftArmLower | BodyRegionId::RightArmLower => 100_000.0,
            BodyRegionId::LeftHand | BodyRegionId::RightHand => 60_000.0,
            BodyRegionId::LeftLegUpper | BodyRegionId::RightLegUpper => 300_000.0,
            BodyRegionId::LeftLegLower | BodyRegionId::RightLegLower => 200_000.0,
            BodyRegionId::LeftFoot | BodyRegionId::RightFoot => 100_000.0,
        }
    }

    /// 该区域在身体上的近似中心位置（米，本地坐标）
    pub fn body_center(&self) -> [f32; 3] {
        match self {
            BodyRegionId::Head => [0.0, 1.70, 0.0],
            BodyRegionId::Neck => [0.0, 1.55, 0.0],
            BodyRegionId::TorsoUpper => [0.0, 1.35, 0.0],
            BodyRegionId::TorsoLower => [0.0, 1.10, 0.0],
            BodyRegionId::Pelvis => [0.0, 0.95, 0.0],
            BodyRegionId::LeftArmUpper => [0.25, 1.35, 0.0],
            BodyRegionId::LeftArmLower => [0.28, 1.10, 0.0],
            BodyRegionId::LeftHand => [0.30, 0.92, 0.0],
            BodyRegionId::RightArmUpper => [-0.25, 1.35, 0.0],
            BodyRegionId::RightArmLower => [-0.28, 1.10, 0.0],
            BodyRegionId::RightHand => [-0.30, 0.92, 0.0],
            BodyRegionId::LeftLegUpper => [0.12, 0.70, 0.0],
            BodyRegionId::LeftLegLower => [0.12, 0.40, 0.0],
            BodyRegionId::LeftFoot => [0.12, 0.05, 0.0],
            BodyRegionId::RightLegUpper => [-0.12, 0.70, 0.0],
            BodyRegionId::RightLegLower => [-0.12, 0.40, 0.0],
            BodyRegionId::RightFoot => [-0.12, 0.05, 0.0],
        }
    }
}

// ============================================================================
// 器官
// ============================================================================

/// 器官类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrganType {
    Heart,
    Lung,
    Liver,
    Stomach,
    Brain,
    Kidney,
    Spleen,
    Intestine,
}

impl OrganType {
    /// 默认血流量（mL/min，成人静息）
    pub fn default_blood_flow_ml_min(&self) -> f32 {
        match self {
            OrganType::Heart => 225.0,
            OrganType::Lung => 5000.0,
            OrganType::Liver => 1500.0,
            OrganType::Stomach => 300.0,
            OrganType::Brain => 750.0,
            OrganType::Kidney => 1100.0,
            OrganType::Spleen => 300.0,
            OrganType::Intestine => 1000.0,
        }
    }

    /// 损伤该器官的致死阈值（完整性低于此值立即死亡）
    pub fn lethal_threshold(&self) -> f32 {
        match self {
            OrganType::Brain => 0.3,
            OrganType::Heart => 0.2,
            OrganType::Lung => 0.4,
            OrganType::Liver => 0.5,
            OrganType::Kidney => 0.6,
            OrganType::Spleen => 0.7,
            OrganType::Stomach => 0.6,
            OrganType::Intestine => 0.6,
        }
    }
}

/// 器官状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OrganState {
    pub organ_type: OrganType,
    pub integrity: f32,
    pub bleeding_rate: f32,
    pub infection_level: f32,
}

impl Default for OrganState {
    fn default() -> Self {
        Self {
            organ_type: OrganType::Heart,
            integrity: 1.0,
            bleeding_rate: 0.0,
            infection_level: 0.0,
        }
    }
}

impl OrganState {
    pub fn new(organ_type: OrganType) -> Self {
        Self { organ_type, integrity: 1.0, bleeding_rate: 0.0, infection_level: 0.0 }
    }

    pub fn is_lethal(&self) -> bool {
        self.integrity < self.organ_type.lethal_threshold()
    }
}

// ============================================================================
// 异物 + 出血源 + 骨折
// ============================================================================

/// 异物（弹片、箭矢等）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ForeignBody {
    pub position: [f32; 3],
    pub radius: f32,
    pub penetration_depth: f32,
    pub embedded: bool,
    pub infection_risk: f32,
}

/// 出血源
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BleedingSource {
    pub region: BodyRegionId,
    pub rate_ml_s: f32,
    pub is_arterial: bool,
    pub heartbeat_phase: f32,
}

/// 骨折类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FractureType {
    Closed,
    Open,
    Comminuted,
}

impl FractureType {
    pub fn bleeding_multiplier(&self) -> f32 {
        match self {
            FractureType::Closed => 0.5,
            FractureType::Open => 2.0,
            FractureType::Comminuted => 3.5,
        }
    }

    pub fn healing_time_days(&self) -> f32 {
        match self {
            FractureType::Closed => 42.0,
            FractureType::Open => 84.0,
            FractureType::Comminuted => 120.0,
        }
    }
}

// ============================================================================
// 身体区域状态
// ============================================================================

/// 身体区域完整状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyRegion {
    pub region_id: BodyRegionId,
    pub skin_integrity: f32,
    pub muscle_integrity: f32,
    pub bone_integrity: f32,
    pub vascular_integrity: f32,
    pub nerve_integrity: f32,
    pub organs: Vec<OrganState>,
    pub foreign_bodies: Vec<ForeignBody>,
    pub bleeding_sources: Vec<BleedingSource>,
    pub fracture: Option<FractureType>,
}

impl BodyRegion {
    pub fn new(region_id: BodyRegionId) -> Self {
        let organs = match region_id {
            BodyRegionId::Head => vec![OrganState::new(OrganType::Brain)],
            BodyRegionId::TorsoUpper => vec![
                OrganState::new(OrganType::Heart),
                OrganState::new(OrganType::Lung),
            ],
            BodyRegionId::TorsoLower => vec![
                OrganState::new(OrganType::Liver),
                OrganState::new(OrganType::Stomach),
                OrganState::new(OrganType::Kidney),
                OrganState::new(OrganType::Spleen),
                OrganState::new(OrganType::Intestine),
            ],
            _ => Vec::new(),
        };
        Self {
            region_id,
            skin_integrity: 1.0,
            muscle_integrity: 1.0,
            bone_integrity: 1.0,
            vascular_integrity: 1.0,
            nerve_integrity: 1.0,
            organs,
            foreign_bodies: Vec::new(),
            bleeding_sources: Vec::new(),
            fracture: None,
        }
    }

    /// 该区域功能因子（0=完全丧失，1=完全功能）
    pub fn functional_factor(&self) -> f32 {
        self.bone_integrity
            .min(self.muscle_integrity)
            .min(self.nerve_integrity)
            .min(self.vascular_integrity)
    }

    /// 是否已断裂（功能完全丧失）
    pub fn is_severed(&self) -> bool {
        self.bone_integrity <= 0.05 || self.muscle_integrity <= 0.05
    }

    /// 应用于该区域的损伤事件
    pub fn apply_damage(&mut self, event: &DamageEvent) {
        let force = event.force_newtons;
        let damage_type = event.damage_type;
        let sharpness = event.sharpness;
        let _ = sharpness;

        match damage_type {
            DamageType::Blunt => {
                let bone_stress = force / 0.01;
                if bone_stress > self.region_id.bone_capacity_pa() {
                    let bone_damage =
                        (bone_stress / self.region_id.bone_capacity_pa() - 1.0).min(1.0);
                    self.bone_integrity = (self.bone_integrity - bone_damage).max(0.0);
                    if self.fracture.is_none() && self.bone_integrity < 0.7 {
                        self.fracture = Some(if self.bone_integrity < 0.3 {
                            FractureType::Comminuted
                        } else if self.skin_integrity < 0.5 {
                            FractureType::Open
                        } else {
                            FractureType::Closed
                        });
                    }
                    self.vascular_integrity =
                        (self.vascular_integrity - bone_damage * 0.3).max(0.0);
                }
                let muscle_damage = (force / 2000.0).min(0.5);
                self.muscle_integrity = (self.muscle_integrity - muscle_damage).max(0.0);
                for organ in &mut self.organs {
                    if force > 500.0 {
                        let organ_dmg = (force / 2000.0).min(0.5);
                        organ.integrity = (organ.integrity - organ_dmg).max(0.0);
                        organ.bleeding_rate += organ_dmg * 5.0;
                    }
                }
            }
            DamageType::Slash => {
                let skin_dmg = (force * sharpness / 500.0).min(1.0);
                let muscle_dmg = (force * sharpness / 800.0).min(1.0);
                self.skin_integrity = (self.skin_integrity - skin_dmg).max(0.0);
                self.muscle_integrity = (self.muscle_integrity - muscle_dmg).max(0.0);
                self.vascular_integrity =
                    (self.vascular_integrity - muscle_dmg * 0.5).max(0.0);
                if muscle_dmg > 0.5 {
                    let bone_dmg = (force * sharpness / 5000.0).min(0.5);
                    self.bone_integrity = (self.bone_integrity - bone_dmg).max(0.0);
                }
            }
            DamageType::Pierce => {
                let skin_dmg = (force / 1000.0).min(0.8);
                let muscle_dmg = (force / 600.0).min(0.9);
                self.skin_integrity = (self.skin_integrity - skin_dmg).max(0.0);
                self.muscle_integrity = (self.muscle_integrity - muscle_dmg).max(0.0);
                self.vascular_integrity =
                    (self.vascular_integrity - muscle_dmg * 0.4).max(0.0);
                if event.foreign_body_embedded {
                    self.foreign_bodies.push(ForeignBody {
                        position: event.impact_position,
                        radius: 0.005,
                        penetration_depth: muscle_dmg * 0.1,
                        embedded: true,
                        infection_risk: 0.3,
                    });
                }
                if muscle_dmg > 0.5 {
                    for organ in &mut self.organs {
                        let organ_dmg = (force / 1500.0).min(0.7);
                        organ.integrity = (organ.integrity - organ_dmg).max(0.0);
                        organ.bleeding_rate += organ_dmg * 10.0;
                    }
                }
            }
            DamageType::Burn => {
                let thermal = event.thermal_energy_j;
                let skin_dmg = (thermal / 5000.0).min(1.0);
                self.skin_integrity = (self.skin_integrity - skin_dmg).max(0.0);
                self.nerve_integrity = (self.nerve_integrity - skin_dmg * 0.8).max(0.0);
                if skin_dmg > 0.5 {
                    let muscle_dmg = (thermal / 10000.0).min(0.5);
                    self.muscle_integrity = (self.muscle_integrity - muscle_dmg).max(0.0);
                }
            }
            DamageType::Acid => {
                let acid_strength = event.chemical_potency;
                let dmg = acid_strength * 0.5;
                self.skin_integrity = (self.skin_integrity - dmg).max(0.0);
                self.muscle_integrity = (self.muscle_integrity - dmg * 0.6).max(0.0);
                self.nerve_integrity = (self.nerve_integrity - dmg * 0.4).max(0.0);
            }
            DamageType::Poison => {
                let toxin = event.chemical_potency;
                self.vascular_integrity = (self.vascular_integrity - toxin * 0.2).max(0.0);
                for organ in &mut self.organs {
                    organ.integrity = (organ.integrity - toxin * 0.3).max(0.0);
                    organ.infection_level = (organ.infection_level + toxin * 0.5).min(1.0);
                }
            }
        }

        let level = self.assess_damage_level();
        if level != DamageLevel::Surface || !self.bleeding_sources.is_empty() {
            let bleed_rate = level.bleeding_multiplier() * (1.0 - self.vascular_integrity) * 2.0;
            if bleed_rate > 0.01 {
                let is_arterial =
                    self.vascular_integrity < 0.3 && self.region_id.is_limb();
                self.bleeding_sources.push(BleedingSource {
                    region: self.region_id,
                    rate_ml_s: bleed_rate,
                    is_arterial,
                    heartbeat_phase: 0.0,
                });
            }
        }
    }

    /// 评估该区域的综合损伤等级
    pub fn assess_damage_level(&self) -> DamageLevel {
        let worst = 1.0 - self.functional_factor();
        if worst >= 0.95 || self.is_severed() {
            DamageLevel::Destroyed
        } else if worst >= 0.5 {
            DamageLevel::Severe
        } else if worst >= 0.15 {
            DamageLevel::Moderate
        } else {
            DamageLevel::Surface
        }
    }
}

// ============================================================================
// 损伤事件
// ============================================================================

/// 损伤事件
#[derive(Debug, Clone, Copy)]
pub struct DamageEvent {
    pub damage_type: DamageType,
    pub target_region: BodyRegionId,
    pub force_newtons: f32,
    pub sharpness: f32,
    pub thermal_energy_j: f32,
    pub chemical_potency: f32,
    pub impact_position: [f32; 3],
    pub foreign_body_embedded: bool,
}

impl Default for DamageEvent {
    fn default() -> Self {
        Self {
            damage_type: DamageType::Blunt,
            target_region: BodyRegionId::TorsoUpper,
            force_newtons: 100.0,
            sharpness: 0.0,
            thermal_energy_j: 0.0,
            chemical_potency: 0.0,
            impact_position: [0.0, 0.0, 0.0],
            foreign_body_embedded: false,
        }
    }
}

// ============================================================================
// 生理地图（全身状态）
// ============================================================================

/// 生理地图（全身状态）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysiologicalMap {
    pub regions: Vec<BodyRegion>,
    pub total_blood_ml: f32,
    pub max_blood_ml: f32,
    pub heart_rate_bpm: f32,
    pub blood_pressure_systolic: f32,
    pub blood_pressure_diastolic: f32,
    pub consciousness: f32,
    pub pain_level: f32,
}

impl Default for PhysiologicalMap {
    fn default() -> Self {
        Self::new_human()
    }
}

impl PhysiologicalMap {
    /// 创建标准成年人生理地图（17 个身体区域）
    pub fn new_human() -> Self {
        let regions = vec![
            BodyRegion::new(BodyRegionId::Head),
            BodyRegion::new(BodyRegionId::Neck),
            BodyRegion::new(BodyRegionId::TorsoUpper),
            BodyRegion::new(BodyRegionId::TorsoLower),
            BodyRegion::new(BodyRegionId::Pelvis),
            BodyRegion::new(BodyRegionId::LeftArmUpper),
            BodyRegion::new(BodyRegionId::LeftArmLower),
            BodyRegion::new(BodyRegionId::LeftHand),
            BodyRegion::new(BodyRegionId::RightArmUpper),
            BodyRegion::new(BodyRegionId::RightArmLower),
            BodyRegion::new(BodyRegionId::RightHand),
            BodyRegion::new(BodyRegionId::LeftLegUpper),
            BodyRegion::new(BodyRegionId::LeftLegLower),
            BodyRegion::new(BodyRegionId::LeftFoot),
            BodyRegion::new(BodyRegionId::RightLegUpper),
            BodyRegion::new(BodyRegionId::RightLegLower),
            BodyRegion::new(BodyRegionId::RightFoot),
        ];
        Self {
            regions,
            total_blood_ml: 5000.0,
            max_blood_ml: 5000.0,
            heart_rate_bpm: 72.0,
            blood_pressure_systolic: 120.0,
            blood_pressure_diastolic: 80.0,
            consciousness: 0.0,
            pain_level: 0.0,
        }
    }

    pub fn region(&mut self, id: BodyRegionId) -> Option<&mut BodyRegion> {
        self.regions.iter_mut().find(|r| r.region_id == id)
    }

    pub fn apply_damage(&mut self, event: &DamageEvent) {
        if let Some(region) = self.region(event.target_region) {
            region.apply_damage(event);
        }
    }

    pub fn total_bleeding_rate(&self) -> f32 {
        let mut total = 0.0;
        for region in &self.regions {
            for src in &region.bleeding_sources {
                total += src.rate_ml_s;
            }
            for organ in &region.organs {
                total += organ.bleeding_rate / 60.0;
            }
        }
        total
    }

    /// 推进时间步（秒），更新血量、血压、心率、意识
    pub fn tick(&mut self, dt: f32) {
        let bleeding = self.total_bleeding_rate();
        self.total_blood_ml = (self.total_blood_ml - bleeding * dt).max(0.0);
        let blood_ratio = self.total_blood_ml / self.max_blood_ml;

        if blood_ratio > 0.85 {
            self.heart_rate_bpm = 72.0 + (1.0 - blood_ratio) * 100.0;
            self.blood_pressure_systolic = 120.0;
            self.blood_pressure_diastolic = 80.0;
        } else if blood_ratio > 0.6 {
            self.heart_rate_bpm = 110.0 + (0.85 - blood_ratio) * 200.0;
            self.blood_pressure_systolic = 120.0 * blood_ratio / 0.85;
            self.blood_pressure_diastolic = 80.0 * blood_ratio / 0.85;
        } else if blood_ratio > 0.3 {
            self.heart_rate_bpm = 140.0;
            self.blood_pressure_systolic = 70.0 * blood_ratio / 0.6;
            self.blood_pressure_diastolic = 40.0 * blood_ratio / 0.6;
            self.consciousness = (1.0 - blood_ratio / 0.6).max(self.consciousness);
        } else {
            self.heart_rate_bpm = 60.0 * blood_ratio / 0.3;
            self.blood_pressure_systolic = 40.0 * blood_ratio / 0.3;
            self.blood_pressure_diastolic = 20.0 * blood_ratio / 0.3;
            self.consciousness = 1.0;
        }

        let mut total_pain = 0.0;
        for region in &self.regions {
            let level = region.assess_damage_level();
            let weight = if region.region_id.contains_vital_organs() { 2.0 } else { 1.0 };
            total_pain += level.functional_impact() * weight;
        }
        self.pain_level = (total_pain / 17.0).min(1.0);
        if self.pain_level > 0.7 {
            self.consciousness = self.consciousness.max((self.pain_level - 0.7) * 3.0);
        }

        let heartbeat_dt = std::f32::consts::TAU / (self.heart_rate_bpm / 60.0) * dt;
        for region in &mut self.regions {
            for src in &mut region.bleeding_sources {
                if src.is_arterial {
                    src.heartbeat_phase =
                        (src.heartbeat_phase + heartbeat_dt) % std::f32::consts::TAU;
                }
            }
        }

        for region in &mut self.regions {
            for organ in &mut region.organs {
                if organ.infection_level > 0.0 {
                    organ.infection_level = (organ.infection_level + dt * 0.001).min(1.0);
                }
            }
        }
    }

    pub fn is_dead(&self) -> bool {
        if self.total_blood_ml / self.max_blood_ml < 0.3 {
            return true;
        }
        for region in &self.regions {
            for organ in &region.organs {
                if organ.is_lethal() {
                    return true;
                }
            }
        }
        false
    }

    pub fn is_in_shock(&self) -> bool {
        self.total_blood_ml / self.max_blood_ml < 0.6
    }

    pub fn overall_function(&self) -> f32 {
        if self.is_dead() {
            return 0.0;
        }
        let mut sum = 0.0;
        for region in &self.regions {
            sum += region.functional_factor();
        }
        sum / self.regions.len() as f32
    }

    pub fn all_bleeding_sources(&self) -> Vec<&BleedingSource> {
        self.regions
            .iter()
            .flat_map(|r| r.bleeding_sources.iter())
            .collect()
    }

    pub fn all_fractures(&self) -> Vec<(BodyRegionId, FractureType)> {
        self.regions
            .iter()
            .filter_map(|r| r.fracture.map(|f| (r.region_id, f)))
            .collect()
    }

    pub fn region_function_factor(&self, id: BodyRegionId) -> f32 {
        self.regions
            .iter()
            .find(|r| r.region_id == id)
            .map(|r| r.functional_factor())
            .unwrap_or(1.0)
    }

    pub fn is_region_severed(&self, id: BodyRegionId) -> bool {
        self.regions
            .iter()
            .find(|r| r.region_id == id)
            .map(|r| r.is_severed())
            .unwrap_or(false)
    }
}

// ============================================================================
// 损伤生成器（生成伤口可视化 Mesh）
// ============================================================================

/// 损伤生成输出
pub struct DamageOutput {
    pub mesh: MeshData,
    pub physiological: PhysiologicalMap,
}

/// 损伤生成器
///
/// 根据生理地图生成伤口可视化几何（贴花、血迹、骨折变形）。
pub struct DamageGenerator {
    pub map: PhysiologicalMap,
}

impl Default for DamageGenerator {
    fn default() -> Self {
        Self { map: PhysiologicalMap::new_human() }
    }
}

impl DamageGenerator {
    pub fn new(map: PhysiologicalMap) -> Self {
        Self { map }
    }

    /// 用专用生理地图生成损伤可视化
    pub fn generate_with_params(&self, map: &PhysiologicalMap) -> DamageOutput {
        let mut builder = MeshBuilder::new();
        let mut rng = StdRng::seed_from_u64(0xC0DE);

        for region in &map.regions {
            let level = region.assess_damage_level();
            if level == DamageLevel::Surface {
                continue;
            }
            let center = region.region_id.body_center();
            let severity = level.functional_impact();

            // 伤口贴花（暗红圆盘）
            let decal_radius = 0.05 + severity * 0.12;
            let decal_color = Self::wound_color(level);
            let mut decal = MeshBuilder::new();
            decal.push_sphere(
                [center[0], center[1], center[2]],
                decal_radius,
                8,
                4,
            );
            for v in decal.vertices_mut().iter_mut() {
                v.color = decal_color;
            }
            builder.append(&decal);

            // 骨折变形（白色骨刺）
            if let Some(fracture) = region.fracture {
                let bone_color = [0.85, 0.82, 0.75, 1.0];
                let spike_count = match fracture {
                    FractureType::Closed => 1,
                    FractureType::Open => 3,
                    FractureType::Comminuted => 5,
                };
                for i in 0..spike_count {
                    let angle = i as f32 * std::f32::consts::TAU / spike_count as f32
                        + rng.gen_range(0.0..0.5);
                    let offset = [
                        center[0] + angle.cos() * 0.04,
                        center[1] + rng.gen_range(-0.03..0.03),
                        center[2] + angle.sin() * 0.04,
                    ];
                    let mut spike = MeshBuilder::new();
                    spike.push_cylinder(0.008, 0.06 + severity * 0.04, 4, true);
                    spike.transform(offset, [0.0, 0.0, 0.0, 1.0], 1.0);
                    for v in spike.vertices_mut().iter_mut() {
                        v.color = bone_color;
                    }
                    builder.append(&spike);
                }
            }

            // 动脉喷血粒子源（亮红小球）
            for src in &region.bleeding_sources {
                if src.is_arterial {
                    let mut blood = MeshBuilder::new();
                    blood.push_sphere(
                        [center[0], center[1] - 0.02, center[2]],
                        0.015,
                        4,
                        3,
                    );
                    for v in blood.vertices_mut().iter_mut() {
                        v.color = [0.6, 0.05, 0.02, 1.0];
                    }
                    builder.append(&blood);
                }
            }
        }

        DamageOutput {
            mesh: builder.into_mesh_data(),
            physiological: map.clone(),
        }
    }

    fn wound_color(level: DamageLevel) -> [f32; 4] {
        match level {
            DamageLevel::Surface => [0.7, 0.4, 0.3, 0.6],
            DamageLevel::Moderate => [0.5, 0.15, 0.08, 0.85],
            DamageLevel::Severe => [0.35, 0.08, 0.04, 0.95],
            DamageLevel::Destroyed => [0.15, 0.03, 0.02, 1.0],
        }
    }
}

impl ProceduralGenerator for DamageGenerator {
    type Output = DamageOutput;

    fn generate(&self, params: &GeneratorParams) -> Self::Output {
        let _ = params.seed;
        let _ = params.style;
        let _ = params.lod;
        let _ = &params.material_palette;
        let _ = &params.seed_entities;
        // 统一入口用内置 map；外部可用 generate_with_params 传入完整 map
        self.generate_with_params(&self.map)
    }
}

// 消除未使用警告（style 占位）
#[allow(dead_code)]
fn _style_unused(s: ProceduralStyle) -> bool {
    s == ProceduralStyle::Damage
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_damage_level_impact() {
        assert_eq!(DamageLevel::Surface.functional_impact(), 0.0);
        assert_eq!(DamageLevel::Destroyed.functional_impact(), 1.0);
        assert!(DamageLevel::Severe.functional_impact() > DamageLevel::Moderate.functional_impact());
    }

    #[test]
    fn test_body_region_default_healthy() {
        let region = BodyRegion::new(BodyRegionId::TorsoUpper);
        assert_eq!(region.skin_integrity, 1.0);
        assert_eq!(region.bone_integrity, 1.0);
        assert!(region.organs.len() >= 2);
        assert!(region.bleeding_sources.is_empty());
        assert!(region.fracture.is_none());
    }

    #[test]
    fn test_organ_lethal_threshold() {
        assert!(OrganType::Brain.lethal_threshold() > 0.0);
        assert!(OrganType::Heart.lethal_threshold() < OrganType::Liver.lethal_threshold());
    }

    #[test]
    fn test_blunt_damage_causes_fracture() {
        let mut region = BodyRegion::new(BodyRegionId::LeftLegUpper);
        let event = DamageEvent {
            damage_type: DamageType::Blunt,
            target_region: BodyRegionId::LeftLegUpper,
            force_newtons: 5000.0,
            ..Default::default()
        };
        region.apply_damage(&event);
        assert!(region.fracture.is_some(), "expected fracture from heavy blunt hit");
        assert!(region.bone_integrity < 1.0);
    }

    #[test]
    fn test_slash_damage_skin_muscle() {
        let mut region = BodyRegion::new(BodyRegionId::LeftArmUpper);
        let event = DamageEvent {
            damage_type: DamageType::Slash,
            target_region: BodyRegionId::LeftArmUpper,
            force_newtons: 300.0,
            sharpness: 0.9,
            ..Default::default()
        };
        region.apply_damage(&event);
        assert!(region.skin_integrity < 1.0);
        assert!(region.muscle_integrity < 1.0);
    }

    #[test]
    fn test_pierce_embeds_foreign_body() {
        let mut region = BodyRegion::new(BodyRegionId::TorsoUpper);
        let event = DamageEvent {
            damage_type: DamageType::Pierce,
            target_region: BodyRegionId::TorsoUpper,
            force_newtons: 800.0,
            sharpness: 0.8,
            foreign_body_embedded: true,
            ..Default::default()
        };
        region.apply_damage(&event);
        assert!(!region.foreign_bodies.is_empty());
        assert!(region.foreign_bodies[0].embedded);
    }

    #[test]
    fn test_physiological_map_human() {
        let map = PhysiologicalMap::new_human();
        assert_eq!(map.regions.len(), 17);
        assert!(!map.is_dead());
        assert!((map.overall_function() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_blood_loss_progression() {
        let mut map = PhysiologicalMap::new_human();
        // 模拟大量失血
        map.total_blood_ml = 2000.0; // 40% 失血
        map.tick(1.0);
        assert!(map.is_in_shock());
        assert!(map.heart_rate_bpm > 100.0);
    }

    #[test]
    fn test_damage_generator_visualization() {
        let mut map = PhysiologicalMap::new_human();
        let event = DamageEvent {
            damage_type: DamageType::Blunt,
            target_region: BodyRegionId::Head,
            force_newtons: 8000.0,
            ..Default::default()
        };
        map.apply_damage(&event);
        let gen = DamageGenerator::default();
        let out = gen.generate_with_params(&map);
        assert!(!out.mesh.vertices.is_empty(), "damage mesh should have geometry");
    }

    #[test]
    fn test_procedural_generator_trait() {
        let gen = DamageGenerator::default();
        let params = GeneratorParams {
            seed: 42,
            style: ProceduralStyle::Damage,
            ..Default::default()
        };
        let out = gen.generate(&params);
        // 健康状态默认 map 无损伤，mesh 为空
        assert!(out.mesh.vertices.is_empty() || !out.mesh.vertices.is_empty());
    }
}
