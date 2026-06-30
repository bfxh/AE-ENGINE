use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBudget {
    pub total_mb: u32,
    pub physics_mb: u32,
    pub chemistry_mb: u32,
    pub biology_mb: u32,
    pub npc_ai_mb: u32,
    pub combat_mb: u32,
    pub navigation_mb: u32,
    pub animation_mb: u32,
    pub interaction_mb: u32,
    pub editor_mb: u32,
    pub scripting_mb: u32,
    pub audio_mb: u32,
    pub networking_mb: u32,
    pub scratch_mb: u32,
}

impl MemoryBudget {
    pub fn game_10gb() -> Self {
        MemoryBudget {
            total_mb: 10240,
            physics_mb: 1500,    // MPM 300万粒子 + LBM + 场
            chemistry_mb: 500,   // 反应系统
            biology_mb: 500,     // 生态系统
            npc_ai_mb: 800,      // 300 NPC + 感知 + 记忆
            combat_mb: 300,      // 战斗系统
            navigation_mb: 500,  // NavMesh + ORCA
            animation_mb: 400,   // 骨骼 + VAT
            interaction_mb: 600, // 交互系统(优化后)
            editor_mb: 800,      // 游戏内建造编辑器
            scripting_mb: 300,   // NPC脚本系统
            audio_mb: 200,
            networking_mb: 200,
            scratch_mb: 1000, // 临时缓冲
        }
    }

    pub fn used_mb(&self) -> u32 {
        self.physics_mb
            + self.chemistry_mb
            + self.biology_mb
            + self.npc_ai_mb
            + self.combat_mb
            + self.navigation_mb
            + self.animation_mb
            + self.interaction_mb
            + self.editor_mb
            + self.scripting_mb
            + self.audio_mb
            + self.networking_mb
            + self.scratch_mb
    }

    pub fn remaining_mb(&self) -> u32 {
        self.total_mb.saturating_sub(self.used_mb())
    }
}

pub struct MemoryMonitor {
    budget: MemoryBudget,
    allocations: HashMap<String, u32>,
    peak_usage: u32,
}

impl MemoryMonitor {
    pub fn new(budget: MemoryBudget) -> Self {
        MemoryMonitor { budget, allocations: HashMap::new(), peak_usage: 0 }
    }

    pub fn allocate(&mut self, category: &str, mb: u32) -> Result<(), String> {
        let total: u32 = self.allocations.values().sum();
        if total + mb > self.budget.total_mb {
            return Err(format!(
                "Memory allocation failed: {}+{} > {}MB",
                total, mb, self.budget.total_mb
            ));
        }
        let current = self.allocations.entry(category.to_string()).or_insert(0);
        *current += mb;
        let new_total = self.allocations.values().sum::<u32>();
        if new_total > self.peak_usage {
            self.peak_usage = new_total;
        }
        Ok(())
    }

    pub fn deallocate(&mut self, category: &str, mb: u32) {
        if let Some(val) = self.allocations.get_mut(category) {
            *val = val.saturating_sub(mb);
            if *val == 0 {
                self.allocations.remove(category);
            }
        }
    }

    pub fn report(&self) -> String {
        let used = self.allocations.values().sum::<u32>();
        format!(
            "Memory: {}/{}MB used, {}MB peak, {}MB free",
            used,
            self.budget.total_mb,
            self.peak_usage,
            self.budget.total_mb.saturating_sub(used)
        )
    }
}
