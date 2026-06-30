//! Phase Change Solver — 相变求解器 (熔化/凝固, Stefan 问题)
//!
//! 基于:
//! - Stefan, J. 1891. "Über die Theorie der Eisbildung." Annalen der Physik.
//! - Voller, V.R., Swaminathan, C.R. 1990. "General Source-Based Method for
//!   Solidification Phase Change." Numerical Heat Transfer B.
//! - Alexiades, V., Solomon, A.D. 1993. "Mathematical Modeling of Melting
//!   and Freezing." Hemisphere.
//! - Nedjar, B. 2018. "An enthalpy-based finite element method for heat
//!   conduction with phase change." IJNME.
//!
//! 核心思想 (焓法 / Enthalpy Method):
//! 1. 定义焓 H(T) = integral(c(T') dT') + L * f_l(T)
//!    - T < T_s: H = rho*c_s*(T - T_ref)         (固相)
//!    - T_s <= T <= T_l: H = rho*c_s*(T_s-T_ref) + rho*L*(T-T_s)/(T_l-T_s) (糊状区)
//!    - T > T_l: H = rho*c_s*(T_s-T_ref) + rho*L + rho*c_l*(T-T_l) (液相)
//! 2. 能量方程: dH/dt = div(k * grad(T)) + Q
//! 3. 显热容法 (Apparent Heat Capacity):
//!    rho * c_eff * dT/dt = div(k * grad(T)) + Q
//!    c_eff = c + L / (T_l - T_s)  (在糊状区, 包含潜热贡献)
//!    在糊状区外, c_eff = c (固相或液相比热)
//! 4. 液相分数 f_l:
//!    T < T_s: f_l = 0
//!    T_s <= T <= T_l: f_l = (T - T_s) / (T_l - T_s)
//!    T > T_l: f_l = 1
//! 5. 有效热导率: k_eff = (1-f_l)*k_s + f_l*k_l (线性混合)
//!
//! 优势:
//! - 无需显式追踪界面 (固定网格法)
//! - 处理糊状区 (合金, 非纯物质)
//! - 与标准热扩散求解器兼容
//! - 无条件稳定 (隐式 Euler)
//!
//! 应用:
//! - 冰融化/水结冰
//! - 金属铸造 (液态 -> 固态)
//! - 烹饪 (蛋白质变性, 脂肪熔化)
//! - 火山岩浆冷却
//! - 蜡熔化
//! - 相变储能材料 (PCM)

use crate::heat_diffusion::{BoundaryType, idx};
use serde::{Deserialize, Serialize};

// ============================================================
// 相变材料
// ============================================================

/// 相变材料 (Phase Change Material, PCM)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PhaseChangeMaterial {
    /// 固相热属性
    pub solid: crate::heat_diffusion::ThermalMaterial,
    /// 液相热属性
    pub liquid: crate::heat_diffusion::ThermalMaterial,
    /// 熔化温度 T_m (K, 纯物质) 或糊状区中点
    pub melting_temperature: f32,
    /// 糊状区半宽 (K, 纯物质用小值如 0.5)
    pub transition_half_width: f32,
    /// 潜热 L (J/kg)
    pub latent_heat: f32,
}

impl PhaseChangeMaterial {
    /// 固相温度 T_s = T_m - delta_T
    #[inline]
    pub fn t_solid(&self) -> f32 {
        self.melting_temperature - self.transition_half_width
    }

    /// 液相温度 T_l = T_m + delta_T
    #[inline]
    pub fn t_liquid(&self) -> f32 {
        self.melting_temperature + self.transition_half_width
    }

    /// 液相分数 f_l(T) in [0, 1]
    ///
    /// T < T_s: 0 (完全固态)
    /// T_s <= T <= T_l: (T - T_s) / (T_l - T_s) (糊状区)
    /// T > T_l: 1 (完全液态)
    pub fn liquid_fraction(&self, t: f32) -> f32 {
        let ts = self.t_solid();
        let tl = self.t_liquid();
        if t <= ts {
            0.0
        } else if t >= tl {
            1.0
        } else {
            let range = tl - ts;
            if range < 1e-12 {
                if t >= self.melting_temperature { 1.0 } else { 0.0 }
            } else {
                (t - ts) / range
            }
        }
    }

    /// 有效比热容 c_eff(T) (J/kg/K)
    ///
    /// 糊状区: c_eff = c_mix + L / (T_l - T_s)  (潜热贡献)
    /// 固相: c_eff = c_s
    /// 液相: c_eff = c_l
    pub fn effective_specific_heat(&self, t: f32) -> f32 {
        let ts = self.t_solid();
        let tl = self.t_liquid();
        let range = tl - ts;
        if t < ts {
            self.solid.specific_heat
        } else if t > tl {
            self.liquid.specific_heat
        } else if range < 1e-12 {
            self.liquid.specific_heat + self.latent_heat
        } else {
            // 混合比热 + 潜热密度
            let f_l = (t - ts) / range;
            let c_mix = (1.0 - f_l) * self.solid.specific_heat + f_l * self.liquid.specific_heat;
            c_mix + self.latent_heat / range
        }
    }

    /// 有效热导率 k_eff(f_l) (W/m/K)
    ///
    /// 线性混合: k_eff = (1-f_l)*k_s + f_l*k_l
    pub fn effective_conductivity(&self, f_l: f32) -> f32 {
        (1.0 - f_l) * self.solid.conductivity + f_l * self.liquid.conductivity
    }

    /// 有效密度 (假设固液密度相近, 取混合)
    pub fn effective_density(&self, f_l: f32) -> f32 {
        (1.0 - f_l) * self.solid.density + f_l * self.liquid.density
    }

    /// 有效热扩散率 alpha_eff = k_eff / (rho * c_eff)
    pub fn effective_diffusivity(&self, t: f32, f_l: f32) -> f32 {
        let k = self.effective_conductivity(f_l);
        let rho = self.effective_density(f_l);
        let c = self.effective_specific_heat(t);
        let denom = rho * c;
        if denom < 1e-12 {
            0.0
        } else {
            k / denom
        }
    }

    /// 焓 H(T) (J/m^3, 相对 T_ref = T_s)
    pub fn enthalpy(&self, t: f32) -> f32 {
        let ts = self.t_solid();
        let tl = self.t_liquid();
        let f_l = self.liquid_fraction(t);
        let rho = self.effective_density(f_l);

        if t <= ts {
            // 固相: H = rho_s * c_s * (T - T_s)
            self.solid.density * self.solid.specific_heat * (t - ts)
        } else if t >= tl {
            // 液相: H = rho_s*c_s*(T_s-T_s) + rho*L + rho_l*c_l*(T-T_l)
            // = rho*L + rho_l*c_l*(T-T_l)
            self.solid.density * self.solid.specific_heat * 0.0
                + self.solid.density * self.latent_heat
                + self.liquid.density * self.liquid.specific_heat * (t - tl)
        } else {
            // 糊状区: H = rho_s*c_s*(T_s-T_s) + rho*L*f_l
            // = rho*L*f_l (相对 T_s)
            // 更准确: 积分 c_eff 从 T_s 到 T
            let range = tl - ts;
            let c_mix = (1.0 - f_l) * self.solid.specific_heat + f_l * self.liquid.specific_heat;
            // 近似: H = rho * (c_mix_avg * (T - T_s) + L * f_l)
            // c_mix 变化不大, 取平均
            rho * (c_mix * (t - ts) + self.latent_heat * f_l)
        }
    }

    /// 预设: 水/冰 (T_m = 273.15 K, L = 334 kJ/kg)
    pub fn water_ice() -> Self {
        Self {
            solid: crate::heat_diffusion::ThermalMaterial {
                conductivity: 2.18,  // 冰: k ~= 2.18 W/m/K
                density: 917.0,      // 冰: rho ~= 917 kg/m^3
                specific_heat: 2090.0, // 冰: c ~= 2090 J/kg/K
            },
            liquid: crate::heat_diffusion::ThermalMaterial::water(),
            melting_temperature: 273.15,
            transition_half_width: 0.5,
            latent_heat: 334_000.0,
        }
    }

    /// 预设: 铁/铁液 (T_m = 1811 K, L = 247 kJ/kg)
    pub fn iron() -> Self {
        Self {
            solid: crate::heat_diffusion::ThermalMaterial::iron(),
            liquid: crate::heat_diffusion::ThermalMaterial {
                conductivity: 35.0,    // 液铁: k ~= 35
                density: 7020.0,       // 液铁: rho ~= 7020
                specific_heat: 820.0,  // 液铁: c ~= 820
            },
            melting_temperature: 1811.0,
            transition_half_width: 2.0,
            latent_heat: 247_000.0,
        }
    }

    /// 预设: 铝 (T_m = 933 K, L = 397 kJ/kg)
    pub fn aluminum() -> Self {
        Self {
            solid: crate::heat_diffusion::ThermalMaterial::aluminum(),
            liquid: crate::heat_diffusion::ThermalMaterial {
                conductivity: 95.0,
                density: 2350.0,
                specific_heat: 1180.0,
            },
            melting_temperature: 933.0,
            transition_half_width: 1.0,
            latent_heat: 397_000.0,
        }
    }

    /// 预设: 石蜡 (PCM, T_m = 330 K, L = 200 kJ/kg)
    pub fn paraffin() -> Self {
        Self {
            solid: crate::heat_diffusion::ThermalMaterial {
                conductivity: 0.24,
                density: 900.0,
                specific_heat: 2900.0,
            },
            liquid: crate::heat_diffusion::ThermalMaterial {
                conductivity: 0.22,
                density: 780.0,
                specific_heat: 3140.0,
            },
            melting_temperature: 330.0,
            transition_half_width: 2.0,
            latent_heat: 200_000.0,
        }
    }
}

impl Default for PhaseChangeMaterial {
    fn default() -> Self {
        Self::water_ice()
    }
}

// ============================================================
// 求解器配置
// ============================================================

/// 相变求解器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseChangeConfig {
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    pub h: f32,
    pub default_material: PhaseChangeMaterial,
    pub boundaries: [BoundaryType; 6],
    pub max_iterations: usize,
    pub tolerance: f32,
    pub ambient_temperature: f32,
}

impl Default for PhaseChangeConfig {
    fn default() -> Self {
        Self {
            nx: 16,
            ny: 16,
            nz: 16,
            h: 0.01,
            default_material: PhaseChangeMaterial::water_ice(),
            boundaries: [
                BoundaryType::Insulated,
                BoundaryType::Insulated,
                BoundaryType::Insulated,
                BoundaryType::Insulated,
                BoundaryType::Insulated,
                BoundaryType::Insulated,
            ],
            max_iterations: 80,
            tolerance: 1e-4,
            ambient_temperature: 300.0,
        }
    }
}

// ============================================================
// 相变求解器
// ============================================================

/// 相变求解器 (焓法 / Enthalpy Method)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseChangeSolver {
    pub config: PhaseChangeConfig,
    /// 温度场 (K)
    pub temperature: Vec<f32>,
    /// 液相分数场 [0, 1]
    pub liquid_fraction: Vec<f32>,
    /// 体积热源 (W/m^3)
    pub heat_source: Vec<f32>,
    pub material_id: Vec<usize>,
    pub materials: Vec<PhaseChangeMaterial>,
    pub fixed_mask: Vec<bool>,
    pub fixed_value: Vec<f32>,
    /// 模拟时间 (s)
    pub time: f32,
}

impl PhaseChangeSolver {
    /// 创建求解器
    pub fn new(config: PhaseChangeConfig) -> Self {
        let n = config.nx * config.ny * config.nz;
        let amb = config.ambient_temperature;
        let default_mat = config.default_material;
        Self {
            config,
            temperature: vec![amb; n],
            liquid_fraction: vec![0.0; n],
            heat_source: vec![0.0; n],
            material_id: vec![0; n],
            materials: vec![default_mat],
            fixed_mask: vec![false; n],
            fixed_value: vec![amb; n],
            time: 0.0,
        }
    }

    #[inline]
    pub fn num_cells(&self) -> usize {
        self.config.nx * self.config.ny * self.config.nz
    }

    #[inline]
    pub fn idx(&self, i: usize, j: usize, k: usize) -> usize {
        idx(i, j, k, self.config.nx, self.config.ny)
    }

    /// 获取温度 (带边界处理)
    pub fn temperature_at(&self, i: isize, j: isize, k: isize) -> f32 {
        let nx = self.config.nx as isize;
        let ny = self.config.ny as isize;
        let nz = self.config.nz as isize;
        if i < 0 || i >= nx || j < 0 || j >= ny || k < 0 || k >= nz {
            return self.boundary_temperature(i, j, k);
        }
        self.temperature[self.idx(i as usize, j as usize, k as usize)]
    }

    fn boundary_temperature(&self, i: isize, j: isize, k: isize) -> f32 {
        let nx = self.config.nx as isize;
        let ny = self.config.ny as isize;
        let nz = self.config.nz as isize;
        let ic = i.clamp(0, nx - 1) as usize;
        let jc = j.clamp(0, ny - 1) as usize;
        let kc = k.clamp(0, nz - 1) as usize;
        let inner = self.temperature[self.idx(ic, jc, kc)];
        let face = if i < 0 { Some(0) }
            else if i >= nx { Some(1) }
            else if j < 0 { Some(2) }
            else if j >= ny { Some(3) }
            else if k < 0 { Some(4) }
            else if k >= nz { Some(5) }
            else { None };
        match face {
            Some(f) => match self.config.boundaries[f] {
                BoundaryType::Insulated => inner,
                BoundaryType::Fixed(t) => t,
                BoundaryType::Convective { h, t_inf } => {
                    let mat = self.materials[self.material_id[self.idx(ic, jc, kc)]];
                    let f_l = self.liquid_fraction[self.idx(ic, jc, kc)];
                    let k_cond = mat.effective_conductivity(f_l);
                    if k_cond < 1e-12 {
                        inner
                    } else {
                        inner - (self.config.h * h / k_cond) * (inner - t_inf)
                    }
                }
            },
            None => inner,
        }
    }

    /// 更新液相分数 (基于当前温度)
    pub fn update_liquid_fractions(&mut self) {
        for c in 0..self.num_cells() {
            let mat = self.materials[self.material_id[c]];
            self.liquid_fraction[c] = mat.liquid_fraction(self.temperature[c]);
        }
    }

    /// 设置温度并同步液相分数
    pub fn set_temperature(&mut self, i: usize, j: usize, k: usize, t: f32) {
        let idx = self.idx(i, j, k);
        self.temperature[idx] = t;
        let mat = self.materials[self.material_id[idx]];
        self.liquid_fraction[idx] = mat.liquid_fraction(t);
    }

    pub fn set_temperature_uniform(&mut self, t: f32) {
        for c in 0..self.num_cells() {
            self.temperature[c] = t;
            let mat = self.materials[self.material_id[c]];
            self.liquid_fraction[c] = mat.liquid_fraction(t);
        }
    }

    pub fn set_fixed(&mut self, i: usize, j: usize, k: usize, t: f32) {
        let idx = self.idx(i, j, k);
        self.fixed_mask[idx] = true;
        self.fixed_value[idx] = t;
        self.temperature[idx] = t;
        let mat = self.materials[self.material_id[idx]];
        self.liquid_fraction[idx] = mat.liquid_fraction(t);
    }

    pub fn clear_fixed(&mut self, i: usize, j: usize, k: usize) {
        let idx = self.idx(i, j, k);
        self.fixed_mask[idx] = false;
    }

    pub fn add_heat_source(&mut self, i: usize, j: usize, k: usize, q: f32) {
        let idx = self.idx(i, j, k);
        self.heat_source[idx] += q;
    }

    pub fn set_material(&mut self, i: usize, j: usize, k: usize, mat_id: usize) {
        let idx = self.idx(i, j, k);
        self.material_id[idx] = mat_id;
    }

    pub fn add_material(&mut self, mat: PhaseChangeMaterial) -> usize {
        self.materials.push(mat);
        self.materials.len() - 1
    }

    /// 计算单元 (i,j,k) 的拉普拉斯算子
    pub fn laplacian(&self, i: usize, j: usize, k: usize) -> f32 {
        let h = self.config.h;
        let h2 = h * h;
        let ii = i as isize;
        let jj = j as isize;
        let kk = k as isize;
        let t_c = self.temperature_at(ii, jj, kk);
        let t_ip = self.temperature_at(ii + 1, jj, kk);
        let t_im = self.temperature_at(ii - 1, jj, kk);
        let t_jp = self.temperature_at(ii, jj + 1, kk);
        let t_jm = self.temperature_at(ii, jj - 1, kk);
        let t_kp = self.temperature_at(ii, jj, kk + 1);
        let t_km = self.temperature_at(ii, jj, kk - 1);
        (t_ip + t_im + t_jp + t_jm + t_kp + t_km - 6.0 * t_c) / h2
    }

    /// 单步隐式 Euler (焓法 + 显热容)
    ///
    /// rho * c_eff * dT/dt = div(k_eff * grad(T)) + Q
    /// 离散: (1 + 6*r) * T_c = T_old + r * sum(T_neighbors) + dt * Q / (rho * c_eff)
    /// 其中 r = alpha_eff * dt / h^2 = k_eff * dt / (rho * c_eff * h^2)
    ///
    /// 注意: c_eff 依赖 T (非线性), 用上一次迭代的 T 计算 c_eff (Picard 迭代)
    pub fn step(&mut self, dt: f32) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let nz = self.config.nz;
        let h = self.config.h;
        let h2 = h * h;

        let t_old = self.temperature.clone();
        let f_l_old = self.liquid_fraction.clone();

        // 先更新液相分数 (基于 T_old)
        self.update_liquid_fractions();

        // 预计算每单元的 r 和 source
        let n = self.num_cells();
        let mut r = vec![0.0_f32; n];
        let mut source = vec![0.0_f32; n];
        for k_ in 0..nz {
            for j_ in 0..ny {
                for i_ in 0..nx {
                    let idx = self.idx(i_, j_, k_);
                    if self.fixed_mask[idx] {
                        continue;
                    }
                    let mat = self.materials[self.material_id[idx]];
                    let t_cur = self.temperature[idx];
                    let f_l = self.liquid_fraction[idx];
                    let k_eff = mat.effective_conductivity(f_l);
                    let rho = mat.effective_density(f_l);
                    let c_eff = mat.effective_specific_heat(t_cur);
                    let rho_c = rho * c_eff;
                    let alpha_eff = if rho_c > 1e-12 { k_eff / rho_c } else { 0.0 };
                    r[idx] = alpha_eff * dt / h2;
                    source[idx] = t_old[idx] + if rho_c > 1e-12 {
                        dt * self.heat_source[idx] / rho_c
                    } else {
                        0.0
                    };
                }
            }
        }

        // Gauss-Seidel 迭代
        for _it in 0..self.config.max_iterations {
            let mut max_diff = 0.0_f32;
            for k_ in 0..nz {
                for j_ in 0..ny {
                    for i_ in 0..nx {
                        let idx = self.idx(i_, j_, k_);
                        if self.fixed_mask[idx] {
                            continue;
                        }
                        let ri = r[idx];
                        let rhs = source[idx];
                        let t_ip = self.temperature_at(i_ as isize + 1, j_ as isize, k_ as isize);
                        let t_im = self.temperature_at(i_ as isize - 1, j_ as isize, k_ as isize);
                        let t_jp = self.temperature_at(i_ as isize, j_ as isize + 1, k_ as isize);
                        let t_jm = self.temperature_at(i_ as isize, j_ as isize - 1, k_ as isize);
                        let t_kp = self.temperature_at(i_ as isize, j_ as isize, k_ as isize + 1);
                        let t_km = self.temperature_at(i_ as isize, j_ as isize, k_ as isize - 1);
                        let sum_neighbors = t_ip + t_im + t_jp + t_jm + t_kp + t_km;
                        let denom = 1.0 + 6.0 * ri;
                        let t_new = if denom > 1e-12 {
                            (rhs + ri * sum_neighbors) / denom
                        } else {
                            rhs
                        };
                        let diff = (t_new - self.temperature[idx]).abs();
                        if diff > max_diff {
                            max_diff = diff;
                        }
                        self.temperature[idx] = t_new;
                    }
                }
            }
            if max_diff < self.config.tolerance {
                break;
            }
        }

        // 步进后更新液相分数
        self.update_liquid_fractions();
        self.time += dt;

        // 保留 f_l_old 用于诊断 (避免 unused warning)
        let _ = f_l_old;
    }

    /// 总液相分数 (熔化进度 0~1)
    pub fn total_liquid_fraction(&self) -> f32 {
        if self.liquid_fraction.is_empty() {
            return 0.0;
        }
        self.liquid_fraction.iter().sum::<f32>() / self.liquid_fraction.len() as f32
    }

    /// 总焓 (J, 相对 T_ref = 0 K)
    pub fn total_enthalpy(&self) -> f32 {
        let h = self.config.h;
        let vol = h * h * h;
        let mut e = 0.0;
        for c in 0..self.num_cells() {
            let mat = self.materials[self.material_id[c]];
            e += mat.enthalpy(self.temperature[c]) * vol;
        }
        e
    }

    pub fn average_temperature(&self) -> f32 {
        if self.temperature.is_empty() {
            return 0.0;
        }
        self.temperature.iter().sum::<f32>() / self.temperature.len() as f32
    }

    pub fn max_temperature(&self) -> f32 {
        self.temperature.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
    }

    pub fn min_temperature(&self) -> f32 {
        self.temperature.iter().cloned().fold(f32::INFINITY, f32::min)
    }

    pub fn clear_heat_sources(&mut self) {
        for q in &mut self.heat_source {
            *q = 0.0;
        }
    }

    pub fn reset(&mut self) {
        let amb = self.config.ambient_temperature;
        for c in 0..self.num_cells() {
            self.temperature[c] = amb;
            let mat = self.materials[self.material_id[c]];
            self.liquid_fraction[c] = mat.liquid_fraction(amb);
            self.heat_source[c] = 0.0;
        }
        self.time = 0.0;
    }

    /// 已熔化单元数
    pub fn num_liquid_cells(&self) -> usize {
        self.liquid_fraction.iter().filter(|&&f| f > 0.99).count()
    }

    /// 已凝固单元数
    pub fn num_solid_cells(&self) -> usize {
        self.liquid_fraction.iter().filter(|&&f| f < 0.01).count()
    }

    /// 糊状区单元数
    pub fn num_mushy_cells(&self) -> usize {
        self.liquid_fraction.iter().filter(|&&f| f >= 0.01 && f <= 0.99).count()
    }
}


// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() < tol
    }

    fn make_small_solver() -> PhaseChangeSolver {
        let cfg = PhaseChangeConfig {
            nx: 4,
            ny: 4,
            nz: 4,
            h: 0.01,
            ..Default::default()
        };
        PhaseChangeSolver::new(cfg)
    }

    #[test]
    fn test_water_ice_preset() {
        let pcm = PhaseChangeMaterial::water_ice();
        assert!(approx(pcm.melting_temperature, 273.15, 1e-3));
        assert!(approx(pcm.latent_heat, 334_000.0, 1.0));
        // 冰的密度 < 水的密度
        assert!(pcm.solid.density < pcm.liquid.density);
        // 冰的热导率 > 水
        assert!(pcm.solid.conductivity > pcm.liquid.conductivity);
    }

    #[test]
    fn test_iron_preset() {
        let pcm = PhaseChangeMaterial::iron();
        assert!(approx(pcm.melting_temperature, 1811.0, 1e-3));
        assert!(pcm.latent_heat > 200_000.0);
    }

    #[test]
    fn test_aluminum_preset() {
        let pcm = PhaseChangeMaterial::aluminum();
        assert!(approx(pcm.melting_temperature, 933.0, 1e-3));
        assert!(pcm.latent_heat > 390_000.0);
    }

    #[test]
    fn test_paraffin_preset() {
        let pcm = PhaseChangeMaterial::paraffin();
        assert!(approx(pcm.melting_temperature, 330.0, 1e-3));
        assert!(pcm.latent_heat > 100_000.0);
    }

    #[test]
    fn test_liquid_fraction_solid() {
        let pcm = PhaseChangeMaterial::water_ice();
        let f = pcm.liquid_fraction(250.0);  // T < T_s
        assert!(approx(f, 0.0, 1e-6));
    }

    #[test]
    fn test_liquid_fraction_liquid() {
        let pcm = PhaseChangeMaterial::water_ice();
        let f = pcm.liquid_fraction(300.0);  // T > T_l
        assert!(approx(f, 1.0, 1e-6));
    }

    #[test]
    fn test_liquid_fraction_mushy() {
        let pcm = PhaseChangeMaterial::water_ice();
        // T = T_m, f_l = 0.5
        let f = pcm.liquid_fraction(273.15);
        assert!(approx(f, 0.5, 1e-3));
        // T_s, f_l = 0
        let f_s = pcm.liquid_fraction(pcm.t_solid());
        assert!(approx(f_s, 0.0, 1e-3));
        // T_l, f_l = 1
        let f_l = pcm.liquid_fraction(pcm.t_liquid());
        assert!(approx(f_l, 1.0, 1e-3));
    }

    #[test]
    fn test_liquid_fraction_monotonic() {
        let pcm = PhaseChangeMaterial::water_ice();
        let mut prev = -1.0;
        for i in 0..100 {
            let t = 260.0 + (i as f32) * 0.3;
            let f = pcm.liquid_fraction(t);
            assert!(f >= prev, "non-monotonic at t={}: f={} prev={}", t, f, prev);
            prev = f;
        }
    }

    #[test]
    fn test_effective_specific_heat_solid() {
        let pcm = PhaseChangeMaterial::water_ice();
        let c = pcm.effective_specific_heat(250.0);
        assert!(approx(c, 2090.0, 1e-3));  // 冰的比热
    }

    #[test]
    fn test_effective_specific_heat_liquid() {
        let pcm = PhaseChangeMaterial::water_ice();
        let c = pcm.effective_specific_heat(300.0);
        assert!(approx(c, 4186.0, 1e-3));  // 水的比热
    }

    #[test]
    fn test_effective_specific_heat_mushy_includes_latent() {
        let pcm = PhaseChangeMaterial::water_ice();
        // 糊状区中点, c_eff 应远大于 c_s 或 c_l
        let c = pcm.effective_specific_heat(273.15);
        // L / (T_l - T_s) = 334000 / 1.0 = 334000
        // c_mix ~= (2090+4186)/2 = 3138
        // c_eff ~= 3138 + 334000 = 337138
        assert!(c > 100_000.0, "c_eff should include latent heat: {}", c);
    }

    #[test]
    fn test_effective_conductivity() {
        let pcm = PhaseChangeMaterial::water_ice();
        let k_solid = pcm.effective_conductivity(0.0);
        let k_liquid = pcm.effective_conductivity(1.0);
        let k_mid = pcm.effective_conductivity(0.5);
        assert!(approx(k_solid, 2.18, 1e-3));
        assert!(approx(k_liquid, 0.6, 1e-3));
        // 0.5 * (2.18 + 0.6) = 1.39
        assert!(approx(k_mid, 1.39, 1e-3));
    }

    #[test]
    fn test_enthalpy_solid() {
        let pcm = PhaseChangeMaterial::water_ice();
        let ts = pcm.t_solid();
        // 在 T_s, H = 0 (参考点)
        let h = pcm.enthalpy(ts);
        assert!(h.abs() < 1.0, "H(T_s) should be 0, got {}", h);
        // T < T_s, H = rho_s * c_s * (T - T_s) < 0
        let h_cold = pcm.enthalpy(ts - 10.0);
        assert!(h_cold < 0.0, "H below T_s should be negative: {}", h_cold);
    }

    #[test]
    fn test_enthalpy_liquid() {
        let pcm = PhaseChangeMaterial::water_ice();
        let tl = pcm.t_liquid();
        // 在 T_l, H = rho_s * L (潜热完全吸收)
        let h = pcm.enthalpy(tl);
        let expected = pcm.solid.density * pcm.latent_heat;
        assert!(approx(h, expected, 1.0), "H(T_l)={}, expected={}", h, expected);
    }

    #[test]
    fn test_enthalpy_jump_at_melting() {
        // 潜热造成的焓跳跃
        let pcm = PhaseChangeMaterial::water_ice();
        let h_solid = pcm.enthalpy(pcm.t_solid() - 0.01);
        let h_liquid = pcm.enthalpy(pcm.t_liquid() + 0.01);
        let jump = h_liquid - h_solid;
        // 跳跃应包含潜热 (rho * L)
        let latent = pcm.solid.density * pcm.latent_heat;
        assert!(jump > latent * 0.9, "jump={}, latent={}", jump, latent);
    }

    #[test]
    fn test_solver_creation() {
        let solver = make_small_solver();
        assert_eq!(solver.num_cells(), 64);
        assert_eq!(solver.temperature.len(), 64);
        assert_eq!(solver.liquid_fraction.len(), 64);
        assert_eq!(solver.time, 0.0);
    }

    #[test]
    fn test_set_temperature_updates_liquid_fraction() {
        let mut solver = make_small_solver();
        solver.set_temperature(0, 0, 0, 300.0);  // 液态
        assert!(approx(solver.liquid_fraction[solver.idx(0, 0, 0)], 1.0, 1e-3));
        solver.set_temperature(1, 0, 0, 250.0);  // 固态
        assert!(approx(solver.liquid_fraction[solver.idx(1, 0, 0)], 0.0, 1e-3));
    }

    #[test]
    fn test_set_temperature_uniform() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(300.0);
        for f in &solver.liquid_fraction {
            assert!(approx(*f, 1.0, 1e-3));
        }
    }

    #[test]
    fn test_step_no_change_uniform() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(300.0);  // 全液态
        let t_before = solver.average_temperature();
        solver.step(0.1);
        let t_after = solver.average_temperature();
        assert!(approx(t_before, t_after, 1e-3));
    }

    #[test]
    fn test_step_heat_source() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(250.0);  // 全固态
        solver.add_heat_source(2, 2, 2, 1e8);
        let t_before = solver.temperature[solver.idx(2, 2, 2)];
        solver.step(0.1);
        let t_after = solver.temperature[solver.idx(2, 2, 2)];
        assert!(t_after > t_before);
    }

    #[test]
    fn test_melting_with_heat_source() {
        // 持续加热固态物质, 应发生熔化
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(270.0);  // 接近 T_m 但仍固态
        solver.add_heat_source(2, 2, 2, 1e9);
        // 步进多次, 中心应熔化
        for _ in 0..20 {
            solver.step(0.1);
        }
        let f_l = solver.liquid_fraction[solver.idx(2, 2, 2)];
        assert!(f_l > 0.5, "center should melt: f_l={}", f_l);
    }

    #[test]
    fn test_freezing_with_cooling() {
        // 持续冷却液态物质, 应发生凝固
        let mut solver = make_small_solver();
        // 使用负热源 (吸热) — 用大的固定低温边界
        solver.set_temperature_uniform(280.0);  // 液态
        // 设置角落为低温固定
        solver.set_fixed(0, 0, 0, 250.0);
        for _ in 0..30 {
            solver.step(0.1);
        }
        // 角落附近应凝固
        let f_l_corner = solver.liquid_fraction[solver.idx(0, 0, 0)];
        assert!(f_l_corner < 0.1, "corner should freeze: f_l={}", f_l_corner);
    }

    #[test]
    fn test_latent_heat_plateau() {
        // 潜热吸收: 熔化过程中温度上升变慢 (Stefan 停滞)
        // 用相同热源加热, 有相变 vs 无相变, 前者升温更慢
        let mut solver_pcm = make_small_solver();
        solver_pcm.set_temperature_uniform(272.0);
        solver_pcm.add_heat_source(0, 0, 0, 5e8);

        for _ in 0..5 {
            solver_pcm.step(0.1);
        }
        let t_after = solver_pcm.temperature[solver_pcm.idx(0, 0, 0)];
        // 因为潜热, 温度上升应受限
        // 在糊状区, 大量能量用于相变而非升温
        let f_l = solver_pcm.liquid_fraction[solver_pcm.idx(0, 0, 0)];
        // 中心应该部分熔化
        assert!(f_l > 0.0, "should start melting: f_l={}", f_l);
    }

    #[test]
    fn test_dirichlet_holds_temperature() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(280.0);
        solver.set_fixed(0, 0, 0, 250.0);
        for _ in 0..10 {
            solver.step(0.1);
        }
        let t = solver.temperature[solver.idx(0, 0, 0)];
        assert!(approx(t, 250.0, 1e-3));
    }

    #[test]
    fn test_time_advances() {
        let mut solver = make_small_solver();
        assert_eq!(solver.time, 0.0);
        solver.step(0.5);
        assert!(approx(solver.time, 0.5, 1e-6));
        solver.step(0.3);
        assert!(approx(solver.time, 0.8, 1e-6));
    }

    #[test]
    fn test_total_liquid_fraction() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(300.0);  // 全液态
        assert!(approx(solver.total_liquid_fraction(), 1.0, 1e-3));
        solver.set_temperature_uniform(250.0);  // 全固态
        assert!(approx(solver.total_liquid_fraction(), 0.0, 1e-3));
    }

    #[test]
    fn test_num_phase_cells() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(250.0);  // 全固态
        assert_eq!(solver.num_solid_cells(), 64);
        assert_eq!(solver.num_liquid_cells(), 0);
        assert_eq!(solver.num_mushy_cells(), 0);

        solver.set_temperature_uniform(300.0);  // 全液态
        assert_eq!(solver.num_solid_cells(), 0);
        assert_eq!(solver.num_liquid_cells(), 64);
    }

    #[test]
    fn test_reset() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(350.0);
        solver.add_heat_source(0, 0, 0, 1e6);
        solver.step(1.0);
        solver.reset();
        assert!(approx(solver.time, 0.0, 1e-6));
        // 重置后温度 = 环境温度 (300K, 液态)
        for t in &solver.temperature {
            assert!(approx(*t, 300.0, 1e-6));
        }
        for q in &solver.heat_source {
            assert!(approx(*q, 0.0, 1e-6));
        }
    }

    #[test]
    fn test_diffusion_spread() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(250.0);  // 固态
        solver.set_temperature(2, 2, 2, 280.0);  // 中心热
        for _ in 0..5 {
            solver.step(0.01);
        }
        let center = solver.temperature[solver.idx(2, 2, 2)];
        let neighbor = solver.temperature[solver.idx(3, 2, 2)];
        assert!(center < 280.0, "center should cool");
        assert!(neighbor > 250.0, "neighbor should warm");
    }

    #[test]
    fn test_multi_material() {
        let mut solver = make_small_solver();
        let iron_id = solver.add_material(PhaseChangeMaterial::iron());
        for k in 0..4 {
            for j in 0..4 {
                for i in 0..2 {
                    solver.set_material(i, j, k, iron_id);
                }
            }
        }
        assert_eq!(solver.material_id[solver.idx(0, 0, 0)], iron_id);
        assert_eq!(solver.material_id[solver.idx(3, 0, 0)], 0);
    }

    #[test]
    fn test_temperature_at_out_of_bounds_insulated() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(280.0);
        let t = solver.temperature_at(-1, 0, 0);
        assert!(approx(t, 280.0, 1e-6));
    }

    #[test]
    fn test_laplacian_zero_uniform() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(280.0);
        let lap = solver.laplacian(1, 1, 1);
        assert!(lap.abs() < 1e-6);
    }

    #[test]
    fn test_melting_temperature_stall() {
        // 经典 Stefan 问题: 持续加热时温度在 T_m 附近停滞
        let cfg = PhaseChangeConfig {
            nx: 4,
            ny: 1,
            nz: 1,
            h: 0.01,
            default_material: PhaseChangeMaterial::water_ice(),
            ..Default::default()
        };
        let mut solver = PhaseChangeSolver::new(cfg);
        // 起始: T_m - 5K (接近熔点固态)
        solver.set_temperature_uniform(268.0);
        // 中心持续小热源 (缓慢升温经过糊状区)
        solver.add_heat_source(1, 0, 0, 1e8);
        // 步进, 跟踪中心温度
        let mut max_temp_during_melt = 0.0_f32;
        let mut reached_melting = false;
        for _ in 0..50 {
            solver.step(0.01);
            let t = solver.temperature[solver.idx(1, 0, 0)];
            let f_l = solver.liquid_fraction[solver.idx(1, 0, 0)];
            if f_l > 0.01 && f_l < 0.99 {
                reached_melting = true;
                if t > max_temp_during_melt {
                    max_temp_during_melt = t;
                }
            }
        }
        assert!(reached_melting, "should enter mushy zone");
        // 熔化中温度不应远超 T_l (潜热吸收限制升温)
        let tl = PhaseChangeMaterial::water_ice().t_liquid();
        assert!(max_temp_during_melt < tl + 5.0,
            "temperature should stall near T_l during melting: max={}", max_temp_during_melt);
    }

    #[test]
    fn test_default_config() {
        let cfg = PhaseChangeConfig::default();
        assert_eq!(cfg.nx, 16);
        assert_eq!(cfg.max_iterations, 80);
        assert_eq!(cfg.ambient_temperature, 300.0);
    }

    #[test]
    fn test_default_material_is_water_ice() {
        let pcm = PhaseChangeMaterial::default();
        assert!(approx(pcm.melting_temperature, 273.15, 1e-3));
    }

    #[test]
    fn test_effective_diffusivity() {
        let pcm = PhaseChangeMaterial::water_ice();
        let d_solid = pcm.effective_diffusivity(250.0, 0.0);
        let d_liquid = pcm.effective_diffusivity(300.0, 1.0);
        // 两者都应为正
        assert!(d_solid > 0.0);
        assert!(d_liquid > 0.0);
    }

    #[test]
    fn test_steady_state_freezing() {
        // 两端固定温度 (一冷一热), 达到稳态后中间应有温度梯度
        let cfg = PhaseChangeConfig {
            nx: 8,
            ny: 1,
            nz: 1,
            h: 0.01,
            default_material: PhaseChangeMaterial::aluminum(),
            ..Default::default()
        };
        let mut solver = PhaseChangeSolver::new(cfg);
        solver.set_temperature_uniform(1000.0);  // 液态 (T > 933)
        solver.set_fixed(0, 0, 0, 850.0);  // 冷端 (固态, T < 933)
        solver.set_fixed(7, 0, 0, 1000.0);  // 热端 (液态)
        // 大量步进
        for _ in 0..300 {
            solver.step(1.0);
        }
        // 冷端附近应凝固
        let f_l_cold = solver.liquid_fraction[solver.idx(1, 0, 0)];
        assert!(f_l_cold < 0.5, "near cold end should freeze: f_l={}", f_l_cold);
        // 热端附近应保持液态
        let f_l_hot = solver.liquid_fraction[solver.idx(6, 0, 0)];
        assert!(f_l_hot > 0.5, "near hot end should stay liquid: f_l={}", f_l_hot);
    }

    #[test]
    fn test_3d_isotropic_diffusion() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(250.0);
        solver.set_temperature(2, 2, 2, 290.0);
        for _ in 0..5 {
            solver.step(0.01);
        }
        let t_xp = solver.temperature[solver.idx(3, 2, 2)];
        let t_xm = solver.temperature[solver.idx(1, 2, 2)];
        let t_yp = solver.temperature[solver.idx(2, 3, 2)];
        let t_ym = solver.temperature[solver.idx(2, 1, 2)];
        let t_zp = solver.temperature[solver.idx(2, 2, 3)];
        let t_zm = solver.temperature[solver.idx(2, 2, 1)];
        let avg = (t_xp + t_xm + t_yp + t_ym + t_zp + t_zm) / 6.0;
        for t in [t_xp, t_xm, t_yp, t_ym, t_zp, t_zm] {
            assert!((t - avg).abs() < 1.0, "isotropy: t={}, avg={}", t, avg);
        }
    }

    #[test]
    fn test_total_enthalpy_increases_with_heating() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(250.0);
        let e_before = solver.total_enthalpy();
        solver.add_heat_source(0, 0, 0, 1e9);
        for _ in 0..5 {
            solver.step(0.1);
        }
        let e_after = solver.total_enthalpy();
        assert!(e_after > e_before, "enthalpy should increase: before={}, after={}", e_before, e_after);
    }
}

