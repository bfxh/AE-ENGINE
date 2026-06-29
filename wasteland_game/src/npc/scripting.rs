use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcScript {
    pub id: u32,
    pub name: String,
    pub triggers: Vec<ScriptTrigger>,
    pub actions: Vec<ScriptAction>,
    pub conditions: Vec<ScriptCondition>,
    pub repeatable: bool,
    pub cooldown: f32,
    pub last_triggered: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScriptTrigger {
    OnSpawn,
    OnDeath,
    OnDamaged { threshold: f32 },
    OnEnemyInRange { distance: f32 },
    OnAllyKilled,
    OnTimeOfDay { hour: u8 },
    OnPlayerNearby { distance: f32 },
    OnHealthBelow { percentage: f32 },
    OnStateChange { from: String, to: String },
    OnTimer { interval: f32 },
    OnScriptEvent { event_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScriptAction {
    MoveTo(Vec3),
    RunTo(Vec3),
    AttackNearest { max_distance: f32 },
    FleeFrom(Vec3),
    SayDialogue { line_id: u32 },
    PlayAnimation { anim_id: u8 },
    ChangeState { new_state: String },
    SpawnNpc { npc_type: String, position: Vec3 },
    DespawnSelf,
    GiveItem { item_id: u32 },
    SetFaction { faction: u8 },
    CallReinforcements { count: u8, radius: f32 },
    TriggerEvent { event_name: String },
    Wait { duration: f32 },
    PatrolTo { point_index: usize },
    GuardPosition { position: Vec3, radius: f32 },
    SearchArea { center: Vec3, radius: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScriptCondition {
    HealthAbove { percentage: f32 },
    HealthBelow { percentage: f32 },
    HasEnemy,
    NoEnemy,
    AllyCountAbove { count: u8 },
    AllyCountBelow { count: u8 },
    TimeOfDayBetween { start: u8, end: u8 },
    DistanceToPlayer { comparison: ComparisonOp, distance: f32 },
    InCombat,
    NotInCombat,
    CustomFlag { flag: String, value: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ComparisonOp {
    Less,
    Greater,
    Equal,
    LessEqual,
    GreaterEqual,
}

pub struct ScriptSystem {
    pub scripts: HashMap<u32, NpcScript>,
    pub npc_script_assignments: HashMap<u64, Vec<u32>>,
    pub npc_flags: HashMap<u64, HashMap<String, bool>>,
    pub global_flags: HashMap<String, bool>,
    pub next_script_id: u32,
}

impl Default for ScriptSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptSystem {
    pub fn new() -> Self {
        ScriptSystem {
            scripts: HashMap::new(),
            npc_script_assignments: HashMap::new(),
            npc_flags: HashMap::new(),
            global_flags: HashMap::new(),
            next_script_id: 1,
        }
    }

    pub fn register_script(&mut self, script: NpcScript) -> u32 {
        let id = script.id;
        self.scripts.insert(id, script);
        if id >= self.next_script_id {
            self.next_script_id = id + 1;
        }
        id
    }

    pub fn assign_script(&mut self, npc_id: u64, script_id: u32) {
        self.npc_script_assignments.entry(npc_id).or_default().push(script_id);
    }

    pub fn set_npc_flag(&mut self, npc_id: u64, flag: &str, value: bool) {
        self.npc_flags.entry(npc_id).or_default().insert(flag.to_string(), value);
    }

    pub fn get_npc_flag(&self, npc_id: u64, flag: &str) -> bool {
        self.npc_flags.get(&npc_id).and_then(|f| f.get(flag)).copied().unwrap_or(false)
    }

    pub fn evaluate_scripts(&self, npc_id: u64, npc_state: &ScriptNpcContext) -> Vec<ScriptAction> {
        let mut actions = Vec::new();
        if let Some(script_ids) = self.npc_script_assignments.get(&npc_id) {
            for &sid in script_ids {
                if let Some(script) = self.scripts.get(&sid) {
                    if script.last_triggered > 0.0 && script.last_triggered < script.cooldown {
                        continue;
                    }
                    if !script.repeatable && script.last_triggered > 0.0 {
                        continue;
                    }
                    let triggered =
                        script.triggers.iter().any(|t| self.check_trigger(t, npc_state));
                    if !triggered {
                        continue;
                    }
                    let conditions_met = script
                        .conditions
                        .iter()
                        .all(|c| self.check_condition(c, npc_state, npc_id));
                    if conditions_met {
                        actions.extend(script.actions.clone());
                    }
                }
            }
        }
        actions
    }

    fn check_trigger(&self, trigger: &ScriptTrigger, ctx: &ScriptNpcContext) -> bool {
        match trigger {
            ScriptTrigger::OnSpawn => ctx.just_spawned,
            ScriptTrigger::OnDeath => ctx.just_died,
            ScriptTrigger::OnDamaged { threshold } => ctx.last_damage >= *threshold,
            ScriptTrigger::OnEnemyInRange { distance } => ctx.nearest_enemy_distance <= *distance,
            ScriptTrigger::OnAllyKilled => ctx.ally_just_killed,
            ScriptTrigger::OnTimeOfDay { hour } => ctx.current_hour == *hour,
            ScriptTrigger::OnPlayerNearby { distance } => ctx.player_distance <= *distance,
            ScriptTrigger::OnHealthBelow { percentage } => ctx.health_percentage <= *percentage,
            ScriptTrigger::OnStateChange { from, to } => {
                ctx.previous_state == *from && ctx.current_state == *to
            },
            ScriptTrigger::OnTimer { interval } => ctx.time_alive % interval < 0.1,
            ScriptTrigger::OnScriptEvent { event_name } => {
                self.global_flags.get(event_name).copied().unwrap_or(false)
            },
        }
    }

    fn check_condition(&self, cond: &ScriptCondition, ctx: &ScriptNpcContext, npc_id: u64) -> bool {
        match cond {
            ScriptCondition::HealthAbove { percentage } => ctx.health_percentage > *percentage,
            ScriptCondition::HealthBelow { percentage } => ctx.health_percentage <= *percentage,
            ScriptCondition::HasEnemy => ctx.has_enemy,
            ScriptCondition::NoEnemy => !ctx.has_enemy,
            ScriptCondition::AllyCountAbove { count } => ctx.ally_count > *count,
            ScriptCondition::AllyCountBelow { count } => ctx.ally_count < *count,
            ScriptCondition::TimeOfDayBetween { start, end } => {
                if start <= end {
                    ctx.current_hour >= *start && ctx.current_hour <= *end
                } else {
                    ctx.current_hour >= *start || ctx.current_hour <= *end
                }
            },
            ScriptCondition::DistanceToPlayer { comparison, distance } => match comparison {
                ComparisonOp::Less => ctx.player_distance < *distance,
                ComparisonOp::Greater => ctx.player_distance > *distance,
                ComparisonOp::Equal => (ctx.player_distance - distance).abs() < 1.0,
                ComparisonOp::LessEqual => ctx.player_distance <= *distance,
                ComparisonOp::GreaterEqual => ctx.player_distance >= *distance,
            },
            ScriptCondition::InCombat => ctx.in_combat,
            ScriptCondition::NotInCombat => !ctx.in_combat,
            ScriptCondition::CustomFlag { flag, value } => {
                self.get_npc_flag(npc_id, flag) == *value
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScriptNpcContext {
    pub npc_id: u64,
    pub position: Vec3,
    pub health_percentage: f32,
    pub has_enemy: bool,
    pub in_combat: bool,
    pub nearest_enemy_distance: f32,
    pub player_distance: f32,
    pub ally_count: u8,
    pub current_hour: u8,
    pub current_state: String,
    pub previous_state: String,
    pub just_spawned: bool,
    pub just_died: bool,
    pub ally_just_killed: bool,
    pub last_damage: f32,
    pub time_alive: f32,
}
