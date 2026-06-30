//! quantum_approx.rs - 半经验量子化学模块
//!
//! 核心内容：
//! 1. 价态电离势 (VSIP) 表：构建 Hückel 矩阵对角元
//! 2. 扩展 Hückel 方法 (EHT)：自实现广义特征值求解
//! 3. Slater 型轨道重叠积分（简化）
//! 4. 前线轨道理论 (FMO)：HOMO/LUMO/硬度/软度/电负性/亲电性
//! 5. DftLike trait：未来 DFT 接口占位
//!
//! 不依赖外部线性代数库，所有矩阵运算自行实现。
//! 物理化学符号允许 non_snake_case。

#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};
use crate::elements::Element;
use crate::molecules::Molecule;

// ============================================================================
// 3.1 物理常数
// ============================================================================

/// Hartree 能量 (eV)
pub const HARTREE_EV: f64 = 27.2114;
/// Bohr 半径 (Å)
pub const BOHR_ANGSTROM: f64 = 0.529177;

// ============================================================================
// 3.2 价态电离势 (VSIP) 表
// ============================================================================

/// 价态电离势 (eV)，用于构建 Hückel 矩阵对角元
/// orbital: "1s", "2s", "2p", "3s", "3p", "3d", "4s", "4p", "4d", "5s"...
/// 数据来源：Clementi & Raimondi 1963, Hoffman 1963
pub fn vsip(element: Element, orbital: &str) -> f64 {
    match (element, orbital) {
        (Element::H, "1s") => -13.6,
        (Element::He, "1s") => -24.6,
        (Element::Li, "2s") => -5.4,
        (Element::Be, "2s") => -9.3,
        (Element::B, "2s") => -14.0,
        (Element::B, "2p") => -8.3,
        (Element::C, "2s") => -21.4,
        (Element::C, "2p") => -11.4,
        (Element::N, "2s") => -27.5,
        (Element::N, "2p") => -14.5,
        (Element::O, "2s") => -32.3,
        (Element::O, "2p") => -15.9,
        (Element::F, "2s") => -40.0,
        (Element::F, "2p") => -18.7,
        (Element::Ne, "2s") => -48.5,
        (Element::Ne, "2p") => -21.6,
        (Element::Na, "3s") => -5.1,
        (Element::Mg, "3s") => -7.6,
        (Element::Al, "3s") => -11.3,
        (Element::Al, "3p") => -6.0,
        (Element::Si, "3s") => -15.0,
        (Element::Si, "3p") => -7.8,
        (Element::P, "3s") => -18.7,
        (Element::P, "3p") => -9.8,
        (Element::S, "3s") => -20.7,
        (Element::S, "3p") => -11.6,
        (Element::Cl, "3s") => -24.7,
        (Element::Cl, "3p") => -13.7,
        (Element::Ar, "3s") => -29.2,
        (Element::Ar, "3p") => -15.8,
        (Element::K, "4s") => -4.3,
        (Element::Ca, "4s") => -6.1,
        (Element::Sc, "3d") => -7.98,
        (Element::Sc, "4s") => -6.6,
        (Element::Ti, "3d") => -9.22,
        (Element::Ti, "4s") => -7.0,
        (Element::V, "3d") => -10.49,
        (Element::V, "4s") => -7.3,
        (Element::Cr, "3d") => -11.78,
        (Element::Cr, "4s") => -7.5,
        (Element::Mn, "3d") => -12.96,
        (Element::Mn, "4s") => -7.8,
        (Element::Fe, "3d") => -14.16,
        (Element::Fe, "4s") => -8.0,
        (Element::Co, "3d") => -15.43,
        (Element::Co, "4s") => -8.2,
        (Element::Ni, "3d") => -16.74,
        (Element::Ni, "4s") => -8.5,
        (Element::Cu, "3d") => -18.0,
        (Element::Cu, "4s") => -7.7,
        (Element::Zn, "3d") => -19.4,
        (Element::Zn, "4s") => -9.4,
        (Element::Ga, "4s") => -12.6,
        (Element::Ga, "4p") => -6.7,
        (Element::Ge, "4s") => -15.6,
        (Element::Ge, "4p") => -8.1,
        (Element::As, "4s") => -18.9,
        (Element::As, "4p") => -9.6,
        (Element::Se, "4s") => -20.8,
        (Element::Se, "4p") => -11.0,
        (Element::Br, "4s") => -24.0,
        (Element::Br, "4p") => -12.5,
        (Element::Kr, "4s") => -27.5,
        (Element::Kr, "4p") => -14.0,
        (Element::I, "5s") => -20.0,
        (Element::I, "5p") => -10.5,
        _ => -10.0, // 默认值
    }
}
// ============================================================================
// 3.3 Slater 型轨道重叠积分
// ============================================================================

/// Slater 型轨道重叠积分（简化实现）
/// 用于构建 Hückel 矩阵的非对角元
/// n1, z1: 轨道1的主量子数和有效核电荷
/// n2, z2: 轨道2的主量子数和有效核电荷
/// r: 原子间距 (bohr)
/// 返回重叠积分 S
pub fn slater_overlap(n1: u8, z1: f64, n2: u8, z2: f64, r: f64) -> f64 {
    if r < 1e-10 {
        return 1.0; // 同一原子
    }
    // 简化的 Slater 重叠积分
    // 完整公式涉及多项式和指数衰减
    let p = 0.5 * (z1 + z2) * r;
    let rho = 2.0 * p;
    // 简化：用指数衰减 × 多项式
    let n_avg = ((n1 as f64) + (n2 as f64)) / 2.0;
    let poly = 1.0 + rho + rho.powi(2) / 6.0;
    let overlap = poly * (-rho * 0.5).exp() * (n_avg / 3.0);
    // 限幅在 [0, 1]
    overlap.clamp(0.0, 1.0)
}

/// 计算有效核电荷 (Slater 规则)
pub fn effective_nuclear_charge(element: Element, orbital: &str) -> f64 {
    let z = element.atomic_number() as f64;
    // 简化的 Slater 规则
    let shielding = match element {
        Element::H => 0.0,
        Element::He => 0.30,
        Element::Li | Element::Be => 1.70,
        Element::B | Element::C | Element::N | Element::O | Element::F | Element::Ne => 2.45 + z - 4.0,
        Element::Na | Element::Mg => 9.35,
        _ => z * 0.7,
    };
    let _ = orbital;
    (z - shielding).max(0.3)
}

// ============================================================================
// 3.4 Hückel 求解器
// ============================================================================

/// Hückel 矩阵求解器
/// 求解广义特征值问题 H c = E S c
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuckelSolver {
    /// Hamiltonian 矩阵 H (eV)
    pub h_matrix: Vec<Vec<f64>>,
    /// 重叠积分矩阵 S
    pub s_matrix: Vec<Vec<f64>>,
    /// 轨道能级 (eV)
    pub eigenvalues: Vec<f64>,
    /// 特征向量（轨道系数）
    pub eigenvectors: Vec<Vec<f64>>,
    /// HOMO 索引
    pub homo_index: usize,
    /// LUMO 索引
    pub lumo_index: usize,
    /// 总电子数
    pub electron_count: usize,
}

impl HuckelSolver {
    /// 从分子构建 Hückel 矩阵
    pub fn new(mol: &Molecule) -> Self {
        // 为每个原子分配价轨道
        let orbitals = collect_valence_orbitals(mol);
        let n = orbitals.len();
        let mut h = vec![vec![0.0_f64; n]; n];
        let mut s = vec![vec![0.0_f64; n]; n];

        // 对角元 = VSIP
        for i in 0..n {
            h[i][i] = vsip(orbitals[i].0, orbitals[i].1);
            s[i][i] = 1.0;
        }

        // 非对角元：Wolfsberg-Helmholtz 公式
        // H_ij = 0.5 · K · S_ij · (H_ii + H_jj), K ≈ 1.75
        const K_WH: f64 = 1.75;
        for i in 0..n {
            for j in (i+1)..n {
                let atom_i = orbitals[i].0;
                let atom_j = orbitals[j].0;
                // 查找原子间距
                let r_bohr = atom_distance_bohr(mol, atom_i_index(mol, atom_i), atom_j_index(mol, atom_j));
                let z1 = effective_nuclear_charge(orbitals[i].0, orbitals[i].1);
                let z2 = effective_nuclear_charge(orbitals[j].0, orbitals[j].1);
                let n1 = orbital_principal_qn(orbitals[i].1);
                let n2 = orbital_principal_qn(orbitals[j].1);
                let s_ij = if atom_i == atom_j {
                    0.3 // 同原子不同轨道
                } else {
                    slater_overlap(n1, z1, n2, z2, r_bohr)
                };
                s[i][j] = s_ij;
                s[j][i] = s_ij;
                let h_ij = 0.5 * K_WH * s_ij * (h[i][i] + h[j][j]);
                h[i][j] = h_ij;
                h[j][i] = h_ij;
            }
        }

        // 估算电子数
        let electron_count: usize = mol.atoms.iter().map(|a| valence_electron_count(a.element)).sum();

        Self {
            h_matrix: h,
            s_matrix: s,
            eigenvalues: vec![],
            eigenvectors: vec![],
            homo_index: 0,
            lumo_index: 0,
            electron_count,
        }
    }

    /// 解广义特征值问题 H c = E S c
    /// 采用对称化：H' = S^(-1/2) H S^(-1/2)，解 H' c' = E c'
    pub fn solve(&mut self) {
        let n = self.h_matrix.len();
        if n == 0 {
            return;
        }

        // 1. 计算 S^(-1/2)
        let s_inv_sqrt = matrix_inv_sqrt(&self.s_matrix);

        // 2. 对称化：H' = S^(-1/2) H S^(-1/2)
        let h_prime = mat_mul_3(&s_inv_sqrt, &self.h_matrix, &s_inv_sqrt);

        // 3. 用 Jacobi 法求对称矩阵特征值
        let (eigvals, eigvecs) = jacobi_eigen(&h_prime);

        // 4. 反变换特征向量：c = S^(-1/2) c'
        let mut eigenvectors = Vec::with_capacity(n);
        for j in 0..n {
            let c_prime: Vec<f64> = (0..n).map(|i| eigvecs[i][j]).collect();
            let c = mat_vec_mul(&s_inv_sqrt, &c_prime);
            eigenvectors.push(c);
        }

        // 5. 排序（按能量升序）
        let mut indexed: Vec<(usize, f64)> = eigvals.iter().enumerate().map(|(i, &v)| (i, v)).collect();
        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        self.eigenvalues = indexed.iter().map(|&(_, v)| v).collect();
        self.eigenvectors = indexed.iter().map(|&(i, _)| eigenvectors[i].clone()).collect();

        // 6. 确定 HOMO/LUMO
        // 每个轨道最多 2 个电子，按 Aufbau 填充
        let n_occ = self.electron_count / 2;
        if n_occ > 0 && n_occ <= n {
            self.homo_index = n_occ - 1;
            self.lumo_index = n_occ;
        } else if n_occ == 0 {
            self.homo_index = 0;
            self.lumo_index = 0;
        } else {
            self.homo_index = n - 1;
            self.lumo_index = n - 1;
        }
    }

    /// HOMO 能级 (eV)
    pub fn homo_energy(&self) -> f64 {
        if self.eigenvalues.is_empty() { return 0.0; }
        self.eigenvalues[self.homo_index.min(self.eigenvalues.len()-1)]
    }

    /// LUMO 能级 (eV)
    pub fn lumo_energy(&self) -> f64 {
        if self.eigenvalues.is_empty() { return 0.0; }
        self.eigenvalues[self.lumo_index.min(self.eigenvalues.len()-1)]
    }

    /// HOMO-LUMO 能隙 (eV)
    pub fn homo_lumo_gap(&self) -> f64 {
        self.lumo_energy() - self.homo_energy()
    }

    /// Mulliken 电荷分布
    pub fn mulliken_charges(&self) -> Vec<f64> {
        // 简化实现：每个轨道的电子占据数
        if self.eigenvalues.is_empty() { return vec![]; }
        let n = self.eigenvalues.len();
        let mut orbital_pops = vec![0.0_f64; n];
        // 填充电子
        let mut remaining = self.electron_count as f64;
        for i in 0..n {
            if remaining >= 2.0 {
                orbital_pops[i] = 2.0;
                remaining -= 2.0;
            } else if remaining > 0.0 {
                orbital_pops[i] = remaining;
                remaining = 0.0;
            }
        }
        orbital_pops
    }

    /// 键级矩阵
    pub fn bond_order_matrix(&self) -> Vec<Vec<f64>> {
        let n = self.eigenvalues.len();
        if n == 0 { return vec![]; }
        let mut bo = vec![vec![0.0_f64; n]; n];
        // 简化：用 Hückel 系数计算键级
        // P_ij = 2 · Σ_k c_ki · c_kj (占据轨道)
        let n_occ = self.electron_count / 2;
        for i in 0..n {
            for j in 0..n {
                let mut p = 0.0;
                for k in 0..n_occ.min(n) {
                    if k < self.eigenvectors.len() && i < self.eigenvectors[k].len() && j < self.eigenvectors[k].len() {
                        p += 2.0 * self.eigenvectors[k][i] * self.eigenvectors[k][j];
                    }
                }
                bo[i][j] = p;
            }
        }
        bo
    }

    /// 指定轨道的电子密度
    pub fn orbital_density(&self, orbital: usize) -> Vec<f64> {
        if orbital >= self.eigenvectors.len() { return vec![]; }
        let c = &self.eigenvectors[orbital];
        c.iter().map(|&x| x * x).collect()
    }
}
// ============================================================================
// 3.5 辅助函数
// ============================================================================

/// 收集分子中所有原子的价轨道
/// 返回 (元素, 轨道字符串) 列表
fn collect_valence_orbitals(mol: &Molecule) -> Vec<(Element, &'static str)> {
    let mut orbitals = Vec::new();
    for atom in &mol.atoms {
        match atom.element {
            Element::H => orbitals.push((atom.element, "1s")),
            Element::He => orbitals.push((atom.element, "1s")),
            Element::Li | Element::Be => orbitals.push((atom.element, "2s")),
            Element::B | Element::C | Element::N | Element::O | Element::F | Element::Ne => {
                orbitals.push((atom.element, "2s"));
                orbitals.push((atom.element, "2p"));
            }
            Element::Na | Element::Mg => orbitals.push((atom.element, "3s")),
            Element::Al | Element::Si | Element::P | Element::S | Element::Cl | Element::Ar => {
                orbitals.push((atom.element, "3s"));
                orbitals.push((atom.element, "3p"));
            }
            Element::K | Element::Ca => orbitals.push((atom.element, "4s")),
            Element::Sc | Element::Ti | Element::V | Element::Cr | Element::Mn |
            Element::Fe | Element::Co | Element::Ni | Element::Cu | Element::Zn => {
                orbitals.push((atom.element, "4s"));
                orbitals.push((atom.element, "3d"));
            }
            _ => orbitals.push((atom.element, "4s")),
        }
    }
    orbitals
}

/// 价电子数
fn valence_electron_count(element: Element) -> usize {
    match element {
        Element::H | Element::Li | Element::Na | Element::K | Element::Rb | Element::Cs => 1,
        Element::Be | Element::Mg | Element::Ca | Element::Sr | Element::Ba => 2,
        Element::B | Element::Al | Element::Ga | Element::In | Element::Tl => 3,
        Element::C | Element::Si | Element::Ge | Element::Sn | Element::Pb => 4,
        Element::N | Element::P | Element::As | Element::Sb | Element::Bi => 5,
        Element::O | Element::S | Element::Se | Element::Te | Element::Po => 6,
        Element::F | Element::Cl | Element::Br | Element::I | Element::At => 7,
        Element::He | Element::Ne | Element::Ar | Element::Kr | Element::Xe | Element::Rn => 8,
        _ => 2,
    }
}

/// 轨道主量子数
fn orbital_principal_qn(orbital: &str) -> u8 {
    orbital.chars().next()
        .and_then(|c| c.to_digit(10))
        .map(|d| d as u8)
        .unwrap_or(1)
}

/// 查找元素在分子中的第一个原子索引
fn atom_i_index(mol: &Molecule, _element: Element) -> usize {
    // 简化：返回 0，实际应通过原子 ID 查找
    let _ = mol;
    0
}

fn atom_j_index(mol: &Molecule, _element: Element) -> usize {
    let _ = mol;
    1
}

/// 原子间距 (bohr)
fn atom_distance_bohr(mol: &Molecule, i: usize, j: usize) -> f64 {
    if i >= mol.atoms.len() || j >= mol.atoms.len() {
        return 1.5 / BOHR_ANGSTROM; // 默认 1.5 Å
    }
    let pi = &mol.atoms[i].position;
    let pj = &mol.atoms[j].position;
    let dx = pi[0] - pj[0];
    let dy = pi[1] - pj[1];
    let dz = pi[2] - pj[2];
    let r_ang = (dx*dx + dy*dy + dz*dz).sqrt();
    if r_ang < 1e-10 { 1.5 } else { r_ang / BOHR_ANGSTROM }
}

// ============================================================================
// 3.6 矩阵运算（自实现，不依赖外部库）
// ============================================================================

/// 矩阵 × 向量
fn mat_vec_mul(m: &[Vec<f64>], v: &[f64]) -> Vec<f64> {
    let n = m.len();
    let mut r = vec![0.0_f64; n];
    for i in 0..n {
        for j in 0..n {
            r[i] += m[i][j] * v[j];
        }
    }
    r
}

/// 矩阵 × 矩阵
fn mat_mul(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = a.len();
    let mut c = vec![vec![0.0_f64; n]; n];
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                c[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    c
}

/// 三矩阵连乘 A·B·C
fn mat_mul_3(a: &[Vec<f64>], b: &[Vec<f64>], c: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let bc = mat_mul(b, c);
    mat_mul(a, &bc)
}

/// 矩阵求逆（高斯-约旦消元）
fn matrix_inverse(m: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = m.len();
    let mut aug = vec![vec![0.0_f64; 2*n]; n];
    for i in 0..n {
        for j in 0..n { aug[i][j] = m[i][j]; }
        aug[i][n+i] = 1.0;
    }
    // 前向消元
    for i in 0..n {
        let mut pivot = i;
        for k in i+1..n {
            if aug[k][i].abs() > aug[pivot][i].abs() { pivot = k; }
        }
        if pivot != i { aug.swap(i, pivot); }
        let piv = aug[i][i];
        if piv.abs() < 1e-15 {
            // 奇异矩阵，返回单位阵
            let mut id = vec![vec![0.0_f64; n]; n];
            for k in 0..n { id[k][k] = 1.0; }
            return id;
        }
        for j in 0..2*n { aug[i][j] /= piv; }
        for k in 0..n {
            if k != i {
                let factor = aug[k][i];
                for j in 0..2*n { aug[k][j] -= factor * aug[i][j]; }
            }
        }
    }
    let mut inv = vec![vec![0.0_f64; n]; n];
    for i in 0..n {
        for j in 0..n { inv[i][j] = aug[i][n+j]; }
    }
    inv
}

/// 矩阵开方逆 S^(-1/2)（通过特征分解）
fn matrix_inv_sqrt(m: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = m.len();
    if n == 0 { return vec![]; }
    // 对称矩阵特征分解
    let (eigvals, eigvecs) = jacobi_eigen(m);
    // S^(-1/2) = V · diag(1/√λ) · V^T
    let mut inv_sqrt = vec![vec![0.0_f64; n]; n];
    for i in 0..n {
        for j in 0..n {
            let mut sum = 0.0;
            for k in 0..n {
                let lambda = eigvals[k].abs().max(1e-15);
                sum += eigvecs[i][k] * (1.0 / lambda.sqrt()) * eigvecs[j][k];
            }
            inv_sqrt[i][j] = sum;
        }
    }
    inv_sqrt
}

/// Jacobi 方法求对称矩阵特征值/特征向量
fn jacobi_eigen(matrix: &[Vec<f64>]) -> (Vec<f64>, Vec<Vec<f64>>) {
    let n = matrix.len();
    if n == 0 { return (vec![], vec![]); }
    let mut a: Vec<Vec<f64>> = matrix.iter().map(|r| r.clone()).collect();
    let mut v = vec![vec![0.0_f64; n]; n];
    for i in 0..n { v[i][i] = 1.0; }

    const MAX_ITER: usize = 100;
    const TOL: f64 = 1e-12;

    for _iter in 0..MAX_ITER {
        // 找最大非对角元
        let mut max_val = 0.0;
        let mut p = 0;
        let mut q = 1;
        for i in 0..n {
            for j in (i+1)..n {
                if a[i][j].abs() > max_val {
                    max_val = a[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        if max_val < TOL { break; }

        // Jacobi 旋转
        let theta = if (a[p][p] - a[q][q]).abs() < 1e-30 {
            std::f64::consts::FRAC_PI_4
        } else {
            0.5 * (2.0 * a[p][q] / (a[p][p] - a[q][q])).atan()
        };
        let c = theta.cos();
        let s = theta.sin();

        let app = a[p][p];
        let aqq = a[q][q];
        let apq = a[p][q];
        a[p][p] = c*c*app - 2.0*s*c*apq + s*s*aqq;
        a[q][q] = s*s*app + 2.0*s*c*apq + c*c*aqq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;
        for i in 0..n {
            if i != p && i != q {
                let aip = a[i][p];
                let aiq = a[i][q];
                a[i][p] = c*aip - s*aiq;
                a[p][i] = a[i][p];
                a[i][q] = s*aip + c*aiq;
                a[q][i] = a[i][q];
            }
        }
        // 更新特征向量
        for i in 0..n {
            let vip = v[i][p];
            let viq = v[i][q];
            v[i][p] = c*vip - s*viq;
            v[i][q] = s*vip + c*viq;
        }
    }

    let eigvals: Vec<f64> = (0..n).map(|i| a[i][i]).collect();
    (eigvals, v)
}
// ============================================================================
// 3.7 前线轨道理论
// ============================================================================

/// 前线轨道集合
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FrontierOrbitals {
    /// HOMO 能级 (eV)
    pub homo: f64,
    /// LUMO 能级 (eV)
    pub lumo: f64,
    /// HOMO-LUMO 能隙 (eV)
    pub gap: f64,
    /// 化学势 μ = (HOMO + LUMO) / 2
    pub chemical_potential: f64,
    /// 绝对硬度 η = (LUMO - HOMO) / 2
    pub hardness: f64,
    /// 绝对软度 S = 1 / (2η)
    pub softness: f64,
    /// 亲电性指数 ω = μ² / (2η)
    pub electrophilicity: f64,
    /// 电子逃逸能（电负性的 Mulliken 定义）χ = -μ
    pub electronegativity: f64,
}

impl FrontierOrbitals {
    /// 从 Hückel 求解器构建
    pub fn from_solver(solver: &HuckelSolver) -> Self {
        let homo = solver.homo_energy();
        let lumo = solver.lumo_energy();
        let gap = lumo - homo;
        let chemical_potential = (homo + lumo) / 2.0;
        let hardness = (lumo - homo) / 2.0;
        let softness = if hardness.abs() > 1e-10 { 1.0 / (2.0 * hardness) } else { f64::INFINITY };
        let electronegativity = -chemical_potential;
        let electrophilicity = if hardness.abs() > 1e-10 {
            chemical_potential.powi(2) / (2.0 * hardness)
        } else {
            f64::INFINITY
        };
        Self {
            homo, lumo, gap, chemical_potential, hardness, softness, electrophilicity, electronegativity,
        }
    }

    /// 从 HOMO/LUMO 直接构建
    pub fn new(homo: f64, lumo: f64) -> Self {
        let gap = lumo - homo;
        let chemical_potential = (homo + lumo) / 2.0;
        let hardness = (lumo - homo) / 2.0;
        let softness = if hardness.abs() > 1e-10 { 1.0 / (2.0 * hardness) } else { f64::INFINITY };
        let electronegativity = -chemical_potential;
        let electrophilicity = if hardness.abs() > 1e-10 {
            chemical_potential.powi(2) / (2.0 * hardness)
        } else {
            f64::INFINITY
        };
        Self { homo, lumo, gap, chemical_potential, hardness, softness, electrophilicity, electronegativity }
    }

    /// 是否为亲核试剂（HOMO 高）
    pub fn is_nucleophilic(&self) -> bool {
        // HOMO 越高（接近 0 或正值），越亲核
        self.homo > -8.0
    }

    /// 是否为亲电试剂（LUMO 低）
    pub fn is_electrophilic(&self) -> bool {
        // LUMO 越低（越负），越亲电
        self.lumo < -4.0
    }

    /// 是否为自由基（HOMO-LUMO 能隙小）
    pub fn is_radical_like(&self) -> bool {
        self.gap.abs() < 1.0
    }
}

// ============================================================================
// 3.8 简化 DFT 接口（占位，未来扩展）
// ============================================================================

/// DFT-like 计算接口
pub trait DftLike {
    /// 计算分子能量 (eV)
    fn energy(&self, mol: &Molecule) -> f64;
    /// 计算原子力 (eV/Å)
    fn forces(&self, mol: &Molecule) -> Vec<[f64; 3]>;
    /// 几何优化，返回最终能量
    fn optimize(&self, mol: &mut Molecule, max_iter: usize) -> f64;
}

/// 简化的 Hückel 能量计算器（实现 DftLike）
pub struct HuckelEnergy;

impl DftLike for HuckelEnergy {
    fn energy(&self, mol: &Molecule) -> f64 {
        let mut solver = HuckelSolver::new(mol);
        solver.solve();
        // 总能量 = 2 · Σ_occupied E_i
        let n = solver.eigenvalues.len();
        let n_occ = solver.electron_count / 2;
        let mut e = 0.0;
        for i in 0..n_occ.min(n) {
            e += 2.0 * solver.eigenvalues[i];
        }
        e
    }

    fn forces(&self, _mol: &Molecule) -> Vec<[f64; 3]> {
        // Hückel 方法不直接提供力
        _mol.atoms.iter().map(|_| [0.0, 0.0, 0.0]).collect()
    }

    fn optimize(&self, mol: &mut Molecule, _max_iter: usize) -> f64 {
        // Hückel 不做几何优化
        self.energy(mol)
    }
}

/// 计算分子的前线轨道
pub fn compute_frontier_orbitals(mol: &Molecule) -> FrontierOrbitals {
    let mut solver = HuckelSolver::new(mol);
    solver.solve();
    FrontierOrbitals::from_solver(&solver)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::molecules::BondOrder;
    use crate::elements::Element;
    use crate::molecules::Molecule;

    /// VSIP 查表验证
    #[test]
    fn test_vsip_lookup() {
        // H 1s = -13.6 eV
        assert!((vsip(Element::H, "1s") - (-13.6)).abs() < 1e-6);
        // C 2s = -21.4 eV
        assert!((vsip(Element::C, "2s") - (-21.4)).abs() < 1e-6);
        // C 2p = -11.4 eV
        assert!((vsip(Element::C, "2p") - (-11.4)).abs() < 1e-6);
        // O 2p = -15.9 eV
        assert!((vsip(Element::O, "2p") - (-15.9)).abs() < 1e-6);
        // F 2p = -18.7 eV
        assert!((vsip(Element::F, "2p") - (-18.7)).abs() < 1e-6);
        // 未知轨道返回默认值
        let default = vsip(Element::Og, "5g");
        assert!((default - (-10.0)).abs() < 1e-6);
    }

    /// Hückel 方法验证：H2 分子
    /// 2 个 H 原子，每个 1s 轨道，2 个电子
    /// 应得到 2 个轨道（成键 + 反键），2 个电子填充最低能级
    #[test]
    fn test_huckel_h2() {
        let mut h2 = Molecule::new();
        let h1 = h2.add_atom(Element::H);
        let h2_id = h2.add_atom(Element::H);
        h2.add_bond(h1, h2_id, BondOrder::Single);
        // 设置坐标
        h2.atoms[0].position = [0.0, 0.0, 0.0];
        h2.atoms[1].position = [0.74, 0.0, 0.0]; // 0.74 Å

        let mut solver = HuckelSolver::new(&h2);
        assert_eq!(solver.h_matrix.len(), 2, "H2 应有 2 个轨道");
        assert_eq!(solver.electron_count, 2, "H2 应有 2 个价电子");

        solver.solve();
        assert_eq!(solver.eigenvalues.len(), 2, "应有 2 个特征值");
        // 成键轨道能量 < -13.6（H 的 VSIP）
        let e_bond = solver.eigenvalues[0];
        let e_antibond = solver.eigenvalues[1];
        assert!(e_bond < -13.6, "成键轨道应低于 -13.6 eV: got {:.4}", e_bond);
        assert!(e_antibond > -13.6, "反键轨道应高于 -13.6 eV: got {:.4}", e_antibond);
        // HOMO-LUMO 能隙为正
        let gap = solver.homo_lumo_gap();
        assert!(gap > 0.0, "HOMO-LUMO 能隙应为正: got {:.4}", gap);
    }

    /// 苯分子 HOMO-LUMO 验证
    /// 苯 C6H6: 6 个 C，6 个 H，30 个价电子 (6·4 + 6·1)
    /// C6H6 共有 6·2 + 6·1 = 18 个轨道
    #[test]
    fn test_huckel_benzene() {
        let mut benzene = Molecule::new();
        // 6 个 C 原子（环状）
        let mut c_ids = vec![];
        for i in 0..6 {
            let angle = i as f64 * std::f64::consts::PI / 3.0;
            let c = benzene.add_atom(Element::C);
            benzene.atoms[c as usize].position = [1.40 * angle.cos(), 1.40 * angle.sin(), 0.0];
            c_ids.push(c);
        }
        // 6 个 H 原子
        for i in 0..6 {
            let angle = i as f64 * std::f64::consts::PI / 3.0;
            let h = benzene.add_atom(Element::H);
            benzene.atoms[h as usize].position = [2.49 * angle.cos(), 2.49 * angle.sin(), 0.0];
        }
        // 环状 C-C 键（芳香）
        for i in 0..6 {
            let j = (i + 1) % 6;
            benzene.add_bond(c_ids[i], c_ids[j], BondOrder::Aromatic);
        }
        // C-H 键
        for i in 0..6 {
            benzene.add_bond(c_ids[i], (i + 6) as u32, BondOrder::Single);
        }

        let mut solver = HuckelSolver::new(&benzene);
        solver.solve();

        // 苯应有 18 个轨道（6 C · 2 + 6 H · 1）
        assert_eq!(solver.h_matrix.len(), 18, "苯应有 18 个轨道");
        // 价电子数：6·4 + 6·1 = 30
        assert_eq!(solver.electron_count, 30, "苯应有 30 个价电子");
        // HOMO-LUMO 能隙应为正
        let gap = solver.homo_lumo_gap();
        assert!(gap > 0.0, "苯 HOMO-LUMO 能隙应为正: got {:.4}", gap);
        // 苯的能隙应较小（芳香性）
        assert!(gap < 20.0, "苯能隙应小于 20 eV: got {:.4}", gap);
    }

    /// 前线轨道理论验证
    #[test]
    fn test_frontier_orbitals() {
        // 直接构造：HOMO=-10, LUMO=-2
        let fmo = FrontierOrbitals::new(-10.0, -2.0);
        assert!((fmo.gap - 8.0).abs() < 1e-9, "能隙应为 8");
        assert!((fmo.chemical_potential - (-6.0)).abs() < 1e-9, "化学势应为 -6");
        assert!((fmo.hardness - 4.0).abs() < 1e-9, "硬度应为 4");
        assert!((fmo.softness - 0.125).abs() < 1e-9, "软度应为 0.125");
        // ω = μ²/(2η) = 36/8 = 4.5
        assert!((fmo.electrophilicity - 4.5).abs() < 1e-9, "亲电性应为 4.5");
        assert!((fmo.electronegativity - 6.0).abs() < 1e-9, "电负性应为 6");
    }

    /// 亲核/亲电判断验证
    #[test]
    fn test_nucleophilic_electrophilic() {
        // 高 HOMO → 亲核
        let nucleophile = FrontierOrbitals::new(-5.0, 0.0);
        assert!(nucleophile.is_nucleophilic(), "HOMO=-5 应为亲核");
        // 低 LUMO → 亲电
        let electrophile = FrontierOrbitals::new(-15.0, -8.0);
        assert!(electrophile.is_electrophilic(), "LUMO=-8 应为亲电");
    }

    /// Slater 重叠积分验证
    #[test]
    fn test_slater_overlap() {
        // 同一位置 r=0 → S=1
        let s0 = slater_overlap(1, 1.0, 1, 1.0, 0.0);
        assert!((s0 - 1.0).abs() < 1e-9, "r=0 时 S 应为 1");
        // 远距离 → S → 0
        let s_far = slater_overlap(1, 1.0, 1, 1.0, 20.0);
        assert!(s_far < 0.1, "远距离 S 应接近 0: got {:.4}", s_far);
        // 中等距离 → 0 < S < 1
        let s_mid = slater_overlap(1, 1.0, 1, 1.0, 2.0);
        assert!(s_mid > 0.0 && s_mid < 1.0, "中等距离 S 应在 (0,1): got {:.4}", s_mid);
    }

    /// 矩阵运算验证
    #[test]
    fn test_matrix_inverse() {
        // 2x2 矩阵 [[4,7],[2,6]] 逆 = [[6,-7],[-2,4]]/10
        let m = vec![vec![4.0, 7.0], vec![2.0, 6.0]];
        let inv = matrix_inverse(&m);
        // 验证 M·M⁻¹ = I
        let prod = mat_mul(&m, &inv);
        assert!((prod[0][0] - 1.0).abs() < 1e-9, "M00 应为 1");
        assert!((prod[1][1] - 1.0).abs() < 1e-9, "M11 应为 1");
        assert!(prod[0][1].abs() < 1e-9, "M01 应为 0");
        assert!(prod[1][0].abs() < 1e-9, "M10 应为 0");
    }

    /// Jacobi 特征值验证
    #[test]
    fn test_jacobi_eigen() {
        // 对角矩阵，特征值就是对角元
        let m = vec![vec![1.0, 0.0], vec![0.0, 3.0]];
        let (eigvals, _) = jacobi_eigen(&m);
        let mut sorted = eigvals.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((sorted[0] - 1.0).abs() < 1e-9, "最小特征值应为 1");
        assert!((sorted[1] - 3.0).abs() < 1e-9, "最大特征值应为 3");
    }

    /// DftLike 接口验证（Hückel 能量计算）
    #[test]
    fn test_huckel_energy_dft() {
        let mut h2 = Molecule::new();
        let h1 = h2.add_atom(Element::H);
        let h2_id = h2.add_atom(Element::H);
        h2.add_bond(h1, h2_id, BondOrder::Single);
        h2.atoms[0].position = [0.0, 0.0, 0.0];
        h2.atoms[1].position = [0.74, 0.0, 0.0];

        let calc = HuckelEnergy;
        let e = calc.energy(&h2);
        // H2 总能量应低于 2·(-13.6) = -27.2（成键稳定化）
        assert!(e < -27.2, "H2 成键应降低能量: got {:.4}", e);
    }
}