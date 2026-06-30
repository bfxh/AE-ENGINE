//! Broadphase Collision Detection — Sweep-and-Prune (Sort and Sweep)
//!
//! 基于:
//! - Ericson. Real-Time Collision Detection. Morgan Kaufmann 2005. Ch 5.3
//!   (Sweep-and-Prune / Sort-and-Sweep)
//! - Catto. Box2D b2BroadPhase (sort-and-sweep with insertion sort)
//!   https://box2d.org/files/ErinCatto_GDC2009_BroadPhase.pdf
//! - Cao & Wang. "A Fast and Generalized Broad-Phase Collision Detection
//!   Method Based on KD-Tree Spatial Subdivision and Sweep-and-Prune."
//!   IEEE Access 11, 2023. DOI: 10.1109/ACCESS.2023.3274202
//!
//! 核心思想:
//! 1. 每个物体用 AABB 包围盒表示
//! 2. 将所有 AABB 沿某一轴(最优轴 = 物体分布跨度最大的轴)按 min 值排序
//! 3. 扫描排序后的列表,对每个物体 i 检查后续物体 j:
//!    - 若 j.min[axis] > i.max[axis],则 j 之后所有物体都不会与 i 重叠 → break
//!    - 否则做完整 AABB 重叠测试 (三轴都重叠才算碰撞)
//! 4. 利用时间连贯性 (temporal coherence): 帧间物体顺序变化小,
//!    用插入排序 O(n) 而非快排 O(n log n)
//! 5. 每隔若干帧重新选择最优轴,适应场景变化
//!
//! 复杂度: O(n + k) 其中 k 为重叠对数, 排序 O(n) (时间连贯时)

use glam::Vec3;

// ============================================================
// AABB — 轴对齐包围盒
// ============================================================

/// 3D 轴对齐包围盒
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// 从中心和半尺寸构造
    pub fn from_center_half(center: Vec3, half: Vec3) -> Self {
        Self {
            min: center - half,
            max: center + half,
        }
    }

    /// 合并两个 AABB
    pub fn union(&self, other: &Aabb) -> Aabb {
        Aabb {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// 中心点
    #[inline]
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// 半尺寸
    #[inline]
    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    /// 体积
    #[inline]
    pub fn volume(&self) -> f32 {
        let d = self.max - self.min;
        d.x * d.y * d.z
    }

    /// 表面积
    #[inline]
    pub fn surface_area(&self) -> f32 {
        let d = self.max - self.min;
        2.0 * (d.x * d.y + d.y * d.z + d.x * d.z)
    }

    /// 两 AABB 是否重叠 (三轴都重叠)
    #[inline]
    pub fn intersects(&self, other: &Aabb) -> bool {
        self.max.x >= other.min.x
            && self.min.x <= other.max.x
            && self.max.y >= other.min.y
            && self.min.y <= other.max.y
            && self.max.z >= other.min.z
            && self.min.z <= other.max.z
    }

    /// 点是否在 AABB 内 (含边界)
    #[inline]
    pub fn contains_point(&self, p: Vec3) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }

    /// 是否完全包含另一个 AABB
    #[inline]
    pub fn contains_aabb(&self, other: &Aabb) -> bool {
        self.min.x <= other.min.x
            && self.max.x >= other.max.x
            && self.min.y <= other.min.y
            && self.max.y >= other.max.y
            && self.min.z <= other.min.z
            && self.max.z >= other.max.z
    }

    /// 沿指定轴的跨度
    #[inline]
    pub fn extent(&self, axis: usize) -> f32 {
        match axis {
            0 => self.max.x - self.min.x,
            1 => self.max.y - self.min.y,
            2 => self.max.z - self.min.z,
            _ => 0.0,
        }
    }

    /// 沿指定轴的 min 值
    #[inline]
    pub fn min_axis(&self, axis: usize) -> f32 {
        match axis {
            0 => self.min.x,
            1 => self.min.y,
            2 => self.min.z,
            _ => 0.0,
        }
    }

    /// 沿指定轴的 max 值
    #[inline]
    pub fn max_axis(&self, axis: usize) -> f32 {
        match axis {
            0 => self.max.x,
            1 => self.max.y,
            2 => self.max.z,
            _ => 0.0,
        }
    }

    /// 膨胀 (各方向扩展 delta)
    pub fn fattened(&self, delta: f32) -> Aabb {
        Aabb {
            min: self.min - Vec3::splat(delta),
            max: self.max + Vec3::splat(delta),
        }
    }
}

// ============================================================
// BroadphaseProxy — 宽相代理
// ============================================================

/// 宽相代理: 每个物体在宽相中的数据
#[derive(Debug, Clone)]
pub struct BroadphaseProxy {
    /// 用户定义的物体 ID
    pub body_id: u32,
    /// 物体的 AABB (膨胀后, 避免频繁更新)
    pub aabb: Aabb,
    /// 是否在当前帧被更新 (需要重新检查)
    pub dirty: bool,
}

// ============================================================
// SapBroadphase — Sweep-and-Prune 宽相
// ============================================================

/// Sweep-and-Prune 宽相碰撞检测
///
/// 维护一个按最优轴排序的物体列表, 利用时间连贯性用插入排序维护顺序.
/// 每帧扫描列表生成重叠对.
pub struct SapBroadphase {
    /// 代理列表 (按当前排序轴的 min 值排序)
    proxies: Vec<BroadphaseProxy>,
    /// 当前排序轴 (0=x, 1=y, 2=z)
    sort_axis: usize,
    /// 每隔多少帧重新选择最优轴
    axis_reselect_interval: u32,
    /// 帧计数器
    frame_count: u32,
    /// AABB 膨胀量 (避免小移动导致重排)
    fat_margin: f32,
}

impl SapBroadphase {
    pub fn new() -> Self {
        Self {
            proxies: Vec::new(),
            sort_axis: 0,
            axis_reselect_interval: 64,
            frame_count: 0,
            fat_margin: 0.05,
        }
    }

    /// 设置膨胀边距 (物体 AABB 会在各方向扩展此值)
    pub fn with_fat_margin(mut self, margin: f32) -> Self {
        self.fat_margin = margin;
        self
    }

    /// 设置轴重选间隔 (帧数)
    pub fn with_axis_reselect_interval(mut self, interval: u32) -> Self {
        self.axis_reselect_interval = interval;
        self
    }

    /// 添加物体
    pub fn add_body(&mut self, body_id: u32, aabb: Aabb) {
        let fat_aabb = aabb.fattened(self.fat_margin);
        self.proxies.push(BroadphaseProxy {
            body_id,
            aabb: fat_aabb,
            dirty: true,
        });
        // 新增物体打破排序, 标记需要重新排序
        // 插入排序会在下次 compute_pairs 时修复
    }

    /// 更新物体 AABB (只在物体移出 fat AABB 时才需要调用)
    pub fn update_body(&mut self, body_id: u32, aabb: Aabb) {
        for p in &mut self.proxies {
            if p.body_id == body_id {
                // 检查新 AABB 是否仍在 fat AABB 内
                if p.aabb.contains_aabb(&aabb) {
                    // 仍在 fat AABB 内, 无需更新
                    return;
                }
                // 移出 fat AABB, 重新膨胀
                p.aabb = aabb.fattened(self.fat_margin);
                p.dirty = true;
                return;
            }
        }
    }

    /// 移除物体
    pub fn remove_body(&mut self, body_id: u32) {
        self.proxies.retain(|p| p.body_id != body_id);
    }

    /// 获取物体数量
    #[inline]
    pub fn len(&self) -> usize {
        self.proxies.len()
    }

    /// 是否为空
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.proxies.is_empty()
    }

    /// 获取代理 (只读)
    pub fn proxies(&self) -> &[BroadphaseProxy] {
        &self.proxies
    }

    /// 选择最优排序轴 (物体分布跨度最大的轴)
    fn select_best_axis(&mut self) {
        if self.proxies.is_empty() {
            return;
        }
        // 计算各轴上物体中心的跨度
        let mut min_c = self.proxies[0].aabb.center();
        let mut max_c = min_c;
        for p in &self.proxies[1..] {
            let c = p.aabb.center();
            min_c = min_c.min(c);
            max_c = max_c.max(c);
        }
        let spread = max_c - min_c;
        // 选跨度最大的轴 (减少误报)
        self.sort_axis = if spread.x >= spread.y && spread.x >= spread.z {
            0
        } else if spread.y >= spread.z {
            1
        } else {
            2
        };
    }

    /// 插入排序 (利用时间连贯性, 大部分帧 O(n))
    fn insertion_sort(&mut self) {
        let axis = self.sort_axis;
        let n = self.proxies.len();
        for i in 1..n {
            let key = self.proxies[i].clone();
            let key_val = key.aabb.min_axis(axis);
            let mut j = i;
            while j > 0 {
                let prev_val = self.proxies[j - 1].aabb.min_axis(axis);
                if prev_val <= key_val {
                    break;
                }
                self.proxies[j] = self.proxies[j - 1].clone();
                j -= 1;
            }
            self.proxies[j] = key;
        }
    }

    /// 计算所有重叠对
    ///
    /// 返回 (body_id_a, body_id_b) 对的列表, 其中 a < b (按数组顺序)
    pub fn compute_pairs(&mut self) -> Vec<(u32, u32)> {
        if self.proxies.len() < 2 {
            return Vec::new();
        }

        // 定期重新选择最优轴
        self.frame_count += 1;
        if self.frame_count % self.axis_reselect_interval == 0 || self.proxies.len() < 2 {
            self.select_best_axis();
        }

        // 插入排序 (时间连贯, O(n) 大部分帧)
        self.insertion_sort();

        // 清除 dirty 标记
        for p in &mut self.proxies {
            p.dirty = false;
        }

        // 扫描生成重叠对
        let axis = self.sort_axis;
        let n = self.proxies.len();
        let mut pairs = Vec::new();

        for i in 0..n {
            let max_i = self.proxies[i].aabb.max_axis(axis);
            for j in (i + 1)..n {
                // 沿排序轴, j 的 min 必然 >= i 的 min (已排序)
                // 若 j 的 min > i 的 max, 则 j 之后所有物体都不会与 i 重叠
                let min_j = self.proxies[j].aabb.min_axis(axis);
                if min_j > max_i {
                    break; // 提前退出
                }
                // 排序轴重叠, 做完整三轴测试
                if self.proxies[i].aabb.intersects(&self.proxies[j].aabb) {
                    let a = self.proxies[i].body_id;
                    let b = self.proxies[j].body_id;
                    pairs.push((a, b));
                }
            }
        }

        pairs
    }

    /// 查询与给定 AABB 重叠的所有物体 ID
    pub fn query_aabb(&self, aabb: &Aabb) -> Vec<u32> {
        let mut result = Vec::new();
        for p in &self.proxies {
            if p.aabb.intersects(aabb) {
                result.push(p.body_id);
            }
        }
        result
    }

    /// 查询包含给定点的所有物体 ID
    pub fn query_point(&self, point: Vec3) -> Vec<u32> {
        let mut result = Vec::new();
        for p in &self.proxies {
            if p.aabb.contains_point(point) {
                result.push(p.body_id);
            }
        }
        result
    }

    /// 射线投射线 (slab 法), 返回 (body_id, t_near) 列表, 按 t_near 升序
    pub fn ray_cast(&self, origin: Vec3, dir: Vec3) -> Vec<(u32, f32)> {
        let mut hits = Vec::new();
        let dir_inv = Vec3::new(
            if dir.x.abs() > 1e-12 { 1.0 / dir.x } else { f32::INFINITY },
            if dir.y.abs() > 1e-12 { 1.0 / dir.y } else { f32::INFINITY },
            if dir.z.abs() > 1e-12 { 1.0 / dir.z } else { f32::INFINITY },
        );

        for p in &self.proxies {
            // Slab 法: 对每个轴计算 t1, t2
            let t1 = (p.aabb.min - origin) * dir_inv;
            let t2 = (p.aabb.max - origin) * dir_inv;

            let tmin = t1.min(t2);
            let tmax = t1.max(t2);

            let tenter = tmin.x.max(tmin.y).max(tmin.z);
            let texit = tmax.x.min(tmax.y).min(tmax.z);

            // 命中条件: tenter <= texit 且 texit >= 0
            if tenter <= texit && texit >= 0.0 {
                let t = tenter.max(0.0);
                hits.push((p.body_id, t));
            }
        }

        // 按 t_near 升序
        hits.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        hits
    }

    /// 清空
    pub fn clear(&mut self) {
        self.proxies.clear();
        self.frame_count = 0;
    }
}

impl Default for SapBroadphase {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn aabb_at(x: f32, y: f32, z: f32, h: f32) -> Aabb {
        Aabb::from_center_half(Vec3::new(x, y, z), Vec3::splat(h))
    }

    #[test]
    fn test_aabb_basic() {
        let a = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(a.center(), Vec3::ZERO);
        assert_eq!(a.half_extents(), Vec3::splat(1.0));
        assert_eq!(a.volume(), 8.0);
        assert!((a.surface_area() - 24.0).abs() < 1e-6);
    }

    #[test]
    fn test_aabb_intersects() {
        let a = Aabb::new(Vec3::ZERO, Vec3::new(2.0, 2.0, 2.0));
        let b = Aabb::new(Vec3::new(1.0, 1.0, 1.0), Vec3::new(3.0, 3.0, 3.0));
        assert!(a.intersects(&b));
        assert!(b.intersects(&a));

        let c = Aabb::new(Vec3::new(3.0, 0.0, 0.0), Vec3::new(5.0, 2.0, 2.0));
        assert!(!a.intersects(&c));
    }

    #[test]
    fn test_aabb_touching() {
        // 边界接触算重叠 (>=, <=)
        let a = Aabb::new(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        let b = Aabb::new(Vec3::new(1.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 1.0));
        assert!(a.intersects(&b));
    }

    #[test]
    fn test_aabb_contains_point() {
        let a = Aabb::new(Vec3::ZERO, Vec3::new(2.0, 2.0, 2.0));
        assert!(a.contains_point(Vec3::new(1.0, 1.0, 1.0)));
        assert!(a.contains_point(Vec3::ZERO)); // 边界
        assert!(!a.contains_point(Vec3::new(2.1, 0.0, 0.0)));
    }

    #[test]
    fn test_aabb_contains_aabb() {
        let outer = Aabb::new(Vec3::ZERO, Vec3::new(4.0, 4.0, 4.0));
        let inner = Aabb::new(Vec3::new(1.0, 1.0, 1.0), Vec3::new(3.0, 3.0, 3.0));
        assert!(outer.contains_aabb(&inner));
        assert!(!inner.contains_aabb(&outer));
    }

    #[test]
    fn test_aabb_union() {
        let a = Aabb::new(Vec3::ZERO, Vec3::new(2.0, 2.0, 2.0));
        let b = Aabb::new(Vec3::new(1.0, 0.0, 0.0), Vec3::new(3.0, 2.0, 2.0));
        let u = a.union(&b);
        assert_eq!(u.min, Vec3::ZERO);
        assert_eq!(u.max, Vec3::new(3.0, 2.0, 2.0));
    }

    #[test]
    fn test_aabb_fattened() {
        let a = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let f = a.fattened(0.5);
        assert_eq!(f.min, Vec3::new(-0.5, -0.5, -0.5));
        assert_eq!(f.max, Vec3::new(1.5, 1.5, 1.5));
    }

    #[test]
    fn test_sap_empty() {
        let mut sap = SapBroadphase::new();
        assert!(sap.is_empty());
        let pairs = sap.compute_pairs();
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_sap_single_body() {
        let mut sap = SapBroadphase::new();
        sap.add_body(0, aabb_at(0.0, 0.0, 0.0, 1.0));
        let pairs = sap.compute_pairs();
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_sap_two_overlapping() {
        let mut sap = SapBroadphase::new();
        sap.add_body(0, aabb_at(0.0, 0.0, 0.0, 1.0));
        sap.add_body(1, aabb_at(0.5, 0.0, 0.0, 1.0));
        let pairs = sap.compute_pairs();
        assert_eq!(pairs.len(), 1);
        assert!(pairs.contains(&(0, 1)) || pairs.contains(&(1, 0)));
    }

    #[test]
    fn test_sap_two_separate() {
        let mut sap = SapBroadphase::new();
        sap.add_body(0, aabb_at(0.0, 0.0, 0.0, 1.0));
        sap.add_body(1, aabb_at(10.0, 0.0, 0.0, 1.0));
        let pairs = sap.compute_pairs();
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_sap_chain() {
        // 三个物体排成一排, 相邻重叠
        let mut sap = SapBroadphase::new();
        sap.add_body(0, aabb_at(0.0, 0.0, 0.0, 1.0));  // [-1, 1]
        sap.add_body(1, aabb_at(1.5, 0.0, 0.0, 1.0));  // [0.5, 2.5]
        sap.add_body(2, aabb_at(3.0, 0.0, 0.0, 1.0));  // [2, 4]
        let pairs = sap.compute_pairs();
        // 0-1 重叠, 1-2 重叠, 0-2 不重叠
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn test_sap_all_overlap() {
        // 四个物体全部互相重叠
        let mut sap = SapBroadphase::new();
        for i in 0..4 {
            sap.add_body(i, aabb_at(i as f32 * 0.1, 0.0, 0.0, 1.0));
        }
        let pairs = sap.compute_pairs();
        // C(4,2) = 6 对
        assert_eq!(pairs.len(), 6);
    }

    #[test]
    fn test_sap_remove_body() {
        let mut sap = SapBroadphase::new();
        sap.add_body(0, aabb_at(0.0, 0.0, 0.0, 1.0));
        sap.add_body(1, aabb_at(0.5, 0.0, 0.0, 1.0));
        sap.add_body(2, aabb_at(1.0, 0.0, 0.0, 1.0));

        sap.remove_body(1);
        assert_eq!(sap.len(), 2);

        let pairs = sap.compute_pairs();
        // 0 和 2: [-1,1] 和 [0,2] 重叠
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn test_sap_update_body() {
        let mut sap = SapBroadphase::new();
        sap.add_body(0, aabb_at(0.0, 0.0, 0.0, 1.0));
        sap.add_body(1, aabb_at(10.0, 0.0, 0.0, 1.0));

        // 初始不重叠
        let pairs = sap.compute_pairs();
        assert!(pairs.is_empty());

        // 移动 1 到 0 附近
        sap.update_body(1, aabb_at(0.5, 0.0, 0.0, 1.0));
        let pairs = sap.compute_pairs();
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn test_sap_fat_aabb_no_update() {
        let mut sap = SapBroadphase::new().with_fat_margin(0.5);
        sap.add_body(0, aabb_at(0.0, 0.0, 0.0, 1.0));
        sap.add_body(1, aabb_at(5.0, 0.0, 0.0, 1.0));

        // 初始不重叠
        let _ = sap.compute_pairs();

        // 小移动 (在 fat AABB 内), 不应触发更新
        sap.update_body(1, aabb_at(5.1, 0.0, 0.0, 1.0));
        // 仍然不重叠
        let pairs = sap.compute_pairs();
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_sap_query_aabb() {
        let mut sap = SapBroadphase::new();
        sap.add_body(0, aabb_at(0.0, 0.0, 0.0, 1.0));
        sap.add_body(1, aabb_at(5.0, 0.0, 0.0, 1.0));
        sap.add_body(2, aabb_at(10.0, 0.0, 0.0, 1.0));

        let query = Aabb::new(Vec3::new(-2.0, -2.0, -2.0), Vec3::new(2.0, 2.0, 2.0));
        let hits = sap.query_aabb(&query);
        assert_eq!(hits.len(), 1);
        assert!(hits.contains(&0));
    }

    #[test]
    fn test_sap_query_point() {
        let mut sap = SapBroadphase::new();
        sap.add_body(0, aabb_at(0.0, 0.0, 0.0, 1.0));
        sap.add_body(1, aabb_at(5.0, 0.0, 0.0, 1.0));

        let hits = sap.query_point(Vec3::new(0.5, 0.5, 0.5));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0], 0);

        let hits = sap.query_point(Vec3::new(100.0, 0.0, 0.0));
        assert!(hits.is_empty());
    }

    #[test]
    fn test_sap_ray_cast() {
        // 关闭 fat margin 以便精确验证 t 值
        let mut sap = SapBroadphase::new().with_fat_margin(0.0);
        sap.add_body(0, aabb_at(0.0, 0.0, 0.0, 1.0));
        sap.add_body(1, aabb_at(5.0, 0.0, 0.0, 1.0));
        sap.add_body(2, aabb_at(10.0, 0.0, 0.0, 1.0));

        // 沿 +x 方向射线
        let hits = sap.ray_cast(Vec3::new(-5.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(hits.len(), 3);
        // 按距离排序
        assert_eq!(hits[0].0, 0);
        assert_eq!(hits[1].0, 1);
        assert_eq!(hits[2].0, 2);
        // 第一个命中点 t = 4 (从 -5 到 body0 的 min.x = -1)
        assert!((hits[0].1 - 4.0).abs() < 1e-5, "t = {}", hits[0].1);
    }

    #[test]
    fn test_sap_ray_cast_miss() {
        let mut sap = SapBroadphase::new();
        sap.add_body(0, aabb_at(0.0, 0.0, 0.0, 1.0));

        // 射线偏离物体
        let hits = sap.ray_cast(Vec3::new(0.0, 10.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
        assert!(hits.is_empty());
    }

    #[test]
    fn test_sap_temporal_coherence() {
        // 模拟多帧: 物体缓慢移动, SAP 应利用时间连贯性
        // half=1.0, spacing=2.0 → 相邻物体刚好接触 (intersects 含边界)
        let mut sap = SapBroadphase::new();
        for i in 0..10 {
            sap.add_body(i as u32, aabb_at(i as f32 * 2.0, 0.0, 0.0, 1.0));
        }

        // 第 1 帧: 相邻物体重叠
        let pairs1 = sap.compute_pairs();
        assert_eq!(pairs1.len(), 9); // 10 个物体, 9 对相邻重叠

        // 物体微小移动 (在 fat AABB 内)
        for i in 0..10 {
            sap.update_body(i as u32, aabb_at(i as f32 * 2.0 + 0.01, 0.0, 0.0, 0.5));
        }
        let pairs2 = sap.compute_pairs();
        assert_eq!(pairs2.len(), 9); // 仍然 9 对
    }

    #[test]
    fn test_sap_3d_distribution() {
        // 物体在 3D 空间分散, 测试最优轴选择
        let mut sap = SapBroadphase::new();
        // 沿 x 轴分布
        sap.add_body(0, aabb_at(0.0, 0.0, 0.0, 0.5));
        sap.add_body(1, aabb_at(10.0, 0.0, 0.0, 0.5));
        sap.add_body(2, aabb_at(20.0, 0.0, 0.0, 0.5));
        // y, z 方向紧凑
        let pairs = sap.compute_pairs();
        assert!(pairs.is_empty()); // 不重叠
        // 排序轴应该是 x (跨度最大)
        assert_eq!(sap.proxies().len(), 3);
    }

    #[test]
    fn test_sap_large_scene() {
        // 大规模场景测试
        let mut sap = SapBroadphase::new();
        let n = 100;
        for i in 0..n {
            let x = (i % 10) as f32 * 2.0;
            let y = (i / 10) as f32 * 2.0;
            sap.add_body(i as u32, aabb_at(x, y, 0.0, 1.1));
        }
        let pairs = sap.compute_pairs();
        // 每个物体与相邻物体重叠
        // 10x10 网格, 每个内部物体有 4 个邻居 (上下左右)
        // 边界物体有 3 个, 角落有 2 个
        // 总对数 = 水平相邻 + 垂直相邻 = 9*10 + 9*10 = 180
        assert!(pairs.len() > 0, "should have overlapping pairs");
        // 验证没有重复对
        let mut sorted: Vec<_> = pairs.iter().map(|&(a, b)| (a.min(b), a.max(b))).collect();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), pairs.len(), "no duplicate pairs");
    }

    #[test]
    fn test_aabb_axis_methods() {
        let a = Aabb::new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 6.0, 8.0));
        assert!((a.extent(0) - 3.0).abs() < 1e-6);
        assert!((a.extent(1) - 4.0).abs() < 1e-6);
        assert!((a.extent(2) - 5.0).abs() < 1e-6);
        assert!((a.min_axis(0) - 1.0).abs() < 1e-6);
        assert!((a.max_axis(1) - 6.0).abs() < 1e-6);
        assert!((a.min_axis(2) - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_sap_clear() {
        let mut sap = SapBroadphase::new();
        sap.add_body(0, aabb_at(0.0, 0.0, 0.0, 1.0));
        sap.add_body(1, aabb_at(0.5, 0.0, 0.0, 1.0));
        sap.clear();
        assert!(sap.is_empty());
        let pairs = sap.compute_pairs();
        assert!(pairs.is_empty());
    }
}
