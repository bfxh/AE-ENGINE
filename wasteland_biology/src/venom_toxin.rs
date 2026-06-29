//! 毒液/毒素系统 —— 基于真实毒理学研究
//!
//! 数据来源：
//! - WHO Guidelines for the Production, Control and Regulation of Snake Antivenom Immunoglobulins
//! - Fry BG et al. "The Toxicogenomic Multiverse: Convergent Recruitment of Proteins into Animal Venoms" (2009)
//! - Mebs D. "Venomous and Poisonous Animals" (2002)
//! - Kini RM. "Snake Venom Phospholipase A2 Enzymes" (1997)
//! - Lewis RJ & Garcia ML. "Therapeutic potential of venom peptides" (2003) Nat Rev Drug Discov
//! - King GF. "The Wonderful World of Spiders" (2004) Toxicon
//! - Possani LD et al. "Scorpion venom components and their antagonistic properties" (2000)
//! - Casewell NR et al. "Medically important differences in snake venom composition" (2014)
//! - FDA / 厂商标签（CroFab, BabyBIG, Antivipmyn, CSL）

use serde::{Deserialize, Serialize};

// ============ 1. 毒素类型分类 ============

/// 毒液大类
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VenomType {
    /// 神经毒素
    Neurotoxin,
    /// 血液毒素
    Hemotoxin,
    /// 细胞毒素
    Cytotoxin,
    /// 心脏毒素
    Cardiotoxin,
    /// 肌肉毒素
    Myotoxin,
    /// 肾毒素
    Nephrotoxin,
    /// 肝毒素
    Hepatotoxin,
    /// 出血毒素
    Hemorrhagin,
    /// 坏死毒素
    Necrotoxin,
    /// 光敏毒素
    Phototoxin,
}

/// 毒素生物来源
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToxinSource {
    /// 眼镜蛇科（神经毒为主）
    SnakeElapidae,
    /// 蝰蛇科（血液毒为主）
    SnakeViperidae,
    /// 游蛇科（部分有毒）
    SnakeColubridae,
    /// 蝎子
    Scorpion,
    /// 蜘蛛
    Spider,
    /// 水母
    Jellyfish,
    /// 芋螺
    ConeSnail,
    /// 章鱼（蓝环）
    Octopus,
    /// 头足类
    Cephalopod,
    /// 蜜蜂
    Bee,
    /// 黄蜂
    Wasp,
    /// 蚂蚁
    Ant,
    /// 蟾蜍
    Toad,
    /// 箭毒蛙
    Frog,
    /// 河豚/石鱼
    Fish,
    /// 刺胞动物
    Cnidarian,
    /// 棘皮动物（海胆）
    Echinoderm,
    /// 千足虫
    Millipede,
    /// 蜈蚣
    Centipede,
    /// 希拉毒蜥
    Lizard,
    /// 植物（蓖麻毒素）
    Plant,
    /// 细菌（肉毒杆菌、破伤风、霍乱）
    Bacteria,
    /// 真菌（鹅膏毒素）
    Fungi,
    /// 甲藻（赤潮毒素）
    Dinoflagellate,
}

/// 毒理学作用机制
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToxinMechanism {
    // ---- 神经毒机制 ----
    /// 钠通道激活（蝎毒、箭毒蛙 batrachotoxin、乌头碱）
    SodiumChannelActivator,
    /// 钠通道阻断（河豚毒素 TTX、石房蛤毒素 STX）
    SodiumChannelBlocker,
    /// 钾通道阻断（蜂毒 apamin、蝎毒 charbdotoxin）
    PotassiumChannelBlocker,
    /// 钙通道阻断（芋螺 ω-conotoxin）
    CalciumChannelBlocker,
    /// 乙酰胆碱受体（眼镜蛇 α-cobratoxin、箭毒蛙）
    AcetylcholineReceptor,
    /// 胆碱酯酶抑制（有机磷、Fasciculin）
    AcetylcholinesteraseInhibitor,
    /// 突触前剪切（肉毒杆菌 BoNT、破伤风 TeNT、α-latrotoxin）
    PresynapticCleaver,
    // ---- 血液毒机制 ----
    /// 凝血激活（锯鳞蝰 Echis、Russell 蝰 RVV-X）
    ClottingActivator,
    /// 抗凝（医用水蛭 hirudin）
    ClottingInhibitor,
    /// 血小板抑制（Echistatin RGD）
    PlateletInhibitor,
    /// 溶血（蜂毒 mellitin）
    Hemolysis,
    /// 出血（蝰蛇金属蛋白酶）
    Hemorrhagic,
    // ---- 细胞毒机制 ----
    /// 形成孔道（蜂毒 mellitin、葡萄球菌 α-toxin）
    PoreFormer,
    /// 膜破坏
    MembraneDisruptor,
    /// 核糖体失活（蓖麻毒素 ricin、Shiga）
    RibosomeInactivator,
    /// 凋亡诱导
    ApoptosisInducer,
    /// DNA 嵌入
    DNAIntercalator,
    /// 蛋白合成抑制（α-amanitin 鹅膏，RNA Pol II）
    ProteinSynthesisInhibitor,
}

/// 毒素化学分类
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToxinChemicalClass {
    /// 小分子（河豚毒素、蟾毒素）
    SmallMolecule,
    /// 多肽（蜂毒肽、芋螺肽）
    Peptide,
    /// 蛋白质（酶、蛇毒金属蛋白酶）
    Protein,
    /// 酶（磷脂酶 A2、透明质酸酶）
    Enzyme,
    /// 生物碱（箭毒蛙 batrachotoxin）
    Alkaloid,
    /// 糖苷
    Glycoside,
    /// 聚酮（黄曲霉毒素、雪卡毒素）
    Polyketide,
    /// 氨基多元醇
    Aminopolyol,
}

// ============ 2. 毒素分子 ============

/// 单个毒素分子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Toxin {
    /// 毒素名称
    pub name: String,
    /// 生物来源
    pub source: ToxinSource,
    /// 毒液大类
    pub venom_type: VenomType,
    /// 作用机制
    pub mechanism: ToxinMechanism,
    /// 分子量（道尔顿）
    pub molecular_weight_da: f32,
    /// 半数致死量（小鼠静脉，mg/kg）
    pub ld50_mg_per_kg: f32,
    /// 单次分泌量（mg）
    pub yield_mg: f32,
    /// 起效时间（分钟）
    pub onset_minutes: f32,
    /// 持续时间（小时）
    pub duration_hours: f32,
    /// 选择性 0.0-1.0
    pub specificity: f32,
    /// 化学分类
    pub chemical_class: ToxinChemicalClass,
}

// ============ 4. 毒作用动力学 ============

/// 毒作用效应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToxinEffect {
    /// 靶系统
    pub target_system: PhysiologicalSystem,
    /// 效应类型
    pub effect_type: EffectType,
    /// 严重度 0.0-1.0
    pub severity: f32,
    /// 是否可逆
    pub reversible: bool,
    /// 恢复时间（小时）
    pub recovery_time_hours: f32,
}

/// 生理系统
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PhysiologicalSystem {
    /// 神经系统
    NervousSystem,
    /// 心血管
    Cardiovascular,
    /// 呼吸
    Respiratory,
    /// 肌肉
    Muscular,
    /// 肾
    Renal,
    /// 肝
    Hepatic,
    /// 血液
    Hematologic,
    /// 胃肠
    Gastrointestinal,
    /// 皮肤
    Dermatologic,
    /// 眼
    Ocular,
    /// 免疫
    Immune,
    /// 内分泌
    Endocrine,
}

/// 效应类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EffectType {
    /// 麻痹
    Paralysis,
    /// 痉挛
    Spasm,
    /// 坏死
    Necrosis,
    /// 溶血
    Hemolysis,
    /// 出血
    Bleeding,
    /// 凝血/DIC
    Clotting,
    /// 低血压
    Hypotension,
    /// 高血压
    Hypertension,
    /// 心律失常
    Arrhythmia,
    /// 呼吸衰竭
    RespiratoryFailure,
    /// 惊厥
    Convulsion,
    /// 水肿
    Edema,
    /// 疼痛
    Pain,
    /// 炎症
    Inflammation,
}

/// 毒性等级（基于 LD50，小鼠静脉 mg/kg）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToxicityRank {
    /// >1000 mg/kg
    RelativelyHarmless,
    /// 500-1000 mg/kg
    SlightlyToxic,
    /// 50-500 mg/kg
    ModeratelyToxic,
    /// 1-50 mg/kg
    HighlyToxic,
    /// <1 mg/kg
    Supertoxic,
    /// <0.001 mg/kg (BoNT)
    ExtremelyToxic,
}

impl Toxin {
    /// 根据机制与毒液类型推演毒作用效应
    ///
    /// 依据毒理学作用机制（mechanism）映射到靶生理系统与效应类型，
    /// 严重度由 LD50 推算（<0.05 mg/kg 视为重度 0.9，否则 0.6）。
    pub fn effects(&self) -> Vec<ToxinEffect> {
        let mut out: Vec<ToxinEffect> = Vec::new();
        let severe = self.ld50_mg_per_kg < 0.05;
        let sev = if severe { 0.9 } else { 0.6 };
        let dur = self.duration_hours;

        match self.mechanism {
            ToxinMechanism::SodiumChannelActivator => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::NervousSystem,
                    effect_type: EffectType::Convulsion,
                    severity: sev,
                    reversible: dur < 24.0,
                    recovery_time_hours: dur,
                });
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Cardiovascular,
                    effect_type: EffectType::Arrhythmia,
                    severity: sev * 0.8,
                    reversible: true,
                    recovery_time_hours: dur,
                });
            }
            ToxinMechanism::SodiumChannelBlocker => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::NervousSystem,
                    effect_type: EffectType::Paralysis,
                    severity: sev,
                    reversible: self.specificity > 0.5,
                    recovery_time_hours: dur,
                });
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Respiratory,
                    effect_type: EffectType::RespiratoryFailure,
                    severity: sev * 0.9,
                    reversible: true,
                    recovery_time_hours: dur,
                });
            }
            ToxinMechanism::PotassiumChannelBlocker => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::NervousSystem,
                    effect_type: EffectType::Spasm,
                    severity: sev,
                    reversible: true,
                    recovery_time_hours: dur,
                });
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Cardiovascular,
                    effect_type: EffectType::Arrhythmia,
                    severity: sev * 0.7,
                    reversible: true,
                    recovery_time_hours: dur,
                });
            }
            ToxinMechanism::CalciumChannelBlocker => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::NervousSystem,
                    effect_type: EffectType::Paralysis,
                    severity: sev,
                    reversible: true,
                    recovery_time_hours: dur,
                });
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Cardiovascular,
                    effect_type: EffectType::Hypotension,
                    severity: sev * 0.6,
                    reversible: true,
                    recovery_time_hours: dur,
                });
            }
            ToxinMechanism::AcetylcholineReceptor => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::NervousSystem,
                    effect_type: EffectType::Paralysis,
                    severity: sev,
                    reversible: self.specificity < 0.7,
                    recovery_time_hours: dur,
                });
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Respiratory,
                    effect_type: EffectType::RespiratoryFailure,
                    severity: sev * 0.9,
                    reversible: self.specificity < 0.7,
                    recovery_time_hours: dur,
                });
            }
            ToxinMechanism::AcetylcholinesteraseInhibitor => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::NervousSystem,
                    effect_type: EffectType::Spasm,
                    severity: sev,
                    reversible: true,
                    recovery_time_hours: dur,
                });
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Respiratory,
                    effect_type: EffectType::RespiratoryFailure,
                    severity: sev * 0.7,
                    reversible: true,
                    recovery_time_hours: dur,
                });
            }
            ToxinMechanism::PresynapticCleaver => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::NervousSystem,
                    effect_type: EffectType::Paralysis,
                    severity: sev,
                    reversible: false,
                    recovery_time_hours: dur * 4.0,
                });
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Respiratory,
                    effect_type: EffectType::RespiratoryFailure,
                    severity: sev * 0.95,
                    reversible: false,
                    recovery_time_hours: dur * 4.0,
                });
            }
            ToxinMechanism::ClottingActivator => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Hematologic,
                    effect_type: EffectType::Clotting,
                    severity: sev,
                    reversible: false,
                    recovery_time_hours: dur,
                });
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Renal,
                    effect_type: EffectType::Necrosis,
                    severity: sev * 0.5,
                    reversible: false,
                    recovery_time_hours: dur * 2.0,
                });
            }
            ToxinMechanism::ClottingInhibitor => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Hematologic,
                    effect_type: EffectType::Bleeding,
                    severity: sev,
                    reversible: true,
                    recovery_time_hours: dur,
                });
            }
            ToxinMechanism::PlateletInhibitor => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Hematologic,
                    effect_type: EffectType::Bleeding,
                    severity: sev * 0.6,
                    reversible: true,
                    recovery_time_hours: dur,
                });
            }
            ToxinMechanism::Hemolysis => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Hematologic,
                    effect_type: EffectType::Hemolysis,
                    severity: sev,
                    reversible: true,
                    recovery_time_hours: dur,
                });
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Renal,
                    effect_type: EffectType::Necrosis,
                    severity: sev * 0.4,
                    reversible: true,
                    recovery_time_hours: dur * 2.0,
                });
            }
            ToxinMechanism::Hemorrhagic => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Hematologic,
                    effect_type: EffectType::Bleeding,
                    severity: sev,
                    reversible: false,
                    recovery_time_hours: dur * 2.0,
                });
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Dermatologic,
                    effect_type: EffectType::Necrosis,
                    severity: sev * 0.6,
                    reversible: false,
                    recovery_time_hours: dur * 3.0,
                });
            }
            ToxinMechanism::PoreFormer => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Dermatologic,
                    effect_type: EffectType::Necrosis,
                    severity: sev,
                    reversible: false,
                    recovery_time_hours: dur * 2.0,
                });
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Hematologic,
                    effect_type: EffectType::Hemolysis,
                    severity: sev * 0.7,
                    reversible: true,
                    recovery_time_hours: dur,
                });
            }
            ToxinMechanism::MembraneDisruptor => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Dermatologic,
                    effect_type: EffectType::Necrosis,
                    severity: sev,
                    reversible: false,
                    recovery_time_hours: dur * 2.0,
                });
            }
            ToxinMechanism::RibosomeInactivator => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Gastrointestinal,
                    effect_type: EffectType::Necrosis,
                    severity: sev,
                    reversible: false,
                    recovery_time_hours: dur * 4.0,
                });
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Hepatic,
                    effect_type: EffectType::Necrosis,
                    severity: sev * 0.6,
                    reversible: false,
                    recovery_time_hours: dur * 4.0,
                });
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Renal,
                    effect_type: EffectType::Necrosis,
                    severity: sev * 0.7,
                    reversible: false,
                    recovery_time_hours: dur * 4.0,
                });
            }
            ToxinMechanism::ApoptosisInducer => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Hepatic,
                    effect_type: EffectType::Necrosis,
                    severity: sev,
                    reversible: false,
                    recovery_time_hours: dur * 3.0,
                });
            }
            ToxinMechanism::DNAIntercalator => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Hematologic,
                    effect_type: EffectType::Necrosis,
                    severity: sev,
                    reversible: false,
                    recovery_time_hours: dur * 4.0,
                });
            }
            ToxinMechanism::ProteinSynthesisInhibitor => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Hepatic,
                    effect_type: EffectType::Necrosis,
                    severity: sev,
                    reversible: false,
                    recovery_time_hours: dur * 4.0,
                });
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Renal,
                    effect_type: EffectType::Necrosis,
                    severity: sev * 0.7,
                    reversible: false,
                    recovery_time_hours: dur * 4.0,
                });
            }
        }

        // 根据 VenomType 二次补充效应
        match self.venom_type {
            VenomType::Cardiotoxin => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Cardiovascular,
                    effect_type: EffectType::Arrhythmia,
                    severity: sev,
                    reversible: true,
                    recovery_time_hours: dur,
                });
            }
            VenomType::Myotoxin => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Muscular,
                    effect_type: EffectType::Necrosis,
                    severity: sev,
                    reversible: false,
                    recovery_time_hours: dur * 3.0,
                });
            }
            VenomType::Nephrotoxin => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Renal,
                    effect_type: EffectType::Necrosis,
                    severity: sev,
                    reversible: false,
                    recovery_time_hours: dur * 3.0,
                });
            }
            VenomType::Hepatotoxin => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Hepatic,
                    effect_type: EffectType::Necrosis,
                    severity: sev,
                    reversible: false,
                    recovery_time_hours: dur * 3.0,
                });
            }
            VenomType::Phototoxin => {
                out.push(ToxinEffect {
                    target_system: PhysiologicalSystem::Dermatologic,
                    effect_type: EffectType::Necrosis,
                    severity: sev * 0.7,
                    reversible: true,
                    recovery_time_hours: dur,
                });
            }
            _ => {}
        }

        out
    }

    /// 计算 70 kg 成人的半数致死量（mg）
    pub fn lethal_dose_for_70kg(&self) -> f32 {
        self.ld50_mg_per_kg * 70.0
    }

    /// 基于 LD50（小鼠静脉 mg/kg）的毒性分级
    pub fn comparative_toxicity(&self) -> ToxicityRank {
        let l = self.ld50_mg_per_kg;
        if l < 0.001 {
            ToxicityRank::ExtremelyToxic
        } else if l < 1.0 {
            ToxicityRank::Supertoxic
        } else if l < 50.0 {
            ToxicityRank::HighlyToxic
        } else if l < 500.0 {
            ToxicityRank::ModeratelyToxic
        } else if l < 1000.0 {
            ToxicityRank::SlightlyToxic
        } else {
            ToxicityRank::RelativelyHarmless
        }
    }
}

// ============ 3. 真实毒素数据库 ============

/// 蛇毒数据库
///
/// 来源：WHO 蛇毒指南 + Kini RM. "Snake Venom Phospholipase A2 Enzymes"
pub fn snake_venom_database() -> Vec<Toxin> {
    vec![
        // ---- 眼镜蛇科 Elapidae ----
        // α-Cobratoxin: 眼镜蛇 Naja kaouthia，长链 α-neurotoxin，71 aa，不可逆结合 nAChR
        Toxin {
            name: "α-Cobratoxin".into(),
            source: ToxinSource::SnakeElapidae,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::AcetylcholineReceptor,
            molecular_weight_da: 7820.0,
            ld50_mg_per_kg: 0.4,
            yield_mg: 250.0,
            onset_minutes: 30.0,
            duration_hours: 48.0,
            specificity: 0.85,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // α-Bungarotoxin: 银环蛇 Bungarus multicinctus，74 aa，不可逆 nAChR 阻断
        Toxin {
            name: "α-Bungarotoxin".into(),
            source: ToxinSource::SnakeElapidae,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::AcetylcholineReceptor,
            molecular_weight_da: 7984.0,
            ld50_mg_per_kg: 0.15,
            yield_mg: 100.0,
            onset_minutes: 45.0,
            duration_hours: 72.0,
            specificity: 0.9,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // Taipoxin: 内陆太攀蛇 Oxyuranus microlepidotus，最毒陆蛇之一，PLA2 复合体
        Toxin {
            name: "Taipoxin".into(),
            source: ToxinSource::SnakeElapidae,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::PresynapticCleaver,
            molecular_weight_da: 45800.0,
            ld50_mg_per_kg: 0.025,
            yield_mg: 44.0,
            onset_minutes: 60.0,
            duration_hours: 96.0,
            specificity: 0.75,
            chemical_class: ToxinChemicalClass::Protein,
        },
        // Fasciculin-2: 绿曼巴 Dendroaspis angusticeps，AChE 抑制
        Toxin {
            name: "Fasciculin-2".into(),
            source: ToxinSource::SnakeElapidae,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::AcetylcholinesteraseInhibitor,
            molecular_weight_da: 6736.0,
            ld50_mg_per_kg: 0.25,
            yield_mg: 60.0,
            onset_minutes: 20.0,
            duration_hours: 24.0,
            specificity: 0.8,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // ---- 蝰蛇科 Viperidae ----
        // Crotoxin: 南美响尾蛇 Crotalus durissus terrificus，PLA2 复合体
        Toxin {
            name: "Crotoxin".into(),
            source: ToxinSource::SnakeViperidae,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::PresynapticCleaver,
            molecular_weight_da: 23000.0,
            ld50_mg_per_kg: 0.07,
            yield_mg: 80.0,
            onset_minutes: 90.0,
            duration_hours: 72.0,
            specificity: 0.7,
            chemical_class: ToxinChemicalClass::Enzyme,
        },
        // Echistatin: 锯鳞蝰 Echis carinatus，RGD 三肽，血小板 GPIIb/IIIa 拮抗
        Toxin {
            name: "Echistatin".into(),
            source: ToxinSource::SnakeViperidae,
            venom_type: VenomType::Hemotoxin,
            mechanism: ToxinMechanism::PlateletInhibitor,
            molecular_weight_da: 5200.0,
            ld50_mg_per_kg: 5.0,
            yield_mg: 200.0,
            onset_minutes: 30.0,
            duration_hours: 12.0,
            specificity: 0.85,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // Hemorrhagin: 蝰蛇金属蛋白酶 SVMP
        Toxin {
            name: "Hemorrhagin".into(),
            source: ToxinSource::SnakeViperidae,
            venom_type: VenomType::Hemorrhagin,
            mechanism: ToxinMechanism::Hemorrhagic,
            molecular_weight_da: 50000.0,
            ld50_mg_per_kg: 1.0,
            yield_mg: 300.0,
            onset_minutes: 30.0,
            duration_hours: 48.0,
            specificity: 0.5,
            chemical_class: ToxinChemicalClass::Enzyme,
        },
        // RVV-X: Russell 蝰 Daboia russelii，因子 X 激活，触发 DIC
        Toxin {
            name: "RVV-X".into(),
            source: ToxinSource::SnakeViperidae,
            venom_type: VenomType::Hemotoxin,
            mechanism: ToxinMechanism::ClottingActivator,
            molecular_weight_da: 79000.0,
            ld50_mg_per_kg: 0.08,
            yield_mg: 200.0,
            onset_minutes: 30.0,
            duration_hours: 24.0,
            specificity: 0.6,
            chemical_class: ToxinChemicalClass::Enzyme,
        },
        // Daboia russelii 全毒
        Toxin {
            name: "Daboia russelii venom".into(),
            source: ToxinSource::SnakeViperidae,
            venom_type: VenomType::Hemotoxin,
            mechanism: ToxinMechanism::ClottingActivator,
            molecular_weight_da: 50000.0,
            ld50_mg_per_kg: 0.08,
            yield_mg: 250.0,
            onset_minutes: 30.0,
            duration_hours: 48.0,
            specificity: 0.5,
            chemical_class: ToxinChemicalClass::Protein,
        },
    ]
}

/// 蝎毒数据库
///
/// 来源：Possani LD et al. "Scorpion venom components" (2000)
pub fn scorpion_venom_database() -> Vec<Toxin> {
    vec![
        // α-Scorpion toxin: Leiurus quinquestriatus，钠通道 site-3，门控修饰
        Toxin {
            name: "α-Scorpion toxin".into(),
            source: ToxinSource::Scorpion,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::SodiumChannelActivator,
            molecular_weight_da: 7000.0,
            ld50_mg_per_kg: 0.25,
            yield_mg: 0.5,
            onset_minutes: 15.0,
            duration_hours: 24.0,
            specificity: 0.85,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // β-Scorpion toxin: Centruroides sculpturatus，钠通道 site-4
        Toxin {
            name: "β-Scorpion toxin".into(),
            source: ToxinSource::Scorpion,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::SodiumChannelActivator,
            molecular_weight_da: 7000.0,
            ld50_mg_per_kg: 0.35,
            yield_mg: 0.5,
            onset_minutes: 15.0,
            duration_hours: 18.0,
            specificity: 0.8,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // Chlorotoxin: Androctonus mauretanicus，氯通道，脑瘤靶向显像剂
        Toxin {
            name: "Chlorotoxin".into(),
            source: ToxinSource::Scorpion,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::PotassiumChannelBlocker,
            molecular_weight_da: 3990.0,
            ld50_mg_per_kg: 0.5,
            yield_mg: 0.3,
            onset_minutes: 30.0,
            duration_hours: 12.0,
            specificity: 0.9,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // Agitoxin-2: Leiurus quinquestriatus，钾通道阻断
        Toxin {
            name: "Agitoxin-2".into(),
            source: ToxinSource::Scorpion,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::PotassiumChannelBlocker,
            molecular_weight_da: 4040.0,
            ld50_mg_per_kg: 0.4,
            yield_mg: 0.3,
            onset_minutes: 20.0,
            duration_hours: 12.0,
            specificity: 0.85,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // Pandinotoxin: Pandinus imperator，钾通道
        Toxin {
            name: "Pandinotoxin".into(),
            source: ToxinSource::Scorpion,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::PotassiumChannelBlocker,
            molecular_weight_da: 3500.0,
            ld50_mg_per_kg: 0.6,
            yield_mg: 0.3,
            onset_minutes: 25.0,
            duration_hours: 12.0,
            specificity: 0.8,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // Androctonus australis 全毒（北非肥尾蝎，最毒蝎之一）
        Toxin {
            name: "Androctonus australis venom".into(),
            source: ToxinSource::Scorpion,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::SodiumChannelActivator,
            molecular_weight_da: 7000.0,
            ld50_mg_per_kg: 0.32,
            yield_mg: 0.6,
            onset_minutes: 15.0,
            duration_hours: 24.0,
            specificity: 0.7,
            chemical_class: ToxinChemicalClass::Protein,
        },
    ]
}

/// 蜘蛛毒数据库
///
/// 来源：King GF. "The Wonderful World of Spiders" (2004)
pub fn spider_venom_database() -> Vec<Toxin> {
    vec![
        // α-Latrotoxin: 黑寡妇 Latrodectus mactans，120 kDa，突触前大量释放神经递质
        Toxin {
            name: "α-Latrotoxin".into(),
            source: ToxinSource::Spider,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::PresynapticCleaver,
            molecular_weight_da: 120000.0,
            ld50_mg_per_kg: 0.025,
            yield_mg: 0.1,
            onset_minutes: 30.0,
            duration_hours: 24.0,
            specificity: 0.6,
            chemical_class: ToxinChemicalClass::Protein,
        },
        // Robustoxin: 悉尼漏斗网 Atrax robustus
        Toxin {
            name: "Robustoxin".into(),
            source: ToxinSource::Spider,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::SodiumChannelActivator,
            molecular_weight_da: 4870.0,
            ld50_mg_per_kg: 0.05,
            yield_mg: 0.1,
            onset_minutes: 15.0,
            duration_hours: 12.0,
            specificity: 0.85,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // ω-Hexatoxin-Hv1a: 蓝山漏斗网 Hadronyche versuta，N 型钙通道
        Toxin {
            name: "ω-Hexatoxin-Hv1a".into(),
            source: ToxinSource::Spider,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::CalciumChannelBlocker,
            molecular_weight_da: 4080.0,
            ld50_mg_per_kg: 0.1,
            yield_mg: 0.1,
            onset_minutes: 20.0,
            duration_hours: 12.0,
            specificity: 0.9,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // ω-Agatoxin IVA: 漏斗蛛 Agelenopsis aperta，P/Q 型钙通道
        Toxin {
            name: "ω-Agatoxin IVA".into(),
            source: ToxinSource::Spider,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::CalciumChannelBlocker,
            molecular_weight_da: 5200.0,
            ld50_mg_per_kg: 0.2,
            yield_mg: 0.1,
            onset_minutes: 25.0,
            duration_hours: 10.0,
            specificity: 0.85,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // Psalmotoxin-1: Psalmopoeus cambridgei，ASIC1a 酸敏感离子通道
        Toxin {
            name: "Psalmotoxin-1".into(),
            source: ToxinSource::Spider,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::SodiumChannelBlocker,
            molecular_weight_da: 4700.0,
            ld50_mg_per_kg: 0.3,
            yield_mg: 0.1,
            onset_minutes: 30.0,
            duration_hours: 10.0,
            specificity: 0.95,
            chemical_class: ToxinChemicalClass::Peptide,
        },
    ]
}

/// 海洋生物毒素数据库
///
/// 来源：Lewis RJ & Garcia ML. Nat Rev Drug Discov (2003)
pub fn marine_venom_database() -> Vec<Toxin> {
    vec![
        // TTX: 河豚/蓝环章鱼/蝾螈，LD50 8 μg/kg，钠通道 site-1 阻断
        Toxin {
            name: "Tetrodotoxin (TTX)".into(),
            source: ToxinSource::Fish,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::SodiumChannelBlocker,
            molecular_weight_da: 319.27,
            ld50_mg_per_kg: 0.008,
            yield_mg: 60.0,
            onset_minutes: 20.0,
            duration_hours: 24.0,
            specificity: 0.95,
            chemical_class: ToxinChemicalClass::SmallMolecule,
        },
        // STX: 甲藻-贝类，赤潮，LD50 3 μg/kg
        Toxin {
            name: "Saxitoxin (STX)".into(),
            source: ToxinSource::Dinoflagellate,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::SodiumChannelBlocker,
            molecular_weight_da: 299.29,
            ld50_mg_per_kg: 0.003,
            yield_mg: 0.5,
            onset_minutes: 15.0,
            duration_hours: 18.0,
            specificity: 0.95,
            chemical_class: ToxinChemicalClass::SmallMolecule,
        },
        // ω-Conotoxin MVIIA: 芋螺 Conus magus，N 型钙通道阻断，ziconotide 镇痛药
        Toxin {
            name: "ω-Conotoxin MVIIA".into(),
            source: ToxinSource::ConeSnail,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::CalciumChannelBlocker,
            molecular_weight_da: 2638.0,
            ld50_mg_per_kg: 0.015,
            yield_mg: 0.5,
            onset_minutes: 20.0,
            duration_hours: 12.0,
            specificity: 0.95,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // α-Conotoxin: nAChR
        Toxin {
            name: "α-Conotoxin".into(),
            source: ToxinSource::ConeSnail,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::AcetylcholineReceptor,
            molecular_weight_da: 1500.0,
            ld50_mg_per_kg: 0.05,
            yield_mg: 0.5,
            onset_minutes: 20.0,
            duration_hours: 12.0,
            specificity: 0.9,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // μ-Conotoxin GIIIA: 钠通道 site-1，与 TTX 位点不同
        Toxin {
            name: "μ-Conotoxin GIIIA".into(),
            source: ToxinSource::ConeSnail,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::SodiumChannelBlocker,
            molecular_weight_da: 2608.0,
            ld50_mg_per_kg: 0.03,
            yield_mg: 0.5,
            onset_minutes: 25.0,
            duration_hours: 12.0,
            specificity: 0.9,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // δ-Conotoxin: 延迟钠通道失活
        Toxin {
            name: "δ-Conotoxin".into(),
            source: ToxinSource::ConeSnail,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::SodiumChannelActivator,
            molecular_weight_da: 3000.0,
            ld50_mg_per_kg: 0.1,
            yield_mg: 0.5,
            onset_minutes: 30.0,
            duration_hours: 12.0,
            specificity: 0.85,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // 箱水母 Chironex fleckeri：死亡 5-30 分钟，心脏毒
        Toxin {
            name: "Chironex fleckeri venom".into(),
            source: ToxinSource::Jellyfish,
            venom_type: VenomType::Cardiotoxin,
            mechanism: ToxinMechanism::MembraneDisruptor,
            molecular_weight_da: 60000.0,
            ld50_mg_per_kg: 0.04,
            yield_mg: 5.0,
            onset_minutes: 5.0,
            duration_hours: 6.0,
            specificity: 0.5,
            chemical_class: ToxinChemicalClass::Protein,
        },
        // Irukandji: Carukia barnesi，儿茶酚胺风暴
        Toxin {
            name: "Irukandji venom".into(),
            source: ToxinSource::Jellyfish,
            venom_type: VenomType::Cardiotoxin,
            mechanism: ToxinMechanism::SodiumChannelActivator,
            molecular_weight_da: 40000.0,
            ld50_mg_per_kg: 0.1,
            yield_mg: 1.0,
            onset_minutes: 20.0,
            duration_hours: 24.0,
            specificity: 0.6,
            chemical_class: ToxinChemicalClass::Protein,
        },
        // 僧帽水母 Physalia physalis
        Toxin {
            name: "Physaliatoxin".into(),
            source: ToxinSource::Cnidarian,
            venom_type: VenomType::Cytotoxin,
            mechanism: ToxinMechanism::PoreFormer,
            molecular_weight_da: 240000.0,
            ld50_mg_per_kg: 0.5,
            yield_mg: 2.0,
            onset_minutes: 10.0,
            duration_hours: 12.0,
            specificity: 0.4,
            chemical_class: ToxinChemicalClass::Protein,
        },
        // 石鱼 Synanceia verrucosa：verrucotoxin
        Toxin {
            name: "Verrucotoxin".into(),
            source: ToxinSource::Fish,
            venom_type: VenomType::Cytotoxin,
            mechanism: ToxinMechanism::MembraneDisruptor,
            molecular_weight_da: 322000.0,
            ld50_mg_per_kg: 0.2,
            yield_mg: 50.0,
            onset_minutes: 10.0,
            duration_hours: 24.0,
            specificity: 0.5,
            chemical_class: ToxinChemicalClass::Protein,
        },
    ]
}

/// 两栖动物毒素数据库
///
/// 来源：Mebs D. "Venomous and Poisonous Animals" (2002)
pub fn amphibian_toxin_database() -> Vec<Toxin> {
    vec![
        // Batrachotoxin: 箭毒蛙 Phyllobates terribilis，LD50 2 μg/kg，钠通道持续激活
        Toxin {
            name: "Batrachotoxin".into(),
            source: ToxinSource::Frog,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::SodiumChannelActivator,
            molecular_weight_da: 538.7,
            ld50_mg_per_kg: 0.002,
            yield_mg: 1.0,
            onset_minutes: 15.0,
            duration_hours: 24.0,
            specificity: 0.85,
            chemical_class: ToxinChemicalClass::Alkaloid,
        },
        // Epibatidine: 树蛙 Epipedobates，nAChR 激动剂，镇痛
        Toxin {
            name: "Epibatidine".into(),
            source: ToxinSource::Frog,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::AcetylcholineReceptor,
            molecular_weight_da: 208.7,
            ld50_mg_per_kg: 0.0013,
            yield_mg: 0.5,
            onset_minutes: 15.0,
            duration_hours: 8.0,
            specificity: 0.9,
            chemical_class: ToxinChemicalClass::Alkaloid,
        },
        // Bufotoxin: 蟾蜍 Bufonidae，bufadienolide，强心苷类
        Toxin {
            name: "Bufotoxin".into(),
            source: ToxinSource::Toad,
            venom_type: VenomType::Cardiotoxin,
            mechanism: ToxinMechanism::SodiumChannelActivator,
            molecular_weight_da: 757.0,
            ld50_mg_per_kg: 0.4,
            yield_mg: 50.0,
            onset_minutes: 30.0,
            duration_hours: 12.0,
            specificity: 0.6,
            chemical_class: ToxinChemicalClass::Glycoside,
        },
        // Samandarin: 蝾螈 Salamandra，甾体生物碱
        Toxin {
            name: "Samandarin".into(),
            source: ToxinSource::Toad,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::SodiumChannelActivator,
            molecular_weight_da: 349.5,
            ld50_mg_per_kg: 0.5,
            yield_mg: 10.0,
            onset_minutes: 30.0,
            duration_hours: 12.0,
            specificity: 0.7,
            chemical_class: ToxinChemicalClass::Alkaloid,
        },
        // TTX 在 Taricha 蝾螈中（与河豚同分子，此处归类两栖）
        Toxin {
            name: "Taricha TTX".into(),
            source: ToxinSource::Frog,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::SodiumChannelBlocker,
            molecular_weight_da: 319.27,
            ld50_mg_per_kg: 0.025,
            yield_mg: 0.5,
            onset_minutes: 20.0,
            duration_hours: 24.0,
            specificity: 0.9,
            chemical_class: ToxinChemicalClass::SmallMolecule,
        },
    ]
}

/// 无脊椎动物毒素数据库
///
/// 来源：Schmidt JO 蜂毒痛指数；Mebs D. (2002)
pub fn invertebrate_venom_database() -> Vec<Toxin> {
    vec![
        // 蜂毒 Melittin: 蜜蜂 Apis mellifera，磷脂酶 A2 激活，溶血，26 aa
        Toxin {
            name: "Melittin".into(),
            source: ToxinSource::Bee,
            venom_type: VenomType::Cytotoxin,
            mechanism: ToxinMechanism::Hemolysis,
            molecular_weight_da: 2846.5,
            ld50_mg_per_kg: 4.0,
            yield_mg: 0.1,
            onset_minutes: 5.0,
            duration_hours: 6.0,
            specificity: 0.3,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // 蜂毒肽 Apamin: 蜜蜂，钾通道阻断（SK 通道）
        Toxin {
            name: "Apamin".into(),
            source: ToxinSource::Bee,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::PotassiumChannelBlocker,
            molecular_weight_da: 2027.3,
            ld50_mg_per_kg: 0.5,
            yield_mg: 0.05,
            onset_minutes: 30.0,
            duration_hours: 12.0,
            specificity: 0.9,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // Mastoparan: 黄蜂，肥大细胞脱颗粒
        Toxin {
            name: "Mastoparan".into(),
            source: ToxinSource::Wasp,
            venom_type: VenomType::Cytotoxin,
            mechanism: ToxinMechanism::PoreFormer,
            molecular_weight_da: 1478.8,
            ld50_mg_per_kg: 1.5,
            yield_mg: 0.1,
            onset_minutes: 5.0,
            duration_hours: 6.0,
            specificity: 0.3,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // 子弹蚁 Dinoponera: Poneratoxin，Schmidt 痛指数 4+
        Toxin {
            name: "Poneratoxin".into(),
            source: ToxinSource::Ant,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::SodiumChannelBlocker,
            molecular_weight_da: 2950.0,
            ld50_mg_per_kg: 1.0,
            yield_mg: 0.05,
            onset_minutes: 5.0,
            duration_hours: 24.0,
            specificity: 0.7,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // 蜈蚣 Scolopendra: Toxin-Smase，鞘磷脂酶
        Toxin {
            name: "Toxin-Smase".into(),
            source: ToxinSource::Centipede,
            venom_type: VenomType::Cytotoxin,
            mechanism: ToxinMechanism::MembraneDisruptor,
            molecular_weight_da: 35000.0,
            ld50_mg_per_kg: 2.0,
            yield_mg: 1.0,
            onset_minutes: 15.0,
            duration_hours: 12.0,
            specificity: 0.4,
            chemical_class: ToxinChemicalClass::Enzyme,
        },
        // 千足虫分泌 HCN（氰化氢）
        Toxin {
            name: "Hydrogen cyanide (millipede)".into(),
            source: ToxinSource::Millipede,
            venom_type: VenomType::Cytotoxin,
            mechanism: ToxinMechanism::ApoptosisInducer,
            molecular_weight_da: 27.03,
            ld50_mg_per_kg: 1.0,
            yield_mg: 0.5,
            onset_minutes: 10.0,
            duration_hours: 6.0,
            specificity: 0.2,
            chemical_class: ToxinChemicalClass::SmallMolecule,
        },
        // 行军蚁 Dorylus：甲酸
        Toxin {
            name: "Formic acid (Dorylus)".into(),
            source: ToxinSource::Ant,
            venom_type: VenomType::Cytotoxin,
            mechanism: ToxinMechanism::MembraneDisruptor,
            molecular_weight_da: 46.03,
            ld50_mg_per_kg: 100.0,
            yield_mg: 0.05,
            onset_minutes: 5.0,
            duration_hours: 3.0,
            specificity: 0.1,
            chemical_class: ToxinChemicalClass::SmallMolecule,
        },
    ]
}

/// 微生物毒素数据库
///
/// 来源：Schantz & Montecucco 细菌毒素综述；FDA 标签
pub fn microbial_toxin_database() -> Vec<Toxin> {
    vec![
        // 肉毒杆菌毒素 BoNT: Clostridium botulinum，LD50 1 ng/kg，最毒物质之一，ACh 释放阻断
        Toxin {
            name: "Botulinum toxin (BoNT)".into(),
            source: ToxinSource::Bacteria,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::PresynapticCleaver,
            molecular_weight_da: 150000.0,
            ld50_mg_per_kg: 0.000001,
            yield_mg: 0.001,
            onset_minutes: 720.0,
            duration_hours: 720.0,
            specificity: 0.95,
            chemical_class: ToxinChemicalClass::Protein,
        },
        // 破伤风毒素 TeNT: Clostridium tetani，LD50 1 ng/kg，抑制性神经元阻断
        Toxin {
            name: "Tetanospasmin (TeNT)".into(),
            source: ToxinSource::Bacteria,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::PresynapticCleaver,
            molecular_weight_da: 150000.0,
            ld50_mg_per_kg: 0.000001,
            yield_mg: 0.001,
            onset_minutes: 4320.0,
            duration_hours: 720.0,
            specificity: 0.95,
            chemical_class: ToxinChemicalClass::Protein,
        },
        // 白喉毒素: Corynebacterium diphtheriae，EF-2 ADP-核糖基化
        Toxin {
            name: "Diphtheria toxin".into(),
            source: ToxinSource::Bacteria,
            venom_type: VenomType::Cytotoxin,
            mechanism: ToxinMechanism::ProteinSynthesisInhibitor,
            molecular_weight_da: 62000.0,
            ld50_mg_per_kg: 0.0001,
            yield_mg: 0.01,
            onset_minutes: 1440.0,
            duration_hours: 168.0,
            specificity: 0.85,
            chemical_class: ToxinChemicalClass::Protein,
        },
        // 霍乱毒素: Vibrio cholerae，Gs α ADP-核糖基化，cAMP 爆发
        Toxin {
            name: "Cholera toxin".into(),
            source: ToxinSource::Bacteria,
            venom_type: VenomType::Cytotoxin,
            mechanism: ToxinMechanism::ApoptosisInducer,
            molecular_weight_da: 84000.0,
            ld50_mg_per_kg: 0.005,
            yield_mg: 0.05,
            onset_minutes: 180.0,
            duration_hours: 72.0,
            specificity: 0.7,
            chemical_class: ToxinChemicalClass::Protein,
        },
        // 百日咳毒素: Bordetella pertussis，Gi 抑制
        Toxin {
            name: "Pertussis toxin".into(),
            source: ToxinSource::Bacteria,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::ApoptosisInducer,
            molecular_weight_da: 105000.0,
            ld50_mg_per_kg: 0.015,
            yield_mg: 0.05,
            onset_minutes: 4320.0,
            duration_hours: 168.0,
            specificity: 0.7,
            chemical_class: ToxinChemicalClass::Protein,
        },
        // 志贺毒素: Shigella dysenteriae，60S 核糖体失活
        Toxin {
            name: "Shiga toxin".into(),
            source: ToxinSource::Bacteria,
            venom_type: VenomType::Cytotoxin,
            mechanism: ToxinMechanism::RibosomeInactivator,
            molecular_weight_da: 70000.0,
            ld50_mg_per_kg: 0.001,
            yield_mg: 0.01,
            onset_minutes: 720.0,
            duration_hours: 168.0,
            specificity: 0.85,
            chemical_class: ToxinChemicalClass::Protein,
        },
        // Vero toxin: E. coli O157:H7
        Toxin {
            name: "Verotoxin (Stx2)".into(),
            source: ToxinSource::Bacteria,
            venom_type: VenomType::Cytotoxin,
            mechanism: ToxinMechanism::RibosomeInactivator,
            molecular_weight_da: 70000.0,
            ld50_mg_per_kg: 0.002,
            yield_mg: 0.01,
            onset_minutes: 720.0,
            duration_hours: 168.0,
            specificity: 0.85,
            chemical_class: ToxinChemicalClass::Protein,
        },
        // 葡萄球菌 α-toxin: Staphylococcus aureus，孔道形成
        Toxin {
            name: "Staph α-toxin".into(),
            source: ToxinSource::Bacteria,
            venom_type: VenomType::Cytotoxin,
            mechanism: ToxinMechanism::PoreFormer,
            molecular_weight_da: 33000.0,
            ld50_mg_per_kg: 0.05,
            yield_mg: 0.1,
            onset_minutes: 60.0,
            duration_hours: 24.0,
            specificity: 0.5,
            chemical_class: ToxinChemicalClass::Protein,
        },
        // 产气荚膜梭菌 ε-toxin
        Toxin {
            name: "ε-Toxin".into(),
            source: ToxinSource::Bacteria,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::PoreFormer,
            molecular_weight_da: 32800.0,
            ld50_mg_per_kg: 0.0001,
            yield_mg: 0.05,
            onset_minutes: 60.0,
            duration_hours: 48.0,
            specificity: 0.8,
            chemical_class: ToxinChemicalClass::Protein,
        },
        // 炭疽毒素: Bacillus anthracis，EF + LF + PA
        Toxin {
            name: "Anthrax lethal toxin".into(),
            source: ToxinSource::Bacteria,
            venom_type: VenomType::Cytotoxin,
            mechanism: ToxinMechanism::ApoptosisInducer,
            molecular_weight_da: 90000.0,
            ld50_mg_per_kg: 0.0005,
            yield_mg: 0.05,
            onset_minutes: 120.0,
            duration_hours: 72.0,
            specificity: 0.85,
            chemical_class: ToxinChemicalClass::Protein,
        },
        // 艰难梭菌 TcdA
        Toxin {
            name: "C. difficile TcdA".into(),
            source: ToxinSource::Bacteria,
            venom_type: VenomType::Cytotoxin,
            mechanism: ToxinMechanism::ApoptosisInducer,
            molecular_weight_da: 308000.0,
            ld50_mg_per_kg: 0.01,
            yield_mg: 0.1,
            onset_minutes: 720.0,
            duration_hours: 168.0,
            specificity: 0.7,
            chemical_class: ToxinChemicalClass::Protein,
        },
    ]
}

/// 植物毒素数据库
///
/// 来源：Lord JM et al. "Ribosome-inactivating lectins" (2003)；Mebs (2002)
pub fn plant_toxin_database() -> Vec<Toxin> {
    vec![
        // 蓖麻毒素 Ricin: Ricinus communis，60S 核糖体失活，LD50 1 μg/kg
        Toxin {
            name: "Ricin".into(),
            source: ToxinSource::Plant,
            venom_type: VenomType::Cytotoxin,
            mechanism: ToxinMechanism::RibosomeInactivator,
            molecular_weight_da: 64000.0,
            ld50_mg_per_kg: 0.001,
            yield_mg: 10.0,
            onset_minutes: 1440.0,
            duration_hours: 72.0,
            specificity: 0.85,
            chemical_class: ToxinChemicalClass::Protein,
        },
        // 相思子毒素 Abrin: Abrus precatorius，比 ricin 毒 75×
        Toxin {
            name: "Abrin".into(),
            source: ToxinSource::Plant,
            venom_type: VenomType::Cytotoxin,
            mechanism: ToxinMechanism::RibosomeInactivator,
            molecular_weight_da: 65000.0,
            ld50_mg_per_kg: 0.00004,
            yield_mg: 5.0,
            onset_minutes: 1440.0,
            duration_hours: 72.0,
            specificity: 0.85,
            chemical_class: ToxinChemicalClass::Protein,
        },
        // 美洲商陆 PAP: Phytolacca americana，抗病毒
        Toxin {
            name: "Pokeweed antiviral protein (PAP)".into(),
            source: ToxinSource::Plant,
            venom_type: VenomType::Cytotoxin,
            mechanism: ToxinMechanism::RibosomeInactivator,
            molecular_weight_da: 29000.0,
            ld50_mg_per_kg: 0.02,
            yield_mg: 5.0,
            onset_minutes: 720.0,
            duration_hours: 48.0,
            specificity: 0.8,
            chemical_class: ToxinChemicalClass::Protein,
        },
        // 强心苷: 洋地黄 Foxglove，Na/K-ATPase 抑制
        Toxin {
            name: "Cardiac glycoside (Digoxin)".into(),
            source: ToxinSource::Plant,
            venom_type: VenomType::Cardiotoxin,
            mechanism: ToxinMechanism::SodiumChannelActivator,
            molecular_weight_da: 780.95,
            ld50_mg_per_kg: 0.3,
            yield_mg: 50.0,
            onset_minutes: 60.0,
            duration_hours: 48.0,
            specificity: 0.8,
            chemical_class: ToxinChemicalClass::Glycoside,
        },
        // 吗啡: 罂粟 Papaver somniferum
        Toxin {
            name: "Morphine".into(),
            source: ToxinSource::Plant,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::CalciumChannelBlocker,
            molecular_weight_da: 285.34,
            ld50_mg_per_kg: 250.0,
            yield_mg: 100.0,
            onset_minutes: 30.0,
            duration_hours: 6.0,
            specificity: 0.9,
            chemical_class: ToxinChemicalClass::Alkaloid,
        },
        // 秋水仙碱: 秋水仙 Colchicum，微管抑制
        Toxin {
            name: "Colchicine".into(),
            source: ToxinSource::Plant,
            venom_type: VenomType::Cytotoxin,
            mechanism: ToxinMechanism::ApoptosisInducer,
            molecular_weight_da: 399.44,
            ld50_mg_per_kg: 1.7,
            yield_mg: 20.0,
            onset_minutes: 120.0,
            duration_hours: 72.0,
            specificity: 0.7,
            chemical_class: ToxinChemicalClass::Alkaloid,
        },
        // 乌头碱: 乌头 Aconitum，钠通道持续激活
        Toxin {
            name: "Aconitine".into(),
            source: ToxinSource::Plant,
            venom_type: VenomType::Cardiotoxin,
            mechanism: ToxinMechanism::SodiumChannelActivator,
            molecular_weight_da: 645.74,
            ld50_mg_per_kg: 0.1,
            yield_mg: 10.0,
            onset_minutes: 30.0,
            duration_hours: 24.0,
            specificity: 0.7,
            chemical_class: ToxinChemicalClass::Alkaloid,
        },
        // 毒蕈碱: 毒蝇伞 Amanita muscaria
        Toxin {
            name: "Muscarine".into(),
            source: ToxinSource::Fungi,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::AcetylcholineReceptor,
            molecular_weight_da: 174.24,
            ld50_mg_per_kg: 0.2,
            yield_mg: 5.0,
            onset_minutes: 30.0,
            duration_hours: 8.0,
            specificity: 0.85,
            chemical_class: ToxinChemicalClass::Alkaloid,
        },
        // 鹅膏毒素: α-amanitin，RNA Pol II 抑制，LD50 0.1 mg/kg（人）
        Toxin {
            name: "α-Amanitin".into(),
            source: ToxinSource::Fungi,
            venom_type: VenomType::Hepatotoxin,
            mechanism: ToxinMechanism::ProteinSynthesisInhibitor,
            molecular_weight_da: 918.97,
            ld50_mg_per_kg: 0.1,
            yield_mg: 5.0,
            onset_minutes: 4320.0,
            duration_hours: 240.0,
            specificity: 0.95,
            chemical_class: ToxinChemicalClass::Peptide,
        },
        // 鹅膏蕈氨酸: 毒蝇伞
        Toxin {
            name: "Ibotenic acid".into(),
            source: ToxinSource::Fungi,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::AcetylcholineReceptor,
            molecular_weight_da: 158.11,
            ld50_mg_per_kg: 15.0,
            yield_mg: 10.0,
            onset_minutes: 30.0,
            duration_hours: 8.0,
            specificity: 0.7,
            chemical_class: ToxinChemicalClass::Aminopolyol,
        },
    ]
}

/// 鱼类毒素数据库
///
/// 来源：Halstead BW "Poisonous and Venomous Marine Animals" ；Yasumoto T 雪卡毒素研究
pub fn fish_toxin_database() -> Vec<Toxin> {
    vec![
        // TTX: 河豚 Takifugu 内脏
        Toxin {
            name: "Pufferfish TTX".into(),
            source: ToxinSource::Fish,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::SodiumChannelBlocker,
            molecular_weight_da: 319.27,
            ld50_mg_per_kg: 0.008,
            yield_mg: 100.0,
            onset_minutes: 20.0,
            duration_hours: 24.0,
            specificity: 0.95,
            chemical_class: ToxinChemicalClass::SmallMolecule,
        },
        // Ciguatoxin: 雪卡毒素，热带鱼食物链累积，钠通道激活
        Toxin {
            name: "Ciguatoxin (CTX-1)".into(),
            source: ToxinSource::Fish,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::SodiumChannelActivator,
            molecular_weight_da: 1111.36,
            ld50_mg_per_kg: 0.00025,
            yield_mg: 0.01,
            onset_minutes: 180.0,
            duration_hours: 720.0,
            specificity: 0.9,
            chemical_class: ToxinChemicalClass::Polyketide,
        },
        // Pahutoxin: Boxfish Ostracion 分泌
        Toxin {
            name: "Pahutoxin".into(),
            source: ToxinSource::Fish,
            venom_type: VenomType::Cytotoxin,
            mechanism: ToxinMechanism::MembraneDisruptor,
            molecular_weight_da: 343.5,
            ld50_mg_per_kg: 0.5,
            yield_mg: 5.0,
            onset_minutes: 15.0,
            duration_hours: 12.0,
            specificity: 0.3,
            chemical_class: ToxinChemicalClass::Glycoside,
        },
    ]
}

// ============ 5. 抗毒血清 ============

/// 抗毒血清
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Antivenom {
    /// 商品名
    pub name: String,
    /// 目标毒素名称
    pub target_toxins: Vec<String>,
    /// 来源动物
    pub origin_animal: String,
    /// 抗体类型
    pub antibody_type: AntibodyType,
    /// 有效性 0.0-1.0
    pub effectiveness: f32,
    /// 给药方式
    pub administration: String,
    /// 起效时间（分钟）
    pub onset_minutes: f32,
}

/// 抗体类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AntibodyType {
    /// 完整 IgG（马血清）
    WholeIgG,
    /// Fab 片段（CroFab）
    FabFragment,
    /// F(ab')2 片段
    FAb2Fragment,
    /// 单克隆
    Monoclonal,
    /// 重组
    Recombinant,
}

/// 抗毒血清数据库
///
/// 来源：WHO 蛇毒指南；FDA / 厂商标签
pub fn antivenom_database() -> Vec<Antivenom> {
    vec![
        // CroFab: Crotalidae Polyvalent Immune Fab，北美蝮蛇
        Antivenom {
            name: "CroFab".into(),
            target_toxins: vec![
                "Crotalus atrox venom".into(),
                "Crotalus adamanteus venom".into(),
                "Agkistrodon piscivorus venom".into(),
            ],
            origin_animal: "Sheep (ovine)".into(),
            antibody_type: AntibodyType::FabFragment,
            effectiveness: 0.85,
            administration: "IV infusion".into(),
            onset_minutes: 60.0,
        },
        // SAIMR Polyvalent: 南非，覆盖 10 种蛇
        Antivenom {
            name: "SAIMR Polyvalent".into(),
            target_toxins: vec![
                "Dendroaspis polylepis venom".into(),
                "Naja nivea venom".into(),
                "Bitis arietans venom".into(),
            ],
            origin_animal: "Horse (equine)".into(),
            antibody_type: AntibodyType::FAb2Fragment,
            effectiveness: 0.8,
            administration: "IV infusion".into(),
            onset_minutes: 60.0,
        },
        // CSUR Antivipmyn: 墨西哥
        Antivenom {
            name: "Antivipmyn".into(),
            target_toxins: vec![
                "Crotalus simus venom".into(),
                "Bothrops asper venom".into(),
            ],
            origin_animal: "Horse (equine)".into(),
            antibody_type: AntibodyType::FAb2Fragment,
            effectiveness: 0.82,
            administration: "IV infusion".into(),
            onset_minutes: 60.0,
        },
        // 兴泰 TIG: 破伤风免疫球蛋白
        Antivenom {
            name: "Tetanus Immune Globulin (TIG)".into(),
            target_toxins: vec!["Tetanospasmin (TeNT)".into()],
            origin_animal: "Human".into(),
            antibody_type: AntibodyType::WholeIgG,
            effectiveness: 0.95,
            administration: "IM injection".into(),
            onset_minutes: 30.0,
        },
        // BabyBIG: 婴儿肉毒杆菌免疫球蛋白
        Antivenom {
            name: "BabyBIG".into(),
            target_toxins: vec!["Botulinum toxin (BoNT)".into()],
            origin_animal: "Human".into(),
            antibody_type: AntibodyType::WholeIgG,
            effectiveness: 0.9,
            administration: "IV infusion".into(),
            onset_minutes: 60.0,
        },
        // Taipan antivenom: CSL
        Antivenom {
            name: "CSL Taipan Antivenom".into(),
            target_toxins: vec!["Taipoxin".into()],
            origin_animal: "Horse (equine)".into(),
            antibody_type: AntibodyType::WholeIgG,
            effectiveness: 0.88,
            administration: "IV infusion".into(),
            onset_minutes: 60.0,
        },
        // 黑寡妇抗血清: Merck
        Antivenom {
            name: "Black Widow Antivenom (Merck)".into(),
            target_toxins: vec!["α-Latrotoxin".into()],
            origin_animal: "Horse (equine)".into(),
            antibody_type: AntibodyType::FAb2Fragment,
            effectiveness: 0.85,
            administration: "IM injection".into(),
            onset_minutes: 30.0,
        },
        // 石鱼抗血清: CSL
        Antivenom {
            name: "CSL Stonefish Antivenom".into(),
            target_toxins: vec!["Verrucotoxin".into()],
            origin_animal: "Horse (equine)".into(),
            antibody_type: AntibodyType::WholeIgG,
            effectiveness: 0.85,
            administration: "IM/IV injection".into(),
            onset_minutes: 30.0,
        },
        // Equine-derived 多克隆马（通用）
        Antivenom {
            name: "Equine Polyvalent (generic)".into(),
            target_toxins: vec![
                "SnakeElapidae venom".into(),
                "SnakeViperidae venom".into(),
            ],
            origin_animal: "Horse (equine)".into(),
            antibody_type: AntibodyType::WholeIgG,
            effectiveness: 0.7,
            administration: "IV infusion".into(),
            onset_minutes: 60.0,
        },
    ]
}

// ============================================================
// 辅助查询方法（v5.0 追加 —— 不修改原有 impl 块）
// ============================================================

impl Toxin {
    /// LD50 毒性分级（同 comparative_toxicity 的语义别名）
    pub fn ld50_class(&self) -> ToxicityRank {
        self.comparative_toxicity()
    }

    /// 毒性类别中文描述
    pub fn toxicity_category(&self) -> &'static str {
        match self.comparative_toxicity() {
            ToxicityRank::ExtremelyToxic => "极毒",
            ToxicityRank::Supertoxic => "剧毒",
            ToxicityRank::HighlyToxic => "有毒",
            ToxicityRank::ModeratelyToxic => "中等毒",
            ToxicityRank::SlightlyToxic => "微毒",
            ToxicityRank::RelativelyHarmless => "无害",
        }
    }

    /// 起效时间分类：速发 <1h，中速 1-24h，迟发 >24h
    pub fn onset_class(&self) -> &'static str {
        if self.onset_minutes < 60.0 {
            "Rapid"
        } else if self.onset_minutes < 1440.0 {
            "Intermediate"
        } else {
            "Delayed"
        }
    }

    /// 是否速发起效（<1 小时）
    pub fn is_rapid_onset(&self) -> bool {
        self.onset_minutes < 60.0
    }

    /// 是否极端致死（LD50 < 0.001 mg/kg）
    pub fn is_extremely_lethal(&self) -> bool {
        self.ld50_mg_per_kg < 0.001
    }

    /// 是否为蛋白/酶类毒素
    pub fn is_protein_toxin(&self) -> bool {
        matches!(
            self.chemical_class,
            ToxinChemicalClass::Protein | ToxinChemicalClass::Enzyme
        )
    }

    /// 是否为小分子毒素（MW < 900 Da）
    pub fn is_small_molecule(&self) -> bool {
        self.molecular_weight_da < 900.0
    }

    /// 分子量（kDa）
    pub fn molar_mass_kda(&self) -> f32 {
        self.molecular_weight_da / 1000.0
    }

    /// 单次分泌量是否足以致死 70kg 成人
    pub fn yield_is_lethal_to_human(&self) -> bool {
        self.yield_mg >= self.lethal_dose_for_70kg()
    }

    /// 安全边际 = yield / LD70（>1 表示单次分泌量足以致死）
    pub fn safety_margin(&self) -> f32 {
        let ld70 = self.lethal_dose_for_70kg();
        if ld70 <= 0.0 {
            0.0
        } else {
            self.yield_mg / ld70
        }
    }

    /// 是否作用于神经系统
    pub fn targets_nervous_system(&self) -> bool {
        self.venom_type == VenomType::Neurotoxin
    }

    /// 给定抗蛇毒血清是否对该毒素有效（按毒素名匹配）
    pub fn antivenom_effective(&self, av: &Antivenom) -> bool {
        av.target_toxins.iter().any(|t| t == &self.name)
    }

    /// 70kg 成人致死剂量（mg） —— lethal_dose_for_70kg 别名
    pub fn human_lethal_dose_mg(&self) -> f32 {
        self.lethal_dose_for_70kg()
    }
}
impl Antivenom {
    /// 是否多价（覆盖 ≥ 2 种毒素）
    pub fn is_polyvalent(&self) -> bool {
        self.target_toxins.len() >= 2
    }

    /// 是否单价
    pub fn is_monovalent(&self) -> bool {
        self.target_toxins.len() == 1
    }

    /// 是否针对某毒素有效（按毒素名匹配）
    pub fn targets_toxin(&self, toxin_name: &str) -> bool {
        self.target_toxins.iter().any(|t| t == toxin_name)
    }

    /// 是否人源抗体
    pub fn is_human_origin(&self) -> bool {
        self.origin_animal.contains("Human")
    }

    /// 是否马源抗体
    pub fn is_equine(&self) -> bool {
        self.origin_animal.contains("Horse") || self.origin_animal.contains("equine")
    }

    /// 是否单克隆抗体
    pub fn is_monoclonal(&self) -> bool {
        self.antibody_type == AntibodyType::Monoclonal
    }

    /// 是否完整 IgG
    pub fn is_whole_ig(&self) -> bool {
        self.antibody_type == AntibodyType::WholeIgG
    }

    /// 起效是否迅速（<60 分钟）
    pub fn is_rapid_onset(&self) -> bool {
        self.onset_minutes < 60.0
    }
}

impl ToxinEffect {
    /// 是否严重（severity > 0.7）
    pub fn is_severe(&self) -> bool {
        self.severity > 0.7
    }

    /// 是否危及生命（severity > 0.8 且不可逆）
    pub fn is_life_threatening(&self) -> bool {
        self.severity > 0.8 && !self.reversible
    }

    /// 是否可完全恢复
    pub fn is_recoverable(&self) -> bool {
        self.reversible
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn make_toxin(name: &str, ld50: f32, onset: f32, mw: f32) -> Toxin {
        Toxin {
            name: name.into(),
            source: ToxinSource::SnakeElapidae,
            venom_type: VenomType::Neurotoxin,
            mechanism: ToxinMechanism::AcetylcholineReceptor,
            molecular_weight_da: mw,
            ld50_mg_per_kg: ld50,
            yield_mg: 1.0,
            onset_minutes: onset,
            duration_hours: 12.0,
            specificity: 0.8,
            chemical_class: ToxinChemicalClass::Peptide,
        }
    }

    #[test]
    fn test_toxin_construction_default_fields() {
        let t = make_toxin("Test", 1.0, 30.0, 5000.0);
        assert_eq!(t.name, "Test");
        assert_eq!(t.source, ToxinSource::SnakeElapidae);
        assert_eq!(t.venom_type, VenomType::Neurotoxin);
        assert_eq!(t.mechanism, ToxinMechanism::AcetylcholineReceptor);
        assert!((t.molecular_weight_da - 5000.0).abs() < 1e-5);
    }

    #[test]
    fn test_ld50_class_extremely_toxic() {
        let t = make_toxin("BoNT", 0.000001, 720.0, 150000.0);
        assert_eq!(t.ld50_class(), ToxicityRank::ExtremelyToxic);
        assert!(t.is_extremely_lethal());
    }

    #[test]
    fn test_ld50_class_supertoxic() {
        let t = make_toxin("TTX", 0.008, 20.0, 319.27);
        assert_eq!(t.ld50_class(), ToxicityRank::Supertoxic);
        assert!(!t.is_extremely_lethal());
    }

    #[test]
    fn test_ld50_class_highly_toxic() {
        let t = make_toxin("Melittin", 4.0, 5.0, 2846.5);
        assert_eq!(t.ld50_class(), ToxicityRank::HighlyToxic);
    }

    #[test]
    fn test_ld50_class_moderately_toxic() {
        let t = make_toxin("Morphine", 250.0, 30.0, 285.34);
        assert_eq!(t.ld50_class(), ToxicityRank::ModeratelyToxic);
    }

    #[test]
    fn test_ld50_class_slightly_toxic() {
        let t = make_toxin("SlightlyToxic", 750.0, 30.0, 200.0);
        assert_eq!(t.ld50_class(), ToxicityRank::SlightlyToxic);
    }

    #[test]
    fn test_ld50_class_harmless() {
        let t = make_toxin("Harmless", 5000.0, 30.0, 18.0);
        assert_eq!(t.ld50_class(), ToxicityRank::RelativelyHarmless);
        assert!(!t.is_extremely_lethal());
    }

    #[test]
    fn test_toxicity_category_labels() {
        assert_eq!(make_toxin("a", 0.000001, 1.0, 1.0).toxicity_category(), "极毒");
        assert_eq!(make_toxin("b", 0.5, 1.0, 1.0).toxicity_category(), "剧毒");
        assert_eq!(make_toxin("c", 25.0, 1.0, 1.0).toxicity_category(), "有毒");
        assert_eq!(make_toxin("d", 200.0, 1.0, 1.0).toxicity_category(), "中等毒");
        assert_eq!(make_toxin("e", 750.0, 1.0, 1.0).toxicity_category(), "微毒");
        assert_eq!(make_toxin("f", 5000.0, 1.0, 1.0).toxicity_category(), "无害");
    }

    #[test]
    fn test_onset_class_rapid() {
        let t = make_toxin("Rapid", 1.0, 30.0, 1000.0);
        assert_eq!(t.onset_class(), "Rapid");
        assert!(t.is_rapid_onset());
    }

    #[test]
    fn test_onset_class_intermediate() {
        let t = make_toxin("Inter", 1.0, 120.0, 1000.0);
        assert_eq!(t.onset_class(), "Intermediate");
        assert!(!t.is_rapid_onset());
    }

    #[test]
    fn test_onset_class_delayed() {
        let t = make_toxin("Delayed", 1.0, 2880.0, 1000.0);
        assert_eq!(t.onset_class(), "Delayed");
        assert!(!t.is_rapid_onset());
    }
    #[test]
    fn test_lethal_dose_for_70kg() {
        let t = make_toxin("X", 2.0, 30.0, 1000.0);
        assert!((t.lethal_dose_for_70kg() - 140.0).abs() < 1e-4);
        assert!((t.human_lethal_dose_mg() - 140.0).abs() < 1e-4);
    }

    #[test]
    fn test_is_protein_toxin_classification() {
        let protein = Toxin {
            chemical_class: ToxinChemicalClass::Protein,
            ..make_toxin("P", 1.0, 30.0, 50000.0)
        };
        let enzyme = Toxin {
            chemical_class: ToxinChemicalClass::Enzyme,
            ..make_toxin("E", 1.0, 30.0, 30000.0)
        };
        let peptide = Toxin {
            chemical_class: ToxinChemicalClass::Peptide,
            ..make_toxin("PE", 1.0, 30.0, 3000.0)
        };
        assert!(protein.is_protein_toxin());
        assert!(enzyme.is_protein_toxin());
        assert!(!peptide.is_protein_toxin());
    }

    #[test]
    fn test_is_small_molecule_threshold() {
        let small = make_toxin("TTX", 0.008, 20.0, 319.27);
        let big = make_toxin("Big", 1.0, 30.0, 50000.0);
        assert!(small.is_small_molecule());
        assert!(!big.is_small_molecule());
    }

    #[test]
    fn test_molar_mass_kda_conversion() {
        let t = make_toxin("X", 1.0, 30.0, 7820.0);
        assert!((t.molar_mass_kda() - 7.82).abs() < 1e-4);
    }

    #[test]
    fn test_yield_is_lethal_to_human() {
        let t = make_toxin("Y", 0.5, 30.0, 1000.0);
        // LD70 = 35 mg, yield = 1 mg → 不足以致死
        assert!(!t.yield_is_lethal_to_human());
        let mut t2 = t.clone();
        t2.yield_mg = 100.0;
        assert!(t2.yield_is_lethal_to_human());
    }

    #[test]
    fn test_safety_margin_ratio() {
        let t = make_toxin("X", 1.0, 30.0, 1000.0);
        // LD70 = 70 mg, yield = 1 mg → margin = 1/70
        let m = t.safety_margin();
        assert!((m - (1.0 / 70.0)).abs() < 1e-5);
    }

    #[test]
    fn test_targets_nervous_system() {
        let neuro = make_toxin("N", 1.0, 30.0, 1000.0);
        assert!(neuro.targets_nervous_system());
        let cyto = Toxin {
            venom_type: VenomType::Cytotoxin,
            ..make_toxin("C", 1.0, 30.0, 1000.0)
        };
        assert!(!cyto.targets_nervous_system());
    }

    #[test]
    fn test_antivenom_effective_match_and_mismatch() {
        let t = make_toxin("α-Cobratoxin", 0.4, 30.0, 7820.0);
        let av = Antivenom {
            name: "SAIMR".into(),
            target_toxins: vec!["α-Cobratoxin".into(), "Taipoxin".into()],
            origin_animal: "Horse (equine)".into(),
            antibody_type: AntibodyType::FAb2Fragment,
            effectiveness: 0.8,
            administration: "IV infusion".into(),
            onset_minutes: 60.0,
        };
        assert!(t.antivenom_effective(&av));

        let other = make_toxin("TTX", 0.008, 20.0, 319.27);
        assert!(!other.antivenom_effective(&av));
    }
    #[test]
    fn test_antivenom_is_polyvalent_and_monovalent() {
        let poly = Antivenom {
            name: "Poly".into(),
            target_toxins: vec!["A".into(), "B".into(), "C".into()],
            origin_animal: "Horse (equine)".into(),
            antibody_type: AntibodyType::WholeIgG,
            effectiveness: 0.7,
            administration: "IV".into(),
            onset_minutes: 60.0,
        };
        let mono = Antivenom {
            name: "Mono".into(),
            target_toxins: vec!["X".into()],
            origin_animal: "Human".into(),
            antibody_type: AntibodyType::WholeIgG,
            effectiveness: 0.9,
            administration: "IM".into(),
            onset_minutes: 30.0,
        };
        assert!(poly.is_polyvalent());
        assert!(!poly.is_monovalent());
        assert!(!mono.is_polyvalent());
        assert!(mono.is_monovalent());
    }

    #[test]
    fn test_antivenom_origin_classification() {
        let human_av = Antivenom {
            name: "BabyBIG".into(),
            target_toxins: vec!["Botulinum toxin (BoNT)".into()],
            origin_animal: "Human".into(),
            antibody_type: AntibodyType::WholeIgG,
            effectiveness: 0.9,
            administration: "IV".into(),
            onset_minutes: 60.0,
        };
        let horse_av = Antivenom {
            name: "CSL".into(),
            target_toxins: vec!["Taipoxin".into()],
            origin_animal: "Horse (equine)".into(),
            antibody_type: AntibodyType::WholeIgG,
            effectiveness: 0.88,
            administration: "IV".into(),
            onset_minutes: 60.0,
        };
        assert!(human_av.is_human_origin());
        assert!(!human_av.is_equine());
        assert!(!horse_av.is_human_origin());
        assert!(horse_av.is_equine());
    }

    #[test]
    fn test_antivenom_targets_toxin_name() {
        let av = Antivenom {
            name: "CroFab".into(),
            target_toxins: vec!["Crotalus atrox venom".into(), "Hemorrhagin".into()],
            origin_animal: "Sheep (ovine)".into(),
            antibody_type: AntibodyType::FabFragment,
            effectiveness: 0.85,
            administration: "IV".into(),
            onset_minutes: 60.0,
        };
        assert!(av.targets_toxin("Hemorrhagin"));
        assert!(!av.targets_toxin("α-Cobratoxin"));
    }

    #[test]
    fn test_antivenom_is_rapid_onset_and_whole_ig() {
        let av = Antivenom {
            name: "TIG".into(),
            target_toxins: vec!["Tetanospasmin (TeNT)".into()],
            origin_animal: "Human".into(),
            antibody_type: AntibodyType::WholeIgG,
            effectiveness: 0.95,
            administration: "IM".into(),
            onset_minutes: 30.0,
        };
        assert!(av.is_rapid_onset());
        assert!(av.is_whole_ig());
        assert!(!av.is_monoclonal());
    }

    #[test]
    fn test_toxin_effect_severity_classification() {
        let severe = ToxinEffect {
            target_system: PhysiologicalSystem::NervousSystem,
            effect_type: EffectType::Paralysis,
            severity: 0.9,
            reversible: false,
            recovery_time_hours: 96.0,
        };
        assert!(severe.is_severe());
        assert!(severe.is_life_threatening());
        assert!(!severe.is_recoverable());

        let moderate = ToxinEffect {
            target_system: PhysiologicalSystem::Hematologic,
            effect_type: EffectType::Bleeding,
            severity: 0.6,
            reversible: true,
            recovery_time_hours: 12.0,
        };
        assert!(!moderate.is_severe());
        assert!(!moderate.is_life_threatening());
        assert!(moderate.is_recoverable());
    }

    #[test]
    fn test_toxin_effects_for_sodium_channel_blocker() {
        let t = make_toxin("TTX", 0.008, 20.0, 319.27);
        let effects = t.effects();
        assert!(effects.iter().any(|e| e.target_system == PhysiologicalSystem::NervousSystem));
        assert!(effects.iter().any(|e| e.target_system == PhysiologicalSystem::Respiratory));
    }
    #[test]
    fn test_snake_venom_database_loaded() {
        let db = snake_venom_database();
        assert!(!db.is_empty());
        assert!(db.iter().any(|t| t.name == "α-Cobratoxin"));
        for t in &db {
            assert!(t.ld50_mg_per_kg > 0.0);
            assert!(t.ld50_mg_per_kg < 1000.0);
        }
    }

    #[test]
    fn test_microbial_toxin_database_extremely_toxic() {
        let db = microbial_toxin_database();
        let bont = db.iter().find(|t| t.name == "Botulinum toxin (BoNT)");
        assert!(bont.is_some());
        let bont = bont.unwrap();
        assert_eq!(bont.ld50_class(), ToxicityRank::ExtremelyToxic);
        assert!(bont.is_extremely_lethal());
        assert!(bont.is_protein_toxin());
    }

    #[test]
    fn test_antivenom_database_loaded() {
        let db = antivenom_database();
        assert!(!db.is_empty());
        let crofab = db.iter().find(|a| a.name == "CroFab");
        assert!(crofab.is_some());
        assert!(crofab.unwrap().is_polyvalent());
        // 至少存在一个单价抗血清
        assert!(db.iter().any(|a| a.is_monovalent()));
    }

    #[test]
    fn test_marine_venom_database_ttx_classification() {
        let db = marine_venom_database();
        let ttx = db.iter().find(|t| t.name == "Tetrodotoxin (TTX)");
        assert!(ttx.is_some());
        let ttx = ttx.unwrap();
        assert_eq!(ttx.ld50_class(), ToxicityRank::Supertoxic);
        assert!(ttx.is_small_molecule());
        assert!(ttx.targets_nervous_system());
        assert_eq!(ttx.onset_class(), "Rapid");
    }

    #[test]
    fn test_plant_toxin_ricin_ribosome_inactivator() {
        let db = plant_toxin_database();
        let ricin = db.iter().find(|t| t.name == "Ricin").unwrap();
        assert_eq!(ricin.mechanism, ToxinMechanism::RibosomeInactivator);
        assert!(ricin.is_protein_toxin());
        assert_eq!(ricin.ld50_class(), ToxicityRank::Supertoxic);
    }

    #[test]
    fn test_toxin_clone_preserves_fields() {
        let t = make_toxin("Original", 0.5, 30.0, 5000.0);
        let t2 = t.clone();
        assert_eq!(t.name, t2.name);
        assert_eq!(t.ld50_mg_per_kg, t2.ld50_mg_per_kg);
        assert_eq!(t.mechanism, t2.mechanism);
        assert_eq!(t.chemical_class, t2.chemical_class);
    }
}
