//! V8 沙盒 NPC 物理实体模块（物理轨，非认知轨）
//!
//! NPC 不是 cell，而是存在于 cell 网格中的实体，通过位置和体积影响环境。
//! 实现体温调节、呼吸气体交换、失血/窒息/燃烧等物理过程。
//!
//! 耦合方向：
//! - 环境 → NPC：温度/气压/O2/CO2/毒气/火源/水深 影响体温/血氧/意识/健康
//! - NPC → 环境：消耗 O2，产生 CO2 + H2O 蒸汽 + 代谢热

use serde::{Deserialize, Serialize};

// ─── 物理常量 ───────────────────────────────────────────────
/// 正常体温 37°C
pub const BODY_TEMP_NORMAL: f32 = 310.15;
/// 失温临界 27°C
pub const BODY_TEMP_HYPOTHERMIA: f32 = 300.0;
/// 中暑临界 40°C
pub const BODY_TEMP_HYPERTHERMIA: f32 = 313.0;
/// 人体比热 J/(kg·K)（主要含水）
pub const BODY_CP: f32 = 3500.0;
/// 正常血量 L
pub const BLOOD_VOLUME_NORMAL: f32 = 5.0;
/// 自然对流换热系数 W/(m²·K)
pub const HEAT_TRANSFER_COEFF: f32 = 5.0;
/// 静息代谢产热 W
pub const METABOLIC_HEAT_REST: f32 = 100.0;
/// 剧烈运动代谢产热 W
pub const METABOLIC_HEAT_ACTIVE: f32 = 500.0;
/// 最大出汗散热 W
pub const MAX_SWEAT_COOLING: f32 = 500.0;
/// 潮气量 L（静息）
pub const TIDAL_VOLUME_L: f32 = 0.5;
/// 吸入 O2 利用率
pub const O2_UTILIZATION: f32 = 0.25;
/// 呼吸商 RQ（CO2 产生 / O2 消耗 摩尔比）
pub const RESPIRATORY_QUOTIENT: f32 = 0.8;
/// O2 摩尔质量 kg/mol
pub const MOLAR_MASS_O2: f32 = 0.032;
/// CO2 摩尔质量 kg/mol
pub const MOLAR_MASS_CO2: f32 = 0.044;
/// 37°C 饱和水蒸气密度 g/m³
pub const SATURATED_H2O_VAPOR_37C: f32 = 44.0;
/// 空气 O2 体积分数
pub const O2_VOLUME_FRACTION: f32 = 0.21;
/// 窒息 O2 阈值（体积分数）
pub const O2_SUFFOCATION_THRESHOLD: f32 = 0.12;
/// CO2 中毒阈值（体积分数）
pub const CO2_TOXIC_THRESHOLD: f32 = 0.08;
/// 毒气中毒阈值（体积分数）
pub const TOXIC_GAS_THRESHOLD: f32 = 0.01;

// ─── 生理指标 ───────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcVitals {
    /// K 体温
    pub body_temp: f32,
    /// bpm 心率
    pub heart_rate: f32,
    /// breaths/min 呼吸频率
    pub breath_rate: f32,
    /// L 血量
    pub blood_volume: f32,
    /// 0..1 血氧饱和度
    pub oxygen_saturation: f32,
    /// 0..1 意识水平
    pub consciousness: f32,
    /// 0..1 疲劳度
    pub fatigue: f32,
    /// 0..1 饥饿度
    pub hunger: f32,
    /// 0..1 口渴度
    pub thirst: f32,
    /// 0..1 综合健康度
    pub health: f32,
}

impl Default for NpcVitals {
    fn default() -> Self {
        Self {
            body_temp: BODY_TEMP_NORMAL,
            heart_rate: 75.0,
            breath_rate: 15.0,
            blood_volume: BLOOD_VOLUME_NORMAL,
            oxygen_saturation: 0.98,
            consciousness: 1.0,
            fatigue: 0.0,
            hunger: 0.0,
            thirst: 0.0,
            health: 1.0,
        }
    }
}

// ─── NPC 实体 ───────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcEntity {
    pub id: u64,
    /// 世界坐标 m
    pub position: [f32; 3],
    /// m/s
    pub velocity: [f32; 3],
    /// kg
    pub mass: f32,
    /// m
    pub height: f32,
    /// m 碰撞半径
    pub radius: f32,
    pub vitals: NpcVitals,
    pub alive: bool,
    /// 基础代谢率倍数（1.0=静息, 5.0=剧烈运动）
    pub metabolic_rate: f32,
    /// 是否浸水
    pub in_water: bool,
    /// 0..1 燃烧度
    pub on_fire: f32,
    /// 可选认知轨：Maslow 需求驱动 + 行为决策 + 记忆
    /// None = 纯物理轨（默认），Some = 物理+认知双轨
    pub cognitive: Option<crate::npc_cognitive::CognitiveState>,
}

impl NpcEntity {
    /// 标准成年男性参数
    pub fn new_default(id: u64) -> Self {
        Self {
            id,
            position: [0.0; 3],
            velocity: [0.0; 3],
            mass: 70.0,
            height: 1.7,
            radius: 0.3,
            vitals: NpcVitals::default(),
            alive: true,
            metabolic_rate: 1.0,
            in_water: false,
            on_fire: 0.0,
            cognitive: None,
        }
    }

    pub fn new(id: u64, position: [f32; 3]) -> Self {
        let mut npc = Self::new_default(id);
        npc.position = position;
        npc
    }

    /// 启用认知轨（双轨耦合）
    pub fn with_cognitive(mut self, personality: crate::npc_cognitive::Personality) -> Self {
        self.cognitive = Some(crate::npc_cognitive::CognitiveState::new(personality));
        self
    }

    /// 推进一步物理，返回对环境的输出
    pub fn step(&mut self, env: &NpcEnvironment, dt: f32) -> NpcOutput {
        if !self.alive {
            return NpcOutput::default();
        }

        let mut output = NpcOutput::default();

        // 1. 体温调节
        let metabolic_heat = self.step_thermoregulation(env, dt);
        output.heat_generated = metabolic_heat;

        // 2. 呼吸气体交换
        let (o2_consumed, co2_produced, h2o_vapor) = self.step_respiration(env, dt);
        output.o2_consumed = o2_consumed;
        output.co2_produced = co2_produced;
        output.h2o_vapor = h2o_vapor;

        // 3. 窒息/中毒判定
        self.step_asphyxia(env, dt);

        // 4. 失血影响
        self.step_hemorrhage(dt);

        // 5. 燃烧伤害
        self.step_burn(dt);

        // 6. 体温极端影响健康
        self.step_temp_health_impact(dt);

        // 7. 环境热交换（NPC 从环境吸热或向环境散热）
        // 体温变化已包含环境热交换，这里计算输出给环境的热量
        let body_surface_area = 2.0 * self.mass.sqrt(); // 简化体表面积 m²
        let temp_diff = env.temperature - self.vitals.body_temp;
        // NPC 从环境吸收的热量（正=吸热，负=散热）
        output.heat_absorbed = HEAT_TRANSFER_COEFF * body_surface_area * temp_diff * dt;

        // 8. 死亡判定
        self.check_death();

        output
    }

    /// 体温调节：代谢产热 + 环境热交换 + 蒸发散热
    fn step_thermoregulation(&mut self, env: &NpcEnvironment, dt: f32) -> f32 {
        // 代谢产热 W
        let metabolic_heat = METABOLIC_HEAT_REST
            + (METABOLIC_HEAT_ACTIVE - METABOLIC_HEAT_REST) * (self.metabolic_rate - 1.0).max(0.0);

        // 体表面积 m²（简化公式）
        let body_surface_area = 2.0 * self.mass.sqrt();

        // 环境热交换 W（对流）
        let conv_heat = HEAT_TRANSFER_COEFF * body_surface_area
            * (env.temperature - self.vitals.body_temp);

        // 蒸发散热 W（出汗/呼吸蒸发，体温升高时增加）
        // 底层改造：出汗系数更激进（/1.5，上限 1.5），体温升高时强化散热
        let sweat_factor = ((self.vitals.body_temp - BODY_TEMP_NORMAL) / 1.5).clamp(0.0, 1.5);
        let sweat_cooling = MAX_SWEAT_COOLING * sweat_factor;

        // 浸水时额外散热（水导热远大于空气）
        let water_cooling = if self.in_water { 200.0 * body_surface_area } else { 0.0 };

        // 净热功率 W
        let net_heat = metabolic_heat + conv_heat - sweat_cooling - water_cooling;

        // 体温变化 dT = Q / (m * cp)
        let d_t = net_heat * dt / (self.mass * BODY_CP);
        self.vitals.body_temp += d_t;

        // 底层改造：热耐受应急保护
        // 体温 > 316K（42.85°C）时启动应急冷却：血管极度扩张+大量出汗
        // 等效为额外散热功率，避免体温失控
        let temp_safety = 316.0;
        if self.vitals.body_temp > temp_safety {
            let excess = self.vitals.body_temp - temp_safety;
            // 应激冷却功率 = excess * 500 W/K（强效散热）
            let emergency_cooling = excess * 500.0;
            let emergency_d_t = -emergency_cooling * dt / (self.mass * BODY_CP);
            self.vitals.body_temp += emergency_d_t;
            // 体温 clamp 到 318K（45°C）上限，避免瞬间死亡
            self.vitals.body_temp = self.vitals.body_temp.min(318.0);
        }

        metabolic_heat * dt // 返回代谢产热 J
    }

    /// 呼吸气体交换
    fn step_respiration(&mut self, env: &NpcEnvironment, dt: f32) -> (f32, f32, f32) {
        // 呼吸频率随代谢率和体温调整
        let effective_breath_rate = self.vitals.breath_rate * self.metabolic_rate
            * (1.0 + (self.vitals.body_temp - BODY_TEMP_NORMAL).max(0.0) * 0.1);

        // 每次呼吸潮气量 L（运动时增加）
        let tidal_volume_l = TIDAL_VOLUME_L * self.metabolic_rate;

        // 每秒呼吸次数
        let breaths_per_sec = effective_breath_rate / 60.0;

        // 每秒吸入空气体积 L
        let inhaled_air_l_per_sec = tidal_volume_l * breaths_per_sec;

        // O2 消耗：吸入空气体积 × O2 体积分数 × 利用率
        // 转换为 kg：L × (mol/L) × (kg/mol)
        // 标准状况 1 mol 气体 = 22.4 L，所以 1 L = 1/22.4 mol
        let o2_moles_consumed = inhaled_air_l_per_sec * O2_VOLUME_FRACTION * O2_UTILIZATION / 22.4;
        let o2_consumed = o2_moles_consumed * MOLAR_MASS_O2 * dt; // kg

        // CO2 产生：按呼吸商 RQ，CO2 mol = RQ × O2 mol
        let co2_moles_produced = o2_moles_consumed * RESPIRATORY_QUOTIENT;
        let co2_produced = co2_moles_produced * MOLAR_MASS_CO2 * dt; // kg

        // 呼出水蒸气：37°C 饱和水蒸气密度 × 呼出体积
        // 吸入空气假设干燥，呼出饱和水蒸气
        let h2o_vapor_g = inhaled_air_l_per_sec * SATURATED_H2O_VAPOR_37C / 1000.0 * dt; // kg
        let h2o_vapor = h2o_vapor_g;

        // 检查环境 O2 是否足够（如果环境无 O2 则无法呼吸）
        if env.oxygen_mass < o2_consumed {
            // O2 不足，血氧下降
            self.vitals.oxygen_saturation -= 0.05 * dt;
            self.vitals.oxygen_saturation = self.vitals.oxygen_saturation.max(0.0);
            return (0.0, co2_produced * 0.1, 0.0); // 几乎无法呼吸
        }

        // 血氧恢复（正常呼吸时）
        if self.vitals.oxygen_saturation < 0.98 {
            self.vitals.oxygen_saturation += 0.02 * dt;
            self.vitals.oxygen_saturation = self.vitals.oxygen_saturation.min(1.0);
        }

        (o2_consumed, co2_produced, h2o_vapor)
    }

    /// 窒息/中毒判定
    fn step_asphyxia(&mut self, env: &NpcEnvironment, dt: f32) {
        // 计算环境气体体积分数（简化：用质量分数近似）
        let total_gas = env.oxygen_mass + env.co2_mass + env.toxic_gas + 0.029; // 加 N2 基准
        let o2_fraction = env.oxygen_mass / total_gas.max(1e-6);
        let co2_fraction = env.co2_mass / total_gas.max(1e-6);
        let toxic_fraction = env.toxic_gas / total_gas.max(1e-6);

        // O2 不足
        if o2_fraction < O2_SUFFOCATION_THRESHOLD {
            let deficit = (O2_SUFFOCATION_THRESHOLD - o2_fraction) / O2_SUFFOCATION_THRESHOLD;
            self.vitals.oxygen_saturation -= deficit * 0.05 * dt;  // 底层改造：窒息速率减半
        }

        // CO2 中毒
        if co2_fraction > CO2_TOXIC_THRESHOLD {
            let excess = (co2_fraction - CO2_TOXIC_THRESHOLD) / CO2_TOXIC_THRESHOLD;
            self.vitals.consciousness -= excess * 0.05 * dt;
        }

        // 毒气中毒
        if toxic_fraction > TOXIC_GAS_THRESHOLD {
            let excess = (toxic_fraction - TOXIC_GAS_THRESHOLD) / TOXIC_GAS_THRESHOLD;
            self.vitals.consciousness -= excess * 0.1 * dt;
            self.vitals.health -= excess * 0.02 * dt;
        }

        // 浸水窒息
        if self.in_water {
            self.vitals.oxygen_saturation -= 0.05 * dt;  // 底层改造：浸水窒息速率减半
        }

        // 血氧低 → 意识下降
        if self.vitals.oxygen_saturation < 0.85 {
            let deficit = (0.85 - self.vitals.oxygen_saturation) / 0.85;
            self.vitals.consciousness -= deficit * 0.05 * dt;
        }

        // 钳制
        self.vitals.oxygen_saturation = self.vitals.oxygen_saturation.clamp(0.0, 1.0);
        self.vitals.consciousness = self.vitals.consciousness.clamp(0.0, 1.0);
        self.vitals.health = self.vitals.health.clamp(0.0, 1.0);
    }

    /// 失血影响
    fn step_hemorrhage(&mut self, dt: f32) {
        // 血量低 → 心率代偿性升高
        let blood_deficit = (BLOOD_VOLUME_NORMAL - self.vitals.blood_volume) / BLOOD_VOLUME_NORMAL;
        if blood_deficit > 0.0 {
            self.vitals.heart_rate = 75.0 + blood_deficit * 100.0; // 失血时心率加快

            // 严重失血 → 血氧下降（供血不足）
            if self.vitals.blood_volume < 3.5 {
                self.vitals.oxygen_saturation -= 0.02 * dt;
            }
            if self.vitals.blood_volume < 2.5 {
                self.vitals.consciousness -= 0.05 * dt;
                self.vitals.health -= 0.02 * dt;
            }
        }

        self.vitals.oxygen_saturation = self.vitals.oxygen_saturation.clamp(0.0, 1.0);
        self.vitals.consciousness = self.vitals.consciousness.clamp(0.0, 1.0);
        self.vitals.health = self.vitals.health.clamp(0.0, 1.0);
    }

    /// 燃烧伤害
    fn step_burn(&mut self, dt: f32) {
        if self.on_fire > 0.0 {
            // 燃烧导致体温快速上升
            let burn_heat = 50_000.0 * self.on_fire; // W 燃烧产热（真实人体着火约50kW）
            let d_t = burn_heat * dt / (self.mass * BODY_CP);
            self.vitals.body_temp += d_t;

            // 燃烧导致健康度下降
            self.vitals.health -= 0.05 * self.on_fire * dt;

            // 燃烧度自然衰减（简化扑救效果）
            self.on_fire -= 0.01 * dt;
            self.on_fire = self.on_fire.max(0.0);
        }
        self.vitals.health = self.vitals.health.clamp(0.0, 1.0);
    }

    /// 体温极端影响健康
    fn step_temp_health_impact(&mut self, dt: f32) {
        // 失温
        if self.vitals.body_temp < BODY_TEMP_HYPOTHERMIA {
            let severity = (BODY_TEMP_HYPOTHERMIA - self.vitals.body_temp) / 10.0;
            self.vitals.consciousness -= severity * 0.02 * dt;
            self.vitals.health -= severity * 0.01 * dt;
        }
        // 中暑
        if self.vitals.body_temp > BODY_TEMP_HYPERTHERMIA {
            let severity = (self.vitals.body_temp - BODY_TEMP_HYPERTHERMIA) / 3.0;
            self.vitals.consciousness -= severity * 0.03 * dt;
            self.vitals.health -= severity * 0.02 * dt;
        }

        self.vitals.consciousness = self.vitals.consciousness.clamp(0.0, 1.0);
        self.vitals.health = self.vitals.health.clamp(0.0, 1.0);
    }

    /// 死亡判定
    fn check_death(&mut self) {
        if self.alive {
            if self.vitals.health <= 0.0
                || self.vitals.blood_volume < 1.5
                || self.vitals.oxygen_saturation <= 0.0
                || self.vitals.body_temp < 270.0
                || self.vitals.body_temp > 322.0
            {
                let reason = if self.vitals.health <= 0.0 { "health=0" }
                    else if self.vitals.blood_volume < 1.5 { "blood_loss" }
                    else if self.vitals.oxygen_saturation <= 0.0 { "suffocation" }
                    else if self.vitals.body_temp < 270.0 { "hypothermia" }
                    else { "hyperthermia" };
                eprintln!("[NPC death] reason={}, health={:.3}, blood={:.2}L, O2={:.3}, T={:.1}K",
                    reason, self.vitals.health, self.vitals.blood_volume,
                    self.vitals.oxygen_saturation, self.vitals.body_temp);
                self.alive = false;
                self.vitals.consciousness = 0.0;
            }
        }
    }

    /// 受伤（减少血量）
    pub fn take_damage(&mut self, blood_loss: f32) {
        self.vitals.blood_volume -= blood_loss;
        self.vitals.blood_volume = self.vitals.blood_volume.max(0.0);
    }

    /// 着火
    pub fn ignite(&mut self, intensity: f32) {
        self.on_fire = intensity.clamp(0.0, 1.0);
    }

    /// 浸水状态更新
    pub fn update_submersion(&mut self, water_depth: f32) {
        self.in_water = water_depth > self.height * 0.5; // 水深超过身高一半视为浸水
    }

    pub fn is_alive(&self) -> bool {
        self.alive
    }
}

// ─── 环境信息（从 cell 读取） ───────────────────────────────
#[derive(Debug, Clone, Default)]
pub struct NpcEnvironment {
    /// K 环境温度
    pub temperature: f32,
    /// Pa
    pub pressure: f32,
    /// kg 周围 O2 量
    pub oxygen_mass: f32,
    /// kg 周围 CO2 量
    pub co2_mass: f32,
    /// kg 有毒气体量（CH4/H2S/CO）
    pub toxic_gas: f32,
    /// m 浸水深度（0=不浸水）
    pub water_depth: f32,
    /// 0..1 周围火源强度
    pub fire_intensity: f32,
    /// m/s 风速
    pub wind_speed: f32,
}

// ─── NPC 对环境的输出 ───────────────────────────────────────
#[derive(Debug, Clone, Default)]
pub struct NpcOutput {
    /// kg 消耗的 O2
    pub o2_consumed: f32,
    /// kg 产生的 CO2
    pub co2_produced: f32,
    /// kg 呼出水蒸气
    pub h2o_vapor: f32,
    /// J 代谢产热
    pub heat_generated: f32,
    /// J 从环境吸热（正=吸热，负=散热）
    pub heat_absorbed: f32,
}

// ─── 单元测试 ───────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn default_env() -> NpcEnvironment {
        NpcEnvironment {
            temperature: 293.15, // 20°C 舒适
            pressure: 101_325.0,
            oxygen_mass: 0.3,    // 充足 O2
            co2_mass: 0.0,
            toxic_gas: 0.0,
            water_depth: 0.0,
            fire_intensity: 0.0,
            wind_speed: 0.0,
        }
    }

    #[test]
    fn test_npc_default_params() {
        let npc = NpcEntity::new_default(1);
        assert!(npc.alive);
        assert!((npc.vitals.body_temp - BODY_TEMP_NORMAL).abs() < 0.1);
        assert!((npc.vitals.blood_volume - BLOOD_VOLUME_NORMAL).abs() < 0.1);
        assert!((npc.vitals.oxygen_saturation - 0.98).abs() < 0.01);
        assert_eq!(npc.mass, 70.0);
    }

    #[test]
    fn test_respiration_consumes_o2() {
        let mut npc = NpcEntity::new_default(1);
        let env = default_env();
        let mut total_o2 = 0.0;
        let mut total_co2 = 0.0;
        for _ in 0..60 {
            let out = npc.step(&env, 1.0 / 60.0);
            total_o2 += out.o2_consumed;
            total_co2 += out.co2_produced;
        }
        assert!(total_o2 > 0.0, "应消耗 O2: {}", total_o2);
        assert!(total_co2 > 0.0, "应产生 CO2: {}", total_co2);
    }

    #[test]
    fn test_metabolic_heat() {
        let mut npc = NpcEntity::new_default(1);
        let env = default_env();
        let out = npc.step(&env, 1.0);
        assert!(out.heat_generated > 0.0, "静息代谢应产热: {}", out.heat_generated);
    }

    #[test]
    fn test_cold_environment_drops_body_temp() {
        let mut npc = NpcEntity::new_default(1);
        let mut env = default_env();
        env.temperature = 270.0; // -3°C 寒冷
        let t0 = npc.vitals.body_temp;
        for _ in 0..60 {
            npc.step(&env, 1.0 / 60.0);
        }
        assert!(npc.vitals.body_temp < t0, "寒冷环境体温应下降: t0={} t1={}", t0, npc.vitals.body_temp);
    }

    #[test]
    fn test_hot_environment_raises_body_temp() {
        let mut npc = NpcEntity::new_default(1);
        let mut env = default_env();
        env.temperature = 340.0; // 67°C 高温
        let t0 = npc.vitals.body_temp;
        for _ in 0..60 {
            npc.step(&env, 1.0 / 60.0);
        }
        assert!(npc.vitals.body_temp > t0, "高温环境体温应上升: t0={} t1={}", t0, npc.vitals.body_temp);
    }

    #[test]
    fn test_hemorrhage_shock() {
        let mut npc = NpcEntity::new_default(1);
        let hr0 = npc.vitals.heart_rate;
        npc.take_damage(2.5); // 失血 2.5L，血量降到 2.5L
        let env = default_env();
        npc.step(&env, 1.0);
        assert!(npc.vitals.heart_rate > hr0, "失血应心率加快: hr0={} hr1={}", hr0, npc.vitals.heart_rate);
    }

    #[test]
    fn test_hemorrhage_death() {
        let mut npc = NpcEntity::new_default(1);
        npc.take_damage(4.0); // 血量降到 1L
        let env = default_env();
        npc.step(&env, 1.0);
        assert!(!npc.alive, "严重失血应死亡");
    }

    #[test]
    fn test_suffocation_low_o2() {
        let mut npc = NpcEntity::new_default(1);
        let mut env = default_env();
        env.oxygen_mass = 0.001; // 极低 O2
        let sat0 = npc.vitals.oxygen_saturation;
        for _ in 0..120 {
            npc.step(&env, 1.0 / 60.0);
        }
        assert!(npc.vitals.oxygen_saturation < sat0, "低 O2 应血氧下降: sat0={} sat1={}", sat0, npc.vitals.oxygen_saturation);
    }

    #[test]
    fn test_co2_poisoning() {
        let mut npc = NpcEntity::new_default(1);
        let mut env = default_env();
        env.co2_mass = 0.5; // 高 CO2
        let con0 = npc.vitals.consciousness;
        for _ in 0..120 {
            npc.step(&env, 1.0 / 60.0);
        }
        assert!(npc.vitals.consciousness < con0, "高 CO2 应意识下降: con0={} con1={}", con0, npc.vitals.consciousness);
    }

    #[test]
    fn test_burn_damage() {
        let mut npc = NpcEntity::new_default(1);
        npc.ignite(1.0);
        let health0 = npc.vitals.health;
        let temp0 = npc.vitals.body_temp;
        let env = default_env();
        for _ in 0..60 {
            npc.step(&env, 1.0 / 60.0);
        }
        assert!(npc.vitals.health < health0, "燃烧应降低健康: h0={} h1={}", health0, npc.vitals.health);
        assert!(npc.vitals.body_temp > temp0, "燃烧应升高体温: t0={} t1={}", temp0, npc.vitals.body_temp);
    }

    #[test]
    fn test_submersion_suffocation() {
        let mut npc = NpcEntity::new_default(1);
        npc.update_submersion(2.0); // 水深 2m，超过身高
        assert!(npc.in_water, "应判定为浸水");
        let env = default_env();
        let sat0 = npc.vitals.oxygen_saturation;
        for _ in 0..60 {
            npc.step(&env, 1.0 / 60.0);
        }
        assert!(npc.vitals.oxygen_saturation < sat0, "浸水应血氧下降");
    }

    #[test]
    fn test_exercise_metabolism() {
        let mut npc_rest = NpcEntity::new_default(1);
        let mut npc_active = NpcEntity::new_default(2);
        npc_active.metabolic_rate = 5.0; // 剧烈运动
        let env = default_env();
        let mut o2_rest = 0.0;
        let mut o2_active = 0.0;
        for _ in 0..60 {
            o2_rest += npc_rest.step(&env, 1.0 / 60.0).o2_consumed;
            o2_active += npc_active.step(&env, 1.0 / 60.0).o2_consumed;
        }
        assert!(o2_active > o2_rest * 2.0, "运动应消耗远多于静息: rest={} active={}", o2_rest, o2_active);
    }
}
