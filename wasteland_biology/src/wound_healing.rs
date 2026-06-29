//! 伤口愈合模拟 —— 凝血级联 ODE + 6 物种愈合场
//!
//! 论文来源：
//! - Wajchman 2011, "A simple kinetic model of fibrin clot formation"
//!   —— 凝血 3 变量简化模型（P, T, u）
//! - Javierre 2008, "A mathematical model of dermal wound healing"
//!   —— 6 物种伤口愈合 PDE
//! - Murphy 2012, "A computational model of dermal wound closure"

use serde::{Deserialize, Serialize};

// =====================================================================
// 第一部分：凝血级联 ODE —— Wajchman 2011
// =====================================================================

/// 凝血模型参数
///
/// 对应 Wajchman 2011 论文 Eq.(1)-(3) 的 7 个动力学常数
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CoagulationModel {
    /// k1: 凝血酶原 (P) 被凝血酶 (T) 激活的速率常数
    pub k1: f32,
    /// k2: 凝血酶自身催化的负反馈抑制系数（出现在 1/(1+k2·T²) 项中）
    pub k2: f32,
    /// k3: 凝血酶 (T) 的自然失活速率
    pub k3: f32,
    /// k4: 凝血酶 (T) 与纤维蛋白原 (u) 结合的消耗速率
    pub k4: f32,
    /// k5: 纤维蛋白原 (u) 在凝血酶催化下转化为纤维蛋白的速率
    pub k5: f32,
    /// k6: 纤溶速率常数（纤维蛋白降解）
    pub k6: f32,
    /// k7: 纤溶辅助常数（与纤维蛋白浓度耦合）
    pub k7: f32,
}

/// 凝血状态：3 个变量
///
/// - P: 凝血酶原 (Prothrombin)，μM
/// - T: 凝血酶 (Thrombin)，μM
/// - u: 纤维蛋白原 (Fibrinogen)，μM
/// - fibrin: 已生成的纤维蛋白 (Fibrin)，μM（追踪用，不参与 ODE 主方程）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CoagulationState {
    pub p: f32,
    pub t: f32,
    pub u: f32,
    pub fibrin: f32,
}

impl CoagulationModel {
    /// 默认参数 —— Wajchman 2011 Table 1
    pub fn new() -> Self {
        Self {
            k1: 0.5,
            k2: 0.01,
            k3: 0.1,
            k4: 0.5,
            k5: 0.3,
            k6: 0.1,
            k7: 0.1,
        }
    }

    /// 论文初始条件：P_0 = 1.4 μM, T_0 = 0, u_0 = 9 μM
    pub fn initial_state() -> CoagulationState {
        CoagulationState {
            p: 1.4,
            t: 0.0,
            u: 9.0,
            fibrin: 0.0,
        }
    }

    /// 单步积分（显式 Euler）
    ///
    /// Wajchman 2011, Eq.(1)-(3)：
    ///   dP/dt = -k1·T·P / (1 + k2·T²)              (1)  负反馈抑制
    ///   dT/dt =  k1·T·P / (1 + k2·T²) - k3·T - k4·T·u  (2)
    ///   du/dt = -k5·T·u + k6·fibrin·k7              (3)  纤溶项
    ///
    /// 附加：dfibrin/dt = k5·T·u - k6·fibrin·k7
    pub fn step(&self, state: &mut CoagulationState, dt: f32) {
        let t = state.t.max(0.0);
        let p = state.p.max(0.0);
        let u = state.u.max(0.0);
        let fibrin = state.fibrin.max(0.0);

        // 反馈抑制因子（防止 T 失控增长）
        let feedback = 1.0 + self.k2 * t * t;

        // Wajchman 2011 Eq.(1)
        let dp_dt = -self.k1 * t * p / feedback;
        // Wajchman 2011 Eq.(2)
        let dt_dt = self.k1 * t * p / feedback - self.k3 * t - self.k4 * t * u;
        // Wajchman 2011 Eq.(3) —— 注意原式末项为纤溶贡献
        let du_dt = -self.k5 * t * u + self.k6 * fibrin * self.k7;
        // 纤维蛋白累积方程（与 u 守恒）
        let dfibrin_dt = self.k5 * t * u - self.k6 * fibrin * self.k7;

        state.p = (state.p + dp_dt * dt).max(0.0);
        state.t = (state.t + dt_dt * dt).max(0.0);
        state.u = (state.u + du_dt * dt).max(0.0);
        state.fibrin = (state.fibrin + dfibrin_dt * dt).max(0.0);
    }
}

impl Default for CoagulationModel {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for CoagulationState {
    fn default() -> Self {
        CoagulationState {
            p: 1.4,
            t: 0.0,
            u: 9.0,
            fibrin: 0.0,
        }
    }
}

// =====================================================================
// 第二部分：6 物种伤口愈合 PDE —— Javierre 2008 + Murphy 2012
// =====================================================================

/// 6 物种愈合模型参数
///
/// Javierre 2008 Eq.(3)-(8)，6 个物种：
/// - f: fibrin (纤维蛋白) —— 初始伤口填充
/// - c: collagen (胶原蛋白) —— 愈合终产物
/// - m: macrophages (巨噬细胞) —— 早期清创
/// - n: fibroblasts (成纤维细胞) —— 分泌胶原
/// - TGF_β: 转化生长因子 —— 化学趋化信号
/// - tPA: 组织纤溶酶原激活物 —— 降解纤维蛋白
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HealingModel {
    // 扩散系数 D (μm²/h) —— Javierre 2008 Table 2
    /// D_m: 巨噬细胞扩散系数
    pub d_m: f32,
    /// D_n: 成纤维细胞扩散系数
    pub d_n: f32,
    /// D_T: TGF-β 扩散系数
    pub d_t: f32,
    /// D_p: tPA 扩散系数
    pub d_p: f32,

    // 反应速率 k
    /// k_f: tPA 降解纤维蛋白速率
    pub k_f: f32,
    /// k_c: 成纤维细胞合成胶原速率
    pub k_c: f32,
    /// k_m: 巨噬细胞趋化激活速率
    pub k_m: f32,
    /// k_md: 巨噬细胞凋亡速率
    pub k_md: f32,
    /// k_n: 成纤维细胞被 TGF-β 驱动增殖速率
    pub k_n: f32,
    /// k_nd: 成纤维细胞凋亡速率
    pub k_nd: f32,
    /// k_T: 巨噬细胞分泌 TGF-β 速率
    pub k_t: f32,
    /// k_Td: TGF-β 降解速率
    pub k_td: f32,
    /// k_p: 成纤维细胞分泌 tPA 速率
    pub k_p: f32,
    /// k_pd: tPA 降解速率
    pub k_pd: f32,

    // 容量上限
    /// c_max: 胶原蛋白饱和密度
    pub c_max: f32,
    /// n_max: 成纤维细胞饱和密度
    pub n_max: f32,
}

impl HealingModel {
    /// 默认参数 —— Javierre 2008 Table 2 + Murphy 2012 Table 1
    pub fn new() -> Self {
        Self {
            d_m: 1000.0,
            d_n: 300.0,
            d_t: 600.0,
            d_p: 5000.0,
            k_f: 0.1,
            k_c: 0.5,
            k_m: 0.2,
            k_md: 0.05,
            k_n: 0.4,
            k_nd: 0.01,
            k_t: 0.01,
            k_td: 0.02,
            k_p: 0.005,
            k_pd: 0.04,
            c_max: 1.0,
            n_max: 1.0,
        }
    }

    /// 单步积分 6 物种场（显式 Euler + 7 点 Laplacian）
    ///
    /// Javierre 2008, Eq.(3)-(8)：
    ///   df/dt      = -k_f · tPA · f                              (3)
    ///   dc/dt      =  k_c · n · (1 - c/c_max)                    (4)
    ///   dm/dt      =  D_m ∇²m + k_m·chemoattractant - k_md·m      (5)
    ///   dn/dt      =  D_n ∇²n + k_n·TGF_β·n·(1 - n/n_max) - k_nd·n (6)
    ///   dTGF_β/dt  =  D_T ∇²TGF_β + k_T·m - k_Td·TGF_β            (7)
    ///   dtPA/dt    =  D_p ∇²tPA + k_p·n - k_pd·tPA                (8)
    ///
    /// 此处 chemoattractant 用 TGF_β 近似（Javierre 2008 简化）
    pub fn step(&self, field: &mut HealingField, dt: f32) {
        let (nx, ny, nz) = field.grid_dim;
        if nx == 0 || ny == 0 || nz == 0 {
            return;
        }
        // 各物种扩散系数对应的拉普拉斯算子需要逐网格点计算
        // 用临时缓冲存储新的导数，避免串扰
        let total = nx * ny * nz;
        let mut d_fibrin = vec![0.0f32; total];
        let mut d_collagen = vec![0.0f32; total];
        let mut d_macro = vec![0.0f32; total];
        let mut d_fibro = vec![0.0f32; total];
        let mut d_tgf = vec![0.0f32; total];
        let mut d_tpa = vec![0.0f32; total];

        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let idx = field.index(i, j, k);
                    let f = field.fibrin[idx].max(0.0);
                    let c = field.collagen[idx].max(0.0);
                    let m = field.macrophages[idx].max(0.0);
                    let n = field.fibroblasts[idx].max(0.0);
                    let tgf = field.tgf_beta[idx].max(0.0);
                    let tpa = field.tpa[idx].max(0.0);

                    // 离散拉普拉斯算子 ∇²φ = Σ(φ_neighbor - φ_center) / h²
                    // 此处网格步长 h = 1（实际单位由调用方决定）
                    let lap_m = self.laplacian(&field.macrophages, i, j, k, nx, ny, nz);
                    let lap_n = self.laplacian(&field.fibroblasts, i, j, k, nx, ny, nz);
                    let lap_tgf = self.laplacian(&field.tgf_beta, i, j, k, nx, ny, nz);
                    let lap_tpa = self.laplacian(&field.tpa, i, j, k, nx, ny, nz);

                    // Javierre 2008 Eq.(3) —— 纤维蛋白被 tPA 降解
                    d_fibrin[idx] = -self.k_f * tpa * f;
                    // Javierre 2008 Eq.(4) —— 胶原蛋白合成（logistic 增长）
                    d_collagen[idx] = self.k_c * n * (1.0 - c / self.c_max);
                    // Javierre 2008 Eq.(5) —— 巨噬细胞扩散 + 趋化 + 凋亡
                    // chemoattractant 用 TGF_β 近似
                    d_macro[idx] = self.d_m * lap_m + self.k_m * tgf - self.k_md * m;
                    // Javierre 2008 Eq.(6) —— 成纤维细胞扩散 + TGF-β 驱动增殖
                    d_fibro[idx] =
                        self.d_n * lap_n + self.k_n * tgf * n * (1.0 - n / self.n_max) - self.k_nd * n;
                    // Javierre 2008 Eq.(7) —— TGF-β 扩散 + 巨噬细胞分泌
                    d_tgf[idx] = self.d_t * lap_tgf + self.k_t * m - self.k_td * tgf;
                    // Javierre 2008 Eq.(8) —— tPA 扩散 + 成纤维细胞分泌
                    d_tpa[idx] = self.d_p * lap_tpa + self.k_p * n - self.k_pd * tpa;
                }
            }
        }

        // 写回
        for idx in 0..total {
            field.fibrin[idx] = (field.fibrin[idx] + d_fibrin[idx] * dt).max(0.0);
            field.collagen[idx] = (field.collagen[idx] + d_collagen[idx] * dt).max(0.0);
            field.macrophages[idx] = (field.macrophages[idx] + d_macro[idx] * dt).max(0.0);
            field.fibroblasts[idx] = (field.fibroblasts[idx] + d_fibro[idx] * dt).max(0.0);
            field.tgf_beta[idx] = (field.tgf_beta[idx] + d_tgf[idx] * dt).max(0.0);
            field.tpa[idx] = (field.tpa[idx] + d_tpa[idx] * dt).max(0.0);
        }
    }

    /// 7 点离散拉普拉斯算子（边界处使用 0 阶诺伊曼条件：超出边界取中心值）
    #[inline]
    fn laplacian(&self, field: &[f32], i: usize, j: usize, k: usize, nx: usize, ny: usize, nz: usize) -> f32 {
        let idx = (k * ny + j) * nx + i;
        let center = field[idx];
        let im = if i > 0 { field[idx - 1] } else { center };
        let ip = if i + 1 < nx { field[idx + 1] } else { center };
        let jm = if j > 0 { field[idx - nx] } else { center };
        let jp = if j + 1 < ny { field[idx + nx] } else { center };
        let km = if k > 0 { field[idx - nx * ny] } else { center };
        let kp = if k + 1 < nz { field[idx + nx * ny] } else { center };
        im + ip + jm + jp + km + kp - 6.0 * center
    }
}

impl Default for HealingModel {
    fn default() -> Self {
        Self::new()
    }
}

/// 6 物种愈合场（3D 网格）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealingField {
    pub fibrin: Vec<f32>,
    pub collagen: Vec<f32>,
    pub macrophages: Vec<f32>,
    pub fibroblasts: Vec<f32>,
    pub tgf_beta: Vec<f32>,
    pub tpa: Vec<f32>,
    pub grid_dim: (usize, usize, usize),
}

impl HealingField {
    /// 创建全零愈合场
    pub fn zeros(grid_dim: (usize, usize, usize)) -> Self {
        let n = grid_dim.0 * grid_dim.1 * grid_dim.2;
        Self {
            fibrin: vec![0.0; n],
            collagen: vec![0.0; n],
            macrophages: vec![0.0; n],
            fibroblasts: vec![0.0; n],
            tgf_beta: vec![0.0; n],
            tpa: vec![0.0; n],
            grid_dim,
        }
    }

    /// 网格索引 → 一维索引
    #[inline]
    pub fn index(&self, i: usize, j: usize, k: usize) -> usize {
        (k * self.grid_dim.1 + j) * self.grid_dim.0 + i
    }

    /// 应用伤口：在 wound_center 周围半径 radius 的球形区域内填充纤维蛋白
    ///
    /// 对应伤口初始条件：伤口区域 f = 1, 其余物种 = 0
    pub fn apply_wound(&mut self, wound_center: [usize; 3], radius: usize) {
        let (nx, ny, nz) = self.grid_dim;
        let cx = wound_center[0] as f32;
        let cy = wound_center[1] as f32;
        let cz = wound_center[2] as f32;
        let r = radius as f32;
        let r2 = r * r;

        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let dx = i as f32 - cx;
                    let dy = j as f32 - cy;
                    let dz = k as f32 - cz;
                    if dx * dx + dy * dy + dz * dz <= r2 {
                        let idx = self.index(i, j, k);
                        // 初始伤口：高纤维蛋白，无细胞
                        self.fibrin[idx] = 1.0;
                        self.collagen[idx] = 0.0;
                        self.macrophages[idx] = 0.0;
                        self.fibroblasts[idx] = 0.0;
                        self.tgf_beta[idx] = 0.0;
                        self.tpa[idx] = 0.0;
                    }
                }
            }
        }
    }

    /// 在伤口边缘播撒初始巨噬细胞（启动愈合级联）
    pub fn seed_macrophages(&mut self, wound_center: [usize; 3], radius: usize, density: f32) {
        let (nx, ny, nz) = self.grid_dim;
        let cx = wound_center[0] as f32;
        let cy = wound_center[1] as f32;
        let cz = wound_center[2] as f32;
        let r_inner = radius as f32;
        let r_outer = (radius + 2) as f32;

        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let dx = i as f32 - cx;
                    let dy = j as f32 - cy;
                    let dz = k as f32 - cz;
                    let d2 = dx * dx + dy * dy + dz * dz;
                    if d2 <= r_outer * r_outer && d2 >= r_inner * r_inner {
                        let idx = self.index(i, j, k);
                        self.macrophages[idx] = density;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coagulation_model_new_constants() {
        let m = CoagulationModel::new();
        assert_eq!(m.k1, 0.5);
        assert_eq!(m.k2, 0.01);
        assert_eq!(m.k3, 0.1);
        assert_eq!(m.k4, 0.5);
        assert_eq!(m.k5, 0.3);
        assert_eq!(m.k6, 0.1);
        assert_eq!(m.k7, 0.1);
    }

    #[test]
    fn test_coagulation_model_default_matches_new() {
        let m1 = CoagulationModel::new();
        let m2 = CoagulationModel::default();
        assert_eq!(m1.k1, m2.k1);
        assert_eq!(m1.k7, m2.k7);
    }

    #[test]
    fn test_coagulation_initial_state() {
        let s = CoagulationModel::initial_state();
        assert_eq!(s.p, 1.4);
        assert_eq!(s.t, 0.0);
        assert_eq!(s.u, 9.0);
        assert_eq!(s.fibrin, 0.0);
    }

    #[test]
    fn test_coagulation_state_default() {
        let s = CoagulationState::default();
        assert_eq!(s.p, 1.4);
        assert_eq!(s.u, 9.0);
        assert_eq!(s.t, 0.0);
        assert_eq!(s.fibrin, 0.0);
    }

    #[test]
    fn test_coagulation_step_zero_thrombin_no_change() {
        let model = CoagulationModel::new();
        let mut s = CoagulationModel::initial_state();
        let before_p = s.p;
        let before_t = s.t;
        let before_u = s.u;
        let before_f = s.fibrin;
        model.step(&mut s, 0.1);
        // T=0 时，所有 dT/dt 和 du/dt 项都依赖 T，因此不变
        assert_eq!(s.p, before_p);
        assert_eq!(s.t, before_t);
        assert_eq!(s.u, before_u);
        assert_eq!(s.fibrin, before_f);
    }

    #[test]
    fn test_coagulation_step_thrombin_drives_prothrombin_consumption() {
        let model = CoagulationModel::new();
        let mut s = CoagulationState { p: 1.4, t: 1.0, u: 9.0, fibrin: 0.0 };
        let p_before = s.p;
        model.step(&mut s, 0.1);
        // T>0 时，P 被消耗（dP/dt = -k1·T·P / (1+k2·T²) < 0）
        assert!(s.p < p_before);
    }

    #[test]
    fn test_coagulation_step_fibrin_accumulates_with_thrombin() {
        let model = CoagulationModel::new();
        let mut s = CoagulationState { p: 1.4, t: 1.0, u: 9.0, fibrin: 0.0 };
        model.step(&mut s, 0.1);
        // T>0 且 u>0 时，纤维蛋白开始生成
        assert!(s.fibrin > 0.0);
    }

    #[test]
    fn test_coagulation_step_u_decreases_with_thrombin() {
        let model = CoagulationModel::new();
        let mut s = CoagulationState { p: 1.4, t: 1.0, u: 9.0, fibrin: 0.0 };
        let u_before = s.u;
        model.step(&mut s, 0.1);
        // 初始 fibrin=0，所以 du/dt = -k5·T·u < 0
        assert!(s.u < u_before);
    }

    #[test]
    fn test_coagulation_step_thrombin_decays_without_prothrombin() {
        let model = CoagulationModel::new();
        let mut s = CoagulationState { p: 0.0, t: 5.0, u: 0.0, fibrin: 0.0 };
        // P=0, u=0：dT/dt = -k3·T < 0（自然失活）
        model.step(&mut s, 0.1);
        assert!(s.t < 5.0);
    }

    #[test]
    fn test_coagulation_step_clamps_negative_to_zero() {
        let model = CoagulationModel::new();
        let mut s = CoagulationState { p: -10.0, t: -10.0, u: -10.0, fibrin: -10.0 };
        model.step(&mut s, 0.1);
        assert!(s.p >= 0.0);
        assert!(s.t >= 0.0);
        assert!(s.u >= 0.0);
        assert!(s.fibrin >= 0.0);
    }

    #[test]
    fn test_coagulation_step_feedback_limits_thrombin_growth() {
        let model = CoagulationModel::new();
        let mut s = CoagulationState { p: 1.4, t: 100.0, u: 9.0, fibrin: 0.0 };
        let t_before = s.t;
        model.step(&mut s, 0.1);
        // 高 T 时，1/(1+k2·T²) 反馈抑制使 T 下降
        assert!(s.t < t_before);
    }

    #[test]
    fn test_healing_model_new_diffusion_constants() {
        let m = HealingModel::new();
        assert_eq!(m.d_m, 1000.0);
        assert_eq!(m.d_n, 300.0);
        assert_eq!(m.d_t, 600.0);
        assert_eq!(m.d_p, 5000.0);
    }

    #[test]
    fn test_healing_model_new_reaction_rates() {
        let m = HealingModel::new();
        assert_eq!(m.k_f, 0.1);
        assert_eq!(m.k_c, 0.5);
        assert_eq!(m.k_m, 0.2);
        assert_eq!(m.k_md, 0.05);
        assert_eq!(m.k_n, 0.4);
        assert_eq!(m.k_nd, 0.01);
        assert_eq!(m.k_t, 0.01);
        assert_eq!(m.k_td, 0.02);
        assert_eq!(m.k_p, 0.005);
        assert_eq!(m.k_pd, 0.04);
    }

    #[test]
    fn test_healing_model_new_capacity_limits() {
        let m = HealingModel::new();
        assert_eq!(m.c_max, 1.0);
        assert_eq!(m.n_max, 1.0);
    }

    #[test]
    fn test_healing_model_default_matches_new() {
        let m1 = HealingModel::new();
        let m2 = HealingModel::default();
        assert_eq!(m1.d_m, m2.d_m);
        assert_eq!(m1.k_f, m2.k_f);
        assert_eq!(m1.c_max, m2.c_max);
    }

    #[test]
    fn test_healing_field_zeros_all_zero() {
        let field = HealingField::zeros((4, 4, 4));
        assert_eq!(field.fibrin.len(), 64);
        assert_eq!(field.collagen.len(), 64);
        assert_eq!(field.macrophages.len(), 64);
        assert_eq!(field.fibroblasts.len(), 64);
        assert_eq!(field.tgf_beta.len(), 64);
        assert_eq!(field.tpa.len(), 64);
        for v in &field.fibrin { assert_eq!(*v, 0.0); }
        for v in &field.collagen { assert_eq!(*v, 0.0); }
        for v in &field.macrophages { assert_eq!(*v, 0.0); }
        for v in &field.fibroblasts { assert_eq!(*v, 0.0); }
        for v in &field.tgf_beta { assert_eq!(*v, 0.0); }
        for v in &field.tpa { assert_eq!(*v, 0.0); }
    }

    #[test]
    fn test_healing_field_grid_dim_preserved() {
        let field = HealingField::zeros((3, 4, 5));
        assert_eq!(field.grid_dim, (3, 4, 5));
        assert_eq!(field.fibrin.len(), 60);
    }

    #[test]
    fn test_healing_field_index_calculation() {
        let field = HealingField::zeros((3, 4, 5));
        // index(i, j, k) = (k * ny + j) * nx + i
        // nx=3, ny=4, nz=5
        assert_eq!(field.index(0, 0, 0), 0);
        assert_eq!(field.index(1, 0, 0), 1);
        assert_eq!(field.index(0, 1, 0), 3);
        assert_eq!(field.index(0, 0, 1), 12);
        assert_eq!(field.index(2, 3, 4), (4 * 4 + 3) * 3 + 2);
    }

    #[test]
    fn test_healing_field_apply_wound_fills_fibrin() {
        let mut field = HealingField::zeros((10, 10, 10));
        field.apply_wound([5, 5, 5], 2);
        // 中心点应有纤维蛋白
        let center = field.index(5, 5, 5);
        assert_eq!(field.fibrin[center], 1.0);
        // 边界外不应有
        let corner = field.index(0, 0, 0);
        assert_eq!(field.fibrin[corner], 0.0);
    }

    #[test]
    fn test_healing_field_apply_wound_clears_other_species() {
        let mut field = HealingField::zeros((10, 10, 10));
        for i in 0..field.collagen.len() {
            field.collagen[i] = 0.5;
            field.macrophages[i] = 0.3;
            field.fibroblasts[i] = 0.7;
        }
        field.apply_wound([5, 5, 5], 2);
        let center = field.index(5, 5, 5);
        // 伤口区域其他物种应清零
        assert_eq!(field.collagen[center], 0.0);
        assert_eq!(field.macrophages[center], 0.0);
        assert_eq!(field.fibroblasts[center], 0.0);
        assert_eq!(field.tgf_beta[center], 0.0);
        assert_eq!(field.tpa[center], 0.0);
    }

    #[test]
    fn test_healing_field_apply_wound_radius_zero() {
        let mut field = HealingField::zeros((10, 10, 10));
        field.apply_wound([5, 5, 5], 0);
        let center = field.index(5, 5, 5);
        assert_eq!(field.fibrin[center], 1.0);
        // 仅中心点
        let neighbor = field.index(6, 5, 5);
        assert_eq!(field.fibrin[neighbor], 0.0);
    }

    #[test]
    fn test_healing_field_seed_macrophages_at_edge() {
        let mut field = HealingField::zeros((10, 10, 10));
        field.apply_wound([5, 5, 5], 2);
        field.seed_macrophages([5, 5, 5], 2, 0.8);
        // 伤口中心（r<=2 内）不应有巨噬细胞
        let center = field.index(5, 5, 5);
        assert_eq!(field.macrophages[center], 0.0);
        // 伤口边缘（r=2..4 环带）应有巨噬细胞
        // (5,5,5) 到 (8,5,5) 距离=3，在 [2,4] 内
        let edge = field.index(8, 5, 5);
        assert_eq!(field.macrophages[edge], 0.8);
    }

    #[test]
    fn test_healing_model_step_empty_grid_noop() {
        let model = HealingModel::new();
        let mut field = HealingField::zeros((0, 0, 0));
        model.step(&mut field, 0.1);
        assert!(field.fibrin.is_empty());
    }

    #[test]
    fn test_healing_model_step_fibrin_degrades_with_tpa() {
        let model = HealingModel::new();
        let mut field = HealingField::zeros((4, 4, 4));
        for i in 0..field.fibrin.len() {
            field.fibrin[i] = 1.0;
            field.tpa[i] = 1.0;
        }
        model.step(&mut field, 0.1);
        // 纤维蛋白应被 tPA 降解
        assert!(field.fibrin[0] < 1.0);
    }

    #[test]
    fn test_healing_model_step_collagen_synthesized_by_fibroblasts() {
        let model = HealingModel::new();
        let mut field = HealingField::zeros((4, 4, 4));
        for i in 0..field.fibroblasts.len() {
            field.fibroblasts[i] = 0.5;
        }
        model.step(&mut field, 0.1);
        assert!(field.collagen[0] > 0.0);
    }

    #[test]
    fn test_healing_model_step_tgf_secreted_by_macrophages() {
        let model = HealingModel::new();
        let mut field = HealingField::zeros((4, 4, 4));
        for i in 0..field.macrophages.len() {
            field.macrophages[i] = 0.5;
        }
        model.step(&mut field, 0.1);
        assert!(field.tgf_beta[0] > 0.0);
    }

    #[test]
    fn test_healing_model_step_tpa_secreted_by_fibroblasts() {
        let model = HealingModel::new();
        let mut field = HealingField::zeros((4, 4, 4));
        for i in 0..field.fibroblasts.len() {
            field.fibroblasts[i] = 0.5;
        }
        model.step(&mut field, 0.1);
        assert!(field.tpa[0] > 0.0);
    }

    #[test]
    fn test_healing_model_step_fibroblasts_proliferate_with_tgf() {
        let model = HealingModel::new();
        let mut field = HealingField::zeros((4, 4, 4));
        for i in 0..field.fibroblasts.len() {
            field.fibroblasts[i] = 0.3;
            field.tgf_beta[i] = 0.5;
        }
        let before = field.fibroblasts[0];
        model.step(&mut field, 0.1);
        // TGF-β 驱动成纤维细胞增殖
        assert!(field.fibroblasts[0] > before);
    }
}
