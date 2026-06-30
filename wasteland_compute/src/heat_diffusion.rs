//! Heat Diffusion Solver — 隐式 Euler 热传导求解器
//!
//! 基于:
//! - Patankar, S.V. "Numerical Heat Transfer and Fluid Flow." 1980.
//! - Incropera et al. "Fundamentals of Heat and Mass Transfer." 2017.
//! - Stam, J. "Stable Fluids." SIGGRAPH 1999. (Gauss-Seidel 隐式求解思想)
//!
//! 核心方程: dT/dt = alpha * laplacian(T) + Q/(rho*c)
//!   T = 温度 (K)
//!   alpha = k/(rho*c) = 热扩散率 (m^2/s)
//!   k = 热导率 (W/m/K)
//!   rho = 密度 (kg/m^3)
//!   c = 比热容 (J/kg/K)
//!   Q = 体积热源 (W/m^3)
//!
//! 离散化:
//! - 3D 笛卡尔网格 (Nx x Ny x Nz)
//! - 单元中心存储温度
//! - 7点拉普拉斯算子
//! - 隐式 Euler 时间积分 (无条件稳定)
//!
//! 边界条件:
//! - Dirichlet: 固定温度 T = T_bc
//! - Neumann: 绝热 dT/dn = 0 (零通量)
//! - 对流: -k * dT/dn = h * (T - T_amb) (Robin)
//!
//! 求解器:
//! - Gauss-Seidel 迭代
//! - 收敛判据: max|T^{k+1} - T^k| < tol
//!
//! 应用:
//! - 游戏烹饪机制 (食物加热)
//! - 冻结/融化
//! - 温度传播
//! - 材料相变
//! - 火焰/热传递
//! - 冷却散热

use serde::{Deserialize, Serialize};

// ============================================================
// 索引工具
// ============================================================

#[inline]
pub fn idx(i: usize, j: usize, k: usize, nx: usize, ny: usize) -> usize {
    i + nx * (j + ny * k)
}

// ============================================================
// 边界条件类型
// ============================================================

/// 边界条件类型
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BoundaryType {
    /// 绝热 (零通量, Neumann dT/dn = 0)
    Insulated,
    /// 固定温度 (Dirichlet T = value)
    Fixed(f32),
    /// 对流换热 (Robin: -k * dT/dn = h * (T - T_inf))
    Convective { h: f32, t_inf: f32 },
}

impl Default for BoundaryType {
    fn default() -> Self {
        BoundaryType::Insulated
    }
}

// ============================================================
// 热材料属性
// ============================================================

/// 热材料属性
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThermalMaterial {
    /// 热导率 k (W/m/K)
    pub conductivity: f32,
    /// 密度 rho (kg/m^3)
    pub density: f32,
    /// 比热容 c (J/kg/K)
    pub specific_heat: f32,
}

impl ThermalMaterial {
    /// 热扩散率 alpha = k / (rho*c)  (m^2/s)
    #[inline]
    pub fn diffusivity(&self) -> f32 {
        let denom = self.density * self.specific_heat;
        if denom < 1e-12 {
            0.0
        } else {
            self.conductivity / denom
        }
    }

    /// 体积热容 rho*c (J/m^3/K)
    #[inline]
    pub fn volumetric_heat_capacity(&self) -> f32 {
        self.density * self.specific_heat
    }

    pub fn water() -> Self {
        Self { conductivity: 0.6, density: 1000.0, specific_heat: 4186.0 }
    }
    pub fn iron() -> Self {
        Self { conductivity: 80.0, density: 7870.0, specific_heat: 450.0 }
    }
    pub fn aluminum() -> Self {
        Self { conductivity: 237.0, density: 2700.0, specific_heat: 900.0 }
    }
    pub fn copper() -> Self {
        Self { conductivity: 401.0, density: 8960.0, specific_heat: 385.0 }
    }
    pub fn wood() -> Self {
        Self { conductivity: 0.15, density: 700.0, specific_heat: 1700.0 }
    }
    pub fn stone() -> Self {
        Self { conductivity: 2.5, density: 2700.0, specific_heat: 800.0 }
    }
    pub fn air() -> Self {
        Self { conductivity: 0.026, density: 1.2, specific_heat: 1005.0 }
    }
    pub fn glass() -> Self {
        Self { conductivity: 1.0, density: 2500.0, specific_heat: 840.0 }
    }
}

impl Default for ThermalMaterial {
    fn default() -> Self {
        // 水: k=0.6, rho=1000, c=4186 -> alpha ~= 1.43e-7
        Self {
            conductivity: 0.6,
            density: 1000.0,
            specific_heat: 4186.0,
        }
    }
}

// ============================================================
// 求解器配置
// ============================================================

/// 热扩散求解器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatDiffusionConfig {
    /// 网格分辨率 (Nx, Ny, Nz)
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    /// 网格间距 h (m)
    pub h: f32,
    /// 默认材料
    pub default_material: ThermalMaterial,
    /// 6 个面的边界条件 (顺序: -x, +x, -y, +y, -z, +z)
    pub boundaries: [BoundaryType; 6],
    /// Gauss-Seidel 最大迭代次数
    pub max_iterations: usize,
    /// 收敛容差 (K)
    pub tolerance: f32,
    /// 环境温度 (K, 用于初始化和对流边界)
    pub ambient_temperature: f32,
}

impl Default for HeatDiffusionConfig {
    fn default() -> Self {
        Self {
            nx: 16,
            ny: 16,
            nz: 16,
            h: 0.01,
            default_material: ThermalMaterial::water(),
            boundaries: [
                BoundaryType::Insulated,
                BoundaryType::Insulated,
                BoundaryType::Insulated,
                BoundaryType::Insulated,
                BoundaryType::Insulated,
                BoundaryType::Insulated,
            ],
            max_iterations: 50,
            tolerance: 1e-4,
            ambient_temperature: 300.0,
        }
    }
}

// ============================================================
// 热扩散求解器
// ============================================================

/// 隐式 Euler 热传导求解器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatDiffusionSolver {
    /// 配置
    pub config: HeatDiffusionConfig,
    /// 温度场 T[i,j,k] (K)
    pub temperature: Vec<f32>,
    /// 体积热源 Q[i,j,k] (W/m^3)
    pub heat_source: Vec<f32>,
    /// 每个单元的材料索引 (引用 materials 向量)
    pub material_id: Vec<usize>,
    /// 材料列表
    pub materials: Vec<ThermalMaterial>,
    /// Dirichlet 固定温度掩码 (true 表示该单元温度固定)
    pub fixed_mask: Vec<bool>,
    /// Dirichlet 固定温度值
    pub fixed_value: Vec<f32>,
    /// 模拟时间 (s)
    pub time: f32,
}

impl HeatDiffusionSolver {
    /// 创建求解器 (全场初始化为环境温度)
    pub fn new(config: HeatDiffusionConfig) -> Self {
        let n = config.nx * config.ny * config.nz;
        let amb = config.ambient_temperature;
        let default_mat = config.default_material;
        Self {
            config,
            temperature: vec![amb; n],
            heat_source: vec![0.0; n],
            material_id: vec![0; n],
            materials: vec![default_mat],
            fixed_mask: vec![false; n],
            fixed_value: vec![amb; n],
            time: 0.0,
        }
    }

    /// 单元数
    #[inline]
    pub fn num_cells(&self) -> usize {
        self.config.nx * self.config.ny * self.config.nz
    }

    /// 线性索引
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

    /// 边界温度 (用于拉普拉斯算子的虚拟单元)
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
                    let k_cond = mat.conductivity;
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

    /// 设置整个区域的温度
    pub fn set_temperature_uniform(&mut self, t: f32) {
        for v in &mut self.temperature {
            *v = t;
        }
    }

    /// 设置某个单元的温度
    pub fn set_temperature(&mut self, i: usize, j: usize, k: usize, t: f32) {
        let idx = self.idx(i, j, k);
        self.temperature[idx] = t;
    }

    /// 设置某单元为 Dirichlet (固定温度)
    pub fn set_fixed(&mut self, i: usize, j: usize, k: usize, t: f32) {
        let idx = self.idx(i, j, k);
        self.fixed_mask[idx] = true;
        self.fixed_value[idx] = t;
        self.temperature[idx] = t;
    }

    /// 解除 Dirichlet 约束
    pub fn clear_fixed(&mut self, i: usize, j: usize, k: usize) {
        let idx = self.idx(i, j, k);
        self.fixed_mask[idx] = false;
    }

    /// 添加体积热源 (W/m^3)
    pub fn add_heat_source(&mut self, i: usize, j: usize, k: usize, q: f32) {
        let idx = self.idx(i, j, k);
        self.heat_source[idx] += q;
    }

    /// 设置某单元材料
    pub fn set_material(&mut self, i: usize, j: usize, k: usize, mat_id: usize) {
        let idx = self.idx(i, j, k);
        self.material_id[idx] = mat_id;
    }

    /// 添加新材料, 返回其 id
    pub fn add_material(&mut self, mat: ThermalMaterial) -> usize {
        self.materials.push(mat);
        self.materials.len() - 1
    }

    /// 计算单元 (i,j,k) 的拉普拉斯算子 laplacian(T)
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

    /// 单步隐式 Euler 时间步进
    ///
    /// (T^{n+1} - T^n) / dt = alpha * laplacian(T^{n+1}) + Q/(rho*c)
    /// -> T^{n+1} - dt*alpha*laplacian(T^{n+1}) = T^n + dt*Q/(rho*c)
    ///
    /// 离散化每个单元 (7点):
    ///   (1 + 6r) * T_c - r * (T_ip + T_im + T_jp + T_jm + T_kp + T_km) = T^n_c + dt*Q/(rho*c)
    ///   其中 r = alpha*dt/h^2
    pub fn step(&mut self, dt: f32) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let nz = self.config.nz;
        let h = self.config.h;
        let h2 = h * h;

        let t_old = self.temperature.clone();

        let n = self.num_cells();
        let mut r = vec![0.0_f32; n];
        let mut source = vec![0.0_f32; n];
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let idx = self.idx(i, j, k);
                    let mat = self.materials[self.material_id[idx]];
                    let alpha = mat.diffusivity();
                    r[idx] = alpha * dt / h2;
                    let rho_c = mat.volumetric_heat_capacity();
                    source[idx] = t_old[idx] + if rho_c > 1e-12 {
                        dt * self.heat_source[idx] / rho_c
                    } else {
                        0.0
                    };
                }
            }
        }

        for _it in 0..self.config.max_iterations {
            let mut max_diff = 0.0_f32;
            for k in 0..nz {
                for j in 0..ny {
                    for i in 0..nx {
                        let idx = self.idx(i, j, k);
                        if self.fixed_mask[idx] {
                            continue;
                        }
                        let ri = r[idx];
                        let rhs = source[idx];
                        let t_ip = self.temperature_at(i as isize + 1, j as isize, k as isize);
                        let t_im = self.temperature_at(i as isize - 1, j as isize, k as isize);
                        let t_jp = self.temperature_at(i as isize, j as isize + 1, k as isize);
                        let t_jm = self.temperature_at(i as isize, j as isize - 1, k as isize);
                        let t_kp = self.temperature_at(i as isize, j as isize, k as isize + 1);
                        let t_km = self.temperature_at(i as isize, j as isize, k as isize - 1);
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

        self.time += dt;
    }

    /// 显式 Euler 步进 (受 CFL 约束 alpha*dt/h^2 <= 0.5)
    pub fn step_explicit(&mut self, dt: f32) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let nz = self.config.nz;
        let h2 = self.config.h * self.config.h;

        let mut new_t = self.temperature.clone();
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let idx = self.idx(i, j, k);
                    if self.fixed_mask[idx] {
                        continue;
                    }
                    let mat = self.materials[self.material_id[idx]];
                    let alpha = mat.diffusivity();
                    let lap = self.laplacian(i, j, k);
                    let rho_c = mat.volumetric_heat_capacity();
                    let q_term = if rho_c > 1e-12 {
                        self.heat_source[idx] / rho_c
                    } else {
                        0.0
                    };
                    new_t[idx] = self.temperature[idx] + dt * (alpha * lap + q_term);
                }
            }
        }
        self.temperature = new_t;
        self.time += dt;
    }

    /// 半拉格朗日对流温度场 (与流体速度场耦合)
    pub fn advect(&mut self, dt: f32, u: &[f32], v: &[f32], w: &[f32]) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let nz = self.config.nz;
        let h = self.config.h;

        let old_t = self.temperature.clone();
        let mut new_t = self.temperature.clone();

        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let idx = self.idx(i, j, k);
                    if self.fixed_mask[idx] {
                        continue;
                    }
                    let ii = i as f32;
                    let jj = j as f32;
                    let kk = k as f32;
                    let u_vel = u[idx];
                    let v_vel = v[idx];
                    let w_vel = w[idx];
                    let pi = ii - dt * u_vel / h;
                    let pj = jj - dt * v_vel / h;
                    let pk = kk - dt * w_vel / h;
                    new_t[idx] = self.trilinear_sample(&old_t, pi, pj, pk);
                }
            }
        }
        self.temperature = new_t;
    }

    /// 三线性插值采样 (网格坐标, 越界用边界条件)
    fn trilinear_sample(&self, field: &[f32], pi: f32, pj: f32, pk: f32) -> f32 {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let nz = self.config.nz;

        let i0 = pi.floor() as isize;
        let j0 = pj.floor() as isize;
        let k0 = pk.floor() as isize;
        let i1 = i0 + 1;
        let j1 = j0 + 1;
        let k1 = k0 + 1;

        let fx = pi - i0 as f32;
        let fy = pj - j0 as f32;
        let fz = pk - k0 as f32;

        let sample = |ii: isize, jj: isize, kk: isize| -> f32 {
            if ii < 0 || ii >= nx as isize || jj < 0 || jj >= ny as isize || kk < 0 || kk >= nz as isize {
                self.boundary_temperature(ii, jj, kk)
            } else {
                field[self.idx(ii as usize, jj as usize, kk as usize)]
            }
        };

        let c000 = sample(i0, j0, k0);
        let c100 = sample(i1, j0, k0);
        let c010 = sample(i0, j1, k0);
        let c110 = sample(i1, j1, k0);
        let c001 = sample(i0, j0, k1);
        let c101 = sample(i1, j0, k1);
        let c011 = sample(i0, j1, k1);
        let c111 = sample(i1, j1, k1);

        let c00 = c000 * (1.0 - fx) + c100 * fx;
        let c10 = c010 * (1.0 - fx) + c110 * fx;
        let c01 = c001 * (1.0 - fx) + c101 * fx;
        let c11 = c011 * (1.0 - fx) + c111 * fx;

        let c0 = c00 * (1.0 - fy) + c10 * fy;
        let c1 = c01 * (1.0 - fy) + c11 * fy;

        c0 * (1.0 - fz) + c1 * fz
    }

    /// 总热能 (J, 相对 0°C = 273.15 K)
    pub fn total_thermal_energy(&self) -> f32 {
        let h = self.config.h;
        let vol = h * h * h;
        let mut e = 0.0;
        for k in 0..self.config.nz {
            for j in 0..self.config.ny {
                for i in 0..self.config.nx {
                    let idx = self.idx(i, j, k);
                    let mat = self.materials[self.material_id[idx]];
                    let rho_c = mat.volumetric_heat_capacity();
                    e += rho_c * (self.temperature[idx] - 273.15) * vol;
                }
            }
        }
        e
    }

    /// 平均温度 (K)
    pub fn average_temperature(&self) -> f32 {
        if self.temperature.is_empty() {
            return 0.0;
        }
        self.temperature.iter().sum::<f32>() / self.temperature.len() as f32
    }

    /// 最高温度 (K)
    pub fn max_temperature(&self) -> f32 {
        self.temperature.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
    }

    /// 最低温度 (K)
    pub fn min_temperature(&self) -> f32 {
        self.temperature.iter().cloned().fold(f32::INFINITY, f32::min)
    }

    /// 清空热源
    pub fn clear_heat_sources(&mut self) {
        for q in &mut self.heat_source {
            *q = 0.0;
        }
    }

    /// 重置 (温度 = 环境温度, 清空热源)
    pub fn reset(&mut self) {
        let amb = self.config.ambient_temperature;
        for t in &mut self.temperature {
            *t = amb;
        }
        for q in &mut self.heat_source {
            *q = 0.0;
        }
        self.time = 0.0;
    }

    /// CFL 时间步长上限 (显式方法, 安全系数 0.5)
    pub fn cfl_dt(&self) -> f32 {
        let h2 = self.config.h * self.config.h;
        let mut max_alpha = 0.0_f32;
        for m in &self.materials {
            let a = m.diffusivity();
            if a > max_alpha {
                max_alpha = a;
            }
        }
        if max_alpha < 1e-12 {
            f32::INFINITY
        } else {
            0.5 * h2 / (6.0 * max_alpha)
        }
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

    fn make_small_solver() -> HeatDiffusionSolver {
        let cfg = HeatDiffusionConfig {
            nx: 4,
            ny: 4,
            nz: 4,
            h: 0.01,
            ..Default::default()
        };
        HeatDiffusionSolver::new(cfg)
    }

    #[test]
    fn test_default_config() {
        let cfg = HeatDiffusionConfig::default();
        assert_eq!(cfg.nx, 16);
        assert_eq!(cfg.ny, 16);
        assert_eq!(cfg.nz, 16);
        assert_eq!(cfg.h, 0.01);
        assert_eq!(cfg.max_iterations, 50);
        assert_eq!(cfg.ambient_temperature, 300.0);
        assert_eq!(cfg.boundaries[0], BoundaryType::Insulated);
    }

    #[test]
    fn test_solver_creation() {
        let solver = make_small_solver();
        assert_eq!(solver.num_cells(), 64);
        assert_eq!(solver.temperature.len(), 64);
        assert_eq!(solver.heat_source.len(), 64);
        assert_eq!(solver.time, 0.0);
    }

    #[test]
    fn test_material_diffusivity() {
        let water = ThermalMaterial::water();
        let alpha = water.diffusivity();
        // alpha = 0.6 / (1000 * 4186) ~= 1.434e-7
        assert!(approx(alpha, 1.434e-7, 1e-9), "water alpha = {}", alpha);

        let iron = ThermalMaterial::iron();
        let alpha_fe = iron.diffusivity();
        // alpha = 80 / (7870 * 450) ~= 2.26e-5
        assert!(approx(alpha_fe, 2.26e-5, 1e-7), "iron alpha = {}", alpha_fe);
    }

    #[test]
    fn test_material_presets() {
        let cu = ThermalMaterial::copper();
        assert!(cu.conductivity > 400.0);
        let cu_alpha = cu.diffusivity();
        let wood = ThermalMaterial::wood();
        let wood_alpha = wood.diffusivity();
        assert!(cu_alpha > wood_alpha * 100.0);
    }

    #[test]
    fn test_indexing() {
        let solver = make_small_solver();
        assert_eq!(solver.idx(0, 0, 0), 0);
        assert_eq!(solver.idx(1, 0, 0), 1);
        assert_eq!(solver.idx(0, 1, 0), 4);
        assert_eq!(solver.idx(0, 0, 1), 16);
        assert_eq!(solver.idx(3, 3, 3), 63);
    }

    #[test]
    fn test_uniform_temperature() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(350.0);
        for t in &solver.temperature {
            assert!(approx(*t, 350.0, 1e-6));
        }
    }

    #[test]
    fn test_set_temperature() {
        let mut solver = make_small_solver();
        solver.set_temperature(1, 2, 3, 500.0);
        let idx = solver.idx(1, 2, 3);
        assert!(approx(solver.temperature[idx], 500.0, 1e-6));
    }

    #[test]
    fn test_fixed_boundary() {
        let mut solver = make_small_solver();
        solver.set_fixed(0, 0, 0, 500.0);
        let idx = solver.idx(0, 0, 0);
        assert!(solver.fixed_mask[idx]);
        assert!(approx(solver.temperature[idx], 500.0, 1e-6));
        assert!(approx(solver.fixed_value[idx], 500.0, 1e-6));
    }

    #[test]
    fn test_clear_fixed() {
        let mut solver = make_small_solver();
        solver.set_fixed(0, 0, 0, 500.0);
        solver.clear_fixed(0, 0, 0);
        let idx = solver.idx(0, 0, 0);
        assert!(!solver.fixed_mask[idx]);
    }

    #[test]
    fn test_heat_source() {
        let mut solver = make_small_solver();
        solver.add_heat_source(2, 2, 2, 1e6);
        let idx = solver.idx(2, 2, 2);
        assert!(approx(solver.heat_source[idx], 1e6, 1e-3));
    }

    #[test]
    fn test_insulated_boundary_ghost() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(300.0);
        let ghost = solver.boundary_temperature(-1, 0, 0);
        assert!(approx(ghost, 300.0, 1e-6));
    }

    #[test]
    fn test_fixed_boundary_ghost() {
        let mut solver = make_small_solver();
        solver.config.boundaries[0] = BoundaryType::Fixed(400.0);
        let ghost = solver.boundary_temperature(-1, 0, 0);
        assert!(approx(ghost, 400.0, 1e-6));
    }

    #[test]
    fn test_laplacian_zero_uniform() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(300.0);
        let lap = solver.laplacian(1, 1, 1);
        assert!(lap.abs() < 1e-6, "laplacian = {}", lap);
    }

    #[test]
    fn test_laplacian_gradient() {
        let mut solver = make_small_solver();
        for k in 0..4 {
            for j in 0..4 {
                for i in 0..4 {
                    solver.set_temperature(i, j, k, 100.0 + 10.0 * i as f32);
                }
            }
        }
        let lap = solver.laplacian(1, 1, 1);
        assert!(lap.abs() < 1e-3, "laplacian = {}", lap);
    }

    #[test]
    fn test_laplacian_quadratic() {
        let mut solver = make_small_solver();
        let h = solver.config.h;
        for k in 0..4 {
            for j in 0..4 {
                for i in 0..4 {
                    solver.set_temperature(i, j, k, (i as f32) * (i as f32));
                }
            }
        }
        let lap = solver.laplacian(2, 1, 1);
        let expected = 2.0 / (h * h);
        assert!(approx(lap, expected, 1e-3), "lap = {}, expected = {}", lap, expected);
    }

    #[test]
    fn test_step_no_change_uniform() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(300.0);
        let t_before = solver.average_temperature();
        solver.step(0.1);
        let t_after = solver.average_temperature();
        assert!(approx(t_before, t_after, 1e-3), "before={}, after={}", t_before, t_after);
    }

    #[test]
    fn test_step_heat_source() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(300.0);
        solver.add_heat_source(2, 2, 2, 1e7);
        let t_before = solver.temperature[solver.idx(2, 2, 2)];
        solver.step(1.0);
        let t_after = solver.temperature[solver.idx(2, 2, 2)];
        assert!(t_after > t_before, "before={}, after={}", t_before, t_after);
    }

    #[test]
    fn test_step_diffusion_spread() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(300.0);
        solver.set_temperature(2, 2, 2, 1000.0);
        for _ in 0..5 {
            solver.step(0.01);
        }
        let center = solver.temperature[solver.idx(2, 2, 2)];
        let neighbor = solver.temperature[solver.idx(3, 2, 2)];
        assert!(center < 1000.0, "center = {} (should drop)", center);
        assert!(neighbor > 300.0, "neighbor = {} (should rise)", neighbor);
    }

    #[test]
    fn test_dirichlet_holds_temperature() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(300.0);
        solver.set_fixed(0, 0, 0, 500.0);
        for _ in 0..10 {
            solver.step(0.1);
        }
        let t = solver.temperature[solver.idx(0, 0, 0)];
        assert!(approx(t, 500.0, 1e-3), "fixed cell drifted to {}", t);
    }

    #[test]
    fn test_insulated_no_heat_loss() {
        let mut solver = make_small_solver();
        solver.set_temperature(1, 1, 1, 500.0);
        let e_before = solver.total_thermal_energy();
        for _ in 0..5 {
            solver.step(0.01);
        }
        let e_after = solver.total_thermal_energy();
        let rel_err = (e_after - e_before).abs() / e_before.abs().max(1e-6);
        assert!(rel_err < 0.05, "energy drift: before={}, after={}, rel={}", e_before, e_after, rel_err);
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
    fn test_explicit_step() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(300.0);
        solver.set_temperature(1, 1, 1, 500.0);
        let before = solver.temperature[solver.idx(1, 1, 1)];
        solver.step_explicit(1e-4);
        let after = solver.temperature[solver.idx(1, 1, 1)];
        assert!(after < before, "before={}, after={}", before, after);
    }

    #[test]
    fn test_cfl_dt() {
        let solver = make_small_solver();
        let dt = solver.cfl_dt();
        assert!(dt > 1.0, "CFL dt = {}", dt);
    }

    #[test]
    fn test_average_temperature() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(300.0);
        assert!(approx(solver.average_temperature(), 300.0, 1e-6));
        solver.set_temperature(0, 0, 0, 700.0);
        assert!(approx(solver.average_temperature(), 306.25, 1e-3));
    }

    #[test]
    fn test_min_max_temperature() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(300.0);
        solver.set_temperature(1, 1, 1, 500.0);
        solver.set_temperature(2, 2, 2, 100.0);
        assert!(approx(solver.max_temperature(), 500.0, 1e-6));
        assert!(approx(solver.min_temperature(), 100.0, 1e-6));
    }

    #[test]
    fn test_total_thermal_energy() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(300.0);
        let e = solver.total_thermal_energy();
        let expected = 64.0 * 1e-6 * 4.186e6 * (300.0 - 273.15);
        assert!(approx(e, expected, 1e-1), "e={}, expected={}", e, expected);
    }

    #[test]
    fn test_multi_material() {
        let mut solver = make_small_solver();
        let iron_id = solver.add_material(ThermalMaterial::iron());
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
    fn test_multi_material_diffusion() {
        let mut solver = make_small_solver();
        let iron_id = solver.add_material(ThermalMaterial::iron());
        for k in 0..4 {
            for j in 0..4 {
                for i in 0..2 {
                    solver.set_material(i, j, k, iron_id);
                }
            }
        }
        solver.set_temperature_uniform(300.0);
        solver.set_temperature(0, 2, 2, 1000.0);
        for _ in 0..3 {
            solver.step(0.01);
        }
        let t_iron = solver.temperature[solver.idx(1, 2, 2)];
        let t_water = solver.temperature[solver.idx(3, 2, 2)];
        assert!(t_iron > t_water, "iron={}, water={}", t_iron, t_water);
    }

    #[test]
    fn test_convective_boundary() {
        let mut solver = make_small_solver();
        solver.config.boundaries[0] = BoundaryType::Convective { h: 100.0, t_inf: 200.0 };
        solver.set_temperature_uniform(400.0);
        let ghost = solver.boundary_temperature(-1, 0, 0);
        assert!(ghost < 400.0, "ghost should be cooler: {}", ghost);
        assert!(ghost > 0.0);
    }

    #[test]
    fn test_advect() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(300.0);
        solver.set_temperature(1, 1, 1, 1000.0);
        let n = solver.num_cells();
        let u = vec![1.0; n];
        let v = vec![0.0; n];
        let w = vec![0.0; n];
        solver.advect(0.005, &u, &v, &w);
        let after_at_2 = solver.temperature[solver.idx(2, 1, 1)];
        assert!(after_at_2 > 300.0, "i=2 should rise: {}", after_at_2);
    }

    #[test]
    fn test_clear_heat_sources() {
        let mut solver = make_small_solver();
        solver.add_heat_source(0, 0, 0, 1e6);
        solver.add_heat_source(1, 1, 1, 2e6);
        solver.clear_heat_sources();
        for q in &solver.heat_source {
            assert!(approx(*q, 0.0, 1e-6));
        }
    }

    #[test]
    fn test_reset() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(500.0);
        solver.add_heat_source(0, 0, 0, 1e6);
        solver.step(1.0);
        solver.reset();
        assert!(approx(solver.time, 0.0, 1e-6));
        for t in &solver.temperature {
            assert!(approx(*t, 300.0, 1e-6));
        }
        for q in &solver.heat_source {
            assert!(approx(*q, 0.0, 1e-6));
        }
    }

    #[test]
    fn test_steady_state_dirichlet() {
        let cfg = HeatDiffusionConfig {
            nx: 8,
            ny: 1,
            nz: 1,
            h: 0.01,
            default_material: ThermalMaterial::copper(),
            ..Default::default()
        };
        let mut solver = HeatDiffusionSolver::new(cfg);
        solver.set_temperature_uniform(300.0);
        solver.set_fixed(0, 0, 0, 300.0);
        solver.set_fixed(7, 0, 0, 400.0);
        // 铜的 alpha ~= 1.16e-4, h=0.01 -> h^2=1e-4
        // dt=1.0 -> r = 1.16, 200 步足够收敛到稳态
        for _ in 0..200 {
            solver.step(1.0);
        }
        for i in 0..8 {
            let expected = 300.0 + 100.0 * (i as f32) / 7.0;
            let actual = solver.temperature[solver.idx(i, 0, 0)];
            assert!(approx(actual, expected, 5.0), "i={}: actual={}, expected={}", i, actual, expected);
        }
    }

    #[test]
    fn test_high_conductivity_diffuses_faster() {
        let mut cfg_cu = HeatDiffusionConfig::default();
        cfg_cu.default_material = ThermalMaterial::copper();
        cfg_cu.nx = 5;
        cfg_cu.ny = 5;
        cfg_cu.nz = 5;
        let mut solver_cu = HeatDiffusionSolver::new(cfg_cu);
        solver_cu.set_temperature_uniform(300.0);
        solver_cu.set_temperature(2, 2, 2, 1000.0);

        let mut cfg_w = HeatDiffusionConfig::default();
        cfg_w.default_material = ThermalMaterial::wood();
        cfg_w.nx = 5;
        cfg_w.ny = 5;
        cfg_w.nz = 5;
        let mut solver_w = HeatDiffusionSolver::new(cfg_w);
        solver_w.set_temperature_uniform(300.0);
        solver_w.set_temperature(2, 2, 2, 1000.0);

        for _ in 0..3 {
            solver_cu.step(0.001);
            solver_w.step(0.001);
        }
        let cu_neighbor = solver_cu.temperature[solver_cu.idx(3, 2, 2)];
        let w_neighbor = solver_w.temperature[solver_w.idx(3, 2, 2)];
        assert!(cu_neighbor > w_neighbor, "cu={}, wood={}", cu_neighbor, w_neighbor);
    }

    #[test]
    fn test_boundary_default() {
        let bt = BoundaryType::default();
        assert_eq!(bt, BoundaryType::Insulated);
    }

    #[test]
    fn test_material_default() {
        let mat = ThermalMaterial::default();
        assert!(approx(mat.conductivity, 0.6, 1e-6));
        assert!(approx(mat.density, 1000.0, 1e-6));
        assert!(approx(mat.specific_heat, 4186.0, 1e-6));
    }

    #[test]
    fn test_volumetric_heat_capacity() {
        let water = ThermalMaterial::water();
        let rho_c = water.volumetric_heat_capacity();
        assert!(approx(rho_c, 4_186_000.0, 1.0));
    }

    #[test]
    fn test_set_material_affects_diffusion() {
        let mut solver = make_small_solver();
        let cu_id = solver.add_material(ThermalMaterial::copper());
        for k in 0..4 {
            for j in 0..4 {
                for i in 0..4 {
                    solver.set_material(i, j, k, cu_id);
                }
            }
        }
        solver.set_temperature_uniform(300.0);
        solver.set_temperature(0, 0, 0, 1000.0);
        let t_before = solver.temperature[solver.idx(1, 0, 0)];
        solver.step(0.01);
        let t_after = solver.temperature[solver.idx(1, 0, 0)];
        assert!(t_after > t_before, "neighbor should heat up");
    }

    #[test]
    fn test_temperature_at_out_of_bounds() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(300.0);
        let t = solver.temperature_at(-1, 0, 0);
        assert!(approx(t, 300.0, 1e-6));
    }

    #[test]
    fn test_3d_diffusion_isotropic() {
        let mut solver = make_small_solver();
        solver.set_temperature_uniform(300.0);
        solver.set_temperature(2, 2, 2, 1000.0);
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
            assert!((t - avg).abs() < 1.0, "isotropy violated: t={}, avg={}", t, avg);
        }
    }
}

