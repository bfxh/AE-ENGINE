//! Stam Stable Fluids 3D 求解器
//!
//! 论文来源：
//! - Stam, J. 1999. "Stable Fluids." SIGGRAPH.
//! - Fedkiw, R., Stam, J., Jensen, H. W. 2001. "Visual Simulation of Smoke." ACM TOG.
//! - Stam, J. 2003. "Real-Time Fluid Dynamics for Games." GDC.
//!
//! 算法核心：
//! 1. add_source - 添加源（速度/密度/温度）
//! 2. diffuse - 隐式扩散（Gauss-Seidel 迭代）
//! 3. advect - 半拉格朗日对流（backtrace + 三线性插值）
//! 4. project - 压力投影（Hodge 分解，使速度场散度为0）
//! 5. vorticity_confinement - 涡量约束（Fedkiw 2001，补充小尺度细节）
//! 6. buoyancy - 浮力（温度差驱动上升，火焰/烟雾核心）

use serde::{Deserialize, Serialize};

/// 3D 网格索引：i,j,k ∈ [0, n+1]，内部 [1, n]
#[inline]
fn ix(i: usize, j: usize, k: usize, n: usize) -> usize {
    let n2 = n + 2;
    i + n2 * (j + n2 * k)
}

/// 标量场边界类型
/// 0 = 自由边界（密度/温度）
/// 1 = x方向反射（速度u分量在边界翻转）
/// 2 = y方向反射（速度v分量）
/// 3 = z方向反射（速度w分量）
fn set_bnd_scalar(b: i32, x: &mut [f32], n: usize) {
    // 6 个面
    for i in 1..=n {
        for j in 1..=n {
            x[ix(i, j, 0, n)] = if b == 3 { -x[ix(i, j, 1, n)] } else { x[ix(i, j, 1, n)] };
            x[ix(i, j, n + 1, n)] = if b == 3 { -x[ix(i, j, n, n)] } else { x[ix(i, j, n, n)] };
            x[ix(i, 0, j, n)] = if b == 2 { -x[ix(i, 1, j, n)] } else { x[ix(i, 1, j, n)] };
            x[ix(i, n + 1, j, n)] = if b == 2 { -x[ix(i, n, j, n)] } else { x[ix(i, n, j, n)] };
            x[ix(0, i, j, n)] = if b == 1 { -x[ix(1, i, j, n)] } else { x[ix(1, i, j, n)] };
            x[ix(n + 1, i, j, n)] = if b == 1 { -x[ix(n, i, j, n)] } else { x[ix(n, i, j, n)] };
        }
    }
    // 12 条边（两邻居平均）
    for i in 1..=n {
        x[ix(i, 0, 0, n)] = 0.5 * (x[ix(i, 1, 0, n)] + x[ix(i, 0, 1, n)]);
        x[ix(i, n + 1, 0, n)] = 0.5 * (x[ix(i, n, 0, n)] + x[ix(i, n + 1, 1, n)]);
        x[ix(i, 0, n + 1, n)] = 0.5 * (x[ix(i, 1, n + 1, n)] + x[ix(i, 0, n, n)]);
        x[ix(i, n + 1, n + 1, n)] = 0.5 * (x[ix(i, n, n + 1, n)] + x[ix(i, n + 1, n, n)]);
        x[ix(0, i, 0, n)] = 0.5 * (x[ix(1, i, 0, n)] + x[ix(0, i, 1, n)]);
        x[ix(n + 1, i, 0, n)] = 0.5 * (x[ix(n, i, 0, n)] + x[ix(n + 1, i, 1, n)]);
        x[ix(0, i, n + 1, n)] = 0.5 * (x[ix(1, i, n + 1, n)] + x[ix(0, i, n, n)]);
        x[ix(n + 1, i, n + 1, n)] = 0.5 * (x[ix(n, i, n + 1, n)] + x[ix(n + 1, i, n, n)]);
        x[ix(0, 0, i, n)] = 0.5 * (x[ix(1, 0, i, n)] + x[ix(0, 1, i, n)]);
        x[ix(0, n + 1, i, n)] = 0.5 * (x[ix(1, n + 1, i, n)] + x[ix(0, n, i, n)]);
        x[ix(n + 1, 0, i, n)] = 0.5 * (x[ix(n, 0, i, n)] + x[ix(n + 1, 1, i, n)]);
        x[ix(n + 1, n + 1, i, n)] = 0.5 * (x[ix(n, n + 1, i, n)] + x[ix(n + 1, n, i, n)]);
    }
    // 8 个角点（三邻居平均）
    x[ix(0, 0, 0, n)] = 0.33 * (x[ix(1, 0, 0, n)] + x[ix(0, 1, 0, n)] + x[ix(0, 0, 1, n)]);
    x[ix(0, n + 1, 0, n)] =
        0.33 * (x[ix(1, n + 1, 0, n)] + x[ix(0, n, 0, n)] + x[ix(0, n + 1, 1, n)]);
    x[ix(0, 0, n + 1, n)] =
        0.33 * (x[ix(1, 0, n + 1, n)] + x[ix(0, 1, n + 1, n)] + x[ix(0, 0, n, n)]);
    x[ix(0, n + 1, n + 1, n)] =
        0.33 * (x[ix(1, n + 1, n + 1, n)] + x[ix(0, n, n + 1, n)] + x[ix(0, n + 1, n, n)]);
    x[ix(n + 1, 0, 0, n)] =
        0.33 * (x[ix(n, 0, 0, n)] + x[ix(n + 1, 1, 0, n)] + x[ix(n + 1, 0, 1, n)]);
    x[ix(n + 1, n + 1, 0, n)] =
        0.33 * (x[ix(n, n + 1, 0, n)] + x[ix(n + 1, n, 0, n)] + x[ix(n + 1, n + 1, 1, n)]);
    x[ix(n + 1, 0, n + 1, n)] =
        0.33 * (x[ix(n, 0, n + 1, n)] + x[ix(n + 1, 1, n + 1, n)] + x[ix(n + 1, 0, n, n)]);
    x[ix(n + 1, n + 1, n + 1, n)] =
        0.33 * (x[ix(n, n + 1, n + 1, n)] + x[ix(n + 1, n, n + 1, n)] + x[ix(n + 1, n + 1, n, n)]);
}

/// Stam Stable Fluids 3D 求解器
///
/// 网格组织：i,j,k ∈ [0, n+1]，内部 [1, n]
/// 实际数据长度 (n+2)^3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StamFluidSolver3D {
    pub n: usize,
    /// 速度场三个分量
    pub u: Vec<f32>,
    pub v: Vec<f32>,
    pub w: Vec<f32>,
    pub u_prev: Vec<f32>,
    pub v_prev: Vec<f32>,
    pub w_prev: Vec<f32>,
    /// 密度场（烟雾/燃料蒸汽）
    pub density: Vec<f32>,
    pub density_prev: Vec<f32>,
    /// 温度场（K，用于浮力计算）
    pub temperature: Vec<f32>,
    pub temperature_prev: Vec<f32>,
    /// 物理参数
    pub visc: f32, // 动力学粘度 (m^2/s)
    pub diff: f32,           // 扩散系数 (m^2/s)
    pub buoyancy_alpha: f32, // 浮力系数 α
    pub ambient_temp: f32,   // 环境温度 T_amb (K)
    pub vorticity_eps: f32,  // 涡量约束强度 ε
    /// 内部缓冲（压力投影用）
    pub pressure: Vec<f32>,
    pub divergence: Vec<f32>,
}

impl StamFluidSolver3D {
    /// 创建求解器
    /// n: 内部分辨率（实际网格 (n+2)^3）
    /// visc: 粘度，烟雾典型 1e-6 ~ 1e-4
    /// diff: 扩散，烟雾典型 1e-6 ~ 1e-4
    pub fn new(n: usize, visc: f32, diff: f32) -> Self {
        let size = (n + 2).pow(3);
        Self {
            n,
            u: vec![0.0; size],
            v: vec![0.0; size],
            w: vec![0.0; size],
            u_prev: vec![0.0; size],
            v_prev: vec![0.0; size],
            w_prev: vec![0.0; size],
            density: vec![0.0; size],
            density_prev: vec![0.0; size],
            temperature: vec![300.0; size],
            temperature_prev: vec![300.0; size],
            visc,
            diff,
            buoyancy_alpha: 1.0,
            ambient_temp: 300.0,
            vorticity_eps: 0.5,
            pressure: vec![0.0; size],
            divergence: vec![0.0; size],
        }
    }

    /// 单步时间步进
    pub fn step(&mut self, dt: f32) {
        let n = self.n;
        // 1. 速度扩散（隐式 Gauss-Seidel 20 次迭代）
        self.diffuse_velocity(dt, 20);
        // 2. 压力投影（使速度场无散）
        self.project(20);
        // 3. 速度对流（半拉格朗日）
        self.advect_velocity(dt);
        // 4. 再次投影
        self.project(20);
        // 5. 涡量约束（Fedkiw 2001，补充小尺度细节）
        if self.vorticity_eps > 0.0 {
            self.vorticity_confinement(dt);
            self.project(20);
        }
        // 6. 浮力（温度差驱动上升）
        self.buoyancy(dt);
        self.project(20);
        // 7. 密度扩散+对流
        let n = self.n;
        Self::diffuse_scalar_field(
            n,
            0,
            &mut self.density,
            &mut self.density_prev,
            self.diff,
            dt,
            20,
        );
        Self::advect_scalar_field(
            n,
            0,
            &mut self.density,
            &mut self.density_prev,
            dt,
            &self.u,
            &self.v,
            &self.w,
        );
        // 8. 温度扩散+对流
        Self::diffuse_scalar_field(
            n,
            0,
            &mut self.temperature,
            &mut self.temperature_prev,
            self.diff,
            dt,
            20,
        );
        Self::advect_scalar_field(
            n,
            0,
            &mut self.temperature,
            &mut self.temperature_prev,
            dt,
            &self.u,
            &self.v,
            &self.w,
        );
        // 9. 清零源
        self.clear_sources();
    }

    /// 添加密度源（烟雾/燃料）
    pub fn add_density_source(&mut self, i: usize, j: usize, k: usize, value: f32) {
        let idx = ix(i, j, k, self.n);
        if idx < self.density_prev.len() {
            self.density_prev[idx] += value;
        }
    }

    /// 添加温度源（火源）
    pub fn add_temperature_source(&mut self, i: usize, j: usize, k: usize, value: f32) {
        let idx = ix(i, j, k, self.n);
        if idx < self.temperature_prev.len() {
            self.temperature_prev[idx] += value;
        }
    }

    /// 添加速度源
    pub fn add_velocity_source(&mut self, i: usize, j: usize, k: usize, du: f32, dv: f32, dw: f32) {
        let idx = ix(i, j, k, self.n);
        if idx < self.u_prev.len() {
            self.u_prev[idx] += du;
            self.v_prev[idx] += dv;
            self.w_prev[idx] += dw;
        }
    }

    /// 清零源缓冲
    fn clear_sources(&mut self) {
        for x in &mut self.u_prev {
            *x = 0.0;
        }
        for x in &mut self.v_prev {
            *x = 0.0;
        }
        for x in &mut self.w_prev {
            *x = 0.0;
        }
        for x in &mut self.density_prev {
            *x = 0.0;
        }
        for x in &mut self.temperature_prev {
            *x = 0.0;
        }
    }

    /// 隐式扩散（Gauss-Seidel 迭代）
    /// 求解 x - dt*diff*∇^2 x = x0
    fn diffuse_scalar_field(
        n: usize,
        b: i32,
        x: &mut [f32],
        x0: &mut [f32],
        diff: f32,
        dt: f32,
        iters: usize,
    ) {
        let a = dt * diff * (n * n) as f32;
        x.swap_with_slice(x0);
        for _ in 0..iters {
            for k in 1..=n {
                for j in 1..=n {
                    for i in 1..=n {
                        x[ix(i, j, k, n)] = (x0[ix(i, j, k, n)]
                            + a * (x[ix(i - 1, j, k, n)]
                                + x[ix(i + 1, j, k, n)]
                                + x[ix(i, j - 1, k, n)]
                                + x[ix(i, j + 1, k, n)]
                                + x[ix(i, j, k - 1, n)]
                                + x[ix(i, j, k + 1, n)]))
                            / (1.0 + 6.0 * a);
                    }
                }
            }
            set_bnd_scalar(b, x, n);
        }
    }

    /// 速度场扩散（内部调度三个分量）
    fn diffuse_velocity(&mut self, dt: f32, iters: usize) {
        let diff = self.visc;
        // u 分量，边界类型 1
        std::mem::swap(&mut self.u, &mut self.u_prev);
        let (n, a) = (self.n, dt * diff * (self.n * self.n) as f32);
        for _ in 0..iters {
            for k in 1..=n {
                for j in 1..=n {
                    for i in 1..=n {
                        self.u[ix(i, j, k, n)] = (self.u_prev[ix(i, j, k, n)]
                            + a * (self.u[ix(i - 1, j, k, n)]
                                + self.u[ix(i + 1, j, k, n)]
                                + self.u[ix(i, j - 1, k, n)]
                                + self.u[ix(i, j + 1, k, n)]
                                + self.u[ix(i, j, k - 1, n)]
                                + self.u[ix(i, j, k + 1, n)]))
                            / (1.0 + 6.0 * a);
                    }
                }
            }
            set_bnd_scalar(1, &mut self.u, n);
        }
        // v 分量，边界类型 2
        std::mem::swap(&mut self.v, &mut self.v_prev);
        for _ in 0..iters {
            for k in 1..=n {
                for j in 1..=n {
                    for i in 1..=n {
                        self.v[ix(i, j, k, n)] = (self.v_prev[ix(i, j, k, n)]
                            + a * (self.v[ix(i - 1, j, k, n)]
                                + self.v[ix(i + 1, j, k, n)]
                                + self.v[ix(i, j - 1, k, n)]
                                + self.v[ix(i, j + 1, k, n)]
                                + self.v[ix(i, j, k - 1, n)]
                                + self.v[ix(i, j, k + 1, n)]))
                            / (1.0 + 6.0 * a);
                    }
                }
            }
            set_bnd_scalar(2, &mut self.v, n);
        }
        // w 分量，边界类型 3
        std::mem::swap(&mut self.w, &mut self.w_prev);
        for _ in 0..iters {
            for k in 1..=n {
                for j in 1..=n {
                    for i in 1..=n {
                        self.w[ix(i, j, k, n)] = (self.w_prev[ix(i, j, k, n)]
                            + a * (self.w[ix(i - 1, j, k, n)]
                                + self.w[ix(i + 1, j, k, n)]
                                + self.w[ix(i, j - 1, k, n)]
                                + self.w[ix(i, j + 1, k, n)]
                                + self.w[ix(i, j, k - 1, n)]
                                + self.w[ix(i, j, k + 1, n)]))
                            / (1.0 + 6.0 * a);
                    }
                }
            }
            set_bnd_scalar(3, &mut self.w, n);
        }
    }

    /// 半拉格朗日对流（标量场）
    fn advect_scalar_field(
        n: usize,
        b: i32,
        d: &mut [f32],
        d0: &mut [f32],
        dt: f32,
        u: &[f32],
        v: &[f32],
        w: &[f32],
    ) {
        let dt0 = dt * n as f32;
        d.swap_with_slice(d0);
        for k in 1..=n {
            for j in 1..=n {
                for i in 1..=n {
                    // backtrace: x_prev = x - dt * v(x)
                    let mut x = i as f32 - dt0 * u[ix(i, j, k, n)];
                    let mut y = j as f32 - dt0 * v[ix(i, j, k, n)];
                    let mut z = k as f32 - dt0 * w[ix(i, j, k, n)];
                    // 钳制到边界
                    if x < 0.5 {
                        x = 0.5;
                    }
                    if x > n as f32 + 0.5 {
                        x = n as f32 + 0.5;
                    }
                    if y < 0.5 {
                        y = 0.5;
                    }
                    if y > n as f32 + 0.5 {
                        y = n as f32 + 0.5;
                    }
                    if z < 0.5 {
                        z = 0.5;
                    }
                    if z > n as f32 + 0.5 {
                        z = n as f32 + 0.5;
                    }
                    // 三线性插值
                    let i0 = x.floor() as usize;
                    let i1 = i0 + 1;
                    let j0 = y.floor() as usize;
                    let j1 = j0 + 1;
                    let k0 = z.floor() as usize;
                    let k1 = k0 + 1;
                    let s1 = x - i0 as f32;
                    let s0 = 1.0 - s1;
                    let t1 = y - j0 as f32;
                    let t0 = 1.0 - t1;
                    let u1 = z - k0 as f32;
                    let u0 = 1.0 - u1;
                    d[ix(i, j, k, n)] = s0
                        * (t0 * (u0 * d0[ix(i0, j0, k0, n)] + u1 * d0[ix(i0, j0, k1, n)])
                            + t1 * (u0 * d0[ix(i0, j1, k0, n)] + u1 * d0[ix(i0, j1, k1, n)]))
                        + s1 * (t0 * (u0 * d0[ix(i1, j0, k0, n)] + u1 * d0[ix(i1, j0, k1, n)])
                            + t1 * (u0 * d0[ix(i1, j1, k0, n)] + u1 * d0[ix(i1, j1, k1, n)]));
                }
            }
        }
        set_bnd_scalar(b, d, n);
    }

    /// 速度场对流（三分量分别 advect）
    fn advect_velocity(&mut self, dt: f32) {
        let n = self.n;
        // u 分量
        std::mem::swap(&mut self.u, &mut self.u_prev);
        let (u_vel, v_vel, w_vel) = (self.u.clone(), self.v.clone(), self.w.clone());
        Self::advect_component(n, 1, &mut self.u, &mut self.u_prev, dt, &u_vel, &v_vel, &w_vel);
        // v 分量
        std::mem::swap(&mut self.v, &mut self.v_prev);
        let (u_vel, v_vel, w_vel) = (self.u.clone(), self.v.clone(), self.w.clone());
        Self::advect_component(n, 2, &mut self.v, &mut self.v_prev, dt, &u_vel, &v_vel, &w_vel);
        // w 分量
        std::mem::swap(&mut self.w, &mut self.w_prev);
        let (u_vel, v_vel, w_vel) = (self.u.clone(), self.v.clone(), self.w.clone());
        Self::advect_component(n, 3, &mut self.w, &mut self.w_prev, dt, &u_vel, &v_vel, &w_vel);
    }

    /// 单分量对流（内部辅助）
    fn advect_component(
        n: usize,
        b: i32,
        d: &mut [f32],
        d0: &mut [f32],
        dt: f32,
        u: &[f32],
        v: &[f32],
        w: &[f32],
    ) {
        let dt0 = dt * n as f32;
        for k in 1..=n {
            for j in 1..=n {
                for i in 1..=n {
                    let mut x = i as f32 - dt0 * u[ix(i, j, k, n)];
                    let mut y = j as f32 - dt0 * v[ix(i, j, k, n)];
                    let mut z = k as f32 - dt0 * w[ix(i, j, k, n)];
                    if x < 0.5 {
                        x = 0.5;
                    }
                    if x > n as f32 + 0.5 {
                        x = n as f32 + 0.5;
                    }
                    if y < 0.5 {
                        y = 0.5;
                    }
                    if y > n as f32 + 0.5 {
                        y = n as f32 + 0.5;
                    }
                    if z < 0.5 {
                        z = 0.5;
                    }
                    if z > n as f32 + 0.5 {
                        z = n as f32 + 0.5;
                    }
                    let i0 = x.floor() as usize;
                    let i1 = i0 + 1;
                    let j0 = y.floor() as usize;
                    let j1 = j0 + 1;
                    let k0 = z.floor() as usize;
                    let k1 = k0 + 1;
                    let s1 = x - i0 as f32;
                    let s0 = 1.0 - s1;
                    let t1 = y - j0 as f32;
                    let t0 = 1.0 - t1;
                    let u1 = z - k0 as f32;
                    let u0 = 1.0 - u1;
                    d[ix(i, j, k, n)] = s0
                        * (t0 * (u0 * d0[ix(i0, j0, k0, n)] + u1 * d0[ix(i0, j0, k1, n)])
                            + t1 * (u0 * d0[ix(i0, j1, k0, n)] + u1 * d0[ix(i0, j1, k1, n)]))
                        + s1 * (t0 * (u0 * d0[ix(i1, j0, k0, n)] + u1 * d0[ix(i1, j0, k1, n)])
                            + t1 * (u0 * d0[ix(i1, j1, k0, n)] + u1 * d0[ix(i1, j1, k1, n)]));
                }
            }
        }
        set_bnd_scalar(b, d, n);
    }

    /// 压力投影（Hodge 分解，使速度场无散度）
    /// 求解 ∇^2 p = ∇·u，然后 u = u - ∇p
    fn project(&mut self, iters: usize) {
        let n = self.n;
        let p = &mut self.pressure;
        let div = &mut self.divergence;
        // 计算散度
        for k in 1..=n {
            for j in 1..=n {
                for i in 1..=n {
                    div[ix(i, j, k, n)] = -0.5 / n as f32
                        * (self.u[ix(i + 1, j, k, n)] - self.u[ix(i - 1, j, k, n)]
                            + self.v[ix(i, j + 1, k, n)]
                            - self.v[ix(i, j - 1, k, n)]
                            + self.w[ix(i, j, k + 1, n)]
                            - self.w[ix(i, j, k - 1, n)]);
                    p[ix(i, j, k, n)] = 0.0;
                }
            }
        }
        set_bnd_scalar(0, div, n);
        set_bnd_scalar(0, p, n);
        // Gauss-Seidel 迭代求解 ∇^2 p = div
        for _ in 0..iters {
            for k in 1..=n {
                for j in 1..=n {
                    for i in 1..=n {
                        p[ix(i, j, k, n)] = (div[ix(i, j, k, n)]
                            + p[ix(i - 1, j, k, n)]
                            + p[ix(i + 1, j, k, n)]
                            + p[ix(i, j - 1, k, n)]
                            + p[ix(i, j + 1, k, n)]
                            + p[ix(i, j, k - 1, n)]
                            + p[ix(i, j, k + 1, n)])
                            / 6.0;
                    }
                }
            }
            set_bnd_scalar(0, p, n);
        }
        // 速度减去压力梯度
        for k in 1..=n {
            for j in 1..=n {
                for i in 1..=n {
                    self.u[ix(i, j, k, n)] -=
                        0.5 * n as f32 * (p[ix(i + 1, j, k, n)] - p[ix(i - 1, j, k, n)]);
                    self.v[ix(i, j, k, n)] -=
                        0.5 * n as f32 * (p[ix(i, j + 1, k, n)] - p[ix(i, j - 1, k, n)]);
                    self.w[ix(i, j, k, n)] -=
                        0.5 * n as f32 * (p[ix(i, j, k + 1, n)] - p[ix(i, j, k - 1, n)]);
                }
            }
        }
        set_bnd_scalar(1, &mut self.u, n);
        set_bnd_scalar(2, &mut self.v, n);
        set_bnd_scalar(3, &mut self.w, n);
    }

    /// 涡量约束（Fedkiw 2001，补充小尺度细节）
    /// f = ε * (N × ω) * h
    /// N = ∇|ω| / |∇|ω||，归一化的涡度梯度方向
    fn vorticity_confinement(&mut self, dt: f32) {
        let n = self.n;
        let size = (n + 2).pow(3);
        let mut omega_x = vec![0.0f32; size];
        let mut omega_y = vec![0.0f32; size];
        let mut omega_z = vec![0.0f32; size];
        let mut omega_mag = vec![0.0f32; size];
        // 1. 计算涡度 ω = ∇ × u
        for k in 1..=n {
            for j in 1..=n {
                for i in 1..=n {
                    let du_dy = 0.5 * (self.u[ix(i, j + 1, k, n)] - self.u[ix(i, j - 1, k, n)]);
                    let du_dz = 0.5 * (self.u[ix(i, j, k + 1, n)] - self.u[ix(i, j, k - 1, n)]);
                    let dv_dx = 0.5 * (self.v[ix(i + 1, j, k, n)] - self.v[ix(i - 1, j, k, n)]);
                    let dv_dz = 0.5 * (self.v[ix(i, j, k + 1, n)] - self.v[ix(i, j, k - 1, n)]);
                    let dw_dx = 0.5 * (self.w[ix(i + 1, j, k, n)] - self.w[ix(i - 1, j, k, n)]);
                    let dw_dy = 0.5 * (self.w[ix(i, j + 1, k, n)] - self.w[ix(i, j - 1, k, n)]);
                    let ox = dw_dy - dv_dz;
                    let oy = du_dz - dw_dx;
                    let oz = dv_dx - du_dy;
                    omega_x[ix(i, j, k, n)] = ox;
                    omega_y[ix(i, j, k, n)] = oy;
                    omega_z[ix(i, j, k, n)] = oz;
                    omega_mag[ix(i, j, k, n)] = (ox * ox + oy * oy + oz * oz).sqrt();
                }
            }
        }
        // 2. 计算涡度梯度方向 N = ∇|ω| / |∇|ω||，应用力 f = ε * (N × ω)
        let eps = self.vorticity_eps;
        let h = 1.0 / n as f32;
        for k in 2..n {
            for j in 2..n {
                for i in 2..n {
                    let idx = ix(i, j, k, n);
                    let ngrad_x =
                        0.5 * (omega_mag[ix(i + 1, j, k, n)] - omega_mag[ix(i - 1, j, k, n)]);
                    let ngrad_y =
                        0.5 * (omega_mag[ix(i, j + 1, k, n)] - omega_mag[ix(i, j - 1, k, n)]);
                    let ngrad_z =
                        0.5 * (omega_mag[ix(i, j, k + 1, n)] - omega_mag[ix(i, j, k - 1, n)]);
                    let ngrad_mag =
                        (ngrad_x * ngrad_x + ngrad_y * ngrad_y + ngrad_z * ngrad_z).sqrt();
                    if ngrad_mag > 1e-8 {
                        let nx = ngrad_x / ngrad_mag;
                        let ny = ngrad_y / ngrad_mag;
                        let nz = ngrad_z / ngrad_mag;
                        // f = ε * (N × ω) * h
                        let wx = omega_x[idx];
                        let wy = omega_y[idx];
                        let wz = omega_z[idx];
                        let fx = eps * (ny * wz - nz * wy) * h;
                        let fy = eps * (nz * wx - nx * wz) * h;
                        let fz = eps * (nx * wy - ny * wx) * h;
                        self.u[idx] += dt * fx;
                        self.v[idx] += dt * fy;
                        self.w[idx] += dt * fz;
                    }
                }
            }
        }
    }

    /// 浮力（温度差驱动上升）
    /// f_y = α * (T - T_amb) * density_correction
    /// 烟雾密度越大下沉感越强（高温时仍上升）
    fn buoyancy(&mut self, dt: f32) {
        let n = self.n;
        let alpha = self.buoyancy_alpha;
        let t_amb = self.ambient_temp;
        for k in 1..=n {
            for j in 1..=n {
                for i in 1..=n {
                    let idx = ix(i, j, k, n);
                    let dT = self.temperature[idx] - t_amb;
                    // 高温上升（y方向为上）
                    self.v[idx] += dt * alpha * dT * 0.1;
                    // 烟雾密度造成的轻微下沉（仅低温时）
                    if dT < 0.0 {
                        self.v[idx] -= dt * alpha * self.density[idx] * 0.01;
                    }
                }
            }
        }
    }

    /// 获取网格点密度（用于渲染）
    pub fn density_at(&self, i: usize, j: usize, k: usize) -> f32 {
        self.density[ix(i, j, k, self.n)]
    }

    /// 获取网格点温度
    pub fn temperature_at(&self, i: usize, j: usize, k: usize) -> f32 {
        self.temperature[ix(i, j, k, self.n)]
    }

    /// 获取网格点速度
    pub fn velocity_at(&self, i: usize, j: usize, k: usize) -> [f32; 3] {
        let idx = ix(i, j, k, self.n);
        [self.u[idx], self.v[idx], self.w[idx]]
    }

    /// 内部分辨率
    pub fn resolution(&self) -> usize {
        self.n
    }
}

/// 黑体辐射色温映射（Planckian locus 近似）
/// 用于火焰/熔岩渲染的颜色计算
///
/// 来源：Tanner Helland 黑体色温算法
/// T in [1000, 40000] K，返回线性 RGB (0.0-1.0)
pub fn blackbody_rgb(t_kelvin: f32) -> (f32, f32, f32) {
    let t = t_kelvin / 100.0;
    let t = t.clamp(10.0, 400.0);
    // Red
    let r = if t <= 66.0 { 255.0 } else { 329.698727446 * (t - 60.0).powf(-0.1332047592) };
    // Green
    let g = if t <= 66.0 {
        99.4708025861 * t.ln() - 161.1195681661
    } else {
        288.1221695283 * (t - 60.0).powf(-0.0755148492)
    };
    // Blue
    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.5177312231 * (t - 10.0).ln() - 305.0447927307
    };
    let clamp = |v: f32| -> f32 { v.clamp(0.0, 255.0) / 255.0 };
    (clamp(r), clamp(g), clamp(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_creation() {
        let solver = StamFluidSolver3D::new(8, 1e-5, 1e-5);
        assert_eq!(solver.resolution(), 8);
        assert_eq!(solver.density.len(), 10 * 10 * 10);
    }

    #[test]
    fn test_blackbody_red_temp() {
        // 1000K 应该是暗红色
        let (r, g, b) = blackbody_rgb(1000.0);
        assert!(r > 0.9);
        assert!(g < 0.3);
        assert!(b < 0.1);
    }

    #[test]
    fn test_blackbody_white() {
        // 6500K 接近 sRGB 白点 (D65 日光)
        let (r, g, b) = blackbody_rgb(6500.0);
        assert!(r > 0.9);
        assert!(g > 0.9);
        assert!(b > 0.9);
    }
}
