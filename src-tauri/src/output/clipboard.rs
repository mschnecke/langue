//! Clipboard write via arboard

use arboard::Clipboard;

use crate::error::AppError;

/// Copy text to the system clipboard
pub fn set_clipboard_text(text: &str) -> Result<(), AppError> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| AppError::Output(format!("Failed to access clipboard: {}", e)))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| AppError::Output(format!("Failed to set clipboard: {}", e)))?;
    Ok(())
}
