//! Clipboard write via arboard

use arboard::Clipboard;
use tracing::debug;

use crate::error::AppError;

/// Copy text to the system clipboard.
/// On macOS, clipboard access must happen on the main thread.
pub fn set_clipboard_text(text: &str) -> Result<(), AppError> {
    debug!(len = text.len(), "Setting clipboard text");
    set_clipboard_text_inner(text)
}

#[cfg(target_os = "macos")]
fn set_clipboard_text_inner(text: &str) -> Result<(), AppError> {
    use crate::tray;
    let app = tray::app_handle()
        .ok_or_else(|| AppError::Output("App handle not available".to_string()))?;

    let text = text.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let result = (|| {
            let mut clipboard = Clipboard::new()
                .map_err(|e| AppError::Output(format!("Failed to access clipboard: {}", e)))?;
            clipboard
                .set_text(text)
                .map_err(|e| AppError::Output(format!("Failed to set clipboard: {}", e)))?;
            Ok(())
        })();
        let _ = tx.send(result);
    })
    .map_err(|e| AppError::Output(format!("Failed to dispatch to main thread: {}", e)))?;

    rx.recv()
        .map_err(|e| AppError::Output(format!("Main thread channel error: {}", e)))?
}

#[cfg(not(target_os = "macos"))]
fn set_clipboard_text_inner(text: &str) -> Result<(), AppError> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| AppError::Output(format!("Failed to access clipboard: {}", e)))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| AppError::Output(format!("Failed to set clipboard: {}", e)))?;
    Ok(())
}
