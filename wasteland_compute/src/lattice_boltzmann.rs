//! Lattice Boltzmann Method (D2Q9, BGK collision)
//!
//! 介观流体模拟方法, 介于宏观 Navier-Stokes 与微观分子动力学之间.
//! 通过恢复 Navier-Stokes 方程在宏观极限下的行为来模拟流体.
//!
//! D2Q9 模型 (2 维, 9 速度):
//!   c_0 = (0,0)     w_0 = 4/9
//!   c_1 = (1,0)     c_2 = (0,1)     c_3 = (-1,0)    c_4 = (0,-1)   w = 1/9
//!   c_5 = (1,1)     c_6 = (-1,1)    c_7 = (-1,-1)   c_8 = (1,-1)   w = 1/36
//!
//! 声速平方: c_s² = 1/3
//!
//! 平衡分布函数:
//!   f_i^eq = w_i · ρ · [1 + (c_i·u)/c_s² + (c_i·u)²/(2·c_s⁴) - |u|²/(2·c_s²)]
//!
//! BGK 碰撞 (单弛豫时间):
//!   f_i* = f_i - (f_i - f_i^eq) / τ
//!
//! 流动:
//!   f_i(x + c_i·Δt, t + Δt) = f_i*(x, t)
//!
//! 宏观量恢复:
//!   ρ = Σ_i f_i
//!   ρ·u = Σ_i c_i · f_i
//!
//! 运动粘度: ν = c_s² · (τ - 0.5) · Δt
//! 稳定性要求: τ > 0.5
//!
//! 应用: 多孔介质流, 复杂边界绕流, 多相流, 热-流耦合, CFD, GPU 并行.
//!
//! 基于 Qian et al. 1992 (BGK), Zou & He 1997 (边界条件).

use serde::{Deserialize, Serialize};

/// D2Q9 速度向量 (格子单位)
pub const D2Q9_VEL: [[i32; 2]; 9] = [
    [0, 0],
    [1, 0],
    [0, 1],
    [-1, 0],
    [0, -1],
    [1, 1],
    [-1, 1],
    [-1, -1],
    [1, -1],
];

/// D2Q9 权重
pub const D2Q9_W: [f32; 9] = [
    4.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
];

/// 声速平方 c_s²
pub const CS2: f32 = 1.0 / 3.0;

/// 反方向索引 (bounce-back 用)
pub const OPP: [usize; 9] = [0, 3, 4, 1, 2, 7, 8, 5, 6];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum BoundaryType {
    /// 反弹边界 (无滑移壁面)
    BounceBack,
    /// 周期性边界
    Periodic,
    /// Zou-He 速度边界 (Dirichlet 速度)
    Velocity { ux: f32, uy: f32 },
    /// Zou-He 压力边界 (Dirichlet 密度)
    Pressure { rho: f32 },
}

impl BoundaryType {
    pub fn is_periodic(&self) -> bool {
        matches!(self, BoundaryType::Periodic)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LbmConfig {
    pub nx: usize,
    pub ny: usize,
    /// 弛豫时间 τ (必须 > 0.5 保证稳定性)
    pub tau: f32,
    pub boundary_north: BoundaryType,
    pub boundary_south: BoundaryType,
    pub boundary_east: BoundaryType,
    pub boundary_west: BoundaryType,
    /// 体积力 (格子单位, Guo 格式)
    pub force_x: f32,
    pub force_y: f32,
}

impl Default for LbmConfig {
    fn default() -> Self {
        LbmConfig {
            nx: 32,
            ny: 32,
            tau: 0.6,
            boundary_north: BoundaryType::BounceBack,
            boundary_south: BoundaryType::BounceBack,
            boundary_east: BoundaryType::BounceBack,
            boundary_west: BoundaryType::BounceBack,
            force_x: 0.0,
            force_y: 0.0,
        }
    }
}

impl LbmConfig {
    /// 运动粘度 ν = c_s² · (τ - 0.5)
    pub fn viscosity(&self) -> f32 {
        CS2 * (self.tau - 0.5)
    }
    /// Reynolds 数 Re = U · L / ν
    pub fn reynolds(&self, velocity: f32, length: f32) -> f32 {
        let nu = self.viscosity();
        if nu.abs() < 1e-12 {
            0.0
        } else {
            velocity * length / nu
        }
    }
}

pub struct LbmSolver {
    pub config: LbmConfig,
    /// 分布函数 f_i (9 个方向, 行主序)
    pub f: Vec<f32>,
    pub f_temp: Vec<f32>,
    pub rho: Vec<f32>,
    pub ux: Vec<f32>,
    pub uy: Vec<f32>,
    /// 固体障碍标记 (反弹)
    pub solid: Vec<bool>,
    pub time: f32,
    pub steps: usize,
}

impl LbmSolver {
    pub fn new(config: LbmConfig) -> Self {
        let n = config.nx * config.ny;
        let mut s = LbmSolver {
            config,
            f: vec![0.0; 9 * n],
            f_temp: vec![0.0; 9 * n],
            rho: vec![1.0; n],
            ux: vec![0.0; n],
            uy: vec![0.0; n],
            solid: vec![false; n],
            time: 0.0,
            steps: 0,
        };
        s.initialize_uniform(1.0, 0.0, 0.0);
        s
    }

    pub fn idx(&self, x: usize, y: usize) -> usize {
        debug_assert!(x < self.config.nx);
        debug_assert!(y < self.config.ny);
        x + self.config.nx * y
    }

    fn fi_idx(&self, x: usize, y: usize, i: usize) -> usize {
        i + 9 * self.idx(x, y)
    }

    /// 平衡分布函数
    pub fn equilibrium(rho: f32, ux: f32, uy: f32) -> [f32; 9] {
        let u2 = ux * ux + uy * uy;
        let inv_cs2 = 1.0 / CS2;
        let inv_cs4 = inv_cs2 * inv_cs2;
        let mut feq = [0.0f32; 9];
        for i in 0..9 {
            let cu = (D2Q9_VEL[i][0] as f32) * ux + (D2Q9_VEL[i][1] as f32) * uy;
            feq[i] = D2Q9_W[i]
                * rho
                * (1.0 + cu * inv_cs2 + 0.5 * cu * cu * inv_cs4 - 0.5 * u2 * inv_cs2);
        }
        feq
    }

    /// 初始化为均匀流
    pub fn initialize_uniform(&mut self, rho: f32, ux: f32, uy: f32) {
        let feq = Self::equilibrium(rho, ux, uy);
        for cell in 0..self.config.nx * self.config.ny {
            self.rho[cell] = rho;
            self.ux[cell] = ux;
            self.uy[cell] = uy;
            for i in 0..9 {
                self.f[9 * cell + i] = feq[i];
            }
        }
    }

    /// 从分布函数计算宏观量
    pub fn compute_macros(&mut self) {
        for y in 0..self.config.ny {
            for x in 0..self.config.nx {
                let cell = self.idx(x, y);
                let mut rho = 0.0;
                let mut ux = 0.0;
                let mut uy = 0.0;
                for i in 0..9 {
                    let fi = self.f[9 * cell + i];
                    rho += fi;
                    ux += (D2Q9_VEL[i][0] as f32) * fi;
                    uy += (D2Q9_VEL[i][1] as f32) * fi;
                }
                if rho > 1e-12 {
                    ux /= rho;
                    uy /= rho;
                } else {
                    ux = 0.0;
                    uy = 0.0;
                }
                self.rho[cell] = rho;
                self.ux[cell] = ux;
                self.uy[cell] = uy;
            }
        }
    }

    /// BGK 碰撞步 (含 Guo 体积力)
    pub fn collide(&mut self) {
        let tau = self.config.tau;
        let fx = self.config.force_x;
        let fy = self.config.force_y;
        for y in 0..self.config.ny {
            for x in 0..self.config.nx {
                let cell = self.idx(x, y);
                if self.solid[cell] {
                    continue;
                }
                let rho = self.rho[cell];
                let ux = self.ux[cell];
                let uy = self.uy[cell];
                let feq = Self::equilibrium(rho, ux, uy);
                // Guo 体力项: F_i = w_i * (1 - 1/(2τ)) * [3*(c_i - u) + 9*(c_i·u)*c_i]
                for i in 0..9 {
                    let ci0 = D2Q9_VEL[i][0] as f32;
                    let ci1 = D2Q9_VEL[i][1] as f32;
                    let cu = ci0 * ux + ci1 * uy;
                    let force_term = D2Q9_W[i] * (1.0 - 0.5 / tau)
                        * (3.0 * (ci0 * fx + ci1 * fy - cu * 0.0)
                            + 9.0 * cu * (ci0 * fx + ci1 * fy));
                    let _ = force_term; // 简化: 此处用速度偏移替代
                    self.f[9 * cell + i] = self.f[9 * cell + i]
                        - (self.f[9 * cell + i] - feq[i]) / tau;
                }
            }
        }
    }

    /// 流动步: f_i(x + c_i, t+1) = f_i*(x, t)
    pub fn stream(&mut self) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        let periodic_x = self.config.boundary_west.is_periodic();
        let periodic_y = self.config.boundary_south.is_periodic();
        for y in 0..ny {
            for x in 0..nx {
                for i in 0..9 {
                    let dx = D2Q9_VEL[i][0];
                    let dy = D2Q9_VEL[i][1];
                    let mut nxp = x as i32 + dx;
                    let mut nyp = y as i32 + dy;
                    if nxp < 0 {
                        nxp = if periodic_x { (nx as i32 - 1) } else { 0 };
                    }
                    if nxp >= nx as i32 {
                        nxp = if periodic_x { 0 } else { (nx as i32 - 1) };
                    }
                    if nyp < 0 {
                        nyp = if periodic_y { (ny as i32 - 1) } else { 0 };
                    }
                    if nyp >= ny as i32 {
                        nyp = if periodic_y { 0 } else { (ny as i32 - 1) };
                    }
                    let src = 9 * self.idx(x, y) + i;
                    let dst = 9 * self.idx(nxp as usize, nyp as usize) + i;
                    self.f_temp[dst] = self.f[src];
                }
            }
        }
        std::mem::swap(&mut self.f, &mut self.f_temp);
    }

    /// 应用固体反弹 (无滑移)
    pub fn apply_bounce_back(&mut self) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        for y in 0..ny {
            for x in 0..nx {
                let cell = self.idx(x, y);
                if self.solid[cell] {
                    for i in 0..9 {
                        self.f[9 * cell + i] = self.f_temp[9 * cell + OPP[i]];
                    }
                }
            }
        }
    }

    /// 应用边界条件 (西/东/南/北)
    pub fn apply_boundary(&mut self) {
        self.apply_west_boundary();
        self.apply_east_boundary();
        self.apply_south_boundary();
        self.apply_north_boundary();
    }

    fn apply_west_boundary(&mut self) {
        let ny = self.config.ny;
        match self.config.boundary_west {
            BoundaryType::Periodic => { /* 流动步已处理 */ }
            BoundaryType::BounceBack => {
                for y in 0..ny {
                    let cell = self.idx(0, y);
                    for i in 0..9 {
                        self.f[9 * cell + i] = self.f_temp[9 * cell + OPP[i]];
                    }
                }
            }
            BoundaryType::Velocity { ux, uy } => {
                for y in 0..ny {
                    self.zou_he_velocity_west(y, ux, uy);
                }
            }
            BoundaryType::Pressure { rho } => {
                for y in 0..ny {
                    self.zou_he_pressure_west(y, rho);
                }
            }
        }
    }

    fn apply_east_boundary(&mut self) {
        let ny = self.config.ny;
        let nx = self.config.nx;
        match self.config.boundary_east {
            BoundaryType::Periodic => {}
            BoundaryType::BounceBack => {
                for y in 0..ny {
                    let cell = self.idx(nx - 1, y);
                    for i in 0..9 {
                        self.f[9 * cell + i] = self.f_temp[9 * cell + OPP[i]];
                    }
                }
            }
            BoundaryType::Velocity { ux, uy } => {
                for y in 0..ny {
                    self.zou_he_velocity_east(y, ux, uy);
                }
            }
            BoundaryType::Pressure { rho } => {
                for y in 0..ny {
                    self.zou_he_pressure_east(y, rho);
                }
            }
        }
    }

    fn apply_south_boundary(&mut self) {
        let nx = self.config.nx;
        match self.config.boundary_south {
            BoundaryType::Periodic => {}
            BoundaryType::BounceBack => {
                for x in 0..nx {
                    let cell = self.idx(x, 0);
                    for i in 0..9 {
                        self.f[9 * cell + i] = self.f_temp[9 * cell + OPP[i]];
                    }
                }
            }
            BoundaryType::Velocity { ux, uy } => {
                for x in 0..nx {
                    self.zou_he_velocity_south(x, ux, uy);
                }
            }
            BoundaryType::Pressure { rho } => {
                for x in 0..nx {
                    self.zou_he_pressure_south(x, rho);
                }
            }
        }
    }

    fn apply_north_boundary(&mut self) {
        let nx = self.config.nx;
        let ny = self.config.ny;
        match self.config.boundary_north {
            BoundaryType::Periodic => {}
            BoundaryType::BounceBack => {
                for x in 0..nx {
                    let cell = self.idx(x, ny - 1);
                    for i in 0..9 {
                        self.f[9 * cell + i] = self.f_temp[9 * cell + OPP[i]];
                    }
                }
            }
            BoundaryType::Velocity { ux, uy } => {
                for x in 0..nx {
                    self.zou_he_velocity_north(x, ux, uy);
                }
            }
            BoundaryType::Pressure { rho } => {
                for x in 0..nx {
                    self.zou_he_pressure_north(x, rho);
                }
            }
        }
    }

    // Zou-He 西边界 (入口, 已知速度)
    fn zou_he_velocity_west(&mut self, y: usize, ux: f32, uy: f32) {
        let cell = self.idx(0, y);
        let rho = (self.f[9 * cell + 0] + self.f[9 * cell + 2] + self.f[9 * cell + 4]
            + 2.0 * (self.f[9 * cell + 3] + self.f[9 * cell + 6] + self.f[9 * cell + 7]))
            / (1.0 - ux);
        self.f[9 * cell + 1] = self.f[9 * cell + 3] + (2.0 / 3.0) * rho * ux;
        self.f[9 * cell + 5] = self.f[9 * cell + 7]
            + 0.5 * (self.f[9 * cell + 2] - self.f[9 * cell + 4])
            + 0.5 * rho * uy
            + (1.0 / 6.0) * rho * ux;
        self.f[9 * cell + 8] = self.f[9 * cell + 6]
            + 0.5 * (self.f[9 * cell + 4] - self.f[9 * cell + 2])
            - 0.5 * rho * uy
            + (1.0 / 6.0) * rho * ux;
    }

    fn zou_he_velocity_east(&mut self, y: usize, ux: f32, uy: f32) {
        let nx = self.config.nx;
        let cell = self.idx(nx - 1, y);
        let rho = (self.f[9 * cell + 0] + self.f[9 * cell + 2] + self.f[9 * cell + 4]
            + 2.0 * (self.f[9 * cell + 1] + self.f[9 * cell + 5] + self.f[9 * cell + 8]))
            / (1.0 + ux);
        self.f[9 * cell + 3] = self.f[9 * cell + 1] - (2.0 / 3.0) * rho * ux;
        self.f[9 * cell + 7] = self.f[9 * cell + 5]
            + 0.5 * (self.f[9 * cell + 4] - self.f[9 * cell + 2])
            - 0.5 * rho * uy
            - (1.0 / 6.0) * rho * ux;
        self.f[9 * cell + 6] = self.f[9 * cell + 8]
            + 0.5 * (self.f[9 * cell + 2] - self.f[9 * cell + 4])
            + 0.5 * rho * uy
            - (1.0 / 6.0) * rho * ux;
    }

    fn zou_he_velocity_south(&mut self, x: usize, ux: f32, uy: f32) {
        let cell = self.idx(x, 0);
        let rho = (self.f[9 * cell + 0] + self.f[9 * cell + 1] + self.f[9 * cell + 3]
            + 2.0 * (self.f[9 * cell + 4] + self.f[9 * cell + 7] + self.f[9 * cell + 8]))
            / (1.0 - uy);
        self.f[9 * cell + 2] = self.f[9 * cell + 4] + (2.0 / 3.0) * rho * uy;
        self.f[9 * cell + 5] = self.f[9 * cell + 7]
            + 0.5 * (self.f[9 * cell + 1] - self.f[9 * cell + 3])
            + 0.5 * rho * ux
            + (1.0 / 6.0) * rho * uy;
        self.f[9 * cell + 6] = self.f[9 * cell + 8]
            + 0.5 * (self.f[9 * cell + 3] - self.f[9 * cell + 1])
            - 0.5 * rho * ux
            + (1.0 / 6.0) * rho * uy;
    }

    fn zou_he_velocity_north(&mut self, x: usize, ux: f32, uy: f32) {
        let ny = self.config.ny;
        let cell = self.idx(x, ny - 1);
        let rho = (self.f[9 * cell + 0] + self.f[9 * cell + 1] + self.f[9 * cell + 3]
            + 2.0 * (self.f[9 * cell + 2] + self.f[9 * cell + 5] + self.f[9 * cell + 6]))
            / (1.0 + uy);
        self.f[9 * cell + 4] = self.f[9 * cell + 2] - (2.0 / 3.0) * rho * uy;
        self.f[9 * cell + 8] = self.f[9 * cell + 6]
            + 0.5 * (self.f[9 * cell + 3] - self.f[9 * cell + 1])
            - 0.5 * rho * ux
            - (1.0 / 6.0) * rho * uy;
        self.f[9 * cell + 7] = self.f[9 * cell + 5]
            + 0.5 * (self.f[9 * cell + 1] - self.f[9 * cell + 3])
            + 0.5 * rho * ux
            - (1.0 / 6.0) * rho * uy;
    }

    fn zou_he_pressure_west(&mut self, y: usize, rho: f32) {
        let cell = self.idx(0, y);
        let ux = -1.0 + (self.f[9 * cell + 0] + self.f[9 * cell + 2] + self.f[9 * cell + 4]
            + 2.0 * (self.f[9 * cell + 3] + self.f[9 * cell + 6] + self.f[9 * cell + 7]))
            / rho;
        let uy = 0.0;
        let feq = Self::equilibrium(rho, ux, uy);
        self.f[9 * cell + 1] = feq[1] + self.f[9 * cell + 3] - feq[3];
        self.f[9 * cell + 5] = feq[5] + self.f[9 * cell + 7] - feq[7];
        self.f[9 * cell + 8] = feq[8] + self.f[9 * cell + 6] - feq[6];
    }

    fn zou_he_pressure_east(&mut self, y: usize, rho: f32) {
        let nx = self.config.nx;
        let cell = self.idx(nx - 1, y);
        let ux = -1.0 + (self.f[9 * cell + 0] + self.f[9 * cell + 2] + self.f[9 * cell + 4]
            + 2.0 * (self.f[9 * cell + 1] + self.f[9 * cell + 5] + self.f[9 * cell + 8]))
            / rho;
        let ux = -ux;
        let uy = 0.0;
        let feq = Self::equilibrium(rho, ux, uy);
        self.f[9 * cell + 3] = feq[3] + self.f[9 * cell + 1] - feq[1];
        self.f[9 * cell + 7] = feq[7] + self.f[9 * cell + 5] - feq[5];
        self.f[9 * cell + 6] = feq[6] + self.f[9 * cell + 8] - feq[8];
    }

    fn zou_he_pressure_south(&mut self, x: usize, rho: f32) {
        let cell = self.idx(x, 0);
        let uy = -1.0 + (self.f[9 * cell + 0] + self.f[9 * cell + 1] + self.f[9 * cell + 3]
            + 2.0 * (self.f[9 * cell + 4] + self.f[9 * cell + 7] + self.f[9 * cell + 8]))
            / rho;
        let ux = 0.0;
        let feq = Self::equilibrium(rho, ux, uy);
        self.f[9 * cell + 2] = feq[2] + self.f[9 * cell + 4] - feq[4];
        self.f[9 * cell + 5] = feq[5] + self.f[9 * cell + 7] - feq[7];
        self.f[9 * cell + 6] = feq[6] + self.f[9 * cell + 8] - feq[8];
    }

    fn zou_he_pressure_north(&mut self, x: usize, rho: f32) {
        let ny = self.config.ny;
        let cell = self.idx(x, ny - 1);
        let uy = -1.0 + (self.f[9 * cell + 0] + self.f[9 * cell + 1] + self.f[9 * cell + 3]
            + 2.0 * (self.f[9 * cell + 2] + self.f[9 * cell + 5] + self.f[9 * cell + 6]))
            / rho;
        let uy = -uy;
        let ux = 0.0;
        let feq = Self::equilibrium(rho, ux, uy);
        self.f[9 * cell + 4] = feq[4] + self.f[9 * cell + 2] - feq[2];
        self.f[9 * cell + 8] = feq[8] + self.f[9 * cell + 6] - feq[6];
        self.f[9 * cell + 7] = feq[7] + self.f[9 * cell + 5] - feq[5];
    }

    /// 应用体积力 (Guo 格式, 简化为速度修正)
    pub fn apply_force(&mut self) {
        if self.config.force_x == 0.0 && self.config.force_y == 0.0 {
            return;
        }
        let half_tau = 0.5 / self.config.tau;
        for cell in 0..self.config.nx * self.config.ny {
            if self.solid[cell] {
                continue;
            }
            let rho = self.rho[cell];
            let ux = self.ux[cell];
            let uy = self.uy[cell];
            let fx = self.config.force_x;
            let fy = self.config.force_y;
            for i in 0..9 {
                let ci0 = D2Q9_VEL[i][0] as f32;
                let ci1 = D2Q9_VEL[i][1] as f32;
                let cu = ci0 * ux + ci1 * uy;
                let force_i = D2Q9_W[i] * (1.0 - half_tau)
                    * (3.0 * (ci0 - ux) * fx + 3.0 * (ci1 - uy) * fy
                        + 9.0 * cu * (ci0 * fx + ci1 * fy));
                self.f[9 * cell + i] += force_i;
            }
            // 修正宏观速度
            self.ux[cell] += fx / rho;
            self.uy[cell] += fy / rho;
        }
    }

    /// 完整一步: 宏观 → 力 → 碰撞 → 流动 → 边界 → 宏观
    pub fn step(&mut self) {
        self.compute_macros();
        self.apply_force();
        self.collide();
        // 保存碰撞后状态供反弹使用
        self.f_temp.copy_from_slice(&self.f);
        self.stream();
        self.apply_boundary();
        self.apply_bounce_back();
        self.compute_macros();
        self.time += 1.0;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    pub fn set_solid(&mut self, x: usize, y: usize) {
        let idx = self.idx(x, y);
        self.solid[idx] = true;
    }

    pub fn clear_solid(&mut self, x: usize, y: usize) {
        let idx = self.idx(x, y);
        self.solid[idx] = false;
    }

    pub fn set_velocity(&mut self, x: usize, y: usize, ux: f32, uy: f32) {
        let cell = self.idx(x, y);
        let rho = self.rho[cell];
        let feq = Self::equilibrium(rho, ux, uy);
        for i in 0..9 {
            self.f[9 * cell + i] = feq[i];
        }
        self.ux[cell] = ux;
        self.uy[cell] = uy;
    }

    pub fn viscosity(&self) -> f32 {
        self.config.viscosity()
    }

    pub fn max_velocity(&self) -> f32 {
        let mut m = 0.0_f32;
        for i in 0..self.ux.len() {
            let v = (self.ux[i] * self.ux[i] + self.uy[i] * self.uy[i]).sqrt();
            if v > m {
                m = v;
            }
        }
        m
    }

    pub fn average_density(&self) -> f32 {
        let mut s = 0.0;
        for r in &self.rho {
            s += r;
        }
        s / self.rho.len() as f32
    }

    pub fn total_mass(&self) -> f32 {
        let mut s = 0.0;
        for cell in 0..self.config.nx * self.config.ny {
            if !self.solid[cell] {
                s += self.rho[cell];
            }
        }
        s
    }

    pub fn average_velocity_x(&self) -> f32 {
        let mut s = 0.0;
        let mut n = 0;
        for cell in 0..self.config.nx * self.config.ny {
            if !self.solid[cell] {
                s += self.ux[cell];
                n += 1;
            }
        }
        if n > 0 { s / n as f32 } else { 0.0 }
    }

    pub fn reset(&mut self) {
        self.initialize_uniform(1.0, 0.0, 0.0);
        for s in self.solid.iter_mut() {
            *s = false;
        }
        self.time = 0.0;
        self.steps = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    fn default_config(nx: usize, ny: usize) -> LbmConfig {
        LbmConfig {
            nx,
            ny,
            tau: 0.6,
            boundary_north: BoundaryType::BounceBack,
            boundary_south: BoundaryType::BounceBack,
            boundary_east: BoundaryType::BounceBack,
            boundary_west: BoundaryType::BounceBack,
            force_x: 0.0,
            force_y: 0.0,
        }
    }

    fn periodic_config(nx: usize, ny: usize) -> LbmConfig {
        LbmConfig {
            nx,
            ny,
            tau: 0.6,
            boundary_north: BoundaryType::Periodic,
            boundary_south: BoundaryType::Periodic,
            boundary_east: BoundaryType::Periodic,
            boundary_west: BoundaryType::Periodic,
            force_x: 0.0,
            force_y: 0.0,
        }
    }

    // ============ 常量测试 ============

    #[test]
    fn test_d2q9_vel_count() {
        assert_eq!(D2Q9_VEL.len(), 9);
        for v in D2Q9_VEL.iter() {
            assert_eq!(v.len(), 2);
        }
    }

    #[test]
    fn test_d2q9_vel_values() {
        assert_eq!(D2Q9_VEL[0], [0, 0]);
        assert_eq!(D2Q9_VEL[1], [1, 0]);
        assert_eq!(D2Q9_VEL[2], [0, 1]);
        assert_eq!(D2Q9_VEL[3], [-1, 0]);
        assert_eq!(D2Q9_VEL[4], [0, -1]);
        assert_eq!(D2Q9_VEL[5], [1, 1]);
        assert_eq!(D2Q9_VEL[6], [-1, 1]);
        assert_eq!(D2Q9_VEL[7], [-1, -1]);
        assert_eq!(D2Q9_VEL[8], [1, -1]);
    }

    #[test]
    fn test_d2q9_weights_sum() {
        let sum: f32 = D2Q9_W.iter().sum();
        assert!(approx_eq(sum, 1.0, 1e-6), "weights sum = {}", sum);
    }

    #[test]
    fn test_d2q9_weights_values() {
        assert!(approx_eq(D2Q9_W[0], 4.0 / 9.0, 1e-6));
        for i in 1..5 {
            assert!(approx_eq(D2Q9_W[i], 1.0 / 9.0, 1e-6));
        }
        for i in 5..9 {
            assert!(approx_eq(D2Q9_W[i], 1.0 / 36.0, 1e-6));
        }
    }

    #[test]
    fn test_cs2_value() {
        assert!(approx_eq(CS2, 1.0 / 3.0, 1e-6));
    }

    #[test]
    fn test_opp_indices_correct() {
        for i in 0..9 {
            let opp = OPP[i];
            assert_eq!(D2Q9_VEL[opp][0], -D2Q9_VEL[i][0]);
            assert_eq!(D2Q9_VEL[opp][1], -D2Q9_VEL[i][1]);
        }
    }

    #[test]
    fn test_opp_symmetry() {
        for i in 0..9 {
            assert_eq!(OPP[OPP[i]], i);
        }
    }

    // ============ BoundaryType 测试 ============

    #[test]
    fn test_boundary_type_periodic_check() {
        assert!(BoundaryType::Periodic.is_periodic());
        assert!(!BoundaryType::BounceBack.is_periodic());
        assert!(!BoundaryType::Velocity { ux: 0.0, uy: 0.0 }.is_periodic());
        assert!(!BoundaryType::Pressure { rho: 1.0 }.is_periodic());
    }

    #[test]
    fn test_boundary_type_equality() {
        assert_eq!(BoundaryType::BounceBack, BoundaryType::BounceBack);
        assert_eq!(
            BoundaryType::Velocity { ux: 0.1, uy: 0.2 },
            BoundaryType::Velocity { ux: 0.1, uy: 0.2 }
        );
        assert_ne!(
            BoundaryType::Velocity { ux: 0.1, uy: 0.0 },
            BoundaryType::Velocity { ux: 0.2, uy: 0.0 }
        );
        assert_eq!(
            BoundaryType::Pressure { rho: 1.0 },
            BoundaryType::Pressure { rho: 1.0 }
        );
    }

    // ============ LbmConfig 测试 ============

    #[test]
    fn test_config_default() {
        let c = LbmConfig::default();
        assert_eq!(c.nx, 32);
        assert_eq!(c.ny, 32);
        assert!(approx_eq(c.tau, 0.6, 1e-6));
        assert_eq!(c.boundary_north, BoundaryType::BounceBack);
        assert_eq!(c.boundary_south, BoundaryType::BounceBack);
        assert_eq!(c.boundary_east, BoundaryType::BounceBack);
        assert_eq!(c.boundary_west, BoundaryType::BounceBack);
        assert_eq!(c.force_x, 0.0);
        assert_eq!(c.force_y, 0.0);
    }

    #[test]
    fn test_config_viscosity() {
        let c = LbmConfig { tau: 0.6, ..Default::default() };
        // nu = (1/3) * (0.6 - 0.5) = 1/30
        assert!(approx_eq(c.viscosity(), 1.0 / 30.0, 1e-6));
    }

    #[test]
    fn test_config_viscosity_tau_half() {
        let c = LbmConfig { tau: 0.5, ..Default::default() };
        assert!(approx_eq(c.viscosity(), 0.0, 1e-6));
    }

    #[test]
    fn test_config_reynolds() {
        let c = LbmConfig { tau: 0.6, ..Default::default() };
        let nu = c.viscosity();
        let re = c.reynolds(0.1, 10.0);
        assert!(approx_eq(re, 0.1 * 10.0 / nu, 1e-4));
    }

    #[test]
    fn test_config_reynolds_zero_viscosity() {
        let c = LbmConfig { tau: 0.5, ..Default::default() };
        assert_eq!(c.reynolds(1.0, 1.0), 0.0);
    }

    // ============ LbmSolver 创建测试 ============

    #[test]
    fn test_solver_new_dimensions() {
        let s = LbmSolver::new(LbmConfig { nx: 16, ny: 8, tau: 0.6, ..Default::default() });
        let n = 16 * 8;
        assert_eq!(s.f.len(), 9 * n);
        assert_eq!(s.f_temp.len(), 9 * n);
        assert_eq!(s.rho.len(), n);
        assert_eq!(s.ux.len(), n);
        assert_eq!(s.uy.len(), n);
        assert_eq!(s.solid.len(), n);
    }

    #[test]
    fn test_solver_new_initialized() {
        let s = LbmSolver::new(LbmConfig::default());
        for r in &s.rho {
            assert!(approx_eq(*r, 1.0, 1e-6));
        }
        for u in &s.ux {
            assert!(approx_eq(*u, 0.0, 1e-6));
        }
        for u in &s.uy {
            assert!(approx_eq(*u, 0.0, 1e-6));
        }
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
    }

    #[test]
    fn test_solver_idx() {
        let s = LbmSolver::new(LbmConfig { nx: 10, ny: 5, tau: 0.6, ..Default::default() });
        assert_eq!(s.idx(0, 0), 0);
        assert_eq!(s.idx(9, 0), 9);
        assert_eq!(s.idx(0, 1), 10);
        assert_eq!(s.idx(3, 2), 23);
    }

    // ============ equilibrium 测试 ============

    #[test]
    fn test_equilibrium_mass_conservation_stationary() {
        let rho = 1.5;
        let feq = LbmSolver::equilibrium(rho, 0.0, 0.0);
        let sum: f32 = feq.iter().sum();
        assert!(approx_eq(sum, rho, 1e-5));
    }

    #[test]
    fn test_equilibrium_mass_conservation_moving() {
        let rho = 2.0;
        let feq = LbmSolver::equilibrium(rho, 0.1, 0.05);
        let sum: f32 = feq.iter().sum();
        assert!(approx_eq(sum, rho, 1e-5));
    }

    #[test]
    fn test_equilibrium_momentum_conservation() {
        let rho = 1.0;
        let ux = 0.1;
        let uy = 0.05;
        let feq = LbmSolver::equilibrium(rho, ux, uy);
        let mut px = 0.0;
        let mut py = 0.0;
        for i in 0..9 {
            px += (D2Q9_VEL[i][0] as f32) * feq[i];
            py += (D2Q9_VEL[i][1] as f32) * feq[i];
        }
        assert!(approx_eq(px, rho * ux, 1e-5));
        assert!(approx_eq(py, rho * uy, 1e-5));
    }

    #[test]
    fn test_equilibrium_stationary_weights() {
        let rho = 1.0;
        let feq = LbmSolver::equilibrium(rho, 0.0, 0.0);
        for i in 0..9 {
            assert!(approx_eq(feq[i], D2Q9_W[i] * rho, 1e-6));
        }
    }

    // ============ initialize_uniform 测试 ============

    #[test]
    fn test_initialize_uniform() {
        let mut s = LbmSolver::new(default_config(8, 8));
        s.initialize_uniform(2.0, 0.1, 0.0);
        for r in &s.rho {
            assert!(approx_eq(*r, 2.0, 1e-6));
        }
        for u in &s.ux {
            assert!(approx_eq(*u, 0.1, 1e-6));
        }
        for u in &s.uy {
            assert!(approx_eq(*u, 0.0, 1e-6));
        }
        let feq = LbmSolver::equilibrium(2.0, 0.1, 0.0);
        for cell in 0..64 {
            for i in 0..9 {
                assert!(approx_eq(s.f[9 * cell + i], feq[i], 1e-6));
            }
        }
    }

    // ============ compute_macros 测试 ============

    #[test]
    fn test_compute_macros_equilibrium() {
        let mut s = LbmSolver::new(default_config(4, 4));
        s.initialize_uniform(1.5, 0.05, -0.03);
        s.compute_macros();
        for r in &s.rho {
            assert!(approx_eq(*r, 1.5, 1e-5));
        }
        for u in &s.ux {
            assert!(approx_eq(*u, 0.05, 1e-5));
        }
        for u in &s.uy {
            assert!(approx_eq(*u, -0.03, 1e-5));
        }
    }

    #[test]
    fn test_compute_macros_perturbed() {
        let mut s = LbmSolver::new(default_config(4, 4));
        s.initialize_uniform(1.0, 0.0, 0.0);
        s.f[0] += 0.05;
        s.compute_macros();
        assert!(approx_eq(s.rho[0], 1.05, 1e-5));
        assert!(approx_eq(s.ux[0], 0.0, 1e-5));
        assert!(approx_eq(s.uy[0], 0.0, 1e-5));
    }

    // ============ collide 测试 ============

    #[test]
    fn test_collide_preserves_mass() {
        let mut s = LbmSolver::new(default_config(8, 8));
        s.initialize_uniform(1.0, 0.0, 0.0);
        for i in 0..s.f.len() {
            s.f[i] += 0.01 * ((i as f32) % 3.0 - 1.0);
        }
        s.compute_macros();
        let mass_before: f32 = s.f.iter().sum();
        s.collide();
        let mass_after: f32 = s.f.iter().sum();
        assert!(approx_eq(mass_after, mass_before, 1e-3),
            "mass before = {}, after = {}", mass_before, mass_after);
    }

    #[test]
    fn test_collide_equilibrium_unchanged() {
        let mut s = LbmSolver::new(default_config(4, 4));
        s.initialize_uniform(1.0, 0.0, 0.0);
        let f_before: Vec<f32> = s.f.clone();
        s.compute_macros();
        s.collide();
        for i in 0..s.f.len() {
            assert!(approx_eq(s.f[i], f_before[i], 1e-6),
                "f[{}] changed: {} -> {}", i, f_before[i], s.f[i]);
        }
    }

    // ============ stream 测试 ============

    #[test]
    fn test_stream_uniform_unchanged() {
        let mut s = LbmSolver::new(periodic_config(8, 8));
        s.initialize_uniform(1.0, 0.0, 0.0);
        let f_before: Vec<f32> = s.f.clone();
        s.stream();
        for i in 0..s.f.len() {
            assert!(approx_eq(s.f[i], f_before[i], 1e-6));
        }
    }

    #[test]
    fn test_stream_periodic_shift() {
        let mut s = LbmSolver::new(periodic_config(8, 8));
        s.initialize_uniform(1.0, 0.0, 0.0);
        let cell = s.idx(4, 4);
        s.f[9 * cell + 1] += 0.1;
        s.stream();
        let dst = s.idx(5, 4);
        let base = LbmSolver::equilibrium(1.0, 0.0, 0.0)[1];
        let excess = s.f[9 * dst + 1] - base;
        assert!(approx_eq(excess, 0.1, 1e-5), "excess = {}", excess);
    }

    // ============ apply_bounce_back 测试 ============

    #[test]
    fn test_bounce_back_solid() {
        let mut s = LbmSolver::new(default_config(4, 4));
        s.initialize_uniform(1.0, 0.0, 0.0);
        s.set_solid(2, 2);
        let cell = s.idx(2, 2);
        for i in 0..9 {
            s.f_temp[9 * cell + i] = 0.1 * ((i as f32) + 1.0);
        }
        s.apply_bounce_back();
        for i in 0..9 {
            let expected = 0.1 * ((OPP[i] as f32) + 1.0);
            assert!(approx_eq(s.f[9 * cell + i], expected, 1e-6),
                "f[{}] = {}, expected {}", i, s.f[9 * cell + i], expected);
        }
    }

    // ============ apply_force 测试 ============

    #[test]
    fn test_apply_force_zero_no_op() {
        let mut s = LbmSolver::new(default_config(4, 4));
        s.initialize_uniform(1.0, 0.05, 0.0);
        let f_before: Vec<f32> = s.f.clone();
        s.apply_force();
        for i in 0..s.f.len() {
            assert!(approx_eq(s.f[i], f_before[i], 1e-6));
        }
    }

    #[test]
    fn test_apply_force_changes_velocity() {
        let cfg = LbmConfig {
            nx: 4,
            ny: 4,
            tau: 0.6,
            boundary_north: BoundaryType::BounceBack,
            boundary_south: BoundaryType::BounceBack,
            boundary_east: BoundaryType::BounceBack,
            boundary_west: BoundaryType::BounceBack,
            force_x: 0.001,
            force_y: 0.0,
        };
        let mut s = LbmSolver::new(cfg);
        s.initialize_uniform(1.0, 0.0, 0.0);
        s.apply_force();
        for u in &s.ux {
            assert!(approx_eq(*u, 0.001, 1e-5), "ux = {}", u);
        }
    }

    // ============ step 测试 ============

    #[test]
    fn test_step_uniform_unchanged() {
        let mut s = LbmSolver::new(periodic_config(8, 8));
        s.initialize_uniform(1.0, 0.0, 0.0);
        let rho_before = s.rho[0];
        s.step();
        assert_eq!(s.steps, 1);
        assert!(approx_eq(s.time, 1.0, 1e-6));
        for r in &s.rho {
            assert!(approx_eq(*r, rho_before, 1e-5));
        }
        for u in &s.ux {
            assert!(approx_eq(*u, 0.0, 1e-5));
        }
    }

    #[test]
    fn test_step_n_progress() {
        let mut s = LbmSolver::new(default_config(4, 4));
        s.step_n(5);
        assert_eq!(s.steps, 5);
        assert!(approx_eq(s.time, 5.0, 1e-6));
    }

    #[test]
    fn test_step_mass_conservation_periodic() {
        let mut s = LbmSolver::new(periodic_config(8, 8));
        s.initialize_uniform(1.0, 0.0, 0.0);
        s.f[0] += 0.1;
        let mass_before: f32 = s.f.iter().sum();
        s.step_n(10);
        let mass_after: f32 = s.f.iter().sum();
        assert!(approx_eq(mass_after, mass_before, 1e-3),
            "mass drift: before = {}, after = {}", mass_before, mass_after);
    }

    // ============ 统计方法测试 ============

    #[test]
    fn test_max_velocity() {
        let mut s = LbmSolver::new(default_config(4, 4));
        s.set_velocity(0, 0, 0.1, 0.0);
        s.set_velocity(1, 1, 0.0, 0.2);
        s.set_velocity(2, 2, 0.15, 0.0);
        let mv = s.max_velocity();
        assert!(approx_eq(mv, 0.2, 1e-5), "max_velocity = {}", mv);
    }

    #[test]
    fn test_average_density() {
        let s = LbmSolver::new(default_config(4, 4));
        let ad = s.average_density();
        assert!(approx_eq(ad, 1.0, 1e-5));
    }

    #[test]
    fn test_total_mass() {
        let s = LbmSolver::new(default_config(4, 4));
        assert!(approx_eq(s.total_mass(), 16.0, 1e-4));
    }

    #[test]
    fn test_total_mass_with_solid() {
        let mut s = LbmSolver::new(default_config(4, 4));
        s.set_solid(0, 0);
        s.set_solid(1, 1);
        assert!(approx_eq(s.total_mass(), 14.0, 1e-4));
    }

    #[test]
    fn test_average_velocity_x() {
        let mut s = LbmSolver::new(default_config(4, 4));
        s.set_velocity(0, 0, 0.1, 0.0);
        s.set_velocity(1, 0, 0.2, 0.0);
        let av = s.average_velocity_x();
        assert!(approx_eq(av, 0.3 / 16.0, 1e-5));
    }

    // ============ set/clear 操作测试 ============

    #[test]
    fn test_set_clear_solid() {
        let mut s = LbmSolver::new(default_config(4, 4));
        assert!(!s.solid[s.idx(2, 2)]);
        s.set_solid(2, 2);
        assert!(s.solid[s.idx(2, 2)]);
        s.clear_solid(2, 2);
        assert!(!s.solid[s.idx(2, 2)]);
    }

    #[test]
    fn test_set_velocity() {
        let mut s = LbmSolver::new(default_config(4, 4));
        s.set_velocity(2, 3, 0.15, -0.05);
        let cell = s.idx(2, 3);
        assert!(approx_eq(s.ux[cell], 0.15, 1e-6));
        assert!(approx_eq(s.uy[cell], -0.05, 1e-6));
        let feq = LbmSolver::equilibrium(1.0, 0.15, -0.05);
        for i in 0..9 {
            assert!(approx_eq(s.f[9 * cell + i], feq[i], 1e-6));
        }
    }

    #[test]
    fn test_reset() {
        let mut s = LbmSolver::new(default_config(4, 4));
        s.set_solid(1, 1);
        s.set_velocity(2, 2, 0.1, 0.0);
        s.step_n(3);
        s.reset();
        assert_eq!(s.steps, 0);
        assert!(approx_eq(s.time, 0.0, 1e-6));
        for r in &s.rho {
            assert!(approx_eq(*r, 1.0, 1e-6));
        }
        for u in &s.ux {
            assert!(approx_eq(*u, 0.0, 1e-6));
        }
        for sd in &s.solid {
            assert!(!sd);
        }
    }

    #[test]
    fn test_viscosity_method() {
        let s = LbmSolver::new(LbmConfig { nx: 4, ny: 4, tau: 0.8, ..Default::default() });
        assert!(approx_eq(s.viscosity(), (1.0 / 3.0) * 0.3, 1e-6));
    }

    // ============ 边界条件测试 ============

    #[test]
    fn test_bounce_back_boundary_no_normal_flow() {
        let mut s = LbmSolver::new(default_config(16, 16));
        s.initialize_uniform(1.0, 0.05, 0.0);
        s.step_n(5);
        for x in 0..16 {
            let south = s.idx(x, 0);
            let north = s.idx(x, 15);
            assert!(s.uy[south].abs() < 0.1, "south uy[{}] = {}", x, s.uy[south]);
            assert!(s.uy[north].abs() < 0.1, "north uy[{}] = {}", x, s.uy[north]);
        }
    }

    #[test]
    fn test_periodic_boundary_wraps() {
        let mut s = LbmSolver::new(periodic_config(8, 8));
        s.initialize_uniform(1.0, 0.0, 0.0);
        let cell = s.idx(0, 4);
        s.f[9 * cell + 3] += 0.1;
        s.stream();
        let dst = s.idx(7, 4);
        let base = LbmSolver::equilibrium(1.0, 0.0, 0.0)[3];
        let excess = s.f[9 * dst + 3] - base;
        assert!(approx_eq(excess, 0.1, 1e-5), "excess = {}", excess);
    }

    #[test]
    fn test_zou_he_velocity_boundary() {
        let cfg = LbmConfig {
            nx: 8,
            ny: 8,
            tau: 0.8,
            boundary_north: BoundaryType::BounceBack,
            boundary_south: BoundaryType::BounceBack,
            boundary_east: BoundaryType::BounceBack,
            boundary_west: BoundaryType::Velocity { ux: 0.1, uy: 0.0 },
            force_x: 0.0,
            force_y: 0.0,
        };
        let mut s = LbmSolver::new(cfg);
        s.initialize_uniform(1.0, 0.0, 0.0);
        s.step();
        let mut sum = 0.0;
        for y in 0..8 {
            let cell = s.idx(0, y);
            sum += s.ux[cell];
        }
        let avg_ux = sum / 8.0;
        assert!(avg_ux > 0.0, "avg ux at west = {}", avg_ux);
    }

    // ============ 物理场景测试 ============

    #[test]
    fn test_poiseuille_flow_direction() {
        let cfg = LbmConfig {
            nx: 16,
            ny: 8,
            tau: 0.8,
            boundary_north: BoundaryType::BounceBack,
            boundary_south: BoundaryType::BounceBack,
            boundary_east: BoundaryType::Periodic,
            boundary_west: BoundaryType::Periodic,
            force_x: 0.0005,
            force_y: 0.0,
        };
        let mut s = LbmSolver::new(cfg);
        s.initialize_uniform(1.0, 0.0, 0.0);
        s.step_n(100);
        let avg_ux = s.average_velocity_x();
        assert!(avg_ux > 0.0, "Poiseuille avg_ux = {} (should be > 0)", avg_ux);
    }

    #[test]
    fn test_long_run_stability() {
        let mut s = LbmSolver::new(default_config(8, 8));
        s.initialize_uniform(1.0, 0.0, 0.0);
        s.step_n(50);
        for r in &s.rho {
            assert!(r.is_finite(), "rho = {}", r);
        }
        for u in &s.ux {
            assert!(u.is_finite(), "ux = {}", u);
        }
        for f in &s.f {
            assert!(f.is_finite(), "f = {}", f);
        }
    }
}

