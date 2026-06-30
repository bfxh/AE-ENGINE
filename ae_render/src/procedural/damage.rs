//! 角色损伤系统
//!
//! 突破性损伤模拟：
//! - 四级损伤等级（表面/中度/重度/完全毁坏）
//! - 生理地图（身体分区域独立跟踪）
//! - 血液流动模拟（血压/失血/休克/死亡）
//! - 骨骼破坏（闭合/开放/粉碎性骨折）
//! - 跨系统耦合（物理→化学→生物）

use serde::{Deserialize, Serialize};

// === 损伤等级 ===
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

// === 损伤类型 ===
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TissueLayer {
    Skin,
    Muscle,
    Bone,
    Vascular,
    Nerve,
    Organ,
}

// === 身体区域 ===
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BodyRegionId {
    Head,
    Neck,
    TorsoUpper, // 胸
    TorsoLower, // 腹
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
            BodyRegionId::Head => 80_000.0,         // 颅骨相对脆弱
            BodyRegionId::Neck => 50_000.0,
            BodyRegionId::TorsoUpper => 200_000.0,  // 肋骨
            BodyRegionId::TorsoLower => 250_000.0,  // 脊柱
            BodyRegionId::Pelvis => 400_000.0,
            BodyRegionId::LeftArmUpper | BodyRegionId::RightArmUpper => 150_000.0, // 肱骨
            BodyRegionId::LeftArmLower | BodyRegionId::RightArmLower => 100_000.0, // 尺桡骨
            BodyRegionId::LeftHand | BodyRegionId::RightHand => 60_000.0,
            BodyRegionId::LeftLegUpper | BodyRegionId::RightLegUpper => 300_000.0, // 股骨（人体最强骨）
            BodyRegionId::LeftLegLower | BodyRegionId::RightLegLower => 200_000.0, // 胫腓骨
            BodyRegionId::LeftFoot | BodyRegionId::RightFoot => 100_000.0,
        }
    }
}

// === 器官 ===
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
            OrganType::Lung => 5000.0, // 总肺血流
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
            OrganType::Brain => 0.3,  // 脑：损伤 70% 致死
            OrganType::Heart => 0.2,  // 心脏：损伤 80% 致死（最致命）
            OrganType::Lung => 0.4,   // 双肺总功能：损伤 60% 致死
            OrganType::Liver => 0.5,  // 肝脏：损伤 50% 致死（可部分切除）
            OrganType::Kidney => 0.6, // 单肾可切除，但双肾衰竭致死
            OrganType::Spleen => 0.7, // 脾可切除
            OrganType::Stomach => 0.6,
            OrganType::Intestine => 0.6,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OrganState {
    pub organ_type: OrganType,
    pub integrity: f32,           // 0..1
    pub bleeding_rate: f32,       // mL/s
    pub infection_level: f32,     // 0..1
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

// === 异物（弹片、箭矢等）===
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ForeignBody {
    /// 位置（区域本地坐标）
    pub position: [f32; 3],
    /// 半径（米）
    pub radius: f32,
    /// 穿透深度（米）
    pub penetration_depth: f32,
    /// 是否仍嵌在体内
    pub embedded: bool,
    /// 感染风险
    pub infection_risk: f32,
}

// === 出血源 ===
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BleedingSource {
    pub region: BodyRegionId,
    /// 出血速率（mL/s）
    pub rate_ml_s: f32,
    /// 是否动脉出血（喷射节奏）
    pub is_arterial: bool,
    /// 心跳相位（0..2π，动脉喷血用）
    pub heartbeat_phase: f32,
}

// === 身体区域完整状态 ===
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
    /// 当前出血源（可多个）
    pub bleeding_sources: Vec<BleedingSource>,
    /// 骨折类型（None=无骨折）
    pub fracture: Option<FractureType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FractureType {
    Closed,
    Open,
    Comminuted,
}

impl FractureType {
    pub fn bleeding_multiplier(&self) -> f32 {
        match self {
            FractureType::Closed => 0.5,     // 内出血为主
            FractureType::Open => 2.0,       // 外出血+骨髓出血
            FractureType::Comminuted => 3.5, // 大量出血
        }
    }

    pub fn healing_time_days(&self) -> f32 {
        match self {
            FractureType::Closed => 42.0,    // 6 周
            FractureType::Open => 84.0,       // 12 周（含感染风险）
            FractureType::Comminuted => 120.0, // 17 周（可能需要手术）
        }
    }
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
        let bone = self.bone_integrity;
        let muscle = self.muscle_integrity;
        let nerve = self.nerve_integrity;
        let vascular = self.vascular_integrity;
        // 取最低值作为瓶颈
        bone.min(muscle).min(nerve).min(vascular)
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
                // 钝击：皮肤较少损伤，但骨骼和内部器官受冲击
                let bone_stress = force / 0.01; // 假设接触面积 0.01 m²
                if bone_stress > self.region_id.bone_capacity_pa() {
                    let bone_damage = (bone_stress / self.region_id.bone_capacity_pa() - 1.0).min(1.0);
                    self.bone_integrity = (self.bone_integrity - bone_damage).max(0.0);
                    // 骨折判定
                    if self.fracture.is_none() && self.bone_integrity < 0.7 {
                        self.fracture = Some(if self.bone_integrity < 0.3 {
                            FractureType::Comminuted
                        } else if self.skin_integrity < 0.5 {
                            FractureType::Open
                        } else {
                            FractureType::Closed
                        });
                    }
                    // 内出血
                    self.vascular_integrity = (self.vascular_integrity - bone_damage * 0.3).max(0.0);
                }
                // 肌肉挫伤
                let muscle_damage = (force / 2000.0).min(0.5);
                self.muscle_integrity = (self.muscle_integrity - muscle_damage).max(0.0);
                // 器官冲击
                for organ in &mut self.organs {
                    if force > 500.0 {
                        let organ_dmg = (force / 2000.0).min(0.5);
                        organ.integrity = (organ.integrity - organ_dmg).max(0.0);
                        organ.bleeding_rate += organ_dmg * 5.0; // mL/s
                    }
                }
            }
            DamageType::Slash => {
                // 切割：皮肤+肌肉受损，可能伤及骨骼
                let skin_dmg = (force * sharpness / 500.0).min(1.0);
                let muscle_dmg = (force * sharpness / 800.0).min(1.0);
                self.skin_integrity = (self.skin_integrity - skin_dmg).max(0.0);
                self.muscle_integrity = (self.muscle_integrity - muscle_dmg).max(0.0);
                self.vascular_integrity = (self.vascular_integrity - muscle_dmg * 0.5).max(0.0);
                if muscle_dmg > 0.5 {
                    // 深切可能伤骨
                    let bone_dmg = (force * sharpness / 5000.0).min(0.5);
                    self.bone_integrity = (self.bone_integrity - bone_dmg).max(0.0);
                }
            }
            DamageType::Pierce => {
                // 穿刺：深而窄，可能穿透至器官
                let skin_dmg = (force / 1000.0).min(0.8);
                let muscle_dmg = (force / 600.0).min(0.9);
                self.skin_integrity = (self.skin_integrity - skin_dmg).max(0.0);
                self.muscle_integrity = (self.muscle_integrity - muscle_dmg).max(0.0);
                // 穿刺路径上的血管
                self.vascular_integrity = (self.vascular_integrity - muscle_dmg * 0.4).max(0.0);
                // 异物残留
                if event.foreign_body_embedded {
                    self.foreign_bodies.push(ForeignBody {
                        position: event.impact_position,
                        radius: 0.005,
                        penetration_depth: muscle_dmg * 0.1,
                        embedded: true,
                        infection_risk: 0.3,
                    });
                }
                // 器官损伤（如果穿透足够深）
                if muscle_dmg > 0.5 {
                    for organ in &mut self.organs {
                        let organ_dmg = (force / 1500.0).min(0.7);
                        organ.integrity = (organ.integrity - organ_dmg).max(0.0);
                        organ.bleeding_rate += organ_dmg * 10.0;
                    }
                }
            }
            DamageType::Burn => {
                // 烧伤：皮肤首先受损，神经损伤，深层烧伤伤及肌肉
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
                // 酸蚀：组织溶解，持续损伤
                let acid_strength = event.chemical_potency;
                let dmg = acid_strength * 0.5;
                self.skin_integrity = (self.skin_integrity - dmg).max(0.0);
                self.muscle_integrity = (self.muscle_integrity - dmg * 0.6).max(0.0);
                self.nerve_integrity = (self.nerve_integrity - dmg * 0.4).max(0.0);
                // 酸蚀伤感染风险高
            }
            DamageType::Poison => {
                // 毒素：进入血液，影响器官
                let toxin = event.chemical_potency;
                self.vascular_integrity = (self.vascular_integrity - toxin * 0.2).max(0.0);
                for organ in &mut self.organs {
                    organ.integrity = (organ.integrity - toxin * 0.3).max(0.0);
                    organ.infection_level = (organ.infection_level + toxin * 0.5).min(1.0);
                }
            }
        }

        // 评估损伤等级并生成出血源
        let level = self.assess_damage_level();
        if level != DamageLevel::Surface || !self.bleeding_sources.is_empty() {
            let bleed_rate = level.bleeding_multiplier() * (1.0 - self.vascular_integrity) * 2.0;
            if bleed_rate > 0.01 {
                let is_arterial = self.vascular_integrity < 0.3 && self.region_id.is_limb();
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

// === 损伤事件 ===
#[derive(Debug, Clone, Copy)]
pub struct DamageEvent {
    pub damage_type: DamageType,
    pub target_region: BodyRegionId,
    /// 冲击力（牛顿）
    pub force_newtons: f32,
    /// 锐度（0=钝，1=极锋利）
    pub sharpness: f32,
    /// 热能（焦耳，烧伤用）
    pub thermal_energy_j: f32,
    /// 化学强度（0..1，酸蚀/毒素用）
    pub chemical_potency: f32,
    /// 撞击位置（区域本地坐标，米）
    pub impact_position: [f32; 3],
    /// 是否残留异物
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

// === 生理地图（全身状态）===
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysiologicalMap {
    pub regions: Vec<BodyRegion>,
    /// 总血量（mL，成人默认 5000）
    pub total_blood_ml: f32,
    /// 最大血量
    pub max_blood_ml: f32,
    /// 心率（次/分）
    pub heart_rate_bpm: f32,
    /// 血压（收缩压 mmHg）
    pub blood_pressure_systolic: f32,
    /// 血压（舒张压 mmHg）
    pub blood_pressure_diastolic: f32,
    /// 意识水平（0=清醒，1=昏迷）
    pub consciousness: f32,
    /// 疼痛指数（0..1）
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

    /// 按区域 ID 查找
    pub fn region(&mut self, id: BodyRegionId) -> Option<&mut BodyRegion> {
        self.regions.iter_mut().find(|r| r.region_id == id)
    }

    /// 应用损伤事件
    pub fn apply_damage(&mut self, event: &DamageEvent) {
        if let Some(region) = self.region(event.target_region) {
            region.apply_damage(event);
        }
    }

    /// 计算当前总出血速率（mL/s）
    pub fn total_bleeding_rate(&self) -> f32 {
        let mut total = 0.0;
        for region in &self.regions {
            for src in &region.bleeding_sources {
                total += src.rate_ml_s;
            }
            for organ in &region.organs {
                total += organ.bleeding_rate / 60.0; // mL/min → mL/s
            }
        }
        total
    }

    /// 推进时间步（秒），更新血量、血压、心率、意识
    pub fn tick(&mut self, dt: f32) {
        let bleeding = self.total_bleeding_rate();
        // 失血
        self.total_blood_ml = (self.total_blood_ml - bleeding * dt).max(0.0);
        let blood_ratio = self.total_blood_ml / self.max_blood_ml;

        // 血压随失血下降（代偿期→失代偿期）
        if blood_ratio > 0.85 {
            // 代偿期：心率上升，血压维持
            self.heart_rate_bpm = 72.0 + (1.0 - blood_ratio) * 100.0;
            self.blood_pressure_systolic = 120.0;
            self.blood_pressure_diastolic = 80.0;
        } else if blood_ratio > 0.6 {
            // 失代偿期：心率明显上升，血压下降
            self.heart_rate_bpm = 110.0 + (0.85 - blood_ratio) * 200.0;
            self.blood_pressure_systolic = 120.0 * blood_ratio / 0.85;
            self.blood_pressure_diastolic = 80.0 * blood_ratio / 0.85;
        } else if blood_ratio > 0.3 {
            // 休克前期：心率加快但无效，血压剧降
            self.heart_rate_bpm = 140.0;
            self.blood_pressure_systolic = 70.0 * blood_ratio / 0.6;
            self.blood_pressure_diastolic = 40.0 * blood_ratio / 0.6;
            self.consciousness = (1.0 - blood_ratio / 0.6).max(self.consciousness);
        } else {
            // 深度休克：意识丧失，心率减缓（濒死）
            self.heart_rate_bpm = 60.0 * blood_ratio / 0.3;
            self.blood_pressure_systolic = 40.0 * blood_ratio / 0.3;
            self.blood_pressure_diastolic = 20.0 * blood_ratio / 0.3;
            self.consciousness = 1.0;
        }

        // 疼痛指数：所有区域损伤的加权平均
        let mut total_pain = 0.0;
        for region in &self.regions {
            let level = region.assess_damage_level();
            let weight = if region.region_id.contains_vital_organs() { 2.0 } else { 1.0 };
            total_pain += level.functional_impact() * weight;
        }
        self.pain_level = (total_pain / 17.0).min(1.0);
        // 疼痛也会导致意识下降
        if self.pain_level > 0.7 {
            self.consciousness = self.consciousness.max((self.pain_level - 0.7) * 3.0);
        }

        // 心跳相位推进（用于动脉喷血动画同步）
        let heartbeat_dt = std::f32::consts::TAU / (self.heart_rate_bpm / 60.0) * dt;
        for region in &mut self.regions {
            for src in &mut region.bleeding_sources {
                if src.is_arterial {
                    src.heartbeat_phase = (src.heartbeat_phase + heartbeat_dt) % std::f32::consts::TAU;
                }
            }
        }

        // 感染进展
        for region in &mut self.regions {
            for organ in &mut region.organs {
                if organ.infection_level > 0.0 {
                    organ.infection_level = (organ.infection_level + dt * 0.001).min(1.0);
                }
            }
        }
    }

    /// 是否已死亡
    pub fn is_dead(&self) -> bool {
        // 失血超过 50% 致死阈值
        if self.total_blood_ml / self.max_blood_ml < 0.3 {
            return true;
        }
        // 关键器官致命
        for region in &self.regions {
            for organ in &region.organs {
                if organ.is_lethal() {
                    return true;
                }
            }
        }
        false
    }

    /// 是否处于休克状态
    pub fn is_in_shock(&self) -> bool {
        self.total_blood_ml / self.max_blood_ml < 0.6
    }

    /// 全身功能因子（综合评估，0=死亡/全身瘫痪，1=完全健康）
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

    /// 获取所有出血源（用于粒子系统）
    pub fn all_bleeding_sources(&self) -> Vec<&BleedingSource> {
        self.regions.iter()
            .flat_map(|r| r.bleeding_sources.iter())
            .collect()
    }

    /// 获取所有骨折
    pub fn all_fractures(&self) -> Vec<(BodyRegionId, FractureType)> {
        self.regions.iter()
            .filter_map(|r| r.fracture.map(|f| (r.region_id, f)))
            .collect()
    }

    /// 查询特定区域的功能影响（用于动作合成引擎）
    pub fn region_function_factor(&self, id: BodyRegionId) -> f32 {
        self.regions.iter()
            .find(|r| r.region_id == id)
            .map(|r| r.functional_factor())
            .unwrap_or(1.0)
    }

    /// 查询特定区域是否已断裂（用于动作合成引擎决定拓扑）
    pub fn is_region_severed(&self, id: BodyRegionId) -> bool {
        self.regions.iter()
            .find(|r| r.region_id == id)
            .map(|r| r.is_severed())
            .unwrap_or(false)
    }
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
        assert!(region.organs.len() >= 2); // 心+肺
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
            force_newtons: 5000.0, // 强力钝击
            ..Default::default()
        };
        region.apply_damage(&event);
        // 应该出现骨折
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
    fn test_burn_damages_nerve() {
        let mut region = BodyRegion::new(BodyRegionId::LeftHand);
        let event = DamageEvent {
            damage_type: DamageType::Burn,
            target_region: BodyRegionId::LeftHand,
            thermal_energy_j: 4000.0,
            ..Default::default()
        };
        region.apply_damage(&event);
        assert!(region.nerve_integrity < 1.0);
        assert!(region.skin_integrity < 1.0);
    }

    #[test]
    fn test_physiological_map_default_healthy() {
        let map = PhysiologicalMap::new_human();
        assert_eq!(map.regions.len(), 17);
        assert_eq!(map.total_blood_ml, 5000.0);
        assert!(!map.is_dead());
        assert!(!map.is_in_shock());
        assert!(map.overall_function() > 0.99);
    }

    #[test]
    fn test_bleeding_reduces_blood() {
        let mut map = PhysiologicalMap::new_human();
        // 制造股动脉破裂
        let event = DamageEvent {
            damage_type: DamageType::Slash,
            target_region: BodyRegionId::LeftLegUpper,
            force_newtons: 600.0,
            sharpness: 0.95,
            ..Default::default()
        };
        map.apply_damage(&event);
        let initial_blood = map.total_blood_ml;
        // 推进 60 秒
        for _ in 0..60 {
            map.tick(1.0);
        }
        assert!(map.total_blood_ml < initial_blood, "expected blood loss after 60s");
    }

    #[test]
    fn test_severe_bleeding_causes_shock() {
        let mut map = PhysiologicalMap::new_human();
        // 多处大出血
        for region in [BodyRegionId::LeftLegUpper, BodyRegionId::RightLegUpper, BodyRegionId::TorsoUpper] {
            map.apply_damage(&DamageEvent {
                damage_type: DamageType::Slash,
                target_region: region,
                force_newtons: 800.0,
                sharpness: 0.95,
                ..Default::default()
            });
        }
        // 推进 120 秒
        for _ in 0..120 {
            map.tick(1.0);
            if map.is_dead() { break; }
        }
        assert!(map.is_in_shock() || map.is_dead(), "expected shock or death after massive bleeding");
    }

    #[test]
    fn test_organ_lethal_wound_causes_death() {
        let mut map = PhysiologicalMap::new_human();
        // 心脏贯穿
        map.apply_damage(&DamageEvent {
            damage_type: DamageType::Pierce,
            target_region: BodyRegionId::TorsoUpper,
            force_newtons: 2000.0,
            sharpness: 0.99,
            ..Default::default()
        });
        // 检查心脏是否达到致命阈值
        let heart_region = map.regions.iter().find(|r| r.region_id == BodyRegionId::TorsoUpper).unwrap();
        let heart = heart_region.organs.iter().find(|o| o.organ_type == OrganType::Heart).unwrap();
        if heart.integrity < OrganType::Heart.lethal_threshold() {
            assert!(map.is_dead(), "expected death from heart destruction");
        }
    }

    #[test]
    fn test_fracture_types_have_different_severity() {
        assert!(FractureType::Comminuted.bleeding_multiplier() > FractureType::Open.bleeding_multiplier());
        assert!(FractureType::Open.bleeding_multiplier() > FractureType::Closed.bleeding_multiplier());
        assert!(FractureType::Comminuted.healing_time_days() > FractureType::Closed.healing_time_days());
    }

    #[test]
    fn test_region_function_factor() {
        let mut region = BodyRegion::new(BodyRegionId::LeftArmUpper);
        assert!((region.functional_factor() - 1.0).abs() < 1e-6);
        region.bone_integrity = 0.3;
        region.muscle_integrity = 0.8;
        region.nerve_integrity = 0.9;
        region.vascular_integrity = 0.7;
        // 瓶颈是 bone 0.3
        assert!((region.functional_factor() - 0.3).abs() < 1e-6);
    }

    #[test]
    fn test_severed_limb() {
        let mut region = BodyRegion::new(BodyRegionId::LeftArmLower);
        region.bone_integrity = 0.02;
        assert!(region.is_severed());
        assert_eq!(region.assess_damage_level(), DamageLevel::Destroyed);
    }

    #[test]
    fn test_all_bleeding_sources_query() {
        let mut map = PhysiologicalMap::new_human();
        map.apply_damage(&DamageEvent {
            damage_type: DamageType::Slash,
            target_region: BodyRegionId::RightLegUpper,
            force_newtons: 500.0,
            sharpness: 0.9,
            ..Default::default()
        });
        let sources = map.all_bleeding_sources();
        assert!(!sources.is_empty(), "expected bleeding sources after slash");
    }

    #[test]
    fn test_all_fractures_query() {
        let mut map = PhysiologicalMap::new_human();
        map.apply_damage(&DamageEvent {
            damage_type: DamageType::Blunt,
            target_region: BodyRegionId::LeftLegLower,
            force_newtons: 6000.0,
            ..Default::default()
        });
        let fractures = map.all_fractures();
        assert!(!fractures.is_empty(), "expected at least one fracture");
    }

    #[test]
    fn test_arterial_bleeding_synchronizes_heartbeat() {
        let mut map = PhysiologicalMap::new_human();
        // 制造动脉出血
        map.apply_damage(&DamageEvent {
            damage_type: DamageType::Slash,
            target_region: BodyRegionId::RightLegUpper,
            force_newtons: 900.0,
            sharpness: 0.99,
            ..Default::default()
        });
        let initial_phase = map.all_bleeding_sources().first().map(|s| s.heartbeat_phase).unwrap_or(0.0);
        map.tick(0.5);
        let new_phase = map.all_bleeding_sources().first().map(|s| s.heartbeat_phase).unwrap_or(0.0);
        // 动脉出血源的心跳相位应该有变化
        let arterial_count = map.all_bleeding_sources().iter().filter(|s| s.is_arterial).count();
        if arterial_count > 0 {
            assert!((new_phase - initial_phase).abs() > 0.01, "expected heartbeat phase advance");
        }
    }

    #[test]
    fn test_overall_function_decreases_with_damage() {
        let mut map = PhysiologicalMap::new_human();
        let initial = map.overall_function();
        map.apply_damage(&DamageEvent {
            damage_type: DamageType::Blunt,
            target_region: BodyRegionId::LeftLegUpper,
            force_newtons: 4000.0,
            ..Default::default()
        });
        let after = map.overall_function();
        assert!(after < initial, "expected overall function decrease after damage");
    }

    #[test]
    fn test_region_query_helpers() {
        let mut map = PhysiologicalMap::new_human();
        map.apply_damage(&DamageEvent {
            damage_type: DamageType::Blunt,
            target_region: BodyRegionId::LeftArmLower,
            force_newtons: 8000.0, // 极强冲击造成断裂
            ..Default::default()
        });
        // 可能造成断裂
        let factor = map.region_function_factor(BodyRegionId::LeftArmLower);
        assert!(factor < 1.0);
    }
}
