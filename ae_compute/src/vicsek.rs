//! Vicsek 模型 — 主动物质集体运动 (Flocking)
//!
//! Tamás Vicsek, András Czirók, Eshel Ben-Jacob, Inon Cohen, Ofer Shochet,
//! "Novel Type of Phase Transition in a System of Self-Driven Particles",
//! Phys. Rev. Lett. 75, 1226 (1995).
//!
//! N 个自驱动粒子在 2D 周期域中运动, 每个粒子有位置 r_i 和朝向 θ_i.
//! 通过局部对齐 + 噪声产生集体涌现 (鸟群、鱼群、细菌群落、交通流).
//!
//! 动力学 (离散时间, dt=1):
//!   1. 邻居平均朝向: θ_avg(i) = atan2( Σ sin θ_j, Σ cos θ_j ), |r_i - r_j| < R
//!   2. 更新朝向: θ_i(t+1) = θ_avg(i) + ξ_i, ξ ~ Uniform[-η/2, η/2]
//!   3. 更新位置: r_i(t+1) = r_i(t) + v0 (cos θ_i, sin θ_i)  (周期边界)
//!
//! 序参量 (极化度):
//!   v_a = |Σ_i v_i| / (N v0) ∈ [0, 1]
//!   - v_a → 1: 有序相 (鸟群对齐, 同向飞行)
//!   - v_a → 0: 无序相 (随机方向)
//!
//! 相变:
//!   - 噪声 η 调控: 低 η → 有序; 高 η → 无序
//!   - 临界噪声 η_c ≈ 0.45 (ρ=0.5, v0=0.5, R=1, 标准参数)
//!   - Vicsek 相变属非平衡一级相变 (存在滞后、相分离带状结构)
//!
//! 物理:
//!   - 破缺旋转对称性 (涌现整体朝向)
//!   - 破缺空间反演对称性 (运动有方向)
//!   - 局部作用 → 全局序 (短程作用长程关联)
//!   - 与 Kuramoto (同步)、Toner-Tu (flocking 流体) 相关

use std::f64::consts::PI;

/// 本地 xorshift64 伪随机数生成器 (零依赖, 可复现)
struct VicsekRng {
    state: u64,
}

impl VicsekRng {
    fn new(seed: u64) -> Self {
        VicsekRng {
            state: if seed == 0 {
                0xdeadbeefcafebabe
            } else {
                seed
            },
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        (self.state >> 32) as u32
    }

    /// 返回 [0, 1) 均匀分布
    fn next_f64(&mut self) -> f64 {
        let hi = self.next_u32() as u64;
        let lo = self.next_u32() as u64;
        let bits = (hi << 21) | (lo >> 11);
        (bits as f64) / ((1u64 << 53) as f64)
    }
}

/// Vicsek 模型配置
#[derive(Clone, Debug)]
pub struct VicsekConfig {
    /// 粒子数 N
    pub n_particles: usize,
    /// 域尺寸 L (周期边界, 域 [0,L)×[0,L))
    pub box_size: f64,
    /// 恒定速率 v0
    pub speed: f64,
    /// 交互半径 R
    pub interaction_radius: f64,
    /// 噪声强度 η ∈ [0, 2π)
    pub noise: f64,
    /// 元胞列表每边格数 (cell_size = L / n_cells, 需 >= R)
    pub n_cells: usize,
}

impl Default for VicsekConfig {
    fn default() -> Self {
        // Vicsek 1995 标准参数: ρ=0.5, v0=0.5, R=1, L=10, N=50
        Self {
            n_particles: 50,
            box_size: 10.0,
            speed: 0.5,
            interaction_radius: 1.0,
            noise: 0.5,
            n_cells: 10, // cell_size = 1.0 = R
        }
    }
}

/// 粒子状态
#[derive(Clone, Debug, Default)]
pub struct Particle {
    pub x: f64,
    pub y: f64,
    pub theta: f64,
}

/// Vicsek 模型求解器
pub struct VicsekSolver {
    pub config: VicsekConfig,
    pub particles: Vec<Particle>,
    pub step_count: u64,
    /// 每步极化度历史 (用于分析相变)
    pub order_history: Vec<f64>,
}

impl VicsekSolver {
    pub fn new(config: VicsekConfig) -> Self {
        assert!(config.n_particles > 0, "n_particles must be > 0");
        assert!(config.box_size > 0.0, "box_size must be > 0");
        assert!(config.speed >= 0.0, "speed must be >= 0");
        assert!(config.interaction_radius > 0.0, "interaction_radius must be > 0");
        assert!(config.noise >= 0.0, "noise must be >= 0");
        assert!(config.n_cells >= 1, "n_cells must be >= 1");

        Self {
            config,
            particles: Vec::new(),
            step_count: 0,
            order_history: Vec::new(),
        }
    }

    /// 初始化: 随机位置 + 随机朝向 ∈ [0, 2π)
    pub fn initialize_random(&mut self, seed: u64) {
        let mut rng = VicsekRng::new(seed);
        let l = self.config.box_size;
        self.particles.clear();
        self.particles.reserve(self.config.n_particles);
        for _ in 0..self.config.n_particles {
            let x = rng.next_f64() * l;
            let y = rng.next_f64() * l;
            let theta = rng.next_f64() * 2.0 * PI;
            self.particles.push(Particle { x, y, theta });
        }
        self.step_count = 0;
        self.order_history.clear();
    }

    /// 初始化: 全部对齐朝向 theta0, 随机位置
    pub fn initialize_aligned(&mut self, seed: u64, theta0: f64) {
        let mut rng = VicsekRng::new(seed);
        let l = self.config.box_size;
        self.particles.clear();
        for _ in 0..self.config.n_particles {
            let x = rng.next_f64() * l;
            let y = rng.next_f64() * l;
            self.particles.push(Particle { x, y, theta: theta0 });
        }
        self.step_count = 0;
        self.order_history.clear();
    }

    /// 计算极化度 (序参量) v_a = |Σ v_i| / (N v0)
    pub fn order_parameter(&self) -> f64 {
        if self.particles.is_empty() || self.config.speed == 0.0 {
            return 0.0;
        }
        let mut sx = 0.0;
        let mut sy = 0.0;
        for p in &self.particles {
            sx += p.theta.cos();
            sy += p.theta.sin();
        }
        let mag = (sx * sx + sy * sy).sqrt();
        mag / (self.particles.len() as f64)
    }

    /// 平均速度向量 (vx, vy) / (N v0) ∈ [-1, 1]^2
    pub fn mean_velocity(&self) -> (f64, f64) {
        if self.particles.is_empty() {
            return (0.0, 0.0);
        }
        let mut sx = 0.0;
        let mut sy = 0.0;
        for p in &self.particles {
            sx += p.theta.cos();
            sy += p.theta.sin();
        }
        let n = self.particles.len() as f64;
        (sx / n, sy / n)
    }

    /// 周期边界最小镜像距离分量
    #[inline]
    fn min_image(dx: f64, l: f64) -> f64 {
        let half = 0.5 * l;
        let mut d = dx;
        if d > half {
            d -= l;
        } else if d < -half {
            d += l;
        }
        d
    }

    /// 周期边界包裹坐标到 [0, L)
    #[inline]
    fn wrap(v: f64, l: f64) -> f64 {
        let mut r = v % l;
        if r < 0.0 {
            r += l;
        }
        r
    }

    /// 构建元胞列表: head/next 链表实现 O(N) 邻居查找
    fn build_cell_list(&self) -> (Vec<i32>, Vec<i32>) {
        let nc = self.config.n_cells;
        let l = self.config.box_size;
        let cell_size = l / nc as f64;
        let n_total = nc * nc;
        let mut head = vec![-1i32; n_total];
        let mut next = vec![-1i32; self.particles.len()];

        for (i, p) in self.particles.iter().enumerate() {
            let cx = (p.x / cell_size).floor() as usize % nc;
            let cy = (p.y / cell_size).floor() as usize % nc;
            let cidx = cy * nc + cx;
            next[i] = head[cidx];
            head[cidx] = i as i32;
        }
        (head, next)
    }

    /// 单步推进 (元胞列表 O(N))
    pub fn step(&mut self, seed: u64) {
        let mut rng = VicsekRng::new(seed);
        let l = self.config.box_size;
        let r = self.config.interaction_radius;
        let r2 = r * r;
        let eta = self.config.noise;
        let v0 = self.config.speed;
        let nc = self.config.n_cells;
        let cell_size = l / nc as f64;

        let (head, next) = self.build_cell_list();
        let n = self.particles.len();

        // 计算每个粒子的新朝向
        let mut new_theta = vec![0.0f64; n];

        for i in 0..n {
            let p = &self.particles[i];
            let cx = (p.x / cell_size).floor() as usize % nc;
            let cy = (p.y / cell_size).floor() as usize % nc;

            let mut sum_sin = 0.0;
            let mut sum_cos = 0.0;

            // 遍历 3x3 邻居元胞 (周期)
            for dcy in [-1i32, 0, 1] {
                for dcx in [-1i32, 0, 1] {
                    let nx_cell = ((cx as i32 + dcx + nc as i32) % nc as i32) as usize;
                    let ny_cell = ((cy as i32 + dcy + nc as i32) % nc as i32) as usize;
                    let cidx = ny_cell * nc + nx_cell;

                    let mut j = head[cidx];
                    while j >= 0 {
                        let ju = j as usize;
                        if ju != i {
                            let q = &self.particles[ju];
                            let dx = Self::min_image(q.x - p.x, l);
                            let dy = Self::min_image(q.y - p.y, l);
                            if dx * dx + dy * dy <= r2 {
                                sum_sin += q.theta.sin();
                                sum_cos += q.theta.cos();
                            }
                        }
                        j = next[ju];
                    }
                }
            }
            // 包含自身 (Vicsek 标准定义: 平均包含自己)
            sum_sin += p.theta.sin();
            sum_cos += p.theta.cos();

            let avg = sum_sin.atan2(sum_cos);
            // 加性噪声 ξ ∈ [-η/2, η/2)
            let xi = (rng.next_f64() - 0.5) * eta;
            new_theta[i] = avg + xi;
        }

        // 应用更新: 先朝向后位置 (避免串行偏差)
        for i in 0..n {
            let th = new_theta[i];
            let p = &mut self.particles[i];
            p.theta = th;
            p.x = Self::wrap(p.x + v0 * th.cos(), l);
            p.y = Self::wrap(p.y + v0 * th.sin(), l);
        }

        self.step_count += 1;
        self.order_history.push(self.order_parameter());
    }

    /// 多步推进
    pub fn run(&mut self, n_steps: usize, seed: u64) {
        for k in 0..n_steps {
            self.step(seed.wrapping_add(k as u64));
        }
    }

    /// 粒子数密度 ρ = N / L²
    pub fn density(&self) -> f64 {
        self.particles.len() as f64 / (self.config.box_size * self.config.box_size)
    }

    /// 平均邻居数 (诊断量)
    pub fn mean_neighbors(&self) -> f64 {
        if self.particles.is_empty() {
            return 0.0;
        }
        let l = self.config.box_size;
        let r = self.config.interaction_radius;
        let r2 = r * r;
        let (head, next) = self.build_cell_list();
        let nc = self.config.n_cells;
        let n = self.particles.len();
        let cell_size = l / nc as f64;
        let mut total = 0u64;

        for i in 0..n {
            let p = &self.particles[i];
            let cx = (p.x / cell_size).floor() as usize % nc;
            let cy = (p.y / cell_size).floor() as usize % nc;
            let mut count = 0u64;
            for dcy in [-1i32, 0, 1] {
                for dcx in [-1i32, 0, 1] {
                    let nx_cell = ((cx as i32 + dcx + nc as i32) % nc as i32) as usize;
                    let ny_cell = ((cy as i32 + dcy + nc as i32) % nc as i32) as usize;
                    let cidx = ny_cell * nc + nx_cell;
                    let mut j = head[cidx];
                    while j >= 0 {
                        let ju = j as usize;
                        if ju != i {
                            let q = &self.particles[ju];
                            let dx = Self::min_image(q.x - p.x, l);
                            let dy = Self::min_image(q.y - p.y, l);
                            if dx * dx + dy * dy <= r2 {
                                count += 1;
                            }
                        }
                        j = next[ju];
                    }
                }
            }
            total += count;
        }
        total as f64 / n as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_default() -> VicsekSolver {
        let mut s = VicsekSolver::new(VicsekConfig::default());
        s.initialize_random(42);
        s
    }

    #[test]
    fn test_default_config() {
        let cfg = VicsekConfig::default();
        assert_eq!(cfg.n_particles, 50);
        assert_eq!(cfg.box_size, 10.0);
        assert_eq!(cfg.speed, 0.5);
        assert_eq!(cfg.interaction_radius, 1.0);
        assert_eq!(cfg.noise, 0.5);
        assert_eq!(cfg.n_cells, 10);
    }

    #[test]
    fn test_solver_creation() {
        let s = VicsekSolver::new(VicsekConfig::default());
        assert!(s.particles.is_empty());
        assert_eq!(s.step_count, 0);
    }

    #[test]
    fn test_initialize_random() {
        let mut s = VicsekSolver::new(VicsekConfig::default());
        s.initialize_random(42);
        assert_eq!(s.particles.len(), 50);
        for p in &s.particles {
            assert!(p.x >= 0.0 && p.x < 10.0, "x in [0,L): {}", p.x);
            assert!(p.y >= 0.0 && p.y < 10.0, "y in [0,L): {}", p.y);
            assert!(p.theta >= 0.0 && p.theta < 2.0 * PI, "theta in [0,2pi): {}", p.theta);
        }
    }

    #[test]
    fn test_initialize_aligned() {
        let mut s = VicsekSolver::new(VicsekConfig::default());
        s.initialize_aligned(42, 0.0);
        assert_eq!(s.particles.len(), 50);
        for p in &s.particles {
            assert!((p.theta).abs() < 1e-12, "all aligned to 0: {}", p.theta);
        }
        // 完全对齐 → 极化度 = 1
        assert!((s.order_parameter() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_order_parameter_bounds() {
        let s = make_default();
        let va = s.order_parameter();
        assert!(va >= 0.0 && va <= 1.0, "v_a in [0,1]: {}", va);
    }

    #[test]
    fn test_order_parameter_random_near_zero() {
        // 大量粒子随机朝向, 极化度应接近 0 (大数定律)
        let cfg = VicsekConfig {
            n_particles: 5000,
            box_size: 100.0,
            ..VicsekConfig::default()
        };
        let mut s = VicsekSolver::new(cfg);
        s.initialize_random(7);
        let va = s.order_parameter();
        assert!(va < 0.05, "random orientations → v_a ≈ 0: {}", va);
    }

    #[test]
    fn test_order_parameter_aligned_is_one() {
        let mut s = VicsekSolver::new(VicsekConfig::default());
        s.initialize_aligned(42, 1.23);
        assert!((s.order_parameter() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_single_step_advances() {
        let mut s = make_default();
        let _va0 = s.order_parameter();
        s.step(99);
        assert_eq!(s.step_count, 1);
        let va1 = s.order_parameter();
        assert!(va1 >= 0.0 && va1 <= 1.0);
        assert_eq!(s.order_history.len(), 1);
        assert!((s.order_history[0] - va1).abs() < 1e-12);
    }

    #[test]
    fn test_particle_count_conserved() {
        let mut s = make_default();
        let n0 = s.particles.len();
        s.run(50, 42);
        assert_eq!(s.particles.len(), n0, "粒子数守恒");
    }

    #[test]
    fn test_periodic_boundary() {
        let mut s = VicsekSolver::new(VicsekConfig {
            n_particles: 1,
            box_size: 4.0,
            speed: 1.0,
            interaction_radius: 0.5,
            noise: 0.0,
            n_cells: 4,
        });
        // 单粒子朝 +x, 从 x=3.9 出发, 一步后应回绕到 x≈0.9
        s.particles.clear();
        s.particles.push(Particle { x: 3.9, y: 2.0, theta: 0.0 });
        s.step(1);
        let p = &s.particles[0];
        assert!((p.x - 0.9).abs() < 1e-9, "wrap +x: {}", p.x);
        assert!((p.y - 2.0).abs() < 1e-9, "y unchanged: {}", p.y);
        assert!((p.theta).abs() < 1e-9, "no noise → theta unchanged");
    }

    #[test]
    fn test_periodic_boundary_negative() {
        let mut s = VicsekSolver::new(VicsekConfig {
            n_particles: 1,
            box_size: 4.0,
            speed: 1.0,
            interaction_radius: 0.5,
            noise: 0.0,
            n_cells: 4,
        });
        // 单粒子朝 -x, 从 x=0.1 出发 → 回绕到 x≈3.1
        s.particles.clear();
        s.particles.push(Particle { x: 0.1, y: 2.0, theta: PI });
        s.step(1);
        let p = &s.particles[0];
        assert!((p.x - 3.1).abs() < 1e-9, "wrap -x: {}", p.x);
    }

    #[test]
    fn test_no_noise_aligned_stays_aligned() {
        // 零噪声 + 初始对齐 → 永远对齐 (有序不动点)
        let mut s = VicsekSolver::new(VicsekConfig {
            noise: 0.0,
            ..VicsekConfig::default()
        });
        s.initialize_aligned(42, 0.7);
        s.run(30, 42);
        assert!((s.order_parameter() - 1.0).abs() < 1e-6, "zero noise aligned → v_a=1");
    }

    #[test]
    fn test_high_noise_disordered() {
        // 高噪声 η=2π (满角随机) → 极化度低
        let mut s = VicsekSolver::new(VicsekConfig {
            noise: 2.0 * PI,
            n_particles: 300,
            box_size: 20.0,
            n_cells: 20,
            ..VicsekConfig::default()
        });
        s.initialize_aligned(42, 0.0);
        s.run(50, 42);
        let va = s.order_parameter();
        assert!(va < 0.2, "high noise → disordered v_a<0.2: {}", va);
    }

    #[test]
    fn test_low_noise_ordered() {
        // 低噪声 + 足够步数 → 高极化度 (有序相)
        let mut s = VicsekSolver::new(VicsekConfig {
            noise: 0.1,
            n_particles: 300,
            box_size: 20.0,
            n_cells: 20,
            ..VicsekConfig::default()
        });
        s.initialize_random(42);
        s.run(200, 42);
        let va = s.order_parameter();
        assert!(va > 0.6, "low noise → ordered v_a>0.6: {}", va);
    }

    #[test]
    fn test_phase_transition_monotonic() {
        // 序参量应随噪声增加而 (统计上) 单调下降
        let etas = [0.1, 0.3, 0.5, 1.0, 2.0, 4.0];
        let mut vas = Vec::new();
        for &eta in &etas {
            let mut s = VicsekSolver::new(VicsekConfig {
                noise: eta,
                n_particles: 200,
                box_size: 20.0,
                n_cells: 20,
                ..VicsekConfig::default()
            });
            s.initialize_random(123);
            s.run(150, 123);
            // 取最后 50 步时间平均
            let tail: f64 = s.order_history.iter().rev().take(50).sum::<f64>() / 50.0;
            vas.push(tail);
        }
        // 端点比较: 最低噪声应明显高于最高噪声
        assert!(vas[0] > vas[etas.len() - 1],
            "v_a(η=0.1)={} > v_a(η=4)={}", vas[0], vas[etas.len() - 1]);
    }

    #[test]
    fn test_density() {
        let s = make_default();
        // N=50, L=10 → ρ=0.5
        assert!((s.density() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_mean_neighbors_positive() {
        let s = make_default();
        let mn = s.mean_neighbors();
        assert!(mn >= 0.0);
    }

    #[test]
    fn test_mean_velocity_bounds() {
        let s = make_default();
        let (vx, vy) = s.mean_velocity();
        assert!(vx.abs() <= 1.0 + 1e-9);
        assert!(vy.abs() <= 1.0 + 1e-9);
    }

    #[test]
    fn test_long_run_no_nan_no_blowup() {
        let mut s = VicsekSolver::new(VicsekConfig {
            n_particles: 100,
            box_size: 15.0,
            noise: 0.5,
            n_cells: 15,
            ..VicsekConfig::default()
        });
        s.initialize_random(2024);
        s.run(500, 2024);
        for p in &s.particles {
            assert!(p.x.is_finite(), "x finite");
            assert!(p.y.is_finite(), "y finite");
            assert!(p.theta.is_finite(), "theta finite");
            assert!(p.x >= 0.0 && p.x < 15.0, "x in [0,L)");
            assert!(p.y >= 0.0 && p.y < 15.0, "y in [0,L)");
        }
        let va = s.order_parameter();
        assert!(va.is_finite() && va >= 0.0 && va <= 1.0);
    }

    #[test]
    fn test_order_history_grows() {
        let mut s = make_default();
        s.run(20, 42);
        assert_eq!(s.order_history.len(), 20);
        for &v in &s.order_history {
            assert!(v >= 0.0 && v <= 1.0);
        }
    }

    #[test]
    fn test_grid_size_flexible() {
        for n_part in [10, 50, 200] {
            let cfg = VicsekConfig {
                n_particles: n_part,
                box_size: 10.0,
                n_cells: 10,
                ..VicsekConfig::default()
            };
            let mut s = VicsekSolver::new(cfg);
            s.initialize_random(42);
            s.run(10, 42);
            assert_eq!(s.particles.len(), n_part);
        }
    }

    #[test]
    fn test_reproducible_same_seed() {
        let mut a = VicsekSolver::new(VicsekConfig::default());
        a.initialize_random(42);
        a.run(20, 42);

        let mut b = VicsekSolver::new(VicsekConfig::default());
        b.initialize_random(42);
        b.run(20, 42);

        for i in 0..a.particles.len() {
            assert!((a.particles[i].x - b.particles[i].x).abs() < 1e-12);
            assert!((a.particles[i].y - b.particles[i].y).abs() < 1e-12);
            assert!((a.particles[i].theta - b.particles[i].theta).abs() < 1e-12);
        }
    }

    #[test]
    fn test_different_seed_different_state() {
        let mut a = VicsekSolver::new(VicsekConfig::default());
        a.initialize_random(1);
        let mut b = VicsekSolver::new(VicsekConfig::default());
        b.initialize_random(2);
        let mut diff = false;
        for i in 0..a.particles.len() {
            if (a.particles[i].x - b.particles[i].x).abs() > 1e-9 {
                diff = true;
                break;
            }
        }
        assert!(diff, "different seeds → different states");
    }

    #[test]
    fn test_neighbor_locality() {
        // 两个相距 > R 的粒子互不为邻居
        let mut s = VicsekSolver::new(VicsekConfig {
            n_particles: 2,
            box_size: 10.0,
            interaction_radius: 1.0,
            speed: 0.0,
            noise: 0.0,
            n_cells: 10,
        });
        s.particles.clear();
        s.particles.push(Particle { x: 1.0, y: 5.0, theta: 0.0 });
        s.particles.push(Particle { x: 6.0, y: 5.0, theta: 0.0 });
        let mn = s.mean_neighbors();
        assert!(mn < 0.01, "distant particles: 0 neighbors, got {}", mn);
    }

    #[test]
    fn test_neighbor_close() {
        let mut s = VicsekSolver::new(VicsekConfig {
            n_particles: 2,
            box_size: 10.0,
            interaction_radius: 1.0,
            speed: 0.0,
            noise: 0.0,
            n_cells: 10,
        });
        s.particles.clear();
        s.particles.push(Particle { x: 5.0, y: 5.0, theta: 0.0 });
        s.particles.push(Particle { x: 5.4, y: 5.0, theta: 0.0 });
        let mn = s.mean_neighbors();
        assert!((mn - 1.0).abs() < 1e-9, "close particles: 1 neighbor each, got {}", mn);
    }

    #[test]
    fn test_periodic_neighbor_wrap() {
        // 两粒子分别在边界两侧, 周期镜像距离 < R
        let mut s = VicsekSolver::new(VicsekConfig {
            n_particles: 2,
            box_size: 10.0,
            interaction_radius: 1.0,
            speed: 0.0,
            noise: 0.0,
            n_cells: 10,
        });
        s.particles.clear();
        s.particles.push(Particle { x: 0.2, y: 5.0, theta: 0.0 });
        s.particles.push(Particle { x: 9.8, y: 5.0, theta: 0.0 });
        let mn = s.mean_neighbors();
        // 周期镜像距离 = 0.4 < R=1 → 互为邻居
        assert!((mn - 1.0).abs() < 1e-9, "periodic neighbor: got {}", mn);
    }

    #[test]
    fn test_invalid_config_panics() {
        assert!(std::panic::catch_unwind(|| {
            VicsekSolver::new(VicsekConfig { n_particles: 0, ..VicsekConfig::default() })
        }).is_err());
        assert!(std::panic::catch_unwind(|| {
            VicsekSolver::new(VicsekConfig { box_size: 0.0, ..VicsekConfig::default() })
        }).is_err());
        assert!(std::panic::catch_unwind(|| {
            VicsekSolver::new(VicsekConfig { interaction_radius: 0.0, ..VicsekConfig::default() })
        }).is_err());
    }
}
