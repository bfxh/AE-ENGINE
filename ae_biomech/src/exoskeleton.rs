// 外骨骼系统 (Exoskeleton) - 节肢动物/软体动物硬组织
// 几丁质外骨骼分层硬化 + 蜕皮动力学
// 甲壳类 CaCO3 钙化 (方解石/文石/ACC 多型)
// 软体动物壳 (角质层/棱柱层/珍珠层)
// 来源:
//   - Vincent JFV (1990) "Structural Biomaterials" Princeton UP
//   - Hepburn HR (1976) "The Insect Integument" Elsevier
//   - Weiner S, Dove PM (2003) "Biomineralization Reviews in Mineralogy" 54:1-29
//   - Vincent JFV, Wegst UGK (2004) Arthropod Struct Dev 33:187-199
//   - Roer RD, Dillaman RM (1984) Am Zool 24:893-909
//   - Barthelat F et al. (2006) J Mater Res 21:1977
//   - Meyers MA et al. (2008) Prog Mater Sci 53:1-206

use serde::{Deserialize, Serialize};

// === 物理常数 (SI 单位, 注明来源) ===
/// 邻苯二酚-醌硬化时间常数 tau_scl (s), ~ 6h = 21600s
/// Vincent 1990, Ch. 4
pub const TAU_SCLEROTIZATION: f32 = 21_600.0;
/// 钙化时间常数 tau_calc (s), ~ 24h = 86400s
/// Roer & Dillaman (1984) Am Zool 24:893-909
pub const TAU_CALCIFICATION: f32 = 86_400.0;
/// 蜕皮后膨胀-硬化窗口 (s), ~ 1h = 3600s
/// Reynolds SE (1977) Adv Insect Physiol 13:1-38
pub const POST_ECDYSIS_WINDOW: f32 = 3_600.0;
/// 20-羟基蜕皮激素 (20-HE) 峰值浓度 (nM), 触发蜕皮
/// Riddiford LM (1993) Insect Biochem Mol Biol 23:131-136
pub const ECDYSONE_PEAK_NM: f32 = 1500.0;

/// 上表皮 (epicuticle) 厚度 (um), ~ 1-3 um
pub const EPICUTICLE_THICKNESS_UM: f32 = 2.0;
/// 外表皮 (exocuticle) 厚度 (um), ~ 10-200 um
pub const EXOCUTICLE_THICKNESS_UM: f32 = 100.0;
/// 内表皮 (endocuticle) 厚度 (um), ~ 50-500 um
pub const ENDOCUTICLE_THICKNESS_UM: f32 = 200.0;

/// 弹性模量上下限 (GPa), Vincent 1990
pub const E_UNSCLEROTIZED_GPA: f32 = 0.1;
pub const E_SCLEROTIZED_GPA: f32 = 5.0;
pub const E_CALCIFIED_GPA: f32 = 10.0;
/// 拉伸强度范围 (MPa)
pub const SIGMA_MIN_MPA: f32 = 100.0;
pub const SIGMA_MAX_MPA: f32 = 300.0;
/// 断裂韧性 K_ic 范围 (MPa*m^0.5)
pub const K_IC_MIN: f32 = 1.0;
pub const K_IC_MAX: f32 = 5.0;

// === 化学组成 (摩尔比, 标注来源) ===
/// N-乙酰葡糖胺 (NAG) 摩尔分数, ~ 0.5
/// 几丁质由 beta-1,4 糖苷键连接 NAG 与葡萄糖胺
pub const NAG_FRACTION: f32 = 0.5;

/// 外骨骼分层
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExoskeletonLayer {
    /// 上表皮 - 蜡质层, 防水
    Epicuticle,
    /// 外表皮 - 硬化+钙化
    Exocuticle,
    /// 内表皮 - 柔软, 未硬化
    Endocuticle,
}

/// 关节类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JointType {
    /// 铰链关节 (单自由度, 如蝗虫膝)
    Hinge,
    /// 球窝关节 (多自由度, 如蝈蝈腿基节)
    BallAndSocket,
    /// 滑动关节
    Gliding,
}

/// 几丁质外骨骼层参数
/// 化学组成: N-乙酰葡糖胺+葡萄糖胺 beta-1,4 糖苷键 (几丁质)
/// 分层: 上表皮(蜡质,防水) / 外表皮(硬化,钙化) / 内表皮(柔软,未硬化)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ChitinExoskeleton {
    /// 上表皮厚度 (um)
    pub epicuticle_thickness_um: f32,
    /// 外表皮厚度 (um)
    pub exocuticle_thickness_um: f32,
    /// 内表皮厚度 (um)
    pub endocuticle_thickness_um: f32,
    /// 硬化程度 s in [0, 1], 0=未硬化, 1=完全硬化
    /// 邻苯二酚氧化为醌, 与蛋白质交联
    pub sclerotization: f32,
    /// 钙化程度 c in [0, 1], 0=未钙化, 1=完全钙化
    pub calcification: f32,
    /// 当前 20-HE 浓度 (nM)
    pub ecdysone_nm: f32,
    /// 蜕皮阶段标记
    pub molting: bool,
}
impl Default for ChitinExoskeleton {
    fn default() -> Self {
        Self {
            epicuticle_thickness_um: EPICUTICLE_THICKNESS_UM,
            exocuticle_thickness_um: EXOCUTICLE_THICKNESS_UM,
            endocuticle_thickness_um: ENDOCUTICLE_THICKNESS_UM,
            sclerotization: 1.0,
            calcification: 0.0,
            ecdysone_nm: 0.0,
            molting: false,
        }
    }
}

impl ChitinExoskeleton {
    pub fn new() -> Self {
        Self::default()
    }

    /// 总厚度 (um)
    pub fn total_thickness_um(&self) -> f32 {
        self.epicuticle_thickness_um
            + self.exocuticle_thickness_um
            + self.endocuticle_thickness_um
    }

    /// 当前弹性模量 (GPa)
    /// E = E_unscl + (E_scl - E_unscl) * s + (E_calc - E_scl) * c
    /// 钙化叠加在硬化之上, Vincent 1990
    pub fn youngs_modulus_gpa(&self) -> f32 {
        let s = self.sclerotization.clamp(0.0, 1.0);
        let c = self.calcification.clamp(0.0, 1.0);
        let e_scl = E_UNSCLEROTIZED_GPA + (E_SCLEROTIZED_GPA - E_UNSCLEROTIZED_GPA) * s;
        e_scl + (E_CALCIFIED_GPA - E_SCLEROTIZED_GPA) * c
    }

    /// 拉伸强度 (MPa) - 与硬化程度线性相关
    pub fn tensile_strength_mpa(&self) -> f32 {
        let s = self.sclerotization.clamp(0.0, 1.0);
        SIGMA_MIN_MPA + (SIGMA_MAX_MPA - SIGMA_MIN_MPA) * s
    }

    /// 断裂韧性 K_ic (MPa*m^0.5) - 钙化提高韧性
    pub fn fracture_toughness(&self) -> f32 {
        let s = self.sclerotization.clamp(0.0, 1.0);
        let c = self.calcification.clamp(0.0, 1.0);
        let k_scl = K_IC_MIN + (K_IC_MAX - K_IC_MIN) * 0.5 * s;
        k_scl + (K_IC_MAX - K_IC_MIN) * 0.5 * c
    }

    /// 硬化动力学: ds/dt = (1 - s) / tau_scl
    /// 邻苯二酚氧化为醌, 与蛋白质交联 (Vincent 1990)
    pub fn step_sclerotization(&mut self, dt: f32) {
        let ds = (1.0 - self.sclerotization) / TAU_SCLEROTIZATION * dt;
        self.sclerotization = (self.sclerotization + ds).clamp(0.0, 1.0);
    }

    /// 钙化动力学: dc/dt = (1 - c) / tau_calc
    /// CaCO3 沉积 (Roer & Dillaman 1984)
    pub fn step_calcification(&mut self, dt: f32) {
        let dc = (1.0 - self.calcification) / TAU_CALCIFICATION * dt;
        self.calcification = (self.calcification + dc).clamp(0.0, 1.0);
    }

    /// 蜕皮激素演化: 20-HE 浓度由内部节律驱动
    /// 峰值 > ECDYSONE_PEAK_NM 时触发蜕皮 (apolysis)
    pub fn step_ecdysone(&mut self, dt: f32, target_nm: f32) {
        // 简化: 一阶趋近 target, tau = 6h
        let tau = 21_600.0;
        let decay = (-dt / tau).exp();
        self.ecdysone_nm = self.ecdysone_nm * decay + target_nm * (1.0 - decay);
        if self.ecdysone_nm >= ECDYSONE_PEAK_NM {
            self.molting = true;
        }
    }

    /// 启动蜕皮过程 (apolysis - 旧表皮分离)
    pub fn trigger_molting(&mut self) {
        self.molting = true;
    }

    /// 完成蜕皮: 新表皮暴露, 开始硬化窗口
    /// 旧表皮被吞食以节省营养 (Weiss-Fogh: 回收率 ~85%)
    pub fn complete_ecdysis(&mut self) {
        if self.molting {
            // 新表皮初始未硬化
            self.sclerotization = 0.0;
            self.calcification = 0.0;
            self.ecdysone_nm = 0.0;
            self.molting = false;
        }
    }

    /// 蜕皮后膨胀窗口检查 (秒)
    /// 在 POST_ECDYSIS_WINDOW 内可膨胀至最终尺寸
    pub fn is_inflation_window(&self, time_since_ecdysis_s: f32) -> bool {
        time_since_ecdysis_s < POST_ECDYSIS_WINDOW
    }
}
/// CaCO3 多型 (Polymorph)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Caco3Polymorph {
    /// 无定形碳酸钙 (ACC) - 前体相
    Acc,
    /// 方解石 calcite - 稳定相 (三方晶系)
    Calcite,
    /// 文石 aragonite - 亚稳相 (正交晶系)
    Aragonite,
}

impl Caco3Polymorph {
    /// 密度 (g/cm^3), Weiner & Dove 2003
    pub fn density_g_cm3(&self) -> f32 {
        match self {
            Self::Acc => 1.6,
            Self::Calcite => 2.71,
            Self::Aragonite => 2.93,
        }
    }
    /// 摩尔质量 (g/mol), CaCO3 (与多型无关)
    pub fn molar_mass_g_mol(&self) -> f32 {
        100.09
    }
    /// 热力学稳定性 (越高越稳定)
    pub fn stability(&self) -> f32 {
        match self {
            Self::Acc => 0.0,
            Self::Aragonite => 0.5,
            Self::Calcite => 1.0,
        }
    }
}

/// 甲壳类钙化模型
/// CaCO3 沉积/溶解由 Ca2+ 和 CO3^2- 浓度驱动
/// d[CaCO3]/dt = k * [Ca2+] * [CO3^2-]
/// 来源:
///   - Roer RD, Dillaman RM (1984) Am Zool 24:893-909
///   - Weiss IM et al. (2002) J Exp Zool 293:478-487
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CrustaceanCalcification {
    /// 钙化速率常数 k (mM^-1 h^-1), ~ 0.1
    pub k_calc: f32,
    /// Ca2+ 浓度 (mM), 海水 ~ 10 mM
    pub ca_conc_mM: f32,
    /// CO3^2- 浓度 (mM), 海水 pH 8.1 ~ 0.2 mM
    pub co3_conc_mM: f32,
    /// 当前 pH
    pub ph: f32,
    /// ACC -> 方解石转化时间常数 tau_acc (h), ~ 6h
    pub tau_acc_to_calcite_h: f32,
    /// 脱钙系数 (1/h), 控制 pH < 7.5 时的溶解速率
    pub decalc_constant: f32,
}

impl Default for CrustaceanCalcification {
    fn default() -> Self {
        Self {
            k_calc: 0.1,
            ca_conc_mM: 10.0,
            co3_conc_mM: 0.2,
            ph: 8.1,
            tau_acc_to_calcite_h: 6.0,
            decalc_constant: 0.5,
        }
    }
}

impl CrustaceanCalcification {
    pub fn new() -> Self {
        Self::default()
    }

    /// 瞬时钙化速率 d[CaCO3]/dt (mM/h)
    /// d[CaCO3]/dt = k * [Ca2+] * [CO3^2-]
    /// 受 pH 调控: 低 pH (酸化) 抑制 CaCO3 沉积
    /// pH 修正因子: f = 1 / (1 + 10^(pK - pH)), pK = 7.5
    pub fn calcification_rate(&self) -> f32 {
        let pK = 7.5;
        let ph_factor = 1.0 / (1.0 + 10.0_f32.powf(pK - self.ph));
        self.k_calc * self.ca_conc_mM * self.co3_conc_mM * ph_factor
    }

    /// 脱钙速率 (mM/h, 正值表示被溶解的速率)
    /// 低 pH (< 7.5) 触发 CaCO3 溶解
    /// 速率正比于 pH 偏离 7.5 的程度: rate = k * (7.5 - pH)
    /// k = 10 mM/(h*pH单位), pH 6.5 时 rate = 10 mM/h
    pub fn decalcification_rate(&self) -> f32 {
        if self.ph < 7.5 {
            self.decalc_constant * 20.0 * (7.5 - self.ph)
        } else {
            0.0
        }
    }

    /// 单步推进钙化 (dt 单位: h)
    /// 同时模拟 ACC -> calcite 转化 (一阶动力学)
    /// caco3_field: 已结晶 CaCO3 浓度 (mM)
    /// acc_field: ACC 前体浓度 (mM)
    pub fn step(&self, caco3_field: &mut [f32], acc_field: &mut [f32], dt: f32) {
        let net_rate = self.calcification_rate();
        let dec_rate = self.decalcification_rate();
        let convert = 1.0 / self.tau_acc_to_calcite_h;
        for (caco3, acc) in caco3_field.iter_mut().zip(acc_field.iter_mut()) {
            // 新沉积的 CaCO3 进入 ACC 池
            *acc = (*acc + net_rate * dt).max(0.0);
            // ACC -> calcite 转化
            let conv = (*acc * convert * dt).min(*acc);
            *acc -= conv;
            *caco3 += conv;
            // 脱钙 (作用于已结晶 CaCO3, pH < 7.5 时)
            let dissolved = (dec_rate * dt).min(*caco3);
            *caco3 -= dissolved;
        }
    }
}
/// 软体动物壳层类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShellLayer {
    /// 角质层 periostracum - conchiolin 蛋白, 外层有机
    Periostracum,
    /// 棱柱层 prismatic - 方解石柱
    Prismatic,
    /// 珍珠层 nacre - 文石片+有机基质 (砖泥结构)
    Nacre,
}

/// 软体动物壳模型
/// 三层结构 + 珍珠层韧化机制 (片层滑移)
/// 来源:
///   - Weiner S, Dove PM (2003) Biomineralization Reviews in Mineralogy 54
///   - Barthelat F et al. (2006) J Mater Res 21:1977
///   - Meyers MA et al. (2008) Prog Mater Sci 53:1-206
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MolluskShell {
    /// 角质层厚度 (um), ~ 10-50
    pub periostracum_thickness_um: f32,
    /// 棱柱层厚度 (um), ~ 100-500
    pub prismatic_thickness_um: f32,
    /// 珍珠层厚度 (um), ~ 50-500
    pub nacre_thickness_um: f32,
    /// 珍珠层中文石片层厚度 (nm), ~ 500
    pub nacre_tablet_thickness_nm: f32,
    /// 珍珠层有机基质体积分数 (0-1), ~ 0.05
    pub nacre_organic_fraction: f32,
    /// 棱柱层方解石柱直径 (um), ~ 5-20
    pub prismatic_column_diameter_um: f32,
    /// 外套膜活性 (0-1), 影响生长率
    pub mantle_activity: f32,
}

impl Default for MolluskShell {
    fn default() -> Self {
        Self {
            periostracum_thickness_um: 30.0,
            prismatic_thickness_um: 300.0,
            nacre_thickness_um: 200.0,
            nacre_tablet_thickness_nm: 500.0,
            nacre_organic_fraction: 0.05,
            prismatic_column_diameter_um: 10.0,
            mantle_activity: 0.8,
        }
    }
}

impl MolluskShell {
    pub fn new() -> Self {
        Self::default()
    }

    /// 总厚度 (um)
    pub fn total_thickness_um(&self) -> f32 {
        self.periostracum_thickness_um + self.prismatic_thickness_um + self.nacre_thickness_um
    }

    /// 珍珠层断裂韧性 K_ic (MPa*m^0.5)
    /// 范围 5-8, 由片层滑移+有机桥联增韧
    /// Barthelat 2006: K_ic = 5 + 3 * (organic_fraction / 0.05)
    pub fn nacre_fracture_toughness(&self) -> f32 {
        let f = self.nacre_organic_fraction.clamp(0.0, 0.2);
        let base = 5.0;
        let bonus = 3.0 * (f / 0.05).min(1.5);
        (base + bonus).clamp(5.0, 8.0)
    }

    /// 珍珠层弹性模量 (GPa)
    /// 文石片层 E_tablet ~ 100 GPa, 复合 E = E_tablet * (1 - organic_fraction)
    pub fn nacre_youngs_modulus_gpa(&self) -> f32 {
        let e_tablet = 100.0;
        e_tablet * (1.0 - self.nacre_organic_fraction.clamp(0.0, 0.5))
    }

    /// 棱柱层弹性模量 (GPa) - 方解石柱, E_calcite ~ 90 GPa
    pub fn prismatic_youngs_modulus_gpa(&self) -> f32 {
        90.0
    }

    /// 生物矿化速率: 外套膜分泌有机基质 -> 矿物成核 -> 晶体生长
    /// 简化为线性生长率 (um/day)
    /// 生长率正比于外套膜活性和 Ca2+ 浓度
    /// 来源: Wilbur KH (1964) in Wilbur & Yonge "Physiology of Mollusca"
    pub fn biomineralization_rate_um_day(&self, ca_conc_mM: f32) -> f32 {
        let k_growth = 0.5; // um/day per mM per unit activity
        k_growth * self.mantle_activity.clamp(0.0, 1.0) * ca_conc_mM
    }

    /// 估算日轮数 (给定总厚度)
    /// 假设每 um 厚度对应 1 个日轮 (典型生长速率 1 um/day)
    pub fn estimate_growth_lines(&self) -> usize {
        (self.total_thickness_um() / 1.0).round() as usize
    }

    /// 单步生长 (dt 单位: day)
    /// 三层按各自生长率增厚
    pub fn step_growth(&mut self, ca_conc_mM: f32, dt: f32) {
        let r = self.biomineralization_rate_um_day(ca_conc_mM);
        // 珍珠层和棱柱层按比例分配
        self.nacre_thickness_um += r * 0.5 * dt;
        self.prismatic_thickness_um += r * 0.4 * dt;
        self.periostracum_thickness_um += r * 0.1 * dt;
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chitin_modulus_range() {
        let mut e = ChitinExoskeleton::default();
        e.sclerotization = 0.0;
        e.calcification = 0.0;
        assert!((e.youngs_modulus_gpa() - E_UNSCLEROTIZED_GPA).abs() < 1e-3);
        e.sclerotization = 1.0;
        e.calcification = 0.0;
        assert!((e.youngs_modulus_gpa() - E_SCLEROTIZED_GPA).abs() < 1e-3);
        e.sclerotization = 1.0;
        e.calcification = 1.0;
        assert!((e.youngs_modulus_gpa() - E_CALCIFIED_GPA).abs() < 1e-3);
    }

    #[test]
    fn test_sclerotization_dynamics() {
        let mut e = ChitinExoskeleton::default();
        e.sclerotization = 0.0;
        // 推进 1 小时, 硬化度应增加
        let dt = 3600.0;
        e.step_sclerotization(dt);
        assert!(e.sclerotization > 0.0);
        assert!(e.sclerotization < 1.0);
        // 推进多个 tau, 应接近 1
        for _ in 0..2000 {
            e.step_sclerotization(TAU_SCLEROTIZATION);
        }
        assert!((e.sclerotization - 1.0).abs() < 1e-2);
    }

    #[test]
    fn test_ecdysis_cycle() {
        let mut e = ChitinExoskeleton::default();
        e.sclerotization = 1.0;
        e.calcification = 0.5;
        // 模拟蜕皮激素峰值 (高于阈值触发 molting)
        // 需要推进多个时间步让 20-HE 浓度逐渐达到峰值
        for _ in 0..12 {
            e.step_ecdysone(3600.0, ECDYSONE_PEAK_NM * 1.5);
        }
        assert!(e.molting);
        // 完成蜕皮, 应重置硬化/钙化
        e.complete_ecdysis();
        assert!(e.sclerotization.abs() < 1e-6);
        assert!(e.calcification.abs() < 1e-6);
        assert!(!e.molting);
        // 检查膨胀窗口
        assert!(e.is_inflation_window(1800.0));
        assert!(!e.is_inflation_window(2.0 * POST_ECDYSIS_WINDOW));
    }

    #[test]
    fn test_calcification_rate_ph_dependency() {
        let mut c = CrustaceanCalcification::default();
        // pH 8.1 (海水) - 正常钙化
        let r_high = c.calcification_rate();
        assert!(r_high > 0.0);
        // pH 7.0 (酸化) - 钙化受抑
        c.ph = 7.0;
        let r_low = c.calcification_rate();
        assert!(r_low < r_high);
        // pH < 7.5 触发脱钙
        let dec = c.decalcification_rate();
        assert!(dec > 0.0);
        // pH 8.1 时脱钙速率为 0
        c.ph = 8.1;
        assert!(c.decalcification_rate().abs() < 1e-6);
    }

    #[test]
    fn test_calcification_step() {
        let c = CrustaceanCalcification::default();
        let mut caco3 = vec![0.0_f32; 10];
        let mut acc = vec![0.0_f32; 10];
        c.step(&mut caco3, &mut acc, 24.0);
        // 应有 CaCO3 沉积
        let total: f32 = caco3.iter().sum();
        assert!(total > 0.0, "CaCO3 总沉积量应大于 0");
    }

    #[test]
    fn test_calcification_acid_dissolution() {
        let mut c = CrustaceanCalcification::default();
        c.ph = 6.5; // 强酸化
        let mut caco3 = vec![10.0_f32; 10];
        let mut acc = vec![0.0_f32; 10];
        c.step(&mut caco3, &mut acc, 1.0);
        // 强酸化条件下 CaCO3 应减少 (溶解)
        let total_after: f32 = caco3.iter().sum();
        assert!(total_after < 100.0, "酸化应导致 CaCO3 溶解");
    }

    #[test]
    fn test_mollusk_shell_toughness() {
        let s = MolluskShell::default();
        let k = s.nacre_fracture_toughness();
        // 珍珠层 K_ic 应在 5-8
        assert!(k >= 5.0 && k <= 8.0);
        // 总厚度应 > 0
        assert!(s.total_thickness_um() > 0.0);
    }

    #[test]
    fn test_mollusk_biomineralization() {
        let mut s = MolluskShell::default();
        s.mantle_activity = 1.0;
        let r = s.biomineralization_rate_um_day(10.0);
        // k=0.5, activity=1, ca=10 -> 5 um/day
        assert!((r - 5.0).abs() < 1e-3);
        // 0 活性 -> 0 生长率
        s.mantle_activity = 0.0;
        assert!(s.biomineralization_rate_um_day(10.0).abs() < 1e-6);
    }

    #[test]
    fn test_mollusk_growth_step() {
        let mut s = MolluskShell::default();
        let t0 = s.total_thickness_um();
        s.mantle_activity = 1.0;
        s.step_growth(10.0, 10.0); // 10 天
        let t1 = s.total_thickness_um();
        assert!(t1 > t0, "生长后总厚度应增加");
    }

    #[test]
    fn test_caco3_polymorph_density() {
        // 密度排序: ACC < calcite < aragonite
        let d_acc = Caco3Polymorph::Acc.density_g_cm3();
        let d_calcite = Caco3Polymorph::Calcite.density_g_cm3();
        let d_aragonite = Caco3Polymorph::Aragonite.density_g_cm3();
        assert!(d_acc < d_calcite);
        assert!(d_calcite < d_aragonite);
    }
}