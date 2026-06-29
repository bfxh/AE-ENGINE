//! 扩展器官系统 —— 完整解剖学器官模型
//!
//! 数据来源：
//! - Guyton & Hall, "Textbook of Medical Physiology" (14th ed.)
//! - Tortora & Derrickson, "Principles of Anatomy and Physiology" (15th ed.)
//! - ICRP Publication 89: Adult Reference Computational Phantoms
//! - Tibbitts et al., "Adult human cell type counts"

use serde::{Deserialize, Serialize};

use crate::tissues::TissueType;

/// 解剖学器官类型 —— 32 种主要器官
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum OrganType {
    Heart,
    Lung,
    Liver,
    Kidney,
    Brain,
    Spleen,
    Stomach,
    SmallIntestine,
    LargeIntestine,
    Pancreas,
    Bladder,
    Gallbladder,
    Skin,
    Eye,
    Ear,
    Nose,
    Tongue,
    Esophagus,
    Trachea,
    Diaphragm,
    Uterus,
    Ovary,
    Testis,
    Prostate,
    Thyroid,
    Adrenal,
    Pituitary,
    Pineal,
    Thymus,
    LymphNode,
    Tonsil,
    BoneMarrow,
    SpinalCord,
}

/// 器官功能模型
///
/// 字段说明：
/// - `mass`: kg，成年男性参考值（Ref: ICRP 89）
/// - `blood_flow`: L/min，安静状态下心输出量 5 L/min 的分配
/// - `oxygen_consumption`: mL/min，安静耗氧分配
/// - `functional_capacity`: 0.0-1.0，健康=1.0
/// - `metabolic_rate`: W/kg
/// - `temperature`: K，深部体温 310.15 K (37°C)
/// - `cell_count`: 该器官估算细胞总数
/// - `primary_tissue`: 主要组织类型
/// - `failure_symptoms`: 功能衰竭的代表性临床表现
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganModel {
    pub organ_type: OrganType,
    pub mass: f32,
    pub blood_flow: f32,
    pub oxygen_consumption: f32,
    pub functional_capacity: f32,
    pub metabolic_rate: f32,
    pub temperature: f32,
    pub cell_count: u64,
    pub primary_tissue: TissueType,
    pub failure_symptoms: Vec<String>,
}

impl OrganModel {
    /// 标准人体深部体温（37 °C）
    pub const CORE_TEMPERATURE: f32 = 310.15;

    /// 创建健康成年男性参考器官
    ///
    /// 参考：心输出量 5 L/min，总耗氧 250 mL/min，肝/肾/脑/心为耗氧大户
    pub fn new(organ_type: OrganType) -> Self {
        let (mass, blood_flow, oxygen_consumption, metabolic_rate, cell_count, primary_tissue) =
            Self::reference_values(organ_type);
        Self {
            organ_type,
            mass,
            blood_flow,
            oxygen_consumption,
            functional_capacity: 1.0,
            metabolic_rate,
            temperature: Self::CORE_TEMPERATURE,
            cell_count,
            primary_tissue,
            failure_symptoms: Self::default_failure_symptoms(organ_type),
        }
    }

    /// 健康成年男性参考值
    ///
    /// 质量参考 ICRP 89；血流/耗氧参考 Guyton；代谢率参考 Elia 1992
    fn reference_values(organ_type: OrganType) -> (f32, f32, f32, f32, u64, TissueType) {
        match organ_type {
            // 心脏：0.3 kg，冠脉血流 225 mL/min，耗氧 30 mL/min
            OrganType::Heart => (
                0.30,
                0.225,
                30.0,
                11.0,
                2_000_000_000, // ~2e9 心肌细胞
                TissueType::MuscleCardiac,
            ),
            // 双肺：1.0 kg，血流 0.5 L/min，耗氧 25 mL/min
            OrganType::Lung => (
                1.00,
                0.50,
                25.0,
                1.5,
                5_000_000_000, // 含 3 亿肺泡细胞
                TissueType::EpithelialColumnar,
            ),
            // 肝脏：1.5 kg，血流 1.5 L/min（含门静脉），耗氧 60 mL/min（最高）
            OrganType::Liver => (
                1.50,
                1.50,
                60.0,
                14.0, // 代谢率最高
                240_000_000_000, // ~2.4e11 肝细胞
                TissueType::EpithelialColumnar,
            ),
            // 双肾：0.3 kg，血流 1.2 L/min（高灌注），耗氧 20 mL/min
            OrganType::Kidney => (
                0.30,
                1.20,
                20.0,
                12.0,
                2_000_000_000, // 含 200 万肾单位
                TissueType::EpithelialCuboidal,
            ),
            // 脑：1.4 kg，血流 0.75 L/min，耗氧 50 mL/min（占全身 20%）
            OrganType::Brain => (
                1.40,
                0.75,
                50.0,
                10.0,
                171_000_000_000, // 860 亿神经元 + 850 亿胶质
                TissueType::Nerve,
            ),
            // 脾：0.15 kg
            OrganType::Spleen => (
                0.15,
                0.30,
                8.0,
                4.0,
                100_000_000_000,
                TissueType::ConnectiveReticular,
            ),
            // 胃：0.15 kg
            OrganType::Stomach => (
                0.15,
                0.15,
                5.0,
                3.0,
                50_000_000_000,
                TissueType::MuscleSmooth,
            ),
            // 小肠：1.0 kg（含内容物）
            OrganType::SmallIntestine => (
                1.00,
                0.50,
                25.0,
                5.0,
                100_000_000_000, // ~1e11 上皮细胞
                TissueType::EpithelialColumnar,
            ),
            // 大肠：0.5 kg
            OrganType::LargeIntestine => (
                0.50,
                0.20,
                8.0,
                4.0,
                30_000_000_000,
                TissueType::EpithelialColumnar,
            ),
            // 胰腺：0.1 kg
            OrganType::Pancreas => (
                0.10,
                0.20,
                8.0,
                6.0,
                15_000_000_000, // 含 100 万胰岛
                TissueType::EpithelialCuboidal,
            ),
            // 膀胱：0.05 kg（充盈时更大）
            OrganType::Bladder => (
                0.05,
                0.05,
                1.0,
                2.0,
                1_000_000_000,
                TissueType::MuscleSmooth,
            ),
            // 胆囊：0.05 kg
            OrganType::Gallbladder => (
                0.05,
                0.05,
                1.0,
                2.0,
                100_000_000,
                TissueType::EpithelialColumnar,
            ),
            // 皮肤：5.0 kg（最大器官）
            OrganType::Skin => (
                5.00,
                0.50,
                10.0,
                0.3,
                20_000_000_000, // ~2e10 细胞
                TissueType::EpithelialSquamous,
            ),
            // 眼：8 g
            OrganType::Eye => (
                0.008,
                0.02,
                2.0,
                8.0,
                130_000_000, // 1.2 亿视杆 + 600 万视锥
                TissueType::ConnectiveDense,
            ),
            // 耳：20 g（含听小骨）
            OrganType::Ear => (
                0.02,
                0.02,
                0.5,
                1.0,
                50_000_000,
                TissueType::BoneCortical,
            ),
            // 鼻：30 g
            OrganType::Nose => (
                0.03,
                0.02,
                0.5,
                1.5,
                30_000_000,
                TissueType::EpithelialColumnar,
            ),
            // 舌：70 g
            OrganType::Tongue => (
                0.07,
                0.05,
                2.0,
                5.0,
                5_000_000_000,
                TissueType::MuscleSkeletal,
            ),
            // 食管：40 g
            OrganType::Esophagus => (
                0.04,
                0.05,
                1.0,
                2.0,
                2_000_000_000,
                TissueType::MuscleSmooth,
            ),
            // 气管：30 g
            OrganType::Trachea => (
                0.03,
                0.02,
                1.0,
                2.0,
                1_000_000_000,
                TissueType::CartilageHyaline,
            ),
            // 膈肌：200 g
            OrganType::Diaphragm => (
                0.20,
                0.05,
                3.0,
                5.0,
                2_000_000_000,
                TissueType::MuscleSkeletal,
            ),
            // 子宫：80 g（非孕期）
            OrganType::Uterus => (
                0.08,
                0.05,
                2.0,
                4.0,
                5_000_000_000,
                TissueType::MuscleSmooth,
            ),
            // 卵巢：10 g（单侧）
            OrganType::Ovary => (
                0.01,
                0.02,
                0.5,
                6.0,
                1_000_000, // 含 100 万原始卵泡
                TissueType::ConnectiveLoose,
            ),
            // 睾丸：25 g（单侧）
            OrganType::Testis => (
                0.025,
                0.01,
                0.5,
                4.0,
                600_000_000_000, // 含生殖细胞总数
                TissueType::EpithelialColumnar,
            ),
            // 前列腺：20 g
            OrganType::Prostate => (
                0.02,
                0.01,
                0.5,
                3.0,
                1_000_000_000,
                TissueType::MuscleSmooth,
            ),
            // 甲状腺：20 g
            OrganType::Thyroid => (
                0.02,
                0.10,
                4.0,
                7.0,
                30_000_000_000, // 含无数滤泡
                TissueType::EpithelialCuboidal,
            ),
            // 肾上腺：14 g（双侧）
            OrganType::Adrenal => (
                0.014,
                0.02,
                2.0,
                10.0,
                1_000_000_000,
                TissueType::EpithelialColumnar,
            ),
            // 垂体：0.5 g
            OrganType::Pituitary => (
                0.0005,
                0.005,
                0.2,
                15.0,
                5_000_000,
                TissueType::EpithelialCuboidal,
            ),
            // 松果体：0.2 g
            OrganType::Pineal => (
                0.0002,
                0.001,
                0.05,
                12.0,
                1_000_000,
                TissueType::ConnectiveLoose,
            ),
            // 胸腺：25 g（成年后萎缩）
            OrganType::Thymus => (
                0.025,
                0.02,
                1.0,
                5.0,
                100_000_000_000,
                TissueType::EpithelialCuboidal,
            ),
            // 单个淋巴结：5 g（全身约 5 g × 500-700 个 = 2.5-3.5 kg？这里取单器官值）
            OrganType::LymphNode => (
                0.005,
                0.005,
                0.2,
                4.0,
                1_000_000_000,
                TissueType::ConnectiveReticular,
            ),
            // 扁桃体：1 g
            OrganType::Tonsil => (
                0.001,
                0.001,
                0.05,
                4.0,
                1_000_000_000,
                TissueType::EpithelialSquamous,
            ),
            // 骨髓：3.0 kg（全身红+黄骨髓）
            OrganType::BoneMarrow => (
                3.00,
                0.30,
                15.0,
                4.0,
                1_000_000_000_000, // ~1e12 细胞
                TissueType::ConnectiveReticular,
            ),
            // 脊髓：35 g
            OrganType::SpinalCord => (
                0.035,
                0.02,
                2.0,
                8.0,
                1_000_000_000,
                TissueType::Nerve,
            ),
        }
    }

    /// 器官功能衰竭的代表性临床表现
    fn default_failure_symptoms(organ_type: OrganType) -> Vec<String> {
        match organ_type {
            OrganType::Heart => vec![
                "dyspnea".into(),
                "edema".into(),
                "fatigue".into(),
                "syncope".into(),
            ],
            OrganType::Lung => vec![
                "hypoxemia".into(),
                "hypercapnia".into(),
                "dyspnea".into(),
                "cyanosis".into(),
            ],
            OrganType::Liver => vec![
                "jaundice".into(),
                "coagulopathy".into(),
                "encephalopathy".into(),
                "ascites".into(),
            ],
            OrganType::Kidney => vec![
                "uremia".into(),
                "hyperkalemia".into(),
                "acidosis".into(),
                "oliguria".into(),
            ],
            OrganType::Brain => vec![
                "coma".into(),
                "seizure".into(),
                "aphasia".into(),
                "paralysis".into(),
            ],
            OrganType::Spleen => {
                vec![
                    "immunodeficiency".into(),
                    "thrombocytosis".into(),
                    "sepsis_risk".into(),
                ]
            }
            OrganType::Stomach => {
                vec![
                    "dyspepsia".into(),
                    "hematemesis".into(),
                    "achlorhydria".into(),
                ]
            }
            OrganType::SmallIntestine => {
                vec![
                    "malabsorption".into(),
                    "diarrhea".into(),
                    "dehydration".into(),
                ]
            }
            OrganType::LargeIntestine => {
                vec![
                    "constipation".into(),
                    "obstruction".into(),
                    "melena".into(),
                ]
            }
            OrganType::Pancreas => {
                vec![
                    "diabetes".into(),
                    "steatorrhea".into(),
                    "pancreatitis".into(),
                ]
            }
            OrganType::Bladder => vec!["retention".into(), "incontinence".into()],
            OrganType::Gallbladder => vec!["biliary_colic".into(), "cholecystitis".into()],
            OrganType::Skin => vec![
                "thermoregulatory_failure".into(),
                "infection_risk".into(),
                "fluid_loss".into(),
            ],
            OrganType::Eye => vec!["blindness".into(), "glaucoma".into(), "cataract".into()],
            OrganType::Ear => vec!["deafness".into(), "vertigo".into(), "tinnitus".into()],
            OrganType::Nose => vec!["anosmia".into(), "obstruction".into()],
            OrganType::Tongue => vec!["dysgeusia".into(), "dysphagia".into()],
            OrganType::Esophagus => vec!["dysphagia".into(), "reflux".into()],
            OrganType::Trachea => vec!["stridor".into(), "obstruction".into()],
            OrganType::Diaphragm => {
                vec![
                    "respiratory_failure".into(),
                    "paradoxical_motion".into(),
                ]
            }
            OrganType::Uterus => vec!["infertility".into(), "hemorrhage".into()],
            OrganType::Ovary => vec!["infertility".into(), "hormone_imbalance".into()],
            OrganType::Testis => vec!["infertility".into(), "hypogonadism".into()],
            OrganType::Prostate => vec!["retention".into(), "hematuria".into()],
            OrganType::Thyroid => {
                vec![
                    "hypothyroidism".into(),
                    "hyperthyroidism".into(),
                    "goiter".into(),
                ]
            }
            OrganType::Adrenal => vec![
                "addisons_disease".into(),
                "cushings_syndrome".into(),
                "adrenal_crisis".into(),
            ],
            OrganType::Pituitary => vec![
                "hypopituitarism".into(),
                "acromegaly".into(),
                "diabetes_insipidus".into(),
            ],
            OrganType::Pineal => vec!["circadian_dysregulation".into()],
            OrganType::Thymus => vec!["immunodeficiency".into(), "myasthenia_gravis".into()],
            OrganType::LymphNode => vec!["lymphedema".into(), "immunodeficiency".into()],
            OrganType::Tonsil => vec!["recurrent_pharyngitis".into()],
            OrganType::BoneMarrow => vec![
                "pancytopenia".into(),
                "anemia".into(),
                "immunodeficiency".into(),
            ],
            OrganType::SpinalCord => vec![
                "paraplegia".into(),
                "sensory_loss".into(),
                "bowel_bladder_dysfunction".into(),
            ],
        }
    }

    /// 功能是否健全
    pub fn is_functional(&self) -> bool {
        self.functional_capacity > 0.2
    }

    /// 器官总代谢功率（W）
    pub fn total_metabolic_power(&self) -> f32 {
        self.mass * self.metabolic_rate
    }

    /// 血流灌注比例（相对心输出量 5 L/min）
    pub fn perfusion_fraction(&self) -> f32 {
        self.blood_flow / 5.0
    }
}

impl OrganType {
    /// 器官特有结构 —— 列出该器官的关键解剖结构
    ///
    /// 用于在仿真/教学场景中暴露器官的精细解剖结构
    pub fn special_structures(&self) -> Vec<&'static str> {
        match self {
            Self::Heart => vec![
                "4 chambers (RA/RV/LA/LV)",
                "4 valves (tricuspid/pulmonary/mitral/aortic)",
                "coronary arteries (LAD/LCx/RCA)",
                "conduction system (SA node/AV node/Purkinje fibers)",
            ],
            Self::Lung => vec![
                "23-generation bronchial tree",
                "~300 million alveoli",
                "gas exchange membrane (0.2-2.5 µm thick)",
                "visceral pleura",
            ],
            Self::Liver => vec![
                "hexagonal lobules",
                "Kupffer cells (resident macrophages)",
                "bile canaliculi and ducts",
                "dual blood supply (hepatic artery + portal vein)",
                "regenerative capacity (70% resectable)",
            ],
            Self::Kidney => vec![
                "~1 million nephrons per kidney",
                "glomerular filtration rate 125 mL/min",
                "loop of Henle (countercurrent multiplier)",
                "juxtaglomerular apparatus (renin secretion)",
            ],
            Self::Brain => vec![
                "~86 billion neurons",
                "~85 billion glial cells",
                "blood-brain barrier (tight junctions + astrocytes)",
                "four ventricles + cerebrospinal fluid",
                "cerebral cortex (6 layers)",
            ],
            Self::Spleen => vec![
                "white pulp (lymphoid)",
                "red pulp (sinusoidal filtration)",
                "marginal zone",
            ],
            Self::Stomach => vec![
                "cardiac/fundic/body/pyloric regions",
                "gastric glands (parietal/chief/G cells)",
                "mucosal barrier (HCO3- + mucus)",
            ],
            Self::SmallIntestine => vec![
                "duodenum/jejunum/ileum",
                "villi and microvilli (brush border)",
                "Peyer patches (GALT)",
                "crypts of Lieberkuhn",
            ],
            Self::LargeIntestine => vec![
                "cecum/colon/rectum",
                "teniae coli",
                "haustra",
                "gut microbiota (10^12 bacteria)",
            ],
            Self::Pancreas => vec![
                "exocrine acini (digestive enzymes)",
                "endocrine islets of Langerhans (alpha/beta/delta/PP)",
                "ductal system",
            ],
            Self::Bladder => vec![
                "detrusor muscle",
                "trigone",
                "internal/external sphincters",
            ],
            Self::Gallbladder => vec!["rugae", "cystic duct (Heister valves)"],
            Self::Skin => vec![
                "epidermis (5 strata)",
                "dermis (papillary/reticular)",
                "hypodermis (subcutaneous)",
                "sweat glands (eccrine/apocrine)",
                "hair follicles + sebaceous glands",
            ],
            Self::Eye => vec![
                "cornea",
                "aqueous humor (anterior/posterior chambers)",
                "lens",
                "vitreous humor",
                "retina (120 million rods + 6 million cones)",
                "fovea centralis",
            ],
            Self::Ear => vec![
                "outer ear (pinna/ear canal)",
                "middle ear (malleus/incus/stapes)",
                "inner ear cochlea (~15000 hair cells)",
                "vestibular system (utricle/saccule/semicircular canals)",
            ],
            Self::Nose => vec![
                "olfactory epithelium",
                "turbinates",
                "paranasal sinuses",
            ],
            Self::Tongue => vec![
                "taste buds (fungiform/foliate/circumvallate)",
                "intrinsic and extrinsic muscles",
            ],
            Self::Esophagus => vec![
                "upper/lower esophageal sphincters",
                "mucosal/glandular layers",
            ],
            Self::Trachea => vec![
                "C-shaped cartilage rings (16-20)",
                "trachealis muscle",
                "ciliated pseudostratified epithelium",
            ],
            Self::Diaphragm => vec![
                "central tendon",
                "crura (right/left)",
                "apertures (aortic/esophageal/vena caval)",
            ],
            Self::Uterus => vec![
                "fundus/body/cervix",
                "endometrium (basalis/functionalis)",
                "myometrium (3 smooth muscle layers)",
            ],
            Self::Ovary => vec![
                "ovarian follicles (primordial/primary/secondary/Graafian)",
                "corpus luteum",
                "cortex and medulla",
            ],
            Self::Testis => vec![
                "seminiferous tubules (~600 per testis)",
                "Sertoli cells",
                "Leydig cells (interstitial)",
                "blood-testis barrier",
            ],
            Self::Prostate => vec![
                "peripheral/central/transition zones",
                "prostatic secretions (PSA)",
            ],
            Self::Thyroid => vec![
                "follicles (colloid)",
                "C cells (parafollicular, calcitonin)",
                "isthmus + two lobes",
            ],
            Self::Adrenal => vec![
                "cortex (glomerulosa/fasciculata/reticularis)",
                "medulla (chromaffin cells)",
            ],
            Self::Pituitary => vec![
                "anterior lobe (adenohypophysis)",
                "posterior lobe (neurohypophysis)",
                "hypothalamic-pituitary portal system",
            ],
            Self::Pineal => vec!["pinealocytes", "brain sand (corpora arenacea)"],
            Self::Thymus => vec![
                "cortex (dense T-cell precursors)",
                "medulla (Hassall corpuscles)",
                "thymopoiesis",
            ],
            Self::LymphNode => vec![
                "cortex (follicles)",
                "paracortex (T-cell zone)",
                "medulla (sinuses)",
                "afferent/efferent lymphatics",
            ],
            Self::Tonsil => vec!["crypts", "lymphoid nodules"],
            Self::BoneMarrow => vec![
                "red marrow (hematopoietic)",
                "yellow marrow (adipose)",
                "hematopoietic stem cells",
                "stromal niche",
            ],
            Self::SpinalCord => vec![
                "31 segments (8C/12T/5L/5S/1Co)",
                "dorsal/ventral roots",
                "gray matter horns",
                "ascending/descending tracts",
            ],
        }
    }
}

impl Default for OrganModel {
    fn default() -> Self {
        Self::new(OrganType::Heart)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- Default 实现 ----------

    #[test]
    fn test_default_returns_heart_model() {
        let m = OrganModel::default();
        assert_eq!(m.organ_type, OrganType::Heart);
    }

    #[test]
    fn test_default_has_full_functional_capacity() {
        let m = OrganModel::default();
        assert!((m.functional_capacity - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_default_uses_core_temperature_constant() {
        let m = OrganModel::default();
        assert!((m.temperature - OrganModel::CORE_TEMPERATURE).abs() < 1e-6);
        assert!((OrganModel::CORE_TEMPERATURE - 310.15).abs() < 1e-6);
    }

    // ---------- new() 构造器返回值 ----------

    #[test]
    fn test_new_heart_mass_matches_reference() {
        let h = OrganModel::new(OrganType::Heart);
        assert!((h.mass - 0.30).abs() < 1e-6);
        assert!((h.blood_flow - 0.225).abs() < 1e-6);
        assert!((h.oxygen_consumption - 30.0).abs() < 1e-6);
        assert_eq!(h.primary_tissue, TissueType::MuscleCardiac);
    }

    #[test]
    fn test_new_liver_has_highest_oxygen_consumption() {
        let liver = OrganModel::new(OrganType::Liver).oxygen_consumption;
        let brain = OrganModel::new(OrganType::Brain).oxygen_consumption;
        let heart = OrganModel::new(OrganType::Heart).oxygen_consumption;
        assert!(liver > brain);
        assert!(liver > heart);
        assert!((liver - 60.0).abs() < 1e-6);
    }

    #[test]
    fn test_new_brain_uses_nerve_tissue() {
        let b = OrganModel::new(OrganType::Brain);
        assert_eq!(b.primary_tissue, TissueType::Nerve);
        assert!((b.mass - 1.40).abs() < 1e-6);
    }

    #[test]
    fn test_new_skin_is_largest_organ_by_mass() {
        let skin_mass = OrganModel::new(OrganType::Skin).mass;
        let liver_mass = OrganModel::new(OrganType::Liver).mass;
        let marrow_mass = OrganModel::new(OrganType::BoneMarrow).mass;
        assert!(skin_mass > liver_mass);
        // 皮肤 5.0kg 是最大器官，大于骨髓 3.0kg
        assert!(skin_mass > marrow_mass);
        assert!((skin_mass - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_new_bone_marrow_has_highest_cell_count() {
        let marrow = OrganModel::new(OrganType::BoneMarrow).cell_count;
        let brain = OrganModel::new(OrganType::Brain).cell_count;
        let liver = OrganModel::new(OrganType::Liver).cell_count;
        assert!(marrow > brain);
        assert!(marrow > liver);
        assert_eq!(marrow, 1_000_000_000_000);
    }

    #[test]
    fn test_new_pineal_is_smallest_by_mass() {
        let pit = OrganModel::new(OrganType::Pituitary).mass;
        let pineal = OrganModel::new(OrganType::Pineal).mass;
        let tonsil = OrganModel::new(OrganType::Tonsil).mass;
        // 松果体 0.2g 是最小器官，小于垂体 0.5g 和扁桃体 1g
        assert!(pineal < pit);
        assert!(pineal < tonsil);
        assert!((pineal - 0.0002).abs() < 1e-9);
    }

    #[test]
    fn test_new_sets_failure_symptoms_nonempty() {
        for organ in [
            OrganType::Heart,
            OrganType::Pineal,
            OrganType::Tonsil,
            OrganType::SpinalCord,
        ] {
            let m = OrganModel::new(organ);
            assert!(!m.failure_symptoms.is_empty(),
                "organ {:?} should have failure symptoms", organ);
        }
    }

    // ---------- is_functional 行为 ----------

    #[test]
    fn test_is_functional_true_for_healthy_organ() {
        let m = OrganModel::new(OrganType::Kidney);
        assert!(m.is_functional());
    }

    #[test]
    fn test_is_functional_false_when_capacity_below_threshold() {
        let mut m = OrganModel::new(OrganType::Heart);
        m.functional_capacity = 0.19;
        assert!(!m.is_functional());
    }

    #[test]
    fn test_is_functional_false_at_exact_threshold() {
        // 阈值是 > 0.2，严格大于，0.2 应判定为不健全
        let mut m = OrganModel::new(OrganType::Lung);
        m.functional_capacity = 0.2;
        assert!(!m.is_functional());
    }

    #[test]
    fn test_is_functional_true_just_above_threshold() {
        let mut m = OrganModel::new(OrganType::Lung);
        m.functional_capacity = 0.2001;
        assert!(m.is_functional());
    }

    #[test]
    fn test_is_functional_false_at_zero_capacity() {
        let mut m = OrganModel::new(OrganType::Liver);
        m.functional_capacity = 0.0;
        assert!(!m.is_functional());
    }

    // ---------- total_metabolic_power 行为 ----------

    #[test]
    fn test_total_metabolic_power_calculation() {
        let m = OrganModel::new(OrganType::Heart);
        let expected = m.mass * m.metabolic_rate;
        assert!((m.total_metabolic_power() - expected).abs() < 1e-6);
        // 0.30 kg * 11.0 W/kg = 3.3 W
        assert!((m.total_metabolic_power() - 3.3).abs() < 1e-5);
    }

    #[test]
    fn test_total_metabolic_power_zero_when_mass_zero() {
        let mut m = OrganModel::new(OrganType::Skin);
        m.mass = 0.0;
        assert!((m.total_metabolic_power() - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_total_metabolic_power_zero_when_rate_zero() {
        let mut m = OrganModel::new(OrganType::Pancreas);
        m.metabolic_rate = 0.0;
        assert!((m.total_metabolic_power() - 0.0).abs() < 1e-9);
    }

    // ---------- perfusion_fraction 行为 ----------

    #[test]
    fn test_perfusion_fraction_uses_5_lpm_denominator() {
        let m = OrganModel::new(OrganType::Kidney);
        // 1.2 / 5.0
        assert!((m.perfusion_fraction() - 0.24).abs() < 1e-6);
    }

    #[test]
    fn test_perfusion_fraction_zero_when_no_flow() {
        let mut m = OrganModel::new(OrganType::Spleen);
        m.blood_flow = 0.0;
        assert!((m.perfusion_fraction() - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_perfusion_fraction_calculation_matches_formula() {
        let m = OrganModel::new(OrganType::Liver);
        let expected = m.blood_flow / 5.0;
        assert!((m.perfusion_fraction() - expected).abs() < 1e-6);
    }

    // ---------- special_structures 行为 ----------

    #[test]
    fn test_special_structures_heart_mentions_chambers_and_valves() {
        let s = OrganType::Heart.special_structures();
        assert!(s.iter().any(|x| x.contains("chambers")));
        assert!(s.iter().any(|x| x.contains("valves")));
        assert!(s.iter().any(|x| x.contains("coronary")));
        assert!(s.len() >= 4);
    }

    #[test]
    fn test_special_structures_brain_mentions_neurons_and_barrier() {
        let s = OrganType::Brain.special_structures();
        assert!(s.iter().any(|x| x.contains("neurons")));
        assert!(s.iter().any(|x| x.contains("blood-brain barrier")));
    }

    #[test]
    fn test_special_structures_all_variants_nonempty() {
        let all = [
            OrganType::Heart, OrganType::Lung, OrganType::Liver, OrganType::Kidney,
            OrganType::Brain, OrganType::Spleen, OrganType::Stomach, OrganType::SmallIntestine,
            OrganType::LargeIntestine, OrganType::Pancreas, OrganType::Bladder,
            OrganType::Gallbladder, OrganType::Skin, OrganType::Eye, OrganType::Ear,
            OrganType::Nose, OrganType::Tongue, OrganType::Esophagus, OrganType::Trachea,
            OrganType::Diaphragm, OrganType::Uterus, OrganType::Ovary, OrganType::Testis,
            OrganType::Prostate, OrganType::Thyroid, OrganType::Adrenal, OrganType::Pituitary,
            OrganType::Pineal, OrganType::Thymus, OrganType::LymphNode, OrganType::Tonsil,
            OrganType::BoneMarrow, OrganType::SpinalCord,
        ];
        for o in all {
            assert!(!o.special_structures().is_empty(),
                "organ {:?} should expose special_structures", o);
        }
    }

    // ---------- failure_symptoms 覆盖度 ----------

    #[test]
    fn test_failure_symptoms_nonempty_for_all_variants() {
        let all = [
            OrganType::Heart, OrganType::Lung, OrganType::Liver, OrganType::Kidney,
            OrganType::Brain, OrganType::Spleen, OrganType::Stomach, OrganType::SmallIntestine,
            OrganType::LargeIntestine, OrganType::Pancreas, OrganType::Bladder,
            OrganType::Gallbladder, OrganType::Skin, OrganType::Eye, OrganType::Ear,
            OrganType::Nose, OrganType::Tongue, OrganType::Esophagus, OrganType::Trachea,
            OrganType::Diaphragm, OrganType::Uterus, OrganType::Ovary, OrganType::Testis,
            OrganType::Prostate, OrganType::Thyroid, OrganType::Adrenal, OrganType::Pituitary,
            OrganType::Pineal, OrganType::Thymus, OrganType::LymphNode, OrganType::Tonsil,
            OrganType::BoneMarrow, OrganType::SpinalCord,
        ];
        for o in all {
            let m = OrganModel::new(o);
            assert!(!m.failure_symptoms.is_empty(),
                "organ {:?} should have failure symptoms", o);
        }
    }

    // ---------- 全部变体可构造（无 panic） ----------

    #[test]
    fn test_all_organ_variants_can_be_constructed() {
        let all = [
            OrganType::Heart, OrganType::Lung, OrganType::Liver, OrganType::Kidney,
            OrganType::Brain, OrganType::Spleen, OrganType::Stomach, OrganType::SmallIntestine,
            OrganType::LargeIntestine, OrganType::Pancreas, OrganType::Bladder,
            OrganType::Gallbladder, OrganType::Skin, OrganType::Eye, OrganType::Ear,
            OrganType::Nose, OrganType::Tongue, OrganType::Esophagus, OrganType::Trachea,
            OrganType::Diaphragm, OrganType::Uterus, OrganType::Ovary, OrganType::Testis,
            OrganType::Prostate, OrganType::Thyroid, OrganType::Adrenal, OrganType::Pituitary,
            OrganType::Pineal, OrganType::Thymus, OrganType::LymphNode, OrganType::Tonsil,
            OrganType::BoneMarrow, OrganType::SpinalCord,
        ];
        for o in all {
            let m = OrganModel::new(o);
            // 健康器官功能容量为 1.0
            assert!((m.functional_capacity - 1.0).abs() < 1e-6);
            // 体温恒定 37°C
            assert!((m.temperature - 310.15).abs() < 1e-6);
            // 健康判定为 true
            assert!(m.is_functional());
            // 质量与代谢率均为正
            assert!(m.mass > 0.0);
            assert!(m.metabolic_rate > 0.0);
            assert!(m.blood_flow > 0.0);
            assert!(m.oxygen_consumption > 0.0);
            assert!(m.cell_count > 0);
        }
    }
}