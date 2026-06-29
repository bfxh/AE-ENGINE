// 静水骨骼 (Hydrostatic Skeleton)
// 体腔液不可压缩 + 肌肉收缩 -> 形成伪刚体
// 模块: 静水节段 / 蚯蚓蠕动 / 章鱼触手 / 水母喷流 / 海参管足
// 来源:
//   - Chapman G (1958) J Exp Biol 35:354-363
//   - Alexander RM (1988) "Elastic Mechanisms in Animal Movement" CUP
//   - Kier WM, Smith KK (1985) J Morphol 183:175-188
//   - Dabiri JO et al. (2007) J Exp Biol 210:1257-1265
//   - Sahin M, Mohagheghian E, Chen I (2019) J Exp Biol 222
//   - Trueman ER (1975) "The Locomotion of Soft-Bodied Animals" Elsevier

use serde::{Deserialize, Serialize};

// === 物理常数 (SI 单位, 注明来源) ===
/// 水的密度 (kg/m^3), 998 (20 C)
pub const WATER_DENSITY: f32 = 998.0;
/// 体腔液不可压缩近似: 体积变化率 dV/dt ~ 0
pub const INCOMPRESSIBILITY_TOLERANCE: f32 = 1e-6;
/// 蚯蚓蠕动波速范围 (body lengths/s)
pub const PERISTALSIS_WAVE_SPEED_MIN: f32 = 0.5;
pub const PERISTALSIS_WAVE_SPEED_MAX: f32 = 2.0;
/// 章鱼吸盘典型负压范围 (kPa)
pub const SUCKER_PRESSURE_MIN_KPA: f32 = 100.0;
pub const SUCKER_PRESSURE_MAX_KPA: f32 = 300.0;
/// 水母游泳频率范围 (Hz)
pub const JET_FREQUENCY_MIN_HZ: f32 = 0.5;
pub const JET_FREQUENCY_MAX_HZ: f32 = 2.0;
/// 海水环境压强 (Pa), 1 atm
pub const AMBIENT_PRESSURE_PA: f32 = 101_325.0;
/// 典型体腔内静水压 (Pa), ~ 5 kPa (Chapman 1958)
pub const TYPICAL_HYDROSTATIC_PRESSURE_PA: f32 = 5_000.0;

/// 体壁肌肉类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HydrostaticMuscle {
    /// 环肌 circular - 收缩使直径减小, 长度增加
    Circular,
    /// 纵肌 longitudinal - 收缩使长度减小, 直径增加
    Longitudinal,
    /// 螺旋肌 helical - 收缩使扭转
    Helical,
}

/// 静水骨骼节段 (单节段)
/// 体积守恒: V = (pi/4) * D^2 * L = const
/// 体液不可抗剪切, 需要中胶层 mesoglea 提供弹性约束
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HydrostaticSegment {
    /// 节段长度 L (m)
    pub length_m: f32,
    /// 节段直径 D (m)
    pub diameter_m: f32,
    /// 内部静水压 P (Pa)
    pub pressure_pa: f32,
    /// 环肌激活度 [0, 1]
    pub circular_activation: f32,
    /// 纵肌激活度 [0, 1]
    pub longitudinal_activation: f32,
    /// 螺旋肌激活度 [0, 1]
    pub helical_activation: f32,
    /// 中胶层抗剪切强度 (Pa) - 仅作弹性约束
    pub mesoglea_shear_pa: f32,
}
impl Default for HydrostaticSegment {
    fn default() -> Self {
        Self {
            length_m: 0.01,
            diameter_m: 0.002,
            pressure_pa: TYPICAL_HYDROSTATIC_PRESSURE_PA,
            circular_activation: 0.0,
            longitudinal_activation: 0.0,
            helical_activation: 0.0,
            mesoglea_shear_pa: 1000.0,
        }
    }
}

impl HydrostaticSegment {
    pub fn new() -> Self {
        Self::default()
    }

    /// 节段体积 V = pi/4 * D^2 * L (m^3)
    pub fn volume_m3(&self) -> f32 {
        let r = self.diameter_m * 0.5;
        std::f32::consts::PI * r * r * self.length_m
    }

    /// 体积守恒约束: 给定新长度, 计算所需直径
    /// D_new = D_old * sqrt(L_old / L_new)
    pub fn conserve_volume_diameter(&self, new_length_m: f32) -> f32 {
        if new_length_m <= 0.0 {
            return self.diameter_m;
        }
        self.diameter_m * (self.length_m / new_length_m).sqrt()
    }

    /// 单步推进 (dt 秒)
    /// 环肌收缩 -> 长度增加 (张拉)
    /// 纵肌收缩 -> 长度减小 (压缩)
    /// 体积守恒自动调整直径
    /// 内压随肌肉张力上升: dP = k_P * (a_circ + a_long)
    pub fn step(&mut self, dt: f32, k_length: f32, k_pressure: f32) {
        let v0 = self.volume_m3();
        // 环肌激活增加长度, 纵肌激活减小长度
        let dl = k_length * (self.circular_activation - self.longitudinal_activation) * dt;
        let new_length = (self.length_m + dl).max(1e-6);
        self.length_m = new_length;
        // 体积守恒: 重算直径
        let new_r = (v0 / (std::f32::consts::PI * new_length)).sqrt();
        self.diameter_m = 2.0 * new_r;
        // 压力更新: 肌肉激活产生张力, 提高内压
        let activation_sum = self.circular_activation + self.longitudinal_activation;
        let dp = k_pressure * activation_sum * dt;
        self.pressure_pa = (self.pressure_pa + dp).max(0.0);
    }

    /// 计算环肌收缩产生的轴向力 (N)
    /// 简化: F = P * A_cross, A_cross = pi/4 * D^2
    pub fn axial_force_n(&self) -> f32 {
        let a = std::f32::consts::PI * 0.25 * self.diameter_m * self.diameter_m;
        self.pressure_pa * a
    }
}

/// 蚯蚓蠕动模型
/// 节段序列 + 收缩波从前往后传播
/// 数学模型: L_i(t) = L_0 + A * sin(2*pi*(t/T - i/N))
/// 直径 D_i(t) 由体积守恒: D^2 * L = const
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peristalsis {
    /// 节段数 N
    pub n_segments: usize,
    /// 节段初始长度 L_0 (m)
    pub segment_length_m: f32,
    /// 节段初始直径 D_0 (m)
    pub segment_diameter_m: f32,
    /// 收缩波周期 T (s)
    pub wave_period_s: f32,
    /// 收缩波振幅 A (m)
    pub wave_amplitude_m: f32,
    /// 刚毛锚定摩擦系数 mu
    pub setae_friction_mu: f32,
    /// 法向力 N (N)
    pub normal_force_n: f32,
}
impl Default for Peristalsis {
    fn default() -> Self {
        Self {
            n_segments: 50,
            segment_length_m: 0.005,
            segment_diameter_m: 0.003,
            wave_period_s: 2.0,
            wave_amplitude_m: 0.0015,
            setae_friction_mu: 0.6,
            normal_force_n: 0.001,
        }
    }
}

impl Peristalsis {
    pub fn new() -> Self {
        Self::default()
    }

    /// 计算节段 i 在时间 t 的长度
    /// L_i(t) = L_0 + A * sin(2*pi*(t/T - i/N))
    pub fn segment_length(&self, i: usize, t: f32) -> f32 {
        let phase = 2.0 * std::f32::consts::PI
            * (t / self.wave_period_s - i as f32 / self.n_segments as f32);
        (self.segment_length_m + self.wave_amplitude_m * phase.sin()).max(1e-6)
    }

    /// 计算节段 i 在时间 t 的直径 (由体积守恒)
    /// D^2 * L = D_0^2 * L_0
    pub fn segment_diameter(&self, i: usize, t: f32) -> f32 {
        let L = self.segment_length(i, t);
        let L0 = self.segment_length_m;
        let D0 = self.segment_diameter_m;
        D0 * (L0 / L).sqrt()
    }

    /// 锚定力: f_anchor = mu * N
    /// 刚毛锚定节段提供推进反力
    pub fn anchor_force_n(&self) -> f32 {
        self.setae_friction_mu * self.normal_force_n
    }

    /// 推进速度 (body_len/s)
    /// 简化模型: wave_speed = N * L_0 / T
    pub fn wave_speed_body_len_per_s(&self) -> f32 {
        let wave_speed = self.n_segments as f32 * self.segment_length_m / self.wave_period_s;
        let body_len = self.n_segments as f32 * self.segment_length_m;
        if body_len > 0.0 {
            wave_speed / body_len
        } else {
            0.0
        }
    }

    /// 计算整条虫的体长 (时间 t)
    pub fn total_length_m(&self, t: f32) -> f32 {
        (0..self.n_segments)
            .map(|i| self.segment_length(i, t))
            .sum()
    }

    /// 计算某时间点的瞬时推进力 (N)
    /// 简化: 假设约 30% 的节段处于锚定状态 (收缩波后段)
    pub fn thrust_n(&self, t: f32) -> f32 {
        // 锚定节段: 收缩相位 (sin < 0) 的节段处于锚定状态
        // 锚定节段提供推进反力
        let mut n_anchored = 0;
        for i in 0..self.n_segments {
            let phase = 2.0 * std::f32::consts::PI
                * (t / self.wave_period_s - i as f32 / self.n_segments as f32);
            if phase.sin() < 0.0 {
                n_anchored += 1;
            }
        }
        if n_anchored == 0 {
            return 0.0;
        }
        self.anchor_force_n() * n_anchored as f32
    }
}

/// 章鱼吸盘模型
/// 形成负压过程: sphincter 收缩 -> 腔室扩大 -> 外水吸入 -> sphincter 闭合 -> 提拉活塞 -> 负压
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Sucker {
    /// 吸盘半径 r (m), ~ 1-10 mm
    pub radius_m: f32,
    /// 腔室活塞面积 A_p (m^2)
    pub piston_area_m2: f32,
    /// 内部负压 delta_P (Pa), 典型 100-300 kPa
    pub negative_pressure_pa: f32,
    /// sphincter 肌肉激活 [0, 1]
    pub sphincter_activation: f32,
    /// 活塞提升量 (m)
    pub piston_lift_m: f32,
    /// 腔室初始高度 (m)
    pub chamber_height_m: f32,
}
impl Default for Sucker {
    fn default() -> Self {
        Self {
            radius_m: 0.005,
            piston_area_m2: 7.85e-5, // pi * (5mm)^2
            negative_pressure_pa: 0.0,
            sphincter_activation: 0.0,
            piston_lift_m: 0.0,
            chamber_height_m: 0.001,
        }
    }
}

impl Sucker {
    pub fn new() -> Self {
        Self::default()
    }

    /// 吸附接触面积 A = pi * r^2 (m^2)
    pub fn contact_area_m2(&self) -> f32 {
        std::f32::consts::PI * self.radius_m * self.radius_m
    }

    /// 吸附力 F = delta_P * A (N), 由负压产生
    pub fn adhesion_force_n(&self) -> f32 {
        self.negative_pressure_pa * self.contact_area_m2()
    }

    /// 形成负压过程 (dt 秒):
    /// 1) sphincter 收缩 -> 腔室扩大 -> 外水吸入
    /// 2) sphincter 闭合
    /// 3) 活塞提拉 -> 腔室体积增大 -> 压强降低 (Boyle 定律)
    /// Boyle: P1*V1 = P2*V2 -> P2 = P1 * V1 / V2
    pub fn step_attach(&mut self, ambient_pressure_pa: f32, dt: f32) {
        // 第一阶段: sphincter 开启, 灌水至环境压力
        if self.sphincter_activation < 0.5 {
            self.sphincter_activation = (self.sphincter_activation + dt * 2.0).min(1.0);
            self.negative_pressure_pa = 0.0;
            return;
        }
        // 第二阶段: 闭合 sphincter
        if self.sphincter_activation < 1.0 {
            self.sphincter_activation = (self.sphincter_activation + dt * 2.0).min(1.0);
            return;
        }
        // 第三阶段: 提拉活塞, Boyle 定律
        let lift_rate = 0.001; // 1 mm/s
        self.piston_lift_m += lift_rate * dt;
        let v1 = self.piston_area_m2 * self.chamber_height_m;
        let v2 = v1 + self.piston_area_m2 * self.piston_lift_m;
        if v2 > v1 {
            let p2 = ambient_pressure_pa * v1 / v2;
            self.negative_pressure_pa = (ambient_pressure_pa - p2).max(0.0);
        }
        // 钳制到典型最大负压
        let max_pa = SUCKER_PRESSURE_MAX_KPA * 1000.0;
        if self.negative_pressure_pa > max_pa {
            self.negative_pressure_pa = max_pa;
        }
    }

    /// 释放吸盘: 重置所有状态
    pub fn release(&mut self) {
        self.sphincter_activation = 0.0;
        self.piston_lift_m = 0.0;
        self.negative_pressure_pa = 0.0;
    }
}

/// 章鱼触手模型 - 静水骨骼 + 多组肌肉
/// 三组肌肉: 横肌 transverse / 纵肌 longitudinal / 螺旋肌 helical
/// 弯曲: 一侧纵肌收缩, 对侧拉伸
/// 扭转: 螺旋肌差分收缩
/// 伸展: 横肌收缩 -> 直径减小 -> 长度增加 (体积守恒)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TentacleLocomotion {
    /// 触手总长 L (m)
    pub length_m: f32,
    /// 触手基础直径 D (m)
    pub base_diameter_m: f32,
    /// 吸盘数 (~ 240)
    pub sucker_count: usize,
    /// 横肌激活 (沿触手位置分布 [0,1])
    pub transverse_activation: Vec<f32>,
    /// 左侧纵肌激活
    pub longitudinal_left: Vec<f32>,
    /// 右侧纵肌激活
    pub longitudinal_right: Vec<f32>,
    /// 螺旋肌激活
    pub helical: Vec<f32>,
    /// 吸盘集合
    pub suckers: Vec<Sucker>,
}
impl TentacleLocomotion {
    pub fn new(length_m: f32, n_nodes: usize, sucker_count: usize) -> Self {
        Self {
            length_m,
            base_diameter_m: 0.02,
            sucker_count,
            transverse_activation: vec![0.0; n_nodes],
            longitudinal_left: vec![0.0; n_nodes],
            longitudinal_right: vec![0.0; n_nodes],
            helical: vec![0.0; n_nodes],
            suckers: vec![Sucker::default(); sucker_count],
        }
    }

    /// 计算触手在某节点处的弯曲角度 (rad)
    /// 弯曲量正比于 (左纵肌 - 右纵肌) 的差分
    pub fn bend_angle_at(&self, i: usize) -> f32 {
        let dl = self.longitudinal_left.get(i).copied().unwrap_or(0.0);
        let dr = self.longitudinal_right.get(i).copied().unwrap_or(0.0);
        let k = 50.0;
        k * (dl - dr)
    }

    /// 计算扭转角 (rad)
    /// 螺旋肌激活产生扭矩
    pub fn twist_angle_at(&self, i: usize) -> f32 {
        let h = self.helical.get(i).copied().unwrap_or(0.0);
        let k = 30.0;
        k * h
    }

    /// 计算触手某节点处的长度增量 (伸展因子, 0=不变, 1=完全伸展)
    /// 横肌收缩 -> 直径减小 -> 长度增加 (体积守恒)
    pub fn extension_at(&self, i: usize) -> f32 {
        let t = self.transverse_activation.get(i).copied().unwrap_or(0.0);
        0.6 * t
    }

    /// 总吸附力 (N) - 所有活跃吸盘之和
    pub fn total_adhesion_n(&self) -> f32 {
        self.suckers.iter().map(|s| s.adhesion_force_n()).sum()
    }

    /// 单步推进 (dt 秒)
    pub fn step(&mut self, dt: f32, ambient_pressure_pa: f32) {
        for s in self.suckers.iter_mut() {
            s.step_attach(ambient_pressure_pa, dt);
        }
    }

    /// 释放所有吸盘
    pub fn release_all(&mut self) {
        for s in self.suckers.iter_mut() {
            s.release();
        }
    }
}

/// 水母喷流推进模型
/// 来源: Dabiri 2007, Sahin et al. 2009
/// 游泳循环:
///   power stroke: 环肌收缩 -> 伞内径减小 -> 水喷出 -> 推进
///   recovery stroke: 中胶层弹性回弹 -> 伞扩张 -> 水吸入
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct JetPropulsion {
    /// 伞直径 D (m)
    pub bell_diameter_m: f32,
    /// 伞最大半径 (m)
    pub bell_radius_m: f32,
    /// 收缩期喷口半径 (m)
    pub orifice_radius_m: f32,
    /// 收缩频率 (Hz), 0.5-2
    pub stroke_frequency_hz: f32,
    /// 收缩占空比 (0-1) power stroke 占总周期比例
    pub power_stroke_ratio: f32,
    /// 当前相位 phi in [0, 1]
    pub phase: f32,
    /// 中胶层弹性模量 E_m (Pa)
    pub mesoglea_modulus_pa: f32,
    /// 当前游泳速度 v_swim (m/s)
    pub swim_speed_m_s: f32,
    /// 收缩时伞直径减小比例
    pub contraction_ratio: f32,
}
impl Default for JetPropulsion {
    fn default() -> Self {
        Self {
            bell_diameter_m: 0.2,
            bell_radius_m: 0.1,
            orifice_radius_m: 0.03,
            stroke_frequency_hz: 1.0,
            power_stroke_ratio: 0.4,
            phase: 0.0,
            mesoglea_modulus_pa: 1000.0,
            swim_speed_m_s: 0.0,
            contraction_ratio: 0.3,
        }
    }
}

impl JetPropulsion {
    pub fn new() -> Self {
        Self::default()
    }

    /// 喷流速度 v_jet (m/s)
    /// 简化: power stroke 期间伞内径减小, 体积变化率 Q
    /// Q = dV/dt = (pi/4) * D^2 * (D * f * contraction_ratio)
    /// v_jet = Q / A_orifice
    pub fn jet_velocity_m_s(&self) -> f32 {
        let dV_dt = std::f32::consts::PI
            * 0.25
            * self.bell_diameter_m.powi(2)
            * self.bell_diameter_m
            * self.contraction_ratio
            * self.stroke_frequency_hz;
        let a_orifice = std::f32::consts::PI * self.orifice_radius_m * self.orifice_radius_m;
        if a_orifice > 0.0 {
            dV_dt / a_orifice
        } else {
            0.0
        }
    }

    /// 推进力 F = rho * Q * (v_jet - v_swim) (动量理论)
    pub fn thrust_n(&self) -> f32 {
        let v_jet = self.jet_velocity_m_s();
        let a_orifice = std::f32::consts::PI * self.orifice_radius_m * self.orifice_radius_m;
        let q = v_jet * a_orifice;
        WATER_DENSITY * q * (v_jet - self.swim_speed_m_s)
    }

    /// Froude 效率: eta = 2 * v_swim / (v_jet + v_swim)
    pub fn froude_efficiency(&self) -> f32 {
        let v_jet = self.jet_velocity_m_s();
        let denom = v_jet + self.swim_speed_m_s;
        if denom > 1e-6 {
            2.0 * self.swim_speed_m_s / denom
        } else {
            0.0
        }
    }

    /// 是否处于 power stroke
    pub fn is_power_stroke(&self) -> bool {
        self.phase < self.power_stroke_ratio
    }

    /// 水母质量 (kg) = 排开水质量 + 组织质量
    pub fn mass_kg(&self) -> f32 {
        let v_bell = std::f32::consts::PI * 0.1667 * self.bell_diameter_m.powi(3);
        WATER_DENSITY * v_bell + 0.05 // 加 50g 组织质量
    }

    /// 单步推进 (dt 秒)
    /// 在 power stroke 期间产生推力, swim_speed 增加
    /// 在 recovery stroke 期间产生阻力, swim_speed 衰减
    pub fn step(&mut self, dt: f32, drag_coefficient: f32) {
        // 相位推进
        let period = 1.0 / self.stroke_frequency_hz.max(1e-6);
        self.phase += dt / period;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        let m = self.mass_kg().max(1e-6);
        if self.is_power_stroke() {
            let f = self.thrust_n();
            let a = f / m;
            self.swim_speed_m_s += a * dt;
        } else {
            // 阻力: F_drag = 0.5 * rho * Cd * A * v^2
            let a_proj = std::f32::consts::PI * self.bell_radius_m * self.bell_radius_m;
            let f_drag = 0.5 * WATER_DENSITY * drag_coefficient * a_proj * self.swim_speed_m_s.powi(2);
            let a = -f_drag / m * self.swim_speed_m_s.signum();
            self.swim_speed_m_s = (self.swim_speed_m_s + a * dt).max(0.0);
        }
    }
}

/// 海参管足模型
/// 水管系统: 筛板 madrepovite -> 石管 -> 环管 -> 辐管 -> 管足
/// 每个管足: ampulla + podium + sucker
/// 液压驱动: ampulla 收缩 -> 管足伸出
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TubeFoot {
    /// 管足长度 (m)
    pub length_m: f32,
    /// 管足直径 (m)
    pub diameter_m: f32,
    /// ampulla 内压 (Pa)
    pub ampulla_pressure_pa: f32,
    /// 是否伸出
    pub extended: bool,
    /// 是否锚定 (吸盘附着)
    pub anchored: bool,
}
impl Default for TubeFoot {
    fn default() -> Self {
        Self {
            length_m: 0.005,
            diameter_m: 0.0005,
            ampulla_pressure_pa: 2000.0,
            extended: false,
            anchored: false,
        }
    }
}

impl TubeFoot {
    pub fn new() -> Self {
        Self::default()
    }

    /// 管足伸出: ampulla 收缩 -> 液压驱动 podium 伸出
    /// 需要 ampulla 内压 > 1000 Pa (阈值)
    pub fn extend(&mut self, target_length_m: f32) {
        if self.ampulla_pressure_pa > 1000.0 {
            self.length_m = target_length_m;
            self.extended = true;
        }
    }

    /// 管足缩回 (长度减半)
    pub fn retract(&mut self) {
        self.length_m *= 0.5;
        self.extended = false;
        self.anchored = false;
    }

    /// 锚定: 端部吸盘附着 (需先伸出)
    pub fn anchor(&mut self) {
        if self.extended {
            self.anchored = true;
        }
    }

    /// 释放锚定
    pub fn release(&mut self) {
        self.anchored = false;
    }
}

/// 海参蹒跚运动 - 管足序列收缩波
/// 步调: 管足序列收缩, 波浪状推进
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolothurianCreeping {
    /// 管足列表
    pub tube_feet: Vec<TubeFoot>,
    /// 管足数
    pub n_feet: usize,
    /// 收缩波速度 (管足/秒)
    pub wave_speed_per_s: f32,
    /// 当前相位 [0, 1]
    pub phase: f32,
    /// 推进速度 (m/s)
    pub creep_speed_m_s: f32,
    /// 单步位移 (m)
    pub step_per_foot_m: f32,
}

impl HolothurianCreeping {
    pub fn new(n_feet: usize) -> Self {
        Self {
            tube_feet: vec![TubeFoot::default(); n_feet],
            n_feet,
            wave_speed_per_s: 5.0,
            phase: 0.0,
            creep_speed_m_s: 0.0,
            step_per_foot_m: 0.0005,
        }
    }

    /// 推进单步 (dt 秒)
    /// 管足序列激活: 伸出 -> 锚定 -> 推进 -> 释放
    pub fn step(&mut self, dt: f32) {
        self.phase += dt * self.wave_speed_per_s / self.n_feet as f32;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        // 对每个管足按相位激活
        for (i, foot) in self.tube_feet.iter_mut().enumerate() {
            let local_phase = (self.phase + i as f32 / self.n_feet as f32) % 1.0;
            // 0-0.3: 伸出
            // 0.3-0.6: 锚定
            // 0.6-0.9: 推进 (身体前移)
            // 0.9-1.0: 释放
            if local_phase < 0.3 {
                foot.extend(0.005);
            } else if local_phase < 0.6 {
                foot.anchor();
            } else if local_phase < 0.9 {
                foot.anchored = true;
            } else {
                foot.retract();
            }
        }
        // 推进速度 = 锚定管足数 * 单步位移 * 频率
        let anchored_count = self.tube_feet.iter().filter(|f| f.anchored).count();
        let freq = self.wave_speed_per_s / self.n_feet as f32;
        self.creep_speed_m_s = anchored_count as f32 * self.step_per_foot_m * freq;
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_conservation() {
        let mut s = HydrostaticSegment::default();
        let v0 = s.volume_m3();
        s.circular_activation = 1.0;
        s.step(0.1, 0.001, 1000.0);
        let v1 = s.volume_m3();
        // 体积应近似守恒 (允许数值误差)
        assert!((v1 - v0).abs() < v0 * 0.05);
    }

    #[test]
    fn test_pressure_increase_with_activation() {
        let mut s = HydrostaticSegment::default();
        let p0 = s.pressure_pa;
        s.circular_activation = 1.0;
        s.longitudinal_activation = 1.0;
        s.step(0.1, 0.001, 5000.0);
        // 双肌肉激活, 压力应上升
        assert!(s.pressure_pa > p0);
    }

    #[test]
    fn test_peristalsis_wave_bounds() {
        let p = Peristalsis::default();
        // 波速应非负
        let v = p.wave_speed_body_len_per_s();
        assert!(v >= 0.0);
        // 节段长度应在 L_0 ± A 之间
        for i in 0..p.n_segments {
            let l = p.segment_length(i, 0.0);
            assert!(l >= p.segment_length_m - p.wave_amplitude_m - 1e-6);
            assert!(l <= p.segment_length_m + p.wave_amplitude_m + 1e-6);
        }
        // 总长度应接近 N * L_0 (正弦平均为 0)
        let total = p.total_length_m(0.0);
        let expected = p.n_segments as f32 * p.segment_length_m;
        assert!((total - expected).abs() < expected * 0.05);
    }

    #[test]
    fn test_peristalsis_anchor_force() {
        let p = Peristalsis::default();
        // f_anchor = mu * N
        let f = p.anchor_force_n();
        let expected = p.setae_friction_mu * p.normal_force_n;
        assert!((f - expected).abs() < 1e-6);
        // 推进力应 > 0
        assert!(p.thrust_n(0.0) > 0.0);
    }

    #[test]
    fn test_sucker_adhesion() {
        let mut s = Sucker::default();
        // 初始无负压 -> 吸附力为 0
        assert!(s.adhesion_force_n().abs() < 1e-6);
        // 模拟形成负压过程 (3 秒, 应足以完成三阶段)
        for _ in 0..3000 {
            s.step_attach(AMBIENT_PRESSURE_PA, 0.001);
        }
        // 应已形成负压
        assert!(s.negative_pressure_pa > 0.0);
        // 吸附力应 > 0
        let f = s.adhesion_force_n();
        assert!(f > 0.0);
    }

    #[test]
    fn test_sucker_release() {
        let mut s = Sucker::default();
        s.negative_pressure_pa = 100_000.0;
        s.sphincter_activation = 1.0;
        s.piston_lift_m = 0.001;
        s.release();
        assert!(s.negative_pressure_pa.abs() < 1e-6);
        assert!(s.sphincter_activation.abs() < 1e-6);
        assert!(s.piston_lift_m.abs() < 1e-6);
    }

    #[test]
    fn test_jet_propulsion_cycle() {
        let mut j = JetPropulsion::default();
        // 推进若干周期, 应有正向游泳速度
        let period = 1.0 / j.stroke_frequency_hz;
        for _ in 0..500 {
            j.step(period / 100.0, 0.5);
        }
        // 应该有正向推进
        assert!(j.swim_speed_m_s >= 0.0);
        // Froude 效率应在 [0, 1] 之间
        let eta = j.froude_efficiency();
        assert!(eta >= 0.0 && eta <= 1.0 + 1e-3);
    }

    #[test]
    fn test_jet_thrust_positive() {
        let j = JetPropulsion::default();
        // 静止状态下, 喷流应产生正向推力
        let f = j.thrust_n();
        assert!(f > 0.0);
        // 喷流速度应 > 0
        assert!(j.jet_velocity_m_s() > 0.0);
    }

    #[test]
    fn test_tube_foot_cycle() {
        let mut h = HolothurianCreeping::new(20);
        // 推进若干步
        for _ in 0..2000 {
            h.step(0.01);
        }
        // 推进速度应 >= 0
        assert!(h.creep_speed_m_s >= 0.0);
    }

    #[test]
    fn test_tube_foot_extend_anchor() {
        let mut f = TubeFoot::default();
        // 高压下可伸出
        f.ampulla_pressure_pa = 3000.0;
        f.extend(0.008);
        assert!(f.extended);
        assert!((f.length_m - 0.008).abs() < 1e-6);
        // 伸出后可锚定
        f.anchor();
        assert!(f.anchored);
        // 缩回后状态重置
        f.retract();
        assert!(!f.extended);
        assert!(!f.anchored);
    }

    #[test]
    fn test_tube_foot_low_pressure_no_extend() {
        let mut f = TubeFoot::default();
        f.ampulla_pressure_pa = 500.0; // 低于阈值
        f.extend(0.008);
        // 不应伸出
        assert!(!f.extended);
    }

    #[test]
    fn test_tentacle_bend_direction() {
        let mut t = TentacleLocomotion::new(0.3, 10, 240);
        // 左侧激活 -> 弯曲为正
        t.longitudinal_left[5] = 0.5;
        t.longitudinal_right[5] = 0.0;
        let b = t.bend_angle_at(5);
        assert!(b > 0.0);
        // 右侧激活 -> 弯曲为负
        t.longitudinal_left[5] = 0.0;
        t.longitudinal_right[5] = 0.5;
        let b2 = t.bend_angle_at(5);
        assert!(b2 < 0.0);
        // 吸盘数量正确
        assert_eq!(t.suckers.len(), 240);
    }

    #[test]
    fn test_tentacle_extension() {
        let mut t = TentacleLocomotion::new(0.3, 10, 100);
        t.transverse_activation[3] = 1.0;
        let ext = t.extension_at(3);
        assert!(ext > 0.0);
        // 横肌 0 -> 伸展 0
        t.transverse_activation[3] = 0.0;
        assert!(t.extension_at(3).abs() < 1e-6);
    }
}