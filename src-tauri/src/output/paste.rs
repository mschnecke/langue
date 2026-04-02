//! Paste simulation via enigo (Ctrl+V / Cmd+V)

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use tracing::debug;

use crate::error::AppError;

/// Simulate a paste keystroke (Ctrl+V on Windows/Linux, Cmd+V on macOS).
/// On macOS, CGEvent-based input simulation must happen on the main thread.
pub fn simulate_paste() -> Result<(), AppError> {
    debug!("Simulating paste keystroke");
    simulate_paste_inner()
}

fn do_paste() -> Result<(), AppError> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| AppError::Output(format!("Failed to create input simulator: {}", e)))?;

    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    enigo
        .key(modifier, Direction::Press)
        .map_err(|e| AppError::Output(format!("Paste simulation failed: {}", e)))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| AppError::Output(format!("Paste simulation failed: {}", e)))?;
    enigo
        .key(modifier, Direction::Release)
        .map_err(|e| AppError::Output(format!("Paste simulation failed: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn simulate_paste_inner() -> Result<(), AppError> {
    use crate::tray;
    let app = tray::app_handle()
        .ok_or_else(|| AppError::Output("App handle not available".to_string()))?;

    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(do_paste());
    })
    .map_err(|e| AppError::Output(format!("Failed to dispatch to main thread: {}", e)))?;

    rx.recv()
        .map_err(|e| AppError::Output(format!("Main thread channel error: {}", e)))?
}

#[cfg(not(target_os = "macos"))]
fn simulate_paste_inner() -> Result<(), AppError> {
    do_paste()
}
