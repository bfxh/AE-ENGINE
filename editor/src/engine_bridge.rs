//! Editor ↔ Engine bridge module.
//!
//! Provides the interface between the editor's scene representation
//! and the Wasteland Engine's GameWorld.
//!
//! Currently uses lightweight mirror types (`engine_types`) since the
//! engine crate has a heavy dependency graph (40+ crates).
//! When the engine compiles cleanly, this module can be extended
//! to call into `wasteland_engine` directly.

use crate::engine_types::*;

/// Manages the connection between editor scene and engine world.
#[derive(Debug, Clone)]
pub struct EngineBridge {
    /// Configuration.
    pub config: EngineBridgeConfig,
    /// Latest snapshot of engine state (for UI display).
    pub snapshot: Option<EngineWorldSnapshot>,
    /// Active entity links.
    pub links: Vec<EngineLink>,
    /// Internal frame counter for snapshot interval.
    frame_count: u32,
}

impl EngineBridge {
    /// Create a new engine bridge.
    pub fn new() -> Self {
        Self {
            config: EngineBridgeConfig::default(),
            snapshot: None,
            links: Vec::new(),
            frame_count: 0,
        }
    }

    /// Called each frame to update engine state.
    ///
    /// In the full implementation, this would poll or receive updates
    /// from the engine's tick loop. Currently generates a synthetic
    /// snapshot for UI development.
    pub fn tick(&mut self, frame_counter: u64) {
        self.frame_count = self.frame_count.wrapping_add(1);

        if self.frame_count.is_multiple_of(self.config.snapshot_interval) {
            self.snapshot = Some(self.generate_synthetic_snapshot(frame_counter));
        }
    }

    /// Generate a synthetic snapshot for editor UI testing.
    fn generate_synthetic_snapshot(&self, frame: u64) -> EngineWorldSnapshot {
        let t = frame as f64 * 0.016; // ~60fps -> seconds
        EngineWorldSnapshot {
            sim_time: t,
            time_scale: 1.0,
            paused: false,
            tick_count: frame,
            // Simulate weather cycling.
            global_temperature: 22.0 + (t as f32 * 0.01).sin() * 15.0,
            global_radiation: 0.01 + (t as f32 * 0.005).cos().abs() * 0.05,
            entity_counts: EngineEntityCounts {
                physics_bodies: 50 + (t as f32 * 0.3).sin() as usize * 5,
                chemistry_entities: 12,
                ecosystems: 3,
                particles: 1000 + (t as f32 * 2.0).sin() as usize * 200,
                meta_entities: 42,
                npcs: 8,
                audio_sources: 4,
                weather_systems: 1,
            },
            world_bounds: WorldBounds::default(),
        }
    }

    /// Link an editor scene node to an engine entity.
    pub fn link_entity(&mut self, node_id: u64, engine_id: &str, engine_type: &str) {
        self.links.push(EngineLink {
            node_id,
            engine_id: engine_id.to_string(),
            engine_type: engine_type.to_string(),
            meta: Vec::new(),
        });
    }

    /// Get engine links for a specific node.
    pub fn links_for_node(&self, node_id: u64) -> Vec<&EngineLink> {
        self.links.iter().filter(|l| l.node_id == node_id).collect()
    }
}

impl Default for EngineBridge {
    fn default() -> Self {
        Self::new()
    }
}
