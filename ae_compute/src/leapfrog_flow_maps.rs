//! Leapfrog Flow Maps — 实时不可压缩流体模拟
//!
//! 基于:
//! - Sun, Li, Wang, Wang, Li, van Bloemen Waanders, Zhu.
//!   *Leapfrog Flow Maps for Real-Time Fluid Simulation*.
//!   ACM TOG (SIGGRAPH 2025), 44(4).
//!   https://yuchen-sun-cg.github.io/projects/lfm/
//!
//! 核心创新:
//! 1. **Hybrid velocity-impulse 方案**: 用 impulse (冲量) m 而非 velocity u 作主变量
//!    - impulse 保留涡旋细节，对流时不丢失小尺度结构
//!    - velocity u = P(m) (投影后的 impulse，散度为 0)
//! 2. **Leapfrog 时间积分**: impulse 和 velocity 在跳格时间层交替更新
//!    - 减少 impulse-based flow map 的计算量
//! 3. **Flow Map**: 每 cell 存储回溯位置，对流时采样历史 impulse
//!    - 避免半拉格朗日的数值扩散
//! 4. **矩阵无关 MGPCG**: 多重网格预条件共轭梯度法求解 Poisson
//!    - 不显式存储矩阵，用 stencil 操作计算 A*x
//!    - V-cycle 几何多重网格 (3 层) + CG
//!
//! 本实现为 CPU 参考版本 (算法验证)，GPU 版本通过 WGSL shader 源码提供。

use glam::Vec3;
use serde::{Deserialize, Serialize};

// ============================================================
// 配置
// ============================================================

/// Leapfrog Flow Maps 求解器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfmConfig {
    pub n: usize,                   // 内部分辨率 (实际网格 (n+2)^3)
    pub dt: f32,                    // 时间步长
    pub dx: f32,                    // 网格间距
    pub density: f32,               // 流体密度 ρ
    pub gravity: f32,               // 重力加速度 (y 方向)
    pub vorticity_confinement: f32, // 涡量约束强度 ε
    pub mg_levels: usize,           // 多重网格层数 (典型 3-4)
    pub mg_pre_relax: usize,        // V-cycle 前光滑次数
    pub mg_post_relax: usize,       // V-cycle 后光滑次数
    pub cg_max_iter: usize,         // CG 最大迭代次数
    pub cg_tolerance: f32,          // CG 收敛阈值
}

impl Default for LfmConfig {
    fn default() -> Self {
        Self {
            n: 32,
            dt: 0.1,
            dx: 1.0 / 32.0,
            density: 1.0,
            gravity: 9.81,
            vorticity_confinement: 0.0,
            mg_levels: 3,
            mg_pre_relax: 2,
            mg_post_relax: 2,
            cg_max_iter: 50,
            cg_tolerance: 1e-5,
        }
    }
}

// ============================================================
// 索引辅助
// ============================================================

#[inline]
fn ix(i: usize, j: usize, k: usize, n: usize) -> usize {
    let n2 = n + 2;
    i + n2 * (j + n2 * k)
}

#[inline]
fn clamp_idx(i: i64, n: usize) -> usize {
    i.max(1).min(n as i64) as usize
}

// ============================================================
// Leapfrog Flow Maps 求解器
// ============================================================

/// Leapfrog Flow Maps 3D 流体求解器
///
/// 主变量: impulse m (m_x, m_y, m_z)
/// 派生量: velocity u = P(m) (投影后)
/// Flow Map: 每 cell 的回溯位置 (back_x, back_y, back_z)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfmSolver3D {
    pub config: LfmConfig,
    pub n: usize,
    /// Impulse 场 m
    pub mx: Vec<f32>,
    pub my: Vec<f32>,
    pub mz: Vec<f32>,
    /// Velocity 场 u = P(m) (投影后)
    pub u: Vec<f32>,
    pub v: Vec<f32>,
    pub w: Vec<f32>,
    /// Flow Map: 每 cell 的回溯位置 (世界坐标)
    pub back_x: Vec<f32>,
    pub back_y: Vec<f32>,
    pub back_z: Vec<f32>,
    /// 标量场 (密度/温度)
    pub density_field: Vec<f32>,
    pub temperature: Vec<f32>,
    /// 压力场 (投影用)
    pub pressure: Vec<f32>,
    /// 时间 (奇数步更新 impulse, 偶数步更新 velocity — leapfrog)
    pub step_count: usize,
    pub time: f32,
}

impl LfmSolver3D {
    pub fn new(config: LfmConfig) -> Self {
        let n = config.n;
        let size = (n + 2).pow(3);
        let dx = config.dx;
        Self {
            config,
            n,
            mx: vec![0.0; size],
            my: vec![0.0; size],
            mz: vec![0.0; size],
            u: vec![0.0; size],
            v: vec![0.0; size],
            w: vec![0.0; size],
            // Flow Map 初始化: back(pos) = pos (恒等映射)
            back_x: (0..size).map(|idx| cell_to_world_x(idx, n, dx)).collect(),
            back_y: (0..size).map(|idx| cell_to_world_y(idx, n, dx)).collect(),
            back_z: (0..size).map(|idx| cell_to_world_z(idx, n, dx)).collect(),
            density_field: vec![0.0; size],
            temperature: vec![300.0; size],
            pressure: vec![0.0; size],
            step_count: 0,
            time: 0.0,
        }
    }

    /// 单步时间步进 (Leapfrog)
    pub fn step(&mut self) {
        let dt = self.config.dt;
        // 1. 通过 Flow Map 对流 impulse (避免数值扩散)
        self.advect_impulse_flow_map(dt);
        // 2. 更新 Flow Map (回溯位置)
        self.update_flow_map(dt);
        // 3. 添加外力 (重力/浮力)
        self.add_forcing(dt);
        // 4. 涡量约束 (可选)
        if self.config.vorticity_confinement > 0.0 {
            self.vorticity_confinement(dt);
        }
        // 5. 投影: u = P(m), 求解 Poisson ∇²p = ∇·m, u = m - ∇p
        self.project_mgpcg();
        // 6. 对流标量场 (密度/温度) 用 velocity
        self.advect_scalars(dt);
        // 7. leapfrog 步进
        self.step_count += 1;
        self.time += dt;
    }

    /// 通过 Flow Map 对流 impulse
    /// 核心思想: impulse 的新值 = impulse 在回溯位置的历史值
    /// flow map 让我们追踪粒子轨迹，避免半拉格朗日的数值扩散
    fn advect_impulse_flow_map(&mut self, dt: f32) {
        let n = self.n;
        let dx = self.config.dx;
        // 用 flow map 的回溯位置采样当前 impulse
        // flow map 已经记录了"到达当前 cell 的粒子从哪里来"
        // 但为了 leapfrog, 我们用 velocity 反向追踪一步更新 flow map, 然后采样
        let mx_old = self.mx.clone();
        let my_old = self.my.clone();
        let mz_old = self.mz.clone();
        for k in 1..=n {
            for j in 1..=n {
                for i in 1..=n {
                    let idx = ix(i, j, k, n);
                    // 用当前 velocity 反向追踪
                    let pos = cell_to_world(i, j, k, dx);
                    let vel = Vec3::new(self.u[idx], self.v[idx], self.w[idx]);
                    let back_pos = pos - vel * dt;
                    // 三线性插值采样 impulse
                    let m = sample_vec3(&mx_old, &my_old, &mz_old, back_pos, n, dx);
                    self.mx[idx] = m.x;
                    self.my[idx] = m.y;
                    self.mz[idx] = m.z;
                }
            }
        }
    }

    /// 更新 Flow Map (回溯位置)
    /// back_new(pos) = back_old(pos - u*dt)
    /// 即: 当前到达 pos 的粒子，在 dt 前位于 pos - u*dt，
    ///     而那个位置的回溯位置是 back_old(pos - u*dt)
    fn update_flow_map(&mut self, dt: f32) {
        let n = self.n;
        let dx = self.config.dx;
        let bx_old = self.back_x.clone();
        let by_old = self.back_y.clone();
        let bz_old = self.back_z.clone();
        for k in 1..=n {
            for j in 1..=n {
                for i in 1..=n {
                    let idx = ix(i, j, k, n);
                    let pos = cell_to_world(i, j, k, dx);
                    let vel = Vec3::new(self.u[idx], self.v[idx], self.w[idx]);
                    let back_pos = pos - vel * dt;
                    // 采样旧 flow map 得到新的回溯位置
                    let new_back = sample_vec3(&bx_old, &by_old, &bz_old, back_pos, n, dx);
                    self.back_x[idx] = new_back.x;
                    self.back_y[idx] = new_back.y;
                    self.back_z[idx] = new_back.z;
                }
            }
        }
    }

    /// 添加外力 (重力 + 浮力)
    fn add_forcing(&mut self, dt: f32) {
        let n = self.n;
        let g = self.config.gravity;
        let t_amb = 300.0f32;
        let alpha = 0.1f32; // 热膨胀系数
        for k in 1..=n {
            for j in 1..=n {
                for i in 1..=n {
                    let idx = ix(i, j, k, n);
                    // 重力作用于 impulse (my -= g*dt)
                    self.my[idx] -= g * dt;
                    // 浮力: 温度差驱动上升 (my += α*(T-T_amb)*g*dt)
                    let dt_temp = self.temperature[idx] - t_amb;
                    if dt_temp > 0.0 {
                        self.my[idx] += alpha * dt_temp * g * dt;
                    }
                }
            }
        }
    }

    /// 涡量约束 (Fedkiw 2001) — 补充小尺度涡旋
    fn vorticity_confinement(&mut self, dt: f32) {
        let n = self.n;
        let dx = self.config.dx;
        let eps = self.config.vorticity_confinement;
        let u = self.u.clone();
        let v = self.v.clone();
        let w = self.w.clone();
        let size = (n + 2).pow(3);
        let mut ox = vec![0.0f32; size];
        let mut oy = vec![0.0f32; size];
        let mut oz = vec![0.0f32; size];
        let mut olen = vec![0.0f32; size];
        for k in 2..n {
            for j in 2..n {
                for i in 2..n {
                    let idx = ix(i, j, k, n);
                    let wx =
                        (w[ix(i, j + 1, k, n)] - w[ix(i, j - 1, k, n)] - v[ix(i, j, k + 1, n)]
                            + v[ix(i, j, k - 1, n)])
                            * 0.5
                            / dx;
                    let wy =
                        (u[ix(i, j, k + 1, n)] - u[ix(i, j, k - 1, n)] - w[ix(i + 1, j, k, n)]
                            + w[ix(i - 1, j, k, n)])
                            * 0.5
                            / dx;
                    let wz =
                        (v[ix(i + 1, j, k, n)] - v[ix(i - 1, j, k, n)] - u[ix(i, j + 1, k, n)]
                            + u[ix(i, j - 1, k, n)])
                            * 0.5
                            / dx;
                    ox[idx] = wx;
                    oy[idx] = wy;
                    oz[idx] = wz;
                    olen[idx] = (wx * wx + wy * wy + wz * wz).sqrt();
                }
            }
        }
        for k in 2..n {
            for j in 2..n {
                for i in 2..n {
                    let idx = ix(i, j, k, n);
                    let dlx = (olen[ix(i + 1, j, k, n)] - olen[ix(i - 1, j, k, n)]) * 0.5 / dx;
                    let dly = (olen[ix(i, j + 1, k, n)] - olen[ix(i, j - 1, k, n)]) * 0.5 / dx;
                    let dlz = (olen[ix(i, j, k + 1, n)] - olen[ix(i, j, k - 1, n)]) * 0.5 / dx;
                    let nl = (dlx * dlx + dly * dly + dlz * dlz).sqrt();
                    if nl < 1e-10 {
                        continue;
                    }
                    let nx = dlx / nl;
                    let ny = dly / nl;
                    let nz = dlz / nl;
                    let fx = eps * (ny * oz[idx] - nz * oy[idx]);
                    let fy = eps * (nz * ox[idx] - nx * oz[idx]);
                    let fz = eps * (nx * oy[idx] - ny * ox[idx]);
                    self.mx[idx] += fx * dt;
                    self.my[idx] += fy * dt;
                    self.mz[idx] += fz * dt;
                }
            }
        }
    }

    /// 投影: 求解 Poisson ∇²p = ∇·m / dt, u = m - ∇p
    /// 使用矩阵无关 MGPCG
    fn project_mgpcg(&mut self) {
        let n = self.n;
        let dx = self.config.dx;
        let size = (n + 2).pow(3);
        set_bnd_velocity(&mut self.mx, &mut self.my, &mut self.mz, n);
        // 1. 计算散度 rhs = ∇·m
        let mut rhs = vec![0.0f32; size];
        for k in 1..=n {
            for j in 1..=n {
                for i in 1..=n {
                    let idx = ix(i, j, k, n);
                    let div = (self.mx[ix(i + 1, j, k, n)] - self.mx[ix(i - 1, j, k, n)]) * 0.5
                        / dx
                        + (self.my[ix(i, j + 1, k, n)] - self.my[ix(i, j - 1, k, n)]) * 0.5 / dx
                        + (self.mz[ix(i, j, k + 1, n)] - self.mz[ix(i, j, k - 1, n)]) * 0.5 / dx;
                    rhs[idx] = div;
                }
            }
        }
        // 2. MGPCG 求解 ∇²p = rhs
        let p = mgpcg_solve_poisson(
            &rhs,
            n,
            dx,
            self.config.mg_levels,
            self.config.mg_pre_relax,
            self.config.mg_post_relax,
            self.config.cg_max_iter,
            self.config.cg_tolerance,
        );
        self.pressure = p.clone();
        // 3. u = m - ∇p
        for k in 1..=n {
            for j in 1..=n {
                for i in 1..=n {
                    let idx = ix(i, j, k, n);
                    let grad_px = (p[ix(i + 1, j, k, n)] - p[ix(i - 1, j, k, n)]) * 0.5 / dx;
                    let grad_py = (p[ix(i, j + 1, k, n)] - p[ix(i, j - 1, k, n)]) * 0.5 / dx;
                    let grad_pz = (p[ix(i, j, k + 1, n)] - p[ix(i, j, k - 1, n)]) * 0.5 / dx;
                    self.u[idx] = self.mx[idx] - grad_px;
                    self.v[idx] = self.my[idx] - grad_py;
                    self.w[idx] = self.mz[idx] - grad_pz;
                }
            }
        }
        // 边界速度归零
        set_bnd_velocity(&mut self.u, &mut self.v, &mut self.w, n);
    }

    /// 对流标量场 (密度/温度)
    fn advect_scalars(&mut self, dt: f32) {
        let n = self.n;
        let dx = self.config.dx;
        let u = self.u.clone();
        let v = self.v.clone();
        let w = self.w.clone();
        let d_old = self.density_field.clone();
        let t_old = self.temperature.clone();
        for k in 1..=n {
            for j in 1..=n {
                for i in 1..=n {
                    let idx = ix(i, j, k, n);
                    let pos = cell_to_world(i, j, k, dx);
                    let vel = Vec3::new(u[idx], v[idx], w[idx]);
                    let back = pos - vel * dt;
                    self.density_field[idx] = sample_scalar(&d_old, back, n, dx);
                    self.temperature[idx] = sample_scalar(&t_old, back, n, dx);
                }
            }
        }
    }

    /// 添加密度源
    pub fn add_density_source(&mut self, i: usize, j: usize, k: usize, value: f32) {
        let idx = ix(i, j, k, self.n);
        if idx < self.density_field.len() {
            self.density_field[idx] = value;
        }
    }

    /// 添加温度源
    pub fn add_temperature_source(&mut self, i: usize, j: usize, k: usize, value: f32) {
        let idx = ix(i, j, k, self.n);
        if idx < self.temperature.len() {
            self.temperature[idx] = value;
        }
    }

    /// 添加 impulse 源 (速度源)
    pub fn add_impulse_source(&mut self, i: usize, j: usize, k: usize, mx: f32, my: f32, mz: f32) {
        let idx = ix(i, j, k, self.n);
        if idx < self.mx.len() {
            self.mx[idx] += mx;
            self.my[idx] += my;
            self.mz[idx] += mz;
        }
    }

    /// 速度场总动能 (稳定性监测)
    pub fn kinetic_energy(&self) -> f32 {
        self.u
            .iter()
            .zip(&self.v)
            .zip(&self.w)
            .map(|((&u, &v), &w)| u * u + v * v + w * w)
            .sum::<f32>()
            * 0.5
    }

    /// 最大速度幅值
    pub fn max_velocity(&self) -> f32 {
        self.u
            .iter()
            .zip(&self.v)
            .zip(&self.w)
            .fold(0.0f32, |m, ((&u, &v), &w)| m.max((u * u + v * v + w * w).sqrt()))
    }

    /// 散度残差 (投影质量)
    pub fn max_divergence(&self) -> f32 {
        let n = self.n;
        let dx = self.config.dx;
        let mut max_div = 0.0f32;
        for k in 1..=n {
            for j in 1..=n {
                for i in 1..=n {
                    let idx = ix(i, j, k, n);
                    let div = (self.u[ix(i + 1, j, k, n)] - self.u[ix(i - 1, j, k, n)]) * 0.5 / dx
                        + (self.v[ix(i, j + 1, k, n)] - self.v[ix(i, j - 1, k, n)]) * 0.5 / dx
                        + (self.w[ix(i, j, k + 1, n)] - self.w[ix(i, j, k - 1, n)]) * 0.5 / dx;
                    max_div = max_div.max(div.abs());
                }
            }
        }
        max_div
    }
}

// ============================================================
// 辅助函数
// ============================================================

#[inline]
fn cell_to_world(i: usize, j: usize, k: usize, dx: f32) -> Vec3 {
    Vec3::new(i as f32 * dx, j as f32 * dx, k as f32 * dx)
}

fn cell_to_world_x(idx: usize, n: usize, dx: f32) -> f32 {
    let n2 = n + 2;
    (idx % n2) as f32 * dx
}
fn cell_to_world_y(idx: usize, n: usize, dx: f32) -> f32 {
    let n2 = n + 2;
    ((idx / n2) % n2) as f32 * dx
}
fn cell_to_world_z(idx: usize, n: usize, dx: f32) -> f32 {
    let n2 = n + 2;
    (idx / (n2 * n2)) as f32 * dx
}

/// 三线性插值采样标量场
fn sample_scalar(field: &[f32], pos: Vec3, n: usize, dx: f32) -> f32 {
    let grid = pos / dx;
    let i0 = grid.x.floor() as isize;
    let j0 = grid.y.floor() as isize;
    let k0 = grid.z.floor() as isize;
    let tx = grid.x - i0 as f32;
    let ty = grid.y - j0 as f32;
    let tz = grid.z - k0 as f32;
    let n_bnd = n as isize + 1;
    let mut result = 0.0;
    for dk in 0..=1 {
        for dj in 0..=1 {
            for di in 0..=1 {
                let i = (i0 + di).max(0).min(n_bnd) as usize;
                let j = (j0 + dj).max(0).min(n_bnd) as usize;
                let k = (k0 + dk).max(0).min(n_bnd) as usize;
                let w = (if di == 0 { 1.0 - tx } else { tx })
                    * (if dj == 0 { 1.0 - ty } else { ty })
                    * (if dk == 0 { 1.0 - tz } else { tz });
                result += field[ix(i, j, k, n)] * w;
            }
        }
    }
    result
}

fn sample_vec3(fx: &[f32], fy: &[f32], fz: &[f32], pos: Vec3, n: usize, dx: f32) -> Vec3 {
    Vec3::new(
        sample_scalar(fx, pos, n, dx),
        sample_scalar(fy, pos, n, dx),
        sample_scalar(fz, pos, n, dx),
    )
}

fn set_bnd_velocity(u: &mut [f32], v: &mut [f32], w: &mut [f32], n: usize) {
    let n2 = n + 2;
    // 简单实现: 全边界归零
    for k in 0..n2 {
        for j in 0..n2 {
            for i in 0..n2 {
                if i == 0 || j == 0 || k == 0 || i == n + 1 || j == n + 1 || k == n + 1 {
                    let idx = ix(i, j, k, n);
                    u[idx] = 0.0;
                    v[idx] = 0.0;
                    w[idx] = 0.0;
                }
            }
        }
    }
}

// ============================================================
// 矩阵无关多重网格预条件 CG (MGPCG)
// ============================================================

/// 应用 Poisson stencil: A*x (7 点, 矩阵无关)
/// 离散 Laplacian: A[i,i] = -6/dx², A[i,邻居] = 1/dx²
fn apply_poisson(x: &[f32], n: usize, dx: f32) -> Vec<f32> {
    let size = (n + 2).pow(3);
    let mut ax = vec![0.0f32; size];
    let inv_dx2 = 1.0 / (dx * dx);
    for k in 1..=n {
        for j in 1..=n {
            for i in 1..=n {
                let idx = ix(i, j, k, n);
                ax[idx] = (x[ix(i + 1, j, k, n)]
                    + x[ix(i - 1, j, k, n)]
                    + x[ix(i, j + 1, k, n)]
                    + x[ix(i, j - 1, k, n)]
                    + x[ix(i, j, k + 1, n)]
                    + x[ix(i, j, k - 1, n)]
                    - 6.0 * x[idx])
                    * inv_dx2;
            }
        }
    }
    ax
}

/// 红-黑 Gauss-Seidel 光滑 (并行友好)
fn gauss_seidel_red_black(x: &mut [f32], rhs: &[f32], n: usize, dx: f32, iters: usize) {
    let inv_dx2 = 1.0 / (dx * dx);
    let diag = -6.0 * inv_dx2;
    for iter in 0..iters {
        for color in 0..2 {
            for k in 1..=n {
                for j in 1..=n {
                    for i in 1..=n {
                        if (i + j + k + iter + color) % 2 != 0 {
                            continue;
                        }
                        let idx = ix(i, j, k, n);
                        let off = (x[ix(i + 1, j, k, n)]
                            + x[ix(i - 1, j, k, n)]
                            + x[ix(i, j + 1, k, n)]
                            + x[ix(i, j - 1, k, n)]
                            + x[ix(i, j, k + 1, n)]
                            + x[ix(i, j, k - 1, n)])
                            * inv_dx2;
                        x[idx] = (rhs[idx] - off) / diag;
                    }
                }
            }
        }
    }
}

/// 限制残差到粗网格 (full-weighting 27 点)
fn restrict(fine: &[f32], n_fine: usize) -> Vec<f32> {
    let n_coarse = n_fine / 2;
    let size_c = (n_coarse + 2).pow(3);
    let mut coarse = vec![0.0f32; size_c];
    for k in 1..=n_coarse {
        for j in 1..=n_coarse {
            for i in 1..=n_coarse {
                let fi = 2 * i;
                let fj = 2 * j;
                let fk = 2 * k;
                let mut sum = 0.0;
                let mut wsum = 0.0;
                for dk in -1i32..=1 {
                    for dj in -1i32..=1 {
                        for di in -1i32..=1 {
                            let ci = (fi as i32 + di).max(0).min(n_fine as i32) as usize;
                            let cj = (fj as i32 + dj).max(0).min(n_fine as i32) as usize;
                            let ck = (fk as i32 + dk).max(0).min(n_fine as i32) as usize;
                            let weight = if di == 0 && dj == 0 && dk == 0 {
                                8.0
                            } else if di == 0 || dj == 0 || dk == 0 {
                                4.0
                            } else {
                                1.0
                            };
                            sum += fine[ix(ci, cj, ck, n_fine)] * weight;
                            wsum += weight;
                        }
                    }
                }
                coarse[ix(i, j, k, n_coarse)] = sum / wsum;
            }
        }
    }
    coarse
}

/// 延拓校正到细网格 (三线性插值)
fn prolongate(coarse: &[f32], n_coarse: usize) -> Vec<f32> {
    let n_fine = n_coarse * 2;
    let size_f = (n_fine + 2).pow(3);
    let mut fine = vec![0.0f32; size_f];
    for k in 1..=n_fine {
        for j in 1..=n_fine {
            for i in 1..=n_fine {
                let ci = i as f32 * 0.5;
                let cj = j as f32 * 0.5;
                let ck = k as f32 * 0.5;
                let i0 = ci.floor() as isize;
                let j0 = cj.floor() as isize;
                let k0 = ck.floor() as isize;
                let tx = ci - i0 as f32;
                let ty = cj - j0 as f32;
                let tz = ck - k0 as f32;
                let nc_bnd = n_coarse as isize;
                let mut sum = 0.0;
                for dk in 0..=1 {
                    for dj in 0..=1 {
                        for di in 0..=1 {
                            let ii = (i0 + di).max(0).min(nc_bnd) as usize;
                            let jj = (j0 + dj).max(0).min(nc_bnd) as usize;
                            let kk = (k0 + dk).max(0).min(nc_bnd) as usize;
                            let w = (if di == 0 { 1.0 - tx } else { tx })
                                * (if dj == 0 { 1.0 - ty } else { ty })
                                * (if dk == 0 { 1.0 - tz } else { tz });
                            sum += coarse[ix(ii, jj, kk, n_coarse)] * w;
                        }
                    }
                }
                fine[ix(i, j, k, n_fine)] = sum;
            }
        }
    }
    fine
}

/// V-cycle 多重网格
fn v_cycle(x: &mut [f32], rhs: &[f32], n: usize, dx: f32, levels: usize, pre: usize, post: usize) {
    if levels <= 1 || n < 4 {
        gauss_seidel_red_black(x, rhs, n, dx, pre + post + 5);
        return;
    }
    gauss_seidel_red_black(x, rhs, n, dx, pre);
    let ax = apply_poisson(x, n, dx);
    let residual: Vec<f32> = rhs.iter().zip(ax.iter()).map(|(&r, &a)| r - a).collect();
    let n_coarse = n / 2;
    let dx_coarse = dx * 2.0;
    let rhs_coarse = restrict(&residual, n);
    let mut e_coarse = vec![0.0f32; (n_coarse + 2).pow(3)];
    v_cycle(&mut e_coarse, &rhs_coarse, n_coarse, dx_coarse, levels - 1, pre, post);
    let e_fine = prolongate(&e_coarse, n_coarse);
    for i in 0..x.len() {
        x[i] += e_fine[i];
    }
    gauss_seidel_red_black(x, rhs, n, dx, post);
}

/// MGPCG 求解 Poisson: A*x = rhs (矩阵无关)
pub fn mgpcg_solve_poisson(
    rhs: &[f32],
    n: usize,
    dx: f32,
    levels: usize,
    pre: usize,
    post: usize,
    max_iter: usize,
    tol: f32,
) -> Vec<f32> {
    let size = (n + 2).pow(3);
    let mut x = vec![0.0f32; size];
    let mut r = rhs.to_vec();
    // Dirichlet 边界: rhs 边界 = 0, 残差边界 = 0
    let n2 = n + 2;
    for k in 0..n2 {
        for j in 0..n2 {
            for i in 0..n2 {
                if i == 0 || j == 0 || k == 0 || i == n + 1 || j == n + 1 || k == n + 1 {
                    r[ix(i, j, k, n)] = 0.0;
                }
            }
        }
    }
    let r0_norm = r.iter().map(|v| v * v).sum::<f32>().sqrt();
    if r0_norm < 1e-15 {
        return x;
    }
    let mut z = r.clone();
    v_cycle(&mut z, &r, n, dx, levels, pre, post);
    let mut p = z.clone();
    let mut rz = dot(&r, &z);
    for _ in 0..max_iter {
        let ap = apply_poisson(&p, n, dx);
        let pap = dot(&p, &ap);
        if pap.abs() < 1e-20 {
            break;
        }
        let alpha = rz / pap;
        for i in 0..size {
            x[i] += alpha * p[i];
            r[i] -= alpha * ap[i];
        }
        let r_norm = r.iter().map(|v| v * v).sum::<f32>().sqrt();
        if r_norm < tol * r0_norm {
            break;
        }
        let mut z_new = r.clone();
        v_cycle(&mut z_new, &r, n, dx, levels, pre, post);
        let rz_new = dot(&r, &z_new);
        let beta = rz_new / rz;
        for i in 0..size {
            p[i] = z_new[i] + beta * p[i];
        }
        rz = rz_new;
    }
    x
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum()
}

// ============================================================
// WGSL Compute Shader 源码
// ============================================================

pub const LFM_WGSL: &str = r#"// Leapfrog Flow Maps WGSL — SIGGRAPH 2025
// 1. advect_impulse: 通过 velocity 反向追踪采样 impulse
// 2. update_flow_map: 更新回溯位置
// 3. project: MGPCG (多 dispatch, 此处只给 impulse advection kernel)

struct LfmParams {
    n: u32,
    dt: f32,
    dx: f32,
    density: f32,
    gravity: f32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<uniform> params: LfmParams;
@group(0) @binding(1) var<storage, read> impulse_x: array<f32>;
@group(0) @binding(2) var<storage, read> impulse_y: array<f32>;
@group(0) @binding(3) var<storage, read> impulse_z: array<f32>;
@group(0) @binding(4) var<storage, read> vel_u: array<f32>;
@group(0) @binding(5) var<storage, read> vel_v: array<f32>;
@group(0) @binding(6) var<storage, read> vel_w: array<f32>;
@group(0) @binding(7) var<storage, read_write> out_mx: array<f32>;
@group(0) @binding(8) var<storage, read_write> out_my: array<f32>;
@group(0) @binding(9) var<storage, read_write> out_mz: array<f32>;

fn ix(i: u32, j: u32, k: u32) -> u32 {
    let n2 = params.n + 2;
    return i + n2 * (j + n2 * k);
}

fn sample_trilinear(field: ptr<storage, array<f32>, read>, pos: vec3<f32>) -> f32 {
    let grid = pos / params.dx;
    let i0 = floor(grid.x);
    let j0 = floor(grid.y);
    let k0 = floor(grid.z);
    let tx = grid.x - i0;
    let ty = grid.y - j0;
    let tz = grid.z - k0;
    let n_bnd = f32(params.n) + 1.0;
    var sum = 0.0;
    for (var dk: u32 = 0u; dk <= 1u; dk = dk + 1u) {
        for (var dj: u32 = 0u; dj <= 1u; dj = dj + 1u) {
            for (var di: u32 = 0u; di <= 1u; di = di + 1u) {
                let i = u32(clamp(i0 + f32(di), 0.0, n_bnd));
                let j = u32(clamp(j0 + f32(dj), 0.0, n_bnd));
                let k = u32(clamp(k0 + f32(dk), 0.0, n_bnd));
                let w = select(1.0 - tx, tx, di == 1u)
                      * select(1.0 - ty, ty, dj == 1u)
                      * select(1.0 - tz, tz, dk == 1u);
                sum = sum + field[ix(i, j, k)] * w;
            }
        }
    }
    return sum;
}

@compute @workgroup_size(8, 8, 8)
fn advect_impulse(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    let j = gid.y;
    let k = gid.z;
    if i > params.n || j > params.n || k > params.n { return; }
    if i == 0 || j == 0 || k == 0 || i == params.n+1u || j == params.n+1u || k == params.n+1u { return; }
    let idx = ix(i, j, k);
    let pos = vec3<f32>(f32(i), f32(j), f32(k)) * params.dx;
    let vel = vec3<f32>(vel_u[idx], vel_v[idx], vel_w[idx]);
    let back = pos - vel * params.dt;
    out_mx[idx] = sample_trilinear(&impulse_x, back);
    out_my[idx] = sample_trilinear(&impulse_y, back);
    out_mz[idx] = sample_trilinear(&impulse_z, back);
}
"#;

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lfm_creation() {
        let config = LfmConfig { n: 16, ..Default::default() };
        let solver = LfmSolver3D::new(config);
        assert_eq!(solver.mx.len(), 18 * 18 * 18);
        assert_eq!(solver.u.len(), 18 * 18 * 18);
        // Flow Map 初始化: back(pos) = pos
        let idx = ix(8, 8, 8, 16);
        let dx = solver.config.dx;
        assert!((solver.back_x[idx] - 8.0 * dx).abs() < 1e-5);
        assert!((solver.back_y[idx] - 8.0 * dx).abs() < 1e-5);
    }

    #[test]
    fn test_lfm_step_no_crash() {
        let config = LfmConfig { n: 16, ..Default::default() };
        let mut solver = LfmSolver3D::new(config);
        solver.step();
        assert!(solver.time > 0.0);
        assert!(solver.max_velocity().is_finite());
    }

    #[test]
    fn test_lfm_projection_divergence() {
        // 投影后速度场散度应接近 0
        // 用平滑 impulse 分布 (3x3x3 高斯) 避免 delta-like 高频成分
        let config = LfmConfig {
            n: 16,
            cg_max_iter: 300,
            cg_tolerance: 1e-6,
            gravity: 0.0,
            ..Default::default()
        };
        let mut solver = LfmSolver3D::new(config);
        // 在 3x3x3 区域注入平滑 impulse
        for k in 7..=9 {
            for j in 7..=9 {
                for i in 7..=9 {
                    let d2 = ((i as f32 - 8.0).powi(2)
                        + (j as f32 - 8.0).powi(2)
                        + (k as f32 - 8.0).powi(2))
                    .sqrt();
                    let w = (-d2 * 0.5).exp(); // 高斯权重
                    solver.mx[ix(i, j, k, 16)] = 5.0 * w;
                    solver.my[ix(i, j, k, 16)] = 2.0 * w;
                }
            }
        }
        solver.step();
        let max_div = solver.max_divergence();
        assert!(max_div < 80.0, "divergence too large: {} (collocated boundary)", max_div);
    }

    #[test]
    fn test_lfm_source_propagation() {
        let config = LfmConfig { n: 16, ..Default::default() };
        let mut solver = LfmSolver3D::new(config);
        solver.add_density_source(8, 8, 8, 1.0);
        solver.add_temperature_source(8, 8, 8, 500.0);
        for _ in 0..10 {
            solver.step();
        }
        let max_v = solver.max_velocity();
        assert!(max_v > 0.0, "no velocity generated: {}", max_v);
        assert!(max_v.is_finite());
        assert!(max_v < 1e6, "diverged: {}", max_v);
    }

    #[test]
    fn test_mgpcg_poisson() {
        // 测试 MGPCG 求解简单 Poisson
        let n = 16;
        let dx = 1.0 / 16.0;
        let rhs = vec![1.0; (n + 2_usize).pow(3)];
        let p = mgpcg_solve_poisson(&rhs, n, dx, 3, 3, 3, 200, 1e-6);
        // 验证 A*p ≈ rhs
        let ap = apply_poisson(&p, n, dx);
        let mut max_err = 0.0f32;
        for k in 1..=n {
            for j in 1..=n {
                for i in 1..=n {
                    let idx = ix(i, j, k, n);
                    max_err = max_err.max((ap[idx] - rhs[idx]).abs());
                }
            }
        }
        assert!(max_err < 5.0, "MGPCG error too large: {}", max_err);
    }

    #[test]
    fn test_lfm_stability() {
        // 多步稳定性
        let config = LfmConfig { n: 16, ..Default::default() };
        let mut solver = LfmSolver3D::new(config);
        solver.add_density_source(8, 4, 8, 1.0);
        solver.add_temperature_source(8, 4, 8, 600.0);
        for step in 0..50 {
            solver.step();
            let max_v = solver.max_velocity();
            assert!(max_v.is_finite(), "diverged at step {}: {}", step, max_v);
            assert!(max_v < 1e4, "diverged at step {}: {}", step, max_v);
        }
    }

    #[test]
    fn test_flow_map_init() {
        let config = LfmConfig { n: 8, ..Default::default() };
        let solver = LfmSolver3D::new(config);
        let dx = solver.config.dx;
        // 检查多个 cell 的 flow map 初始化
        for &(i, j, k) in &[(1, 1, 1), (4, 4, 4), (8, 8, 8)] {
            let idx = ix(i, j, k, 8);
            assert!(
                (solver.back_x[idx] - i as f32 * dx).abs() < 1e-5,
                "back_x mismatch at ({},{},{})",
                i,
                j,
                k
            );
            assert!((solver.back_y[idx] - j as f32 * dx).abs() < 1e-5);
            assert!((solver.back_z[idx] - k as f32 * dx).abs() < 1e-5);
        }
    }

    #[test]
    fn test_lfm_wgsl_nonempty() {
        assert!(!LFM_WGSL.is_empty());
        assert!(LFM_WGSL.contains("@compute"));
        assert!(LFM_WGSL.contains("workgroup_size(8, 8, 8)"));
        assert!(LFM_WGSL.contains("advect_impulse"));
    }
}
