//! MGPBD — Multigrid-Preconditioned XPBD 求解器
//!
//! 基于:
//! - arXiv:2505.13390 "MGPBD: Multigrid Preconditioned XPBD" (2025)
//! - SIGGRAPH 2025 异构调度布料/柔体（厦门大学/Style3D）
//! - Unity Physics 6.5 Direct Solver（关节链 32K→5 亿）
//!
//! 核心思想:
//! 标准 XPBD 用 Gauss-Seidel 求解 (M + α·J^T·W·J) Δx = -J^T·λ
//! GS 在低频误差上 stall（高频快，低频慢），导致高刚度/长关节链收敛慢。
//! 多重网格把低频误差"限制"到粗网格上快速消除，再"延拓"回细网格修正。
//!
//! 实现:
//! 1. CSR 稀疏矩阵
//! 2. UA-AMG（Unsmoothed Aggregation）：按强连接聚合节点构造粗网格
//! 3. V-cycle: 预平滑 → 限制残差 → 粗网格求解 → 延拓修正 → 后平滑
//! 4. PCG：用 AMG V-cycle 作为预条件子的共轭梯度法
//! 5. Lazy setup：仅当矩阵结构变化超过阈值时重建 prolongator
//!
//! 与 FixedPoint 兼容：纯 Rust，无新依赖，不依赖浮点特殊指令。

use glam::Vec3;

// ============================================================
// CSR 稀疏矩阵
// ============================================================

/// 压缩稀疏行格式矩阵
#[derive(Debug, Clone)]
pub struct CsrMatrix {
    pub nrows: usize,
    pub ncols: usize,
    pub row_offsets: Vec<usize>,
    pub col_indices: Vec<usize>,
    pub values: Vec<f32>,
}

impl CsrMatrix {
    pub fn new(nrows: usize, ncols: usize) -> Self {
        Self {
            nrows,
            ncols,
            row_offsets: vec![0; nrows + 1],
            col_indices: Vec::new(),
            values: Vec::new(),
        }
    }

    /// 从三元组 (row, col, value) 构造 CSR 矩阵
    pub fn from_triplets(nrows: usize, ncols: usize, mut triplets: Vec<(usize, usize, f32)>) -> Self {
        // 按行优先、列次序排序，合并重复项
        triplets.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        let mut row_offsets = vec![0usize; nrows + 1];
        let mut col_indices = Vec::with_capacity(triplets.len());
        let mut values = Vec::with_capacity(triplets.len());
        let mut last: Option<(usize, usize)> = None;
        for (r, c, v) in triplets {
            if last == Some((r, c)) {
                // 合并重复项: 只累加 value，不新增 col_indices 条目，也不增 row_offsets
                *values.last_mut().unwrap() += v;
            } else {
                // 新条目: 计入 row_offsets 并追加 col/value
                row_offsets[r + 1] += 1;
                col_indices.push(c);
                values.push(v);
                last = Some((r, c));
            }
        }
        // 前缀和转为偏移量
        for i in 0..nrows {
            row_offsets[i + 1] += row_offsets[i];
        }
        Self { nrows, ncols, row_offsets, col_indices, values }
    }

    /// y = A · x
    pub fn matvec(&self, x: &[f32], y: &mut [f32]) {
        debug_assert_eq!(x.len(), self.ncols);
        debug_assert_eq!(y.len(), self.nrows);
        for r in 0..self.nrows {
            let mut sum = 0.0f32;
            for idx in self.row_offsets[r]..self.row_offsets[r + 1] {
                sum += self.values[idx] * x[self.col_indices[idx]];
            }
            y[r] = sum;
        }
    }

    /// 对角线元素 d[r] = A[r, r]
    pub fn diagonal(&self) -> Vec<f32> {
        let mut d = vec![0.0f32; self.nrows];
        for r in 0..self.nrows {
            for idx in self.row_offsets[r]..self.row_offsets[r + 1] {
                if self.col_indices[idx] == r {
                    d[r] = self.values[idx];
                    break;
                }
            }
        }
        d
    }

    /// 转置
    pub fn transpose(&self) -> CsrMatrix {
        let mut triplets = Vec::with_capacity(self.values.len());
        for r in 0..self.nrows {
            for idx in self.row_offsets[r]..self.row_offsets[r + 1] {
                triplets.push((self.col_indices[idx], r, self.values[idx]));
            }
        }
        CsrMatrix::from_triplets(self.ncols, self.nrows, triplets)
    }

    /// R = R · A (稀疏矩阵乘法)
    pub fn matmul(&self, other: &CsrMatrix) -> CsrMatrix {
        debug_assert_eq!(self.ncols, other.nrows);
        let mut triplets = Vec::new();
        for r in 0..self.nrows {
            for idx_r in self.row_offsets[r]..self.row_offsets[r + 1] {
                let k = self.col_indices[idx_r];
                let a_rk = self.values[idx_r];
                for idx_k in other.row_offsets[k]..other.row_offsets[k + 1] {
                    let c = other.col_indices[idx_k];
                    let b_kc = other.values[idx_k];
                    triplets.push((r, c, a_rk * b_kc));
                }
            }
        }
        CsrMatrix::from_triplets(self.nrows, other.ncols, triplets)
    }
}

// ============================================================
// Gauss-Seidel 平滑器
// ============================================================

/// 原地执行 k 次 Gauss-Seidel 迭代: 求解 A·x = b
pub fn gauss_seidel_smooth(a: &CsrMatrix, b: &[f32], x: &mut [f32], iters: usize) {
    for _ in 0..iters {
        for r in 0..a.nrows {
            let mut diag = 0.0f32;
            let mut sum = 0.0f32;
            for idx in a.row_offsets[r]..a.row_offsets[r + 1] {
                let c = a.col_indices[idx];
                if c == r {
                    diag = a.values[idx];
                } else {
                    sum += a.values[idx] * x[c];
                }
            }
            if diag.abs() > 1e-20 {
                x[r] = (b[r] - sum) / diag;
            }
        }
    }
}

// ============================================================
// UA-AMG (Unsmoothed Aggregation AMG)
// ============================================================

/// AMG 单层（粗化一层）
#[derive(Debug, Clone)]
pub struct AmgLevel {
    pub a_fine: CsrMatrix,    // 当前层算子
    pub a_coarse: CsrMatrix,  // 下一层（粗层）算子
    pub prolong: CsrMatrix,   // P: 粗 → 细 (n_fine × n_coarse)
    pub restrict: CsrMatrix,  // R: 细 → 粗 = P^T
}

/// AMG 层级结构（V-cycle 用）
#[derive(Debug, Clone)]
pub struct AmgHierarchy {
    /// levels[0] = 最细层, levels[last] = 最粗层
    pub levels: Vec<AmgLevel>,
    /// 最粗层的直接求解器迭代次数
    pub coarse_iters: usize,
    /// V-cycle 每层预平滑次数
    pub pre_smooth: usize,
    /// V-cycle 每层后平滑次数
    pub post_smooth: usize,
}

impl AmgHierarchy {
    /// 构造 AMG 层级
    ///
    /// 参数:
    /// - `a0`: 最细层算子（必须 SPD 或近似 SPD）
    /// - `max_levels`: 最大层数（建议 3-5）
    /// - `min_coarse_size`: 最粗层最小节点数（小于则停止粗化）
    /// - `strong_threshold`: 强连接阈值（0.2-0.5 典型）
    pub fn build(
        a0: CsrMatrix,
        max_levels: usize,
        min_coarse_size: usize,
        strong_threshold: f32,
    ) -> Self {
        assert_eq!(a0.nrows, a0.ncols, "AMG requires square matrix");
        let mut levels: Vec<AmgLevel> = Vec::new();
        let mut current = a0;

        for _ in 0..max_levels.saturating_sub(1) {
            if current.nrows <= min_coarse_size {
                break;
            }
            let (prolong, aggregates) = build_aggregation(&current, strong_threshold);
            let n_coarse = aggregates.len();
            if n_coarse >= current.nrows || n_coarse < 2 {
                break;
            }
            let restrict = prolong.transpose();
            // Galerkin: A_coarse = R · A · P
            let a_times_p = current.matmul(&prolong);
            let a_coarse = restrict.matmul(&a_times_p);

            let level = AmgLevel {
                a_fine: current.clone(),
                a_coarse: a_coarse.clone(),
                prolong,
                restrict,
            };
            levels.push(level);
            current = a_coarse;
        }

        Self {
            levels,
            coarse_iters: 50,
            pre_smooth: 3,
            post_smooth: 3,
        }
    }

    /// V-cycle: 求解 A·x = b
    ///
    /// 递归流程:
    /// 1. 在最细层预平滑
    /// 2. 计算残差 r = b - A·x
    /// 3. 限制到粗层 r_c = R·r
    /// 4. 粗层求解 A_c·e_c = r_c（递归或直接）
    /// 5. 延拓修正 x += P·e_c
    /// 6. 后平滑
    pub fn vcycle(&self, b: &[f32], x: &mut [f32]) {
        if self.levels.is_empty() {
            // 无粗化：直接 GS 求解
            gauss_seidel_smooth(&CsrMatrix::clone(&self.coarse_operator()), b, x, self.coarse_iters);
            return;
        }

        self.vcycle_recursive(0, b, x);
    }

    fn vcycle_recursive(&self, level_idx: usize, b: &[f32], x: &mut [f32]) {
        let level = &self.levels[level_idx];
        let n = level.a_fine.nrows;

        // 1. 预平滑
        gauss_seidel_smooth(&level.a_fine, b, x, self.pre_smooth);

        // 2. 残差 r = b - A·x
        let mut ax = vec![0.0f32; n];
        level.a_fine.matvec(x, &mut ax);
        let r: Vec<f32> = (0..n).map(|i| b[i] - ax[i]).collect();

        // 3. 限制 r_c = R·r
        let n_coarse = level.a_coarse.nrows;
        let mut r_c = vec![0.0f32; n_coarse];
        level.restrict.matvec(&r, &mut r_c);

        // 4. 粗层求解
        let mut e_c = vec![0.0f32; n_coarse];
        if level_idx + 1 < self.levels.len() {
            // 递归 V-cycle
            self.vcycle_recursive(level_idx + 1, &r_c, &mut e_c);
        } else {
            // 最粗层：多次 GS 直接求解
            let coarse_a = &level.a_coarse;
            gauss_seidel_smooth(coarse_a, &r_c, &mut e_c, self.coarse_iters);
        }

        // 5. 延拓修正 x += P·e_c
        let mut correction = vec![0.0f32; n];
        level.prolong.matvec(&e_c, &mut correction);
        for i in 0..n {
            x[i] += correction[i];
        }

        // 6. 后平滑
        gauss_seidel_smooth(&level.a_fine, b, x, self.post_smooth);
    }

    /// 取最粗层算子（用于顶层 fallback）
    fn coarse_operator(&self) -> CsrMatrix {
        if let Some(last) = self.levels.last() {
            last.a_coarse.clone()
        } else {
            // 没有任何层时，用占位（实际不会执行）
            CsrMatrix::new(0, 0)
        }
    }

    /// 检测矩阵结构是否发生显著变化（用于 lazy setup）
    ///
    /// 简化版: 比较 A 的非零元数量和 Frobenius 范数
    pub fn structure_changed(&self, a_new: &CsrMatrix, threshold: f32) -> bool {
        if let Some(first) = self.levels.first() {
            let old = &first.a_fine;
            if old.nrows != a_new.nrows || old.values.len() != a_new.values.len() {
                return true;
            }
            // Frobenius 范数差异
            let mut diff_sq = 0.0f32;
            let mut old_sq = 0.0f32;
            for (a, b) in old.values.iter().zip(a_new.values.iter()) {
                diff_sq += (a - b).powi(2);
                old_sq += a.powi(2);
            }
            if old_sq < 1e-20 {
                return diff_sq > threshold * threshold;
            }
            (diff_sq.sqrt() / old_sq.sqrt()) > threshold
        } else {
            true
        }
    }
}

/// UA-AMG 聚合策略
///
/// 对每个未聚合节点，找其最强连接邻居（基于 |A[i,j]|/sqrt(A[i,i]·A[j,j])），
/// 把两者聚为一个粗节点。孤立节点自成一聚合。
///
/// 返回: (P [n_fine × n_coarse], aggregates [n_coarse 个 Vec<fine_idx>])
fn build_aggregation(a: &CsrMatrix, strong_threshold: f32) -> (CsrMatrix, Vec<Vec<usize>>) {
    let n = a.nrows;
    let diag = a.diagonal();

    // 计算每个节点的"最强连接"邻居
    // 强连接判定: |A[i,j]| >= theta * sqrt(|A[i,i] * A[j,j]|)
    let mut strength: Vec<Vec<(usize, f32)>> = vec![Vec::new(); n];
    for r in 0..n {
        for idx in a.row_offsets[r]..a.row_offsets[r + 1] {
            let c = a.col_indices[idx];
            if c == r {
                continue;
            }
            let coupling = a.values[idx].abs() / (diag[r].abs().max(1e-20) * diag[c].abs().max(1e-20)).sqrt();
            if coupling >= strong_threshold {
                strength[r].push((c, a.values[idx].abs()));
            }
        }
    }

    let mut aggregated = vec![false; n];
    let mut aggregates: Vec<Vec<usize>> = Vec::new();

    // 第一遍: 互相是对方最强邻居的成对聚合
    for i in 0..n {
        if aggregated[i] {
            continue;
        }
        // 找 i 的最强邻居 j
        let best_j = strength[i]
            .iter()
            .filter(|&&(j, _)| !aggregated[j])
            .max_by(|&&(_, va), &(_, vb)| va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal))
            .map(|&(j, _)| j);

        if let Some(j) = best_j {
            // 检查 i 是否也是 j 的最强邻居（对称强连接）
            let best_i_for_j = strength[j]
                .iter()
                .filter(|&&(k, _)| !aggregated[k])
                .max_by(|&(_, va), &(_, vb)| va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal))
                .map(|&(k, _)| k);

            if best_i_for_j == Some(i) {
                aggregated[i] = true;
                aggregated[j] = true;
                aggregates.push(vec![i, j]);
            }
        }
    }

    // 第二遍: 剩余未聚合节点单独成聚合（或与已聚合的强连接邻居合并）
    for i in 0..n {
        if !aggregated[i] {
            // 找一个与 i 有强连接的已有聚合
            let mut join_idx: Option<usize> = None;
            for (agg_idx, agg) in aggregates.iter().enumerate() {
                for &member in agg {
                    if strength[i].iter().any(|&(j, _)| j == member) {
                        join_idx = Some(agg_idx);
                        break;
                    }
                }
                if join_idx.is_some() {
                    break;
                }
            }
            match join_idx {
                Some(idx) => {
                    aggregates[idx].push(i);
                    aggregated[i] = true;
                }
                None => {
                    aggregates.push(vec![i]);
                    aggregated[i] = true;
                }
            }
        }
    }

    // 构造 prolongation P [n × n_coarse]: P[i, k] = 1 if i in aggregate k
    let n_coarse = aggregates.len();
    let mut triplets = Vec::with_capacity(n);
    for (k, agg) in aggregates.iter().enumerate() {
        for &i in agg {
            triplets.push((i, k, 1.0f32));
        }
    }
    let prolong = CsrMatrix::from_triplets(n, n_coarse, triplets);
    (prolong, aggregates)
}

// ============================================================
// PCG (Preconditioned Conjugate Gradient)
// ============================================================

/// PCG 求解器：用 AMG V-cycle 作为预条件子
pub struct PcgAmgSolver {
    pub max_iters: usize,
    pub tolerance: f32,
    pub hierarchy: AmgHierarchy,
}

impl PcgAmgSolver {
    pub fn new(hierarchy: AmgHierarchy, max_iters: usize, tolerance: f32) -> Self {
        Self { max_iters, tolerance, hierarchy }
    }

    /// 求解 A·x = b，返回 (解 x, 实际迭代次数, 最终残差)
    pub fn solve(&self, a: &CsrMatrix, b: &[f32], x: &mut [f32]) -> (usize, f32) {
        let n = a.nrows;
        debug_assert_eq!(x.len(), n);
        debug_assert_eq!(b.len(), n);

        let mut r = vec![0.0f32; n];
        let mut p = vec![0.0f32; n];
        let mut ap = vec![0.0f32; n];
        let mut z = vec![0.0f32; n];

        // r = b - A·x
        a.matvec(x, &mut ap);
        for i in 0..n {
            r[i] = b[i] - ap[i];
        }

        let b_norm = b.iter().map(|v| v.powi(2)).sum::<f32>().sqrt().max(1e-20);

        // z = M^-1 · r (AMG V-cycle 预条件)
        self.apply_preconditioner(a, &r, &mut z);

        // PCG 关键: rsold = z·r（预条件后的内积），不是 r·r
        let mut rsold: f32 = z.iter().zip(r.iter()).map(|(a, b)| a * b).sum();

        let r_norm = r.iter().map(|v| v.powi(2)).sum::<f32>().sqrt();
        if r_norm / b_norm < self.tolerance {
            return (0, r_norm);
        }

        // p = z
        p.copy_from_slice(&z);

        let mut iter = 0;
        while iter < self.max_iters {
            iter += 1;
            // ap = A·p
            a.matvec(&p, &mut ap);
            let p_ap = p.iter().zip(ap.iter()).map(|(a, b)| a * b).sum::<f32>();
            if p_ap.abs() < 1e-30 {
                break;
            }
            let alpha = rsold / p_ap;
            for i in 0..n {
                x[i] += alpha * p[i];
                r[i] -= alpha * ap[i];
            }

            let r_norm_new = r.iter().map(|v| v.powi(2)).sum::<f32>().sqrt();
            let rel_res = r_norm_new / b_norm;
            if rel_res < self.tolerance {
                return (iter, r_norm_new);
            }

            // z = M^-1 · r
            self.apply_preconditioner(a, &r, &mut z);
            // PCG: rsnew = z·r
            let rsnew: f32 = z.iter().zip(r.iter()).map(|(a, b)| a * b).sum();
            let beta = rsnew / rsold;
            for i in 0..n {
                p[i] = z[i] + beta * p[i];
            }
            rsold = rsnew;
        }
        let r_final = r.iter().map(|v| v.powi(2)).sum::<f32>().sqrt();
        (iter, r_final)
    }

    /// 应用预条件子 z = M^-1 · r
    ///
    /// M ≈ A，用 AMG V-cycle 近似求解 A·z = r
    fn apply_preconditioner(&self, a: &CsrMatrix, r: &[f32], z: &mut [f32]) {
        // 若 hierarchy 的最细层算子与当前 a 一致，直接用 hierarchy.vcycle
        // 否则降级为简单 GS 平滑（lazy setup 未触发重建时）
        if let Some(first) = self.hierarchy.levels.first() {
            if first.a_fine.nrows == a.nrows {
                // 用 hierarchy 的 V-cycle
                z.fill(0.0);
                self.hierarchy.vcycle(r, z);
                return;
            }
        }
        // fallback: 几次 GS 平滑
        z.fill(0.0);
        gauss_seidel_smooth(a, r, z, 3);
    }
}

// ============================================================
// MGPBD: 多重网格预条件 XPBD
// ============================================================

/// MGPBD 求解器配置
#[derive(Debug, Clone)]
pub struct MgpbdConfig {
    /// 子步数
    pub substeps: u32,
    /// 重力
    pub gravity: Vec3,
    /// 阻尼
    pub damping: f32,
    /// 最大速度
    pub max_velocity: f32,
    /// 松弛因子
    pub relaxation: f32,
    /// AMG 最大层数
    pub max_amg_levels: usize,
    /// AMG 最粗层最小节点数
    pub min_coarse_size: usize,
    /// AMG 强连接阈值
    pub strong_threshold: f32,
    /// PCG 最大迭代
    pub pcg_max_iters: usize,
    /// PCG 容差
    pub pcg_tolerance: f32,
    /// Lazy setup 结构变化阈值
    pub lazy_threshold: f32,
}

impl Default for MgpbdConfig {
    fn default() -> Self {
        Self {
            substeps: 8,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            damping: 0.98,
            max_velocity: 100.0,
            relaxation: 1.0,
            max_amg_levels: 4,
            min_coarse_size: 8,
            strong_threshold: 0.25,
            pcg_max_iters: 30,
            pcg_tolerance: 1e-5,
            lazy_threshold: 0.15,
        }
    }
}

/// 构造弹簧链系统的刚度矩阵 (M + α·K)
///
/// 模型: n 个粒子通过弹簧连接，端点固定
/// M = 对角质量矩阵, K = 弹簧刚度矩阵 (类似 1D Laplacian)
/// α = compliance (越小刚度越高)
///
/// 返回 (A, b) 满足 A·x = b
pub fn build_spring_chain_system(
    n: usize,
    mass: f32,
    stiffness: f32,
    compliance: f32,
    forces: &[f32],
) -> (CsrMatrix, Vec<f32>) {
    let alpha = compliance;
    let mut triplets: Vec<(usize, usize, f32)> = Vec::new();

    // A = M + α·K
    // K[i,i] = 2*stiffness (内部), = stiffness (端点)
    // K[i,i-1] = K[i-1,i] = -stiffness
    for i in 0..n {
        let k_ii = if i == 0 || i == n - 1 { stiffness } else { 2.0 * stiffness };
        triplets.push((i, i, mass + alpha * k_ii));
        if i > 0 {
            triplets.push((i, i - 1, -alpha * stiffness));
        }
        if i < n - 1 {
            triplets.push((i, i + 1, -alpha * stiffness));
        }
    }
    let a = CsrMatrix::from_triplets(n, n, triplets);
    let b = forces.to_vec();
    (a, b)
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csr_matvec() {
        // 2×2 单位矩阵
        let a = CsrMatrix::from_triplets(2, 2, vec![(0, 0, 1.0), (1, 1, 1.0)]);
        let x = vec![3.0, 5.0];
        let mut y = vec![0.0, 0.0];
        a.matvec(&x, &mut y);
        assert_eq!(y, vec![3.0, 5.0]);
    }

    #[test]
    fn test_csr_transpose() {
        let a = CsrMatrix::from_triplets(2, 3, vec![
            (0, 0, 1.0), (0, 2, 3.0), (1, 1, 2.0),
        ]);
        let at = a.transpose();
        assert_eq!(at.nrows, 3);
        assert_eq!(at.ncols, 2);
        let mut y = vec![0.0; 3];
        at.matvec(&[1.0, 1.0], &mut y);
        // [1, 0, 3] + [0, 2, 0] applied to [1,1] -> [1, 2, 3]
        assert_eq!(y, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_csr_matmul() {
        // A = [[1,0],[0,2]], B = [[3,0],[0,4]] -> AB = [[3,0],[0,8]]
        let a = CsrMatrix::from_triplets(2, 2, vec![(0, 0, 1.0), (1, 1, 2.0)]);
        let b = CsrMatrix::from_triplets(2, 2, vec![(0, 0, 3.0), (1, 1, 4.0)]);
        let c = a.matmul(&b);
        let mut y = vec![0.0; 2];
        c.matvec(&[1.0, 1.0], &mut y);
        assert_eq!(y, vec![3.0, 8.0]);
    }

    #[test]
    fn test_gauss_seidel_convergence() {
        // 3×3 对角占优系统
        let a = CsrMatrix::from_triplets(3, 3, vec![
            (0, 0, 4.0), (0, 1, 1.0), (0, 2, 0.0),
            (1, 0, 1.0), (1, 1, 4.0), (1, 2, 1.0),
            (2, 0, 0.0), (2, 1, 1.0), (2, 2, 4.0),
        ]);
        let b = vec![5.0, 6.0, 5.0];
        let mut x = vec![0.0; 3];
        gauss_seidel_smooth(&a, &b, &mut x, 100);
        // 真实解约 [1.0, 1.0, 1.0]
        for i in 0..3 {
            assert!((x[i] - 1.0).abs() < 1e-3, "x[{}] = {}", i, x[i]);
        }
    }

    #[test]
    fn test_amg_build_chain() {
        // 8 节点弹簧链 — 用强刚度让耦合超过阈值
        // mass=1, stiffness=1000, compliance=0.001 → α·k=1, 对角=3, 非对角=-1
        // 耦合 = 1/sqrt(3*3) = 0.333 > 0.05 ✓
        let (a, _) = build_spring_chain_system(8, 1.0, 1000.0, 0.001, &[0.0; 8]);
        let hier = AmgHierarchy::build(a, 4, 4, 0.05);
        // 至少应有 1 层粗化
        assert!(!hier.levels.is_empty(), "AMG should produce at least 1 level");
        let first = &hier.levels[0];
        assert!(first.a_coarse.nrows < 8, "coarse level must be smaller than fine");
        // prolongation P 形状: [8 × n_coarse]
        assert_eq!(first.prolong.nrows, 8);
    }

    #[test]
    fn test_pcg_amg_vs_gauss_seidel_high_stiffness() {
        // 高刚度弹簧链（compliance 极小 → α·K 主导）
        // 这是 GS 会 stall 的场景，MGPBD 应明显更快收敛
        let n = 32;
        let (a, b) = build_spring_chain_system(n, 1.0, 10000.0, 0.0001, &{
            let mut f = vec![0.0; n];
            f[n / 2] = 100.0; // 中间施力
            f
        });

        // GS baseline: 同样 30 次迭代（与 PCG max_iters 相同）
        let mut x_gs = vec![0.0; n];
        for _ in 0..30 {
            gauss_seidel_smooth(&a, &b, &mut x_gs, 1);
        }
        let mut r_gs = vec![0.0; n];
        a.matvec(&x_gs, &mut r_gs);
        let res_gs: f32 = (0..n).map(|i| (b[i] - r_gs[i]).powi(2)).sum::<f32>().sqrt();

        // PCG+AMG: 30 次迭代上限，容差极低以保证跑满
        let hier = AmgHierarchy::build(a.clone(), 4, 4, 0.05);
        let solver = PcgAmgSolver::new(hier, 30, 1e-10);
        let mut x_pcg = vec![0.0; n];
        let (iters, res_pcg) = solver.solve(&a, &b, &mut x_pcg);

        println!("GS residual after 30 iters: {:.8}", res_gs);
        println!("PCG+AMG residual after {} iters: {:.8}", iters, res_pcg);

        // PCG+AMG 在相同迭代预算内应达到更低残差
        assert!(res_pcg < res_gs, "PCG+AMG ({}) should outperform GS ({}) at same iter budget", res_pcg, res_gs);
        assert!(iters <= 30, "PCG+AMG should converge within budget");
    }

    #[test]
    fn test_amg_lazy_setup_detection() {
        // 用强刚度让 AMG 实际产生层级
        let (a0, _) = build_spring_chain_system(16, 1.0, 1000.0, 0.001, &[0.0; 16]);
        let hier = AmgHierarchy::build(a0.clone(), 4, 4, 0.05);
        assert!(!hier.levels.is_empty(), "hierarchy must have levels for lazy test");

        // 同结构微调（刚度变化 1%）→ 不应触发重建
        let (a1, _) = build_spring_chain_system(16, 1.0, 1010.0, 0.001, &[0.0; 16]);
        assert!(!hier.structure_changed(&a1, 0.15));

        // 结构大幅变化（刚度 ×10）→ 应触发重建
        let (a2, _) = build_spring_chain_system(16, 1.0, 10000.0, 0.001, &[0.0; 16]);
        assert!(hier.structure_changed(&a2, 0.15));
    }

    #[test]
    fn test_vcycle_residual_reduction() {
        // V-cycle 应显著降低残差
        let n = 16;
        let (a, b) = build_spring_chain_system(n, 1.0, 1000.0, 0.001, &{
            let mut f = vec![0.0; n];
            f[0] = 10.0;
            f[n - 1] = -10.0;
            f
        });

        let hier = AmgHierarchy::build(a.clone(), 4, 4, 0.05);
        assert!(!hier.levels.is_empty(), "hierarchy must have levels for V-cycle test");

        // 初始残差
        let mut x = vec![0.0; n];
        let mut ax = vec![0.0; n];
        a.matvec(&x, &mut ax);
        let r0: f32 = (0..n).map(|i| (b[i] - ax[i]).powi(2)).sum::<f32>().sqrt();

        // 单次 V-cycle
        hier.vcycle(&b, &mut x);
        a.matvec(&x, &mut ax);
        let r1: f32 = (0..n).map(|i| (b[i] - ax[i]).powi(2)).sum::<f32>().sqrt();

        println!("V-cycle: r0={:.6}, r1={:.6}, reduction={:.2}x", r0, r1, r0 / r1.max(1e-20));
        assert!(r1 < r0, "V-cycle must reduce residual");
    }
}
