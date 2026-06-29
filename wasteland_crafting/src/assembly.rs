use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::blueprint::{Blueprint, BlueprintLibrary};
use crate::socket::SocketConnection;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssemblyResult {
    pub success: bool,
    pub entity_id: Option<Uuid>,
    pub entity_type: String,
    pub quality: f32,
    pub derived_functions: Vec<FunctionScore>,
    pub socket_fill_status: Vec<Option<Uuid>>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionScore {
    pub function: String,
    pub confidence: f32,
    pub contributing_sockets: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssemblySession {
    pub session_id: Uuid,
    pub blueprint_id: Uuid,
    pub filled_sockets: Vec<SocketConnection>,
    pub start_time: u64,
    pub last_modified: u64,
    pub undo_stack: Vec<AssemblySnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssemblySnapshot {
    pub connections: Vec<SocketConnection>,
    pub timestamp: u64,
}

impl AssemblySession {
    pub fn new(blueprint_id: Uuid, tick: u64) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            blueprint_id,
            filled_sockets: Vec::new(),
            start_time: tick,
            last_modified: tick,
            undo_stack: Vec::new(),
        }
    }

    pub fn attach_part(
        &mut self,
        socket_index: usize,
        part_id: Uuid,
        part_tags: &[String],
        tick: u64,
    ) -> bool {
        self.save_snapshot(tick);

        if let Some(existing) =
            self.filled_sockets.iter_mut().find(|c| c.socket_index == socket_index)
        {
            existing.part_id = part_id;
            existing.part_tags = part_tags.to_vec();
        } else {
            self.filled_sockets.push(SocketConnection {
                socket_index,
                part_id,
                part_tags: part_tags.to_vec(),
                locked: false,
            });
        }

        self.last_modified = tick;
        true
    }

    pub fn detach_part(&mut self, socket_index: usize, tick: u64) -> bool {
        self.save_snapshot(tick);

        let len_before = self.filled_sockets.len();
        self.filled_sockets.retain(|c| c.socket_index != socket_index);

        self.last_modified = tick;
        self.filled_sockets.len() < len_before
    }

    pub fn undo(&mut self) -> bool {
        if let Some(snapshot) = self.undo_stack.pop() {
            self.filled_sockets = snapshot.connections;
            self.last_modified = snapshot.timestamp;
            true
        } else {
            false
        }
    }

    fn save_snapshot(&mut self, tick: u64) {
        self.undo_stack
            .push(AssemblySnapshot { connections: self.filled_sockets.clone(), timestamp: tick });

        if self.undo_stack.len() > 50 {
            self.undo_stack.remove(0);
        }
    }

    pub fn get_filled_indices(&self) -> Vec<usize> {
        self.filled_sockets.iter().map(|c| c.socket_index).collect()
    }

    pub fn is_socket_filled(&self, index: usize) -> bool {
        self.filled_sockets.iter().any(|c| c.socket_index == index)
    }
}

pub struct AssemblySystem {
    pub blueprint_library: BlueprintLibrary,
    pub active_sessions: Vec<AssemblySession>,
    pub assembly_history: Vec<AssemblyResult>,
}

impl AssemblySystem {
    pub fn new() -> Self {
        Self {
            blueprint_library: BlueprintLibrary::new(),
            active_sessions: Vec::new(),
            assembly_history: Vec::new(),
        }
    }

    pub fn start_assembly(&mut self, blueprint_id: Uuid, tick: u64) -> Option<usize> {
        let blueprint = self.blueprint_library.find_by_id(blueprint_id)?;
        let session = AssemblySession::new(blueprint.id, tick);
        let index = self.active_sessions.len();
        self.active_sessions.push(session);
        Some(index)
    }

    pub fn assemble(
        &mut self,
        session_index: usize,
        blueprint: &Blueprint,
        part_registry: &[(Uuid, &[String], f32)],
        _tick: u64,
    ) -> AssemblyResult {
        if session_index >= self.active_sessions.len() {
            return AssemblyResult {
                success: false,
                entity_id: None,
                entity_type: String::new(),
                quality: 0.0,
                derived_functions: Vec::new(),
                socket_fill_status: Vec::new(),
                warnings: Vec::new(),
                errors: vec!["Invalid session index".to_string()],
            };
        }

        let session = &self.active_sessions[session_index];
        let fill_status: Vec<Option<usize>> = (0..blueprint.sockets.len())
            .map(|i| {
                session
                    .filled_sockets
                    .iter()
                    .find(|c| c.socket_index == i)
                    .and_then(|c| part_registry.iter().position(|(id, _, _)| *id == c.part_id))
            })
            .collect();

        let validation = blueprint.validate(&fill_status);
        if !validation.valid {
            return AssemblyResult {
                success: false,
                entity_id: None,
                entity_type: String::new(),
                quality: 0.0,
                derived_functions: Vec::new(),
                socket_fill_status: Vec::new(),
                warnings: validation.warnings,
                errors: validation.errors,
            };
        }

        let mut total_quality = 0.0f32;
        let mut count = 0u32;
        for conn in &session.filled_sockets {
            if let Some((_, _, quality)) =
                part_registry.iter().find(|(id, _, _)| *id == conn.part_id)
            {
                total_quality += quality;
                count += 1;
            }
        }
        let avg_quality = if count > 0 { total_quality / count as f32 } else { 0.5 };

        let derived_functions = Self::derive_functions(blueprint, &session.filled_sockets);

        let result = AssemblyResult {
            success: true,
            entity_id: Some(Uuid::new_v4()),
            entity_type: blueprint.name.clone(),
            quality: avg_quality,
            derived_functions,
            socket_fill_status: session.filled_sockets.iter().map(|c| Some(c.part_id)).collect(),
            warnings: validation.warnings,
            errors: Vec::new(),
        };

        self.assembly_history.push(result.clone());

        if let Some(ref mut bp) =
            self.blueprint_library.blueprints.iter_mut().find(|b| b.id == blueprint.id)
        {
            bp.usage_count += 1;
        }

        result
    }

    fn derive_functions(
        blueprint: &Blueprint,
        connections: &[SocketConnection],
    ) -> Vec<FunctionScore> {
        let mut functions = Vec::new();

        let has_blade = connections.iter().any(|c| {
            let socket = blueprint.sockets.get(c.socket_index);
            socket.map(|s| matches!(s.slot_type, crate::socket::SlotType::Blade)).unwrap_or(false)
        });

        let has_handle = connections.iter().any(|c| {
            let socket = blueprint.sockets.get(c.socket_index);
            socket.map(|s| matches!(s.slot_type, crate::socket::SlotType::Handle)).unwrap_or(false)
        });

        if has_blade && has_handle {
            functions.push(FunctionScore {
                function: "Cutting".to_string(),
                confidence: 0.85,
                contributing_sockets: connections.iter().map(|c| c.socket_index).collect(),
            });
        }

        if has_blade {
            functions.push(FunctionScore {
                function: "Piercing".to_string(),
                confidence: 0.6,
                contributing_sockets: connections.iter().map(|c| c.socket_index).collect(),
            });
        }

        let has_barrel = connections.iter().any(|c| {
            let socket = blueprint.sockets.get(c.socket_index);
            socket.map(|s| matches!(s.slot_type, crate::socket::SlotType::Barrel)).unwrap_or(false)
        });

        if has_barrel {
            functions.push(FunctionScore {
                function: "Projectile".to_string(),
                confidence: 0.9,
                contributing_sockets: connections.iter().map(|c| c.socket_index).collect(),
            });
        }

        functions
    }

    pub fn dismantle(&self, session_index: usize) -> Vec<Uuid> {
        if session_index >= self.active_sessions.len() {
            return Vec::new();
        }

        self.active_sessions[session_index].filled_sockets.iter().map(|c| c.part_id).collect()
    }
}

impl Default for AssemblySystem {
    fn default() -> Self {
        Self::new()
    }
}
