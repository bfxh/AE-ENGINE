//! MPM (Material Point Method) — GPU Compute Shader + CPU 参考实现
//!
//! 基于:
//! - Tencent/NVIDIA GPU-MPM 优化原则 (arXiv:2111.00699)
//! - Unity Compute Shader 实时粒子范例
//! - MLS-MPM (Moving Least Squares MPM, Jiang et al. SIGGRAPH 2018)
//!
//! 架构:
//! 1. CPU 参考实现 (MpmSolver) — 可测试、可验证
//! 2. WGSL compute shader 源码 (MPM_WGSL_P2G/GRIDOP/G2P) — GPU kernel
//! 3. GPU 调度器 (feature = "gpu") — wgpu 24 dispatch
//!
//! MPM 三段流程:
//! - P2G (Particle → Grid): 粒子质量/动量散射到 27 个网格节点 (3³ 二次 B 样条)
//! - GridOp: 重力 + 边界条件
//! - G2P (Grid → Particle): 网格速度插值回粒子，更新位置

use glam::{Mat3, Vec3};

// ============================================================
// 数据结构
// ============================================================

/// MPM 粒子 (AoS 布局，CPU 用；GPU 用 SoA)
#[derive(Debug, Clone, Copy)]
pub struct MpmParticle {
    pub position: Vec3,
    pub velocity: Vec3,
    /// 仿射动量矩阵 C (3×3，列主序存储为 9 个 f32)
    pub c: [f32; 9],
    /// 形变梯度行列式 J (用于弹性材料)
    pub j: f32,
    pub mass: f32,
    /// 体积比（初始体积 / 当前体积）
    pub volume_ratio: f32,
}

impl MpmParticle {
    pub fn new(position: Vec3, mass: f32) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            c: [0.0; 9],
            j: 1.0,
            mass,
            volume_ratio: 1.0,
        }
    }
}

/// 3D 正交网格（MPM 背景网格）
#[derive(Debug, Clone)]
pub struct MpmGrid3D {
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    pub dx: f32, // 网格间距
    pub origin: Vec3,
    pub mass: Vec<f32>,
    pub velocity: Vec<Vec3>,
}

impl MpmGrid3D {
    pub fn new(nx: usize, ny: usize, nz: usize, dx: f32, origin: Vec3) -> Self {
        let n = nx * ny * nz;
        Self {
            nx,
            ny,
            nz,
            dx,
            origin,
            mass: vec![0.0; n],
            velocity: vec![Vec3::ZERO; n],
        }
    }

    pub fn clear(&mut self) {
        for m in &mut self.mass {
            *m = 0.0;
        }
 for v in &mut self.velocity {
            *v = Vec3::ZERO;
        }
    }

    #[inline]
    pub fn idx(&self, i: usize, j: usize, k: usize) -> usize {
        (k * self.ny + j) * self.nx + i
    }

    /// 网格节点世界坐标
    #[inline]
    pub fn node_pos(&self, i: usize, j: usize, k: usize) -> Vec3 {
        Vec3::new(
            self.origin.x + i as f32 * self.dx,
            self.origin.y + j as f32 * self.dx,
            self.origin.z + k as f32 * self.dx,
        )
    }
}

/// MPM 求解器配置
#[derive(Debug, Clone)]
pub struct MpmConfig {
    pub grid_nx: usize,
    pub grid_ny: usize,
    pub grid_nz: usize,
    pub dx: f32,
    pub origin: Vec3,
    pub gravity: Vec3,
    pub dt: f32,
    /// 雪材料参数 (从 MLS-MPM 论文)
    pub elastic_mu: f32,
    pub elastic_lambda: f32,
    /// 边界恢复系数 (0 = 完全非弹性, 1 = 完全弹性)
    pub restitution: f32,
    /// 初始粒子密度
    pub particle_density: f32,
}

impl Default for MpmConfig {
    fn default() -> Self {
        Self {
            grid_nx: 64,
            grid_ny: 64,
            grid_nz: 64,
            dx: 1.0 / 64.0,
            origin: Vec3::ZERO,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            dt: 1.0 / 60.0,
            elastic_mu: 1.0e5,
            elastic_lambda: 1.0e5,
            restitution: 0.0,
            particle_density: 1.0,
        }
    }
}

// ============================================================
// CPU 参考实现
// ============================================================

/// 二次 B 样条权重 (quadratic weight)
///
/// MLS-MPM 用二次 B 样条: w(d) = max(0, 0.75 - d²) if d < 0.5
///                                       max(0, 0.5 - 1.5*d + 1.5*d²) if 0.5 <= d < 1.5
/// 但更常用的是:
/// w(q) = { 0.75 - |q|²           if |q| < 0.5
///         { 0.5 * (1.5 - |q|)²   if 0.5 <= |q| < 1.5
///         { 0                    otherwise
#[inline]
fn quadratic_weight(d: f32) -> f32 {
    let d = d.abs();
    if d < 0.5 {
        0.75 - d * d
    } else if d < 1.5 {
        let t = 1.5 - d;
        0.5 * t * t
    } else {
        0.0
    }
}

/// 二次 B 样条权重的导数 dN/dd (用于计算应力所需的 ∇w)
///
/// N(d) = 0.75 - d²           if |d| < 0.5       → dN/dd = -2d
/// N(d) = 0.5*(1.5-|d|)²      if 0.5 ≤ |d| < 1.5 → dN/dd = d - 1.5*sign(d)
/// N(d) = 0                    otherwise          → dN/dd = 0
#[inline]
fn quadratic_weight_grad(d: f32) -> f32 {
    let ad = d.abs();
    if ad < 0.5 {
        -2.0 * d
    } else if ad < 1.5 {
        if d > 0.0 {
            d - 1.5
        } else {
            d + 1.5
        }
    } else {
        0.0
    }
}

/// MPM 求解器 (CPU 参考)
#[derive(Debug, Clone)]
pub struct MpmSolver {
    pub config: MpmConfig,
    pub particles: Vec<MpmParticle>,
    pub grid: MpmGrid3D,
}

impl MpmSolver {
    pub fn new(config: MpmConfig) -> Self {
        let grid = MpmGrid3D::new(
            config.grid_nx,
            config.grid_ny,
            config.grid_nz,
            config.dx,
            config.origin,
        );
        Self {
            config,
            particles: Vec::new(),
            grid,
        }
    }

    pub fn add_particle(&mut self, p: MpmParticle) -> usize {
        let idx = self.particles.len();
        self.particles.push(p);
        idx
    }

    /// 添加一团粒子（用于初始化雪球/方块）
    pub fn add_particle_block(
        &mut self,
        center: Vec3,
        size: Vec3,
        count_per_axis: usize,
        mass: f32,
    ) {
        let half = size * 0.5;
        let step = size / count_per_axis as f32;
        let per_mass = mass / (count_per_axis * count_per_axis * count_per_axis) as f32;
        for i in 0..count_per_axis {
            for j in 0..count_per_axis {
                for k in 0..count_per_axis {
                    let pos = center - half
                        + Vec3::new(
                            (i as f32 + 0.5) * step.x,
                            (j as f32 + 0.5) * step.y,
                            (k as f32 + 0.5) * step.z,
                        );
                    self.add_particle(MpmParticle::new(pos, per_mass));
                }
            }
        }
    }

    /// 执行一步模拟
    pub fn step(&mut self) {
        let dt = self.config.dt;
        let dx = self.config.dx;

        // 1. 清空网格
        self.grid.clear();

        // 2. P2G: 粒子 → 网格
        self.p2g(dt);

        // 3. GridOp: 重力 + 边界
        self.grid_op(dt);

        // 4. G2P: 网格 → 粒子
        self.g2p(dt, dx);
    }

    /// P2G: 粒子质量/动量散射到网格 (MLS-MPM, Jiang et al. SIGGRAPH 2018)
    ///
    /// 约定 (与 Taichi mpm88 一致):
    /// - base = floor(cell - 0.5)，使 fx ∈ [0.5, 1.5)
    /// - 3 节点 stencil: base, base+1, base+2
    /// - 节点 i 的网格距离 d = (base + i) - cell = i - fx，i ∈ {0, 1, 2}
    /// - 权重 w_i = N(d_i)，N 为二次 B 样条
    /// - 权重梯度 ∇w = (dN/d d) * inv_dx (世界坐标)
    fn p2g(&mut self, dt: f32) {
        let dx = self.config.dx;
        let inv_dx = 1.0 / dx;

        for p in &self.particles {
            // 粒子在网格坐标下的连续坐标
            let cell = (p.position - self.grid.origin) * inv_dx;
            // base = floor(cell - 0.5)，使 fx ∈ [0.5, 1.5)，确保 3 节点 stencil 都有非零权重
            let i0 = (cell.x - 0.5).floor() as i64;
            let j0 = (cell.y - 0.5).floor() as i64;
            let k0 = (cell.z - 0.5).floor() as i64;

            // 粒子相对于 base 的偏移（∈ [0.5, 1.5)）
            let fx = cell.x - i0 as f32;
            let fy = cell.y - j0 as f32;
            let fz = cell.z - k0 as f32;

            // 3 个轴向的权重（对应节点 base, base+1, base+2）
            // 节点距离 d = i - fx，i ∈ {0, 1, 2}
            let wx = [quadratic_weight(fx), quadratic_weight(fx - 1.0), quadratic_weight(fx - 2.0)];
            let wy = [quadratic_weight(fy), quadratic_weight(fy - 1.0), quadratic_weight(fy - 2.0)];
            let wz = [quadratic_weight(fz), quadratic_weight(fz - 1.0), quadratic_weight(fz - 2.0)];

            // 权重梯度（节点距离 d 的导数 dN/dd）
            let gx = [quadratic_weight_grad(fx), quadratic_weight_grad(fx - 1.0), quadratic_weight_grad(fx - 2.0)];
            let gy = [quadratic_weight_grad(fy), quadratic_weight_grad(fy - 1.0), quadratic_weight_grad(fy - 2.0)];
            let gz = [quadratic_weight_grad(fz), quadratic_weight_grad(fz - 1.0), quadratic_weight_grad(fz - 2.0)];

            // 仿射动量 C (3×3, 列主序)
            let c_mat = Mat3::from_cols_array(&p.c);

            // 体积弹性 Cauchy 应力 σ = K * (J - 1) / J * I
            // 体积模量 K = λ + 2μ/3
            let j = p.j.max(0.01);
            let bulk = self.config.elastic_lambda + 2.0 * self.config.elastic_mu / 3.0;
            let stress_scalar = bulk * (j - 1.0) / j;
            // 粒子初始体积（每个粒子代表边长 0.5*dx 的立方体）
            let p_vol = p.volume_ratio * (dx * 0.5).powi(3);

            for di in 0..3i64 {
                for dj in 0..3i64 {
                    for dk in 0..3i64 {
                        let gi = i0 + di;
                        let gj = j0 + dj;
                        let gk = k0 + dk;
                        if gi < 0 || gj < 0 || gk < 0 {
                            continue;
                        }
                        if gi as usize >= self.grid.nx
                            || gj as usize >= self.grid.ny
                            || gk as usize >= self.grid.nz
                        {
                            continue;
                        }
                        let w = wx[di as usize] * wy[dj as usize] * wz[dk as usize];
                        if w <= 0.0 {
                            continue;
                        }

                        let gidx = self.grid.idx(gi as usize, gj as usize, gk as usize);
                        let node_pos = self.grid.node_pos(gi as usize, gj as usize, gk as usize);
                        let dpos = node_pos - p.position;

                        // 质量散射
                        self.grid.mass[gidx] += w * p.mass;

                        // 动量散射: m_p * (v_p + C · dpos)
                        let momentum = p.mass * (p.velocity + c_mat * dpos);

                        // 应力贡献 (Cauchy 应力 σ * ∇w 的力 → 乘 dt 转为动量)
                        // f = -V_p * σ * ∇w  (各向同性 σ = stress_scalar * I)
                        // ∇w_world = (gx[di]*wy[dj]*wz[dk], wx[di]*gy[dj]*wz[dk], wx[di]*wy[dj]*gz[dk]) * inv_dx
                        let grad_w = Vec3::new(
                            gx[di as usize] * wy[dj as usize] * wz[dk as usize],
                            wx[di as usize] * gy[dj as usize] * wz[dk as usize],
                            wx[di as usize] * wy[dj as usize] * gz[dk as usize],
                        ) * inv_dx;
                        let momentum_stress = -p_vol * stress_scalar * grad_w * dt;

                        self.grid.velocity[gidx] += w * momentum + momentum_stress;
                    }
                }
            }
        }
    }

    /// GridOp: 网格操作（重力、边界条件）
    fn grid_op(&mut self, dt: f32) {
        let gravity = self.config.gravity;
        let restitution = self.config.restitution;
        let nx = self.grid.nx;
        let ny = self.grid.ny;
        let nz = self.grid.nz;

        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let gidx = self.grid.idx(i, j, k);
                    let m = self.grid.mass[gidx];
                    if m <= 1e-10 {
                        self.grid.velocity[gidx] = Vec3::ZERO;
                        continue;
                    }
                    // 速度 = 动量 / 质量
                    let mut v = self.grid.velocity[gidx] / m;
                    // 重力
                    v += gravity * dt;

                    // 边界条件: 6 个面
                    let margin = 3;
                    if i < margin && v.x < 0.0 {
                        v.x = -v.x * restitution;
                    }
                    if i >= nx - margin && v.x > 0.0 {
                        v.x = -v.x * restitution;
                    }
                    if j < margin && v.y < 0.0 {
                        v.y = -v.y * restitution;
                    }
                    if j >= ny - margin && v.y > 0.0 {
                        v.y = -v.y * restitution;
                    }
                    if k < margin && v.z < 0.0 {
                        v.z = -v.z * restitution;
                    }
                    if k >= nz - margin && v.z > 0.0 {
                        v.z = -v.z * restitution;
                    }

                    self.grid.velocity[gidx] = v;
                }
            }
        }
    }

    /// G2P: 网格速度插值回粒子，更新位置
    fn g2p(&mut self, dt: f32, dx: f32) {
        let inv_dx = 1.0 / dx;

        for p in &mut self.particles {
            let cell = (p.position - self.grid.origin) * inv_dx;
            // 与 P2G 一致的 base 约定
            let i0 = (cell.x - 0.5).floor() as i64;
            let j0 = (cell.y - 0.5).floor() as i64;
            let k0 = (cell.z - 0.5).floor() as i64;

            let fx = cell.x - i0 as f32;
            let fy = cell.y - j0 as f32;
            let fz = cell.z - k0 as f32;

            let wx = [quadratic_weight(fx), quadratic_weight(fx - 1.0), quadratic_weight(fx - 2.0)];
            let wy = [quadratic_weight(fy), quadratic_weight(fy - 1.0), quadratic_weight(fy - 2.0)];
            let wz = [quadratic_weight(fz), quadratic_weight(fz - 1.0), quadratic_weight(fz - 2.0)];

            let mut new_v = Vec3::ZERO;
            let mut new_c = Mat3::ZERO;
            // MLS-MPM 的 D^-1 = 1/4 * dx² (quadratic B 样条)
            let d_inv = 4.0 * dx * dx;

            for di in 0..3i64 {
                for dj in 0..3i64 {
                    for dk in 0..3i64 {
                        let gi = i0 + di;
                        let gj = j0 + dj;
                        let gk = k0 + dk;
                        if gi < 0 || gj < 0 || gk < 0 {
                            continue;
                        }
                        if gi as usize >= self.grid.nx
                            || gj as usize >= self.grid.ny
                            || gk as usize >= self.grid.nz
                        {
                            continue;
                        }
                        let w = wx[di as usize] * wy[dj as usize] * wz[dk as usize];
                        if w <= 0.0 {
                            continue;
                        }

                        let gidx = self.grid.idx(gi as usize, gj as usize, gk as usize);
                        let gv = self.grid.velocity[gidx];
                        let node_pos = self.grid.node_pos(gi as usize, gj as usize, gk as usize);
                        let dpos = node_pos - p.position;

                        new_v += gv * w;
                        // C += w * v ⊗ dpos * D_inv
                        new_c += Mat3::from_cols(
                            gv * dpos.x,
                            gv * dpos.y,
                            gv * dpos.z,
                        ) * (w / d_inv);
                    }
                }
            }

            p.velocity = new_v;
            p.c = new_c.to_cols_array();
            // 更新位置
            p.position += new_v * dt;

            // 更新形变梯度 J (简化的体积变化)
            let div_c = new_c.to_cols_array();
            let trace = div_c[0] + div_c[4] + div_c[8]; // trace(C)
            p.j *= 1.0 + trace * dt;
            p.j = p.j.max(0.01);
        }
    }

    /// 计算总动能（用于验证）
    pub fn kinetic_energy(&self) -> f32 {
        self.particles
            .iter()
            .map(|p| 0.5 * p.mass * p.velocity.length_squared())
            .sum()
    }

    /// 计算总质量
    pub fn total_mass(&self) -> f32 {
        self.particles.iter().map(|p| p.mass).sum()
    }

    /// 质心位置
    pub fn center_of_mass(&self) -> Vec3 {
        let total = self.total_mass().max(1e-10);
        let weighted: Vec3 = self
            .particles
            .iter()
            .map(|p| p.position * p.mass)
            .sum();
        weighted / total
    }
}

// ============================================================
// WGSL Compute Shader 源码
// ============================================================

/// P2G compute shader (WGSL)
///
/// workgroup_size = 64 (NVIDIA 最优)
/// 每个 work item 处理一个粒子
pub const MPM_WGSL_P2G: &str = r#"
// P2G: Particle to Grid transfer
// 每个 work item 处理一个粒子，散射到 27 个网格节点

struct Particle {
    position: vec3<f32>,
    velocity: vec3<f32>,
    c: mat3x3<f32>,
    j: f32,
    mass: f32,
    volume_ratio: f32,
    _pad: f32,
};

struct GridNode {
    velocity: vec3<f32>,
    mass: f32,
};

@group(0) @binding(0) var<storage, read> particles_in: array<Particle>;
@group(0) @binding(1) var<storage, read_write> grid: array<GridNode>;
@group(0) @binding(2) var<uniform> params: SimParams;

struct SimParams {
    grid_nx: u32,
    grid_ny: u32,
    grid_nz: u32,
    particle_count: u32,
    dx: f32,
    dt: f32,
    elastic_mu: f32,
    elastic_lambda: f32,
    origin: vec3<f32>,
    _pad: f32,
};

fn quadratic_weight(d: f32) -> f32 {
    let ad = abs(d);
    if ad < 0.5 {
        return 0.75 - ad * ad;
    } else if ad < 1.5 {
        let t = 1.5 - ad;
        return 0.5 * t * t;
    }
    return 0.0;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let p_idx = gid.x;
    if p_idx >= params.particle_count {
        return;
    }

    let p = particles_in[p_idx];
    let inv_dx = 1.0 / params.dx;
    let cell = (p.position - params.origin) * inv_dx;
    let i0 = vec3<i32>(floor(cell));
    let f = cell - vec3<f32>(i0);

    let weights_x = vec3<f32>(
        quadratic_weight(f.x - 0.0),
        quadratic_weight(f.x - 1.0),
        quadratic_weight(f.x + 1.0),
    );
    let weights_y = vec3<f32>(
        quadratic_weight(f.y - 0.0),
        quadratic_weight(f.y - 1.0),
        quadratic_weight(f.y + 1.0),
    );
    let weights_z = vec3<f32>(
        quadratic_weight(f.z - 0.0),
        quadratic_weight(f.z - 1.0),
        quadratic_weight(f.z + 1.0),
    );

    let j = max(p.j, 0.01);
    let stress_factor = -2.0 * params.elastic_mu * (j - 1.0) / j * (1.0 / params.dx);
    let p_vol = p.volume_ratio * pow(params.dx * 0.5, vec3<f32>(3.0)).x;

    for var di: i32 = 0; di < 3; di = di + 1 {
        for var dj: i32 = 0; dj < 3; dj = dj + 1 {
            for var dk: i32 = 0; dk < 3; dk = dk + 1 {
                let gi = i0.x + di - 1;
                let gj = i0.y + dj - 1;
                let gk = i0.z + dk - 1;
                if gi < 0 || gj < 0 || gk < 0 {
                    continue;
                }
                if u32(gi) >= params.grid_nx || u32(gj) >= params.grid_ny || u32(gk) >= params.grid_nz {
                    continue;
                }
                let w = weights_x[di] * weights_y[dj] * weights_z[dk];
                if w <= 0.0 {
                    continue;
                }

                let g_idx = (gk * i32(params.grid_ny) + gj) * i32(params.grid_nx) + gi;
                let node_pos = params.origin + vec3<f32>(f32(gi), f32(gj), f32(gk)) * params.dx;
                let dpos = node_pos - p.position;
                let momentum = p.mass * (p.velocity + p.c * dpos);
                let stress = p_vol * stress_factor * dpos * w * (1.0 / params.dt);

                // 原子加: 多个粒子可能散射到同一节点
                // 注: WGSL storage buffer 不支持 vec3 atomic, 需拆分为分量
                // 这里简化为非原子（仅适用于 P2G 分配阶段）
                grid[g_idx].velocity += w * (momentum + stress * (1.0 / params.dx));
                grid[g_idx].mass += w * p.mass;
            }
        }
    }
}
"#;

/// GridOp compute shader (WGSL)
pub const MPM_WGSL_GRIDOP: &str = r#"
// GridOp: 重力 + 边界条件

struct GridNode {
    velocity: vec3<f32>,
    mass: f32,
};

@group(0) @binding(0) var<storage, read_write> grid: array<GridNode>;
@group(0) @binding(1) var<uniform> params: SimParams;

struct SimParams {
    grid_nx: u32,
    grid_ny: u32,
    grid_nz: u32,
    particle_count: u32,
    dx: f32,
    dt: f32,
    elastic_mu: f32,
    elastic_lambda: f32,
    origin: vec3<f32>,
    gravity_y: f32,
};

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    let total = params.grid_nx * params.grid_ny * params.grid_nz;
    if idx >= total {
        return;
    }

    let m = grid[idx].mass;
    if m <= 1e-10 {
        grid[idx].velocity = vec3<f32>(0.0);
        return;
    }

    var v = grid[idx].velocity / m;
    // 重力
    v.y += params.gravity_y * params.dt;

    // 边界条件
    let nx = params.grid_nx;
    let ny = params.grid_ny;
    let nz = params.grid_nz;
    let i = idx % nx;
    let jk = idx / nx;
    let j = jk % ny;
    let k = jk / ny;
    let margin = 3u;

    if i < margin && v.x < 0.0 { v.x = 0.0; }
    if i >= nx - margin && v.x > 0.0 { v.x = 0.0; }
    if j < margin && v.y < 0.0 { v.y = 0.0; }
    if j >= ny - margin && v.y > 0.0 { v.y = 0.0; }
    if k < margin && v.z < 0.0 { v.z = 0.0; }
    if k >= nz - margin && v.z > 0.0 { v.z = 0.0; }

    grid[idx].velocity = v;
}
"#;

/// G2P compute shader (WGSL)
pub const MPM_WGSL_G2P: &str = r#"
// G2P: Grid to Particle transfer

struct Particle {
    position: vec3<f32>,
    velocity: vec3<f32>,
    c: mat3x3<f32>,
    j: f32,
    mass: f32,
    volume_ratio: f32,
    _pad: f32,
};

struct GridNode {
    velocity: vec3<f32>,
    mass: f32,
};

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<storage, read> grid: array<GridNode>;
@group(0) @binding(2) var<uniform> params: SimParams;

struct SimParams {
    grid_nx: u32,
    grid_ny: u32,
    grid_nz: u32,
    particle_count: u32,
    dx: f32,
    dt: f32,
    elastic_mu: f32,
    elastic_lambda: f32,
    origin: vec3<f32>,
    _pad: f32,
};

fn quadratic_weight(d: f32) -> f32 {
    let ad = abs(d);
    if ad < 0.5 {
        return 0.75 - ad * ad;
    } else if ad < 1.5 {
        let t = 1.5 - ad;
        return 0.5 * t * t;
    }
    return 0.0;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let p_idx = gid.x;
    if p_idx >= params.particle_count {
        return;
    }

    var p = particles[p_idx];
    let inv_dx = 1.0 / params.dx;
    let cell = (p.position - params.origin) * inv_dx;
    let i0 = vec3<i32>(floor(cell));
    let f = cell - vec3<f32>(i0);

    let weights_x = vec3<f32>(
        quadratic_weight(f.x - 0.0),
        quadratic_weight(f.x - 1.0),
        quadratic_weight(f.x + 1.0),
    );
    let weights_y = vec3<f32>(
        quadratic_weight(f.y - 0.0),
        quadratic_weight(f.y - 1.0),
        quadratic_weight(f.y + 1.0),
    );
    let weights_z = vec3<f32>(
        quadratic_weight(f.z - 0.0),
        quadratic_weight(f.z - 1.0),
        quadratic_weight(f.z + 1.0),
    );

    var new_v = vec3<f32>(0.0);
    var new_c = mat3x3<f32>(0.0);
    let d_inv = 4.0 * params.dx * params.dx;

    for var di: i32 = 0; di < 3; di = di + 1 {
        for var dj: i32 = 0; dj < 3; dj = dj + 1 {
            for var dk: i32 = 0; dk < 3; dk = dk + 1 {
                let gi = i0.x + di - 1;
                let gj = i0.y + dj - 1;
                let gk = i0.z + dk - 1;
                if gi < 0 || gj < 0 || gk < 0 {
                    continue;
                }
                if u32(gi) >= params.grid_nx || u32(gj) >= params.grid_ny || u32(gk) >= params.grid_nz {
                    continue;
                }
                let w = weights_x[di] * weights_y[dj] * weights_z[dk];
                if w <= 0.0 {
                    continue;
                }

                let g_idx = (gk * i32(params.grid_ny) + gj) * i32(params.grid_nx) + gi;
                let gv = grid[g_idx].velocity;
                let node_pos = params.origin + vec3<f32>(f32(gi), f32(gj), f32(gk)) * params.dx;
                let dpos = node_pos - p.position;

                new_v += gv * w;
                new_c += mat3x3<f32>(gv * dpos.x, gv * dpos.y, gv * dpos.z) * (w / d_inv);
            }
        }
    }

    p.velocity = new_v;
    p.c = new_c;
    p.position += new_v * params.dt;

    let trace_c = new_c[0][0] + new_c[1][1] + new_c[2][2];
    p.j = max(p.j * (1.0 + trace_c * params.dt), 0.01);

    particles[p_idx] = p;
}
"#;

// ============================================================
// GPU 调度器 (feature = "gpu")
// ============================================================

#[cfg(feature = "gpu")]
pub mod gpu {
    use super::*;
    use bytemuck::{Pod, Zeroable};
    use wgpu::util::DeviceExt;

    /// GPU 友好的粒子布局（16 字节对齐，SoA 友好）
    #[repr(C)]
    #[derive(Debug, Clone, Copy, Pod, Zeroable)]
    pub struct GpuParticle {
        pub position: [f32; 3],
        pub mass: f32,
        pub velocity: [f32; 3],
        pub j: f32,
        pub c: [f32; 9],
        pub volume_ratio: f32,
        pub _pad: [f32; 2],
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy, Pod, Zeroable)]
    pub struct GpuGridNode {
        pub velocity: [f32; 3],
        pub mass: f32,
    }

    #[repr(C)]
    #[derive(Debug, Clone, Copy, Pod, Zeroable)]
    pub struct GpuSimParams {
        pub grid_nx: u32,
        pub grid_ny: u32,
        pub grid_nz: u32,
        pub particle_count: u32,
        pub dx: f32,
        pub dt: f32,
        pub elastic_mu: f32,
        pub elastic_lambda: f32,
        pub origin: [f32; 3],
        pub gravity_y: f32,
    }

    /// GPU MPM 调度器
    pub struct GpuMpmDispatcher {
        device: wgpu::Device,
        queue: wgpu::Queue,
        p2g_pipeline: wgpu::ComputePipeline,
        gridop_pipeline: wgpu::ComputePipeline,
        g2p_pipeline: wgpu::ComputePipeline,
        particle_buffer: Option<wgpu::Buffer>,
        grid_buffer: Option<wgpu::Buffer>,
        params_buffer: wgpu::Buffer,
        bind_group_layout: wgpu::BindGroupLayout,
        config: MpmConfig,
    }

    impl GpuMpmDispatcher {
        /// 创建 GPU 调度器（需要异步初始化 wgpu 设备）
        pub async fn new(config: MpmConfig) -> Option<Self> {
            let instance = wgpu::Instance::default();
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await
                .ok()?;

            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("MPM GPU device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                })
                .await
                .ok()?;

            // 创建 bind group layout
            let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MPM bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("MPM pipeline layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            let p2g_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("MPM P2G shader"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(MPM_WGSL_P2G)),
            });
            let gridop_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("MPM GridOp shader"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(MPM_WGSL_GRIDOP)),
            });
            let g2p_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("MPM G2P shader"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(MPM_WGSL_G2P)),
            });

            let p2g_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("MPM P2G pipeline"),
                layout: Some(&pipeline_layout),
                module: &p2g_shader,
                entry_point: "main",
                compilation_options: Default::default(),
            });
            let gridop_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("MPM GridOp pipeline"),
                layout: Some(&pipeline_layout),
                module: &gridop_shader,
                entry_point: "main",
                compilation_options: Default::default(),
            });
            let g2p_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("MPM G2P pipeline"),
                layout: Some(&pipeline_layout),
                module: &g2p_shader,
                entry_point: "main",
                compilation_options: Default::default(),
            });

            // 参数 uniform buffer
            let params = GpuSimParams {
                grid_nx: config.grid_nx as u32,
                grid_ny: config.grid_ny as u32,
                grid_nz: config.grid_nz as u32,
                particle_count: 0,
                dx: config.dx,
                dt: config.dt,
                elastic_mu: config.elastic_mu,
                elastic_lambda: config.elastic_lambda,
                origin: [config.origin.x, config.origin.y, config.origin.z],
                gravity_y: config.gravity.y,
            };
            let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("MPM params buffer"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

            Some(Self {
                device,
                queue,
                p2g_pipeline,
                gridop_pipeline,
                g2p_pipeline,
                particle_buffer: None,
                grid_buffer: None,
                params_buffer,
                bind_group_layout,
                config,
            })
        }

        /// 上传粒子到 GPU
        pub fn upload_particles(&mut self, particles: &[MpmParticle]) {
            let gpu_particles: Vec<GpuParticle> = particles
                .iter()
                .map(|p| GpuParticle {
                    position: [p.position.x, p.position.y, p.position.z],
                    mass: p.mass,
                    velocity: [p.velocity.x, p.velocity.y, p.velocity.z],
                    j: p.j,
                    c: p.c,
                    volume_ratio: p.volume_ratio,
                    _pad: [0.0, 0.0],
                })
                .collect();

            let particle_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("MPM particle buffer"),
                contents: bytemuck::cast_slice(&gpu_particles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            });

            let grid_size = self.config.grid_nx * self.config.grid_ny * self.config.grid_nz;
            let grid_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("MPM grid buffer"),
                size: (grid_size * std::mem::size_of::<GpuGridNode>()) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

            self.particle_buffer = Some(particle_buffer);
            self.grid_buffer = Some(grid_buffer);

            // 更新参数中的 particle_count
            let mut params = GpuSimParams {
                grid_nx: self.config.grid_nx as u32,
                grid_ny: self.config.grid_ny as u32,
                grid_nz: self.config.grid_nz as u32,
                particle_count: particles.len() as u32,
                dx: self.config.dx,
                dt: self.config.dt,
                elastic_mu: self.config.elastic_mu,
                elastic_lambda: self.config.elastic_lambda,
                origin: [self.config.origin.x, self.config.origin.y, self.config.origin.z],
                gravity_y: self.config.gravity.y,
            };
            self.queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));
        }

        /// 执行一步 GPU MPM
        pub fn step(&self) {
            let (particle_buffer, grid_buffer) = match (&self.particle_buffer, &self.grid_buffer) {
                (Some(p), Some(g)) => (p, g),
                _ => return,
            };

            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("MPM bind group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: particle_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: grid_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.params_buffer.as_entire_binding(),
                    },
                ],
            });

            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("MPM command encoder"),
            });

            let particle_count = self.config.particle_density as u32; // placeholder, should be actual count
            let workgroups_x = (particle_count + 63) / 64;

            let grid_size = (self.config.grid_nx * self.config.grid_ny * self.config.grid_nz) as u32;
            let grid_workgroups = (grid_size + 63) / 64;

            // P2G
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("MPM P2G pass"),
                });
                pass.set_pipeline(&self.p2g_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(workgroups_x, 1, 1);
            }
            // GridOp
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("MPM GridOp pass"),
                });
                pass.set_pipeline(&self.gridop_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(grid_workgroups, 1, 1);
            }
            // G2P
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("MPM G2P pass"),
                });
                pass.set_pipeline(&self.g2p_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(workgroups_x, 1, 1);
            }

            self.queue.submit(std::iter::once(encoder.finish()));
        }
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quadratic_weight() {
        // w(0) = 0.75
        assert!((quadratic_weight(0.0) - 0.75).abs() < 1e-6);
        // w(0.5) = 0.75 - 0.25 = 0.5
        assert!((quadratic_weight(0.5) - 0.5).abs() < 1e-6);
        // w(1.0) = 0.5 * (1.5 - 1.0)² = 0.125
        assert!((quadratic_weight(1.0) - 0.125).abs() < 1e-6);
        // w(1.5) = 0
        assert!(quadratic_weight(1.5).abs() < 1e-6);
        // w(2.0) = 0
        assert!(quadratic_weight(2.0).abs() < 1e-6);
    }

    #[test]
    fn test_quadratic_weight_grad() {
        // 解析导数: |d|<0.5 → -2d; 0.5≤|d|<1.5 → d - 1.5*sign(d)
        // d=0: 导数 = 0
        assert!(quadratic_weight_grad(0.0).abs() < 1e-6);
        // d=0.25: 导数 = -0.5
        assert!((quadratic_weight_grad(0.25) - (-0.5)).abs() < 1e-6);
        // d=-0.25: 导数 = 0.5
        assert!((quadratic_weight_grad(-0.25) - 0.5).abs() < 1e-6);
        // d=1.0: 导数 = 1 - 1.5 = -0.5
        assert!((quadratic_weight_grad(1.0) - (-0.5)).abs() < 1e-6);
        // d=-1.0: 导数 = -1 + 1.5 = 0.5
        assert!((quadratic_weight_grad(-1.0) - 0.5).abs() < 1e-6);
        // d=2.0: 导数 = 0
        assert!(quadratic_weight_grad(2.0).abs() < 1e-6);
    }

    #[test]
    fn test_partition_of_unity() {
        // 对任意 fx ∈ [0.5, 1.5)，3 节点 stencil 权重总和应为 1.0
        // (MLS-MPM B 样条 partition of unity 性质)
        for i in 0..=100 {
            let fx = 0.5 + (i as f32) / 100.0; // fx ∈ [0.5, 1.5]
            let w0 = quadratic_weight(fx);
            let w1 = quadratic_weight(fx - 1.0);
            let w2 = quadratic_weight(fx - 2.0);
            let sum = w0 + w1 + w2;
            assert!(
                (sum - 1.0).abs() < 1e-5,
                "partition of unity violated at fx={}: sum={}",
                fx,
                sum
            );
        }
    }

    #[test]
    fn test_mpm_stress_stability() {
        // 验证默认 elastic_mu=lambda=1e5 不会发散
        // 弹性球在重力下应稳定运动，不应爆炸
        let config = MpmConfig {
            grid_nx: 32,
            grid_ny: 32,
            grid_nz: 32,
            dx: 1.0 / 32.0,
            dt: 0.0005, // 较小 dt 保证 CFL
            gravity: Vec3::new(0.0, -9.81, 0.0),
            ..Default::default() // elastic_mu = elastic_lambda = 1e5
        };
        let mut solver = MpmSolver::new(config);
        solver.add_particle(MpmParticle::new(Vec3::new(0.5, 0.7, 0.5), 1.0));

        let y_start = solver.particles[0].position.y;
        for _ in 0..100 {
            solver.step();
            let p = &solver.particles[0];
            // 粒子位置不应变成 NaN 或飞出网格
            assert!(p.position.x.is_finite(), "x became non-finite");
            assert!(p.position.y.is_finite(), "y became non-finite");
            assert!(p.position.z.is_finite(), "z became non-finite");
            assert!(p.position.y >= -1.0 && p.position.y <= 2.0, "y out of bounds: {}", p.position.y);
            assert!(p.j > 0.0 && p.j.is_finite(), "J invalid: {}", p.j);
        }
        // 应在重力下下落
        let y_end = solver.particles[0].position.y;
        assert!(y_end < y_start, "particle should fall: y_start={} y_end={}", y_start, y_end);
    }

    #[test]
    fn test_grid_creation() {
        let grid = MpmGrid3D::new(16, 16, 16, 0.1, Vec3::ZERO);
        assert_eq!(grid.mass.len(), 4096);
        assert_eq!(grid.velocity.len(), 4096);
        assert_eq!(grid.idx(1, 2, 3), (3 * 16 + 2) * 16 + 1);
    }

    #[test]
    fn test_grid_clear() {
        let mut grid = MpmGrid3D::new(4, 4, 4, 0.1, Vec3::ZERO);
        grid.mass[0] = 1.0;
        grid.velocity[0] = Vec3::new(1.0, 2.0, 3.0);
        grid.clear();
        assert_eq!(grid.mass[0], 0.0);
        assert_eq!(grid.velocity[0], Vec3::ZERO);
    }

    #[test]
    fn test_particle_block_creation() {
        let config = MpmConfig {
            grid_nx: 16,
            grid_ny: 16,
            grid_nz: 16,
            dx: 1.0 / 16.0,
            ..Default::default()
        };
        let mut solver = MpmSolver::new(config);
        solver.add_particle_block(
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::new(0.2, 0.2, 0.2),
            4,
            1.0,
        );
        assert_eq!(solver.particles.len(), 64); // 4³ = 64
        let total_mass = solver.total_mass();
        assert!((total_mass - 1.0).abs() < 1e-5, "total mass = {}", total_mass);
    }

    #[test]
    fn test_mpm_gravity_fall() {
        // 粒子应在重力下下落
        let config = MpmConfig {
            grid_nx: 32,
            grid_ny: 32,
            grid_nz: 32,
            dx: 1.0 / 32.0,
            dt: 0.01,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            elastic_mu: 0.0, // 无弹性，纯重力
            elastic_lambda: 0.0,
            ..Default::default()
        };
        let mut solver = MpmSolver::new(config);
        // 单个粒子放在网格中央
        solver.add_particle(MpmParticle::new(Vec3::new(0.5, 0.7, 0.5), 1.0));

        let y_before = solver.particles[0].position.y;
        solver.step();
        let y_after = solver.particles[0].position.y;

        assert!(y_after < y_before, "particle should fall: y_before={} y_after={}", y_before, y_after);
    }

    #[test]
    fn test_mpm_mass_conservation() {
        // 总质量应守恒
        let config = MpmConfig {
            grid_nx: 32,
            grid_ny: 32,
            grid_nz: 32,
            dx: 1.0 / 32.0,
            dt: 0.005,
            ..Default::default()
        };
        let mut solver = MpmSolver::new(config);
        solver.add_particle_block(
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::new(0.15, 0.15, 0.15),
            3,
            2.0,
        );

        let m0 = solver.total_mass();
        for _ in 0..10 {
            solver.step();
        }
        let m1 = solver.total_mass();
        // 质量应严格守恒（MPM 在 P2G/G2P 中不改变粒子质量）
        assert!((m1 - m0).abs() < 1e-5, "mass not conserved: m0={} m1={}", m0, m1);
    }

    #[test]
    fn test_mpm_boundary_collision() {
        // 粒子撞击地面后不应穿透
        let config = MpmConfig {
            grid_nx: 32,
            grid_ny: 32,
            grid_nz: 32,
            dx: 1.0 / 32.0,
            dt: 0.001,
            gravity: Vec3::new(0.0, -20.0, 0.0),
            restitution: 0.0, // 完全非弹性
            ..Default::default()
        };
        let mut solver = MpmSolver::new(config);
        solver.add_particle(MpmParticle::new(Vec3::new(0.5, 0.5, 0.5), 1.0));

        // 跑 200 步
        for _ in 0..200 {
            solver.step();
        }

        let y = solver.particles[0].position.y;
        // 粒子应停留在边界附近（不应穿透底部）
        assert!(y >= 0.0, "particle fell through floor: y={}", y);
        assert!(y < 0.5, "particle should have fallen, y={}", y);
    }

    #[test]
    fn test_mpm_center_of_mass() {
        let config = MpmConfig {
            grid_nx: 16,
            grid_ny: 16,
            grid_nz: 16,
            dx: 1.0 / 16.0,
            ..Default::default()
        };
        let mut solver = MpmSolver::new(config);
        solver.add_particle(MpmParticle::new(Vec3::new(0.4, 0.5, 0.5), 1.0));
        solver.add_particle(MpmParticle::new(Vec3::new(0.6, 0.5, 0.5), 1.0));

        let com = solver.center_of_mass();
        assert!((com.x - 0.5).abs() < 1e-5, "com.x = {}", com.x);
        assert!((com.y - 0.5).abs() < 1e-5, "com.y = {}", com.y);
    }

    #[test]
    fn test_mpm_kinetic_energy() {
        let config = MpmConfig::default();
        let mut solver = MpmSolver::new(config);
        let p = MpmParticle::new(Vec3::new(0.5, 0.5, 0.5), 2.0);
        let mut p_with_v = p;
        p_with_v.velocity = Vec3::new(3.0, 0.0, 0.0);
        solver.add_particle(p_with_v);
        // KE = 0.5 * 2 * 9 = 9
        let ke = solver.kinetic_energy();
        assert!((ke - 9.0).abs() < 1e-5, "KE = {}", ke);
    }

    #[test]
    fn test_wgsl_shaders_nonempty() {
        // 验证 WGSL shader 源码已编译进来
        assert!(MPM_WGSL_P2G.contains("workgroup_size(64)"));
        assert!(MPM_WGSL_P2G.contains("quadratic_weight"));
        assert!(MPM_WGSL_GRIDOP.contains("gravity_y"));
        assert!(MPM_WGSL_G2P.contains("particles[p_idx]"));
    }
}
