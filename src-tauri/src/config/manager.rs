//! Settings file I/O and initialization

use std::path::PathBuf;

use crate::error::AppError;

use super::presets::get_builtin_presets;
use super::schema::AppSettings;

const SETTINGS_FILE: &str = ".pisum-transcript.json";

/// Get the settings file path (~/.pisum-transcript.json)
fn settings_path() -> Result<PathBuf, AppError> {
    let home = dirs::home_dir()
        .ok_or_else(|| AppError::Config("Could not determine home directory".to_string()))?;
    Ok(home.join(SETTINGS_FILE))
}

/// Initialize config: create defaults if file doesn't exist.
/// Returns `true` if this is a first launch (file was just created).
pub fn init() -> Result<bool, AppError> {
    let path = settings_path()?;

    if path.exists() {
        return Ok(false);
    }

    let defaults = AppSettings::default();
    save_settings(&defaults)?;
    Ok(true)
}

/// Load settings from disk, merging in any missing built-in presets.
pub fn load_settings() -> Result<AppSettings, AppError> {
    let path = settings_path()?;
    let contents = std::fs::read_to_string(&path)
        .map_err(|e| AppError::Config(format!("Failed to read settings: {}", e)))?;

    let mut settings: AppSettings = serde_json::from_str(&contents)
        .map_err(|e| AppError::Config(format!("Failed to parse settings: {}", e)))?;

    // Merge built-in presets: add any that are missing
    let builtins = get_builtin_presets();
    for builtin in &builtins {
        if !settings.presets.iter().any(|p| p.id == builtin.id) {
            settings.presets.push(builtin.clone());
        }
    }

    // Validate active_preset_id: fall back to first built-in if invalid
    if !settings.presets.iter().any(|p| p.id == settings.active_preset_id) {
        let fallback_id = builtins
            .first()
            .map(|p| p.id.clone())
            .unwrap_or_else(|| "de-transcribe".to_string());
        settings.active_preset_id = fallback_id;
        // Persist the corrected setting
        save_settings(&settings)?;
    }

    Ok(settings)
}

/// Save settings to disk
pub fn save_settings(settings: &AppSettings) -> Result<(), AppError> {
    let path = settings_path()?;
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| AppError::Config(format!("Failed to serialize settings: {}", e)))?;
    std::fs::write(&path, json)
        .map_err(|e| AppError::Config(format!("Failed to write settings: {}", e)))?;
    Ok(())
}
