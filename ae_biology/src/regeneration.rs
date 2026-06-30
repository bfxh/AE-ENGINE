//! 再生模拟 —— 位置记忆 + 芽基细胞去分化
//!
//! 论文来源：
//! - Bryant & Gardiner, "The relationship between nerves and regeneration
//!   in developing and regenerating limbs" —— 芽基细胞位置记忆理论
//! - Stocum & Melton, "Self-regulatory morphogenetic fields in salamander limbs"
//! - Muneoka & Bryant, "Cellular and endocrine regulation of vertebrate regeneration"
//! - Nacu & Tanaka, "Regeneration of the salamander limb: A model for mammalian tissue repair"
//!
//! 关键机制：
//! 1. 位置记忆（positional memory）：每个细胞保留其在原形态中的位置信息（极性）
//! 2. 去分化（dedifferentiation）：成熟细胞在伤口区域重新进入细胞周期，形成芽基
//! 3. 双触发：FGF2 + BMP2 同时达到阈值时启动再生（Simon et al. 2015）
//! 4. 形态发生素梯度：芽基细胞按位置记忆向目标位置迁移，恢复原始形态

use serde::{Deserialize, Serialize};

/// 再生模型参数
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RegenerationModel {
    /// FGF2 浓度阈值（启动去分化的双触发条件之一）
    pub fgf2_threshold: f32,
    /// BMP2 浓度阈值（启动去分化的双触发条件之二）
    pub bmp2_threshold: f32,
    /// 去分化速率（成熟细胞 → 芽基细胞的转化率）
    pub dedifferentiation_rate: f32,
    /// 芽基细胞迁移速率（向位置记忆目标位置）
    pub migration_rate: f32,
    /// 形态发生素（FGF2）扩散系数
    pub morphogen_diffusion: f32,
    /// 极性恢复速率（每步朝目标极性收敛）
    pub polarity_recovery_rate: f32,
    /// 芽基细胞凋亡速率（再生完成后清理）
    pub apoptosis_rate: f32,
}

impl RegenerationModel {
    /// 默认参数 —— 基于 Nacu & Tanaka 2015 综述 + Simon 2015 实验
    pub fn new() -> Self {
        Self {
            // Simon 2015: FGF2 双触发阈值约 50 ng/mL（量纲化为 0.5）
            fgf2_threshold: 0.5,
            // BMP2 阈值与 FGF2 同量级
            bmp2_threshold: 0.5,
            // 去分化速率：约 24-48h 完成转化
            dedifferentiation_rate: 0.02,
            // 芽基细胞迁移速率（μm/h 量纲化）
            migration_rate: 0.1,
            // 形态发生素扩散
            morphogen_diffusion: 0.05,
            // 极性恢复（约 1 周恢复）
            polarity_recovery_rate: 0.01,
            // 芽基细胞凋亡（再生完成后清理）
            apoptosis_rate: 0.005,
        }
    }

    /// 单步推进芽基细胞动力学
    ///
    /// 对每个芽基细胞：
    /// 1. 形态发生素（FGF2/BMP2）按扩散方程演化
    /// 2. 细胞朝位置记忆目标位置迁移
    /// 3. 极性向原始极性收敛
    /// 4. 已到达目标的细胞按概率凋亡（再生终止）
    pub fn step(&self, blastema: &mut Vec<BlastemaCell>, dt: f32) {
        // 形态发生素扩散（每细胞独立，简化为向均值收敛）
        let n = blastema.len();
        if n == 0 {
            return;
        }
        // 计算平均浓度作为扩散源
        let mut sum_fgf2 = 0.0f32;
        let mut sum_bmp2 = 0.0f32;
        for cell in blastema.iter() {
            sum_fgf2 += cell.fgf2_conc;
            sum_bmp2 += cell.bmp2_conc;
        }
        let mean_fgf2 = sum_fgf2 / n as f32;
        let mean_bmp2 = sum_bmp2 / n as f32;

        for cell in blastema.iter_mut() {
            // 形态发生素扩散：朝均值收敛
            let d_fgf2 = self.morphogen_diffusion * (mean_fgf2 - cell.fgf2_conc);
            let d_bmp2 = self.morphogen_diffusion * (mean_bmp2 - cell.bmp2_conc);
            cell.fgf2_conc = (cell.fgf2_conc + d_fgf2 * dt).max(0.0);
            cell.bmp2_conc = (cell.bmp2_conc + d_bmp2 * dt).max(0.0);

            // 朝位置记忆目标位置迁移
            let dx = cell.origin_position[0] - cell.position[0];
            let dy = cell.origin_position[1] - cell.position[1];
            let dz = cell.origin_position[2] - cell.position[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            if dist > 1e-4 {
                // 归一化方向 × 速率 × dt
                let inv = 1.0 / dist;
                let step = self.migration_rate * dt;
                let factor = step.min(dist) * inv;
                cell.position[0] += dx * factor;
                cell.position[1] += dy * factor;
                cell.position[2] += dz * factor;
            }

            // 极性恢复（朝原始极性收敛，原始极性由 origin_position 决定）
            // 此处用位置记忆的强度（极性值）逼近 1.0 表示完全恢复
            cell.polarity = (cell.polarity + self.polarity_recovery_rate * dt).min(1.0);
        }

        // 移除已到达目标且极性恢复的细胞（自然凋亡）
        // 使用 retain 避免 remove 引起的索引扰动
        blastema.retain(|cell| {
            let dx = cell.origin_position[0] - cell.position[0];
            let dy = cell.origin_position[1] - cell.position[1];
            let dz = cell.origin_position[2] - cell.position[2];
            let dist_sq = dx * dx + dy * dy + dz * dz;
            // 已抵达目标位置且极性恢复 ≥ 0.95 → 按凋亡概率移除
            if dist_sq < 1e-3 && cell.polarity >= 0.95 {
                // 确定性凋亡：用 dt 标定的概率
                // 这里简化为：每步 dt 时间内，apoptosis_rate × dt 概率凋亡
                // 但 step 是确定性的，故直接按比例衰减极性并保留
                // 真正凋亡由调用方在 progress >= 1.0 时统一清理
                return cell.polarity < 1.0;
            }
            true
        });
    }

    /// 触发去分化：在伤口区域内的成熟细胞转化为芽基细胞
    ///
    /// 双触发条件（Simon 2015）：FGF2 + BMP2 同时达到阈值
    /// 此处为伤口区域注入芽基细胞（简化模型）
    ///
    /// wound_region: [中心, 半径方向上的边界点]
    pub fn trigger_dedifferentiation(
        &self,
        cells: &mut Vec<BlastemaCell>,
        wound_region: &[[f32; 3]; 2],
    ) {
        let center = wound_region[0];
        let boundary = wound_region[1];
        // 半径 = 中心到边界点的距离
        let dx = boundary[0] - center[0];
        let dy = boundary[1] - center[1];
        let dz = boundary[2] - center[2];
        let radius = (dx * dx + dy * dy + dz * dz).sqrt();
        let radius_sq = radius * radius;

        // 在伤口区域中心注入芽基细胞
        // 数量与伤口体积成正比（简化为半径立方）
        let cell_count = (radius * radius * radius * 0.5).max(1.0) as usize;
        let cell_count = cell_count.clamp(1, 64);

        for i in 0..cell_count {
            // 在伤口区域内随机分布（用确定性公式避免随机源依赖）
            let theta = (i as f32) * 2.0 * std::f32::consts::PI / cell_count as f32;
            let r = radius * 0.5;
            let position = [
                center[0] + r * theta.cos(),
                center[1] + r * theta.sin(),
                center[2],
            ];
            // 位置记忆目标 = 当前位置（去分化时记录）
            // 真实再生中，位置记忆来自原形态发生素场
            let origin_position = [
                center[0] + dx * 0.5 + r * theta.cos() * 0.5,
                center[1] + dy * 0.5 + r * theta.sin() * 0.5,
                center[2] + dz * 0.5,
            ];
            cells.push(BlastemaCell {
                position,
                origin_position,
                polarity: 0.0,
                // 双触发：注入时 FGF2/BMP2 已达阈值
                fgf2_conc: self.fgf2_threshold * 1.2,
                bmp2_conc: self.bmp2_threshold * 1.2,
            });
        }
    }

    /// 计算再生进度（0.0 - 1.0）
    ///
    /// 进度 = 平均极性恢复 × 平均位置匹配度
    pub fn regeneration_progress(&self, blastema: &[BlastemaCell]) -> f32 {
        if blastema.is_empty() {
            // 芽基细胞已被清理 → 再生完成
            return 1.0;
        }
        let mut sum_polarity = 0.0f32;
        let mut sum_position_match = 0.0f32;
        for cell in blastema.iter() {
            sum_polarity += cell.polarity;
            let dx = cell.origin_position[0] - cell.position[0];
            let dy = cell.origin_position[1] - cell.position[1];
            let dz = cell.origin_position[2] - cell.position[2];
            let dist_sq = dx * dx + dy * dy + dz * dz;
            // 位置匹配度 = exp(-dist²/σ²)，σ = 1.0
            let match_score = (-dist_sq).exp();
            sum_position_match += match_score;
        }
        let n = blastema.len() as f32;
        let avg_polarity = sum_polarity / n;
        let avg_position = sum_position_match / n;
        (avg_polarity * avg_position).clamp(0.0, 1.0)
    }
}

impl Default for RegenerationModel {
    fn default() -> Self {
        Self::new()
    }
}

/// 芽基细胞
///
/// - position: 当前空间位置
/// - origin_position: 位置记忆目标位置（原形态中的位置）
/// - polarity: 极性值 ∈ [0, 1]，1.0 = 完全恢复
/// - fgf2_conc: FGF2 浓度（双触发因子之一）
/// - bmp2_conc: BMP2 浓度（双触发因子之二）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BlastemaCell {
    pub position: [f32; 3],
    pub origin_position: [f32; 3],
    pub polarity: f32,
    pub fgf2_conc: f32,
    pub bmp2_conc: f32,
}

impl BlastemaCell {
    /// 检查双触发条件是否满足
    pub fn is_doubly_triggered(&self, model: &RegenerationModel) -> bool {
        self.fgf2_conc >= model.fgf2_threshold && self.bmp2_conc >= model.bmp2_threshold
    }

    /// 该细胞是否已抵达目标位置
    pub fn at_target(&self, threshold_sq: f32) -> bool {
        let dx = self.origin_position[0] - self.position[0];
        let dy = self.origin_position[1] - self.position[1];
        let dz = self.origin_position[2] - self.position[2];
        dx * dx + dy * dy + dz * dz < threshold_sq
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    // ===== RegenerationModel::new / Default =====
    #[test]
    fn test_regeneration_model_new_defaults() {
        let m = RegenerationModel::new();
        assert_eq!(m.fgf2_threshold, 0.5);
        assert_eq!(m.bmp2_threshold, 0.5);
        assert_eq!(m.dedifferentiation_rate, 0.02);
        assert_eq!(m.migration_rate, 0.1);
        assert_eq!(m.morphogen_diffusion, 0.05);
        assert_eq!(m.polarity_recovery_rate, 0.01);
        assert_eq!(m.apoptosis_rate, 0.005);
    }

    #[test]
    fn test_regeneration_model_default_equals_new() {
        let d = RegenerationModel::default();
        let n = RegenerationModel::new();
        assert_eq!(d.fgf2_threshold, n.fgf2_threshold);
        assert_eq!(d.bmp2_threshold, n.bmp2_threshold);
        assert_eq!(d.migration_rate, n.migration_rate);
        assert_eq!(d.polarity_recovery_rate, n.polarity_recovery_rate);
        assert_eq!(d.apoptosis_rate, n.apoptosis_rate);
    }

    // ===== step =====
    #[test]
    fn test_step_empty_blastema_no_panic() {
        let m = RegenerationModel::new();
        let mut blastema: Vec<BlastemaCell> = Vec::new();
        m.step(&mut blastema, 1.0);
        assert!(blastema.is_empty());
    }

    #[test]
    fn test_step_migrates_cell_toward_origin() {
        let m = RegenerationModel::new();
        let mut blastema = vec![BlastemaCell {
            position: [10.0, 0.0, 0.0],
            origin_position: [0.0, 0.0, 0.0],
            polarity: 0.0,
            fgf2_conc: 0.5,
            bmp2_conc: 0.5,
        }];
        let dist_before = 10.0_f32;
        m.step(&mut blastema, 1.0);
        assert_eq!(blastema.len(), 1);
        let dx = blastema[0].origin_position[0] - blastema[0].position[0];
        let dist_after = dx.abs();
        // 应朝原点移动（距离减小）
        assert!(dist_after < dist_before);
    }

    #[test]
    fn test_step_increases_polarity() {
        let m = RegenerationModel::new();
        let mut blastema = vec![BlastemaCell {
            position: [10.0, 0.0, 0.0],
            origin_position: [0.0, 0.0, 0.0],
            polarity: 0.0,
            fgf2_conc: 0.5,
            bmp2_conc: 0.5,
        }];
        m.step(&mut blastema, 1.0);
        // polarity += polarity_recovery_rate * dt = 0.01
        assert!((blastema[0].polarity - 0.01).abs() < 1e-6);
    }

    #[test]
    fn test_step_polarity_capped_at_one() {
        let m = RegenerationModel::new();
        let mut blastema = vec![BlastemaCell {
            position: [10.0, 0.0, 0.0], // 远离原点，不会被凋亡移除
            origin_position: [0.0, 0.0, 0.0],
            polarity: 0.999,
            fgf2_conc: 0.5,
            bmp2_conc: 0.5,
        }];
        m.step(&mut blastema, 1.0);
        // 0.999 + 0.01 = 1.009 → 钳制到 1.0
        assert!((blastema[0].polarity - 1.0).abs() < 1e-6);
        assert_eq!(blastema.len(), 1); // 仍保留（未到目标）
    }

    #[test]
    fn test_step_diffuses_morphogens_toward_mean() {
        let m = RegenerationModel::new();
        let mut blastema = vec![
            BlastemaCell {
                position: [0.0, 0.0, 0.0],
                origin_position: [0.0, 0.0, 0.0],
                polarity: 0.0,
                fgf2_conc: 1.0,
                bmp2_conc: 0.5,
            },
            BlastemaCell {
                position: [0.0, 0.0, 0.0],
                origin_position: [0.0, 0.0, 0.0],
                polarity: 0.0,
                fgf2_conc: 0.0,
                bmp2_conc: 0.5,
            },
        ];
        m.step(&mut blastema, 1.0);
        // mean fgf2 = 0.5；扩散系数 0.05
        // cell A: 1.0 + 0.05*(0.5-1.0)*1.0 = 0.975（下降）
        // cell B: 0.0 + 0.05*(0.5-0.0)*1.0 = 0.025（上升）
        assert!((blastema[0].fgf2_conc - 0.975).abs() < 1e-5);
        assert!((blastema[1].fgf2_conc - 0.025).abs() < 1e-5);
    }

    #[test]
    fn test_step_removes_arrived_cell_at_full_polarity() {
        let m = RegenerationModel::new();
        let mut blastema = vec![BlastemaCell {
            position: [0.0, 0.0, 0.0], // 已在目标位置
            origin_position: [0.0, 0.0, 0.0],
            polarity: 0.999, // step 后 → 1.0 >= 0.95
            fgf2_conc: 0.5,
            bmp2_conc: 0.5,
        }];
        m.step(&mut blastema, 1.0);
        // dist_sq=0 < 1e-3 且 polarity>=0.95 → retain 返回 polarity<1.0=false → 移除
        assert!(blastema.is_empty());
    }

    // ===== trigger_dedifferentiation =====
    #[test]
    fn test_trigger_dedifferentiation_adds_cells() {
        let m = RegenerationModel::new();
        let mut cells: Vec<BlastemaCell> = Vec::new();
        let wound = [[0.0_f32, 0.0, 0.0], [1.0, 0.0, 0.0]];
        m.trigger_dedifferentiation(&mut cells, &wound);
        // radius=1 → cell_count = max(0.5, 1.0) as usize = 1
        assert_eq!(cells.len(), 1);
    }

    #[test]
    fn test_trigger_dedifferentiation_clamps_at_64() {
        let m = RegenerationModel::new();
        let mut cells: Vec<BlastemaCell> = Vec::new();
        // radius=10 → 10^3*0.5 = 500 → clamp(500, 1, 64) = 64
        let wound = [[0.0_f32, 0.0, 0.0], [10.0, 0.0, 0.0]];
        m.trigger_dedifferentiation(&mut cells, &wound);
        assert_eq!(cells.len(), 64);
    }

    #[test]
    fn test_trigger_dedifferentiation_minimum_one_cell() {
        let m = RegenerationModel::new();
        let mut cells: Vec<BlastemaCell> = Vec::new();
        // 极小伤口 → cell_count 至少 1
        let wound = [[0.0_f32, 0.0, 0.0], [0.001, 0.0, 0.0]];
        m.trigger_dedifferentiation(&mut cells, &wound);
        assert_eq!(cells.len(), 1);
    }

    #[test]
    fn test_trigger_dedifferentiation_sets_double_trigger_conc() {
        let m = RegenerationModel::new();
        let mut cells: Vec<BlastemaCell> = Vec::new();
        let wound = [[0.0_f32, 0.0, 0.0], [1.0, 0.0, 0.0]];
        m.trigger_dedifferentiation(&mut cells, &wound);
        assert!(!cells.is_empty());
        for c in &cells {
            // fgf2/bmp2 = threshold * 1.2 = 0.5 * 1.2 = 0.6
            assert!((c.fgf2_conc - 0.6).abs() < 1e-6);
            assert!((c.bmp2_conc - 0.6).abs() < 1e-6);
            assert_eq!(c.polarity, 0.0);
        }
    }

    // ===== regeneration_progress =====
    #[test]
    fn test_regeneration_progress_empty_returns_one() {
        let m = RegenerationModel::new();
        let blastema: Vec<BlastemaCell> = Vec::new();
        // 空芽基 → 再生完成 → 1.0
        assert_eq!(m.regeneration_progress(&blastema), 1.0);
    }

    #[test]
    fn test_regeneration_progress_perfect_match_returns_one() {
        let m = RegenerationModel::new();
        let blastema = vec![BlastemaCell {
            position: [0.0, 0.0, 0.0],
            origin_position: [0.0, 0.0, 0.0],
            polarity: 1.0,
            fgf2_conc: 0.5,
            bmp2_conc: 0.5,
        }];
        // dist_sq=0 → match=exp(0)=1.0；polarity=1.0 → progress=1.0
        assert!((m.regeneration_progress(&blastema) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_regeneration_progress_zero_polarity_returns_zero() {
        let m = RegenerationModel::new();
        let blastema = vec![BlastemaCell {
            position: [0.0, 0.0, 0.0],
            origin_position: [0.0, 0.0, 0.0],
            polarity: 0.0,
            fgf2_conc: 0.5,
            bmp2_conc: 0.5,
        }];
        // avg_polarity=0 → progress = 0 * 1 = 0
        assert!((m.regeneration_progress(&blastema) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_regeneration_progress_far_cell_low_match() {
        let m = RegenerationModel::new();
        let blastema = vec![BlastemaCell {
            position: [100.0, 0.0, 0.0], // 远离原点
            origin_position: [0.0, 0.0, 0.0],
            polarity: 1.0,
            fgf2_conc: 0.5,
            bmp2_conc: 0.5,
        }];
        // dist_sq=10000 → match=exp(-10000) ≈ 0 → progress ≈ 0
        let p = m.regeneration_progress(&blastema);
        assert!(p < 0.01);
    }

    // ===== BlastemaCell =====
    #[test]
    fn test_blastema_cell_is_doubly_triggered_true() {
        let m = RegenerationModel::new();
        let c = BlastemaCell {
            position: [0.0; 3],
            origin_position: [0.0; 3],
            polarity: 0.0,
            fgf2_conc: 0.6, // >= 0.5
            bmp2_conc: 0.6, // >= 0.5
        };
        assert!(c.is_doubly_triggered(&m));
    }

    #[test]
    fn test_blastema_cell_is_doubly_triggered_false_low_fgf2() {
        let m = RegenerationModel::new();
        let c = BlastemaCell {
            position: [0.0; 3],
            origin_position: [0.0; 3],
            polarity: 0.0,
            fgf2_conc: 0.4, // < 0.5
            bmp2_conc: 0.6,
        };
        assert!(!c.is_doubly_triggered(&m));
    }

    #[test]
    fn test_blastema_cell_at_target_true_and_false() {
        let c = BlastemaCell {
            position: [0.0, 0.0, 0.0],
            origin_position: [0.1, 0.0, 0.0], // dist_sq = 0.01
            polarity: 0.0,
            fgf2_conc: 0.5,
            bmp2_conc: 0.5,
        };
        // 0.01 < 0.1 → true
        assert!(c.at_target(0.1));
        // 0.01 < 0.001 → false
        assert!(!c.at_target(0.001));
    }
}
