use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VramBudget {
    pub total_mb: u32,
    pub system_reserved_mb: u32,
    pub rendering_mb: u32,
    pub physics_mb: u32,
    pub npc_ai_mb: u32,
    pub combat_mb: u32,
    pub animation_mb: u32,
    pub textures_mb: u32,
    pub audio_mb: u32,
    pub scratch_mb: u32,
}

impl VramBudget {
    pub fn game_8gb() -> Self {
        VramBudget {
            total_mb: 8192,
            system_reserved_mb: 256,
            rendering_mb: 600,
            physics_mb: 2400,
            npc_ai_mb: 300,
            combat_mb: 80,
            animation_mb: 100,
            textures_mb: 2400,
            audio_mb: 80,
            scratch_mb: 200,
        }
    }

    pub fn engine_6gb() -> Self {
        VramBudget {
            total_mb: 6144,
            system_reserved_mb: 500,
            rendering_mb: 537,
            physics_mb: 1145,
            npc_ai_mb: 80,
            combat_mb: 50,
            animation_mb: 50,
            textures_mb: 1408,
            audio_mb: 40,
            scratch_mb: 200,
        }
    }

    pub fn used_mb(&self) -> u32 {
        self.system_reserved_mb
            + self.rendering_mb
            + self.physics_mb
            + self.npc_ai_mb
            + self.combat_mb
            + self.animation_mb
            + self.textures_mb
            + self.audio_mb
            + self.scratch_mb
    }

    pub fn remaining_mb(&self) -> u32 {
        self.total_mb.saturating_sub(self.used_mb())
    }

    pub fn physics_allocation(&self, subsystem: PhysicsSubsystem) -> u32 {
        match subsystem {
            PhysicsSubsystem::Mpm => self.physics_mb * 17 / 100,
            PhysicsSubsystem::Lbm => self.physics_mb * 43 / 100,
            PhysicsSubsystem::ReactionDiffusion => self.physics_mb * 18 / 100,
            PhysicsSubsystem::PhaseField => self.physics_mb * 5 / 100,
            PhysicsSubsystem::BackgroundGrid => self.physics_mb * 3 / 100,
            PhysicsSubsystem::Chemistry => self.physics_mb * 2 / 100,
            PhysicsSubsystem::Biology => self.physics_mb * 4 / 100,
            PhysicsSubsystem::Scratch => self.physics_mb * 8 / 100,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PhysicsSubsystem {
    Mpm,
    Lbm,
    ReactionDiffusion,
    PhaseField,
    BackgroundGrid,
    Chemistry,
    Biology,
    Scratch,
}

pub struct VramMonitor {
    budget: VramBudget,
    current_usage: std::collections::HashMap<String, u32>,
}

impl VramMonitor {
    pub fn new(budget: VramBudget) -> Self {
        VramMonitor { budget, current_usage: std::collections::HashMap::new() }
    }

    pub fn allocate(&mut self, category: &str, mb: u32) -> Result<(), String> {
        let current = self.current_usage.values().sum::<u32>();
        if current + mb > self.budget.total_mb {
            return Err(format!(
                "VRAM allocation failed: {}+{} > {} (remaining: {}MB)",
                current,
                mb,
                self.budget.total_mb,
                self.budget.total_mb.saturating_sub(current)
            ));
        }
        self.current_usage.insert(category.to_string(), mb);
        Ok(())
    }

    pub fn deallocate(&mut self, category: &str) {
        self.current_usage.remove(category);
    }

    pub fn usage_report(&self) -> String {
        let mut report = String::new();
        report.push_str(&format!("VRAM Usage Report ({}MB total):\n", self.budget.total_mb));
        let mut entries: Vec<_> = self.current_usage.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));
        for (cat, mb) in entries {
            report.push_str(&format!(
                "  {:30} {:6} MB ({:.1}%)\n",
                cat,
                mb,
                *mb as f32 / self.budget.total_mb as f32 * 100.0
            ));
        }
        let used: u32 = self.current_usage.values().sum();
        report.push_str(&format!(
            "  {:30} {:6} MB ({:.1}%)\n",
            "TOTAL USED",
            used,
            used as f32 / self.budget.total_mb as f32 * 100.0
        ));
        report.push_str(&format!(
            "  {:30} {:6} MB\n",
            "REMAINING",
            self.budget.total_mb.saturating_sub(used)
        ));
        report
    }
}
