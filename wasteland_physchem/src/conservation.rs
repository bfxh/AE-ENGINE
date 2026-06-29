//! conservation.rs - 物理化学守恒律执行器
//!
//! 核心理念：**绝对守恒**。每个物理/化学操作前后强制验证守恒律。
//! - 质量守恒（原子计数，整数精确）
//! - 能量守恒（热力学第一定律）
//! - 动量守恒（牛顿第三定律）
//! - 角动量守恒
//! - 电荷守恒
//! - 原子计数守恒（每元素原子数，整数精确）
//! - 重子数守恒（核反应）
//! - 轻子数守恒（核反应/β衰变）
//!
//! 设计原则：
//! 1. f64 全精度，不用 f32
//! 2. 原子计数用 u64 整数，精确匹配
//! 3. 浮点量用相对容差 + 绝对容差双重判定
//! 4. 可扩展：用户可注册自定义守恒律
//! 5. 不阉割：strict 模式容差 1e-12

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// 元素原子序数到计数的映射（z -> atom count）
pub type AtomCounts = HashMap<u8, u64>;

/// 守恒状态：系统在某一时刻所有守恒量的快照
///
/// 每个物理/化学操作前后都应生成快照，由 ConservationChecker 验证。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConservationState {
    /// 总质量 (kg) - 由原子计数和元素原子量精确求和
    pub total_mass_kg: f64,
    /// 总能量 (J) - 动能+势能+化学能+热能+辐射能+核能
    pub total_energy_j: f64,
    /// 线动量矢量 (kg·m/s)
    pub momentum: [f64; 3],
    /// 角动量矢量 (kg·m²/s)
    pub angular_momentum: [f64; 3],
    /// 总电荷 (C)
    pub total_charge_c: f64,
    /// 每元素原子数（z -> count）- 整数精确守恒
    pub atom_counts: AtomCounts,
    /// 重子数（核子总数，质子+中子）- 核反应守恒
    pub baryon_number: i64,
    /// 轻子数（电子-正电子+中微子-反中微子）- β衰变守恒
    pub lepton_number: i64,
    /// 自定义扩展守恒量（供未来扩展，如奇异数、同位旋）
    pub custom: HashMap<String, f64>,
}

impl Default for ConservationState {
    fn default() -> Self {
        Self {
            total_mass_kg: 0.0,
            total_energy_j: 0.0,
            momentum: [0.0; 3],
            angular_momentum: [0.0; 3],
            total_charge_c: 0.0,
            atom_counts: HashMap::new(),
            baryon_number: 0,
            lepton_number: 0,
            custom: HashMap::new(),
        }
    }
}

impl ConservationState {
    pub fn new() -> Self {
        Self::default()
    }

    /// 从原子计数构建（质量自动从元素原子量求和）
    pub fn from_atom_counts(counts: &AtomCounts) -> Self {
        let mut s = Self::default();
        for (&z, &count) in counts {
            s.atom_counts.insert(z, count);
            if let Some(elem) = crate::elements::Element::from_atomic_number(z) {
                let mass_kg = elem.atomic_mass() * 1.66053906660e-27 * count as f64;
                s.total_mass_kg += mass_kg;
                // 电荷：质子数 = 原子序数 × 原子数
                s.total_charge_c += z as f64 * count as f64 * 1.602176634e-19;
                // 重子数 = 质量数（近似用 atomic_mass 四舍五入）
                let mass_number = elem.atomic_mass().round() as i64;
                s.baryon_number += mass_number * count as i64;
                // 轻子数 = -电子数（中性原子：电子数=质子数，轻子数=-质子数）
                s.lepton_number -= z as i64 * count as i64;
            }
        }
        s
    }

    /// 合并两个状态（如反应物 A + B）
    pub fn merge(&self, other: &Self) -> Self {
        let mut merged = self.clone();
        merged.total_mass_kg += other.total_mass_kg;
        merged.total_energy_j += other.total_energy_j;
        for i in 0..3 {
            merged.momentum[i] += other.momentum[i];
            merged.angular_momentum[i] += other.angular_momentum[i];
        }
        merged.total_charge_c += other.total_charge_c;
        for (&z, &c) in &other.atom_counts {
            *merged.atom_counts.entry(z).or_insert(0) += c;
        }
        merged.baryon_number += other.baryon_number;
        merged.lepton_number += other.lepton_number;
        for (k, &v) in &other.custom {
            *merged.custom.entry(k.clone()).or_insert(0.0) += v;
        }
        merged
    }

    /// 减去另一个状态（如生成物 - 反应物，用于检查守恒）
    pub fn diff(&self, other: &Self) -> Self {
        let mut d = self.clone();
        d.total_mass_kg -= other.total_mass_kg;
        d.total_energy_j -= other.total_energy_j;
        for i in 0..3 {
            d.momentum[i] -= other.momentum[i];
            d.angular_momentum[i] -= other.angular_momentum[i];
        }
        d.total_charge_c -= other.total_charge_c;
        for (&z, &c) in &other.atom_counts {
            let entry = d.atom_counts.entry(z).or_insert(0);
            *entry = (*entry as i64 - c as i64) as u64; // 保留差值（可能下溢，用 i64 解释）
        }
        d.baryon_number -= other.baryon_number;
        d.lepton_number -= other.lepton_number;
        d
    }

    /// 添加一个原子
    pub fn add_atom(&mut self, z: u8, count: u64) {
        *self.atom_counts.entry(z).or_insert(0) += count;
        if let Some(elem) = crate::elements::Element::from_atomic_number(z) {
            self.total_mass_kg += elem.atomic_mass() * 1.66053906660e-27 * count as f64;
            self.total_charge_c += z as f64 * count as f64 * 1.602176634e-19;
            self.baryon_number += elem.atomic_mass().round() as i64 * count as i64;
            self.lepton_number -= z as i64 * count as i64;
        }
    }

    /// 设置动能 (J)
    pub fn with_kinetic_energy(mut self, ke: f64) -> Self {
        self.total_energy_j += ke;
        self
    }

    /// 设置动量 (kg·m/s)
    pub fn with_momentum(mut self, p: [f64; 3]) -> Self {
        self.momentum = p;
        self
    }

    /// 检查是否为零状态（所有量都为零）
    pub fn is_zero(&self) -> bool {
        self.total_mass_kg.abs() < 1e-30
            && self.total_energy_j.abs() < 1e-30
            && self.momentum.iter().all(|&v| v.abs() < 1e-30)
            && self.angular_momentum.iter().all(|&v| v.abs() < 1e-30)
            && self.total_charge_c.abs() < 1e-30
            && self.atom_counts.values().all(|&c| c == 0)
            && self.baryon_number == 0
            && self.lepton_number == 0
    }
}

/// 守恒违反严重度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationSeverity {
    /// 数值精度范围内（相对误差 < 1e-12）
    Info,
    /// 小幅偏差（相对误差 1e-12 ~ 1e-9）
    Warning,
    /// 明显违反（相对误差 1e-9 ~ 1e-3）
    Error,
    /// 严重违反（相对误差 > 1e-3 或整数不匹配）
    Critical,
}

/// 单个守恒违反
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConservationViolation {
    /// 违反的量名称
    pub quantity: String,
    /// 操作前的值
    pub before: f64,
    /// 操作后的值
    pub after: f64,
    /// 绝对误差 |after - before|
    pub absolute_error: f64,
    /// 相对误差 |after - before| / max(|before|, |after|, 1)
    pub relative_error: f64,
    /// 容差
    pub tolerance: f64,
    /// 严重度
    pub severity: ViolationSeverity,
}

impl ConservationViolation {
    pub fn new(quantity: &str, before: f64, after: f64, tolerance: f64) -> Self {
        let abs_err = (after - before).abs();
        let scale = before.abs().max(after.abs()).max(1.0);
        let rel_err = abs_err / scale;
        let severity = if abs_err <= tolerance {
            ViolationSeverity::Info
        } else if rel_err < 1e-9 {
            ViolationSeverity::Warning
        } else if rel_err < 1e-3 {
            ViolationSeverity::Error
        } else {
            ViolationSeverity::Critical
        };
        Self {
            quantity: quantity.to_string(),
            before,
            after,
            absolute_error: abs_err,
            relative_error: rel_err,
            tolerance,
            severity,
        }
    }

    pub fn is_violation(&self) -> bool {
        !matches!(self.severity, ViolationSeverity::Info)
    }
}

/// 单个守恒律的检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConservationResult {
    /// 守恒律名称
    pub law_name: String,
    /// 是否通过（无违反）
    pub passed: bool,
    /// 违反列表（Info 级别的不算违反）
    pub violations: Vec<ConservationViolation>,
}

impl ConservationResult {
    pub fn passed(law_name: &str) -> Self {
        Self {
            law_name: law_name.to_string(),
            passed: true,
            violations: vec![],
        }
    }

    pub fn failed(law_name: &str, violations: Vec<ConservationViolation>) -> Self {
        let real_violations: Vec<_> = violations.into_iter().filter(|v| v.is_violation()).collect();
        let passed = real_violations.is_empty();
        Self {
            law_name: law_name.to_string(),
            passed,
            violations: real_violations,
        }
    }
}

/// 守恒律 trait - 可扩展接口
pub trait ConservationLaw: Send + Sync {
    /// 守恒律名称
    fn name(&self) -> &'static str;
    /// 守恒律描述
    fn description(&self) -> &'static str;
    /// 检查 before → after 是否守恒
    fn check(&self, before: &ConservationState, after: &ConservationState, tolerance: f64) -> ConservationResult;
}

/// 质量守恒（总质量）
pub struct MassConservation;

impl ConservationLaw for MassConservation {
    fn name(&self) -> &'static str { "MassConservation" }
    fn description(&self) -> &'static str { "质量守恒：总质量在孤立系统中保持不变" }
    fn check(&self, before: &ConservationState, after: &ConservationState, tolerance: f64) -> ConservationResult {
        let v = ConservationViolation::new("total_mass_kg", before.total_mass_kg, after.total_mass_kg, tolerance);
        if v.is_violation() {
            ConservationResult::failed(self.name(), vec![v])
        } else {
            ConservationResult::passed(self.name())
        }
    }
}

/// 能量守恒（热力学第一定律）
pub struct EnergyConservation;

impl ConservationLaw for EnergyConservation {
    fn name(&self) -> &'static str { "EnergyConservation" }
    fn description(&self) -> &'static str { "能量守恒：总能量（动能+势能+化学能+热能+辐射能+核能）保持不变" }
    fn check(&self, before: &ConservationState, after: &ConservationState, tolerance: f64) -> ConservationResult {
        let v = ConservationViolation::new("total_energy_j", before.total_energy_j, after.total_energy_j, tolerance);
        if v.is_violation() {
            ConservationResult::failed(self.name(), vec![v])
        } else {
            ConservationResult::passed(self.name())
        }
    }
}

/// 动量守恒（牛顿第三定律）
pub struct MomentumConservation;

impl ConservationLaw for MomentumConservation {
    fn name(&self) -> &'static str { "MomentumConservation" }
    fn description(&self) -> &'static str { "动量守恒：线动量矢量在无外力时保持不变" }
    fn check(&self, before: &ConservationState, after: &ConservationState, tolerance: f64) -> ConservationResult {
        let mut violations = vec![];
        for (i, axis) in ["px", "py", "pz"].iter().enumerate() {
            let v = ConservationViolation::new(axis, before.momentum[i], after.momentum[i], tolerance);
            if v.is_violation() {
                violations.push(v);
            }
        }
        if violations.is_empty() {
            ConservationResult::passed(self.name())
        } else {
            ConservationResult::failed(self.name(), violations)
        }
    }
}

/// 角动量守恒
pub struct AngularMomentumConservation;

impl ConservationLaw for AngularMomentumConservation {
    fn name(&self) -> &'static str { "AngularMomentumConservation" }
    fn description(&self) -> &'static str { "角动量守恒：角动量矢量在无外力矩时保持不变" }
    fn check(&self, before: &ConservationState, after: &ConservationState, tolerance: f64) -> ConservationResult {
        let mut violations = vec![];
        for (i, axis) in ["Lx", "Ly", "Lz"].iter().enumerate() {
            let v = ConservationViolation::new(axis, before.angular_momentum[i], after.angular_momentum[i], tolerance);
            if v.is_violation() {
                violations.push(v);
            }
        }
        if violations.is_empty() {
            ConservationResult::passed(self.name())
        } else {
            ConservationResult::failed(self.name(), violations)
        }
    }
}

/// 电荷守恒
pub struct ChargeConservation;

impl ConservationLaw for ChargeConservation {
    fn name(&self) -> &'static str { "ChargeConservation" }
    fn description(&self) -> &'static str { "电荷守恒：总电荷在孤立系统中保持不变" }
    fn check(&self, before: &ConservationState, after: &ConservationState, tolerance: f64) -> ConservationResult {
        let v = ConservationViolation::new("total_charge_c", before.total_charge_c, after.total_charge_c, tolerance);
        if v.is_violation() {
            ConservationResult::failed(self.name(), vec![v])
        } else {
            ConservationResult::passed(self.name())
        }
    }
}

/// 原子计数守恒（每元素原子数精确匹配）
pub struct AtomCountConservation;

impl AtomCountConservation {
    /// 收集所有出现过的元素 z
    fn all_elements(before: &ConservationState, after: &ConservationState) -> Vec<u8> {
        let mut zs: Vec<u8> = before.atom_counts.keys().copied().collect();
        for &z in after.atom_counts.keys() {
            if !zs.contains(&z) {
                zs.push(z);
            }
        }
        zs.sort_unstable();
        zs
    }
}

impl ConservationLaw for AtomCountConservation {
    fn name(&self) -> &'static str { "AtomCountConservation" }
    fn description(&self) -> &'static str { "原子计数守恒：每种元素的原子数在化学反应中精确不变（整数匹配）" }
    fn check(&self, before: &ConservationState, after: &ConservationState, _tolerance: f64) -> ConservationResult {
        let mut violations = vec![];
        for z in Self::all_elements(before, after) {
            let before_count = before.atom_counts.get(&z).copied().unwrap_or(0) as f64;
            let after_count = after.atom_counts.get(&z).copied().unwrap_or(0) as f64;
            if before_count as u64 != after_count as u64 {
                let elem_name = crate::elements::Element::from_atomic_number(z)
                    .map(|e| e.symbol().to_string())
                    .unwrap_or_else(|| format!("Z{}", z));
                let v = ConservationViolation::new(&format!("atoms_{}", elem_name), before_count, after_count, 0.0);
                violations.push(v);
            }
        }
        if violations.is_empty() {
            ConservationResult::passed(self.name())
        } else {
            ConservationResult::failed(self.name(), violations)
        }
    }
}

/// 重子数守恒（核反应）
pub struct BaryonNumberConservation;

impl ConservationLaw for BaryonNumberConservation {
    fn name(&self) -> &'static str { "BaryonNumberConservation" }
    fn description(&self) -> &'static str { "重子数守恒：核子总数（质子+中子）在核反应中保持不变" }
    fn check(&self, before: &ConservationState, after: &ConservationState, _tolerance: f64) -> ConservationResult {
        if before.baryon_number != after.baryon_number {
            let v = ConservationViolation::new("baryon_number", before.baryon_number as f64, after.baryon_number as f64, 0.0);
            ConservationResult::failed(self.name(), vec![v])
        } else {
            ConservationResult::passed(self.name())
        }
    }
}

/// 轻子数守恒（β衰变）
pub struct LeptonNumberConservation;

impl ConservationLaw for LeptonNumberConservation {
    fn name(&self) -> &'static str { "LeptonNumberConservation" }
    fn description(&self) -> &'static str { "轻子数守恒：轻子数（电子-正电子+中微子）在弱相互作用中保持不变" }
    fn check(&self, before: &ConservationState, after: &ConservationState, _tolerance: f64) -> ConservationResult {
        if before.lepton_number != after.lepton_number {
            let v = ConservationViolation::new("lepton_number", before.lepton_number as f64, after.lepton_number as f64, 0.0);
            ConservationResult::failed(self.name(), vec![v])
        } else {
            ConservationResult::passed(self.name())
        }
    }
}

/// 守恒检查器 - 聚合所有守恒律
pub struct ConservationChecker {
    laws: Vec<Arc<dyn ConservationLaw>>,
    /// 相对容差（用于浮点量）
    tolerance_relative: f64,
    /// 绝对容差（用于接近零的情况）
    tolerance_absolute: f64,
}

impl Default for ConservationChecker {
    fn default() -> Self {
        Self::standard()
    }
}

impl ConservationChecker {
    /// 严格模式：1e-12 相对容差（用于高精度科学计算）
    pub fn strict() -> Self {
        Self {
            laws: default_laws(),
            tolerance_relative: 1e-12,
            tolerance_absolute: 1e-15,
        }
    }

    /// 标准模式：1e-9 相对容差（默认，平衡精度和数值稳定性）
    pub fn standard() -> Self {
        Self {
            laws: default_laws(),
            tolerance_relative: 1e-9,
            tolerance_absolute: 1e-12,
        }
    }

    /// 宽松模式：1e-6 相对容差（用于游戏级模拟，允许浮点累积误差）
    pub fn lenient() -> Self {
        Self {
            laws: default_laws(),
            tolerance_relative: 1e-6,
            tolerance_absolute: 1e-9,
        }
    }

    /// 注册自定义守恒律（可扩展）
    pub fn register_law(&mut self, law: Arc<dyn ConservationLaw>) {
        self.laws.push(law);
    }

    /// 获取容差（取相对和绝对的最大值）
    pub fn tolerance_for(&self, scale: f64) -> f64 {
        self.tolerance_relative * scale.abs().max(1.0) + self.tolerance_absolute
    }

    /// 检查所有守恒律
    pub fn check(&self, before: &ConservationState, after: &ConservationState) -> Vec<ConservationResult> {
        let scale = before.total_energy_j.abs()
            .max(after.total_energy_j.abs())
            .max(before.total_mass_kg.abs())
            .max(after.total_mass_kg.abs())
            .max(1.0);
        let tol = self.tolerance_for(scale);
        self.laws.iter().map(|law| law.check(before, after, tol)).collect()
    }

    /// 断言守恒：返回 Err 包含所有违反，或 Ok
    pub fn assert_conserved(&self, before: &ConservationState, after: &ConservationState) -> Result<(), Vec<ConservationViolation>> {
        let results = self.check(before, after);
        let mut all_violations = vec![];
        for r in results {
            if !r.passed {
                all_violations.extend(r.violations);
            }
        }
        if all_violations.is_empty() {
            Ok(())
        } else {
            Err(all_violations)
        }
    }

    /// 生成人类可读的守恒报告
    pub fn report(&self, before: &ConservationState, after: &ConservationState) -> String {
        let results = self.check(before, after);
        let mut report = String::new();
        report.push_str("=== Conservation Report ===\n");
        let all_passed = results.iter().all(|r| r.passed);
        if all_passed {
            report.push_str("✓ ALL CONSERVATION LAWS PASSED\n");
        } else {
            report.push_str("✗ CONSERVATION VIOLATIONS DETECTED\n");
        }
        for r in &results {
            let status = if r.passed { "✓" } else { "✗" };
            report.push_str(&format!("{} {}: {}\n", status, r.law_name, if r.passed { "PASS" } else { "FAIL" }));
            for v in &r.violations {
                report.push_str(&format!(
                    "    {} {} → {} (err={:.3e}, rel={:.3e}, sev={:?})\n",
                    v.quantity, v.before, v.after, v.absolute_error, v.relative_error, v.severity
                ));
            }
        }
        report
    }

    /// 获取已注册的守恒律数量
    pub fn law_count(&self) -> usize {
        self.laws.len()
    }
}

/// 默认守恒律集合
fn default_laws() -> Vec<Arc<dyn ConservationLaw>> {
    vec![
        Arc::new(MassConservation),
        Arc::new(EnergyConservation),
        Arc::new(MomentumConservation),
        Arc::new(AngularMomentumConservation),
        Arc::new(ChargeConservation),
        Arc::new(AtomCountConservation),
        Arc::new(BaryonNumberConservation),
        Arc::new(LeptonNumberConservation),
    ]
}

/// 守恒日志 - 记录每次操作前后的状态，用于审计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConservationLog {
    pub entries: Vec<ConservationLogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConservationLogEntry {
    pub timestamp: f64,
    pub operation: String,
    pub before: ConservationState,
    pub after: ConservationState,
    pub results: Vec<ConservationResult>,
    pub all_passed: bool,
}

impl Default for ConservationLog {
    fn default() -> Self {
        Self { entries: vec![] }
    }
}

impl ConservationLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一次操作
    pub fn record(
        &mut self,
        checker: &ConservationChecker,
        timestamp: f64,
        operation: &str,
        before: ConservationState,
        after: ConservationState,
    ) -> bool {
        let results = checker.check(&before, &after);
        let all_passed = results.iter().all(|r| r.passed);
        self.entries.push(ConservationLogEntry {
            timestamp,
            operation: operation.to_string(),
            before,
            after,
            results,
            all_passed,
        });
        all_passed
    }

    /// 获取所有违反记录
    pub fn violations(&self) -> Vec<&ConservationViolation> {
        self.entries.iter()
            .flat_map(|e| e.results.iter())
            .flat_map(|r| r.violations.iter())
            .collect()
    }

    /// 通过率
    pub fn pass_rate(&self) -> f64 {
        if self.entries.is_empty() {
            return 1.0;
        }
        let passed = self.entries.iter().filter(|e| e.all_passed).count();
        passed as f64 / self.entries.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_merge_roundtrip() {
        let mut a = ConservationState::new();
        a.add_atom(1, 4); // 4 H
        a.add_atom(8, 2); // 2 O
        // 4H + 2O = 2 H2O
        assert!(a.total_mass_kg > 0.0);
        assert_eq!(a.atom_counts.get(&1), Some(&4));
        assert_eq!(a.atom_counts.get(&8), Some(&2));
    }

    #[test]
    fn test_mass_conservation_pass() {
        let mut before = ConservationState::new();
        before.add_atom(1, 4);
        before.add_atom(8, 2);
        let after = before.clone();
        // 质量应完全相同
        let checker = ConservationChecker::strict();
        let results = checker.check(&before, &after);
        assert!(results.iter().all(|r| r.passed));
    }

    #[test]
    fn test_atom_count_conservation_detects_violation() {
        let mut before = ConservationState::new();
        before.add_atom(1, 4); // 4 H
        before.add_atom(8, 2); // 2 O

        let mut after = ConservationState::new();
        after.add_atom(1, 4); // 4 H - 保持
        after.add_atom(8, 1); // 1 O - 少了一个！违反

        let checker = ConservationChecker::strict();
        let results = checker.check(&before, &after);
        let atom_result = results.iter().find(|r| r.law_name == "AtomCountConservation").unwrap();
        assert!(!atom_result.passed, "应该检测到 O 原子数违反");
    }

    #[test]
    fn test_charge_conservation() {
        let mut before = ConservationState::new();
        before.add_atom(1, 2); // 2 H
        before.add_atom(8, 1); // 1 O
        // 中性水分子

        let mut after = before.clone();
        after.total_charge_c = 1.0; // 引入 1 库仑电荷（远超容差）
        let checker = ConservationChecker::standard();
        let results = checker.check(&before, &after);
        let charge_result = results.iter().find(|r| r.law_name == "ChargeConservation").unwrap();
        assert!(!charge_result.passed, "应该检测到电荷违反");
    }

    #[test]
    fn test_energy_conservation_within_tolerance() {
        let before = ConservationState {
            total_energy_j: 1.0e6,
            ..Default::default()
        };
        let after = ConservationState {
            total_energy_j: 1.0e6 + 1.0e-6, // 微小误差
            ..Default::default()
        };
        let checker = ConservationChecker::lenient();
        let results = checker.check(&before, &after);
        let energy_result = results.iter().find(|r| r.law_name == "EnergyConservation").unwrap();
        assert!(energy_result.passed, "微小误差应在容差内通过");
    }

    #[test]
    fn test_momentum_vector_conservation() {
        let before = ConservationState {
            momentum: [1.0, -2.0, 3.0],
            ..Default::default()
        };
        let after = ConservationState {
            momentum: [1.0, -2.0, 3.0],
            ..Default::default()
        };
        let checker = ConservationChecker::strict();
        let results = checker.check(&before, &after);
        let mom_result = results.iter().find(|r| r.law_name == "MomentumConservation").unwrap();
        assert!(mom_result.passed);
    }

    #[test]
    fn test_baryon_number_conservation() {
        let mut before = ConservationState::new();
        before.add_atom(1, 1); // 1 H
        let after = before.clone();
        let checker = ConservationChecker::strict();
        let results = checker.check(&before, &after);
        let baryon_result = results.iter().find(|r| r.law_name == "BaryonNumberConservation").unwrap();
        assert!(baryon_result.passed);
    }

    #[test]
    fn test_conservation_log() {
        let mut log = ConservationLog::new();
        let checker = ConservationChecker::standard();
        let state = ConservationState::new();
        let passed = log.record(&checker, 0.0, "no-op", state.clone(), state);
        assert!(passed);
        assert_eq!(log.pass_rate(), 1.0);
    }

    #[test]
    fn test_report_generation() {
        let before = ConservationState::new();
        let after = ConservationState::new();
        let checker = ConservationChecker::standard();
        let report = checker.report(&before, &after);
        assert!(report.contains("Conservation Report"));
        assert!(report.contains("PASSED") || report.contains("VIOLATIONS"));
    }

    #[test]
    fn test_custom_law_registration() {
        struct CustomLaw;
        impl ConservationLaw for CustomLaw {
            fn name(&self) -> &'static str { "Custom" }
            fn description(&self) -> &'static str { "test" }
            fn check(&self, _before: &ConservationState, _after: &ConservationState, _t: f64) -> ConservationResult {
                ConservationResult::passed("Custom")
            }
        }
        let mut checker = ConservationChecker::standard();
        checker.register_law(Arc::new(CustomLaw));
        assert_eq!(checker.law_count(), 9); // 8 default + 1 custom
    }

    #[test]
    fn test_is_zero_state() {
        let s = ConservationState::new();
        assert!(s.is_zero());
    }
}
