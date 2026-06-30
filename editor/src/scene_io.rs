//! Scene I/O: load and save scene files as JSON.
//!
//! Supports:
//! - Native `.ae` JSON scene format
//! - glTF import stub (placeholder for P1)

use crate::scene::Scene;
use anyhow::{Context, Result};
use std::path::Path;

/// Serialise the scene to a JSON string.
pub fn serialise_scene(scene: &Scene) -> Result<String> {
    serde_json::to_string_pretty(scene).context("Failed to serialise scene")
}

/// Deserialise a scene from a JSON string.
pub fn deserialise_scene(json: &str) -> Result<Scene> {
    serde_json::from_str(json).context("Failed to deserialise scene")
}

/// Load a scene from a file path.
pub fn load_scene(path: &Path) -> Result<Scene> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read scene file: {}", path.display()))?;
    deserialise_scene(&data)
}

/// Save a scene to a file path.
pub fn save_scene(scene: &Scene, path: &Path) -> Result<()> {
    let json = serialise_scene(scene)?;
    std::fs::write(path, &json)
        .with_context(|| format!("Failed to write scene file: {}", path.display()))?;
    log::info!("Scene saved to {}", path.display());
    Ok(())
}

/// Import a glTF file into a scene (placeholder for P1).
///
/// Currently returns an error indicating the feature is not yet implemented.
/// This will be expanded in P1 with full glTF parsing and material import.
pub fn import_gltf_to_scene(_path: &Path) -> Result<Scene> {
    anyhow::bail!("glTF import is not yet implemented (planned for P1)")
}
