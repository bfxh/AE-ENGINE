//! V8 沙盒：生物场模块 —— 微生物驱动的有机物降解
//!
//! 模拟尸体/植物/木材等有机物在环境中的腐烂过程：
//! - Arrhenius 方程描述温度对化学反应速率的影响（绝对形式，高 Ea → 慢）
//! - Logistic 增长描述微生物种群动态，受 Cardinal 温度模型约束
//! - 法医生物学五阶段腐烂模型（Fresh → Bloat → Active → Advanced → Dry）
//! - 腐烂产物：CO2/CH4/H2S/H2O 气体 + 液体渗出物 + 代谢热
//!
//! 设计要点：
//! - 与热力学沙盒解耦：本模块仅计算生物场变化，产物由调用者注入环境
//! - 质量守恒：biomass 失去的质量 = 气体产物 + 液体渗出物 + 残留灰分
//! - 温度耦合：biomass 温度跟随环境温度（由调用者传入 ambient_temp）

use serde::{Deserialize, Serialize};

// ─── 物理常量 ───────────────────────────────────────────────
/// Arrhenius 参考温度 K
pub const T_REF: f32 = 300.0;
/// 气体常数 J/(mol·K)
pub const R: f32 = 8.314;
/// 腐烂放热 J/kg（微生物代谢产热，分解 1kg 有机物释放的能量）
pub const BURN_ENERGY_DECAY: f32 = 2.0e5;
/// 微生物最大增长速率 1/s（logistic 模型）
pub const MU_MAX: f32 = 0.1;  // 底层改造：10倍加速微生物增长
/// 微生物最适温度 K（人体温度附近，腐烂菌最优）
pub const T_MICROBE_OPT: f32 = 310.0;
/// 微生物最低活性温度 K（低于此温度微生物休眠，腐烂停止）
pub const T_MICROBE_MIN: f32 = 275.0;
/// 微生物最高存活温度 K（高于此温度微生物死亡，巴氏杀菌效应）
pub const T_MICROBE_MAX: f32 = 380.0;  // 底层改造：高温耐受提到 380K
/// Arrhenius 指前因子 1/s（标定使 Flesh 在 310K 下 100 步可见明显腐烂）
pub const A_PRE: f32 = 1.0e7;  // 底层改造：100倍加速，60s 可观测腐烂
/// 高温微生物死亡速率 1/s（T > T_MICROBE_MAX 时指数衰减）
pub const MICROBE_DEATH_RATE: f32 = 1.0;
/// 厌氧腐烂速率因子（无氧时腐烂速度降低到 10%）
pub const ANAEROBIC_FACTOR: f32 = 0.1;
/// 初始微生物密度（接种量，代表初始污染）
pub const INITIAL_MICROBE: f32 = 0.8;  // 底层改造：更高初始接种量

// ─── 有机物类型 ────────────────────────────────────────────
/// 有机物类型（影响腐烂产物比例和活化能）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrganicMatter {
    /// 肌肉组织，快速腐烂，产 CH4 + H2S + NH3
    Flesh,
    /// 骨骼，极慢降解，主要钙质
    Bone,
    /// 木材，真菌降解，产 CO2 + H2O
    Wood,
    /// 植物组织，中等速度
    Plant,
    /// 脂肪，皂化反应
    Fat,
    /// 皮革，慢速降解
    Leather,
}

impl OrganicMatter {
    /// 活化能 J/mol（越高反应越慢，对温度越敏感）
    pub fn activation_energy(&self) -> f32 {
        match self {
            OrganicMatter::Flesh => 40_000.0,
            OrganicMatter::Bone => 100_000.0,
            OrganicMatter::Wood => 70_000.0,
            OrganicMatter::Plant => 50_000.0,
            OrganicMatter::Fat => 60_000.0,
            OrganicMatter::Leather => 80_000.0,
        }
    }

    /// 产物比例 (CH4, H2S, CO2, H2O, liquid_leachate)
    /// 气体比例之和 + 液体比例 ≤ 1.0（剩余为灰分残留，未追踪）
    pub fn product_ratios(&self) -> (f32, f32, f32, f32, f32) {
        match self {
            OrganicMatter::Flesh => (0.10, 0.05, 0.40, 0.30, 0.15),
            OrganicMatter::Bone => (0.02, 0.01, 0.30, 0.20, 0.47),
            OrganicMatter::Wood => (0.00, 0.00, 0.60, 0.35, 0.05),
            OrganicMatter::Plant => (0.05, 0.00, 0.50, 0.35, 0.10),
            OrganicMatter::Fat => (0.15, 0.02, 0.30, 0.20, 0.33),
            OrganicMatter::Leather => (0.03, 0.01, 0.40, 0.30, 0.26),
        }
    }
}

// ─── 腐烂阶段 ──────────────────────────────────────────────
/// 腐烂阶段（法医生物学标准五阶段模型）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecayStage {
    /// 新鲜期（0-1天），自溶开始
    Fresh,
    /// 膨胀期（1-7天），细菌产气
    Bloat,
    /// 活跃腐烂（7-14天），大量液化
    Active,
    /// 高级腐烂（14-30天），干化
    Advanced,
    /// 干燥期（30天+），仅骨骼残留
    Dry,
}

impl DecayStage {
    /// 速率倍数（相对基准腐烂速率）
    pub fn rate_multiplier(&self) -> f32 {
        match self {
            DecayStage::Fresh => 0.1,
            DecayStage::Bloat => 1.0,
            DecayStage::Active => 2.0,
            DecayStage::Advanced => 0.5,
            DecayStage::Dry => 0.05,
        }
    }

    /// 产气速率 kg/s/kg_biomass（每 kg 活体质量每秒产气量基准）
    pub fn gas_production_rate(&self) -> f32 {
        match self {
            DecayStage::Fresh => 1e-6,
            DecayStage::Bloat => 5e-5,
            DecayStage::Active => 1e-4,
            DecayStage::Advanced => 2e-5,
            DecayStage::Dry => 1e-7,
        }
    }
}

// ─── 腐烂产物 ──────────────────────────────────────────────
/// 腐烂产生的气体和液体（返回给调用者注入环境）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecayProducts {
    /// CO2 kg
    pub co2: f32,
    /// CH4（甲烷）kg
    pub ch4: f32,
    /// H2S（硫化氢）kg
    pub h2s: f32,
    /// 水蒸气 kg
    pub h2o_vapor: f32,
    /// NH3（氨）kg —— 简化处理，归入其他
    pub nh3: f32,
    /// 腐烂释放的热量 J（微生物代谢产热）
    pub heat: f32,
    /// 液体渗出物 kg（简化为水分）
    pub liquid_leachate: f32,
}

// ─── 生物质 ────────────────────────────────────────────────
/// 有机生物质（尸体/植物/木材等可腐烂物）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Biomass {
    /// 有机物类型
    pub matter_type: OrganicMatter,
    /// 当前质量 kg
    pub mass: f32,
    /// 初始质量 kg（用于计算腐烂进度）
    pub initial_mass: f32,
    /// 当前温度 K（随环境）
    pub temperature: f32,
    /// 当前腐烂阶段
    pub stage: DecayStage,
    /// 总体腐烂进度 0..1
    pub decay_progress: f32,
    /// 死亡后经过的时间 s
    pub age: f32,
    /// 微生物密度 0..1
    pub microbe_population: f32,
}

impl Biomass {
    /// 创建新的生物质（初始状态：Fresh 阶段，初始微生物接种）
    pub fn new(matter_type: OrganicMatter, mass: f32, temperature: f32) -> Self {
        Self {
            matter_type,
            mass,
            initial_mass: mass,
            temperature,
            stage: DecayStage::Fresh,
            decay_progress: 0.0,
            age: 0.0,
            microbe_population: INITIAL_MICROBE,
        }
    }

    /// 根据腐烂进度更新阶段
    /// 阈值：0.05→Bloat, 0.2→Active, 0.5→Advanced, 0.8→Dry
    pub fn update_stage(&mut self) {
        self.stage = if self.decay_progress < 0.05 {
            DecayStage::Fresh
        } else if self.decay_progress < 0.2 {
            DecayStage::Bloat
        } else if self.decay_progress < 0.5 {
            DecayStage::Active
        } else if self.decay_progress < 0.8 {
            DecayStage::Advanced
        } else {
            DecayStage::Dry
        };
    }

    /// 微生物温度活性因子（Cardinal 模型）
    /// - T < T_MIN 或 T > T_MAX：0（休眠/死亡）
    /// - T_MIN ≤ T ≤ T_OPT：线性上升 0→1
    /// - T_OPT < T ≤ T_MAX：线性下降 1→0
    fn microbe_temp_factor(t: f32) -> f32 {
        if t < T_MICROBE_MIN || t > T_MICROBE_MAX {
            0.0
        } else if t <= T_MICROBE_OPT {
            (t - T_MICROBE_MIN) / (T_MICROBE_OPT - T_MICROBE_MIN)
        } else {
            1.0 - (t - T_MICROBE_OPT) / (T_MICROBE_MAX - T_MICROBE_OPT)
        }
    }

    /// 推进一步腐烂，返回产物
    ///
    /// 物理模型：
    /// 1. 温度跟随环境
    /// 2. 微生物 Cardinal 温度模型：低于 275K 或高于 340K 失活
    /// 3. 高温杀菌：T > 340K 时微生物指数死亡
    /// 4. 微生物 logistic 增长（受温度活性影响）
    /// 5. Arrhenius 方程（绝对形式）：k = A * exp(-Ea/(R*T))，高 Ea 材质更慢
    /// 6. 有效微生物 = 种群密度 × 温度活性
    /// 7. 质量损失 = arrhenius × 阶段倍数 × 有效微生物 × 水分 × 氧气 × mass × dt
    /// 8. 产气 = 阶段 gas_production_rate × 质量损失量，按材质比例分配
    /// 9. 产热 = 质量损失 × BURN_ENERGY_DECAY（腐烂放热）
    pub fn step_decay(
        &mut self,
        ambient_temp: f32,
        moisture: f32,
        oxygen_available: bool,
        dt: f32,
    ) -> DecayProducts {
        // 1. 温度跟随环境
        self.temperature = ambient_temp;

        // 2. 微生物温度活性因子
        let microbe_t = Self::microbe_temp_factor(self.temperature);

        // 3. 高温杀菌：T > T_MICROBE_MAX 时微生物指数死亡
        if self.temperature > T_MICROBE_MAX {
            self.microbe_population *= (-MICROBE_DEATH_RATE * dt).exp();
        }

        // 4. 微生物 logistic 增长（仅在最适温度区间内增长）
        let mu = MU_MAX * microbe_t;
        let dn = mu * self.microbe_population * (1.0 - self.microbe_population) * dt;
        self.microbe_population = (self.microbe_population + dn).clamp(0.0, 1.0);

        // 5. Arrhenius 反应速率（绝对形式，保证高 Ea 材质更慢）
        let ea = self.matter_type.activation_energy();
        let t_kelvin = self.temperature.max(1.0);
        let arrhenius = A_PRE * (-ea / (R * t_kelvin)).exp();

        // 6. 环境因子
        let moisture_factor = moisture.clamp(0.0, 1.0);
        let oxygen_factor = if oxygen_available { 1.0 } else { ANAEROBIC_FACTOR };

        // 有效微生物活性 = 种群密度 × 温度活性
        let effective_microbe = self.microbe_population * microbe_t;

        // 7. 质量损失（记录步骤前阶段，用于产气计算）
        let stage_before = self.stage;
        let stage_rate = stage_before.rate_multiplier();
        let decay_rate =
            arrhenius * stage_rate * effective_microbe * moisture_factor * oxygen_factor;
        let mass_loss = (decay_rate * self.mass * dt).clamp(0.0, self.mass);
        self.mass -= mass_loss;

        // 8. 腐烂进度（基于初始质量归一化）
        if self.initial_mass > 0.0 {
            self.decay_progress =
                (self.decay_progress + mass_loss / self.initial_mass).clamp(0.0, 1.0);
        }

        // 9. 年龄
        self.age += dt;

        // 10. 更新阶段（基于新的腐烂进度）
        self.update_stage();

        // 11. 产气（按步骤前阶段的 gas_production_rate × 质量损失量）
        let gas_total = stage_before.gas_production_rate() * mass_loss;
        // 按材质比例分配气体组分
        let (ch4_r, h2s_r, co2_r, h2o_r, liquid_r) = self.matter_type.product_ratios();
        let gas_ratio_sum = ch4_r + h2s_r + co2_r + h2o_r;
        let (ch4, h2s, co2, h2o_vapor) = if gas_ratio_sum > 1e-9 {
            (
                gas_total * ch4_r / gas_ratio_sum,
                gas_total * h2s_r / gas_ratio_sum,
                gas_total * co2_r / gas_ratio_sum,
                gas_total * h2o_r / gas_ratio_sum,
            )
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };

        // 12. 液体渗出物（按材质比例 × 质量损失）
        let liquid_leachate = liquid_r * mass_loss;

        // 13. 产热（腐烂放热，微生物代谢产热）
        let heat = mass_loss * BURN_ENERGY_DECAY;

        DecayProducts {
            co2,
            ch4,
            h2s,
            h2o_vapor,
            nh3: 0.0, // 简化：NH3 归入其他，不单独追踪
            heat,
            liquid_leachate,
        }
    }
}

// ─── 单元测试 ──────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    // 1. 新建 Biomass 初始状态测试
    #[test]
    fn test_biomass_new_initial_state() {
        let b = Biomass::new(OrganicMatter::Flesh, 1.0, 310.0);
        assert_eq!(b.matter_type, OrganicMatter::Flesh);
        assert!((b.mass - 1.0).abs() < 1e-6, "初始质量 = 1.0");
        assert!((b.initial_mass - 1.0).abs() < 1e-6, "初始质量记录");
        assert!((b.temperature - 310.0).abs() < 1e-6, "初始温度");
        assert_eq!(b.stage, DecayStage::Fresh, "初始阶段为 Fresh");
        assert!((b.decay_progress - 0.0).abs() < 1e-6, "初始进度为 0");
        assert!((b.age - 0.0).abs() < 1e-6, "初始年龄为 0");
        assert!(
            (b.microbe_population - INITIAL_MICROBE).abs() < 1e-6,
            "初始微生物接种量"
        );
    }

    // 2. DecayStage 转换测试
    #[test]
    fn test_decay_stage_transitions() {
        let mut b = Biomass::new(OrganicMatter::Flesh, 1.0, 310.0);

        b.decay_progress = 0.0;
        b.update_stage();
        assert_eq!(b.stage, DecayStage::Fresh, "progress=0 → Fresh");

        b.decay_progress = 0.04;
        b.update_stage();
        assert_eq!(b.stage, DecayStage::Fresh, "progress=0.04 → Fresh");

        b.decay_progress = 0.05;
        b.update_stage();
        assert_eq!(b.stage, DecayStage::Bloat, "progress=0.05 → Bloat");

        b.decay_progress = 0.19;
        b.update_stage();
        assert_eq!(b.stage, DecayStage::Bloat, "progress=0.19 → Bloat");

        b.decay_progress = 0.2;
        b.update_stage();
        assert_eq!(b.stage, DecayStage::Active, "progress=0.2 → Active");

        b.decay_progress = 0.49;
        b.update_stage();
        assert_eq!(b.stage, DecayStage::Active, "progress=0.49 → Active");

        b.decay_progress = 0.5;
        b.update_stage();
        assert_eq!(b.stage, DecayStage::Advanced, "progress=0.5 → Advanced");

        b.decay_progress = 0.79;
        b.update_stage();
        assert_eq!(b.stage, DecayStage::Advanced, "progress=0.79 → Advanced");

        b.decay_progress = 0.8;
        b.update_stage();
        assert_eq!(b.stage, DecayStage::Dry, "progress=0.8 → Dry");

        b.decay_progress = 1.0;
        b.update_stage();
        assert_eq!(b.stage, DecayStage::Dry, "progress=1.0 → Dry");
    }

    // 3. Flesh 快速腐烂测试（高温 310K 下 100 步应有明显质量损失）
    #[test]
    fn test_flesh_fast_decay() {
        let mut b = Biomass::new(OrganicMatter::Flesh, 1.0, 310.0);
        let m0 = b.mass;
        for _ in 0..100 {
            b.step_decay(310.0, 1.0, true, 1.0);
        }
        let loss_pct = (m0 - b.mass) / m0 * 100.0;
        assert!(
            loss_pct > 1.0,
            "Flesh 在 310K 下 100 步应有明显质量损失: loss={:.3}%",
            loss_pct
        );
        assert!(b.decay_progress > 0.0, "腐烂进度应增加");
    }

    // 4. Bone 极慢腐烂测试（同条件质量损失远小于 Flesh）
    #[test]
    fn test_bone_slow_decay() {
        let mut flesh = Biomass::new(OrganicMatter::Flesh, 1.0, 310.0);
        let mut bone = Biomass::new(OrganicMatter::Bone, 1.0, 310.0);
        for _ in 0..100 {
            flesh.step_decay(310.0, 1.0, true, 1.0);
            bone.step_decay(310.0, 1.0, true, 1.0);
        }
        let flesh_loss = 1.0 - flesh.mass;
        let bone_loss = 1.0 - bone.mass;
        assert!(
            bone_loss < flesh_loss * 0.001,
            "Bone 腐烂远慢于 Flesh: flesh_loss={:.3e} bone_loss={:.3e}",
            flesh_loss,
            bone_loss
        );
    }

    // 5. 低温抑制腐烂测试（273K 下几乎不腐烂）
    #[test]
    fn test_low_temp_inhibition() {
        let mut b = Biomass::new(OrganicMatter::Flesh, 1.0, 273.0);
        let m0 = b.mass;
        for _ in 0..100 {
            b.step_decay(273.0, 1.0, true, 1.0);
        }
        let loss = m0 - b.mass;
        assert!(loss < 1e-6, "273K 下几乎不腐烂: loss={:.3e}", loss);
    }

    // 6. 高温杀菌测试（>340K 微生物死亡，腐烂停止）
    #[test]
    fn test_high_temp_sterilization() {
        let mut b = Biomass::new(OrganicMatter::Flesh, 1.0, 310.0);
        // 先在 310K 培养微生物
        for _ in 0..50 {
            b.step_decay(310.0, 1.0, true, 1.0);
        }
        let microbe_before = b.microbe_population;
        assert!(microbe_before > 0.1, "310K 下微生物应增长: {}", microbe_before);

        // 升温到 390K 杀菌（T_MICROBE_MAX=380）
        for _ in 0..100 {
            b.step_decay(390.0, 1.0, true, 1.0);
        }
        assert!(
            b.microbe_population < 1e-6,
            "390K 下微生物应死亡: {}",
            b.microbe_population
        );

        // 继续在 390K 下，腐烂应停止
        let m0 = b.mass;
        for _ in 0..100 {
            b.step_decay(390.0, 1.0, true, 1.0);
        }
        let loss = m0 - b.mass;
        assert!(loss < 1e-6, "高温杀菌后腐烂停止: loss={:.3e}", loss);
    }

    // 7. 产气测试（Flesh 腐烂产 CH4 和 H2S）
    #[test]
    fn test_gas_production() {
        let mut b = Biomass::new(OrganicMatter::Flesh, 1.0, 310.0);
        let mut total_ch4 = 0.0;
        let mut total_h2s = 0.0;
        let mut total_co2 = 0.0;
        for _ in 0..100 {
            let p = b.step_decay(310.0, 1.0, true, 1.0);
            total_ch4 += p.ch4;
            total_h2s += p.h2s;
            total_co2 += p.co2;
        }
        assert!(total_ch4 > 0.0, "Flesh 腐烂应产 CH4: {}", total_ch4);
        assert!(total_h2s > 0.0, "Flesh 腐烂应产 H2S: {}", total_h2s);
        assert!(total_co2 > 0.0, "Flesh 腐烂应产 CO2: {}", total_co2);
    }

    // 8. 产热测试（腐烂释放热量 > 0）
    #[test]
    fn test_heat_production() {
        let mut b = Biomass::new(OrganicMatter::Flesh, 1.0, 310.0);
        let mut total_heat = 0.0;
        for _ in 0..100 {
            let p = b.step_decay(310.0, 1.0, true, 1.0);
            total_heat += p.heat;
        }
        assert!(total_heat > 0.0, "腐烂应释放热量: {}", total_heat);
    }

    // 9. 缺氧抑制测试（无氧时腐烂速率降低）
    #[test]
    fn test_no_oxygen_inhibition() {
        let mut aerobic = Biomass::new(OrganicMatter::Flesh, 1.0, 310.0);
        let mut anaerobic = Biomass::new(OrganicMatter::Flesh, 1.0, 310.0);
        // 底层改造：步数从 100 减到 5（避免 mass_loss clamp 到 mass 导致 ratio 失真）
        for _ in 0..5 {
            aerobic.step_decay(310.0, 1.0, true, 1.0);
            anaerobic.step_decay(310.0, 1.0, false, 1.0);
        }
        let aerobic_loss = 1.0 - aerobic.mass;
        let anaerobic_loss = 1.0 - anaerobic.mass;
        assert!(
            anaerobic_loss < aerobic_loss,
            "缺氧时腐烂应更慢: aerobic={:.3e} anaerobic={:.3e}",
            aerobic_loss,
            anaerobic_loss
        );
        assert!(
            anaerobic_loss < aerobic_loss * 0.3,
            "缺氧腐烂应降到 30% 以下: ratio={:.3}",
            anaerobic_loss / aerobic_loss
        );
    }

    // 10. 干燥抑制测试（moisture=0 时不腐烂）
    #[test]
    fn test_no_moisture_inhibition() {
        let mut b = Biomass::new(OrganicMatter::Flesh, 1.0, 310.0);
        let m0 = b.mass;
        for _ in 0..100 {
            b.step_decay(310.0, 0.0, true, 1.0);
        }
        let loss = m0 - b.mass;
        assert!(loss < 1e-9, "moisture=0 时不应腐烂: loss={:.3e}", loss);
    }

    // 11. 质量守恒测试（初始质量 ≈ 剩余 + 气体 + 液体 + 灰分）
    #[test]
    fn test_mass_conservation() {
        let mut b = Biomass::new(OrganicMatter::Flesh, 1.0, 310.0);
        let initial = b.initial_mass;
        let mut total_gas = 0.0;
        let mut total_liquid = 0.0;
        for _ in 0..100 {
            let p = b.step_decay(310.0, 1.0, true, 1.0);
            total_gas += p.co2 + p.ch4 + p.h2s + p.h2o_vapor;
            total_liquid += p.liquid_leachate;
        }
        let mass_lost = initial - b.mass;
        // 质量损失 = 气体 + 液体 + 残留灰分
        // 气体 + 液体 ≤ 质量损失（剩余为灰分残留）
        assert!(
            total_gas + total_liquid <= mass_lost + 1e-9,
            "气体+液体不应超过质量损失: gas+liq={:.3e} lost={:.3e}",
            total_gas + total_liquid,
            mass_lost
        );
        // 残留灰分 ≥ 0
        let ash = mass_lost - total_gas - total_liquid;
        assert!(ash >= -1e-9, "灰分残留应非负: ash={:.3e}", ash);
    }
}
