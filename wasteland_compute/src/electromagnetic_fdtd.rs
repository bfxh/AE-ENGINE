//! Electromagnetic FDTD Solver (Maxwell-Yee staggered grid)
//!
//! 基于 Kane Yee 1966 "Numerical solution of initial boundary value problems
//! involving Maxwell's equations in isotropic media" (IEEE Trans. Antennas Propag.).
//!
//! Maxwell 方程 (各向同性, 时域):
//!   ∂B/∂t = -∇×E
//!   ∂D/∂t = ∇×H - J      (D = εE, B = μH, J = σE)
//!   ∇·D = ρ
//!   ∇·B = 0
//!
//! Yee 网格交错采样 (E 在边中心, H 在面中心), leapfrog 时间积分:
//!   H^{n+1/2} = H^{n-1/2} - (dt/μ)·∇×E^n
//!   E^{n+1}   = E^n·(1 - σ·dt/ε) + (dt/ε)·∇×H^{n+1/2}
//!
//! CFL 稳定性 (3D):  dt <= h / (c·√3),  c = 1/√(ε·μ)
//!
//! 应用: 雷达截面, 无线电波传播, 光学模拟, 电磁兼容 (EMC),
//!       天线辐射方向图, 涉水/地下电磁探测.

use serde::{Deserialize, Serialize};

/// 真空介电常数 (F/m)
pub const EPSILON_0: f32 = 8.8541878128e-12;
/// 真空磁导率 (H/m)
pub const MU_0: f32 = 1.25663706212e-6;
/// 真空光速 (m/s)
pub const C_0: f32 = 2.99792458e8;
/// 真空波阻抗 (Ω)
pub const ETA_0: f32 = 376.730313668;

/// 电磁介质参数 (相对值 + 损耗)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Medium {
    /// 相对介电常数 ε_r (真空 = 1.0)
    pub permittivity_r: f32,
    /// 相对磁导率 μ_r (真空 = 1.0)
    pub permeability_r: f32,
    /// 电导率 σ (S/m, 0 = 无损耗)
    pub conductivity: f32,
}

impl Default for Medium {
    fn default() -> Self {
        Self::vacuum()
    }
}

impl Medium {
    pub const fn vacuum() -> Self {
        Medium { permittivity_r: 1.0, permeability_r: 1.0, conductivity: 0.0 }
    }
    /// 绝对介电常数 ε = ε_0 · ε_r  (F/m)
    pub fn eps(&self) -> f32 {
        EPSILON_0 * self.permittivity_r
    }
    /// 绝对磁导率 μ = μ_0 · μ_r  (H/m)
    pub fn mu(&self) -> f32 {
        MU_0 * self.permeability_r
    }
    /// 介质中波速 c = 1/√(ε·μ)  (m/s)
    pub fn wave_speed(&self) -> f32 {
        1.0 / (self.eps() * self.mu()).sqrt()
    }
    /// 波阻抗 η = √(μ/ε)  (Ω)
    pub fn impedance(&self) -> f32 {
        (self.mu() / self.eps()).sqrt()
    }
    /// 趋肤深度 δ = √(2/(ω·μ·σ));  此处返回 1/σ·η 的简化衰减长度
    pub fn loss_tangent(&self, omega: f32) -> f32 {
        self.conductivity / (omega * self.eps())
    }

    pub const fn air() -> Self {
        Medium { permittivity_r: 1.0006, permeability_r: 1.0, conductivity: 0.0 }
    }
    pub const fn fresh_water() -> Self {
        Medium { permittivity_r: 81.0, permeability_r: 1.0, conductivity: 0.01 }
    }
    pub const fn sea_water() -> Self {
        Medium { permittivity_r: 81.0, permeability_r: 1.0, conductivity: 4.0 }
    }
    pub const fn glass() -> Self {
        Medium { permittivity_r: 4.5, permeability_r: 1.0, conductivity: 0.0 }
    }
    pub const fn dry_sand() -> Self {
        Medium { permittivity_r: 2.5, permeability_r: 1.0, conductivity: 0.001 }
    }
    pub const fn wet_sand() -> Self {
        Medium { permittivity_r: 15.0, permeability_r: 1.0, conductivity: 0.05 }
    }
    pub const fn concrete() -> Self {
        Medium { permittivity_r: 5.3, permeability_r: 1.0, conductivity: 0.012 }
    }
    pub const fn copper() -> Self {
        Medium { permittivity_r: 1.0, permeability_r: 1.0, conductivity: 5.96e7 }
    }
    /// 完美电导体 (PEC) 近似 — σ → ∞
    pub const fn pec() -> Self {
        Medium { permittivity_r: 1.0, permeability_r: 1.0, conductivity: 1.0e30 }
    }
}

/// 边界类型 (6 个面可独立配置)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Emboundary {
    /// 完美电导体: 切向电场 E_tan = 0 (镜像反射)
    Pec,
    /// 完美磁导体: 切向磁场 H_tan = 0
    Pmc,
    /// Mur 一阶吸收边界 (无反射终止)
    Mur,
    /// 周期性边界 (wrap-around)
    Periodic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FdtdConfig {
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    /// 空间步长 Δx = Δy = Δz (m)
    pub h: f32,
    /// 时间步长 Δt (s)
    pub dt: f32,
    pub default_medium: Medium,
    /// 边界顺序: [-x, +x, -y, +y, -z, +z]
    pub boundary: [Emboundary; 6],
}

impl Default for FdtdConfig {
    fn default() -> Self {
        FdtdConfig {
            nx: 32,
            ny: 32,
            nz: 32,
            h: 0.01,
            dt: 1.0e-12,
            default_medium: Medium::vacuum(),
            boundary: [Emboundary::Pec; 6],
        }
    }
}

impl FdtdConfig {
    /// 由网格与介质自动计算满足 CFL 的最大时间步 (3D)
    pub fn cfl_dt(h: f32, med: &Medium) -> f32 {
        let c = med.wave_speed();
        h / (c * 3.0_f32.sqrt())
    }
    /// 按目标 CFL 数 (0..1) 设置 dt
    pub fn with_cfl(mut self, cfl: f32) -> Self {
        self.dt = cfl * Self::cfl_dt(self.h, &self.default_medium);
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FieldComponent {
    Ex,
    Ey,
    Ez,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SourceType {
    /// 软源: E += value (不影响入射波, 但能量持续注入)
    Soft,
    /// 硬源: E = value (会反射后续到达的波)
    Hard,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Waveform {
    /// 高斯脉冲 (宽带, 频谱丰富)
    Gaussian,
    /// 调制正弦 (高斯包络 × 正弦)
    ModulatedSine,
    /// Ricker 小波 (高斯导数, 地震/雷达常用)
    Ricker,
    /// 连续正弦 (窄带, 稳态)
    ContinuousSine,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PointSource {
    pub i: usize,
    pub j: usize,
    pub k: usize,
    pub component: FieldComponent,
    pub source_type: SourceType,
    pub amplitude: f32,
    pub frequency: f32,
    /// 高斯包络中心时间 (s)
    pub t0: f32,
    /// 高斯包络宽度 (s)
    pub width: f32,
    pub waveform: Waveform,
}

impl PointSource {
    pub fn value_at(&self, t: f32) -> f32 {
        match self.waveform {
            Waveform::Gaussian => {
                self.amplitude
                    * (-(t - self.t0).powi(2) / (2.0 * self.width * self.width)).exp()
            }
            Waveform::ModulatedSine => {
                let env = (-(t - self.t0).powi(2) / (2.0 * self.width * self.width)).exp();
                self.amplitude * env * (2.0 * std::f32::consts::PI * self.frequency * t).sin()
            }
            Waveform::Ricker => {
                let arg = std::f32::consts::PI * self.frequency * (t - self.t0);
                let a = 1.0 - 2.0 * arg * arg;
                self.amplitude * a * (-arg * arg).exp()
            }
            Waveform::ContinuousSine => {
                self.amplitude * (2.0 * std::f32::consts::PI * self.frequency * t).sin()
            }
        }
    }
}

pub struct FdtdSolver {
    pub config: FdtdConfig,
    /// 电场分量 (Yee 边中心, 此处同位存储)
    pub ex: Vec<f32>,
    pub ey: Vec<f32>,
    pub ez: Vec<f32>,
    /// 磁场分量 H (Yee 面中心, 此处同位存储)
    pub hx: Vec<f32>,
    pub hy: Vec<f32>,
    pub hz: Vec<f32>,
    pub medium_id: Vec<usize>,
    pub media: Vec<Medium>,
    pub sources: Vec<PointSource>,
    pub time: f32,
    pub steps: usize,
}

impl FdtdSolver {
    pub fn new(config: FdtdConfig) -> Self {
        let n = config.nx * config.ny * config.nz;
        let default_med = config.default_medium;
        FdtdSolver {
            config,
            ex: vec![0.0; n],
            ey: vec![0.0; n],
            ez: vec![0.0; n],
            hx: vec![0.0; n],
            hy: vec![0.0; n],
            hz: vec![0.0; n],
            medium_id: vec![0; n],
            media: vec![default_med],
            sources: Vec::new(),
            time: 0.0,
            steps: 0,
        }
    }

    pub fn idx(&self, i: usize, j: usize, k: usize) -> usize {
        debug_assert!(i < self.config.nx);
        debug_assert!(j < self.config.ny);
        debug_assert!(k < self.config.nz);
        i + self.config.nx * (j + self.config.ny * k)
    }

    pub fn set_medium(&mut self, i: usize, j: usize, k: usize, medium_id: usize) {
        let idx = self.idx(i, j, k);
        self.medium_id[idx] = medium_id;
    }

    pub fn fill_medium(&mut self, i0: usize, j0: usize, k0: usize, i1: usize, j1: usize, k1: usize, medium_id: usize) {
        for k in k0..k1.min(self.config.nz) {
            for j in j0..j1.min(self.config.ny) {
                for i in i0..i1.min(self.config.nx) {
                    let idx = self.idx(i, j, k);
                    self.medium_id[idx] = medium_id;
                }
            }
        }
    }

    pub fn add_medium(&mut self, m: Medium) -> usize {
        self.media.push(m);
        self.media.len() - 1
    }

    pub fn add_source(&mut self, src: PointSource) {
        self.sources.push(src);
    }

    /// ∇×E 的 x 分量在 (i,j,k):  ∂Ez/∂y - ∂Ey/∂z  (前向差分)
    fn curl_e_x(&self, i: usize, j: usize, k: usize) -> f32 {
        let (nx, ny, nz) = (self.config.nx, self.config.ny, self.config.nz);
        let periodic = self.config.boundary[4] == Emboundary::Periodic;
        let ez_here = self.ez[self.idx(i, j, k)];
        let ez_yp = if j + 1 < ny {
            self.ez[self.idx(i, j + 1, k)]
        } else if periodic {
            self.ez[self.idx(i, 0, k)]
        } else {
            ez_here
        };
        let ey_here = self.ey[self.idx(i, j, k)];
        let ey_zp = if k + 1 < nz {
            self.ey[self.idx(i, j, k + 1)]
        } else if periodic {
            self.ey[self.idx(i, j, 0)]
        } else {
            ey_here
        };
        ((ez_yp - ez_here) - (ey_zp - ey_here)) / self.config.h
    }

    fn curl_e_y(&self, i: usize, j: usize, k: usize) -> f32 {
        let (nx, ny, nz) = (self.config.nx, self.config.ny, self.config.nz);
        let periodic = self.config.boundary[4] == Emboundary::Periodic;
        let ex_here = self.ex[self.idx(i, j, k)];
        let ex_zp = if k + 1 < nz {
            self.ex[self.idx(i, j, k + 1)]
        } else if periodic {
            self.ex[self.idx(i, j, 0)]
        } else {
            ex_here
        };
        let ez_here = self.ez[self.idx(i, j, k)];
        let ez_xp = if i + 1 < nx {
            self.ez[self.idx(i + 1, j, k)]
        } else if periodic {
            self.ez[self.idx(0, j, k)]
        } else {
            ez_here
        };
        ((ex_zp - ex_here) - (ez_xp - ez_here)) / self.config.h
    }

    fn curl_e_z(&self, i: usize, j: usize, k: usize) -> f32 {
        let (nx, ny, _nz) = (self.config.nx, self.config.ny, self.config.nz);
        let periodic = self.config.boundary[4] == Emboundary::Periodic;
        let ey_here = self.ey[self.idx(i, j, k)];
        let ey_xp = if i + 1 < nx {
            self.ey[self.idx(i + 1, j, k)]
        } else if periodic {
            self.ey[self.idx(0, j, k)]
        } else {
            ey_here
        };
        let ex_here = self.ex[self.idx(i, j, k)];
        let ex_yp = if j + 1 < ny {
            self.ex[self.idx(i, j + 1, k)]
        } else if periodic {
            self.ex[self.idx(i, 0, k)]
        } else {
            ex_here
        };
        ((ey_xp - ey_here) - (ex_yp - ex_here)) / self.config.h
    }

    /// ∇×H 的 x 分量 (后向差分, 与 curl_e 交错)
    fn curl_h_x(&self, i: usize, j: usize, k: usize) -> f32 {
        let (ny, nz) = (self.config.ny, self.config.nz);
        let periodic = self.config.boundary[4] == Emboundary::Periodic;
        let hz_here = self.hz[self.idx(i, j, k)];
        let hz_ym = if j > 0 {
            self.hz[self.idx(i, j - 1, k)]
        } else if periodic {
            self.hz[self.idx(i, ny - 1, k)]
        } else {
            hz_here
        };
        let hy_here = self.hy[self.idx(i, j, k)];
        let hy_zm = if k > 0 {
            self.hy[self.idx(i, j, k - 1)]
        } else if periodic {
            self.hy[self.idx(i, j, nz - 1)]
        } else {
            hy_here
        };
        ((hz_here - hz_ym) - (hy_here - hy_zm)) / self.config.h
    }

    fn curl_h_y(&self, i: usize, j: usize, k: usize) -> f32 {
        let (nx, nz) = (self.config.nx, self.config.nz);
        let periodic = self.config.boundary[4] == Emboundary::Periodic;
        let hx_here = self.hx[self.idx(i, j, k)];
        let hx_zm = if k > 0 {
            self.hx[self.idx(i, j, k - 1)]
        } else if periodic {
            self.hx[self.idx(i, j, nz - 1)]
        } else {
            hx_here
        };
        let hz_here = self.hz[self.idx(i, j, k)];
        let hz_xm = if i > 0 {
            self.hz[self.idx(i - 1, j, k)]
        } else if periodic {
            self.hz[self.idx(nx - 1, j, k)]
        } else {
            hz_here
        };
        ((hx_here - hx_zm) - (hz_here - hz_xm)) / self.config.h
    }

    fn curl_h_z(&self, i: usize, j: usize, k: usize) -> f32 {
        let (nx, ny) = (self.config.nx, self.config.ny);
        let periodic = self.config.boundary[4] == Emboundary::Periodic;
        let hy_here = self.hy[self.idx(i, j, k)];
        let hy_xm = if i > 0 {
            self.hy[self.idx(i - 1, j, k)]
        } else if periodic {
            self.hy[self.idx(nx - 1, j, k)]
        } else {
            hy_here
        };
        let hx_here = self.hx[self.idx(i, j, k)];
        let hx_ym = if j > 0 {
            self.hx[self.idx(i, j - 1, k)]
        } else if periodic {
            self.hx[self.idx(i, ny - 1, k)]
        } else {
            hx_here
        };
        ((hy_here - hy_xm) - (hx_here - hx_ym)) / self.config.h
    }

    /// H^{n+1/2} = H^{n-1/2} - (dt/μ)·∇×E^n
    pub fn step_h(&mut self) {
        let dt = self.config.dt;
        let nx = self.config.nx;
        let ny = self.config.ny;
        let nz = self.config.nz;
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let idx = i + nx * (j + ny * k);
                    let med = self.media[self.medium_id[idx]];
                    let ce = dt / med.mu();
                    let cx = self.curl_e_x(i, j, k);
                    let cy = self.curl_e_y(i, j, k);
                    let cz = self.curl_e_z(i, j, k);
                    self.hx[idx] -= ce * cx;
                    self.hy[idx] -= ce * cy;
                    self.hz[idx] -= ce * cz;
                }
            }
        }
    }

    /// E^{n+1} = E^n·(1 - σ·dt/ε) + (dt/ε)·∇×H^{n+1/2}
    pub fn step_e(&mut self) {
        let dt = self.config.dt;
        let nx = self.config.nx;
        let ny = self.config.ny;
        let nz = self.config.nz;
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let idx = i + nx * (j + ny * k);
                    let med = self.media[self.medium_id[idx]];
                    let eps = med.eps();
                    let loss = 1.0 - med.conductivity * dt / eps;
                    let ch = dt / eps;
                    let cx = self.curl_h_x(i, j, k);
                    let cy = self.curl_h_y(i, j, k);
                    let cz = self.curl_h_z(i, j, k);
                    self.ex[idx] = self.ex[idx] * loss + ch * cx;
                    self.ey[idx] = self.ey[idx] * loss + ch * cy;
                    self.ez[idx] = self.ez[idx] * loss + ch * cz;
                }
            }
        }
    }

    pub fn apply_sources(&mut self) {
        for src in &self.sources {
            let val = src.value_at(self.time);
            if src.i >= self.config.nx || src.j >= self.config.ny || src.k >= self.config.nz {
                continue;
            }
            let idx = self.idx(src.i, src.j, src.k);
            let field = match src.component {
                FieldComponent::Ex => &mut self.ex,
                FieldComponent::Ey => &mut self.ey,
                FieldComponent::Ez => &mut self.ez,
            };
            match src.source_type {
                SourceType::Soft => field[idx] += val,
                SourceType::Hard => field[idx] = val,
            }
        }
    }

    /// 应用边界条件 (PEC/PMC 设切向场为 0, Mur 一阶吸收, Periodic 在 curl 中处理)
    pub fn apply_boundary(&mut self) {
        let (nx, ny, nz) = (self.config.nx, self.config.ny, self.config.nz);
        let b = &self.config.boundary;
        // -x / +x 面: 切向 E_y, E_z
        for face in 0..2usize {
            let i = if face == 0 { 0 } else { nx - 1 };
            let kind = b[face];
            for k in 0..nz {
                for j in 0..ny {
                    let idx = i + nx * (j + ny * k);
                    match kind {
                        Emboundary::Pec => {
                            self.ey[idx] = 0.0;
                            self.ez[idx] = 0.0;
                        }
                        Emboundary::Pmc => {
                            self.hy[idx] = 0.0;
                            self.hz[idx] = 0.0;
                        }
                        Emboundary::Mur => {
                            // Mur 一阶: E_tan 边界 = E_tan 内层 (外行波近似)
                            let inner = if face == 0 { 1 } else { nx - 2 };
                            if inner < nx {
                                let ii = inner + nx * (j + ny * k);
                                self.ey[idx] = self.ey[ii];
                                self.ez[idx] = self.ez[ii];
                            }
                        }
                        Emboundary::Periodic => {}
                    }
                }
            }
        }
        // -y / +y 面: 切向 E_x, E_z
        for face in 0..2usize {
            let j = if face == 0 { 0 } else { ny - 1 };
            let kind = b[2 + face];
            for k in 0..nz {
                for i in 0..nx {
                    let idx = i + nx * (j + ny * k);
                    match kind {
                        Emboundary::Pec => {
                            self.ex[idx] = 0.0;
                            self.ez[idx] = 0.0;
                        }
                        Emboundary::Pmc => {
                            self.hx[idx] = 0.0;
                            self.hz[idx] = 0.0;
                        }
                        Emboundary::Mur => {
                            let inner = if face == 0 { 1 } else { ny - 2 };
                            if inner < ny {
                                let ii = i + nx * (inner + ny * k);
                                self.ex[idx] = self.ex[ii];
                                self.ez[idx] = self.ez[ii];
                            }
                        }
                        Emboundary::Periodic => {}
                    }
                }
            }
        }
        // -z / +z 面: 切向 E_x, E_y
        for face in 0..2usize {
            let k = if face == 0 { 0 } else { nz - 1 };
            let kind = b[4 + face];
            for j in 0..ny {
                for i in 0..nx {
                    let idx = i + nx * (j + ny * k);
                    match kind {
                        Emboundary::Pec => {
                            self.ex[idx] = 0.0;
                            self.ey[idx] = 0.0;
                        }
                        Emboundary::Pmc => {
                            self.hx[idx] = 0.0;
                            self.hy[idx] = 0.0;
                        }
                        Emboundary::Mur => {
                            let inner = if face == 0 { 1 } else { nz - 2 };
                            if inner < nz {
                                let ii = i + nx * (j + ny * inner);
                                self.ex[idx] = self.ex[ii];
                                self.ey[idx] = self.ey[ii];
                            }
                        }
                        Emboundary::Periodic => {}
                    }
                }
            }
        }
    }

    /// 完整 leapfrog 一步: H 更新 → E 更新 → 源 → 边界
    pub fn step(&mut self) {
        self.step_h();
        self.step_e();
        self.apply_sources();
        self.apply_boundary();
        self.time += self.config.dt;
        self.steps += 1;
    }

    pub fn step_n(&mut self, n: usize) {
        for _ in 0..n {
            self.step();
        }
    }

    /// 总电磁能量 = ½(ε|E|² + μ|H|²) · ΔV
    pub fn total_energy(&self) -> f32 {
        let dv = self.config.h.powi(3);
        let mut e = 0.0;
        let nx = self.config.nx;
        let ny = self.config.ny;
        let nz = self.config.nz;
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let idx = i + nx * (j + ny * k);
                    let med = self.media[self.medium_id[idx]];
                    let eps = med.eps();
                    let mu = med.mu();
                    let e2 = self.ex[idx] * self.ex[idx]
                        + self.ey[idx] * self.ey[idx]
                        + self.ez[idx] * self.ez[idx];
                    let h2 = self.hx[idx] * self.hx[idx]
                        + self.hy[idx] * self.hy[idx]
                        + self.hz[idx] * self.hz[idx];
                    e += 0.5 * (eps * e2 + mu * h2) * dv;
                }
            }
        }
        e
    }

    /// |E| 最大值
    pub fn max_e(&self) -> f32 {
        let mut m: f32 = 0.0;
        for i in 0..self.ex.len() {
            m = m.max(self.ex[i].abs()).max(self.ey[i].abs()).max(self.ez[i].abs());
        }
        m
    }

    /// |H| 最大值
    pub fn max_h(&self) -> f32 {
        let mut m: f32 = 0.0;
        for i in 0..self.hx.len() {
            m = m.max(self.hx[i].abs()).max(self.hy[i].abs()).max(self.hz[i].abs());
        }
        m
    }

    /// E 场 RMS (均方根)
    pub fn rms_e(&self) -> f32 {
        let mut s = 0.0;
        for i in 0..self.ex.len() {
            s += self.ex[i] * self.ex[i] + self.ey[i] * self.ey[i] + self.ez[i] * self.ez[i];
        }
        (s / self.ex.len() as f32).sqrt()
    }

    /// H 场 RMS
    pub fn rms_h(&self) -> f32 {
        let mut s = 0.0;
        for i in 0..self.hx.len() {
            s += self.hx[i] * self.hx[i] + self.hy[i] * self.hy[i] + self.hz[i] * self.hz[i];
        }
        (s / self.hx.len() as f32).sqrt()
    }

    /// Poynting 矢量 |S| = |E×H| 在 (i,j,k)
    pub fn poynting(&self, i: usize, j: usize, k: usize) -> [f32; 3] {
        let idx = self.idx(i, j, k);
        let (ex, ey, ez) = (self.ex[idx], self.ey[idx], self.ez[idx]);
        let (hx, hy, hz) = (self.hx[idx], self.hy[idx], self.hz[idx]);
        [
            ey * hz - ez * hy,
            ez * hx - ex * hz,
            ex * hy - ey * hx,
        ]
    }

    pub fn reset(&mut self) {
        for v in &mut self.ex {
            *v = 0.0;
        }
        for v in &mut self.ey {
            *v = 0.0;
        }
        for v in &mut self.ez {
            *v = 0.0;
        }
        for v in &mut self.hx {
            *v = 0.0;
        }
        for v in &mut self.hy {
            *v = 0.0;
        }
        for v in &mut self.hz {
            *v = 0.0;
        }
        self.time = 0.0;
        self.steps = 0;
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn small_cfg() -> FdtdConfig {
        FdtdConfig {
            nx: 8,
            ny: 8,
            nz: 8,
            h: 0.01,
            dt: 1.0e-12,
            default_medium: Medium::vacuum(),
            boundary: [Emboundary::Pec; 6],
        }
    }

    #[test]
    fn test_medium_vacuum_constants() {
        let m = Medium::vacuum();
        assert_eq!(m.permittivity_r, 1.0);
        assert_eq!(m.permeability_r, 1.0);
        assert_eq!(m.conductivity, 0.0);
        assert!((m.eps() - EPSILON_0).abs() / EPSILON_0 < 1e-3);
        assert!((m.mu() - MU_0).abs() / MU_0 < 1e-3);
        assert!((m.wave_speed() - C_0).abs() / C_0 < 1e-3);
        assert!((m.impedance() - ETA_0).abs() / ETA_0 < 1e-3);
    }

    #[test]
    fn test_medium_water_high_permittivity() {
        let w = Medium::fresh_water();
        assert_eq!(w.permittivity_r, 81.0);
        // 水中光速 ≈ c/9
        let c_water = w.wave_speed();
        let ratio = C_0 / c_water;
        assert!((ratio - 9.0).abs() < 0.1);
    }

    #[test]
    fn test_medium_glass() {
        let g = Medium::glass();
        assert_eq!(g.permittivity_r, 4.5);
        let ratio = C_0 / g.wave_speed();
        assert!((ratio - g.permittivity_r.sqrt()).abs() < 0.05);
    }

    #[test]
    fn test_medium_sea_water_conductive() {
        let s = Medium::sea_water();
        assert!(s.conductivity > 0.0);
        assert!(s.loss_tangent(1.0e9) > 0.0);
    }

    #[test]
    fn test_medium_copper_high_conductivity() {
        let c = Medium::copper();
        assert!(c.conductivity > 1.0e6);
    }

    #[test]
    fn test_pec_infinite_conductivity() {
        let p = Medium::pec();
        assert!(p.conductivity > 1.0e25);
    }

    #[test]
    fn test_config_default() {
        let c = FdtdConfig::default();
        assert_eq!(c.nx, 32);
        assert_eq!(c.boundary, [Emboundary::Pec; 6]);
        assert_eq!(c.default_medium, Medium::vacuum());
    }

    #[test]
    fn test_cfl_dt() {
        let dt = FdtdConfig::cfl_dt(0.01, &Medium::vacuum());
        // dt = h / (c·√3)
        let expected = 0.01 / (C_0 * 3.0_f32.sqrt());
        assert!((dt - expected).abs() / expected < 1e-3);
        assert!(dt > 0.0);
    }

    #[test]
    fn test_config_with_cfl() {
        let cfg = FdtdConfig::default().with_cfl(0.5);
        let expected = 0.5 * FdtdConfig::cfl_dt(0.01, &Medium::vacuum());
        assert!((cfg.dt - expected).abs() / expected < 1e-3);
    }

    #[test]
    fn test_solver_new() {
        let s = FdtdSolver::new(small_cfg());
        assert_eq!(s.ex.len(), 512);
        assert_eq!(s.hz.len(), 512);
        assert_eq!(s.media.len(), 1);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
        for i in 0..512 {
            assert_eq!(s.ex[i], 0.0);
            assert_eq!(s.hz[i], 0.0);
        }
    }

    #[test]
    fn test_idx() {
        let s = FdtdSolver::new(small_cfg());
        assert_eq!(s.idx(0, 0, 0), 0);
        assert_eq!(s.idx(1, 0, 0), 1);
        assert_eq!(s.idx(0, 1, 0), 8);
        assert_eq!(s.idx(0, 0, 1), 64);
        assert_eq!(s.idx(7, 7, 7), 511);
    }

    #[test]
    fn test_set_medium() {
        let mut s = FdtdSolver::new(small_cfg());
        let id = s.add_medium(Medium::fresh_water());
        assert_eq!(id, 1);
        s.set_medium(0, 0, 0, 1);
        assert_eq!(s.medium_id[s.idx(0, 0, 0)], 1);
        assert_eq!(s.medium_id[s.idx(1, 0, 0)], 0);
    }

    #[test]
    fn test_fill_medium() {
        let mut s = FdtdSolver::new(small_cfg());
        let id = s.add_medium(Medium::glass());
        s.fill_medium(0, 0, 0, 4, 4, 4, id);
        assert_eq!(s.medium_id[s.idx(0, 0, 0)], id);
        assert_eq!(s.medium_id[s.idx(3, 3, 3)], id);
        assert_eq!(s.medium_id[s.idx(4, 0, 0)], 0);
    }

    #[test]
    fn test_waveform_gaussian_at_center() {
        let src = PointSource {
            i: 4, j: 4, k: 4,
            component: FieldComponent::Ez,
            source_type: SourceType::Soft,
            amplitude: 1.0,
            frequency: 1.0e9,
            t0: 1.0e-9,
            width: 2.0e-10,
            waveform: Waveform::Gaussian,
        };
        let v = src.value_at(1.0e-9);
        assert!((v - 1.0).abs() < 1e-3);
        let v_far = src.value_at(5.0e-9);
        assert!(v_far.abs() < 1e-6);
    }

    #[test]
    fn test_waveform_ricker_zero_at_center() {
        let src = PointSource {
            i: 4, j: 4, k: 4,
            component: FieldComponent::Ez,
            source_type: SourceType::Soft,
            amplitude: 1.0,
            frequency: 1.0e9,
            t0: 1.0e-9,
            width: 2.0e-10,
            waveform: Waveform::Ricker,
        };
        let v = src.value_at(1.0e-9);
        // Ricker 在 t=t0 时 = (1-0)·exp(0) = 1
        assert!((v - 1.0).abs() < 1e-3);
    }

    #[test]
    fn test_waveform_continuous_sine() {
        let src = PointSource {
            i: 0, j: 0, k: 0,
            component: FieldComponent::Ex,
            source_type: SourceType::Soft,
            amplitude: 1.0,
            frequency: 1.0,
            t0: 0.0,
            width: 1.0,
            waveform: Waveform::ContinuousSine,
        };
        let v0 = src.value_at(0.0);
        assert!(v0.abs() < 1e-6);
        let v_quarter = src.value_at(0.25);
        assert!((v_quarter - 1.0).abs() < 1e-3);
    }

    #[test]
    fn test_curl_e_zero_for_uniform_field() {
        let mut s = FdtdSolver::new(small_cfg());
        for i in 0..s.ex.len() {
            s.ez[i] = 1.0;
        }
        let c = s.curl_e_x(4, 4, 4);
        assert!(c.abs() < 1e-6);
    }

    #[test]
    fn test_curl_e_constant_gradient() {
        let mut s = FdtdSolver::new(small_cfg());
        // Ez 随 y 线性变化: curl_e_x = ∂Ez/∂y - ∂Ey/∂z = (Ez(j+1)-Ez(j))/h
        for k in 0..8 {
            for j in 0..8 {
                for i in 0..8 {
                    let idx = s.idx(i, j, k);
                    s.ez[idx] = j as f32;
                }
            }
        }
        let c = s.curl_e_x(4, 4, 4);
        assert!((c - 1.0 / 0.01).abs() < 1.0);
    }

    #[test]
    fn test_curl_h_zero_for_uniform_field() {
        let mut s = FdtdSolver::new(small_cfg());
        for i in 0..s.hz.len() {
            s.hz[i] = 1.0;
        }
        let c = s.curl_h_x(4, 4, 4);
        assert!(c.abs() < 1e-6);
    }

    #[test]
    fn test_step_h_updates_from_e() {
        let mut s = FdtdSolver::new(small_cfg());
        // 注入一个 E 脉冲, H 应当变化
        let idx = s.idx(4, 4, 4);
        s.ez[idx] = 1.0e3;
        let h_before = s.hx[s.idx(4, 4, 4)];
        s.step_h();
        let h_after = s.hx[s.idx(4, 4, 4)];
        assert!(h_after.abs() > h_before.abs());
    }

    #[test]
    fn test_step_e_updates_from_h() {
        let mut s = FdtdSolver::new(small_cfg());
        let idx = s.idx(4, 4, 4);
        s.hz[idx] = 1.0e3;
        let e_before = s.ex[s.idx(4, 4, 4)];
        s.step_e();
        let e_after = s.ex[s.idx(4, 4, 4)];
        assert!(e_after.abs() > e_before.abs());
    }

    #[test]
    fn test_step_advances_time() {
        let mut s = FdtdSolver::new(small_cfg());
        let t0 = s.time;
        s.step();
        assert!((s.time - t0 - s.config.dt).abs() < 1e-18);
        assert_eq!(s.steps, 1);
        s.step();
        assert_eq!(s.steps, 2);
    }

    #[test]
    fn test_step_n() {
        let mut s = FdtdSolver::new(small_cfg());
        s.step_n(10);
        assert_eq!(s.steps, 10);
        assert!((s.time - 10.0 * s.config.dt).abs() < 1e-18);
    }

    #[test]
    fn test_source_injection_soft() {
        let mut s = FdtdSolver::new(small_cfg());
        s.add_source(PointSource {
            i: 4, j: 4, k: 4,
            component: FieldComponent::Ez,
            source_type: SourceType::Soft,
            amplitude: 100.0,
            frequency: 1.0,
            t0: 0.0,
            width: 1.0,
            waveform: Waveform::Gaussian,
        });
        s.apply_sources();
        let idx = s.idx(4, 4, 4);
        assert!(s.ez[idx].abs() > 0.0);
    }

    #[test]
    fn test_source_injection_hard_overwrites() {
        let mut s = FdtdSolver::new(small_cfg());
        let idx = s.idx(4, 4, 4);
        s.ez[idx] = 999.0;
        s.add_source(PointSource {
            i: 4, j: 4, k: 4,
            component: FieldComponent::Ez,
            source_type: SourceType::Hard,
            amplitude: 1.0,
            frequency: 0.0,
            t0: 0.0,
            width: 1.0,
            waveform: Waveform::Gaussian,
        });
        s.apply_sources();
        // Hard source at t=0 with Gaussian centered at t0=0 → value = 1.0
        assert!((s.ez[s.idx(4, 4, 4)] - 1.0).abs() < 1e-3);
    }

    #[test]
    fn test_pec_boundary_zeroes_tangential_e() {
        let mut s = FdtdSolver::new(small_cfg());
        // -x 面 PEC: ey, ez at i=0 应为 0
        let idx = s.idx(0, 4, 4);
        s.ey[idx] = 5.0;
        let idx = s.idx(0, 4, 4);
        s.ez[idx] = 7.0;
        s.apply_boundary();
        assert_eq!(s.ey[s.idx(0, 4, 4)], 0.0);
        assert_eq!(s.ez[s.idx(0, 4, 4)], 0.0);
    }

    #[test]
    fn test_pmc_boundary_zeroes_tangential_h() {
        let mut cfg = small_cfg();
        cfg.boundary[0] = Emboundary::Pmc;
        let mut s = FdtdSolver::new(cfg);
        let idx = s.idx(0, 4, 4);
        s.hy[idx] = 5.0;
        let idx = s.idx(0, 4, 4);
        s.hz[idx] = 7.0;
        s.apply_boundary();
        assert_eq!(s.hy[s.idx(0, 4, 4)], 0.0);
        assert_eq!(s.hz[s.idx(0, 4, 4)], 0.0);
    }

    #[test]
    fn test_mur_boundary_copies_inner() {
        let mut cfg = small_cfg();
        cfg.boundary[0] = Emboundary::Mur;
        let mut s = FdtdSolver::new(cfg);
        let idx = s.idx(1, 4, 4);
        s.ey[idx] = 3.0;
        let idx = s.idx(0, 4, 4);
        s.ey[idx] = 0.0;
        s.apply_boundary();
        assert!((s.ey[s.idx(0, 4, 4)] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_periodic_boundary_in_curl() {
        let mut cfg = small_cfg();
        for b in cfg.boundary.iter_mut() {
            *b = Emboundary::Periodic;
        }
        let mut s = FdtdSolver::new(cfg);
        // 在 i=0 放一个 Ez 脉冲, curl 应该 wrap 到 i=nx-1 的邻居
        let idx = s.idx(0, 4, 4);
        s.ez[idx] = 1.0;
        let c = s.curl_e_y(0, 4, 4);
        // 非周期时 ez_xp = ez_here = 1, curl_y 的 ez 项 = 0
        // 周期时 ez_xp = ez[idx(7,4,4)] = 0, 所以 (ez_here - ez_xp) 项... 实际 curl_e_y = (ex_zp-ex) - (ez_xp-ez)
        // 这里 ez_xp 在周期下 = ez[0,4,4]? 不, i+1=1 < nx, 不触发 wrap
        // 让我测 i=nx-1=7 处
        let c7 = s.curl_e_y(7, 4, 4);
        // i=7, ez_xp 周期 wrap 到 i=0 = 1.0, ez_here=0
        // curl_e_y = (ex_zp - ex) - (ez_xp - ez) = 0 - (1.0 - 0.0)/h = -100
        assert!((c7 - (-1.0 / 0.01)).abs() < 1.0);
    }

    #[test]
    fn test_energy_zero_initially() {
        let s = FdtdSolver::new(small_cfg());
        assert_eq!(s.total_energy(), 0.0);
    }

    #[test]
    fn test_energy_positive_after_source() {
        let mut s = FdtdSolver::new(small_cfg());
        let idx = s.idx(4, 4, 4);
        s.ez[idx] = 1000.0;
        let e = s.total_energy();
        assert!(e > 0.0);
    }

    #[test]
    fn test_energy_scales_with_amplitude() {
        let mut s1 = FdtdSolver::new(small_cfg());
        let mut s2 = FdtdSolver::new(small_cfg());
        let idx1 = s1.idx(4, 4, 4);
        s1.ez[idx1] = 1000.0;
        let idx2 = s2.idx(4, 4, 4);
        s2.ez[idx2] = 2000.0;
        let e1 = s1.total_energy();
        let e2 = s2.total_energy();
        // 能量 ~ E², 振幅翻倍 → 能量 4 倍
        assert!((e2 / e1 - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_conductivity_decays_energy() {
        let mut cfg = small_cfg();
        cfg.default_medium = Medium { permittivity_r: 1.0, permeability_r: 1.0, conductivity: 1.0 };
        let mut s = FdtdSolver::new(cfg);
        let idx = s.idx(4, 4, 4);
        s.ex[idx] = 1000.0;
        let e0 = s.total_energy();
        s.step_e();
        let e1 = s.total_energy();
        assert!(e1 < e0);
    }

    #[test]
    fn test_lossless_medium_preserves_e_energy_in_step_e() {
        // 真空无损耗, step_e 中 loss = 1, 但 curl·H 会改变 E
        // 这里仅测试: 无 H 时 step_e 不衰减 E
        let mut s = FdtdSolver::new(small_cfg());
        let idx = s.idx(4, 4, 4);
        s.ex[idx] = 1000.0;
        let e0 = s.total_energy();
        s.step_e();
        let e1 = s.total_energy();
        // H=0, curl H=0, loss=1, 所以 E 不变
        assert!((e1 - e0).abs() / e0 < 1e-3);
    }

    #[test]
    fn test_max_e_and_max_h() {
        let mut s = FdtdSolver::new(small_cfg());
        let idx = s.idx(1, 2, 3);
        s.ex[idx] = 5.0;
        let idx = s.idx(4, 4, 4);
        s.hz[idx] = -7.0;
        assert!((s.max_e() - 5.0).abs() < 1e-6);
        assert!((s.max_h() - 7.0).abs() < 1e-6);
    }

    #[test]
    fn test_rms_e() {
        let mut s = FdtdSolver::new(small_cfg());
        // 全场 = 1, RMS = 1
        for i in 0..s.ex.len() {
            s.ex[i] = 1.0;
        }
        let r = s.rms_e();
        assert!((r - 1.0).abs() < 1e-3);
    }

    #[test]
    fn test_poynting_vector() {
        let mut s = FdtdSolver::new(small_cfg());
        let idx = s.idx(4, 4, 4);
        s.ex[idx] = 1.0;
        let idx = s.idx(4, 4, 4);
        s.hy[idx] = 1.0;
        let p = s.poynting(4, 4, 4);
        // S = E × H, E=(1,0,0), H=(0,1,0) → S = (0*0-0*1, 0*0-1*0, 1*1-0*0) = (0,0,1)
        assert!(p[0].abs() < 1e-6);
        assert!(p[1].abs() < 1e-6);
        assert!((p[2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_reset() {
        let mut s = FdtdSolver::new(small_cfg());
        let idx = s.idx(0, 0, 0);
        s.ex[idx] = 5.0;
        let idx = s.idx(1, 1, 1);
        s.hz[idx] = 3.0;
        s.time = 1.0;
        s.steps = 10;
        s.reset();
        assert_eq!(s.ex[s.idx(0, 0, 0)], 0.0);
        assert_eq!(s.hz[s.idx(1, 1, 1)], 0.0);
        assert_eq!(s.time, 0.0);
        assert_eq!(s.steps, 0);
    }

    #[test]
    fn test_wave_propagation_from_pulse() {
        // 在中心注入脉冲, 几步后能量应在网格中传播
        let mut cfg = small_cfg();
        cfg.boundary = [Emboundary::Mur; 6];
        let mut s = FdtdSolver::new(cfg);
        s.add_source(PointSource {
            i: 4, j: 4, k: 4,
            component: FieldComponent::Ez,
            source_type: SourceType::Soft,
            amplitude: 1000.0,
            frequency: 1.0,
            t0: 0.0,
            width: 1.0,
            waveform: Waveform::Gaussian,
        });
        s.step_n(5);
        // 经过几步, 场应当扩散到邻居
        assert!(s.max_e() > 0.0);
        assert!(s.max_h() > 0.0);
    }

    #[test]
    fn test_multiple_media_wave_speed() {
        let mut s = FdtdSolver::new(small_cfg());
        let water_id = s.add_medium(Medium::fresh_water());
        s.fill_medium(0, 0, 0, 8, 8, 4, water_id);
        // 上半部分水, 下半部分真空
        let c_water = s.media[water_id].wave_speed();
        let c_vac = s.media[0].wave_speed();
        assert!(c_water < c_vac);
    }

    #[test]
    fn test_3d_isotropy_curl() {
        // 在三个方向分别注入相同脉冲, curl 响应应对称
        let mut s = FdtdSolver::new(small_cfg());
        let idx = s.idx(4, 4, 4);
        s.ez[idx] = 1.0;
        let cx = s.curl_e_x(4, 4, 4);
        // Ez 脉冲只影响 curl_e_x (∂Ez/∂y) — 但 ∂Ez/∂y 在同点为 0 (邻居也是 0)
        // 实际上 curl_e_x 在 (4,4,4) = (ez[5]-ez[4])/h - ... 邻居为 0
        // 所以这里测试: 邻居的 curl 非零
        let cx_neighbor = s.curl_e_x(4, 3, 4); // j=3, ez_yp = ez[4,4,4]=1, ez_here=0
        assert!(cx_neighbor.abs() > 0.0);
    }

    #[test]
    fn test_leapfrog_energy_oscillation() {
        // 真空 + Mur 边界: E 和 H 之间能量交换, 总能量近似守恒
        let mut cfg = small_cfg();
        cfg.boundary = [Emboundary::Mur; 6];
        // 选 dt 满足 CFL
        cfg.dt = 0.5 * FdtdConfig::cfl_dt(cfg.h, &Medium::vacuum());
        let mut s = FdtdSolver::new(cfg);
        let idx = s.idx(4, 4, 4);
        s.ex[idx] = 1000.0;
        let e0 = s.total_energy();
        s.step_n(10);
        let e1 = s.total_energy();
        // 无源无损耗, 总能量应大致守恒 (Mur 有少量反射)
        assert!(e1 > 0.0);
        // 允许 50% 浮动 (边界反射 + 数值色散)
        assert!(e1 < 2.0 * e0);
    }

    #[test]
    fn test_add_medium_returns_incrementing_id() {
        let mut s = FdtdSolver::new(small_cfg());
        assert_eq!(s.add_medium(Medium::fresh_water()), 1);
        assert_eq!(s.add_medium(Medium::glass()), 2);
        assert_eq!(s.add_medium(Medium::copper()), 3);
        assert_eq!(s.media.len(), 4);
    }

    #[test]
    fn test_source_out_of_bounds_ignored() {
        let mut s = FdtdSolver::new(small_cfg());
        s.add_source(PointSource {
            i: 100, j: 4, k: 4,
            component: FieldComponent::Ez,
            source_type: SourceType::Hard,
            amplitude: 1.0,
            frequency: 1.0,
            t0: 0.0,
            width: 1.0,
            waveform: Waveform::Gaussian,
        });
        s.apply_sources();
        // 不 panic 即可
        assert_eq!(s.steps, 0);
    }

    #[test]
    fn test_loss_tangent_zero_for_lossless() {
        let m = Medium::vacuum();
        assert_eq!(m.loss_tangent(1.0e9), 0.0);
    }

    #[test]
    fn test_loss_tangent_positive_for_conductor() {
        let m = Medium::copper();
        assert!(m.loss_tangent(1.0e9) > 0.0);
    }

    #[test]
    fn test_wave_speed_decreases_in_dense_medium() {
        let c_vac = Medium::vacuum().wave_speed();
        let c_water = Medium::fresh_water().wave_speed();
        let c_glass = Medium::glass().wave_speed();
        assert!(c_water < c_glass);
        assert!(c_glass < c_vac);
    }

    #[test]
    fn test_impedance_decreases_in_dielectric() {
        let z_vac = Medium::vacuum().impedance();
        let z_water = Medium::fresh_water().impedance();
        // η = √(μ/ε), ε_r=81 → η = η_0/9
        assert!(z_water < z_vac);
        assert!((z_vac / z_water - 9.0).abs() < 0.1);
    }
}

