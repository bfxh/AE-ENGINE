//! Lightweight mirror types for the Wasteland Engine.
//!
//! These types mirror the `ae_engine` data structures without
//! depending on the heavy engine crate (40+ dependencies).
//! The editor uses these for display and light interaction.
//!
//! When the engine compiles successfully, a `engine_bridge.rs` can map
//! between these and the real engine types.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Lightweight projection of the engine's GameWorld.
///
/// Only includes fields relevant to the editor UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineWorldSnapshot {
    /// Simulation time in seconds.
    pub sim_time: f64,
    /// Time scale multiplier.
    pub time_scale: f32,
    /// Whether the simulation is paused.
    pub paused: bool,
    /// Number of ticks executed.
    pub tick_count: u64,
    /// World temperature (global).
    pub global_temperature: f32,
    /// World radiation level (global).
    pub global_radiation: f32,

    /// Entity counts by system.
    pub entity_counts: EngineEntityCounts,

    /// World bounds.
    pub world_bounds: WorldBounds,
}

/// Counts of entities across engine subsystems.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EngineEntityCounts {
    pub physics_bodies: usize,
    pub chemistry_entities: usize,
    pub ecosystems: usize,
    pub particles: usize,
    pub meta_entities: usize,
    pub npcs: usize,
    pub audio_sources: usize,
    pub weather_systems: usize,
}

/// World boundary definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldBounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl Default for WorldBounds {
    fn default() -> Self {
        Self { min: Vec3::new(-1000.0, -100.0, -1000.0), max: Vec3::new(1000.0, 500.0, 1000.0) }
    }
}

impl Default for EngineWorldSnapshot {
    fn default() -> Self {
        Self {
            sim_time: 0.0,
            time_scale: 1.0,
            paused: false,
            tick_count: 0,
            global_temperature: 22.0,
            global_radiation: 0.01,
            entity_counts: EngineEntityCounts::default(),
            world_bounds: WorldBounds::default(),
        }
    }
}

/// Association between a scene node and an engine entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineLink {
    /// Scene node id.
    pub node_id: u64,
    /// Engine entity identifier (could be a UUID or index).
    pub engine_id: String,
    /// Human-readable type label.
    pub engine_type: String,
    /// Additional metadata.
    pub meta: Vec<(String, String)>,
}

/// Engine ↔ Editor bridge configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineBridgeConfig {
    /// Whether the engine bridge is enabled.
    pub enabled: bool,
    /// Interval in frames between engine snapshots.
    pub snapshot_interval: u32,
    /// Whether to auto-link engine entities to scene nodes.
    pub auto_link: bool,
}

impl Default for EngineBridgeConfig {
    fn default() -> Self {
        Self { enabled: true, snapshot_interval: 60, auto_link: true }
    }
}
