mod audio;
mod error;
mod hotkey;
mod logging;
mod tray;

use hotkey::conflict::HotkeyBinding;
use tauri::AppHandle;
use tauri_plugin_autostart::MacosLauncher;

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

/// Check if a binding conflicts with a known system hotkey.
#[tauri::command]
async fn check_system_conflict(binding: HotkeyBinding) -> Result<bool, String> {
    Ok(hotkey::conflict::conflicts_with_system(&binding))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging first so all subsequent setup is logged
    logging::init();
    tracing::info!("Starting Pisum Langue v{}", env!("CARGO_PKG_VERSION"));

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            register_hotkey,
            unregister_hotkey,
            check_system_conflict,
        ])
        .setup(|app| {
            tray::setup_tray(app)?;

            // Initialize hotkey manager on main thread
            hotkey::manager::init(app.handle())?;

            // Register default hotkey: Ctrl+Shift+Space (Windows) / Cmd+Shift+Space (macOS)
            let default_binding = HotkeyBinding {
                #[cfg(target_os = "macos")]
                modifiers: vec!["Cmd".to_string(), "Shift".to_string()],
                #[cfg(not(target_os = "macos"))]
                modifiers: vec!["Ctrl".to_string(), "Shift".to_string()],
                key: "Space".to_string(),
            };

            match hotkey::manager::register(&default_binding) {
                Ok(()) => tracing::info!(
                    "Default hotkey registered: {}",
                    hotkey::manager::format_hotkey(&default_binding)
                ),
                Err(e) => tracing::warn!("Failed to register default hotkey: {}", e),
            }

            tracing::info!("App setup complete");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
