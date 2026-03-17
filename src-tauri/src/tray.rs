use once_cell::sync::Lazy;
use std::sync::RwLock;
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

static APP_HANDLE: Lazy<RwLock<Option<AppHandle>>> = Lazy::new(|| RwLock::new(None));

/// Set up the system tray icon and menu.
pub fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Store the app handle globally for notifications from any module
    {
        let mut handle = APP_HANDLE.write().unwrap();
        *handle = Some(app.handle().clone());
    }

    let settings_item = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&settings_item)
        .separator()
        .item(&quit_item)
        .build()?;

    let tray_icon = load_tray_icon(app);

    let tray_builder = TrayIconBuilder::with_id("main")
        .icon(tray_icon)
        .tooltip("Pisum Transcript")
        .menu(&menu);

    // macOS: mark as template image so the system auto-inverts for dark/light mode
    #[cfg(target_os = "macos")]
    let tray_builder = tray_builder.icon_as_template(true);

    let _tray = tray_builder
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "settings" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    // Intercept window close to hide instead of quit
    if let Some(window) = app.get_webview_window("main") {
        let window_clone = window.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window_clone.hide();
            }
        });
    }

    Ok(())
}

/// Send an OS-native notification.
/// Respects the `showTrayNotifications` setting unless `force` is true.
/// Error notifications should use `force = true` to always show.
pub fn send_notification(title: &str, message: &str) {
    send_notification_impl(title, message, true)
}

/// Send an informational notification that respects the user's notification preference.
pub fn send_info_notification(title: &str, message: &str) {
    send_notification_impl(title, message, false)
}

fn send_notification_impl(title: &str, message: &str, force: bool) {
    if !force {
        if let Ok(settings) = crate::SETTINGS.read() {
            if !settings.show_tray_notifications {
                return;
            }
        }
    }

    let handle = APP_HANDLE.read().unwrap();
    if let Some(app) = handle.as_ref() {
        use tauri_plugin_notification::NotificationExt;
        let _ = app
            .notification()
            .builder()
            .title(title)
            .body(message)
            .show();
    }
}

/// Update the tray tooltip to show the active preset name.
pub fn set_tray_tooltip(preset_name: &str) {
    let handle = APP_HANDLE.read().unwrap();
    if let Some(app) = handle.as_ref() {
        if let Some(tray) = app.tray_by_id("main") {
            let tooltip = format!("Pisum Transcript — {}", preset_name);
            let _ = tray.set_tooltip(Some(&tooltip));
        }
    }
}

/// Update the tray icon to reflect recording state.
pub fn set_recording_state(recording: bool) {
    let handle = APP_HANDLE.read().unwrap();
    if let Some(app) = handle.as_ref() {
        if let Some(tray) = app.tray_by_id("main") {
            let icon_name = if recording {
                get_recording_icon_name()
            } else {
                get_idle_icon_name()
            };

            // Try resource dir first, then dev paths
            let icon = try_load_icon_by_name(app, &icon_name)
                .unwrap_or_else(|| {
                    if recording {
                        Image::from_bytes(include_bytes!("../icons/tray-icon-recording.png"))
                            .expect("Failed to load embedded recording icon")
                    } else {
                        Image::from_bytes(include_bytes!("../icons/tray-icon.png"))
                            .expect("Failed to load embedded tray icon")
                    }
                });

            let _ = tray.set_icon(Some(icon));
        }
    }
}

/// Try to load a tray icon by filename from known paths.
fn try_load_icon_by_name(app: &AppHandle, icon_name: &str) -> Option<Image<'static>> {
    // Resource directory (production)
    if let Ok(resource_dir) = app.path().resource_dir() {
        for path in [
            resource_dir.join(icon_name),
            resource_dir.join("icons").join(icon_name),
        ] {
            if path.exists() {
                if let Ok(img) = Image::from_path(&path) {
                    return Some(img);
                }
            }
        }
    }

    // Dev mode: walk up from executable to find src-tauri/icons
    if let Ok(exe_path) = std::env::current_exe() {
        let mut dir = exe_path.parent().map(|p| p.to_path_buf());
        for _ in 0..5 {
            if let Some(ref d) = dir {
                let candidate = d.join("src-tauri").join("icons").join(icon_name);
                if candidate.exists() {
                    if let Ok(img) = Image::from_path(&candidate) {
                        return Some(img);
                    }
                }
                dir = d.parent().map(|p| p.to_path_buf());
            }
        }
    }

    None
}

/// Load the tray icon image.
fn load_tray_icon(app: &tauri::App) -> Image<'static> {
    let icon_name = get_idle_icon_name();

    // Try to load from known paths
    if let Some(img) = try_load_icon_by_name(app.handle(), &icon_name) {
        return img;
    }

    Image::from_bytes(include_bytes!("../icons/tray-icon.png"))
        .expect("Failed to load embedded tray icon")
}

/// Get the idle tray icon filename for the current platform.
fn get_idle_icon_name() -> String {
    #[cfg(target_os = "macos")]
    {
        // macOS: use template images — the system handles dark/light adaptation
        "tray-iconTemplate.png".to_string()
    }
    #[cfg(not(target_os = "macos"))]
    {
        // Windows/Linux: use theme-specific icons
        if is_dark_mode() {
            "tray-icon-light.png".to_string()
        } else {
            "tray-icon-dark.png".to_string()
        }
    }
}

/// Get the recording tray icon filename for the current platform.
fn get_recording_icon_name() -> String {
    #[cfg(target_os = "macos")]
    {
        "tray-iconTemplate-recording.png".to_string()
    }
    #[cfg(not(target_os = "macos"))]
    {
        "tray-icon-recording.png".to_string()
    }
}

/// Detect if the system is in dark mode (Windows/Linux only).
#[cfg(target_os = "windows")]
fn is_dark_mode() -> bool {
    use windows::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD,
    };
    use windows::core::w;

    let mut data: u32 = 1;
    let mut size = std::mem::size_of::<u32>() as u32;

    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut data as *mut u32 as *mut _),
            Some(&mut size),
        )
    };

    result.is_ok() && data == 0
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn is_dark_mode() -> bool {
    false
}
