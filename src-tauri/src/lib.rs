mod ai;
mod audio;
mod config;
mod error;
mod hotkey;
mod logging;
mod output;
mod tray;

use std::sync::RwLock;

use ai::gemini::{GeminiProvider, ModelInfo};
use ai::pool::{ProviderEntry, ProviderPool};
use config::schema::{AppSettings, Preset, ProviderConfig};
use hotkey::conflict::HotkeyBinding;
use once_cell::sync::Lazy;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::MacosLauncher;

/// Global provider pool, accessible from the hotkey manager
pub static PROVIDER_POOL: Lazy<RwLock<ProviderPool>> =
    Lazy::new(|| RwLock::new(ProviderPool::new()));

/// Global settings cache, used to read the active preset's system prompt
pub static SETTINGS: Lazy<RwLock<AppSettings>> =
    Lazy::new(|| RwLock::new(AppSettings::default()));

/// Get the active system prompt from the cached settings
pub fn active_system_prompt() -> String {
    let settings = SETTINGS.read().unwrap();
    settings
        .presets
        .iter()
        .find(|p| p.id == settings.active_preset_id)
        .map(|p| p.system_prompt.clone())
        .unwrap_or_else(|| {
            "Transcribe the following audio accurately. Output only the transcription without any additional commentary.".to_string()
        })
}

// ── Hotkey commands ──────────────────────────────────────────────

/// Register a new hotkey binding. Must run on the main thread.
#[tauri::command]
async fn register_hotkey(binding: HotkeyBinding, app: AppHandle) -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let result = hotkey::manager::register(&binding);
        let _ = tx.send(result);
    })
    .map_err(|e| e.to_string())?;

    rx.recv()
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Unregister the current hotkey. Must run on the main thread.
#[tauri::command]
async fn unregister_hotkey(app: AppHandle) -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let result = hotkey::manager::unregister();
        let _ = tx.send(result);
    })
    .map_err(|e| e.to_string())?;

    rx.recv()
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Check if a binding conflicts with the currently registered app hotkey.
#[tauri::command]
async fn check_conflict(binding: HotkeyBinding) -> Result<bool, String> {
    if let Ok(settings) = SETTINGS.read() {
        let current = HotkeyBinding {
            modifiers: settings.hotkey.modifiers.clone(),
            key: settings.hotkey.key.clone(),
        };
        Ok(hotkey::conflict::bindings_match(&binding, &current))
    } else {
        Ok(false)
    }
}

/// Check if a binding conflicts with a known system hotkey.
#[tauri::command]
async fn check_system_conflict(binding: HotkeyBinding) -> Result<bool, String> {
    Ok(hotkey::conflict::conflicts_with_system(&binding))
}

// ── Auto-start command ───────────────────────────────────────────

#[tauri::command]
async fn set_autostart(enabled: bool, app: AppHandle) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())?;
    } else {
        manager.disable().map_err(|e| e.to_string())?;
    }
    tracing::info!("Auto-start {}", if enabled { "enabled" } else { "disabled" });
    Ok(())
}

// ── Provider commands ────────────────────────────────────────────

/// Test an AI provider connection.
#[tauri::command]
async fn test_provider_connection(provider: ProviderConfig) -> Result<bool, String> {
    let provider_type_str = match provider.provider_type {
        config::schema::ProviderType::Gemini => "gemini".to_string(),
    };
    let entry = ProviderEntry {
        api_key: provider.api_key,
        model: provider.model,
        provider_type: provider_type_str,
    };
    ProviderPool::test_provider(&entry)
        .await
        .map_err(|e| e.to_string())
}

/// List available models for a provider type.
#[tauri::command]
async fn list_provider_models(
    provider_type: String,
    api_key: String,
) -> Result<Vec<ModelInfo>, String> {
    match provider_type.to_lowercase().as_str() {
        "gemini" => GeminiProvider::list_models(&api_key)
            .await
            .map_err(|e| e.to_string()),
        _ => Err(format!("Unknown provider type: {}", provider_type)),
    }
}

// ── Settings commands ────────────────────────────────────────────

#[tauri::command]
async fn load_settings() -> Result<AppSettings, String> {
    config::manager::load_settings().map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_settings(settings: AppSettings, app: AppHandle) -> Result<(), String> {
    config::manager::save_settings(&settings).map_err(|e| e.to_string())?;
    apply_settings(&settings, &app).await;
    Ok(())
}

// ── Preset commands ──────────────────────────────────────────────

#[tauri::command]
async fn get_presets() -> Result<Vec<Preset>, String> {
    let settings = config::manager::load_settings().map_err(|e| e.to_string())?;
    Ok(settings.presets)
}

#[tauri::command]
async fn set_active_preset(preset_id: String) -> Result<(), String> {
    let mut settings = config::manager::load_settings().map_err(|e| e.to_string())?;

    if !settings.presets.iter().any(|p| p.id == preset_id) {
        return Err(format!("Preset '{}' not found", preset_id));
    }

    settings.active_preset_id = preset_id;
    config::manager::save_settings(&settings).map_err(|e| e.to_string())?;

    // Update cached settings and tray tooltip
    if let Some(preset) = settings.presets.iter().find(|p| p.id == settings.active_preset_id) {
        tray::set_tray_tooltip(&preset.name);
    }
    if let Ok(mut cached) = SETTINGS.write() {
        *cached = settings;
    }

    Ok(())
}

#[tauri::command]
async fn save_preset(preset: Preset) -> Result<(), String> {
    let mut settings = config::manager::load_settings().map_err(|e| e.to_string())?;

    if let Some(existing) = settings.presets.iter_mut().find(|p| p.id == preset.id) {
        existing.name = preset.name;
        existing.system_prompt = preset.system_prompt;
    } else {
        settings.presets.push(preset);
    }

    config::manager::save_settings(&settings).map_err(|e| e.to_string())?;

    // Update tray tooltip if active preset name changed
    if let Some(active) = settings.presets.iter().find(|p| p.id == settings.active_preset_id) {
        tray::set_tray_tooltip(&active.name);
    }
    if let Ok(mut cached) = SETTINGS.write() {
        *cached = settings;
    }

    Ok(())
}

#[tauri::command]
async fn delete_preset(preset_id: String) -> Result<(), String> {
    let mut settings = config::manager::load_settings().map_err(|e| e.to_string())?;

    // Check if it's a built-in preset
    if let Some(preset) = settings.presets.iter().find(|p| p.id == preset_id) {
        if preset.is_builtin {
            return Err("Cannot delete a built-in preset".to_string());
        }
    } else {
        return Err(format!("Preset '{}' not found", preset_id));
    }

    settings.presets.retain(|p| p.id != preset_id);

    // If the deleted preset was active, fall back
    if settings.active_preset_id == preset_id {
        settings.active_preset_id = settings
            .presets
            .first()
            .map(|p| p.id.clone())
            .unwrap_or_else(|| "de-transcribe".to_string());
    }

    config::manager::save_settings(&settings).map_err(|e| e.to_string())?;

    // Update tray tooltip (active preset may have changed due to fallback)
    if let Some(active) = settings.presets.iter().find(|p| p.id == settings.active_preset_id) {
        tray::set_tray_tooltip(&active.name);
    }
    if let Ok(mut cached) = SETTINGS.write() {
        *cached = settings;
    }

    Ok(())
}

// ── Settings application ─────────────────────────────────────────

/// Apply settings: rebuild provider pool, re-register hotkey
async fn apply_settings(settings: &AppSettings, app: &AppHandle) {
    // Update cached settings
    if let Ok(mut cached) = SETTINGS.write() {
        *cached = settings.clone();
    }

    // Rebuild provider pool from enabled providers
    let entries: Vec<ProviderEntry> = settings
        .providers
        .iter()
        .filter(|p| p.enabled)
        .map(|p| {
            let provider_type_str = match p.provider_type {
                config::schema::ProviderType::Gemini => "gemini".to_string(),
            };
            ProviderEntry {
                api_key: p.api_key.clone(),
                model: p.model.clone(),
                provider_type: provider_type_str,
            }
        })
        .collect();

    if let Ok(mut pool) = PROVIDER_POOL.write() {
        pool.rebuild(&entries);
    }

    // Re-register hotkey on main thread
    let binding = HotkeyBinding {
        modifiers: settings.hotkey.modifiers.clone(),
        key: settings.hotkey.key.clone(),
    };
    let app_clone = app.clone();
    let _ = app_clone.run_on_main_thread(move || {
        match hotkey::manager::register(&binding) {
            Ok(()) => tracing::info!(
                "Hotkey re-registered: {}",
                hotkey::manager::format_hotkey(&binding)
            ),
            Err(e) => tracing::warn!("Failed to re-register hotkey: {}", e),
        }
    });

    // Update tray tooltip with active preset name
    if let Some(preset) = settings.presets.iter().find(|p| p.id == settings.active_preset_id) {
        tray::set_tray_tooltip(&preset.name);
    }
}

// ── App entry point ──────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::init();
    tracing::info!("Starting Pisum Transcript v{}", env!("CARGO_PKG_VERSION"));

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            register_hotkey,
            unregister_hotkey,
            check_conflict,
            check_system_conflict,
            test_provider_connection,
            list_provider_models,
            load_settings,
            save_settings,
            get_presets,
            set_active_preset,
            save_preset,
            delete_preset,
            set_autostart,
        ])
        .setup(|app| {
            // Hide from macOS dock — this is a menu-bar-only (tray) app
            #[cfg(target_os = "macos")]
            {
                use objc2::MainThreadMarker;
                use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
                let mtm = unsafe { MainThreadMarker::new_unchecked() };
                let ns_app = NSApplication::sharedApplication(mtm);
                ns_app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
            }

            tray::setup_tray(app)?;

            // Initialize hotkey manager on main thread
            hotkey::manager::init(app.handle())?;

            // Initialize config (creates defaults on first launch)
            let is_first_launch = config::manager::init()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

            // Load settings and apply
            let settings = config::manager::load_settings()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

            // Register hotkey from config
            let binding = HotkeyBinding {
                modifiers: settings.hotkey.modifiers.clone(),
                key: settings.hotkey.key.clone(),
            };
            match hotkey::manager::register(&binding) {
                Ok(()) => tracing::info!(
                    "Hotkey registered: {}",
                    hotkey::manager::format_hotkey(&binding)
                ),
                Err(e) => tracing::warn!("Failed to register hotkey: {}", e),
            }

            // Rebuild provider pool from config
            let entries: Vec<ProviderEntry> = settings
                .providers
                .iter()
                .filter(|p| p.enabled)
                .map(|p| {
                    let provider_type_str = match p.provider_type {
                        config::schema::ProviderType::Gemini => "gemini".to_string(),
                    };
                    ProviderEntry {
                        api_key: p.api_key.clone(),
                        model: p.model.clone(),
                        provider_type: provider_type_str,
                    }
                })
                .collect();

            if let Ok(mut pool) = PROVIDER_POOL.write() {
                pool.rebuild(&entries);
            }

            // Cache settings
            if let Ok(mut cached) = SETTINGS.write() {
                *cached = settings.clone();
            }

            if settings.providers.is_empty() {
                tracing::info!(
                    "No AI providers configured. Configure a provider in Settings."
                );
            }

            // First launch: enable auto-start, show welcome notification, open settings
            if is_first_launch {
                if settings.start_with_system {
                    use tauri_plugin_autostart::ManagerExt;
                    if let Err(e) = app.handle().autolaunch().enable() {
                        tracing::warn!("Failed to enable auto-start on first launch: {}", e);
                    }
                }

                tray::send_notification(
                    "Welcome to Pisum Transcript!",
                    "Please configure an AI provider to get started.",
                );
                // Open settings window
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }

            // Set tray tooltip with active preset
            if let Some(preset) = settings.presets.iter().find(|p| p.id == settings.active_preset_id) {
                tray::set_tray_tooltip(&preset.name);
            }

            tracing::info!("App setup complete");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
