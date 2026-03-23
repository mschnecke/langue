//! Paste simulation via enigo (Ctrl+V / Cmd+V)

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use tracing::debug;

use crate::error::AppError;

/// Simulate a paste keystroke (Ctrl+V on Windows/Linux, Cmd+V on macOS)
pub fn simulate_paste() -> Result<(), AppError> {
    debug!("Simulating paste keystroke");
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
