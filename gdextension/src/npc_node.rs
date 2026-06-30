use godot::prelude::*;
use std::sync::Mutex;
use uuid::Uuid;
use ae_engine::{
    NpcCombatState, NpcSpecies, NpcSystem, create_default_npc_definition,
};

#[derive(GodotClass)]
#[class(base=Node3D)]
struct WastelandNPC {
    system: Mutex<Option<NpcSystem>>,

    #[var]
    max_npcs: i64,

    #[base]
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for WastelandNPC {
    fn init(base: Base<Node3D>) -> Self {
        Self { system: Mutex::new(None), max_npcs: 500, base }
    }

    fn process(&mut self, delta: f64) {
        if let Ok(mut guard) = self.system.lock() {
            if let Some(ref mut system) = *guard {
                system.update(delta as f32, 0.0);
            }
        }
    }
}

#[godot_api]
impl WastelandNPC {
    #[func]
    fn init_system(&mut self, max_npcs: i64) {
        self.max_npcs = max_npcs;
        *self.system.lock().unwrap() = Some(NpcSystem::new(max_npcs as usize));
    }

    #[func]
    fn spawn_npc(
        &mut self,
        name: GString,
        px: f32,
        py: f32,
        pz: f32,
        species: GString,
        faction: GString,
    ) -> GString {
        if let Ok(mut guard) = self.system.lock() {
            if let Some(ref mut system) = *guard {
                if self.max_npcs == 0 {
                    self.max_npcs = 500;
                    *system = NpcSystem::new(500);
                }
                let sp = match species.to_string().as_str() {
                    "human" => NpcSpecies::Human,
                    "mutant" => NpcSpecies::Mutant,
                    "ghoul" => NpcSpecies::Ghoul,
                    "robot" => NpcSpecies::Robot,
                    "animal" => NpcSpecies::Animal,
                    s if s.starts_with("custom_") => {
                        let id = s.trim_start_matches("custom_").parse::<u32>().unwrap_or(0);
                        NpcSpecies::Custom(id)
                    },
                    _ => NpcSpecies::Human,
                };
                let def = create_default_npc_definition(
                    &name.to_string(),
                    glam::Vec3::new(px, py, pz),
                    sp,
                    &faction.to_string(),
                );
                let id = def.id;
                system.queue_spawn(def);
                let id_str = id.to_string();
                return GString::from(id_str.as_str());
            }
        }
        GString::new()
    }

    #[func]
    fn despawn_npc(&mut self, npc_id: GString) {
        if let Ok(parsed) = Uuid::parse_str(&npc_id.to_string()) {
            if let Ok(mut guard) = self.system.lock() {
                if let Some(ref mut system) = *guard {
                    system.queue_despawn(parsed);
                }
            }
        }
    }

    #[func]
    fn apply_damage(&mut self, npc_id: GString, damage: f32, damage_type: GString) {
        if let Ok(parsed) = Uuid::parse_str(&npc_id.to_string()) {
            if let Ok(mut guard) = self.system.lock() {
                if let Some(ref mut system) = *guard {
                    system.apply_damage(parsed, damage, &damage_type.to_string());
                }
            }
        }
    }

    #[func]
    fn apply_force(&mut self, npc_id: GString, fx: f32, fy: f32, fz: f32) {
        if let Ok(parsed) = Uuid::parse_str(&npc_id.to_string()) {
            if let Ok(mut guard) = self.system.lock() {
                if let Some(ref mut system) = *guard {
                    system.apply_force(parsed, glam::Vec3::new(fx, fy, fz));
                }
            }
        }
    }

    #[func]
    fn process_dialogue(
        &mut self,
        npc_id: GString,
        player_message: GString,
        time_of_day: f32,
        weather: GString,
        player_reputation: f32,
    ) -> Dictionary<Variant, Variant> {
        if let Ok(parsed) = Uuid::parse_str(&npc_id.to_string()) {
            if let Ok(mut guard) = self.system.lock() {
                if let Some(ref mut system) = *guard {
                    if let Some(resp) = system.process_dialogue(
                        parsed,
                        &player_message.to_string(),
                        time_of_day,
                        &weather.to_string(),
                        player_reputation,
                    ) {
                        let resp_npc_id = GString::from(resp.npc_id.as_str());
                        let resp_text = GString::from(resp.text.as_str());
                        let resp_emotion = GString::from(resp.emotion.as_str());
                        let resp_action =
                            GString::from(resp.action.clone().unwrap_or_default().as_str());
                        return dict! {
                            "npc_id" => &resp_npc_id,
                            "text" => &resp_text,
                            "emotion" => &resp_emotion,
                            "action" => &resp_action,
                            "memory_updated" => resp.memory_updated,
                            "relationship_changed" => resp.relationship_changed,
                            "affinity_delta" => resp.affinity_delta,
                        };
                    }
                }
            }
        }
        dict! {}
    }

    #[func]
    fn inject_knowledge(
        &mut self,
        npc_id: GString,
        fact: GString,
        confidence: f32,
        source: GString,
    ) -> bool {
        if let Ok(parsed) = Uuid::parse_str(&npc_id.to_string()) {
            if let Ok(mut guard) = self.system.lock() {
                if let Some(ref mut system) = *guard {
                    use ae_ai_bridge::character_bridge::FactInjection;
                    let facts = vec![FactInjection {
                        topic: fact.to_string(),
                        content: fact.to_string(),
                        confidence,
                        source: source.to_string(),
                    }];
                    return system.inject_knowledge(parsed, facts);
                }
            }
        }
        false
    }

    #[func]
    fn update_emotion(&mut self, npc_id: GString, emotion: GString, intensity: f32) -> bool {
        if let Ok(parsed) = Uuid::parse_str(&npc_id.to_string()) {
            if let Ok(mut guard) = self.system.lock() {
                if let Some(ref mut system) = *guard {
                    return system.update_emotion(parsed, &emotion.to_string(), intensity);
                }
            }
        }
        false
    }

    #[func]
    fn get_npc_count(&self) -> i64 {
        if let Ok(guard) = self.system.lock() {
            if let Some(ref system) = *guard {
                return system.npc_count() as i64;
            }
        }
        0
    }

    #[func]
    fn get_total_npcs(&self) -> i64 {
        if let Ok(guard) = self.system.lock() {
            if let Some(ref system) = *guard {
                return system.total_npcs() as i64;
            }
        }
        0
    }

    #[func]
    fn get_npc_positions(&self) -> PackedVector3Array {
        let mut arr = PackedVector3Array::new();
        if let Ok(guard) = self.system.lock() {
            if let Some(ref system) = *guard {
                for pos in system.get_npc_positions() {
                    arr.push(Vector3::new(pos[0], pos[1], pos[2]));
                }
            }
        }
        arr
    }

    #[func]
    fn get_npc_colors(&self) -> PackedColorArray {
        let mut arr = PackedColorArray::new();
        if let Ok(guard) = self.system.lock() {
            if let Some(ref system) = *guard {
                for color in system.get_npc_colors() {
                    arr.push(Color::from_rgba(color[0], color[1], color[2], color[3]));
                }
            }
        }
        arr
    }

    #[func]
    fn get_npc_info(&self, npc_id: GString) -> Dictionary<Variant, Variant> {
        if let Ok(parsed) = Uuid::parse_str(&npc_id.to_string()) {
            if let Ok(guard) = self.system.lock() {
                if let Some(ref system) = *guard {
                    if let Some(npc) = system.get_npc(parsed) {
                        let combat_str = match npc.combat_state {
                            NpcCombatState::Idle => "idle",
                            NpcCombatState::Alert => "alert",
                            NpcCombatState::Combat => "combat",
                            NpcCombatState::Fleeing => "fleeing",
                            NpcCombatState::Dead => "dead",
                        };
                        let species_label = match npc.species {
                            NpcSpecies::Human => "human",
                            NpcSpecies::Mutant => "mutant",
                            NpcSpecies::Ghoul => "ghoul",
                            NpcSpecies::Robot => "robot",
                            NpcSpecies::Animal => "animal",
                            NpcSpecies::Custom(_) => "custom",
                        };
                        let name = GString::from(npc.name.as_str());
                        let species_gs = GString::from(species_label);
                        let faction_gs = GString::from(npc.faction.as_str());
                        let combat_gs = GString::from(combat_str);
                        let emotion_gs = GString::from(npc.emotion.dominant_emotion.as_str());
                        let goal_gs =
                            GString::from(npc.current_goal.clone().unwrap_or_default().as_str());
                        let action_gs =
                            GString::from(npc.current_action.clone().unwrap_or_default().as_str());
                        return dict! {
                            "name" => &name,
                            "position_x" => npc.position.x,
                            "position_y" => npc.position.y,
                            "position_z" => npc.position.z,
                            "health" => npc.health,
                            "max_health" => npc.max_health,
                            "alive" => npc.alive,
                            "species" => &species_gs,
                            "faction" => &faction_gs,
                            "combat_state" => &combat_gs,
                            "emotion" => &emotion_gs,
                            "emotion_intensity" => npc.emotion.intensity,
                            "stress_level" => npc.stress_level,
                            "radiation_dose" => npc.radiation_dose,
                            "toxin_level" => npc.toxin_level,
                            "current_goal" => &goal_gs,
                            "current_action" => &action_gs,
                        };
                    }
                }
            }
        }
        dict! {}
    }

    #[func]
    fn get_npcs_by_faction(&self, faction: GString) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        if let Ok(guard) = self.system.lock() {
            if let Some(ref system) = *guard {
                for npc in system.get_npcs_by_faction(&faction.to_string()) {
                    let id_str = npc.id.to_string();
                    arr.push(id_str.as_str());
                }
            }
        }
        arr
    }

    #[func]
    fn get_npcs_in_radius(&self, cx: f32, cy: f32, cz: f32, radius: f32) -> PackedStringArray {
        let mut arr = PackedStringArray::new();
        if let Ok(guard) = self.system.lock() {
            if let Some(ref system) = *guard {
                for npc in system.get_npcs_in_radius(glam::Vec3::new(cx, cy, cz), radius) {
                    let id_str = npc.id.to_string();
                    arr.push(id_str.as_str());
                }
            }
        }
        arr
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        if let Ok(guard) = self.system.lock() {
            if let Some(ref system) = *guard {
                let stats = system.stats();
                return dict! {
                    "total" => stats.total as i64,
                    "alive" => stats.alive as i64,
                    "dead" => stats.dead as i64,
                    "faction_count" => stats.faction_count as i64,
                    "pending_spawn" => stats.pending_spawn as i64,
                    "pending_despawn" => stats.pending_despawn as i64,
                    "interactions" => stats.interactions as i64,
                    "bridge_npcs" => stats.bridge_npcs as i64,
                };
            }
        }
        dict! {}
    }

    #[func]
    fn set_npc_behavior(
        &mut self,
        npc_id: GString,
        behavior_type: GString,
        target_x: f32,
        target_y: f32,
        target_z: f32,
    ) -> bool {
        if let Ok(parsed) = Uuid::parse_str(&npc_id.to_string()) {
            if let Ok(mut guard) = self.system.lock() {
                if let Some(ref mut system) = *guard {
                    if let Some(npc) = system.get_npc_mut(parsed) {
                        npc.current_action = Some(behavior_type.to_string());
                        let behavior_str = behavior_type.to_string();
                        npc.current_goal = Some(format!(
                            "{}_at_{:.1}_{:.1}_{:.1}",
                            behavior_str,
                            target_x,
                            target_y,
                            target_z
                        ));
                        return true;
                    }
                }
            }
        }
        false
    }

    #[func]
    fn get_npc_memories(&self, npc_id: GString) -> Array<Variant> {
        let mut arr = Array::new();
        if let Ok(parsed) = Uuid::parse_str(&npc_id.to_string()) {
            if let Ok(guard) = self.system.lock() {
                if let Some(ref system) = *guard {
                    if let Some(npc) = system.get_npc(parsed) {
                        let d: Dictionary<Variant, Variant> = dict! {
                            "content" => format!("npc_{}_memory", npc.name).as_str(),
                            "importance" => 0.5f32,
                            "timestamp" => 0.0f32,
                            "emotion_tag" => npc.emotion.dominant_emotion.as_str(),
                        };
                        arr.push(&d);
                    }
                }
            }
        }
        arr
    }

    #[func]
    fn get_npc_relationships(&self, npc_id: GString) -> Dictionary<Variant, Variant> {
        if let Ok(parsed) = Uuid::parse_str(&npc_id.to_string()) {
            if let Ok(guard) = self.system.lock() {
                if let Some(ref system) = *guard {
                    if let Some(npc) = system.get_npc(parsed) {
                        let faction_gs = GString::from(npc.faction.as_str());
                        let inner: Dictionary<Variant, Variant> = dict! {
                            "affinity" => 0.5f32,
                            "trust" => 0.5f32,
                            "fear" => npc.stress_level,
                            "respect" => 0.5f32,
                        };
                        return dict! {
                            &faction_gs => &inner,
                        };
                    }
                }
            }
        }
        dict! {}
    }

    #[func]
    fn set_npc_goal(&mut self, npc_id: GString, goal_name: GString, priority: i64) -> bool {
        if let Ok(parsed) = Uuid::parse_str(&npc_id.to_string()) {
            if let Ok(mut guard) = self.system.lock() {
                if let Some(ref mut system) = *guard {
                    if let Some(npc) = system.get_npc_mut(parsed) {
                        npc.current_goal = Some(goal_name.to_string());
                        npc.emotion.intensity = (priority as f32 / 10.0).clamp(0.0, 1.0);
                        return true;
                    }
                }
            }
        }
        false
    }
}
