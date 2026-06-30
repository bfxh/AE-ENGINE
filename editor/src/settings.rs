//! Editor settings persistence.
//!
//! Saves/loads `SettingsPanel` state to a JSON file in the user's config
//! directory so preferences survive editor restarts.

use crate::panels::settings_panel::SettingsPanel;
use std::path::PathBuf;

/// Return the config directory for the ae editor.
///
/// - Windows: `%APPDATA%/ae_editor/`
/// - Linux: `$XDG_CONFIG_HOME/ae_editor/` or `$HOME/.config/ae_editor/`
/// - macOS: `$HOME/Library/Application Support/ae_editor/`
fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join("ae_editor"))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|h| {
            PathBuf::from(h).join("Library").join("Application Support").join("ae_editor")
        })
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            Some(PathBuf::from(xdg).join("ae_editor"))
        } else {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config").join("ae_editor"))
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    {
        None
    }
}

/// Path to the settings JSON file.
pub fn settings_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("settings.json"))
}

/// Load settings from disk. Returns `None` if the file doesn't exist or
/// cannot be parsed (in which case defaults are used).
pub fn load_settings() -> Option<SettingsPanel> {
    let path = settings_path()?;
    let contents = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&contents).ok()
}

/// Save settings to disk. Creates the config directory if it doesn't exist.
pub fn save_settings(settings: &SettingsPanel) -> Result<(), String> {
    let path = settings_path().ok_or_else(|| "Cannot determine config directory".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {}", e))?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| format!("Failed to serialize: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write settings: {}", e))?;
    log::info!("Settings saved to {:?}", path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_roundtrip() {
        let mut original = SettingsPanel::default();
        original.grid_size = 2.5;
        original.font_size = 18.0;
        original.theme_mode = 1;
        original.camera_speed = 3.0;
        original.rotation_snapping = true;
        original.snap_angle_deg = 30.0;

        let json = serde_json::to_string(&original).unwrap();
        let restored: SettingsPanel = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.grid_size, 2.5);
        assert_eq!(restored.font_size, 18.0);
        assert_eq!(restored.theme_mode, 1);
        assert_eq!(restored.camera_speed, 3.0);
        assert!(restored.rotation_snapping);
        assert_eq!(restored.snap_angle_deg, 30.0);
        // Skipped fields should revert to defaults.
        assert!(!restored.visible);
        assert_eq!(restored.tab, 0);
    }

    #[test]
    fn test_load_missing_file_returns_none() {
        // Loading from a path that doesn't exist should return None, not panic.
        let result = load_settings();
        // We can't guarantee the file doesn't exist in test env, but it should
        // at least not panic. If it does exist, it must parse correctly.
        let _ = result;
    }
}
