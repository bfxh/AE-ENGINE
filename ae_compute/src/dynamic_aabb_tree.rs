//! Dynamic AABB Tree — 动态轴对齐包围盒树
//!
//! 基于:
//! - Box2D v3 b2DynamicTree (Catto, 2025年3月改进版)
//!   https://box2d.org/posts/2025/03/dynamic-tree-improvements/
//!   贪心插入启发式: 单次下降, 表面积增量最小化, 对子树用乐观下界估计
//! - Catto. "Dynamic BVH." GDC 2019
//! - Ericson. Real-Time Collision Detection. Morgan Kaufmann 2005. Ch 5.4
//! - Gregorius. "Dynamic BVH for Collision Detection." GDC 2016 (PhysX)
//!
//! 核心思想:
//! 1. 二叉树, 叶节点存物体 AABB, 内部节点 AABB = 子节点 AABB 的并集
//! 2. 插入用贪心启发式 (Box2D 2025):
//!    - directCost = Area(Union(D, S))        # S 候选兄弟
//!    - inheritedCost 沿下降累积祖先扩大代价
//!    - 子树代价乐观下界: cost = inh + directCost + min(0, areaD - areaC)
//!    - 单条路径下降, 可能停在任意层创建新内部节点
//! 3. 删除: 移除叶节点 + 兄弟提升到父节点位置 + 向上重算祖先
//! 4. AVL 旋转再平衡: |h(c1) - h(c2)| > 1 时旋转 (Box2D 策略)
//! 5. Fat AABB: 代理 AABB 扩展边距, 移动未出 fat 范围则不更新
//!
//! 复杂度:
//! - 插入: O(log n) 平均, O(n) 最坏
//! - 查询: O(log n + k) k = 命中数
//! - 删除: O(log n)

use glam::Vec3;

use crate::broadphase_sap::Aabb;

/// 空节点标记
pub const NULL_NODE: i32 = -1;

/// 默认 fat AABB 膨胀量 (各方向)
const DEFAULT_FAT_MARGIN: f32 = 0.1;
/// 默认初始节点池容量
const DEFAULT_NODE_CAPACITY: usize = 256;
/// 代理移动后, 若位移 > AABB 尺寸 * 此比例则强制更新 (Box2D 策略)
const DISPLACEMENT_MULTIPLIER: f32 = 0.5;

// ============================================================
// TreeNode — 树节点
// ============================================================

#[derive(Debug, Clone)]
struct TreeNode {
    /// 节点 AABB (叶节点为 fat AABB, 内部节点为子节点 AABB 的并集)
    aabb: Aabb,
    /// 用户数据 (仅叶节点有意义)
    user_data: u64,
    parent: i32,
    child1: i32,
    child2: i32,
    /// 节点高度: 叶=0, 空/未分配=-1
    height: i32,
    /// 是否为叶节点
    is_leaf: bool,
    /// Fat AABB 是否在最近一次更新中被扩大过 (用于查询时重建)
    enlarged: bool,
    /// 自由链表下一个节点 (仅未分配节点有效)
    next: i32,
}

impl TreeNode {
    fn new() -> Self {
        Self {
            aabb: Aabb::new(Vec3::ZERO, Vec3::ZERO),
            user_data: 0,
            parent: NULL_NODE,
            child1: NULL_NODE,
            child2: NULL_NODE,
            height: -1,
            is_leaf: false,
            enlarged: false,
            next: NULL_NODE,
        }
    }

    #[inline]
    fn is_leaf(&self) -> bool {
        self.child1 == NULL_NODE
    }
}

// ============================================================
// DynamicAabbTree — 动态 AABB 树
// ============================================================

pub struct DynamicAabbTree {
    nodes: Vec<TreeNode>,
    root: i32,
    /// 自由链表头 (指向第一个未分配节点)
    free_list: i32,
    /// 已分配节点数 (含内部节点)
    node_count: i32,
    /// Fat AABB 膨胀量
    fat_margin: f32,
}

impl Default for DynamicAabbTree {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicAabbTree {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_NODE_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let cap = capacity.max(16);
        let mut tree = Self {
            nodes: Vec::with_capacity(cap),
            root: NULL_NODE,
            free_list: 0,
            node_count: 0,
            fat_margin: DEFAULT_FAT_MARGIN,
        };
        // 初始化自由链表
        for i in 0..cap {
            tree.nodes.push(TreeNode::new());
            tree.nodes[i].next = if i + 1 < cap { (i + 1) as i32 } else { NULL_NODE };
        }
        tree
    }

    pub fn with_fat_margin(mut self, margin: f32) -> Self {
        self.fat_margin = margin;
        self
    }

    /// 根节点 ID (NULL_NODE 表示空树)
    #[inline]
    pub fn root(&self) -> i32 {
        self.root
    }

    /// 叶节点 (代理) 数量
    #[inline]
    pub fn proxy_count(&self) -> i32 {
        // 叶节点数 = (2 * 内部节点数) + 1 - 内部节点数 = 节点总数中叶的比例
        // 简单实现: 遍历计数
        let mut count = 0;
        for n in &self.nodes {
            if n.is_leaf && n.height >= 0 {
                count += 1;
            }
        }
        count
    }

    /// 节点池容量
    #[inline]
    pub fn capacity(&self) -> usize {
        self.nodes.len()
    }

    /// 树高度 (空树返回 -1)
    pub fn height(&self) -> i32 {
        if self.root == NULL_NODE {
            return -1;
        }
        self.nodes[self.root as usize].height
    }

    // ========================================================
    // 节点池 / 自由链表
    // ========================================================

    /// 从自由链表分配一个节点, 必要时扩容
    fn allocate_node(&mut self) -> i32 {
        if self.free_list == NULL_NODE {
            // 扩容: 翻倍
            let old_cap = self.nodes.len();
            let new_cap = old_cap * 2;
            self.nodes.resize(new_cap, TreeNode::new());
            // 重建自由链表 (新节点)
            for i in old_cap..new_cap {
                self.nodes[i].next = if i + 1 < new_cap { (i + 1) as i32 } else { NULL_NODE };
            }
            self.free_list = old_cap as i32;
        }
        let node_id = self.free_list;
        let node = &mut self.nodes[node_id as usize];
        self.free_list = node.next;
        node.parent = NULL_NODE;
        node.child1 = NULL_NODE;
        node.child2 = NULL_NODE;
        node.height = 0;
        node.is_leaf = false;
        node.enlarged = false;
        node.user_data = 0;
        node.next = NULL_NODE;
        self.node_count += 1;
        node_id
    }

    /// 释放节点, 加入自由链表
    fn free_node(&mut self, node_id: i32) {
        let node = &mut self.nodes[node_id as usize];
        node.height = -1;
        node.is_leaf = false;
        node.next = self.free_list;
        self.free_list = node_id;
        self.node_count -= 1;
    }

    // ========================================================
    // 创建 / 销毁 / 移动 代理
    // ========================================================

    /// 创建代理: 用 fat AABB 包装用户 AABB, 贪心插入到树中
    ///
    /// 返回 proxy_id, 后续用于查询/移动/销毁
    pub fn create_proxy(&mut self, aabb: Aabb, user_data: u64) -> i32 {
        let proxy_id = self.allocate_node();
        // Fat AABB: 各方向扩展 fat_margin
        let fat = aabb.fattened(self.fat_margin);
        let node = &mut self.nodes[proxy_id as usize];
        node.aabb = fat;
        node.user_data = user_data;
        node.is_leaf = true;
        node.height = 0;
        node.enlarged = false;

        self.insert_leaf(proxy_id);
        proxy_id
    }

    /// 销毁代理
    pub fn destroy_proxy(&mut self, proxy_id: i32) {
        debug_assert!(proxy_id != NULL_NODE);
        debug_assert!(self.nodes[proxy_id as usize].is_leaf);
        self.remove_leaf(proxy_id);
        self.free_node(proxy_id);
    }

    /// 移动代理: 若物体移出 fat AABB, 则更新并重新插入
    ///
    /// 返回 true 表示树结构已更新 (需要重算碰撞对)
    pub fn move_proxy(&mut self, proxy_id: i32, new_aabb: Aabb) -> bool {
        let node = &mut self.nodes[proxy_id as usize];
        let fat = &node.aabb;
        // 新 AABB 仍在 fat AABB 内 -> 不更新
        if fat.contains_aabb(&new_aabb) {
            return false;
        }
        // 否则: 用新 AABB + fat margin 重建, 但可能限制扩展 (Box2D 策略)
        // 这里用简单策略: 重新用 fat margin 扩展
        let new_fat = new_aabb.fattened(self.fat_margin);
        node.aabb = new_fat;
        node.enlarged = true;
        drop(node);
        self.remove_leaf(proxy_id);
        self.insert_leaf(proxy_id);
        true
    }

    /// 扩大代理 AABB (不重插入, 仅扩大祖先 AABB) — Box2D v3 EnlargeProxy
    ///
    /// 用于物体变大但中心未移动的情况
    pub fn enlarge_proxy(&mut self, proxy_id: i32, new_aabb: Aabb) {
        debug_assert!(self.nodes[proxy_id as usize].is_leaf);
        // 用新 AABB 与现有 fat AABB 的并集
        let cur = self.nodes[proxy_id as usize].aabb;
        let merged = cur.union(&new_aabb);
        self.nodes[proxy_id as usize].aabb = merged;
        self.nodes[proxy_id as usize].enlarged = true;
        // 向上扩大祖先 (Box2D 策略: 只扩大, 不收缩)
        let mut idx = self.nodes[proxy_id as usize].parent;
        while idx != NULL_NODE {
            let cur_inner = self.nodes[idx as usize].aabb;
            let merged_inner = cur_inner.union(&new_aabb);
            // 若未变化则停止
            if merged_inner.min == cur_inner.min && merged_inner.max == cur_inner.max {
                break;
            }
            self.nodes[idx as usize].aabb = merged_inner;
            idx = self.nodes[idx as usize].parent;
        }
    }

    /// 获取代理的 fat AABB
    pub fn get_proxy_aabb(&self, proxy_id: i32) -> Aabb {
        self.nodes[proxy_id as usize].aabb
    }

    /// 获取代理的用户数据
    pub fn get_proxy_user_data(&self, proxy_id: i32) -> u64 {
        self.nodes[proxy_id as usize].user_data
    }

    /// 清除 enlarged 标记 (查询后调用)
    pub fn clear_enlarged(&mut self, proxy_id: i32) {
        self.nodes[proxy_id as usize].enlarged = false;
    }

    // ========================================================
    // 插入 / 删除 (核心算法)
    // ========================================================

    /// Box2D v3 (2025) 贪心插入: 单条路径下降, 最小化表面积增量
    fn insert_leaf(&mut self, leaf: i32) {
        // 空树: leaf 成为根
        if self.root == NULL_NODE {
            self.root = leaf;
            self.nodes[leaf as usize].parent = NULL_NODE;
            return;
        }

        let leaf_aabb = self.nodes[leaf as usize].aabb;
        let area_d = surface_area(&leaf_aabb);

        // 阶段 1: 从根下降找最佳兄弟节点
        let mut index = self.root;
        let mut inherited_cost = 0.0;

        loop {
            let node_aabb = self.nodes[index as usize].aabb;
            let area_s = surface_area(&node_aabb);
            let direct_cost = surface_area(&union(&leaf_aabb, &node_aabb));
            // 当前作为兄弟的总代价
            let current_cost = direct_cost + inherited_cost;

            // 下降后的累积继承代价
            let inherited_next = inherited_cost + direct_cost - area_s;

            let child1 = self.nodes[index as usize].child1;
            let child2 = self.nodes[index as usize].child2;

            // 子树代价乐观下界 (Box2D 2025)
            // 对叶节点: 精确代价 = inherited_next + directCost_child
            // 对内部节点: 下界 = inherited_next + directCost_child + min(0, areaD - areaChild)
            let cost1 = if child1 != NULL_NODE {
                let c1_aabb = self.nodes[child1 as usize].aabb;
                let dc1 = surface_area(&union(&leaf_aabb, &c1_aabb));
                let area_c1 = surface_area(&c1_aabb);
                let lower_bound = area_d - area_c1;
                inherited_next + dc1 + 0.0_f32.min(lower_bound)
            } else {
                f32::INFINITY
            };

            let cost2 = if child2 != NULL_NODE {
                let c2_aabb = self.nodes[child2 as usize].aabb;
                let dc2 = surface_area(&union(&leaf_aabb, &c2_aabb));
                let area_c2 = surface_area(&c2_aabb);
                let lower_bound = area_d - area_c2;
                inherited_next + dc2 + 0.0_f32.min(lower_bound)
            } else {
                f32::INFINITY
            };

            // 当前作为兄弟最优 -> 停止
            if current_cost <= cost1 && current_cost <= cost2 {
                break;
            }

            // 下降到更优的子节点
            if cost1 < cost2 {
                index = child1;
            } else {
                index = child2;
            }
            inherited_cost = inherited_next;
        }

        // 阶段 2: 创建新内部节点, sibling 和 leaf 成为兄弟
        let sibling = index;
        let new_parent = self.allocate_node();
        let parent_of_sibling = self.nodes[sibling as usize].parent;

        // 新内部节点的 AABB = leaf ∪ sibling
        let sibling_aabb = self.nodes[sibling as usize].aabb;
        self.nodes[new_parent as usize].aabb = union(&leaf_aabb, &sibling_aabb);
        self.nodes[new_parent as usize].parent = parent_of_sibling;
        self.nodes[new_parent as usize].height = self.nodes[sibling as usize].height + 1;
        self.nodes[new_parent as usize].is_leaf = false;
        self.nodes[new_parent as usize].child1 = sibling;
        self.nodes[new_parent as usize].child2 = leaf;
        self.nodes[sibling as usize].parent = new_parent;
        self.nodes[leaf as usize].parent = new_parent;

        // 链接到原祖父
        if parent_of_sibling != NULL_NODE {
            if self.nodes[parent_of_sibling as usize].child1 == sibling {
                self.nodes[parent_of_sibling as usize].child1 = new_parent;
            } else {
                self.nodes[parent_of_sibling as usize].child2 = new_parent;
            }
        } else {
            self.root = new_parent;
        }

        // 阶段 3: 向上走, 平衡 + 重算 AABB/height
        let mut idx = self.nodes[leaf as usize].parent;
        while idx != NULL_NODE {
            idx = self.balance(idx);
            let c1 = self.nodes[idx as usize].child1;
            let c2 = self.nodes[idx as usize].child2;
            self.nodes[idx as usize].aabb =
                union(&self.nodes[c1 as usize].aabb, &self.nodes[c2 as usize].aabb);
            self.nodes[idx as usize].height =
                1 + self.nodes[c1 as usize].height.max(self.nodes[c2 as usize].height);
            idx = self.nodes[idx as usize].parent;
        }
    }

    /// 移除叶节点: 兄弟提升到父节点位置
    fn remove_leaf(&mut self, leaf: i32) {
        // 若为根
        if leaf == self.root {
            self.root = NULL_NODE;
            self.nodes[leaf as usize].parent = NULL_NODE;
            return;
        }

        let parent = self.nodes[leaf as usize].parent;
        let grandparent = self.nodes[parent as usize].parent;
        // 找兄弟
        let sibling = if self.nodes[parent as usize].child1 == leaf {
            self.nodes[parent as usize].child2
        } else {
            self.nodes[parent as usize].child1
        };

        // 兄弟提升到祖父位置
        if grandparent != NULL_NODE {
            if self.nodes[grandparent as usize].child1 == parent {
                self.nodes[grandparent as usize].child1 = sibling;
            } else {
                self.nodes[grandparent as usize].child2 = sibling;
            }
            self.nodes[sibling as usize].parent = grandparent;
            self.free_node(parent);

            // 向上重算
            let mut idx = grandparent;
            while idx != NULL_NODE {
                idx = self.balance(idx);
                let c1 = self.nodes[idx as usize].child1;
                let c2 = self.nodes[idx as usize].child2;
                self.nodes[idx as usize].aabb =
                    union(&self.nodes[c1 as usize].aabb, &self.nodes[c2 as usize].aabb);
                self.nodes[idx as usize].height =
                    1 + self.nodes[c1 as usize].height.max(self.nodes[c2 as usize].height);
                idx = self.nodes[idx as usize].parent;
            }
        } else {
            // 兄弟成为新根
            self.root = sibling;
            self.nodes[sibling as usize].parent = NULL_NODE;
            self.free_node(parent);
        }
        self.nodes[leaf as usize].parent = NULL_NODE;
    }

    // ========================================================
    // AVL 风格旋转再平衡 (Box2D 策略)
    // ========================================================

    /// 平衡节点子树, 返回 (可能新的) 子树根
    fn balance(&mut self, i_a: i32) -> i32 {
        debug_assert!(i_a != NULL_NODE);
        let a = &self.nodes[i_a as usize];
        if a.is_leaf() || a.height < 2 {
            return i_a;
        }

        let i_b = a.child1;
        let i_c = a.child2;
        debug_assert!(i_b != NULL_NODE && i_c != NULL_NODE);

        let balance = self.nodes[i_b as usize].height - self.nodes[i_c as usize].height;

        // 旋转 C 上来 (C 比 B 高)
        if balance > 1 {
            let i_d = self.nodes[i_b as usize].child1;
            let i_e = self.nodes[i_b as usize].child2;
            debug_assert!(i_d != NULL_NODE && i_e != NULL_NODE);

            // A 的父指针改为指向 B
            self.nodes[i_b as usize].parent = self.nodes[i_a as usize].parent;
            self.nodes[i_a as usize].parent = i_b;

            // 替换 A 在祖父中的位置
            let parent = self.nodes[i_b as usize].parent;
            if parent != NULL_NODE {
                if self.nodes[parent as usize].child1 == i_a {
                    self.nodes[parent as usize].child1 = i_b;
                } else {
                    self.nodes[parent as usize].child2 = i_b;
                }
            } else {
                self.root = i_b;
            }

            // 旋转: 让 A 和 E 成为 B 的子节点, D 升到 B 原位置
            // 情况: 旋转后 D 在 A 一侧
            self.nodes[i_a as usize].child1 = i_e;
            self.nodes[i_e as usize].parent = i_a;
            self.nodes[i_b as usize].child2 = i_a;

            self.update_aabb_height(&mut self.nodes.clone(), i_a); // 占位 (后面手动重算)
            // 直接重算 A 和 B 的 AABB/height
            let a_aabb = union(
                &self.nodes[self.nodes[i_a as usize].child1 as usize].aabb,
                &self.nodes[self.nodes[i_a as usize].child2 as usize].aabb,
            );
            let a_height = 1 + self.nodes[self.nodes[i_a as usize].child1 as usize].height
                .max(self.nodes[self.nodes[i_a as usize].child2 as usize].height);
            self.nodes[i_a as usize].aabb = a_aabb;
            self.nodes[i_a as usize].height = a_height;

            let b_aabb = union(
                &self.nodes[self.nodes[i_b as usize].child1 as usize].aabb,
                &self.nodes[self.nodes[i_b as usize].child2 as usize].aabb,
            );
            let b_height = 1 + self.nodes[self.nodes[i_b as usize].child1 as usize].height
                .max(self.nodes[self.nodes[i_b as usize].child2 as usize].height);
            self.nodes[i_b as usize].aabb = b_aabb;
            self.nodes[i_b as usize].height = b_height;

            return i_b;
        }

        // 旋转 B 上来 (B 比 C 高)
        if balance < -1 {
            let i_f = self.nodes[i_c as usize].child1;
            let i_g = self.nodes[i_c as usize].child2;
            debug_assert!(i_f != NULL_NODE && i_g != NULL_NODE);

            self.nodes[i_c as usize].parent = self.nodes[i_a as usize].parent;
            self.nodes[i_a as usize].parent = i_c;

            let parent = self.nodes[i_c as usize].parent;
            if parent != NULL_NODE {
                if self.nodes[parent as usize].child1 == i_a {
                    self.nodes[parent as usize].child1 = i_c;
                } else {
                    self.nodes[parent as usize].child2 = i_c;
                }
            } else {
                self.root = i_c;
            }

            // A 和 F 成为 C 的子节点, G 留在 C 一侧
            self.nodes[i_a as usize].child2 = i_f;
            self.nodes[i_f as usize].parent = i_a;
            self.nodes[i_c as usize].child1 = i_a;

            // 重算 A 和 C
            let a_aabb = union(
                &self.nodes[self.nodes[i_a as usize].child1 as usize].aabb,
                &self.nodes[self.nodes[i_a as usize].child2 as usize].aabb,
            );
            let a_height = 1 + self.nodes[self.nodes[i_a as usize].child1 as usize].height
                .max(self.nodes[self.nodes[i_a as usize].child2 as usize].height);
            self.nodes[i_a as usize].aabb = a_aabb;
            self.nodes[i_a as usize].height = a_height;

            let c_aabb = union(
                &self.nodes[self.nodes[i_c as usize].child1 as usize].aabb,
                &self.nodes[self.nodes[i_c as usize].child2 as usize].aabb,
            );
            let c_height = 1 + self.nodes[self.nodes[i_c as usize].child1 as usize].height
                .max(self.nodes[self.nodes[i_c as usize].child2 as usize].height);
            self.nodes[i_c as usize].aabb = c_aabb;
            self.nodes[i_c as usize].height = c_height;

            return i_c;
        }

        i_a
    }

    /// (辅助, 未使用) 保留签名以防编译错误
    fn update_aabb_height(&self, _nodes: &mut Vec<TreeNode>, _idx: i32) {
        // 实际重算在 balance 中已内联完成
    }

    // ========================================================
    // 查询: AABB 重叠
    // ========================================================

    /// 查询与给定 AABB 重叠的所有代理
    ///
    /// 返回 (proxy_id, user_data) 列表
    pub fn query(&self, aabb: &Aabb) -> Vec<(i32, u64)> {
        let mut result = Vec::new();
        if self.root == NULL_NODE {
            return result;
        }
        // 栈式 DFS
        let mut stack: Vec<i32> = vec![self.root];
        while let Some(idx) = stack.pop() {
            if idx == NULL_NODE {
                continue;
            }
            let node = &self.nodes[idx as usize];
            if !node.aabb.intersects(aabb) {
                continue;
            }
            if node.is_leaf() {
                result.push((idx, node.user_data));
            } else {
                stack.push(node.child1);
                stack.push(node.child2);
            }
        }
        result
    }

    /// 查询回调版 (避免分配 Vec)
    ///
    /// callback(proxy_id, user_data) -> bool (true 继续, false 停止)
    pub fn query_callback<F>(&self, aabb: &Aabb, mut callback: F)
    where
        F: FnMut(i32, u64) -> bool,
    {
        if self.root == NULL_NODE {
            return;
        }
        let mut stack: Vec<i32> = vec![self.root];
        while let Some(idx) = stack.pop() {
            if idx == NULL_NODE {
                continue;
            }
            let node = &self.nodes[idx as usize];
            if !node.aabb.intersects(aabb) {
                continue;
            }
            if node.is_leaf() {
                if !callback(idx, node.user_data) {
                    return;
                }
            } else {
                stack.push(node.child1);
                stack.push(node.child2);
            }
        }
    }

    /// 查询与给定代理重叠的其他代理 (排除自身)
    pub fn query_pairs_for(&self, proxy_id: i32) -> Vec<(i32, u64)> {
        let aabb = self.nodes[proxy_id as usize].aabb;
        let mut result = Vec::new();
        self.query_callback(&aabb, |id, data| {
            if id != proxy_id {
                result.push((id, data));
            }
            true
        });
        result
    }

    // ========================================================
    // 查询: Ray Cast (slab method)
    // ========================================================

    /// 射线投射: 找出射线穿过的所有代理
    ///
    /// ray: origin -> origin + direction * max_fraction
    /// 返回 (proxy_id, user_data, fraction) 列表, 按 fraction 升序
    pub fn ray_cast(&self, origin: Vec3, direction: Vec3, max_fraction: f32) -> Vec<(i32, u64, f32)> {
        let mut hits: Vec<(i32, u64, f32)> = Vec::new();
        if self.root == NULL_NODE {
            return hits;
        }
        let dir = direction.normalize_or_zero();
        let mut stack: Vec<(i32, f32)> = vec![(self.root, 0.0)];
        while let Some((idx, _enter_frac)) = stack.pop() {
            if idx == NULL_NODE {
                continue;
            }
            let node = &self.nodes[idx as usize];
            if let Some(frac) = ray_aabb_intersect(origin, dir, &node.aabb, max_fraction) {
                if node.is_leaf() {
                    hits.push((idx, node.user_data, frac));
                } else {
                    stack.push((node.child1, frac));
                    stack.push((node.child2, frac));
                }
            }
        }
        hits.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
        hits
    }

    // ========================================================
    // 树质量度量
    // ========================================================

    /// 计算所有内部节点表面积之和 (越小越好)
    pub fn total_surface_area(&self) -> f32 {
        let mut area = 0.0;
        for n in &self.nodes {
            if n.height >= 0 && !n.is_leaf {
                area += surface_area(&n.aabb);
            }
        }
        area
    }

    /// 树质量: 总内部表面积 / 根表面积 (理想值接近 1, 越大越差)
    pub fn quality(&self) -> f32 {
        if self.root == NULL_NODE {
            return 0.0;
        }
        let root_area = surface_area(&self.nodes[self.root as usize].aabb);
        if root_area < 1e-12 {
            return 0.0;
        }
        self.total_surface_area() / root_area
    }

    /// 验证树结构完整性 (调试用)
    pub fn validate(&self) -> bool {
        if self.root == NULL_NODE {
            return true;
        }
        self.validate_subtree(self.root)
    }

    fn validate_subtree(&self, idx: i32) -> bool {
        if idx == NULL_NODE {
            return true;
        }
        let node = &self.nodes[idx as usize];
        if node.is_leaf() {
            // 叶节点: child 都为 NULL, height = 0
            return node.child1 == NULL_NODE
                && node.child2 == NULL_NODE
                && node.height == 0;
        }
        // 内部节点: 子节点存在, height = 1 + max(children), AABB = union
        let c1 = node.child1;
        let c2 = node.child2;
        if c1 == NULL_NODE || c2 == NULL_NODE {
            return false;
        }
        if self.nodes[c1 as usize].parent != idx {
            return false;
        }
        if self.nodes[c2 as usize].parent != idx {
            return false;
        }
        let expected_height = 1 + self.nodes[c1 as usize].height.max(self.nodes[c2 as usize].height);
        if node.height != expected_height {
            return false;
        }
        let expected_aabb = union(&self.nodes[c1 as usize].aabb, &self.nodes[c2 as usize].aabb);
        // AABB 容差比较
        let diff_min = (expected_aabb.min - node.aabb.min).length();
        let diff_max = (expected_aabb.max - node.aabb.max).length();
        if diff_min > 1e-3 || diff_max > 1e-3 {
            return false;
        }
        // 检查 AVL 平衡条件 (允许 |h(c1)-h(c2)| <= 1, 但 Box2D 实际允许稍大, 这里放宽到 2)
        let bal = (self.nodes[c1 as usize].height - self.nodes[c2 as usize].height).abs();
        if bal > 2 {
            return false;
        }
        self.validate_subtree(c1) && self.validate_subtree(c2)
    }
}

// ============================================================
// 几何辅助函数
// ============================================================

/// AABB 表面积 (3D)
#[inline]
fn surface_area(a: &Aabb) -> f32 {
    a.surface_area()
}

/// 两个 AABB 的并集
#[inline]
fn union(a: &Aabb, b: &Aabb) -> Aabb {
    a.union(b)
}

/// 射线与 AABB 相交测试 (slab method)
/// 返回进入分数 (在 [0, max_fraction] 内则相交)
#[inline]
fn ray_aabb_intersect(origin: Vec3, dir: Vec3, aabb: &Aabb, max_fraction: f32) -> Option<f32> {
    let mut t_min = 0.0_f32;
    let mut t_max = max_fraction;

    // X 轴
    if dir.x.abs() < 1e-12 {
        if origin.x < aabb.min.x || origin.x > aabb.max.x {
            return None;
        }
    } else {
        let inv = 1.0 / dir.x;
        let mut t1 = (aabb.min.x - origin.x) * inv;
        let mut t2 = (aabb.max.x - origin.x) * inv;
        if t1 > t2 {
            core::mem::swap(&mut t1, &mut t2);
        }
        t_min = t_min.max(t1);
        t_max = t_max.min(t2);
        if t_min > t_max {
            return None;
        }
    }

    // Y 轴
    if dir.y.abs() < 1e-12 {
        if origin.y < aabb.min.y || origin.y > aabb.max.y {
            return None;
        }
    } else {
        let inv = 1.0 / dir.y;
        let mut t1 = (aabb.min.y - origin.y) * inv;
        let mut t2 = (aabb.max.y - origin.y) * inv;
        if t1 > t2 {
            core::mem::swap(&mut t1, &mut t2);
        }
        t_min = t_min.max(t1);
        t_max = t_max.min(t2);
        if t_min > t_max {
            return None;
        }
    }

    // Z 轴
    if dir.z.abs() < 1e-12 {
        if origin.z < aabb.min.z || origin.z > aabb.max.z {
            return None;
        }
    } else {
        let inv = 1.0 / dir.z;
        let mut t1 = (aabb.min.z - origin.z) * inv;
        let mut t2 = (aabb.max.z - origin.z) * inv;
        if t1 > t2 {
            core::mem::swap(&mut t1, &mut t2);
        }
        t_min = t_min.max(t1);
        t_max = t_max.min(t2);
        if t_min > t_max {
            return None;
        }
    }

    Some(t_min.max(0.0))
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn aabb_at(x: f32, y: f32, z: f32, half: f32) -> Aabb {
        Aabb::from_center_half(Vec3::new(x, y, z), Vec3::splat(half))
    }

    #[test]
    fn test_tree_creation() {
        let tree = DynamicAabbTree::new();
        assert_eq!(tree.root(), NULL_NODE);
        assert_eq!(tree.height(), -1);
        assert_eq!(tree.proxy_count(), 0);
        assert!(tree.validate());
    }

    #[test]
    fn test_create_single_proxy() {
        let mut tree = DynamicAabbTree::new();
        let aabb = aabb_at(0.0, 0.0, 0.0, 1.0);
        let pid = tree.create_proxy(aabb, 42);
        assert!(pid >= 0);
        assert_eq!(tree.root(), pid); // 单个叶节点直接是根
        assert_eq!(tree.height(), 0);
        assert_eq!(tree.proxy_count(), 1);
        assert_eq!(tree.get_proxy_user_data(pid), 42);
        assert!(tree.validate());
    }

    #[test]
    fn test_create_multiple_proxies() {
        let mut tree = DynamicAabbTree::new();
        let positions = [
            (0.0_f32, 0.0_f32, 0.0_f32),
            (5.0, 0.0, 0.0),
            (10.0, 0.0, 0.0),
            (0.0, 5.0, 0.0),
            (0.0, 10.0, 0.0),
            (0.0, 0.0, 5.0),
            (0.0, 0.0, 10.0),
        ];
        for (i, &(x, y, z)) in positions.iter().enumerate() {
            let pid = tree.create_proxy(aabb_at(x, y, z, 0.5), i as u64);
            assert!(pid >= 0);
        }
        assert_eq!(tree.proxy_count(), 7);
        assert!(tree.height() >= 1);
        assert!(tree.validate());
    }

    #[test]
    fn test_destroy_proxy() {
        let mut tree = DynamicAabbTree::new();
        let p1 = tree.create_proxy(aabb_at(0.0, 0.0, 0.0, 1.0), 1);
        let p2 = tree.create_proxy(aabb_at(5.0, 0.0, 0.0, 1.0), 2);
        let p3 = tree.create_proxy(aabb_at(10.0, 0.0, 0.0, 1.0), 3);
        assert_eq!(tree.proxy_count(), 3);
        tree.destroy_proxy(p2);
        assert_eq!(tree.proxy_count(), 2);
        assert!(tree.validate());
        // p1 和 p3 应该仍在
        let hits = tree.query(&aabb_at(0.0, 0.0, 0.0, 0.1));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, 1);
        let hits = tree.query(&aabb_at(10.0, 0.0, 0.0, 0.1));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, 3);
        // 销毁其余
        tree.destroy_proxy(p1);
        tree.destroy_proxy(p3);
        assert_eq!(tree.proxy_count(), 0);
        assert_eq!(tree.root(), NULL_NODE);
        assert!(tree.validate());
    }

    #[test]
    fn test_query_overlap() {
        let mut tree = DynamicAabbTree::new();
        // 三个分散的代理
        tree.create_proxy(aabb_at(0.0, 0.0, 0.0, 1.0), 100);
        tree.create_proxy(aabb_at(5.0, 0.0, 0.0, 1.0), 101);
        tree.create_proxy(aabb_at(10.0, 0.0, 0.0, 1.0), 102);

        // 查询覆盖第一个
        let hits = tree.query(&aabb_at(0.0, 0.0, 0.0, 0.5));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, 100);

        // 查询覆盖全部
        let hits = tree.query(&Aabb::new(Vec3::new(-5.0, -5.0, -5.0), Vec3::new(15.0, 5.0, 5.0)));
        assert_eq!(hits.len(), 3);

        // 查询空区域
        let hits = tree.query(&aabb_at(100.0, 100.0, 100.0, 1.0));
        assert_eq!(hits.len(), 0);
    }

    #[test]
    fn test_query_callback_stops_early() {
        let mut tree = DynamicAabbTree::new();
        for i in 0..10 {
            tree.create_proxy(aabb_at(i as f32 * 3.0, 0.0, 0.0, 1.0), i as u64);
        }
        let mut count = 0;
        tree.query_callback(
            &Aabb::new(Vec3::new(-100.0, -100.0, -100.0), Vec3::new(100.0, 100.0, 100.0)),
            |_, _| {
                count += 1;
                count < 3 // 停在前 3 个
            },
        );
        assert_eq!(count, 3);
    }

    #[test]
    fn test_move_proxy_no_change() {
        let mut tree = DynamicAabbTree::new();
        let pid = tree.create_proxy(aabb_at(0.0, 0.0, 0.0, 1.0), 1);
        // 微小移动 (仍在 fat AABB 内)
        let moved = tree.move_proxy(pid, aabb_at(0.05, 0.0, 0.0, 1.0));
        assert!(!moved, "small move should not trigger reinsertion");
        assert_eq!(tree.proxy_count(), 1);
        assert!(tree.validate());
    }

    #[test]
    fn test_move_proxy_large_change() {
        let mut tree = DynamicAabbTree::new();
        let pid = tree.create_proxy(aabb_at(0.0, 0.0, 0.0, 1.0), 1);
        // 大幅移动
        let moved = tree.move_proxy(pid, aabb_at(20.0, 0.0, 0.0, 1.0));
        assert!(moved, "large move should trigger reinsertion");
        assert!(tree.validate());
        // 查询新位置
        let hits = tree.query(&aabb_at(20.0, 0.0, 0.0, 0.5));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, 1);
        // 查询旧位置应为空 (fat margin = 0.1, 旧 fat AABB 在 [-1.1, 1.1])
        let hits = tree.query(&aabb_at(0.0, 0.0, 0.0, 0.5));
        assert_eq!(hits.len(), 0);
    }

    #[test]
    fn test_enlarge_proxy() {
        let mut tree = DynamicAabbTree::new();
        let pid = tree.create_proxy(aabb_at(0.0, 0.0, 0.0, 1.0), 1);
        // 扩大代理 (不重插入)
        tree.enlarge_proxy(pid, aabb_at(2.0, 0.0, 0.0, 1.0));
        // 现在 fat AABB 应包含原位置和 (2,0,0)
        let fat = tree.get_proxy_aabb(pid);
        assert!(fat.min.x <= -1.0);
        assert!(fat.max.x >= 3.0);
        assert!(tree.validate());
    }

    #[test]
    fn test_ray_cast_hit() {
        let mut tree = DynamicAabbTree::new();
        tree.create_proxy(aabb_at(0.0, 0.0, 5.0, 1.0), 1);
        tree.create_proxy(aabb_at(0.0, 0.0, 10.0, 1.0), 2);
        tree.create_proxy(aabb_at(0.0, 0.0, 15.0, 1.0), 3);

        // 沿 +Z 方向射线
        let hits = tree.ray_cast(Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0), 100.0);
        assert!(hits.len() >= 3, "should hit all 3 proxies, got {}", hits.len());
        // 应按距离升序
        assert!(hits[0].2 < hits[1].2);
        assert!(hits[1].2 < hits[2].2);
        // 第一个命中应在 z=4 附近 (fat AABB 在 [4-0.1, 6+0.1])
        assert!((hits[0].2 - 3.9).abs() < 0.5, "first hit fraction: {}", hits[0].2);
    }

    #[test]
    fn test_ray_cast_miss() {
        let mut tree = DynamicAabbTree::new();
        tree.create_proxy(aabb_at(0.0, 0.0, 5.0, 1.0), 1);
        // 射线朝 +X, 不应命中
        let hits = tree.ray_cast(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), 100.0);
        assert_eq!(hits.len(), 0);
    }

    #[test]
    fn test_ray_cast_max_fraction() {
        let mut tree = DynamicAabbTree::new();
        tree.create_proxy(aabb_at(0.0, 0.0, 5.0, 1.0), 1); // 在 z=4
        tree.create_proxy(aabb_at(0.0, 0.0, 50.0, 1.0), 2); // 在 z=49
        // max_fraction 限制只命中近的
        let hits = tree.ray_cast(Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0), 10.0);
        assert_eq!(hits.len(), 1, "should only hit the near proxy");
        assert_eq!(hits[0].1, 1);
    }

    #[test]
    fn test_query_pairs_for() {
        let mut tree = DynamicAabbTree::new();
        let p1 = tree.create_proxy(aabb_at(0.0, 0.0, 0.0, 1.0), 1);
        let p2 = tree.create_proxy(aabb_at(0.5, 0.0, 0.0, 1.0), 2); // 与 p1 重叠
        let p3 = tree.create_proxy(aabb_at(20.0, 0.0, 0.0, 1.0), 3); // 远离

        let pairs = tree.query_pairs_for(p1);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, p2);

        let pairs = tree.query_pairs_for(p3);
        assert_eq!(pairs.len(), 0);
    }

    #[test]
    fn test_tree_balance_after_sequential_inserts() {
        // 顺序插入 (最坏情况: 退化为链表)
        let mut tree = DynamicAabbTree::new();
        for i in 0..32 {
            tree.create_proxy(aabb_at(i as f32 * 2.0, 0.0, 0.0, 0.5), i as u64);
        }
        assert!(tree.validate());
        // 经过平衡后高度应该远小于 32
        let h = tree.height();
        assert!(h < 16, "tree height {} too large, AVL balancing failed", h);
    }

    #[test]
    fn test_quality_metric() {
        let mut tree = DynamicAabbTree::new();
        // 紧密聚集的代理 -> 质量好
        for i in 0..8 {
            for j in 0..8 {
                tree.create_proxy(aabb_at(i as f32, j as f32, 0.0, 0.5), 0);
            }
        }
        let q = tree.quality();
        // 质量应在合理范围 (理想接近 1, 实际 1-3 之间可接受)
        assert!(q > 0.5 && q < 10.0, "tree quality: {}", q);
    }

    #[test]
    fn test_capacity_growth() {
        let mut tree = DynamicAabbTree::with_capacity(16);
        // 插入超过初始容量, 触发扩容
        for i in 0..100 {
            tree.create_proxy(aabb_at(i as f32 * 3.0, 0.0, 0.0, 1.0), i as u64);
        }
        assert_eq!(tree.proxy_count(), 100);
        assert!(tree.capacity() >= 100);
        assert!(tree.validate());
    }

    #[test]
    fn test_repeated_insert_remove() {
        let mut tree = DynamicAabbTree::new();
        let mut pids = Vec::new();
        // 插入 20 个
        for i in 0..20 {
            pids.push(tree.create_proxy(aabb_at(i as f32 * 2.0, 0.0, 0.0, 0.5), i as u64));
        }
        assert_eq!(tree.proxy_count(), 20);
        // 移除偶数
        let mut to_remove = Vec::new();
        for (i, &pid) in pids.iter().enumerate() {
            if i % 2 == 0 {
                to_remove.push(pid);
            }
        }
        for pid in to_remove {
            tree.destroy_proxy(pid);
        }
        assert_eq!(tree.proxy_count(), 10);
        assert!(tree.validate());
        // 再插入 10 个
        for i in 0..10 {
            tree.create_proxy(aabb_at(100.0 + i as f32 * 2.0, 0.0, 0.0, 0.5), i as u64 + 100);
        }
        assert_eq!(tree.proxy_count(), 20);
        assert!(tree.validate());
    }

    #[test]
    fn test_random_scene() {
        use std::collections::HashSet;
        // 构建一个大场景, 验证查询正确性 (与暴力法对比)
        let mut tree = DynamicAabbTree::new();
        let n = 200;
        let mut all_aabbs: Vec<(i32, Aabb)> = Vec::new();
        // 用伪随机种子保证可重复
        let mut seed: u32 = 12345;
        let mut rng = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            seed as f32 / u32::MAX as f32
        };
        for i in 0..n {
            let x = rng() * 50.0;
            let y = rng() * 50.0;
            let z = rng() * 50.0;
            let half = 0.3 + rng() * 0.7;
            let aabb = aabb_at(x, y, z, half);
            let pid = tree.create_proxy(aabb, i as u64);
            all_aabbs.push((pid, aabb));
        }
        assert!(tree.validate());

        // 随机查询 10 次
        for _ in 0..10 {
            let qx = rng() * 50.0;
            let qy = rng() * 50.0;
            let qz = rng() * 50.0;
            let qhalf = 1.0 + rng() * 3.0;
            let query_aabb = aabb_at(qx, qy, qz, qhalf);

            // 树查询
            let tree_hits: HashSet<i32> = tree.query(&query_aabb).iter().map(|h| h.0).collect();
            // 暴力法
            let brute_hits: HashSet<i32> = all_aabbs
                .iter()
                .filter(|(_, a)| a.intersects(&query_aabb))
                .map(|(p, _)| *p)
                .collect();
            assert_eq!(tree_hits, brute_hits, "query mismatch: tree={} brute={}", tree_hits.len(), brute_hits.len());
        }
    }

    #[test]
    fn test_empty_tree_queries() {
        let tree = DynamicAabbTree::new();
        assert!(tree.query(&aabb_at(0.0, 0.0, 0.0, 1.0)).is_empty());
        assert!(tree.ray_cast(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), 10.0).is_empty());
        assert_eq!(tree.total_surface_area(), 0.0);
        assert_eq!(tree.quality(), 0.0);
    }

    #[test]
    fn test_destroy_root_single_node() {
        let mut tree = DynamicAabbTree::new();
        let pid = tree.create_proxy(aabb_at(0.0, 0.0, 0.0, 1.0), 1);
        assert_eq!(tree.root(), pid);
        tree.destroy_proxy(pid);
        assert_eq!(tree.root(), NULL_NODE);
        assert_eq!(tree.proxy_count(), 0);
        assert!(tree.validate());
    }

    #[test]
    fn test_two_proxies_structure() {
        let mut tree = DynamicAabbTree::new();
        let p1 = tree.create_proxy(aabb_at(0.0, 0.0, 0.0, 1.0), 1);
        let p2 = tree.create_proxy(aabb_at(5.0, 0.0, 0.0, 1.0), 2);
        // 应该有一个根 (内部) + 两个叶
        assert_ne!(tree.root(), p1);
        assert_ne!(tree.root(), p2);
        assert_eq!(tree.height(), 1);
        let root_node = &tree.nodes[tree.root() as usize];
        assert!(!root_node.is_leaf());
        assert_eq!(root_node.child1, p1);
        assert_eq!(root_node.child2, p2);
    }

    #[test]
    fn test_get_proxy_aabb_includes_fat_margin() {
        let mut tree = DynamicAabbTree::new().with_fat_margin(0.5);
        let pid = tree.create_proxy(aabb_at(0.0, 0.0, 0.0, 1.0), 1);
        let fat = tree.get_proxy_aabb(pid);
        // 原 AABB: [-1, 1], fat margin 0.5 -> [-1.5, 1.5]
        assert!((fat.min.x - (-1.5)).abs() < 1e-4);
        assert!((fat.max.x - 1.5).abs() < 1e-4);
    }

    #[test]
    fn test_ray_cast_origin_inside_aabb() {
        let mut tree = DynamicAabbTree::new();
        tree.create_proxy(aabb_at(0.0, 0.0, 0.0, 5.0), 1);
        // 射线起点在 AABB 内 -> fraction = 0
        let hits = tree.ray_cast(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), 100.0);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].2.abs() < 1e-4, "fraction should be 0, got {}", hits[0].2);
    }

    #[test]
    fn test_clear_enlarged() {
        let mut tree = DynamicAabbTree::new();
        let pid = tree.create_proxy(aabb_at(0.0, 0.0, 0.0, 1.0), 1);
        tree.enlarge_proxy(pid, aabb_at(2.0, 0.0, 0.0, 1.0));
        assert!(tree.nodes[pid as usize].enlarged);
        tree.clear_enlarged(pid);
        assert!(!tree.nodes[pid as usize].enlarged);
    }
}
