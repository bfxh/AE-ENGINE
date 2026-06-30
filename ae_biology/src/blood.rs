//! 血液系统模块
//!
//! 基于: Hoffbrand & Pettit, Essential Haematology (7th Edition)
//! 参考: ABO 血型系统 (Landsteiner, 1900)、Rh 因子、贫血分类诊断、
//!       血红蛋白携氧能力 (1 g Hb 结合 1.34 ml O2)
//! 单位约定: 血红蛋白 g/dL, 红细胞 10⁶/µL, 白细胞 /µL, 血小板 /µL,
//!           血浆 L, 铁 µg/dL, B12 pg/mL, 叶酸 ng/mL
//!
//! ABO/Rh 兼容性原则:
//!   - 受血者血浆中的抗体不得与供血者红细胞抗原反应
//!   - A 型血含抗-B, B 型血含抗-A, O 型血含抗-A 和抗-B, AB 型血无 ABO 抗体
//!   - Rh- 受血者不应接受 Rh+ 血液

use serde::{Deserialize, Serialize};

/// ABO/Rh 血型 (8 种组合)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BloodType {
    APositive,
    ANegative,
    BPositive,
    BNegative,
    ABPositive,
    ABNegative,
    OPositive,
    ONegative,
}

/// 血液成分类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BloodCellType {
    /// 红细胞
    RBC,
    /// 白细胞
    WBC,
    /// 血小板
    Platelet,
    /// 血浆
    Plasma,
}

/// 贫血类型分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnemiaType {
    /// 无贫血 (Hb ≥ 13.5)
    None,
    /// 缺铁性贫血 (铁 < 50 µg/dL)
    IronDeficiency,
    /// 维生素 B12 缺乏 (B12 < 200 pg/mL)
    B12Deficiency,
    /// 叶酸缺乏 (叶酸 < 3 ng/mL)
    FolateDeficiency,
    /// 再生障碍性贫血 (RBC < 2.0)
    Aplastic,
    /// 溶血性贫血 (其它正常但 Hb 低)
    Hemolytic,
}

impl BloodType {
    /// 是否含 A 抗原
    pub fn has_a_antigen(&self) -> bool {
        matches!(
            self,
            BloodType::APositive
                | BloodType::ANegative
                | BloodType::ABPositive
                | BloodType::ABNegative
        )
    }

    /// 是否含 B 抗原
    pub fn has_b_antigen(&self) -> bool {
        matches!(
            self,
            BloodType::BPositive
                | BloodType::BNegative
                | BloodType::ABPositive
                | BloodType::ABNegative
        )
    }

    /// 是否含 Rh(D) 抗原
    pub fn has_rh_antigen(&self) -> bool {
        matches!(
            self,
            BloodType::APositive
                | BloodType::BPositive
                | BloodType::ABPositive
                | BloodType::OPositive
        )
    }

    /// 供血者红细胞能否安全输给该受血者
    /// 规则: 供血者红细胞抗原不得与受血者血浆抗体冲突
    pub fn can_receive_from(&self, donor: &BloodType) -> bool {
        donor.can_donate_to(self)
    }

    /// 该血型作为供血者能否输给 recipient
    pub fn can_donate_to(&self, recipient: &BloodType) -> bool {
        // 供血者红细胞有 A 抗原 → 受血者不能有抗-A → 受血者必须有 A 抗原
        if self.has_a_antigen() && !recipient.has_a_antigen() {
            return false;
        }
        // 供血者红细胞有 B 抗原 → 受血者必须有 B 抗原
        if self.has_b_antigen() && !recipient.has_b_antigen() {
            return false;
        }
        // Rh+ 供血者只能输给 Rh+ 受血者
        if self.has_rh_antigen() && !recipient.has_rh_antigen() {
            return false;
        }
        true
    }
}

/// 血液系统状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloodSystem {
    /// 血红蛋白 (g/dL)
    pub hemoglobin: f32,
    /// 红细胞压积 (百分比)
    pub hematocrit: f32,
    /// 红细胞计数 (10⁶/µL)
    pub red_blood_cells: f32,
    /// 白细胞计数 (/µL)
    pub white_blood_cells: f32,
    /// 血小板计数 (/µL)
    pub platelets: f32,
    /// 血浆容量 (L)
    pub plasma_volume: f32,
    /// 血型
    pub blood_type: BloodType,
    /// 血清铁 (µg/dL)
    pub iron_level: f32,
    /// 维生素 B12 (pg/mL)
    pub b12_level: f32,
    /// 叶酸 (ng/mL)
    pub folate_level: f32,
}

impl BloodSystem {
    /// 创建健康成人默认血液系统
    /// Hb=15, Hct=45%, RBC=5.0, WBC=7000, PLT=250000, 血型 O+
    pub fn new() -> Self {
        Self {
            hemoglobin: 15.0,
            hematocrit: 45.0,
            red_blood_cells: 5.0,
            white_blood_cells: 7000.0,
            platelets: 250000.0,
            plasma_volume: 2.7,
            blood_type: BloodType::OPositive,
            iron_level: 100.0,
            b12_level: 500.0,
            folate_level: 10.0,
        }
    }

    /// 贫血分类诊断 (基于 Hb 阈值 13.5 与营养指标)
    pub fn classify_anemia(&self) -> AnemiaType {
        if self.hemoglobin >= 13.5 {
            return AnemiaType::None;
        }
        if self.iron_level < 50.0 {
            AnemiaType::IronDeficiency
        } else if self.b12_level < 200.0 {
            AnemiaType::B12Deficiency
        } else if self.folate_level < 3.0 {
            AnemiaType::FolateDeficiency
        } else if self.red_blood_cells < 2.0 {
            AnemiaType::Aplastic
        } else {
            AnemiaType::Hemolytic
        }
    }

    /// 血红蛋白理论携氧能力 (ml O2/dL blood)
    /// 每 g Hb 结合 1.34 ml O2
    pub fn oxygen_carrying_capacity(&self) -> f32 {
        self.hemoglobin * 1.34
    }

    /// 每帧更新: 血细胞与营养指标缓慢回归基线 (RBC 寿命 ~120 天)
    pub fn update(&mut self, dt: f32) {
        let baseline_rbc = 5.0;
        let baseline_hb = 15.0;
        self.red_blood_cells += (baseline_rbc - self.red_blood_cells) * 0.0001 * dt;
        self.hemoglobin += (baseline_hb - self.hemoglobin) * 0.0001 * dt;
        // Hct ≈ 3 × Hb
        self.hematocrit = self.hemoglobin * 3.0;
        // 营养指标向基线恢复
        self.iron_level += (100.0 - self.iron_level) * 0.0005 * dt;
        self.b12_level += (500.0 - self.b12_level) * 0.0005 * dt;
        self.folate_level += (10.0 - self.folate_level) * 0.0005 * dt;
    }
}

impl Default for BloodSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_hemoglobin() {
        let sys = BloodSystem::new();
        assert_eq!(sys.hemoglobin, 15.0);
    }

    #[test]
    fn test_default_hematocrit() {
        let sys = BloodSystem::new();
        assert_eq!(sys.hematocrit, 45.0);
    }

    #[test]
    fn test_default_rbc() {
        let sys = BloodSystem::new();
        assert_eq!(sys.red_blood_cells, 5.0);
    }

    #[test]
    fn test_default_iron_level() {
        let sys = BloodSystem::new();
        assert_eq!(sys.iron_level, 100.0);
    }

    #[test]
    fn test_classify_no_anemia() {
        let sys = BloodSystem::new();
        assert_eq!(sys.classify_anemia(), AnemiaType::None);
    }

    #[test]
    fn test_classify_iron_deficiency() {
        let mut sys = BloodSystem::new();
        sys.hemoglobin = 10.0;
        sys.iron_level = 30.0;
        assert_eq!(sys.classify_anemia(), AnemiaType::IronDeficiency);
    }

    #[test]
    fn test_classify_b12_deficiency() {
        let mut sys = BloodSystem::new();
        sys.hemoglobin = 10.0;
        sys.iron_level = 100.0; // 铁正常
        sys.b12_level = 150.0;
        assert_eq!(sys.classify_anemia(), AnemiaType::B12Deficiency);
    }

    #[test]
    fn test_classify_folate_deficiency() {
        let mut sys = BloodSystem::new();
        sys.hemoglobin = 10.0;
        sys.iron_level = 100.0;
        sys.b12_level = 500.0;
        sys.folate_level = 2.0;
        assert_eq!(sys.classify_anemia(), AnemiaType::FolateDeficiency);
    }

    #[test]
    fn test_classify_aplastic() {
        let mut sys = BloodSystem::new();
        sys.hemoglobin = 7.0;
        sys.iron_level = 100.0;
        sys.b12_level = 500.0;
        sys.folate_level = 10.0;
        sys.red_blood_cells = 1.5;
        assert_eq!(sys.classify_anemia(), AnemiaType::Aplastic);
    }

    #[test]
    fn test_classify_hemolytic() {
        let mut sys = BloodSystem::new();
        sys.hemoglobin = 9.0;
        sys.iron_level = 100.0;
        sys.b12_level = 500.0;
        sys.folate_level = 10.0;
        sys.red_blood_cells = 4.0; // RBC 不算极低
        assert_eq!(sys.classify_anemia(), AnemiaType::Hemolytic);
    }

    #[test]
    fn test_oxygen_carrying_capacity() {
        let sys = BloodSystem::new();
        // 15 * 1.34 = 20.1
        assert!((sys.oxygen_carrying_capacity() - 20.1).abs() < 0.01);
    }

    #[test]
    fn test_o_negative_universal_donor() {
        let o_neg = BloodType::ONegative;
        let all_types = [
            BloodType::APositive,
            BloodType::ANegative,
            BloodType::BPositive,
            BloodType::BNegative,
            BloodType::ABPositive,
            BloodType::ABNegative,
            BloodType::OPositive,
            BloodType::ONegative,
        ];
        for recipient in &all_types {
            assert!(
                o_neg.can_donate_to(recipient),
                "O- 应能输给所有血型, 失败于 {:?}",
                recipient
            );
        }
    }

    #[test]
    fn test_ab_positive_universal_recipient() {
        let ab_pos = BloodType::ABPositive;
        let all_types = [
            BloodType::APositive,
            BloodType::ANegative,
            BloodType::BPositive,
            BloodType::BNegative,
            BloodType::ABPositive,
            BloodType::ABNegative,
            BloodType::OPositive,
            BloodType::ONegative,
        ];
        for donor in &all_types {
            assert!(
                ab_pos.can_receive_from(donor),
                "AB+ 应能接受所有血型, 失败于 {:?}",
                donor
            );
        }
    }

    #[test]
    fn test_a_positive_donates_to_a_positive_and_ab_positive() {
        let a_pos = BloodType::APositive;
        assert!(a_pos.can_donate_to(&BloodType::APositive));
        assert!(a_pos.can_donate_to(&BloodType::ABPositive));
        // A+ 不能输给 B+, O+
        assert!(!a_pos.can_donate_to(&BloodType::BPositive));
        assert!(!a_pos.can_donate_to(&BloodType::OPositive));
    }

    #[test]
    fn test_o_positive_cannot_donate_to_o_negative() {
        let o_pos = BloodType::OPositive;
        assert!(!o_pos.can_donate_to(&BloodType::ONegative));
    }

    #[test]
    fn test_a_negative_cannot_receive_a_positive() {
        let a_neg = BloodType::ANegative;
        // A- 受血者接受 Rh+ 血会致敏
        assert!(!a_neg.can_receive_from(&BloodType::APositive));
        // A- 可以接受 A-
        assert!(a_neg.can_receive_from(&BloodType::ANegative));
    }

    #[test]
    fn test_rh_negative_cannot_receive_rh_positive() {
        let b_neg = BloodType::BNegative;
        assert!(!b_neg.can_receive_from(&BloodType::BPositive));
        let ab_neg = BloodType::ABNegative;
        assert!(!ab_neg.can_receive_from(&BloodType::ABPositive));
    }

    #[test]
    fn test_o_negative_has_no_antigens() {
        let o_neg = BloodType::ONegative;
        assert!(!o_neg.has_a_antigen());
        assert!(!o_neg.has_b_antigen());
        assert!(!o_neg.has_rh_antigen());
    }

    #[test]
    fn test_update_normalizes_hemoglobin() {
        let mut sys = BloodSystem::new();
        sys.hemoglobin = 9.0;
        sys.update(600.0);
        assert!(sys.hemoglobin > 9.0);
    }

    #[test]
    fn test_serialization_round_trip() {
        let sys = BloodSystem::new();
        let json = serde_json::to_string(&sys).unwrap();
        let restored: BloodSystem = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.hemoglobin, sys.hemoglobin);
        assert_eq!(restored.blood_type, sys.blood_type);
    }
}
